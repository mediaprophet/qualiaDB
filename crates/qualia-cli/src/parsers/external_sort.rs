use qualia_core_db::{QualiaQuin, QUINS_PER_BLOCK};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

// 50MB buffer limit: ~1 million Quins (48 bytes each -> 48MB)
const CHUNK_SIZE_LIMIT: usize = 1_000_000;

pub struct ExternalSorter {
    buffer: Vec<QualiaQuin>,
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

    pub fn push(&mut self, quin: QualiaQuin) -> std::io::Result<()> {
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
        let mut file = BufWriter::new(File::create(&chunk_path)?);

        for q in &self.buffer {
            file.write_all(bytemuck::bytes_of(q))?;
        }
        file.flush()?;

        self.chunk_files.push(chunk_path);
        self.buffer.clear();
        Ok(())
    }

    pub fn merge(mut self, final_q42: &Path, final_bidx: &Path) -> std::io::Result<u64> {
        // Flush any remaining quins
        self.flush_chunk()?;

        let q42_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(final_q42)?;
        let mut q42_out = BufWriter::new(q42_file);

        let bidx_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(final_bidx)?;
        let mut bidx_out = BufWriter::new(bidx_file);

        let mut block_ranges: Vec<(u64, u64)> = Vec::new();

        // If nothing was written
        if self.chunk_files.is_empty() {
            Self::write_bidx_file(&mut bidx_out, &block_ranges)?;
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
            quin: QualiaQuin,
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
        let mut block_seq = 0;

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
                let min_hash = block_buffer.first().unwrap().object;
                let max_hash = block_buffer.last().unwrap().object;
                block_ranges.push((min_hash, max_hash));

                Self::write_superblock(&mut q42_out, block_seq, &block_buffer)?;
                block_buffer.clear();
                block_seq += 1;
            }
        }

        // Flush remaining in block buffer
        if !block_buffer.is_empty() {
            let min_hash = block_buffer.first().unwrap().object;
            let max_hash = block_buffer.last().unwrap().object;
            block_ranges.push((min_hash, max_hash));

            Self::write_superblock(&mut q42_out, block_seq, &block_buffer)?;
            block_seq += 1;
        }

        q42_out.flush()?;

        // Write the .bidx sidecar
        Self::write_bidx_file(&mut bidx_out, &block_ranges)?;

        // Cleanup temp files
        for chunk_path in &self.chunk_files {
            let _ = std::fs::remove_file(chunk_path);
        }

        Ok(block_seq as u64)
    }

    fn read_quin(reader: &mut BufReader<File>) -> std::io::Result<Option<QualiaQuin>> {
        let mut bytes = [0u8; 48];
        match reader.read_exact(&mut bytes) {
            Ok(_) => Ok(Some(*bytemuck::from_bytes(&bytes))),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn write_superblock(
        writer: &mut impl Write,
        seq_id: u64,
        quins: &[QualiaQuin],
    ) -> std::io::Result<()> {
        writer.write_all(&seq_id.to_le_bytes())?;
        writer.write_all(&0u64.to_le_bytes())?;
        writer.write_all(&(quins.len() as u64).to_le_bytes())?;
        writer.write_all(&0u32.to_le_bytes())?;
        writer.write_all(&0u32.to_le_bytes())?;
        writer.write_all(&[0u8; 128])?;

        let zero = [0u8; 48];
        for q in quins {
            writer.write_all(bytemuck::bytes_of(q))?;
        }
        for _ in quins.len()..QUINS_PER_BLOCK {
            writer.write_all(&zero)?;
        }
        Ok(())
    }

    fn write_bidx_file(w: &mut BufWriter<File>, ranges: &[(u64, u64)]) -> std::io::Result<()> {
        w.write_all(b"BIDX")?;
        w.write_all(&1u32.to_le_bytes())?;
        w.write_all(&(ranges.len() as u32).to_le_bytes())?;
        w.write_all(&0u32.to_le_bytes())?;
        for (min, max) in ranges {
            w.write_all(&min.to_le_bytes())?;
            w.write_all(&max.to_le_bytes())?;
        }
        w.flush()?;
        Ok(())
    }
}
