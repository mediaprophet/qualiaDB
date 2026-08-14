//! Cold, bounded-memory unified-Q42 writer for large sorted streams.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::super::{
    encode_lex, encode_superblock, header_to_bytes, BlockDirectoryEntry, Q42VolumeHeader,
    FLAG_BLOCKS_LZ4, FLAG_OBJECT_SORTED, HEADER_SIZE, Q42_VERSION_V3, QUINS_PER_BLOCK,
    SUPERBLOCK_SIZE,
};
use crate::NQuin;

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

/// Writes a sorted Q42 segment without retaining its compressed payload or
/// per-block metadata in heap collections. Temporary streams are RAII-owned.
pub struct StreamingQ42VolumeWriter {
    _temp: TempDir,
    bidx_path: PathBuf,
    directory_path: PathBuf,
    data_path: PathBuf,
    bidx: BufWriter<File>,
    directory: BufWriter<File>,
    data: BufWriter<File>,
    lex_bytes: Vec<u8>,
    block_count: u64,
    data_length: u64,
    last_object_hash: Option<u64>,
}

impl StreamingQ42VolumeWriter {
    pub fn new(lexicon: &HashMap<u64, String>) -> io::Result<Self> {
        let temp = TempDir::new()?;
        let bidx_path = temp.path().join("bidx.entries");
        let directory_path = temp.path().join("block.directory");
        let data_path = temp.path().join("blocks.lz4");
        let open = |path: &Path| OpenOptions::new().create_new(true).write(true).open(path);
        Ok(Self {
            bidx: BufWriter::new(open(&bidx_path)?),
            directory: BufWriter::new(open(&directory_path)?),
            data: BufWriter::new(open(&data_path)?),
            _temp: temp,
            bidx_path,
            directory_path,
            data_path,
            lex_bytes: encode_lex(lexicon)
                .map_err(|error| invalid(format!("invalid Q42LEX: {error:?}")))?,
            block_count: 0,
            data_length: 0,
            last_object_hash: None,
        })
    }

    pub fn push_block(&mut self, seq_id: u64, quins: &[NQuin]) -> io::Result<()> {
        let Some(first) = quins.first() else {
            return Err(invalid("Q42 SuperBlock must not be empty"));
        };
        if quins.len() > QUINS_PER_BLOCK {
            return Err(invalid("Q42 SuperBlock exceeds Quin capacity"));
        }
        let mut previous = first.object;
        for quin in &quins[1..] {
            if quin.object < previous {
                return Err(invalid("Q42 SuperBlock is not object-sorted"));
            }
            previous = quin.object;
        }
        if self
            .last_object_hash
            .is_some_and(|last| first.object < last)
        {
            return Err(invalid("Q42 blocks are not globally object-sorted"));
        }
        let compressed = lz4_flex::compress_prepend_size(&encode_superblock(seq_id, quins));
        let compressed_len = u32::try_from(compressed.len())
            .map_err(|_| invalid("compressed Q42 block exceeds u32"))?;
        self.bidx.write_all(&first.object.to_le_bytes())?;
        self.bidx.write_all(&previous.to_le_bytes())?;
        BlockDirectoryEntry {
            rel_offset: self.data_length,
            comp_len: compressed_len,
            uncomp_len: SUPERBLOCK_SIZE as u32,
        }
        .write_to(&mut self.directory)?;
        self.data.write_all(&compressed)?;
        self.data_length = self
            .data_length
            .checked_add(compressed.len() as u64)
            .ok_or_else(|| invalid("Q42 data length overflow"))?;
        self.block_count += 1;
        self.last_object_hash = Some(previous);
        Ok(())
    }

    pub fn block_count(&self) -> u64 {
        self.block_count
    }

    /// The exact final length if the writer were finished now. It includes the
    /// front matter and fixed BIDX/directory records, not just compressed data.
    pub fn estimated_final_length(&self) -> io::Result<u64> {
        let bidx_length = 16u64
            .checked_add(
                self.block_count
                    .checked_mul(16)
                    .ok_or_else(|| invalid("Q42 BIDX length overflow"))?,
            )
            .ok_or_else(|| invalid("Q42 BIDX length overflow"))?;
        let directory_length = self
            .block_count
            .checked_mul(BlockDirectoryEntry::SIZE as u64)
            .ok_or_else(|| invalid("Q42 directory length overflow"))?;
        (HEADER_SIZE as u64)
            .checked_add(self.lex_bytes.len() as u64)
            .and_then(|value| value.checked_add(bidx_length))
            .and_then(|value| value.checked_add(directory_length))
            .and_then(|value| value.checked_add(self.data_length))
            .ok_or_else(|| invalid("Q42 final length overflow"))
    }

    /// A safe upper bound for the final length after one additional block.
    /// It lets volume publishers split before a block crosses their byte cap
    /// without retaining compressed payloads.
    pub fn maximum_final_length_after_next_block(&self) -> io::Result<u64> {
        self.estimated_final_length()?
            .checked_add(32) // one BIDX interval plus one directory entry
            .and_then(|value| {
                value.checked_add(crate::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE as u64)
            })
            .ok_or_else(|| invalid("Q42 final length overflow"))
    }

    pub fn finish(mut self, path: &Path) -> io::Result<()> {
        self.bidx.flush()?;
        self.directory.flush()?;
        self.data.flush()?;
        let bidx_length = 16
            + self
                .block_count
                .checked_mul(16)
                .ok_or_else(|| invalid("Q42 BIDX length overflow"))?;
        let directory_length = self
            .block_count
            .checked_mul(BlockDirectoryEntry::SIZE as u64)
            .ok_or_else(|| invalid("Q42 directory length overflow"))?;
        let lex_offset = HEADER_SIZE as u64;
        let bidx_offset = lex_offset + self.lex_bytes.len() as u64;
        let directory_offset = bidx_offset + bidx_length;
        let data_offset = directory_offset + directory_length;
        let header = Q42VolumeHeader {
            magic: super::super::Q42_MAGIC,
            version: Q42_VERSION_V3,
            flags: FLAG_BLOCKS_LZ4 | FLAG_OBJECT_SORTED,
            lex_offset,
            lex_length: self.lex_bytes.len() as u64,
            bidx_offset,
            bidx_length,
            block_dir_offset: directory_offset,
            block_dir_length: directory_length,
            data_offset,
            data_length: self.data_length,
            block_count: self.block_count,
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
        let mut output = BufWriter::new(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(path)?,
        );
        output.write_all(&header_to_bytes(&header))?;
        output.write_all(&self.lex_bytes)?;
        output.write_all(&super::super::BIDX_MAGIC)?;
        output.write_all(&1u32.to_le_bytes())?;
        output.write_all(&(self.block_count as u32).to_le_bytes())?;
        output.write_all(&0u32.to_le_bytes())?;
        copy_file(&self.bidx_path, &mut output)?;
        copy_file(&self.directory_path, &mut output)?;
        copy_file(&self.data_path, &mut output)?;
        output.flush()
    }
}

fn copy_file(path: &Path, output: &mut BufWriter<File>) -> io::Result<()> {
    let mut input = File::open(path)?;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            return Ok(());
        }
        output.write_all(&buffer[..count])?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42_volume::Q42Volume;
    use tempfile::NamedTempFile;

    #[test]
    fn streams_a_readable_volume_without_payload_accumulation() {
        let mut lex = HashMap::new();
        lex.insert(1, "s".to_string());
        lex.insert(2, "p".to_string());
        lex.insert(3, "o".to_string());
        let mut writer = StreamingQ42VolumeWriter::new(&lex).unwrap();
        writer
            .push_block(
                0,
                &[NQuin {
                    subject: 1,
                    predicate: 2,
                    object: 3,
                    context: 0,
                    metadata: 0,
                    parity: 0,
                }],
            )
            .unwrap();
        let output = NamedTempFile::new().unwrap();
        writer.finish(output.path()).unwrap();
        let volume = Q42Volume::open(output.path()).unwrap();
        assert_eq!(volume.block_count(), 1);
        assert_eq!(volume.object_hash_bounds(), Some((3, 3)));
    }
}
