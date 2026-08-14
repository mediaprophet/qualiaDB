//! Structural validation for the unified Q42 v3 reader.

use std::io;

use super::super::{
    BlockDirectoryEntry, Q42VolumeHeader, FLAG_BLOCKS_LZ4, FLAG_OBJECT_SORTED, HEADER_SIZE,
    QUINS_PER_BLOCK, SUPERBLOCK_SIZE,
};
use super::index::validate_bidx;

#[derive(Clone, Copy)]
struct Section {
    name: &'static str,
    start: usize,
    end: usize,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn checked_usize(value: u64, name: &str) -> io::Result<usize> {
    usize::try_from(value).map_err(|_| invalid(format!("{name} does not fit this platform")))
}

fn section(
    name: &'static str,
    offset: u64,
    length: u64,
    file_len: usize,
) -> io::Result<Option<Section>> {
    if length == 0 {
        return Ok(None);
    }
    let start = checked_usize(offset, name)?;
    let length = checked_usize(length, name)?;
    if start < HEADER_SIZE {
        return Err(invalid(format!("{name} begins inside the fixed header")));
    }
    let end = start
        .checked_add(length)
        .ok_or_else(|| invalid(format!("{name} range overflows usize")))?;
    if end > file_len {
        return Err(invalid(format!("{name} range exceeds the Q42 file length")));
    }
    Ok(Some(Section { name, start, end }))
}

fn validate_non_overlapping(sections: &[Option<Section>]) -> io::Result<()> {
    for (left_index, left) in sections.iter().enumerate() {
        let Some(left) = left else {
            continue;
        };
        for right in sections.iter().skip(left_index + 1).flatten() {
            if left.start < right.end && right.start < left.end {
                return Err(invalid(format!(
                    "Q42 sections {} and {} overlap",
                    left.name, right.name
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_volume_structure(header: &Q42VolumeHeader, bytes: &[u8]) -> io::Result<()> {
    let flags = header.flags;
    let block_size = header.block_size;
    let quins_per_block = header.quins_per_block;
    let block_count = checked_usize(header.block_count, "block count")?;
    let lex_offset = header.lex_offset;
    let lex_length = header.lex_length;
    let bidx_offset = header.bidx_offset;
    let bidx_length = header.bidx_length;
    let dir_offset = header.block_dir_offset;
    let dir_length = header.block_dir_length;
    let data_offset = header.data_offset;
    let data_length = header.data_length;
    let temporal_offset = header.temporal_index_offset;
    let temporal_length = header.temporal_index_length;
    let dag_offset = header.dag_root_offset;
    let dag_length = header.dag_root_length;
    let natural_person_offset = header.natural_person_did_offset;
    let software_agent_offset = header.software_agent_did_offset;
    let manifest_range = header.volume_manifest_range();

    if flags & FLAG_BLOCKS_LZ4 == 0 {
        return Err(invalid("Q42 v3 volume does not declare block-local LZ4"));
    }
    if block_count != 0 && flags & FLAG_OBJECT_SORTED == 0 {
        return Err(invalid(
            "Q42 volume has blocks but does not declare its required object sort order",
        ));
    }
    if block_size != SUPERBLOCK_SIZE as u32 {
        return Err(invalid(format!(
            "Q42 block size {block_size} is not the v3 SuperBlock size {SUPERBLOCK_SIZE}"
        )));
    }
    if quins_per_block != QUINS_PER_BLOCK as u32 {
        return Err(invalid(format!(
            "Q42 Quins per block {quins_per_block} does not match {QUINS_PER_BLOCK}"
        )));
    }

    let expected_dir_length = block_count
        .checked_mul(BlockDirectoryEntry::SIZE)
        .ok_or_else(|| invalid("Q42 directory length overflows usize"))?;
    if checked_usize(dir_length, "block directory length")? != expected_dir_length {
        return Err(invalid(format!(
            "Q42 block directory length does not match {block_count} entries"
        )));
    }
    if block_count != 0 && bidx_length == 0 {
        return Err(invalid("Q42 volume with blocks has no BIDX section"));
    }
    if block_count == 0 && data_length != 0 {
        return Err(invalid(
            "Q42 volume has data bytes but no directory entries",
        ));
    }

    let manifest_section = match manifest_range {
        Some((offset, length)) => {
            if length == 0 || length > super::manifest::MAX_VOLUME_MANIFEST_BYTES as u64 {
                return Err(invalid("Q42 root has an invalid volume manifest length"));
            }
            Some(
                section("volume manifest", offset, length, bytes.len())?
                    .ok_or_else(|| invalid("Q42 root has an empty volume manifest"))?,
            )
        }
        None => None,
    };
    let sections = [
        section("lexicon", lex_offset, lex_length, bytes.len())?,
        manifest_section,
        section("BIDX", bidx_offset, bidx_length, bytes.len())?,
        section("block directory", dir_offset, dir_length, bytes.len())?,
        section("block data", data_offset, data_length, bytes.len())?,
        section(
            "temporal index",
            temporal_offset,
            temporal_length,
            bytes.len(),
        )?,
        section("Merkle DAG", dag_offset, dag_length, bytes.len())?,
    ];
    validate_non_overlapping(&sections)?;

    if let Some(manifest) = manifest_section {
        super::manifest::Q42VolumeManifest::decode(&bytes[manifest.start..manifest.end])
            .map_err(|error| invalid(format!("embedded volume manifest is invalid: {error}")))?;
    }

    for (name, offset) in [
        ("natural-person DID offset", natural_person_offset),
        ("software-agent DID offset", software_agent_offset),
    ] {
        if offset != 0 && checked_usize(offset, name)? > bytes.len() {
            return Err(invalid(format!("{name} exceeds the Q42 file length")));
        }
    }

    if lex_length != 0 {
        let lex_start = checked_usize(lex_offset, "lexicon offset")?;
        let lex_end = lex_start + checked_usize(lex_length, "lexicon length")?;
        crate::q42_lex::Q42LexMmap::from_bytes(&bytes[lex_start..lex_end])
            .map_err(|error| invalid(format!("embedded Q42LEX header is invalid: {error:?}")))?;
    }

    if bidx_length != 0 {
        let bidx_start = checked_usize(bidx_offset, "BIDX offset")?;
        let bidx_end = bidx_start + checked_usize(bidx_length, "BIDX length")?;
        validate_bidx(&bytes[bidx_start..bidx_end], block_count)?;
    }

    let dir_start = checked_usize(dir_offset, "block directory offset")?;
    let data_len = checked_usize(data_length, "data length")?;
    let mut expected_rel_offset = 0usize;
    for index in 0..block_count {
        let offset = dir_start + index * BlockDirectoryEntry::SIZE;
        let mut entry_bytes = [0u8; BlockDirectoryEntry::SIZE];
        entry_bytes.copy_from_slice(&bytes[offset..offset + BlockDirectoryEntry::SIZE]);
        let entry = BlockDirectoryEntry::from_bytes(&entry_bytes);
        let rel_offset = checked_usize(entry.rel_offset, "block relative offset")?;
        let compressed_len = entry.comp_len as usize;
        let uncompressed_len = entry.uncomp_len;
        if compressed_len < 4 {
            return Err(invalid(format!(
                "Q42 block {index} is too short for an LZ4 size prefix"
            )));
        }
        if uncompressed_len != SUPERBLOCK_SIZE as u32 {
            return Err(invalid(format!(
                "Q42 block {index} declares {} decoded bytes, expected {SUPERBLOCK_SIZE}",
                uncompressed_len
            )));
        }
        if rel_offset != expected_rel_offset {
            return Err(invalid(format!(
                "Q42 block {index} is not contiguous with the preceding block"
            )));
        }
        expected_rel_offset = rel_offset
            .checked_add(compressed_len)
            .ok_or_else(|| invalid("Q42 compressed block range overflows usize"))?;
        if expected_rel_offset > data_len {
            return Err(invalid(format!(
                "Q42 block {index} exceeds the declared data section"
            )));
        }
    }
    if expected_rel_offset != data_len {
        return Err(invalid(
            "Q42 data section has bytes not described by the block directory",
        ));
    }

    Ok(())
}
