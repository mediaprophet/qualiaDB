//! Materialized graph quins — append-only projection of checkpointed WAL commits.

use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use qualia_core_db::NQuin;

pub const GRAPH_QUINS_FILE: &str = "wellfair/graph/quins.bin";
pub const MAX_LIST: usize = 512;

pub struct GraphStore {
    path: PathBuf,
    quin_count: usize,
}

impl GraphStore {
    pub fn open(storage_root: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = storage_root.as_ref().join(GRAPH_QUINS_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            OpenOptions::new().create(true).write(true).open(&path)?;
        }
        let quin_count = count_quins(&path)?;
        Ok(Self { path, quin_count })
    }

    pub fn count(&self) -> usize {
        self.quin_count
    }

    pub fn append_quins(&mut self, quins: &[NQuin]) -> std::io::Result<()> {
        if quins.is_empty() {
            return Ok(());
        }
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        for quin in quins {
            let bytes = quin_as_bytes(quin);
            file.write_all(bytes)?;
        }
        file.sync_all()?;
        self.quin_count += quins.len();
        Ok(())
    }

    pub fn list_recent(&self, limit: usize) -> std::io::Result<Vec<NQuin>> {
        let mut file = fs::File::open(&self.path)?;
        let len = file.metadata()?.len();
        let quin_size = std::mem::size_of::<NQuin>() as u64;
        if len < quin_size {
            return Ok(Vec::new());
        }
        let total = (len / quin_size) as usize;
        let keep = limit.min(MAX_LIST).min(total);
        let start = len - (keep as u64 * quin_size);
        file.seek(SeekFrom::Start(start))?;
        let mut buffer = vec![0u8; keep * std::mem::size_of::<NQuin>()];
        file.read_exact(&mut buffer)?;
        Ok(decode_quins(&buffer))
    }
}

fn count_quins(path: &Path) -> std::io::Result<usize> {
    let len = fs::metadata(path)?.len();
    Ok((len as usize) / std::mem::size_of::<NQuin>())
}

fn quin_as_bytes(quin: &NQuin) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            (quin as *const NQuin) as *const u8,
            std::mem::size_of::<NQuin>(),
        )
    }
}

fn decode_quins(buffer: &[u8]) -> Vec<NQuin> {
    let quin_size = std::mem::size_of::<NQuin>();
    buffer
        .chunks_exact(quin_size)
        .map(|chunk| unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const NQuin) })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_store_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = GraphStore::open(dir.path()).unwrap();
        let q = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 4,
            metadata: 5,
            parity: 6,
        };
        store.append_quins(&[q]).unwrap();
        assert_eq!(store.count(), 1);
        let listed = store.list_recent(10).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].subject, 1);
    }
}
