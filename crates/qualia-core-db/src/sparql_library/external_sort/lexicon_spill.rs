//! Bounded Complete-mode lexicon: intern in a capped map, spill sorted runs to disk.
//!
//! Continent OSM WKT cannot live in a process-wide `HashMap<u64, String>` (the AU dump
//! wedged at ~80 GiB commit / 0 CPU). Cold ingest may allocate; it may not keep every
//! unique literal resident until merge.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Keep at most this many interned UTF-8 bytes in RAM before flushing a sorted run.
pub const DEFAULT_LEX_MEM_BYTES: usize = 256 * 1024 * 1024;
const MAX_TERM_BYTES: usize = 256 * 1024 * 1024;
const MERGE_FAN_IN: usize = 32;

pub struct LexiconSpill {
    map: HashMap<u64, String>,
    bytes: usize,
    cap: usize,
    runs: Vec<PathBuf>,
    temp_dir: PathBuf,
    collisions: u64,
    interned: u64,
    note: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

impl LexiconSpill {
    pub fn new(temp_dir: PathBuf) -> Self {
        Self::with_cap(temp_dir, DEFAULT_LEX_MEM_BYTES)
    }

    pub fn with_cap(temp_dir: PathBuf, cap: usize) -> Self {
        let _ = std::fs::create_dir_all(&temp_dir);
        Self {
            map: HashMap::new(),
            bytes: 0,
            cap: cap.max(1),
            runs: Vec::new(),
            temp_dir,
            collisions: 0,
            interned: 0,
            note: None,
        }
    }

    pub fn set_note_sink(&mut self, sink: Arc<dyn Fn(&str) + Send + Sync>) {
        self.note = Some(sink);
    }

    fn note(&self, msg: impl AsRef<str>) {
        let msg = msg.as_ref();
        if let Some(sink) = &self.note {
            sink(msg);
        } else {
            println!("{msg}");
        }
    }

    pub fn collision_count(&self) -> u64 {
        self.collisions
    }

    pub fn interned_count(&self) -> u64 {
        self.interned
    }

    pub fn run_count(&self) -> usize {
        self.runs.len() + usize::from(!self.map.is_empty())
    }

    /// Pick up `lexrun_*.tmp` already on disk (resume / adopt-scratch).
    pub fn adopt_existing_runs(&mut self) -> io::Result<()> {
        if !self.temp_dir.is_dir() {
            return Ok(());
        }
        let mut names: Vec<PathBuf> = std::fs::read_dir(&self.temp_dir)?
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("lexrun_") && n.ends_with(".tmp"))
            })
            .collect();
        names.sort();
        self.runs = names;
        Ok(())
    }

    /// In-memory map when nothing has spilled (small catalogs / unit tests).
    pub fn memory_map(&self) -> Option<&HashMap<u64, String>> {
        if self.runs.is_empty() {
            Some(&self.map)
        } else {
            None
        }
    }

    pub fn intern(&mut self, hash: u64, term: &str) -> io::Result<()> {
        match self.map.get(&hash) {
            Some(existing) if existing == term => return Ok(()),
            Some(_) => {
                self.collisions += 1;
                return Ok(());
            }
            None => {}
        }
        if term.len() > MAX_TERM_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("lexicon term exceeds {MAX_TERM_BYTES} bytes"),
            ));
        }
        if !self.map.is_empty() && self.bytes.saturating_add(term.len()) > self.cap {
            self.flush_run()?;
        }
        self.bytes = self.bytes.saturating_add(term.len());
        self.map.insert(hash, term.to_string());
        self.interned = self.interned.saturating_add(1);
        Ok(())
    }

    pub fn flush_run(&mut self) -> io::Result<()> {
        if self.map.is_empty() {
            return Ok(());
        }
        let mut entries: Vec<(u64, String)> = self.map.drain().collect();
        entries.sort_unstable_by_key(|(h, _)| *h);
        let path = self
            .temp_dir
            .join(format!("lexrun_{}.tmp", self.runs.len()));
        let mut out = BufWriter::new(File::create(&path)?);
        for (hash, term) in &entries {
            write_record(&mut out, *hash, term.as_bytes())?;
        }
        out.flush()?;
        self.note(format!(
            "Lexicon spill: wrote {} ({} terms, {} bytes interned this run).",
            path.display(),
            entries.len(),
            self.bytes
        ));
        self.bytes = 0;
        self.runs.push(path);
        Ok(())
    }

    /// Unique `(hash, term)` in hash order. First writer wins on collisions across runs.
    pub fn for_each_sorted<F>(&mut self, mut visit: F) -> io::Result<u64>
    where
        F: FnMut(u64, String) -> io::Result<()>,
    {
        self.flush_run()?;
        if self.runs.len() > MERGE_FAN_IN {
            return self.for_each_sorted_bucketed(&mut visit);
        }
        self.compact_runs()?;
        if self.runs.is_empty() {
            return Ok(0);
        }
        if self.runs.len() == 1 {
            return drain_run(&self.runs[0], &mut visit);
        }
        merge_runs(&self.runs, &mut visit, &mut self.collisions)
    }

    /// Split spilled runs into 16 hash-prefix buckets, delete each source run,
    /// then emit one bucket at a time so lexicon shards do not need a second
    /// full copy of the dictionary.
    fn for_each_sorted_bucketed<F>(&mut self, visit: &mut F) -> io::Result<u64>
    where
        F: FnMut(u64, String) -> io::Result<()>,
    {
        const BUCKETS: usize = 16;
        let runs = std::mem::take(&mut self.runs);
        let n_runs = runs.len();
        let mut buckets: Vec<Vec<PathBuf>> = vec![Vec::new(); BUCKETS];
        for (i, run) in runs.iter().enumerate() {
            let mut writers: Vec<Option<BufWriter<File>>> = (0..BUCKETS).map(|_| None).collect();
            let mut pieces: Vec<Option<PathBuf>> = vec![None; BUCKETS];
            let mut reader = BufReader::with_capacity(256 * 1024, File::open(run)?);
            while let Some((hash, term)) = read_record(&mut reader)? {
                let b = ((hash & 0x0FFF_FFFF_FFFF_FFFF) >> 56) as usize % BUCKETS;
                if writers[b].is_none() {
                    let path = self.temp_dir.join(format!("lexbuck_{b:02}_{i}.tmp"));
                    pieces[b] = Some(path.clone());
                    writers[b] = Some(BufWriter::new(File::create(&path)?));
                }
                write_record(writers[b].as_mut().unwrap(), hash, term.as_bytes())?;
            }
            drop(writers);
            for (b, piece) in pieces.into_iter().enumerate() {
                if let Some(path) = piece {
                    buckets[b].push(path);
                }
            }
            let _ = std::fs::remove_file(run);
            if i == 0 || (i + 1) % 16 == 0 || i + 1 == n_runs {
                self.note(format!("lexicon range-split {}/{n_runs} runs", i + 1));
            }
        }
        let mut emitted = 0u64;
        for (b, mut pieces) in buckets.into_iter().enumerate() {
            if pieces.is_empty() {
                continue;
            }
            self.note(format!(
                "lexicon bucket {b}/{BUCKETS}  {} piece(s)",
                pieces.len()
            ));
            while pieces.len() > MERGE_FAN_IN {
                let mut next = Vec::new();
                for (group, inputs) in pieces.chunks(MERGE_FAN_IN).enumerate() {
                    let output = self.temp_dir.join(format!("lexbuckmerge_{b}_{group}.tmp"));
                    write_merged_runs(inputs, &output, &mut self.collisions)?;
                    next.push(output);
                    for old in inputs {
                        let _ = std::fs::remove_file(old);
                    }
                }
                pieces = next;
            }
            emitted += if pieces.len() == 1 {
                drain_run(&pieces[0], visit)?
            } else {
                merge_runs(&pieces, visit, &mut self.collisions)?
            };
            for old in pieces {
                let _ = std::fs::remove_file(old);
            }
        }
        Ok(emitted)
    }

    fn compact_runs(&mut self) -> io::Result<()> {
        let mut pass = 0usize;
        while self.runs.len() > MERGE_FAN_IN {
            self.note(format!(
                "Lexicon spill: compact pass {pass}, {} runs.",
                self.runs.len()
            ));
            let mut next = Vec::new();
            for (group, inputs) in self.runs.chunks(MERGE_FAN_IN).enumerate() {
                let output = self.temp_dir.join(format!("lexmerge_{pass}_{group}.tmp"));
                write_merged_runs(inputs, &output, &mut self.collisions)?;
                next.push(output);
                for old in inputs {
                    let _ = std::fs::remove_file(old);
                }
            }
            self.runs = next;
            pass += 1;
        }
        Ok(())
    }
}

fn write_record(out: &mut impl Write, hash: u64, bytes: &[u8]) -> io::Result<()> {
    let len = u32::try_from(bytes.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "lexicon term longer than u32::MAX",
        )
    })?;
    out.write_all(&hash.to_le_bytes())?;
    out.write_all(&len.to_le_bytes())?;
    out.write_all(bytes)
}

fn read_record(reader: &mut impl Read) -> io::Result<Option<(u64, String)>> {
    let mut hdr = [0u8; 12];
    match reader.read_exact(&mut hdr) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let hash = u64::from_le_bytes(hdr[0..8].try_into().unwrap());
    let len = u32::from_le_bytes(hdr[8..12].try_into().unwrap()) as usize;
    if len > MAX_TERM_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "lexicon run record exceeds MAX_TERM_BYTES",
        ));
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    let term = String::from_utf8(buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("lexicon UTF-8: {e}")))?;
    Ok(Some((hash, term)))
}

fn drain_run<F>(path: &Path, visit: &mut F) -> io::Result<u64>
where
    F: FnMut(u64, String) -> io::Result<()>,
{
    let mut reader = BufReader::with_capacity(1024 * 1024, File::open(path)?);
    let mut n = 0u64;
    let mut last: Option<u64> = None;
    while let Some((hash, term)) = read_record(&mut reader)? {
        if last == Some(hash) {
            continue;
        }
        last = Some(hash);
        visit(hash, term)?;
        n += 1;
    }
    Ok(n)
}

struct HeapItem {
    hash: u64,
    term: String,
    reader: usize,
}

impl Eq for HeapItem {}
impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.reader == other.reader
    }
}
impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .hash
            .cmp(&self.hash)
            .then_with(|| self.reader.cmp(&other.reader))
    }
}
impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn merge_runs<F>(paths: &[PathBuf], visit: &mut F, collisions: &mut u64) -> io::Result<u64>
where
    F: FnMut(u64, String) -> io::Result<()>,
{
    let mut readers: Vec<BufReader<File>> = Vec::with_capacity(paths.len());
    for path in paths {
        readers.push(BufReader::with_capacity(256 * 1024, File::open(path)?));
    }
    let mut heap = BinaryHeap::new();
    for (idx, reader) in readers.iter_mut().enumerate() {
        if let Some((hash, term)) = read_record(reader)? {
            heap.push(HeapItem {
                hash,
                term,
                reader: idx,
            });
        }
    }
    let mut emitted = 0u64;
    let mut last_hash: Option<u64> = None;
    let mut last_term: Option<String> = None;
    while let Some(item) = heap.pop() {
        if let Some((hash, term)) = read_record(&mut readers[item.reader])? {
            heap.push(HeapItem {
                hash,
                term,
                reader: item.reader,
            });
        }
        if last_hash == Some(item.hash) {
            if last_term.as_deref() != Some(item.term.as_str()) {
                *collisions = collisions.saturating_add(1);
            }
            continue;
        }
        last_hash = Some(item.hash);
        last_term = Some(item.term.clone());
        visit(item.hash, item.term)?;
        emitted += 1;
    }
    Ok(emitted)
}

fn write_merged_runs(inputs: &[PathBuf], output: &Path, collisions: &mut u64) -> io::Result<()> {
    let mut out = BufWriter::new(File::options().create_new(true).write(true).open(output)?);
    merge_runs(
        inputs,
        &mut |hash, term| write_record(&mut out, hash, term.as_bytes()),
        collisions,
    )?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn intern_is_idempotent_and_counts_collisions() {
        let dir = TempDir::new().unwrap();
        let mut spill = LexiconSpill::with_cap(dir.path().to_path_buf(), 1024);
        spill.intern(1, "alpha").unwrap();
        spill.intern(1, "alpha").unwrap();
        assert_eq!(spill.collision_count(), 0);
        spill.intern(1, "beta").unwrap();
        assert_eq!(spill.collision_count(), 1);
        spill.intern(2, "gamma").unwrap();
        assert_eq!(spill.interned_count(), 2);
    }

    #[test]
    fn spill_then_sorted_unique() {
        let dir = TempDir::new().unwrap();
        let mut spill = LexiconSpill::with_cap(dir.path().to_path_buf(), 8);
        spill.intern(30, "ccc").unwrap();
        spill.intern(10, "aaa").unwrap();
        spill.intern(20, "bbb").unwrap();
        spill.intern(10, "aaa").unwrap();
        let mut got = Vec::new();
        let n = spill
            .for_each_sorted(|h, t| {
                got.push((h, t));
                Ok(())
            })
            .unwrap();
        assert_eq!(n, 3);
        assert_eq!(
            got,
            vec![(10, "aaa".into()), (20, "bbb".into()), (30, "ccc".into())]
        );
    }
}
