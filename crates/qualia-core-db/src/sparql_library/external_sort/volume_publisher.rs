//! Immutable, size-capped logical Q42 volume publication from external runs.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

use crate::q42_volume::{
    root_relative_path, write_volume_root_with_lex, Q42VolumeManifest, Q42VolumeSegment,
    StreamingQ42VolumeWriter,
};
use crate::{NQuin, QUINS_PER_BLOCK};

use super::ExternalSorter;

/// Default physical child cap for online-safe Q42 distribution.
pub const DEFAULT_Q42_SEGMENT_MAX_BYTES: u64 = 512 * 1024 * 1024;

/// Outcome of publishing one logical root and its immutable child segments.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42VolumePublishStats {
    pub blocks_written: u64,
    pub segments_written: u64,
    pub root_bytes: u64,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

impl ExternalSorter {
    /// Merge sorted runs into immutable, SuperBlock-aligned child segments and
    /// publish their descriptor plus the lossless shared lexicon in `root`.
    ///
    /// The cap applies to every child segment's whole physical file, including
    /// header, index, directory, and compressed data. The root is checked
    /// separately because an unpaged shared lexicon can itself exceed the cap.
    pub fn merge_volume_set(
        mut self,
        root: &Path,
        max_segment_bytes: u64,
    ) -> io::Result<Q42VolumePublishStats> {
        if max_segment_bytes == 0 {
            return Err(invalid("Q42 volume segment cap must be non-zero"));
        }
        self.flush_chunk()?;
        if self.lex_collisions > 0 {
            return Err(invalid(format!(
                "cannot publish lossless Q42 volume: {} lexical token collision(s)",
                self.lex_collisions
            )));
        }
        if self.chunk_files.is_empty() {
            return Err(invalid("cannot publish a logical Q42 volume with no Quins"));
        }

        let chunk_files = self.compact_runs()?;
        let mut readers = Vec::with_capacity(chunk_files.len());
        for chunk_path in &chunk_files {
            readers.push(BufReader::with_capacity(
                1024 * 1024,
                File::open(chunk_path)?,
            ));
        }

        #[derive(Eq)]
        struct HeapItem {
            quin: NQuin,
            reader_idx: usize,
        }
        impl Ord for HeapItem {
            fn cmp(&self, other: &Self) -> Ordering {
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
        for (idx, reader) in readers.iter_mut().enumerate() {
            if let Some(quin) = Self::read_quin(reader)? {
                heap.push(HeapItem {
                    quin,
                    reader_idx: idx,
                });
            }
        }

        let empty_lex = HashMap::new();
        let mut writer = Some(StreamingQ42VolumeWriter::new(&empty_lex)?);
        let mut block_buffer = Vec::with_capacity(QUINS_PER_BLOCK);
        let mut segments = Vec::new();
        let mut segment_index = 0usize;
        let mut blocks_written = 0u64;

        while let Some(item) = heap.pop() {
            block_buffer.push(item.quin);
            if let Some(next) = Self::read_quin(&mut readers[item.reader_idx])? {
                heap.push(HeapItem {
                    quin: next,
                    reader_idx: item.reader_idx,
                });
            }
            if block_buffer.len() == QUINS_PER_BLOCK {
                publish_block(
                    &mut writer,
                    &empty_lex,
                    root,
                    max_segment_bytes,
                    &mut segments,
                    &mut segment_index,
                    &block_buffer,
                )?;
                blocks_written += 1;
                block_buffer.clear();
            }
        }
        if !block_buffer.is_empty() {
            publish_block(
                &mut writer,
                &empty_lex,
                root,
                max_segment_bytes,
                &mut segments,
                &mut segment_index,
                &block_buffer,
            )?;
            blocks_written += 1;
        }

        let final_writer = writer.take().expect("segment writer must remain present");
        segments.push(finish_segment(final_writer, root, segment_index)?);
        for chunk_path in &chunk_files {
            let _ = std::fs::remove_file(chunk_path);
        }

        let manifest = Q42VolumeManifest {
            generation: 1,
            segments,
        };
        write_volume_root_with_lex(root, &self.lex, &manifest)?;
        let root_bytes = std::fs::metadata(root)?.len();
        if root_bytes > max_segment_bytes {
            return Err(invalid(
                "Q42 root lexicon exceeds the physical segment cap; use paged Q42LEX before publishing this dataset",
            ));
        }
        Ok(Q42VolumePublishStats {
            blocks_written,
            segments_written: manifest.segments.len() as u64,
            root_bytes,
        })
    }
}

fn publish_block(
    writer: &mut Option<StreamingQ42VolumeWriter>,
    empty_lex: &HashMap<u64, String>,
    root: &Path,
    max_segment_bytes: u64,
    segments: &mut Vec<Q42VolumeSegment>,
    segment_index: &mut usize,
    block: &[NQuin],
) -> io::Result<()> {
    let current = writer.as_ref().expect("segment writer must be present");
    if current.maximum_final_length_after_next_block()? > max_segment_bytes {
        if current.block_count() == 0 {
            return Err(invalid(
                "Q42 segment cap is smaller than one encoded SuperBlock",
            ));
        }
        let complete = writer.take().expect("segment writer must be present");
        segments.push(finish_segment(complete, root, *segment_index)?);
        *segment_index += 1;
        *writer = Some(StreamingQ42VolumeWriter::new(empty_lex)?);
    }
    let current = writer.as_mut().expect("segment writer must be present");
    current.push_block(current.block_count(), block)
}

fn finish_segment(
    writer: StreamingQ42VolumeWriter,
    root: &Path,
    segment_index: usize,
) -> io::Result<Q42VolumeSegment> {
    let path = child_segment_path(root, segment_index)?;
    writer.finish(&path)?;
    let locator = root_relative_path(root, &path)?;
    Q42VolumeManifest::segment_from_file(&path, locator)
}

fn child_segment_path(root: &Path, segment_index: usize) -> io::Result<PathBuf> {
    let parent = root.parent().unwrap_or_else(|| Path::new("."));
    let stem = root
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| invalid("Q42 root output path has no UTF-8 file stem"))?;
    Ok(parent.join(format!("{stem}.segment-{segment_index:05}.q42")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42_volume::{Q42Volume, Q42VolumeSet};
    use tempfile::TempDir;

    #[test]
    fn publisher_splits_only_at_superblocks_and_publishes_a_lossless_root() {
        let dir = TempDir::new().unwrap();
        let mut sorter = ExternalSorter::new(dir.path().join("sort"));
        for block in 0..2u64 {
            for index in 0..QUINS_PER_BLOCK as u64 {
                let seed = block * QUINS_PER_BLOCK as u64 + index;
                sorter
                    .push(NQuin {
                        subject: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15),
                        predicate: seed.wrapping_mul(0xD6E8_FEB8_6659_FD93),
                        object: seed,
                        context: seed.wrapping_mul(0xA076_1D64_78BD_642F),
                        metadata: seed.wrapping_mul(0xE703_7ED1_A0B4_28DB),
                        parity: seed.wrapping_mul(0x8EBC_6AF0_9C88_C6E3),
                    })
                    .unwrap();
            }
        }
        sorter.push_lex(7, "urn:q42:shared-term");
        let root = dir.path().join("dataset.q42");
        let stats = sorter.merge_volume_set(&root, 42 * 1024).unwrap();
        assert_eq!(stats.blocks_written, 2);
        assert_eq!(stats.segments_written, 2);
        assert!(stats.root_bytes <= 42 * 1024);

        let root_volume = Q42Volume::open(&root).unwrap();
        let manifest = root_volume.volume_manifest().unwrap().unwrap();
        assert_eq!(manifest.segments.len(), 2);
        assert!(root_volume.lex_view().unwrap().lookup_hash(7).is_some());
        let set = Q42VolumeSet::open_root(&root).unwrap();
        assert!(set.root().lex_view().unwrap().lookup_hash(7).is_some());
        set.verify_segment_hashes(&root).unwrap();
    }

    #[test]
    fn publisher_rejects_a_cap_that_cannot_hold_one_superblock() {
        let dir = TempDir::new().unwrap();
        let mut sorter = ExternalSorter::new(dir.path().join("sort"));
        sorter
            .push(NQuin {
                subject: 1,
                predicate: 2,
                object: 3,
                context: 0,
                metadata: 0,
                parity: 0,
            })
            .unwrap();
        assert!(sorter
            .merge_volume_set(&dir.path().join("dataset.q42"), 1)
            .is_err());
    }
}
