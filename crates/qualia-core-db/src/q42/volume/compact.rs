//! Compact a logical Q42 volume set (or a single-file volume) into a new generation.
//!
//! SuperBlocks are streamed in object order through a caller-owned 40960-byte
//! buffer. The hot per-Quin loop only copies into a stack `[NQuin; QUINS_PER_BLOCK]`.
//! Integrator wires `mod compact;` in `volume/mod.rs`.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use super::super::{
    write_volume_root_with_lex, Q42Volume, Q42VolumeManifest, StreamingQ42VolumeWriter,
    FLAG_PERMISSIVE_COMMONS, FLAG_SANCTUARY, QUIN_SIZE, SUPERBLOCK_HEADER, SUPERBLOCK_SIZE,
};
use super::manifest::Q42VolumeSet;
use super::publication::quin_requires_sanctuary;
use super::publish::{Q42RolloverPublisher, DEFAULT_SEGMENT_MAX_BYTES};
use crate::{NQuin, QUINS_PER_BLOCK};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

/// Compact `root` into `out_dir`.
///
/// A logical volume-set root is rewritten through [`Q42RolloverPublisher`]
/// (or [`StreamingQ42VolumeWriter`] when every input may remain Permissive
/// Commons). A single-file volume is rewritten through the streaming writer
/// so the result carries PIDX + FIDX + BIDX. The new catalog is published
/// with the append temp+rename pattern.
pub fn compact_volume_set(root: &Path, out_dir: &Path) -> io::Result<PathBuf> {
    std::fs::create_dir_all(out_dir)?;
    let volume = Q42Volume::open(root)?;
    if volume.volume_manifest()?.is_some() {
        compact_logical_set(root, out_dir)
    } else {
        compact_single_volume(&volume, root, out_dir)
    }
}

fn compact_logical_set(root: &Path, out_dir: &Path) -> io::Result<PathBuf> {
    let set = Q42VolumeSet::open_root(root)?;
    if set.segments().is_empty() {
        return Err(invalid("Q42 volume set has no data segments"));
    }

    let mut lex = HashMap::new();
    merge_lexicon(set.root(), &mut lex)?;
    for segment in set.segments() {
        merge_lexicon(segment, &mut lex)?;
    }
    for segment in set.lexicon_segments() {
        merge_lexicon(segment, &mut lex)?;
    }

    let all_commons = set.segments().iter().all(volume_declares_commons);
    let saw_sanctuary = scan_set_for_sanctuary(&set)?;
    let declare_commons = all_commons && !saw_sanctuary;
    let generation = set.manifest().generation.saturating_add(1);
    let stem = output_stem(root);

    let child_paths = if declare_commons {
        rewrite_set_through_streaming_writer(&set, out_dir, &stem, &lex, true)?
    } else {
        rewrite_set_through_publisher(&set, out_dir, &stem, lex.clone())?
    };

    let mut segments = Vec::with_capacity(child_paths.len());
    for path in &child_paths {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| invalid("Q42 compacted child name"))?
            .to_string();
        segments.push(Q42VolumeManifest::segment_from_file(path, name)?);
    }
    let manifest = Q42VolumeManifest {
        generation,
        segments,
        lexicon_segments: Vec::new(),
    };
    let root_path = out_dir.join(format!("{stem}-root.q42"));
    publish_root_atomically(&root_path, &lex, &manifest)?;
    Ok(root_path)
}

fn compact_single_volume(volume: &Q42Volume, root: &Path, out_dir: &Path) -> io::Result<PathBuf> {
    if volume.block_count() == 0 {
        return Err(invalid("Q42 volume has no SuperBlocks to compact"));
    }
    let mut lex = HashMap::new();
    merge_lexicon(volume, &mut lex)?;
    let declare_commons = volume_declares_commons(volume) && !scan_volume_for_sanctuary(volume)?;

    let mut writer = StreamingQ42VolumeWriter::new(&lex)?;
    if declare_commons {
        writer.declare_permissive_commons();
    }
    stream_volume_blocks(volume, |seq, quins| writer.push_block(seq, quins))?;

    let out = out_dir.join(format!("{}.q42", output_stem(root)));
    writer.finish(&out)?;
    Ok(out)
}

fn rewrite_set_through_publisher(
    set: &Q42VolumeSet,
    out_dir: &Path,
    stem: &str,
    lex: HashMap<u64, String>,
) -> io::Result<Vec<PathBuf>> {
    let mut publisher = Q42RolloverPublisher::new(out_dir, stem, lex)?;
    stream_set_blocks(set, |seq, quins| publisher.push_block(seq, quins))?;
    let produced = publisher.finish()?;
    let catalog = Q42Volume::open(&produced)?;
    let manifest = catalog.volume_manifest()?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Q42 rollover publisher produced no root catalog",
        )
    })?;
    let parent = produced.parent().unwrap_or(out_dir);
    let children = manifest
        .segments
        .iter()
        .map(|segment| parent.join(&segment.locator))
        .collect();
    // The publisher already wrote a generation-1 catalog; the caller replaces
    // it atomically with the incremented generation.
    let _ = std::fs::remove_file(&produced);
    Ok(children)
}

fn rewrite_set_through_streaming_writer(
    set: &Q42VolumeSet,
    out_dir: &Path,
    stem: &str,
    lex: &HashMap<u64, String>,
    declare_commons: bool,
) -> io::Result<Vec<PathBuf>> {
    let mut writer = CompactRollover::new(out_dir, stem, lex, declare_commons)?;
    stream_set_blocks(set, |seq, quins| writer.push_block(seq, quins))?;
    writer.finish_children()
}

/// Same rollover contract as [`Q42RolloverPublisher`], with publication flags.
struct CompactRollover {
    dir: PathBuf,
    stem: String,
    lex: HashMap<u64, String>,
    max_bytes: u64,
    writer: StreamingQ42VolumeWriter,
    child_paths: Vec<PathBuf>,
    next_index: u32,
    declare_commons: bool,
}

impl CompactRollover {
    fn new(
        dir: impl Into<PathBuf>,
        stem: impl Into<String>,
        lex: &HashMap<u64, String>,
        declare_commons: bool,
    ) -> io::Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            writer: new_streaming_writer(lex, declare_commons)?,
            lex: lex.clone(),
            dir,
            stem: stem.into(),
            max_bytes: DEFAULT_SEGMENT_MAX_BYTES,
            child_paths: Vec::new(),
            next_index: 0,
            declare_commons,
        })
    }

    fn push_block(&mut self, seq_id: u64, quins: &[NQuin]) -> io::Result<()> {
        let projected = self.writer.maximum_final_length_after_next_block()?;
        if self.writer.block_count() > 0 && projected > self.max_bytes {
            self.roll()?;
        }
        self.writer.push_block(seq_id, quins)
    }

    fn roll(&mut self) -> io::Result<()> {
        let path = self.child_path(self.next_index);
        let writer = std::mem::replace(
            &mut self.writer,
            new_streaming_writer(&self.lex, self.declare_commons)?,
        );
        writer.finish(&path)?;
        self.child_paths.push(path);
        self.next_index += 1;
        Ok(())
    }

    fn child_path(&self, index: u32) -> PathBuf {
        self.dir.join(format!("{}-{:04}.q42", self.stem, index))
    }

    fn finish_children(mut self) -> io::Result<Vec<PathBuf>> {
        if self.writer.block_count() > 0 || self.child_paths.is_empty() {
            let path = self.child_path(self.next_index);
            self.writer.finish(&path)?;
            self.child_paths.push(path);
        }
        Ok(self.child_paths)
    }
}

fn new_streaming_writer(
    lex: &HashMap<u64, String>,
    declare_commons: bool,
) -> io::Result<StreamingQ42VolumeWriter> {
    let mut writer = StreamingQ42VolumeWriter::new(lex)?;
    if declare_commons {
        writer.declare_permissive_commons();
    }
    Ok(writer)
}

fn publish_root_atomically(
    root_path: &Path,
    lex: &HashMap<u64, String>,
    manifest: &Q42VolumeManifest,
) -> io::Result<()> {
    let parent = root_path
        .parent()
        .ok_or_else(|| invalid("Q42 compact root has no parent"))?;
    let tmp = parent.join(format!(
        ".{}.tmp",
        root_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("root.q42")
    ));
    write_volume_root_with_lex(&tmp, lex, manifest)?;
    std::fs::rename(&tmp, root_path)
}

fn stream_set_blocks(
    set: &Q42VolumeSet,
    mut push: impl FnMut(u64, &[NQuin]) -> io::Result<()>,
) -> io::Result<()> {
    let mut seq = 0u64;
    for segment in set.segments() {
        stream_volume_blocks(segment, |_, quins| {
            let result = push(seq, quins);
            seq += 1;
            result
        })?;
    }
    if seq == 0 {
        return Err(invalid("Q42 volume set has no SuperBlocks to compact"));
    }
    Ok(())
}

fn stream_volume_blocks(
    volume: &Q42Volume,
    mut push: impl FnMut(u64, &[NQuin]) -> io::Result<()>,
) -> io::Result<()> {
    let mut decoded = [0u8; SUPERBLOCK_SIZE];
    let mut block = [NQuin::default(); QUINS_PER_BLOCK];
    for index in 0..volume.block_count() as usize {
        volume.read_superblock_into(index, &mut decoded)?;
        let live = decode_live_quins(&decoded, &mut block)?;
        push(index as u64, &block[..live])?;
    }
    Ok(())
}

fn decode_live_quins(
    decoded: &[u8; SUPERBLOCK_SIZE],
    out: &mut [NQuin; QUINS_PER_BLOCK],
) -> io::Result<usize> {
    let live = u64::from_le_bytes(decoded[16..24].try_into().unwrap()) as usize;
    if live == 0 || live > QUINS_PER_BLOCK {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Q42 SuperBlock has invalid live Quin count",
        ));
    }
    for i in 0..live {
        let offset = SUPERBLOCK_HEADER + i * QUIN_SIZE;
        out[i] = bytemuck::pod_read_unaligned(&decoded[offset..offset + QUIN_SIZE]);
    }
    Ok(live)
}

fn merge_lexicon(volume: &Q42Volume, dest: &mut HashMap<u64, String>) -> io::Result<()> {
    let view = volume.lex_view().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid Q42LEX: {error:?}"),
        )
    })?;
    for i in 0..view.entry_count() {
        let Some(hash) = view.hash_at(i) else {
            continue;
        };
        if let Some(text) = view.string_at(i) {
            dest.entry(hash).or_insert_with(|| text.to_owned());
        }
    }
    Ok(())
}

fn volume_declares_commons(volume: &Q42Volume) -> bool {
    volume.header().flags & FLAG_PERMISSIVE_COMMONS != 0
}

fn scan_set_for_sanctuary(set: &Q42VolumeSet) -> io::Result<bool> {
    for segment in set.segments() {
        if segment.header().flags & FLAG_SANCTUARY != 0 || scan_volume_for_sanctuary(segment)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn scan_volume_for_sanctuary(volume: &Q42Volume) -> io::Result<bool> {
    let mut decoded = [0u8; SUPERBLOCK_SIZE];
    let mut block = [NQuin::default(); QUINS_PER_BLOCK];
    for index in 0..volume.block_count() as usize {
        volume.read_superblock_into(index, &mut decoded)?;
        let live = decode_live_quins(&decoded, &mut block)?;
        if block[..live].iter().any(quin_requires_sanctuary) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn output_stem(root: &Path) -> String {
    let stem = root
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("volume");
    stem.strip_suffix("-root").unwrap_or(stem).to_string()
}

#[cfg(test)]
mod tests {
    use super::super::super::{
        write_volume_root, FLAG_FIELD_POSTINGS, FLAG_FIELD_RANGES, FLAG_OBJECT_SORTED,
    };
    use super::*;
    use crate::q42_volume::write_unified_volume;

    fn quin(object: u64) -> NQuin {
        NQuin {
            subject: object,
            predicate: 1,
            object,
            context: 0,
            metadata: 0,
            parity: object ^ 1 ^ object,
        }
    }

    fn medical_quin(object: u64) -> NQuin {
        let mut q = quin(object);
        q.set_sensitivity_byte(NQuin::SENSITIVITY_CLASSIFIED);
        q.set_sensitivity_tier(NQuin::SENSITIVITY_TIER_MEDICAL);
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    fn lex_for(entries: &[(u64, &str)]) -> HashMap<u64, String> {
        entries
            .iter()
            .map(|(hash, text)| (*hash, (*text).to_string()))
            .collect()
    }

    fn write_child(path: &Path, quins: &[NQuin], lex: &HashMap<u64, String>, commons: bool) {
        if commons {
            let mut writer = StreamingQ42VolumeWriter::new(lex).unwrap();
            writer.declare_permissive_commons();
            writer.push_block(0, quins).unwrap();
            writer.finish(path).unwrap();
        } else {
            let first = quins[0].object;
            let last = quins[quins.len() - 1].object;
            write_unified_volume(path, lex, &[(first, last)], &[quins.to_vec()]).unwrap();
        }
    }

    fn write_two_child_root(
        dir: &Path,
        first: &[NQuin],
        first_lex: &HashMap<u64, String>,
        first_commons: bool,
        second: &[NQuin],
        second_lex: &HashMap<u64, String>,
        second_commons: bool,
    ) -> PathBuf {
        let a = dir.join("segment-000.q42");
        let b = dir.join("segment-001.q42");
        write_child(&a, first, first_lex, first_commons);
        write_child(&b, second, second_lex, second_commons);
        let root = dir.join("set-root.q42");
        let manifest = Q42VolumeManifest {
            generation: 3,
            segments: vec![
                Q42VolumeManifest::segment_from_file(&a, "segment-000.q42".into()).unwrap(),
                Q42VolumeManifest::segment_from_file(&b, "segment-001.q42".into()).unwrap(),
            ],
            lexicon_segments: Vec::new(),
        };
        write_volume_root(&root, &manifest).unwrap();
        root
    }

    fn compacted_quins(root: &Path) -> Vec<NQuin> {
        let set = Q42VolumeSet::open_root(root).unwrap();
        let mut out = Vec::new();
        for segment in set.segments() {
            out.extend(segment.read_all_quins().unwrap());
        }
        out
    }

    #[test]
    fn two_children_compact_to_a_readable_root() {
        let src = tempfile::TempDir::new().unwrap();
        let out = tempfile::TempDir::new().unwrap();
        let first_lex = lex_for(&[(1, "p"), (10, "o-a")]);
        let second_lex = lex_for(&[(1, "p"), (20, "o-b")]);
        let root = write_two_child_root(
            src.path(),
            &[quin(10)],
            &first_lex,
            false,
            &[quin(20)],
            &second_lex,
            false,
        );

        let compacted = compact_volume_set(&root, out.path()).unwrap();
        let set = Q42VolumeSet::open_root(&compacted).unwrap();
        assert_eq!(set.manifest().generation, 4);
        assert!(!set.segments().is_empty());
        let quins = compacted_quins(&compacted);
        assert_eq!(quins.len(), 2);
        assert!(quins.iter().any(|q| q.object == 10));
        assert!(quins.iter().any(|q| q.object == 20));
        assert_eq!(set.lookup_hash(10), Some("o-a"));
        assert_eq!(set.lookup_hash(20), Some("o-b"));
    }

    #[test]
    fn object_order_preserved() {
        let src = tempfile::TempDir::new().unwrap();
        let out = tempfile::TempDir::new().unwrap();
        let lex = lex_for(&[(1, "p")]);
        let root = write_two_child_root(
            src.path(),
            &[quin(2), quin(4)],
            &lex,
            false,
            &[quin(6), quin(8)],
            &lex,
            false,
        );

        let compacted = compact_volume_set(&root, out.path()).unwrap();
        let objects: Vec<u64> = compacted_quins(&compacted)
            .iter()
            .map(|q| q.object)
            .collect();
        assert_eq!(objects, vec![2, 4, 6, 8]);
        assert!(objects.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn sanctuary_child_cannot_become_commons() {
        let src = tempfile::TempDir::new().unwrap();
        let out = tempfile::TempDir::new().unwrap();
        let lex = lex_for(&[(1, "p")]);
        let root = write_two_child_root(
            src.path(),
            &[quin(3)],
            &lex,
            true,
            &[medical_quin(9)],
            &lex,
            false,
        );

        let compacted = compact_volume_set(&root, out.path()).unwrap();
        let set = Q42VolumeSet::open_root(&compacted).unwrap();
        for segment in set.segments() {
            let flags = segment.header().flags;
            assert_eq!(
                flags & FLAG_PERMISSIVE_COMMONS,
                0,
                "Sanctuary input must not mint FLAG_PERMISSIVE_COMMONS"
            );
            assert_ne!(flags & FLAG_SANCTUARY, 0);
        }
        assert!(compacted_quins(&compacted)
            .iter()
            .any(quin_requires_sanctuary));
    }

    #[test]
    fn single_file_rewrites_with_field_indexes() {
        let src = tempfile::TempDir::new().unwrap();
        let out = tempfile::TempDir::new().unwrap();
        let path = src.path().join("plain.q42");
        let lex = lex_for(&[(1, "s"), (2, "p"), (3, "o")]);
        write_unified_volume(&path, &lex, &[(3, 3)], &[vec![quin(3)]]).unwrap();

        let compacted = compact_volume_set(&path, out.path()).unwrap();
        let volume = Q42Volume::open(&compacted).unwrap();
        let flags = volume.header().flags;
        assert_ne!(flags & FLAG_OBJECT_SORTED, 0);
        assert_ne!(flags & FLAG_FIELD_RANGES, 0);
        assert_ne!(flags & FLAG_FIELD_POSTINGS, 0);
        assert_eq!(volume.read_all_quins().unwrap()[0].object, 3);
        assert_eq!(volume.lex_view().unwrap().lookup_hash(3), Some("o"));
    }
}
