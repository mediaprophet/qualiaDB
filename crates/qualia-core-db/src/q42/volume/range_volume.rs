//! Range-backed Q42 segment reader for local, HTTP, and IPFS sources.

use std::io;

use super::range::{Q42ByteRange, Q42RangeSource};
use super::super::{
    header_from_bytes, BlockDirectoryEntry, Q42VolumeHeader, FLAG_BLOCKS_LZ4, HEADER_SIZE,
    MAX_COMPRESSED_SUPERBLOCK_SIZE, QUINS_PER_BLOCK, Q42_VERSION_V3, SUPERBLOCK_SIZE,
};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// A Q42 reader that fetches exactly the bytes needed from a random-access
/// source. All variable-size buffers remain caller-owned.
pub struct Q42RangeVolume<S: Q42RangeSource> {
    source: S,
    header: Q42VolumeHeader,
    source_length: u64,
}

impl<S: Q42RangeSource> Q42RangeVolume<S> {
    pub fn open(source: S) -> io::Result<Self> {
        let source_length = source.length()?;
        if source_length < HEADER_SIZE as u64 {
            return Err(invalid("Q42 range source is shorter than its header"));
        }
        let mut bytes = [0u8; HEADER_SIZE];
        source.read_range_into(Q42ByteRange { offset: 0, length: HEADER_SIZE }, &mut bytes)?;
        let header = header_from_bytes(&bytes)?;
        let version = header.version;
        let flags = header.flags;
        let block_size = header.block_size;
        let quins_per_block = header.quins_per_block;
        if version != Q42_VERSION_V3 || flags & FLAG_BLOCKS_LZ4 == 0 || block_size != SUPERBLOCK_SIZE as u32 || quins_per_block != QUINS_PER_BLOCK as u32 {
            return Err(invalid("unsupported Q42 range-volume header"));
        }
        for (name, offset, length) in [
            ("lexicon", header.lex_offset, header.lex_length),
            ("BIDX", header.bidx_offset, header.bidx_length),
            ("block directory", header.block_dir_offset, header.block_dir_length),
            ("block data", header.data_offset, header.data_length),
        ] {
            if length != 0 && (offset < HEADER_SIZE as u64 || offset.checked_add(length).is_none_or(|end| end > source_length)) {
                return Err(invalid(format!("Q42 {name} section lies outside the range source")));
            }
        }
        let expected_directory = header.block_count.checked_mul(BlockDirectoryEntry::SIZE as u64).ok_or_else(|| invalid("Q42 directory length overflow"))?;
        if header.block_dir_length != expected_directory {
            return Err(invalid("Q42 directory does not match its block count"));
        }
        Ok(Self { source, header, source_length })
    }

    pub fn header(&self) -> &Q42VolumeHeader { &self.header }
    pub fn source_length(&self) -> u64 { self.source_length }
    pub fn block_count(&self) -> u64 { self.header.block_count }

    pub fn read_lexicon_into(&self, out: &mut [u8]) -> io::Result<()> {
        self.read_section(self.header.lex_offset, self.header.lex_length, out)
    }
    pub fn read_bidx_into(&self, out: &mut [u8]) -> io::Result<()> {
        self.read_section(self.header.bidx_offset, self.header.bidx_length, out)
    }
    fn read_section(&self, offset: u64, length: u64, out: &mut [u8]) -> io::Result<()> {
        let length = usize::try_from(length).map_err(|_| invalid("Q42 section exceeds platform"))?;
        if out.len() != length { return Err(io::Error::new(io::ErrorKind::InvalidInput, "Q42 section output buffer has wrong length")); }
        self.source.read_range_into(Q42ByteRange { offset, length }, out)
    }

    pub fn block_directory_entry(&self, index: usize) -> io::Result<BlockDirectoryEntry> {
        if index >= self.header.block_count as usize { return Err(io::Error::new(io::ErrorKind::InvalidInput, "Q42 block index out of range")); }
        let offset = self.header.block_dir_offset.checked_add((index * BlockDirectoryEntry::SIZE) as u64).ok_or_else(|| invalid("Q42 directory offset overflow"))?;
        let mut bytes = [0u8; BlockDirectoryEntry::SIZE];
        self.source.read_range_into(Q42ByteRange { offset, length: BlockDirectoryEntry::SIZE }, &mut bytes)?;
        Ok(BlockDirectoryEntry::from_bytes(&bytes))
    }

    /// Fetch and decode one block. `compressed` must fit the directory entry;
    /// `out` must be at least one full decoded SuperBlock.
    pub fn read_superblock_into(&self, index: usize, compressed: &mut [u8], out: &mut [u8]) -> io::Result<usize> {
        if out.len() < SUPERBLOCK_SIZE { return Err(io::Error::new(io::ErrorKind::InvalidInput, "Q42 decoded output buffer is too small")); }
        let entry = self.block_directory_entry(index)?;
        let compressed_len = entry.comp_len as usize;
        if compressed_len < 4
            || compressed_len > MAX_COMPRESSED_SUPERBLOCK_SIZE
            || compressed.len() < compressed_len
            || entry.uncomp_len != SUPERBLOCK_SIZE as u32
        {
            return Err(invalid("invalid Q42 compressed block directory entry"));
        }
        let offset = self.header.data_offset.checked_add(entry.rel_offset).ok_or_else(|| invalid("Q42 compressed block offset overflow"))?;
        self.source.read_range_into(Q42ByteRange { offset, length: compressed_len }, &mut compressed[..compressed_len])?;
        let declared = u32::from_le_bytes(compressed[0..4].try_into().unwrap()) as usize;
        if declared != SUPERBLOCK_SIZE { return Err(invalid("Q42 LZ4 prefix does not declare one SuperBlock")); }
        let decoded = lz4_flex::decompress_into(&compressed[4..compressed_len], &mut out[..declared]).map_err(|error| invalid(format!("decode Q42 range block: {error}")))?;
        if decoded != declared { return Err(invalid("Q42 range block decoded to an unexpected length")); }
        Ok(decoded)
    }

    pub fn into_source(self) -> S { self.source }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::write_unified_volume;
    use crate::mini_parser::hash_token;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
    use crate::NQuin;
    use std::collections::HashMap;
    use tempfile::NamedTempFile;

    fn sample_volume() -> (NamedTempFile, NQuin) {
        let subject = hash_token("urn:q42:range-subject");
        let predicate = hash_token("urn:q42:range-predicate");
        let object = hash_token("urn:q42:range-object");
        let quin = NQuin { subject, predicate, object, context: 0, metadata: 0, parity: 0 };
        let mut lex = HashMap::new();
        lex.insert(subject, "urn:q42:range-subject".to_string());
        lex.insert(predicate, "urn:q42:range-predicate".to_string());
        lex.insert(object, "urn:q42:range-object".to_string());
        let file = NamedTempFile::new().unwrap();
        write_unified_volume(file.path(), &lex, &[(object, object)], &[vec![quin]]).unwrap();
        (file, quin)
    }

    #[test]
    fn range_volume_reads_only_the_directory_entry_and_block() {
        let (file, quin) = sample_volume();
        let source = super::super::range::LocalFileRangeSource::open(file.path()).unwrap();
        let volume = Q42RangeVolume::open(source).unwrap();
        assert_eq!(volume.block_count(), 1);
        let entry = volume.block_directory_entry(0).unwrap();
        assert!(entry.comp_len as usize <= MAX_COMPRESSED_SUPERBLOCK_SIZE);

        let mut compressed = [0u8; MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; SUPERBLOCK_SIZE];
        assert_eq!(volume.read_superblock_into(0, &mut compressed, &mut decoded).unwrap(), SUPERBLOCK_SIZE);
        assert_eq!(u64::from_le_bytes(decoded[16..24].try_into().unwrap()), 1);
        assert_eq!(u64::from_le_bytes(decoded[160..168].try_into().unwrap()), quin.subject);
    }

    #[test]
    fn range_volume_block_read_is_zero_heap() {
        let (file, _) = sample_volume();
        let source = super::super::range::LocalFileRangeSource::open(file.path()).unwrap();
        let volume = Q42RangeVolume::open(source).unwrap();
        let mut compressed = [0u8; MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; SUPERBLOCK_SIZE];
        assert_zero_alloc("q42_range_volume_block_read", || {
            volume.read_superblock_into(0, &mut compressed, &mut decoded).unwrap();
        });
    }
}
