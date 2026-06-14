import re

with open('crates/qualia-core-db/src/q42_volume.rs', 'r', encoding='utf-8') as f:
    content = f.read()

streaming_appender = '''
/// Streaming append-only interface for Q42 Unified Volumes.
/// Allows continuous block accumulation without loading the entire volume in memory.
pub struct StreamingVolumeAppender {
    file: std::fs::File,
    header: Q42VolumeHeader,
    block_ranges: Vec<(u64, u64)>,
    dir_entries: Vec<BlockDirectoryEntry>,
    dag_store: crate::git_bridge::DagStore,
    author_did: u64,
    last_dag_hash: [u8; 32],
}

impl StreamingVolumeAppender {
    pub fn new(path: &std::path::Path) -> std::io::Result<Self> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)?;

        let mut header = Q42VolumeHeader {
            magic: Q42_MAGIC,
            version: Q42_VERSION_V3,
            flags: FLAG_BLOCKS_LZ4,
            lex_offset: HEADER_SIZE as u64,
            lex_length: 0,
            bidx_offset: HEADER_SIZE as u64,
            bidx_length: 0,
            block_dir_offset: HEADER_SIZE as u64,
            block_dir_length: 0,
            data_offset: HEADER_SIZE as u64,
            data_length: 0,
            block_count: 0,
            block_size: SUPERBLOCK_SIZE as u32,
            quins_per_block: QUINS_PER_BLOCK as u32,
            temporal_index_offset: 0,
            temporal_index_length: 0,
            merkle_root: [0; 32],
            assertion_timestamp: 0,
            dag_root_offset: 0,
            dag_root_length: 0,
            _reserved: [0; 96],
        };

        if file.metadata()?.len() >= HEADER_SIZE as u64 {
            let mut hdr_buf = [0u8; HEADER_SIZE];
            use std::io::{Read, Seek, SeekFrom};
            file.seek(SeekFrom::Start(0))?;
            file.read_exact(&mut hdr_buf)?;
            if let Ok(h) = header_from_bytes(&hdr_buf) {
                header = h;
            }
        } else {
            use std::io::{Seek, SeekFrom, Write};
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&header_to_bytes(&header))?;
        }

        Ok(Self {
            file,
            header,
            block_ranges: Vec::new(),
            dir_entries: Vec::new(),
            dag_store: crate::git_bridge::DagStore::new(),
            author_did: 0,
            last_dag_hash: [0u8; 32],
        })
    }

    pub fn with_author_did(mut self, did: u64) -> Self {
        self.author_did = did;
        self
    }

    pub fn append_block(&mut self, seq_id: u64, quins: &[NQuin]) -> std::io::Result<()> {
        let min_hash = quins.first().map(|q| q.object).unwrap_or(0);
        let max_hash = quins.last().map(|q| q.object).unwrap_or(0);
        self.block_ranges.push((min_hash, max_hash));

        let raw = encode_superblock(seq_id, quins);
        let compressed = lz4_flex::compress_prepend_size(&raw);

        use std::io::{Seek, SeekFrom, Write};
        let append_offset = self.header.data_offset + self.header.data_length;
        self.file.seek(SeekFrom::Start(append_offset))?;
        
        self.dir_entries.push(BlockDirectoryEntry {
            rel_offset: self.header.data_length,
            comp_len: compressed.len() as u32,
            uncomp_len: SUPERBLOCK_SIZE as u32,
        });

        self.file.write_all(&compressed)?;
        self.header.data_length += compressed.len() as u64;
        self.header.block_count += 1;

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let msg = format!("runtime block {}", seq_id);

        self.last_dag_hash = if self.last_dag_hash == [0u8; 32] {
            self.dag_store.genesis_node(quins, self.author_did, ts, &msg)
        } else {
            self.dag_store.commit_node(self.last_dag_hash, quins, self.author_did, ts, &msg)
        };

        // Write BIDX and Directory at the end
        let bidx_bytes = encode_bidx(&self.block_ranges);
        self.header.bidx_offset = self.header.data_offset + self.header.data_length;
        self.header.bidx_length = bidx_bytes.len() as u64;
        self.file.write_all(&bidx_bytes)?;

        self.header.block_dir_offset = self.header.bidx_offset + self.header.bidx_length;
        self.header.block_dir_length = (self.dir_entries.len() * BlockDirectoryEntry::SIZE) as u64;
        let mut dir_bytes = Vec::with_capacity(self.header.block_dir_length as usize);
        for entry in &self.dir_entries {
            dir_bytes.extend_from_slice(&entry.rel_offset.to_le_bytes());
            dir_bytes.extend_from_slice(&entry.comp_len.to_le_bytes());
            dir_bytes.extend_from_slice(&entry.uncomp_len.to_le_bytes());
        }
        self.file.write_all(&dir_bytes)?;

        // Write DAG
        let dag_bytes = self.dag_store.serialize();
        self.header.dag_root_offset = self.header.block_dir_offset + self.header.block_dir_length;
        self.header.dag_root_length = dag_bytes.len() as u64;
        self.file.write_all(&dag_bytes)?;

        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(self.last_dag_hash);
        self.header.merkle_root = h.finalize().into();
        self.header.assertion_timestamp = ts;

        // Update header
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&header_to_bytes(&self.header))?;
        self.file.sync_all()?;

        Ok(())
    }
}
'''

if 'pub struct StreamingVolumeAppender' not in content:
    content += '\n' + streaming_appender + '\n'
    with open('crates/qualia-core-db/src/q42_volume.rs', 'w', encoding='utf-8') as f:
        f.write(content)
