#[cfg(not(target_arch = "wasm32"))]
use crate::q42_volume::UnifiedVolumeBuilder;
use crate::NQuin;
#[cfg(not(target_arch = "wasm32"))]
use crate::QUINS_PER_BLOCK;
#[cfg(not(target_arch = "wasm32"))]
use std::cmp::Ordering;
#[cfg(not(target_arch = "wasm32"))]
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

// 50MB buffer limit: ~1 million Quins (48 bytes each -> 48MB)
const CHUNK_SIZE_LIMIT: usize = 1_000_000;

pub struct ExternalSorter {
    buffer: Vec<NQuin>,
    chunk_files: Vec<PathBuf>,
    temp_dir: PathBuf,
    total_quins: u64,
    /// `hash → lexical string` for every term seen, written to the volume's
    /// front-of-file Q42LEX section so literals/IRIs are recoverable from the `.q42`
    /// alone (no separate `.lex` sidecar). Cold ingest path — heap is expected here.
    lex: HashMap<u64, String>,
    /// Count of genuine 60-bit handle collisions seen at intern (distinct terms, same
    /// token). First writer is kept; this makes a collision LOUD instead of a silent
    /// assumption (lexicon collision backstop, task #22).
    lex_collisions: u64,
}

impl ExternalSorter {
    pub fn new(temp_dir: PathBuf) -> Self {
        // Ensure temp_dir exists
        std::fs::create_dir_all(&temp_dir).unwrap();
        Self {
            buffer: Vec::with_capacity(CHUNK_SIZE_LIMIT),
            chunk_files: Vec::new(),
            temp_dir,
            total_quins: 0,
            lex: HashMap::new(),
            lex_collisions: 0,
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
        match self.lex.get(&hash) {
            None => {
                self.lex.insert(hash, term.to_string());
            }
            Some(existing) if existing == term => {} // idempotent: same token, same term
            Some(_) => {
                self.lex_collisions += 1;
            }
        }
    }

    /// Number of distinct-term / same-token collisions detected during ingest. Non-zero
    /// means a 60-bit handle was reused for different lexical values — rare, but no
    /// longer silent. (See `lexicon::LexiconInterner` for the value-preserving form.)
    pub fn lex_collision_count(&self) -> u64 {
        self.lex_collisions
    }

    fn flush_chunk(&mut self) -> std::io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        // 1. Sort the array by Object Hash in place
        self.buffer.sort_unstable_by_key(|q| q.object);

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

        let mut builder = UnifiedVolumeBuilder::with_lex_map(&self.lex).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid Q42LEX: {e:?}"),
            )
        })?;

        if self.chunk_files.is_empty() {
            builder.finish(final_q42)?;
            return Ok(0);
        }

        // Open all chunk files
        let mut readers: Vec<BufReader<File>> = Vec::new();
        for chunk_path in &self.chunk_files {
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
                builder.push_block(block_seq, &block_buffer)?;
                block_buffer.clear();
                block_seq += 1;
            }
        }

        // Flush remaining in block buffer
        if !block_buffer.is_empty() {
            builder.push_block(block_seq, &block_buffer)?;
            block_seq += 1;
        }

        builder.finish(final_q42)?;

        // Cleanup temp files
        for chunk_path in &self.chunk_files {
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
