//! Immutable, size-capped logical Q42 volume publication from external runs.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::q42_volume::{
    root_relative_path, Q42LexiconSegment, Q42VolumeManifest, Q42VolumeSegment,
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

/// Catalog ontologies (Pages / Commons ingest). Sanctuary bits on a Quin
/// still win inside the writer.
fn new_catalog_writer(lex: &HashMap<u64, String>) -> io::Result<StreamingQ42VolumeWriter> {
    let mut writer = StreamingQ42VolumeWriter::new(lex)?;
    writer.declare_permissive_commons();
    Ok(writer)
}

impl ExternalSorter {
    /// Merge sorted runs into immutable, SuperBlock-aligned child segments and
    /// publish their descriptor plus front-manifested, size-capped lossless
    /// Q42LEX shards.  No standalone sidecar format is introduced.
    ///
    /// The cap applies to every child segment's whole physical file, including
    /// header, index, directory, and compressed data. The root is checked
    /// separately because the dictionary is physically sharded as well.
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

        let empty_lex = HashMap::new();
        let mut writer = Some(new_catalog_writer(&empty_lex)?);
        let mut segments = Vec::new();
        let mut segment_index = 0usize;
        let blocks_written = if self.chunk_files.len() > super::MAX_MERGE_FAN_IN {
            self.note(format!(
                "space-safe range merge of {} runs (no second full copy)",
                self.chunk_files.len()
            ));
            self.merge_by_object_range(
                root,
                max_segment_bytes,
                &empty_lex,
                &mut writer,
                &mut segments,
                &mut segment_index,
            )?
        } else {
            self.merge_compacted_runs(
                root,
                max_segment_bytes,
                &empty_lex,
                &mut writer,
                &mut segments,
                &mut segment_index,
            )?
        };

        if let Some(final_writer) = writer.take() {
            if final_writer.block_count() > 0 || segments.is_empty() {
                segments.push(finish_segment(final_writer, root, segment_index)?);
            }
        }

        let lexicon_segments = publish_lexicon_segments(root, &mut self.lex, max_segment_bytes)?;
        let manifest = Q42VolumeManifest {
            generation: 1,
            segments,
            lexicon_segments,
        };
        crate::q42_volume::write_volume_root_for_commons(root, &manifest)?;
        let root_bytes = std::fs::metadata(root)?.len();
        if root_bytes > max_segment_bytes {
            return Err(invalid(
                "Q42 root manifest exceeds the physical segment cap; reduce the segment count or use hierarchical manifests",
            ));
        }
        Ok(Q42VolumePublishStats {
            blocks_written,
            segments_written: manifest.segments.len() as u64,
            root_bytes,
        })
    }

    fn merge_compacted_runs(
        &mut self,
        root: &Path,
        max_segment_bytes: u64,
        empty_lex: &HashMap<u64, String>,
        writer: &mut Option<StreamingQ42VolumeWriter>,
        segments: &mut Vec<Q42VolumeSegment>,
        segment_index: &mut usize,
    ) -> io::Result<u64> {
        let chunk_files = self.compact_runs()?;
        let blocks = kway_merge_files(
            &chunk_files,
            root,
            max_segment_bytes,
            empty_lex,
            writer,
            segments,
            segment_index,
            self.quin_total(),
            &self.note,
        )?;
        for chunk_path in &chunk_files {
            let _ = std::fs::remove_file(chunk_path);
        }
        Ok(blocks)
    }

    /// Split each sorted chunk into 16 object-prefix ranges, delete the chunk,
    /// then k-way merge one range at a time into the volume. Peak extra space
    /// is one chunk (~48 MiB), not a second copy of the whole graph.
    fn merge_by_object_range(
        &mut self,
        root: &Path,
        max_segment_bytes: u64,
        empty_lex: &HashMap<u64, String>,
        writer: &mut Option<StreamingQ42VolumeWriter>,
        segments: &mut Vec<Q42VolumeSegment>,
        segment_index: &mut usize,
    ) -> io::Result<u64> {
        const BUCKETS: usize = 16;
        let paths: Vec<PathBuf> = (0..BUCKETS)
            .map(|i| self.temp_dir.join(format!("range_{i:02}.tmp")))
            .collect();
        let mut files: Vec<std::io::BufWriter<File>> = Vec::with_capacity(BUCKETS);
        for path in &paths {
            files.push(std::io::BufWriter::new(File::create(path)?));
        }
        let mut lens = [0u64; BUCKETS];
        let mut runs: Vec<Vec<(u64, u64)>> = vec![Vec::new(); BUCKETS];
        let chunks = std::mem::take(&mut self.chunk_files);
        let n_chunks = chunks.len();
        for (i, chunk) in chunks.iter().enumerate() {
            let starts = lens;
            let mut src = File::open(chunk)?;
            let mut buf = [0u8; 48];
            loop {
                match src.read_exact(&mut buf) {
                    Ok(()) => {
                        let quin: NQuin = bytemuck::pod_read_unaligned(&buf);
                        let bucket = range_bucket(quin.object, BUCKETS);
                        files[bucket].write_all(&buf)?;
                        lens[bucket] += 48;
                    }
                    Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(e),
                }
            }
            drop(src);
            for b in 0..BUCKETS {
                if lens[b] > starts[b] {
                    runs[b].push((starts[b], lens[b]));
                }
            }
            let _ = std::fs::remove_file(chunk);
            if i == 0 || (i + 1) % 64 == 0 || i + 1 == n_chunks {
                self.note(format!(
                    "range-split {}/{} chunks (reclaimed each source run)",
                    i + 1,
                    n_chunks
                ));
            }
        }
        for mut f in files {
            f.flush()?;
        }

        let mut blocks_written = 0u64;
        let total_quins = self.quin_total();
        for b in 0..BUCKETS {
            if runs[b].is_empty() {
                let _ = std::fs::remove_file(&paths[b]);
                continue;
            }
            self.note(format!(
                "range {b}/{BUCKETS}  {} run(s)  {} bytes",
                runs[b].len(),
                lens[b]
            ));
            blocks_written += kway_merge_slices(
                &paths[b],
                &runs[b],
                root,
                max_segment_bytes,
                empty_lex,
                writer,
                segments,
                segment_index,
                total_quins,
                blocks_written,
                &self.note,
            )?;
            let _ = std::fs::remove_file(&paths[b]);
        }
        Ok(blocks_written)
    }
}

const fn range_bucket(object: u64, buckets: usize) -> usize {
    let payload = object & 0x0FFF_FFFF_FFFF_FFFF;
    ((payload >> 56) as usize) % buckets
}

#[allow(clippy::too_many_arguments)]
fn kway_merge_files(
    files: &[PathBuf],
    root: &Path,
    max_segment_bytes: u64,
    empty_lex: &HashMap<u64, String>,
    writer: &mut Option<StreamingQ42VolumeWriter>,
    segments: &mut Vec<Q42VolumeSegment>,
    segment_index: &mut usize,
    total_quins: u64,
    note: &Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
) -> io::Result<u64> {
    let mut readers = Vec::with_capacity(files.len());
    for path in files {
        readers.push(BufReader::with_capacity(64 * 1024, File::open(path)?));
    }
    kway_merge_readers(
        &mut readers,
        root,
        max_segment_bytes,
        empty_lex,
        writer,
        segments,
        segment_index,
        total_quins,
        0,
        note,
    )
}

#[allow(clippy::too_many_arguments)]
fn kway_merge_slices(
    path: &Path,
    spans: &[(u64, u64)],
    root: &Path,
    max_segment_bytes: u64,
    empty_lex: &HashMap<u64, String>,
    writer: &mut Option<StreamingQ42VolumeWriter>,
    segments: &mut Vec<Q42VolumeSegment>,
    segment_index: &mut usize,
    total_quins: u64,
    blocks_already: u64,
    note: &Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
) -> io::Result<u64> {
    let mut readers = Vec::with_capacity(spans.len());
    for &(start, end) in spans {
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(start))?;
        readers.push(LimitedQuinReader {
            inner: BufReader::with_capacity(64 * 1024, file),
            remaining: end.saturating_sub(start),
        });
    }
    kway_merge_readers(
        &mut readers,
        root,
        max_segment_bytes,
        empty_lex,
        writer,
        segments,
        segment_index,
        total_quins,
        blocks_already,
        note,
    )
}

struct LimitedQuinReader {
    inner: BufReader<File>,
    remaining: u64,
}

impl QuinSource for LimitedQuinReader {
    fn next_quin(&mut self) -> io::Result<Option<NQuin>> {
        if self.remaining < 48 {
            return Ok(None);
        }
        let mut bytes = [0u8; 48];
        self.inner.read_exact(&mut bytes)?;
        self.remaining -= 48;
        Ok(Some(bytemuck::pod_read_unaligned(&bytes)))
    }
}

impl QuinSource for BufReader<File> {
    fn next_quin(&mut self) -> io::Result<Option<NQuin>> {
        ExternalSorter::read_quin(self)
    }
}

trait QuinSource {
    fn next_quin(&mut self) -> io::Result<Option<NQuin>>;
}

#[allow(clippy::too_many_arguments)]
fn kway_merge_readers<R: QuinSource>(
    readers: &mut [R],
    root: &Path,
    max_segment_bytes: u64,
    empty_lex: &HashMap<u64, String>,
    writer: &mut Option<StreamingQ42VolumeWriter>,
    segments: &mut Vec<Q42VolumeSegment>,
    segment_index: &mut usize,
    total_quins: u64,
    mut blocks_written: u64,
    note: &Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
) -> io::Result<u64> {
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
        if let Some(quin) = reader.next_quin()? {
            heap.push(HeapItem {
                quin,
                reader_idx: idx,
            });
        }
    }
    let mut block_buffer = Vec::with_capacity(QUINS_PER_BLOCK);
    let mut merged = 0u64;
    while let Some(item) = heap.pop() {
        block_buffer.push(item.quin);
        merged += 1;
        if let Some(next) = readers[item.reader_idx].next_quin()? {
            heap.push(HeapItem {
                quin: next,
                reader_idx: item.reader_idx,
            });
        }
        if block_buffer.len() == QUINS_PER_BLOCK {
            publish_block(
                writer,
                empty_lex,
                root,
                max_segment_bytes,
                segments,
                segment_index,
                &block_buffer,
            )?;
            blocks_written += 1;
            if blocks_written == 1 || blocks_written % 64 == 0 {
                let pct = if total_quins > 0 {
                    (merged as f64 / total_quins as f64) * 100.0
                } else {
                    0.0
                };
                if let Some(note) = note {
                    note(&format!("merge {pct:.1}%  SuperBlocks {blocks_written}"));
                }
            }
            block_buffer.clear();
        }
    }
    if !block_buffer.is_empty() {
        publish_block(
            writer,
            empty_lex,
            root,
            max_segment_bytes,
            segments,
            segment_index,
            &block_buffer,
        )?;
        blocks_written += 1;
    }
    Ok(blocks_written)
}

fn publish_lexicon_segments(
    root: &Path,
    lexicon: &mut super::LexiconSpill,
    max_segment_bytes: u64,
) -> io::Result<Vec<Q42LexiconSegment>> {
    if max_segment_bytes <= 1024 {
        return Err(invalid("Q42 segment cap is too small for a lexicon shard"));
    }
    let payload_budget = usize::try_from(max_segment_bytes - 1024)
        .map_err(|_| invalid("Q42 segment cap exceeds platform"))?;
    let mut shards = Vec::new();
    let mut pending = HashMap::new();
    let mut pending_bytes = 0usize;
    let unique = lexicon.for_each_sorted(|hash, term| {
        let cost = term
            .len()
            .checked_add(40)
            .ok_or_else(|| invalid("Q42 lexicon term length overflow"))?;
        if !pending.is_empty() && pending_bytes.saturating_add(cost) > payload_budget {
            shards.push(finish_lexicon_segment(
                root,
                shards.len(),
                &pending,
                max_segment_bytes,
            )?);
            pending.clear();
            pending_bytes = 0;
        }
        pending.insert(hash, term);
        pending_bytes = pending_bytes.saturating_add(cost);
        Ok(())
    })?;
    if !pending.is_empty() {
        shards.push(finish_lexicon_segment(
            root,
            shards.len(),
            &pending,
            max_segment_bytes,
        )?);
    }
    log::info!(
        "Lexicon publish: {unique} unique terms in {} shard(s).",
        shards.len()
    );
    Ok(shards)
}

fn finish_lexicon_segment(
    root: &Path,
    index: usize,
    lexicon: &HashMap<u64, String>,
    max_segment_bytes: u64,
) -> io::Result<Q42LexiconSegment> {
    let path = lexicon_segment_path(root, index)?;
    let mut writer = StreamingQ42VolumeWriter::new(lexicon)?;
    writer.declare_permissive_commons();
    writer.finish(&path)?;
    if std::fs::metadata(&path)?.len() > max_segment_bytes {
        return Err(invalid(
            "a single Q42 lexicon shard exceeds the physical segment cap",
        ));
    }
    Q42VolumeManifest::lexicon_segment_from_file(&path, root_relative_path(root, &path)?)
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
        *writer = Some(new_catalog_writer(empty_lex)?);
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

fn lexicon_segment_path(root: &Path, index: usize) -> io::Result<PathBuf> {
    let parent = root.parent().unwrap_or_else(|| Path::new("."));
    let stem = root
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| invalid("Q42 root output path has no UTF-8 file stem"))?;
    Ok(parent.join(format!("{stem}.lex-{index:05}.q42")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42_volume::{Q42Volume, Q42VolumeSet};
    use tempfile::TempDir;

    #[test]
    fn range_merge_reclaims_many_small_runs() {
        let dir = TempDir::new().unwrap();
        let sort_dir = dir.path().join("sort");
        std::fs::create_dir_all(&sort_dir).unwrap();
        let mut files = Vec::new();
        for i in 0..40u64 {
            let path = sort_dir.join(format!("chunk_{i}.tmp"));
            let quin = NQuin {
                subject: i + 1,
                predicate: 2,
                object: (i.wrapping_mul(0x9E37_79B9_7F4A_7C15)) & 0x0FFF_FFFF_FFFF_FFFF,
                context: 0,
                metadata: 0,
                parity: 0,
            };
            std::fs::write(&path, bytemuck::bytes_of(&quin)).unwrap();
            files.push(path);
        }
        let mut sorter = ExternalSorter::new(sort_dir);
        sorter.replace_chunks(files, 40);
        sorter.push_lex(1, "urn:q42:range-merge");
        let root = dir.path().join("range.q42");
        let stats = sorter.merge_volume_set(&root, 8 * 1024 * 1024).unwrap();
        assert!(stats.blocks_written >= 1);
        assert!(Q42Volume::open(&root).is_ok());
    }

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
        assert_eq!(root_volume.lex_view().unwrap().entry_count(), 0);
        assert_eq!(manifest.lexicon_segments.len(), 1);
        let set = Q42VolumeSet::open_root(&root).unwrap();
        assert_ne!(
            root_volume.header().flags & crate::q42_volume::FLAG_PERMISSIVE_COMMONS,
            0
        );
        assert_eq!(set.lookup_hash(7), Some("urn:q42:shared-term"));
        assert_eq!(
            crate::q42_lex::Q42Lexicon::load_for_q42(&root)
                .unwrap()
                .lookup(7),
            Some("urn:q42:shared-term")
        );
        set.verify_segment_hashes(&root).unwrap();
    }

    #[test]
    #[ignore = "opens the local 8+ GiB Monarch volume set when present"]
    fn monarch_volume_set_resolves_a_lexicon_term() {
        let root = std::path::Path::new(r"C:\Projects\monarch-kg\monarch-kg-root.q42");
        if !root.is_file() {
            return;
        }
        let set = Q42VolumeSet::open_root(root).expect("open monarch volume set");
        assert!(
            !set.lexicon_segments().is_empty(),
            "monarch root must name lexicon shards"
        );
        assert!(
            !set.segments().is_empty(),
            "monarch root must name data segments"
        );
        let view = set.lexicon_segments()[0]
            .lex_view()
            .expect("open first lexicon shard");
        let hash = view.hash_at(0).expect("first lexicon entry");
        let term = set
            .lookup_hash(hash)
            .expect("volume-set lookup through shard");
        assert!(!term.is_empty(), "resolved monarch term must be non-empty");
        println!("monarch lookup ok: {term}");
        set.verify_segment_hashes(root).expect("segment sha256");
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
