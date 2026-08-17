#[cfg(not(target_arch = "wasm32"))]
use crate::q42_volume::StreamingQ42VolumeWriter;
use crate::NQuin;
#[cfg(not(target_arch = "wasm32"))]
use crate::QUINS_PER_BLOCK;
#[cfg(not(target_arch = "wasm32"))]
use std::cmp::Ordering;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::BinaryHeap;
#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

// 50MB buffer limit: ~1 million Quins (48 bytes each -> 48MB)
const CHUNK_SIZE_LIMIT: usize = 1_000_000;
/// Bound simultaneously open sorted runs and their reader buffers.
pub(super) const MAX_MERGE_FAN_IN: usize = 32;

#[cfg(not(target_arch = "wasm32"))]
mod volume_publisher;
#[cfg(not(target_arch = "wasm32"))]
pub use volume_publisher::{Q42VolumePublishStats, DEFAULT_Q42_SEGMENT_MAX_BYTES};
#[cfg(not(target_arch = "wasm32"))]
mod lexicon_spill;
#[cfg(not(target_arch = "wasm32"))]
use lexicon_spill::LexiconSpill;

pub struct ExternalSorter {
    buffer: Vec<NQuin>,
    chunk_files: Vec<PathBuf>,
    temp_dir: PathBuf,
    note: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    total_quins: u64,
    /// Complete-mode terms. Small catalogs stay in RAM; continent dumps spill
    /// sorted lex runs under `temp_dir` (see `lexicon_spill`).
    #[cfg(not(target_arch = "wasm32"))]
    lex: LexiconSpill,
    #[cfg(target_arch = "wasm32")]
    lex: HashMap<u64, String>,
    lex_collisions: u64,
}

impl ExternalSorter {
    pub fn new(temp_dir: PathBuf) -> Self {
        // Ensure temp_dir exists
        std::fs::create_dir_all(&temp_dir).unwrap();
        Self {
            buffer: Vec::with_capacity(CHUNK_SIZE_LIMIT),
            chunk_files: Vec::new(),
            note: None,
            #[cfg(not(target_arch = "wasm32"))]
            lex: LexiconSpill::new(temp_dir.clone()),
            #[cfg(target_arch = "wasm32")]
            lex: HashMap::new(),
            temp_dir,
            total_quins: 0,
            lex_collisions: 0,
        }
    }

    pub fn set_note_sink(&mut self, sink: std::sync::Arc<dyn Fn(&str) + Send + Sync>) {
        self.note = Some(sink.clone());
        #[cfg(not(target_arch = "wasm32"))]
        self.lex.set_note_sink(sink);
    }

    fn note(&self, msg: impl AsRef<str>) {
        let msg = msg.as_ref();
        if let Some(sink) = &self.note {
            sink(msg);
        } else {
            println!("{msg}");
        }
    }

    pub fn push(&mut self, quin: NQuin) -> std::io::Result<()> {
        self.buffer.push(quin);
        self.total_quins += 1;
        if self.buffer.len() >= CHUNK_SIZE_LIMIT {
            self.flush_chunk()?;
        }
        Ok(())
    }

    /// Record a term so its hash resolves back to its lexical string in the volume.
    ///
    /// First writer wins for the stored value. A genuine handle collision (a DIFFERENT
    /// term hashing to an already-seen token) is COUNTED rather than silently assumed
    /// impossible, so ingest can surface it (lexicon collision backstop, task #22).
    pub fn push_lex(&mut self, hash: u64, term: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Err(err) = self.lex.intern(hash, term) {
                panic!("lexicon spill I/O during intern: {err}");
            }
            self.lex_collisions = self.lex.collision_count();
        }
        #[cfg(target_arch = "wasm32")]
        {
            match self.lex.get(&hash) {
                None => {
                    self.lex.insert(hash, term.to_string());
                }
                Some(existing) if existing == term => {}
                Some(_) => {
                    self.lex_collisions += 1;
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn intern_term(&mut self, hash: u64, term: &str) -> std::io::Result<()> {
        self.lex.intern(hash, term)?;
        self.lex_collisions = self.lex.collision_count();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn intern_term(&mut self, hash: u64, term: &str) -> std::io::Result<()> {
        self.push_lex(hash, term);
        Ok(())
    }

    pub fn lex_collision_count(&self) -> u64 {
        self.lex_collisions
    }

    pub fn lex_interned_count(&self) -> u64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.lex.interned_count()
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.lex.len() as u64
        }
    }

    pub fn quin_run_count(&self) -> usize {
        self.chunk_files.len()
    }

    pub fn quin_total(&self) -> u64 {
        self.total_quins
    }

    pub fn replace_chunks(&mut self, files: Vec<PathBuf>, total_quins: u64) {
        self.chunk_files = files;
        self.total_quins = total_quins;
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn adopt_lex_runs(&mut self) -> std::io::Result<()> {
        self.lex.adopt_existing_runs()
    }

    /// Rebuild over flushed `chunk_*.tmp` / `lexrun_*.tmp` in `temp_dir`.
    pub fn adopt_existing(temp_dir: PathBuf) -> std::io::Result<Self> {
        let mut chunk_files = Vec::new();
        let mut total_quins = 0u64;
        if temp_dir.is_dir() {
            let mut names: Vec<PathBuf> = std::fs::read_dir(&temp_dir)?
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("chunk_") && n.ends_with(".tmp"))
                })
                .collect();
            names.sort();
            for path in names {
                let len = std::fs::metadata(&path)?.len();
                if len % 48 != 0 {
                    continue;
                }
                total_quins += len / 48;
                chunk_files.push(path);
            }
        }
        let mut sorter = Self::new(temp_dir);
        sorter.replace_chunks(chunk_files, total_quins);
        #[cfg(not(target_arch = "wasm32"))]
        sorter.adopt_lex_runs()?;
        Ok(sorter)
    }

    pub fn lex_run_count(&self) -> usize {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.lex.run_count()
        }
        #[cfg(target_arch = "wasm32")]
        {
            0
        }
    }

    fn flush_chunk(&mut self) -> std::io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        // 1. Sort by object hash: GPU radix when eligible, CPU radix floor otherwise.
        let sort_started = std::time::Instant::now();
        let sort = crate::query::graph_accel::sort_quins_by_object(&mut self.buffer);
        let sort_ms = sort_started.elapsed().as_millis();
        self.note(format!(
            "quin chunk {} sorted n={} path={} {}ms",
            self.chunk_files.len(),
            sort.n,
            sort.path,
            sort_ms
        ));

        // 2. Flush to disk as a temporary file
        let chunk_path = self
            .temp_dir
            .join(format!("chunk_{}.tmp", self.chunk_files.len()));
        let mut file = std::io::BufWriter::new(File::create(&chunk_path)?);

        for q in &self.buffer {
            file.write_all(bytemuck::bytes_of(q))?;
        }
        file.flush()?;

        self.chunk_files.push(chunk_path);
        self.buffer.clear();
        Ok(())
    }

    /// K-way merge sorted chunks into a unified v2 `.q42` volume.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn merge(mut self, final_q42: &Path) -> std::io::Result<u64> {
        // Flush any remaining quins
        self.flush_chunk()?;

        if self.lex_collisions > 0 {
            eprintln!(
                "warning: {} lexicon handle collision(s) during ingest — distinct terms \
                 shared a 60-bit token; first writer kept (task #22 backstop)",
                self.lex_collisions
            );
        }

        let lex_map = match self.lex.memory_map() {
            Some(map) => map.clone(),
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "lexicon spilled to disk; republish with --segment-mib so lexicon shards stay bounded",
                ));
            }
        };
        let mut writer = StreamingQ42VolumeWriter::new(&lex_map)?;
        // Catalog ontologies (Pages ingest), not personal records.
        // Sanctuary bits on a Quin still win inside the writer.
        writer.declare_permissive_commons();

        if self.chunk_files.is_empty() {
            writer.finish(final_q42)?;
            return Ok(0);
        }

        // Compact raw sorted runs hierarchically before the final Q42 merge.
        // This keeps file descriptors and buffered reader memory bounded.
        let chunk_files = self.compact_runs()?;

        // Open the bounded final run set.
        let mut readers: Vec<BufReader<File>> = Vec::new();
        for chunk_path in &chunk_files {
            let f = File::open(chunk_path)?;
            readers.push(BufReader::with_capacity(1024 * 1024, f)); // 1MB buffer per file
        }

        // Priority queue for K-way merge
        #[derive(Eq)]
        struct HeapItem {
            quin: NQuin,
            reader_idx: usize,
        }

        impl Ord for HeapItem {
            fn cmp(&self, other: &Self) -> Ordering {
                // Min-heap based on object hash
                other.quin.object.cmp(&self.quin.object)
            }
        }
        impl PartialOrd for HeapItem {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl PartialEq for HeapItem {
            fn eq(&self, other: &Self) -> bool {
                self.quin.object == other.quin.object
            }
        }

        let mut heap = BinaryHeap::new();

        // Initialize heap with first quin from each file
        for (idx, reader) in readers.iter_mut().enumerate() {
            if let Some(quin) = Self::read_quin(reader)? {
                heap.push(HeapItem {
                    quin,
                    reader_idx: idx,
                });
            }
        }

        let mut block_buffer = Vec::with_capacity(QUINS_PER_BLOCK);
        let mut block_seq = 0u64;

        while let Some(item) = heap.pop() {
            block_buffer.push(item.quin);

            // Fetch next from the same reader
            let idx = item.reader_idx;
            if let Some(next_quin) = Self::read_quin(&mut readers[idx])? {
                heap.push(HeapItem {
                    quin: next_quin,
                    reader_idx: idx,
                });
            }

            if block_buffer.len() == QUINS_PER_BLOCK {
                writer.push_block(block_seq, &block_buffer)?;
                block_buffer.clear();
                block_seq += 1;
            }
        }

        // Flush remaining in block buffer
        if !block_buffer.is_empty() {
            writer.push_block(block_seq, &block_buffer)?;
            block_seq += 1;
        }

        writer.finish(final_q42)?;

        // Cleanup temp files
        for chunk_path in &chunk_files {
            let _ = std::fs::remove_file(chunk_path);
        }

        Ok(block_seq)
    }

    /// Mock for WASM
    #[cfg(target_arch = "wasm32")]
    pub fn merge(self, _final_q42: &Path) -> std::io::Result<u64> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Not supported on WASM",
        ))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn read_quin(reader: &mut BufReader<File>) -> std::io::Result<Option<NQuin>> {
        let mut bytes = [0u8; 48];
        match reader.read_exact(&mut bytes) {
            Ok(_) => Ok(Some(bytemuck::pod_read_unaligned(&bytes))),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => Err(e),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn compact_runs(&self) -> std::io::Result<Vec<PathBuf>> {
        let mut runs = self.chunk_files.clone();
        let mut pass = 0usize;
        while runs.len() > MAX_MERGE_FAN_IN {
            self.note(format!(
                "Quin run compact pass {pass}: {} runs (fan-in {MAX_MERGE_FAN_IN}).",
                runs.len()
            ));
            let mut next =
                Vec::with_capacity((runs.len() + MAX_MERGE_FAN_IN - 1) / MAX_MERGE_FAN_IN);
            for (group, inputs) in runs.chunks(MAX_MERGE_FAN_IN).enumerate() {
                let output = self.temp_dir.join(format!("merge_{pass}_{group}.tmp"));
                Self::merge_raw_runs(inputs, &output)?;
                next.push(output);
                // Delete this group immediately. Waiting until the whole pass
                // finishes doubles disk use (the OSM merge died on that).
                for input in inputs {
                    let _ = std::fs::remove_file(input);
                }
            }
            runs = next;
            pass += 1;
        }
        Ok(runs)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn merge_raw_runs(inputs: &[PathBuf], output: &Path) -> std::io::Result<()> {
        if inputs.is_empty() || inputs.len() > MAX_MERGE_FAN_IN {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid sorted-run merge group",
            ));
        }
        #[derive(Eq)]
        struct Item {
            quin: NQuin,
            reader: usize,
        }
        impl Ord for Item {
            fn cmp(&self, other: &Self) -> Ordering {
                other.quin.object.cmp(&self.quin.object)
            }
        }
        impl PartialOrd for Item {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl PartialEq for Item {
            fn eq(&self, other: &Self) -> bool {
                self.quin.object == other.quin.object
            }
        }
        let mut readers = Vec::with_capacity(inputs.len());
        for input in inputs {
            readers.push(BufReader::with_capacity(64 * 1024, File::open(input)?));
        }
        let mut heap = BinaryHeap::new();
        for (reader, input) in readers.iter_mut().enumerate() {
            if let Some(quin) = Self::read_quin(input)? {
                heap.push(Item { quin, reader });
            }
        }
        let mut writer = std::io::BufWriter::new(
            std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(output)?,
        );
        while let Some(item) = heap.pop() {
            writer.write_all(bytemuck::bytes_of(&item.quin))?;
            if let Some(quin) = Self::read_quin(&mut readers[item.reader])? {
                heap.push(Item {
                    quin,
                    reader: item.reader,
                });
            }
        }
        writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    use tempfile::TempDir;

    #[test]
    fn push_lex_detects_handle_collisions_without_silent_assumption() {
        let dir = std::env::temp_dir().join("qualia_extsort_lex_collision_test");
        let mut s = ExternalSorter::new(dir);
        let h = 0x0123_4567_89AB_CDEF;
        s.push_lex(h, "alpha"); // New
        s.push_lex(h, "alpha"); // idempotent — same token, same term
        assert_eq!(
            s.lex_collision_count(),
            0,
            "re-interning the same term is not a collision"
        );
        s.push_lex(h, "beta"); // distinct term, same token -> a real collision
        assert_eq!(
            s.lex_collision_count(),
            1,
            "a distinct term on an already-seen token must be counted, not silently ignored"
        );
        s.push_lex(0x0FED_CBA9_8765_4321, "gamma"); // different token -> no collision
        assert_eq!(s.lex_collision_count(), 1);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn bounded_raw_run_merge_preserves_global_object_order() {
        let dir = TempDir::new().unwrap();
        let mut inputs = Vec::new();
        for (index, object) in [[1u64, 7], [2, 8], [3, 9]].iter().enumerate() {
            let path = dir.path().join(format!("input-{index}.tmp"));
            let mut file = std::io::BufWriter::new(File::create(&path).unwrap());
            for object in object {
                let quin = NQuin {
                    subject: 0,
                    predicate: 0,
                    object: *object,
                    context: 0,
                    metadata: 0,
                    parity: 0,
                };
                file.write_all(bytemuck::bytes_of(&quin)).unwrap();
            }
            file.flush().unwrap();
            inputs.push(path);
        }
        let output = dir.path().join("output.tmp");
        ExternalSorter::merge_raw_runs(&inputs, &output).unwrap();
        let mut reader = BufReader::new(File::open(output).unwrap());
        let mut objects = [0u64; 6];
        for object in &mut objects {
            *object = ExternalSorter::read_quin(&mut reader)
                .unwrap()
                .unwrap()
                .object;
        }
        assert_eq!(objects, [1, 2, 3, 7, 8, 9]);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn merge_declares_permissive_commons_for_catalog_ingest() {
        use crate::q42_volume::{Q42Volume, FLAG_PERMISSIVE_COMMONS, FLAG_SANCTUARY};

        let dir = TempDir::new().unwrap();
        let mut sorter = ExternalSorter::new(dir.path().join("sort"));
        sorter
            .push(NQuin {
                subject: 1,
                predicate: 2,
                object: 3,
                context: 0,
                metadata: 0,
                parity: 0,
            })
            .unwrap();
        sorter.push_lex(1, "urn:q42:catalog-subject");
        let out = dir.path().join("catalog.q42");
        sorter.merge(&out).unwrap();
        let volume = Q42Volume::open(&out).unwrap();
        let flags = volume.header().flags;
        assert_ne!(
            flags & FLAG_PERMISSIVE_COMMONS,
            0,
            "Pages catalog merge must set FLAG_PERMISSIVE_COMMONS"
        );
        assert_eq!(
            flags & FLAG_SANCTUARY,
            0,
            "unmarked catalog Quins must not flip Sanctuary"
        );
    }
}
