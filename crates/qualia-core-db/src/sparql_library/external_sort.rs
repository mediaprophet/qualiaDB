#[cfg(not(target_arch = "wasm32"))]
use crate::q42_volume::UnifiedVolumeBuilder;
use crate::{NQuin, QUINS_PER_BLOCK};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

// 50MB buffer limit: ~1 million Quins (48 bytes each -> 48MB)
const CHUNK_SIZE_LIMIT: usize = 1_000_000;

pub struct ExternalSorter {
    buffer: Vec<NQuin>,
    chunk_files: Vec<PathBuf>,
    temp_dir: PathBuf,
    total_quins: u64,
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

        let mut builder = UnifiedVolumeBuilder::with_empty_lex();

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
                builder.push_block(block_seq, &block_buffer);
                block_buffer.clear();
                block_seq += 1;
            }
        }

        // Flush remaining in block buffer
        if !block_buffer.is_empty() {
            builder.push_block(block_seq, &block_buffer);
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
    pub fn merge(mut self, _final_q42: &Path) -> std::io::Result<u64> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Not supported on WASM"))
    }


    fn read_quin(reader: &mut BufReader<File>) -> std::io::Result<Option<NQuin>> {
        let mut bytes = [0u8; 48];
        match reader.read_exact(&mut bytes) {
            Ok(_) => Ok(Some(bytemuck::pod_read_unaligned(&bytes))),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => Err(e),
        }
    }
}
