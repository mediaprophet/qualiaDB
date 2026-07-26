//! Unified `.q42` v3 volume — lexicon, block index, and LZ4-compressed SuperBlocks in one file.
//!
//! Layout (all little-endian):
//! ```text
//! [0..256)              Q42VolumeHeader
//! [lex_offset ..]       Q42LEX blob (uncompressed)
//! [bidx_offset ..]      BIDX blob (uncompressed)
//! [block_dir_offset ..] block_count × BlockDirectoryEntry (16 bytes each)
//! [data_offset ..]      concatenated LZ4 block payloads (lz4_flex prepend_size)
//! ```
//!
//! Legacy v1 sidecars (`.q42.lex`, `.q42.bidx`) and separate `.c.q42` transport files
//! are deprecated; new ingest writes v3 only.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;

use memmap2::{Mmap, MmapOptions};

use crate::q42_lex::{LexError, LexiconEntry, Q42LexMmap, LEX_MAGIC};
use crate::{NQuin, QUINS_PER_BLOCK};

pub const Q42_MAGIC: [u8; 4] = [0x51, 0x34, 0x32, 0x00]; // "Q42\0"
pub const Q42_VERSION_V3: u16 = 3;
pub const HEADER_SIZE: usize = 256;
pub const SUPERBLOCK_SIZE: usize = 40_960;
pub const SUPERBLOCK_HEADER: usize = 160;
pub const QUIN_SIZE: usize = 48;
pub const BIDX_MAGIC: [u8; 4] = *b"BIDX";
pub const FLAG_BLOCKS_LZ4: u16 = 0x0001;
pub const FLAG_OBJECT_SORTED: u16 = 0x0002;

/// Q42 volume header — 280 bytes, `repr(C, packed)`.
/// v3 builds hard-reject files with `version < 3` — run `q42 migrate meta` first.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct Q42VolumeHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub lex_offset: u64,
    pub lex_length: u64,
    pub bidx_offset: u64,
    pub bidx_length: u64,
    pub block_dir_offset: u64,
    pub block_dir_length: u64,
    pub data_offset: u64,
    pub data_length: u64,
    pub block_count: u64,
    pub block_size: u32,
    pub quins_per_block: u32,
    // v3 extension fields (carved from former _reserved[0..88]):
    pub temporal_index_offset: u64,
    pub temporal_index_length: u64,
    pub merkle_root: [u8; 32], // SHA3-256 of DAG root; all-zero = no history
    pub assertion_timestamp: u64, // ms since Unix epoch when volume was last written
    pub dag_root_offset: u64,  // offset into file of DagNode store; 0 = absent
    pub dag_root_length: u64,  // byte length of DagNode store section

    // Governance / Identity Bifurcation
    pub natural_person_did_offset: u64, // Offset to human-reality declarative/consent DAG
    pub software_agent_did_offset: u64, // Offset to agent-reality logic/policy DAG

    pub _reserved: [u8; 80], // remaining reserved (88 named + 72 v3 ext + 16 gov + 80 = 256 bytes)
}

const _: () = assert!(
    std::mem::size_of::<Q42VolumeHeader>() == 256,
    "Q42VolumeHeader must be exactly 256 bytes — matches HEADER_SIZE constant"
);

impl Q42VolumeHeader {
    /// Reject v2 files. Call before any read/write on a mapped header.
    pub fn verify_version(&self) -> Result<(), String> {
        // Copy fields out of the packed struct before comparing to avoid unaligned refs.
        let magic = self.magic;
        let version = { self.version };
        if magic != Q42_MAGIC {
            return Err(format!("bad magic {magic:?}"));
        }
        if version != Q42_VERSION_V3 {
            return Err(format!("Q42 file is version {version}; strict v3 required"));
        }
        Ok(())
    }

    /// Build a minimal valid v3 header with all extension fields zeroed.
    pub fn new_v3(
        lex_offset: u64,
        lex_length: u64,
        bidx_offset: u64,
        bidx_length: u64,
        block_dir_offset: u64,
        block_dir_length: u64,
        data_offset: u64,
        data_length: u64,
        block_count: u64,
        block_size: u32,
        quins_per_block: u32,
    ) -> Self {
        let assertion_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            magic: Q42_MAGIC,
            version: Q42_VERSION_V3,
            flags: FLAG_BLOCKS_LZ4 | FLAG_OBJECT_SORTED,
            lex_offset,
            lex_length,
            bidx_offset,
            bidx_length,
            block_dir_offset,
            block_dir_length,
            data_offset,
            data_length,
            block_count,
            block_size,
            quins_per_block,
            temporal_index_offset: 0,
            temporal_index_length: 0,
            merkle_root: [0u8; 32],
            assertion_timestamp,
            dag_root_offset: 0,
            dag_root_length: 0,
            natural_person_did_offset: 0,
            software_agent_did_offset: 0,
            _reserved: [0u8; 80],
        }
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct BlockDirectoryEntry {
    pub rel_offset: u64,
    pub comp_len: u32,
    pub uncomp_len: u32,
}

impl BlockDirectoryEntry {
    pub const SIZE: usize = 16;

    pub fn write_to(&self, out: &mut impl Write) -> io::Result<()> {
        out.write_all(&self.rel_offset.to_le_bytes())?;
        out.write_all(&self.comp_len.to_le_bytes())?;
        out.write_all(&self.uncomp_len.to_le_bytes())?;
        Ok(())
    }

    fn from_bytes(buf: &[u8; 16]) -> Self {
        Self {
            rel_offset: u64::from_le_bytes(buf[0..8].try_into().unwrap()),
            comp_len: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
            uncomp_len: u32::from_le_bytes(buf[12..16].try_into().unwrap()),
        }
    }
}

/// Upgrade a v1/v2 `.q42` file to v3 by rewriting the version field and zeroing
/// the new extension fields in the 256-byte header.  Data blocks are untouched.
pub fn migrate_v2_to_v3(path: &Path) -> io::Result<()> {
    use std::io::{Seek, SeekFrom};
    let mut f = OpenOptions::new().read(true).write(true).open(path)?;
    let mut header = [0u8; HEADER_SIZE];
    f.read_exact(&mut header)?;
    if &header[0..4] != &Q42_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not a Q42 volume",
        ));
    }
    let version = u16::from_le_bytes([header[4], header[5]]);
    if version >= Q42_VERSION_V3 as u16 {
        return Ok(()); // already v3
    }
    // Bump version to 3.
    header[4..6].copy_from_slice(&(Q42_VERSION_V3 as u16).to_le_bytes());
    // Zero out the v3 extension fields (bytes 88..256 within the header).
    header[88..256].fill(0);
    f.seek(SeekFrom::Start(0))?;
    f.write_all(&header)?;
    f.flush()
}

/// Returns true if `path` begins with a unified volume header.
pub fn is_unified_volume(path: &Path) -> io::Result<bool> {
    let mut f = File::open(path)?;
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)?;
    Ok(magic == Q42_MAGIC)
}

/// Encode Q42LEX bytes from a hash → string map.
pub fn encode_lex(lex: &HashMap<u64, String>) -> Vec<u8> {
    // Single source of truth for the Q42LEX write format. `serialize_string_lexicon` is UTF-8 and
    // truncates over-long literals at a CHARACTER boundary (never mid-codepoint), so multilingual
    // literals round-trip byte-intact — a plain `b.len().min(65535)` byte cut could split a codepoint
    // and produce invalid UTF-8 that the reader then drops.
    crate::q42_lex::serialize_string_lexicon(lex)
}

/// Encode Q42LEX bytes from a hash → LexiconEntry map (supports embedded triples).
pub fn encode_lex_with_entries(lex: &HashMap<u64, LexiconEntry>) -> Vec<u8> {
    let mut entries: Vec<(u64, &LexiconEntry)> = lex.iter().map(|(&h, e)| (h, e)).collect();
    entries.sort_unstable_by_key(|&(h, _)| h);

    let entry_count = entries.len() as u64;
    let strings_offset = 32 + entry_count * 16;

    let mut string_blob: Vec<u8> = Vec::new();
    let mut index = Vec::with_capacity(entries.len() * 16);
    for (hash, entry) in &entries {
        let str_off = string_blob.len() as u64;
        match entry {
            LexiconEntry::String(text) => {
                // Write type tag
                string_blob.push(0x01);
                let b = text.as_bytes();
                let len = b.len().min(65535) as u16;
                string_blob.extend_from_slice(&len.to_le_bytes());
                string_blob.extend_from_slice(&b[..len as usize]);
            }
            LexiconEntry::EmbeddedTriple(triple) => {
                // Write type tag
                string_blob.push(0x02);
                for &id in triple {
                    string_blob.extend_from_slice(&id.to_le_bytes());
                }
            }
            LexiconEntry::Webizen(webid) => {
                // Write type tag
                string_blob.push(0x03);
                let b = webid.as_bytes();
                let len = b.len().min(65535) as u16;
                string_blob.extend_from_slice(&len.to_le_bytes());
                string_blob.extend_from_slice(&b[..len as usize]);
            }
        }
        index.extend_from_slice(&hash.to_le_bytes());
        index.extend_from_slice(&str_off.to_le_bytes());
    }

    let mut out = Vec::with_capacity(strings_offset as usize + string_blob.len());
    out.extend_from_slice(&LEX_MAGIC);
    out.extend_from_slice(&entry_count.to_le_bytes());
    out.extend_from_slice(&strings_offset.to_le_bytes());
    out.extend_from_slice(&1u64.to_le_bytes());
    out.extend_from_slice(&index);
    out.extend_from_slice(&string_blob);
    out
}

/// Encode BIDX bytes from per-block min/max object hashes.
pub fn encode_bidx(ranges: &[(u64, u64)]) -> Vec<u8> {
    let block_count = ranges.len() as u32;
    let mut out = Vec::with_capacity(16 + ranges.len() * 16);
    out.extend_from_slice(&BIDX_MAGIC);
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&block_count.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for (min, max) in ranges {
        out.extend_from_slice(&min.to_le_bytes());
        out.extend_from_slice(&max.to_le_bytes());
    }
    out
}

/// Encode Q42LEX bytes from a hash → LexiconEntry map (supports embedded triples).
pub fn encode_superblock(seq_id: u64, quins: &[NQuin]) -> [u8; SUPERBLOCK_SIZE] {
    debug_assert!(quins.len() <= QUINS_PER_BLOCK);
    let mut block = [0u8; SUPERBLOCK_SIZE];
    block[0..8].copy_from_slice(&seq_id.to_le_bytes());
    block[16..24].copy_from_slice(&(quins.len() as u64).to_le_bytes());
    let zero = [0u8; QUIN_SIZE];
    let mut off = SUPERBLOCK_HEADER;
    for q in quins {
        block[off..off + QUIN_SIZE].copy_from_slice(bytemuck::bytes_of(q));
        off += QUIN_SIZE;
    }
    for _ in quins.len()..QUINS_PER_BLOCK {
        block[off..off + QUIN_SIZE].copy_from_slice(&zero);
        off += QUIN_SIZE;
    }
    block
}

pub fn header_to_bytes(h: &Q42VolumeHeader) -> [u8; HEADER_SIZE] {
    let mut buf = [0u8; HEADER_SIZE];
    // Core fields (0..88)
    buf[0..4].copy_from_slice(&h.magic);
    buf[4..6].copy_from_slice(&h.version.to_le_bytes());
    buf[6..8].copy_from_slice(&h.flags.to_le_bytes());
    buf[8..16].copy_from_slice(&h.lex_offset.to_le_bytes());
    buf[16..24].copy_from_slice(&h.lex_length.to_le_bytes());
    buf[24..32].copy_from_slice(&h.bidx_offset.to_le_bytes());
    buf[32..40].copy_from_slice(&h.bidx_length.to_le_bytes());
    buf[40..48].copy_from_slice(&h.block_dir_offset.to_le_bytes());
    buf[48..56].copy_from_slice(&h.block_dir_length.to_le_bytes());
    buf[56..64].copy_from_slice(&h.data_offset.to_le_bytes());
    buf[64..72].copy_from_slice(&h.data_length.to_le_bytes());
    buf[72..80].copy_from_slice(&h.block_count.to_le_bytes());
    buf[80..84].copy_from_slice(&h.block_size.to_le_bytes());
    buf[84..88].copy_from_slice(&h.quins_per_block.to_le_bytes());
    // v3 extension fields (88..160)
    buf[88..96].copy_from_slice(&h.temporal_index_offset.to_le_bytes());
    buf[96..104].copy_from_slice(&h.temporal_index_length.to_le_bytes());
    buf[104..136].copy_from_slice(&h.merkle_root);
    buf[136..144].copy_from_slice(&h.assertion_timestamp.to_le_bytes());
    buf[144..152].copy_from_slice(&h.dag_root_offset.to_le_bytes());
    buf[152..160].copy_from_slice(&h.dag_root_length.to_le_bytes());
    buf
}

fn header_from_bytes(buf: &[u8; HEADER_SIZE]) -> io::Result<Q42VolumeHeader> {
    if buf[0..4] != Q42_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid Q42 magic",
        ));
    }
    let version = u16::from_le_bytes(buf[4..6].try_into().unwrap());
    if version != Q42_VERSION_V3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Q42 file is version {version}; strict v3 required"),
        ));
    }
    Ok(Q42VolumeHeader {
        magic: Q42_MAGIC,
        version,
        flags: u16::from_le_bytes(buf[6..8].try_into().unwrap()),
        lex_offset: u64::from_le_bytes(buf[8..16].try_into().unwrap()),
        lex_length: u64::from_le_bytes(buf[16..24].try_into().unwrap()),
        bidx_offset: u64::from_le_bytes(buf[24..32].try_into().unwrap()),
        bidx_length: u64::from_le_bytes(buf[32..40].try_into().unwrap()),
        block_dir_offset: u64::from_le_bytes(buf[40..48].try_into().unwrap()),
        block_dir_length: u64::from_le_bytes(buf[48..56].try_into().unwrap()),
        data_offset: u64::from_le_bytes(buf[56..64].try_into().unwrap()),
        data_length: u64::from_le_bytes(buf[64..72].try_into().unwrap()),
        block_count: u64::from_le_bytes(buf[72..80].try_into().unwrap()),
        block_size: u32::from_le_bytes(buf[80..84].try_into().unwrap()),
        quins_per_block: u32::from_le_bytes(buf[84..88].try_into().unwrap()),
        temporal_index_offset: u64::from_le_bytes(buf[88..96].try_into().unwrap()),
        temporal_index_length: u64::from_le_bytes(buf[96..104].try_into().unwrap()),
        merkle_root: buf[104..136].try_into().unwrap(),
        assertion_timestamp: u64::from_le_bytes(buf[136..144].try_into().unwrap()),
        dag_root_offset: u64::from_le_bytes(buf[144..152].try_into().unwrap()),
        dag_root_length: u64::from_le_bytes(buf[152..160].try_into().unwrap()),
        natural_person_did_offset: u64::from_le_bytes(buf[160..168].try_into().unwrap()),
        software_agent_did_offset: u64::from_le_bytes(buf[168..176].try_into().unwrap()),
        _reserved: buf[176..256].try_into().unwrap(),
    })
}

/// Write a unified v3 `.q42` volume.
pub fn write_unified_volume(
    path: &Path,
    lex: &HashMap<u64, String>,
    block_ranges: &[(u64, u64)],
    blocks: &[Vec<NQuin>],
) -> io::Result<()> {
    if blocks.len() != block_ranges.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "block count mismatch",
        ));
    }
    let mut builder = UnifiedVolumeBuilder::with_lex_map(lex);
    for (seq, quins) in blocks.iter().enumerate() {
        builder.push_block(seq as u64, quins);
    }
    builder.finish(path)
}

/// Write a unified v3 .q42 volume with embedded triple support.
///
/// Accepts `HashMap<u64, LexiconEntry>` to support SPARQL-Star embedded triples.
pub fn write_unified_volume_with_entries(
    path: &Path,
    lex: &HashMap<u64, LexiconEntry>,
    block_ranges: &[(u64, u64)],
    blocks: &[Vec<NQuin>],
) -> io::Result<()> {
    if blocks.len() != block_ranges.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "block count mismatch",
        ));
    }
    let mut builder = UnifiedVolumeBuilder::with_lex_entries(lex);
    for (seq, quins) in blocks.iter().enumerate() {
        builder.push_block(seq as u64, quins);
    }
    builder.finish(path)
}

/// Incremental builder for large external-sort merges (one SuperBlock at a time).
pub struct UnifiedVolumeBuilder {
    lex_bytes: Vec<u8>,
    block_ranges: Vec<(u64, u64)>,
    dir_entries: Vec<BlockDirectoryEntry>,
    data_blob: Vec<u8>,
    /// Merkle-DAG commit history — populated as each SuperBlock is pushed.
    dag_store: crate::git_bridge::DagStore,
    /// DID hash of the agent performing this ingest (0 = system/anonymous).
    author_did: u64,
    /// Hash of the last committed DagNode; all-zero until first push.
    last_dag_hash: [u8; 32],
}

impl UnifiedVolumeBuilder {
    pub fn with_lex_map(lex: &HashMap<u64, String>) -> Self {
        Self {
            lex_bytes: encode_lex(lex),
            block_ranges: Vec::new(),
            dir_entries: Vec::new(),
            data_blob: Vec::new(),
            dag_store: crate::git_bridge::DagStore::new(),
            author_did: 0,
            last_dag_hash: [0u8; 32],
        }
    }

    /// Create a builder with a lexicon that supports embedded triples (LexiconEntry).
    pub fn with_lex_entries(lex: &HashMap<u64, LexiconEntry>) -> Self {
        Self {
            lex_bytes: encode_lex_with_entries(lex),
            block_ranges: Vec::new(),
            dir_entries: Vec::new(),
            data_blob: Vec::new(),
            dag_store: crate::git_bridge::DagStore::new(),
            author_did: 0,
            last_dag_hash: [0u8; 32],
        }
    }

    pub fn with_empty_lex() -> Self {
        Self::with_lex_map(&HashMap::new())
    }

    /// Set the author DID for DAG commit nodes (optional; defaults to 0 = system).
    pub fn with_author_did(mut self, did: u64) -> Self {
        self.author_did = did;
        self
    }

    pub fn push_block(&mut self, seq_id: u64, quins: &[NQuin]) {
        let min_hash = quins.first().map(|q| q.object).unwrap_or(0);
        let max_hash = quins.last().map(|q| q.object).unwrap_or(0);
        self.block_ranges.push((min_hash, max_hash));
        let raw = encode_superblock(seq_id, quins);
        let compressed = lz4_flex::compress_prepend_size(&raw);
        self.dir_entries.push(BlockDirectoryEntry {
            rel_offset: self.data_blob.len() as u64,
            comp_len: compressed.len() as u32,
            uncomp_len: SUPERBLOCK_SIZE as u32,
        });
        self.data_blob.extend_from_slice(&compressed);

        // Commit this SuperBlock to the Merkle-DAG.
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let msg = format!("ingest block {seq_id}");
        self.last_dag_hash = if self.last_dag_hash == [0u8; 32] {
            self.dag_store
                .genesis_node(quins, self.author_did, ts, &msg)
        } else {
            self.dag_store
                .commit_node(self.last_dag_hash, quins, self.author_did, ts, &msg)
        };
    }

    pub fn block_count(&self) -> u64 {
        self.block_ranges.len() as u64
    }

    pub fn finish(self, path: &Path) -> io::Result<()> {
        let bytes = self.finish_to_bytes();
        let out = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        let mut w = BufWriter::new(out);
        w.write_all(&bytes)?;
        w.flush()?;
        Ok(())
    }

    /// Serialise the unified v3 `.q42` volume to an in-memory byte buffer.
    ///
    /// Byte-for-byte the same volume [`finish`](Self::finish) writes to disk, but
    /// returned as bytes for callers that embed a `.q42` *inside another container*
    /// — e.g. a [`crate::bundle`] `.hmc` pack — or ship it over the wire without
    /// touching the filesystem. `finish` is exactly this plus a file write.
    pub fn finish_to_bytes(self) -> Vec<u8> {
        let bidx_bytes = encode_bidx(&self.block_ranges);
        let block_count = self.block_ranges.len() as u64;

        let lex_offset = HEADER_SIZE as u64;
        let bidx_offset = lex_offset + self.lex_bytes.len() as u64;
        let block_dir_offset = bidx_offset + bidx_bytes.len() as u64;
        let block_dir_length = block_count * BlockDirectoryEntry::SIZE as u64;
        let data_offset = block_dir_offset + block_dir_length;

        let assertion_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Serialize the Merkle-DAG and compute layout.
        let dag_bytes = self.dag_store.serialize();
        let dag_root_offset = if dag_bytes.is_empty() {
            0
        } else {
            data_offset + self.data_blob.len() as u64
        };
        let dag_root_length = dag_bytes.len() as u64;

        // merkle_root = SHA-256 of last committed DagNode hash (all-zero if no blocks).
        let merkle_root = if self.last_dag_hash == [0u8; 32] {
            [0u8; 32]
        } else {
            // Re-hash the tip hash so the header field is a hash-of-hash, not the raw node hash.
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(self.last_dag_hash);
            h.finalize().into()
        };

        let header = Q42VolumeHeader {
            magic: Q42_MAGIC,
            version: Q42_VERSION_V3,
            flags: FLAG_BLOCKS_LZ4 | FLAG_OBJECT_SORTED,
            lex_offset,
            lex_length: self.lex_bytes.len() as u64,
            bidx_offset,
            bidx_length: bidx_bytes.len() as u64,
            block_dir_offset,
            block_dir_length,
            data_offset,
            data_length: self.data_blob.len() as u64,
            block_count,
            block_size: SUPERBLOCK_SIZE as u32,
            quins_per_block: QUINS_PER_BLOCK as u32,
            temporal_index_offset: 0,
            temporal_index_length: 0,
            merkle_root,
            assertion_timestamp,
            dag_root_offset,
            dag_root_length,
            natural_person_did_offset: 0,
            software_agent_did_offset: 0,
            _reserved: [0; 80],
        };

        let mut out = Vec::with_capacity(
            HEADER_SIZE
                + self.lex_bytes.len()
                + bidx_bytes.len()
                + block_dir_length as usize
                + self.data_blob.len()
                + dag_bytes.len(),
        );
        out.extend_from_slice(&header_to_bytes(&header));
        out.extend_from_slice(&self.lex_bytes);
        out.extend_from_slice(&bidx_bytes);
        for entry in &self.dir_entries {
            // Writing to a `Vec<u8>` is infallible.
            entry.write_to(&mut out).expect("Vec<u8> write cannot fail");
        }
        out.extend_from_slice(&self.data_blob);
        if !dag_bytes.is_empty() {
            out.extend_from_slice(&dag_bytes);
        }
        out
    }
}

/// Memory-mapped unified v2 volume reader.
pub struct Q42Volume {
    mmap: Mmap,
    header: Q42VolumeHeader,
}

impl Q42Volume {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        if mmap.len() < HEADER_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "file too small for Q42 header",
            ));
        }
        let mut hdr_buf = [0u8; HEADER_SIZE];
        hdr_buf.copy_from_slice(&mmap[0..HEADER_SIZE]);
        let header = header_from_bytes(&hdr_buf)?;
        let end = header.data_offset.saturating_add(header.data_length);
        if end as usize > mmap.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Q42 volume truncated",
            ));
        }
        Ok(Self { mmap, header })
    }

    pub fn header(&self) -> &Q42VolumeHeader {
        &self.header
    }

    pub fn lex_bytes(&self) -> &[u8] {
        let start = self.header.lex_offset as usize;
        let end = start + self.header.lex_length as usize;
        &self.mmap[start..end]
    }

    pub fn lex_view(&self) -> Result<Q42LexMmap<'_>, LexError> {
        Q42LexMmap::from_bytes(self.lex_bytes())
    }

    pub fn bidx_bytes(&self) -> &[u8] {
        let start = self.header.bidx_offset as usize;
        let end = start + self.header.bidx_length as usize;
        &self.mmap[start..end]
    }

    pub fn block_count(&self) -> u64 {
        self.header.block_count
    }

    /// Binary-search BIDX for `object_hash`; returns block indices that may contain it.
    pub fn bidx_blocks_for_hash(&self, object_hash: u64) -> Vec<usize> {
        bidx_blocks_for_hash(self.bidx_bytes(), object_hash)
    }

    pub fn block_directory_entry(&self, index: usize) -> io::Result<BlockDirectoryEntry> {
        if index >= self.header.block_count as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "block index out of range",
            ));
        }
        let start = self.header.block_dir_offset as usize + index * BlockDirectoryEntry::SIZE;
        let end = start + BlockDirectoryEntry::SIZE;
        let mut buf = [0u8; 16];
        buf.copy_from_slice(&self.mmap[start..end]);
        Ok(BlockDirectoryEntry::from_bytes(&buf))
    }

    /// Decompress SuperBlock `index` into `out` (must be >= 40960 bytes).
    pub fn read_superblock_into(&self, index: usize, out: &mut [u8]) -> io::Result<usize> {
        if out.len() < SUPERBLOCK_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "output buffer too small",
            ));
        }
        let entry = self.block_directory_entry(index)?;
        let start = self.header.data_offset as usize + entry.rel_offset as usize;
        let end = start + entry.comp_len as usize;
        let compressed = &self.mmap[start..end];
        let decoded = lz4_flex::decompress_size_prepended(compressed).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("LZ4 decompress block {index}: {e}"),
            )
        })?;
        let n = decoded.len().min(SUPERBLOCK_SIZE);
        out[..n].copy_from_slice(&decoded[..n]);
        Ok(n)
    }

    /// Read every live Quin from this volume's SuperBlocks into a heap `Vec`.
    ///
    /// Cold path (CLI / daemon vault load). A `.q42` is **not** a flat array of
    /// 48-byte records — each SuperBlock is LZ4-compressed and carries a 160-byte
    /// header whose bytes `[16..24]` hold the block's live quin count. Callers that
    /// `bytemuck::cast_slice` the raw file will panic (`OutputSliceWouldHaveSlop`)
    /// because the file length is not a multiple of `QUIN_SIZE`. Use this instead.
    pub fn read_all_quins(&self) -> io::Result<Vec<crate::NQuin>> {
        let rd = |b: &[u8], o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap());
        let mut out = Vec::new();
        let mut sb = vec![0u8; SUPERBLOCK_SIZE];
        for i in 0..self.block_count() as usize {
            self.read_superblock_into(i, &mut sb)?;
            let quin_count = rd(&sb, 16) as usize;
            for k in 0..quin_count {
                let o = SUPERBLOCK_HEADER + k * QUIN_SIZE;
                if o + QUIN_SIZE > sb.len() {
                    break;
                }
                out.push(crate::NQuin {
                    subject: rd(&sb, o),
                    predicate: rd(&sb, o + 8),
                    object: rd(&sb, o + 16),
                    context: rd(&sb, o + 24),
                    metadata: rd(&sb, o + 32),
                    parity: rd(&sb, o + 40),
                });
            }
        }
        Ok(out)
    }
}

/// BIDX binary search (shared with sidecar format).
pub fn bidx_blocks_for_hash(bidx: &[u8], object_hash: u64) -> Vec<usize> {
    if bidx.len() < 16 || bidx[0..4] != BIDX_MAGIC {
        return Vec::new();
    }
    let block_count = u32::from_le_bytes(bidx[8..12].try_into().unwrap()) as usize;
    let mut hits = Vec::new();
    let mut lo = 0usize;
    let mut hi = block_count;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let off = 16 + mid * 16;
        if off + 16 > bidx.len() {
            break;
        }
        let min_h = u64::from_le_bytes(bidx[off..off + 8].try_into().unwrap());
        let max_h = u64::from_le_bytes(bidx[off + 8..off + 16].try_into().unwrap());
        if object_hash < min_h {
            hi = mid;
        } else if object_hash > max_h {
            lo = mid + 1;
        } else {
            hits.push(mid);
            break;
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mini_parser::hash_token;
    use tempfile::NamedTempFile;

    fn sample_quin(subj: &str, pred: &str, obj: &str) -> (NQuin, HashMap<u64, String>) {
        let mut lex = HashMap::new();
        let sh = hash_token(subj);
        let ph = hash_token(pred);
        let oh = hash_token(obj);
        lex.insert(sh, subj.to_string());
        lex.insert(ph, pred.to_string());
        lex.insert(oh, obj.to_string());
        let q = NQuin {
            subject: sh,
            predicate: ph,
            object: oh,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        (q, lex)
    }

    #[test]
    fn unified_volume_roundtrip() {
        let (q1, mut lex) = sample_quin("Patient", "fever", "True");
        let (q2, lex2) = sample_quin("Patient", "has", "pain");
        lex.extend(lex2);

        let mut blocks = vec![vec![q1], vec![q2]];
        blocks.sort_by_key(|chunk| chunk[0].object);

        let tmp = NamedTempFile::new().unwrap();
        let ranges: Vec<_> = blocks
            .iter()
            .map(|chunk| {
                let h = chunk[0].object;
                (h, h)
            })
            .collect();
        write_unified_volume(tmp.path(), &lex, &ranges, &blocks).unwrap();

        let vol = Q42Volume::open(tmp.path()).unwrap();
        assert_eq!(vol.block_count(), 2);
        assert!(vol.lex_view().unwrap().lookup_hash(q1.object).is_some());

        let hits = vol.bidx_blocks_for_hash(q1.object);
        assert!(!hits.is_empty(), "bidx miss for object hash {}", q1.object);

        let mut block = [0u8; SUPERBLOCK_SIZE];
        vol.read_superblock_into(hits[0], &mut block).unwrap();
        let active = u64::from_le_bytes(block[16..24].try_into().unwrap());
        assert_eq!(active, 1);
    }

    /// Regression: `read_all_quins` must reconstruct every stored Quin across
    /// SuperBlocks. A `.q42` is not a flat 48-byte array — the CLI query path used
    /// to `bytemuck::cast_slice` the raw file and panic (`OutputSliceWouldHaveSlop`)
    /// because the file length is not a multiple of `QUIN_SIZE`.
    #[test]
    fn read_all_quins_roundtrip() {
        let (q1, mut lex) = sample_quin("Alice", "rdf:type", "values:NaturalPerson");
        let (q2, lex2) = sample_quin("Bob", "values:claims", "values:Right");
        let (q3, lex3) = sample_quin("AcmeCorp", "rdf:type", "values:CorporatePerson");
        lex.extend(lex2);
        lex.extend(lex3);

        let mut blocks = vec![vec![q1], vec![q2], vec![q3]];
        blocks.sort_by_key(|chunk| chunk[0].object);
        let ranges: Vec<_> = blocks.iter().map(|c| (c[0].object, c[0].object)).collect();

        let tmp = NamedTempFile::new().unwrap();
        write_unified_volume(tmp.path(), &lex, &ranges, &blocks).unwrap();

        let vol = Q42Volume::open(tmp.path()).unwrap();
        let quins = vol.read_all_quins().unwrap();
        assert_eq!(
            quins.len(),
            3,
            "read_all_quins must return every stored quin"
        );
        for q in [q1, q2, q3] {
            assert!(
                quins.iter().any(|r| r.subject == q.subject
                    && r.predicate == q.predicate
                    && r.object == q.object),
                "read_all_quins dropped a quin: {q:?}"
            );
        }
    }

    /// The streaming ingest path (`turtle_doc` → `ExternalSorter` → `merge`) must embed a
    /// populated Q42LEX section at the FRONT of the `.q42` — no separate `.lex` sidecar —
    /// so literal text and IRIs are recoverable from the volume alone.
    #[test]
    fn streaming_ingest_embeds_recoverable_lex() {
        use crate::external_sort::ExternalSorter;
        use crate::lexicon::generate_60bit_token;
        use crate::sparql_library::parsers::turtle_doc::parse_turtle_doc_into;

        let doc = r#"
@prefix dc:     <http://purl.org/dc/terms/> .
@prefix values: <https://ns.webcivics.net/values/> .
@prefix doc:    <https://ns.webcivics.net/values/inst#> .
doc:article-1 a values:Undertaking ;
    dc:title "Article 1" ;
    values:originalText "Each Member undertakes to suppress forced labour." .
"#;
        let tmp_dir = std::env::temp_dir().join(format!("q42_lex_rt_{}", std::process::id()));
        let mut sorter = ExternalSorter::new(tmp_dir);
        parse_turtle_doc_into(doc.as_bytes(), 0, &mut sorter).unwrap();
        let out = NamedTempFile::new().unwrap();
        sorter.merge(out.path()).unwrap();

        let vol = Q42Volume::open(out.path()).unwrap();
        let lex = vol
            .lex_view()
            .expect("volume must carry an embedded front-of-file Q42LEX section");

        // The verbatim literal is recoverable from the .q42 alone — the CML prerequisite.
        let lit = "Each Member undertakes to suppress forced labour.";
        assert_eq!(
            lex.lookup_hash(generate_60bit_token(lit.as_bytes())),
            Some(lit)
        );
        // Expanded IRIs are recoverable too (queries become human-readable).
        let undertaking = "https://ns.webcivics.net/values/Undertaking";
        assert_eq!(
            lex.lookup_hash(generate_60bit_token(undertaking.as_bytes())),
            Some(undertaking)
        );
    }

    /// `finish_to_bytes` must yield the same recoverable volume as `finish` writes to disk —
    /// the in-memory path used to embed a `.q42` inside a `.hmc` bundle.
    #[test]
    fn finish_to_bytes_matches_a_readable_on_disk_volume() {
        let (q1, mut lex) = sample_quin("Heart", "geo:bodySystem", "circulatory");
        let (q2, lex2) = sample_quin("Lung", "geo:bodySystem", "respiratory");
        lex.extend(lex2);
        let mut blocks = vec![vec![q1], vec![q2]];
        blocks.sort_by_key(|c| c[0].object);

        let mut builder = UnifiedVolumeBuilder::with_lex_map(&lex);
        for (seq, chunk) in blocks.iter().enumerate() {
            builder.push_block(seq as u64, chunk);
        }
        let bytes = builder.finish_to_bytes();

        assert!(bytes.starts_with(&Q42_MAGIC), "bytes are a Q42 volume");
        // The bytes open as a valid unified volume with every fact recoverable.
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &bytes).unwrap();
        let vol = Q42Volume::open(tmp.path()).unwrap();
        assert_eq!(vol.block_count(), 2);
        let quins = vol.read_all_quins().unwrap();
        assert_eq!(quins.len(), 2);
        let lexv = vol.lex_view().unwrap();
        let vals: Vec<&str> = quins
            .iter()
            .filter_map(|q| lexv.lookup_hash(q.object))
            .collect();
        assert!(vals.contains(&"circulatory") && vals.contains(&"respiratory"));
    }

    #[test]
    fn v2_magic_detected() {
        let (q, lex) = sample_quin("a", "b", "c");
        let tmp = NamedTempFile::new().unwrap();
        write_unified_volume(tmp.path(), &lex, &[(q.object, q.object)], &[vec![q]]).unwrap();
        assert!(is_unified_volume(tmp.path()).unwrap());
    }
}

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
            natural_person_did_offset: 0,
            software_agent_did_offset: 0,
            _reserved: [0; 80],
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
            self.dag_store
                .genesis_node(quins, self.author_did, ts, &msg)
        } else {
            self.dag_store
                .commit_node(self.last_dag_hash, quins, self.author_did, ts, &msg)
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
