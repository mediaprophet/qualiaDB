//! Segment rollover and append-as-new-segment for logical Q42 volumes.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use super::super::{
    write_volume_root, Q42Volume, Q42VolumeManifest, StreamingQ42VolumeWriter,
};
use crate::NQuin;

/// Default child-segment cap (512 MiB). Includes header, lexicon, indexes, data.
pub const DEFAULT_SEGMENT_MAX_BYTES: u64 = 512 * 1024 * 1024;

/// Writes object-sorted SuperBlocks into successive `.q42` children and
/// publishes an atomic root catalog when finished.
pub struct Q42RolloverPublisher {
    dir: PathBuf,
    stem: String,
    lex: HashMap<u64, String>,
    max_bytes: u64,
    writer: StreamingQ42VolumeWriter,
    child_paths: Vec<PathBuf>,
    next_index: u32,
}

impl Q42RolloverPublisher {
    pub fn new(dir: impl Into<PathBuf>, stem: impl Into<String>, lex: HashMap<u64, String>) -> io::Result<Self> {
        Self::with_limit(dir, stem, lex, DEFAULT_SEGMENT_MAX_BYTES)
    }

    pub fn with_limit(
        dir: impl Into<PathBuf>,
        stem: impl Into<String>,
        lex: HashMap<u64, String>,
        max_bytes: u64,
    ) -> io::Result<Self> {
        if max_bytes < 8 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 segment cap is too small",
            ));
        }
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            writer: StreamingQ42VolumeWriter::new(&lex)?,
            lex,
            dir,
            stem: stem.into(),
            max_bytes,
            child_paths: Vec::new(),
            next_index: 0,
        })
    }

    pub fn push_block(&mut self, seq_id: u64, quins: &[NQuin]) -> io::Result<()> {
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
            StreamingQ42VolumeWriter::new(&self.lex)?,
        );
        writer.finish(&path)?;
        self.child_paths.push(path);
        self.next_index += 1;
        Ok(())
    }

    fn child_path(&self, index: u32) -> PathBuf {
        self.dir.join(format!("{}-{:04}.q42", self.stem, index))
    }

    /// Finish the open child and write `stem-root.q42` naming every child.
    pub fn finish(mut self) -> io::Result<PathBuf> {
        if self.writer.block_count() > 0 || self.child_paths.is_empty() {
            let path = self.child_path(self.next_index);
            self.writer.finish(&path)?;
            self.child_paths.push(path);
        }
        let mut segments = Vec::new();
        for path in &self.child_paths {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Q42 child name"))?
                .to_string();
            segments.push(Q42VolumeManifest::segment_from_file(path, name)?);
        }
        let root = self.dir.join(format!("{}-root.q42", self.stem));
        write_volume_root(
            &root,
            &Q42VolumeManifest {
                generation: 1,
                segments,
                lexicon_segments: Vec::new(),
            },
        )?;
        Ok(root)
    }
}

/// Append a finished child segment to an existing root by publishing a new
/// generation (`stem-root.q42` overwritten atomically via temp + rename).
pub fn append_segment_to_root(root_path: &Path, new_child: &Path) -> io::Result<()> {
    let parent = root_path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Q42 root has no parent"))?;
    let volume = Q42Volume::open(root_path)?;
    let mut manifest = volume.volume_manifest()?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "append requires a volume-root catalog",
        )
    })?;
    let name = new_child
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "child name"))?
        .to_string();
    manifest
        .segments
        .push(Q42VolumeManifest::segment_from_file(new_child, name)?);
    manifest.generation = manifest.generation.saturating_add(1);
    let tmp = parent.join(format!(
        ".{}.tmp",
        root_path.file_name().and_then(|n| n.to_str()).unwrap_or("root.q42")
    ));
    write_volume_root(&tmp, &manifest)?;
    std::fs::rename(&tmp, root_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NQuin;

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

    #[test]
    fn rolls_when_the_next_block_would_exceed_the_cap() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut lex = HashMap::new();
        lex.insert(1, "p".into());
        let mut pubr = Q42RolloverPublisher::with_limit(dir.path(), "set", lex, 16 * 1024).unwrap();
        for i in 0..8u64 {
            pubr.push_block(i, &[quin(i + 1)]).unwrap();
        }
        let root = pubr.finish().unwrap();
        let volume = Q42Volume::open(&root).unwrap();
        let manifest = volume.volume_manifest().unwrap().unwrap();
        assert!(
            manifest.segments.len() >= 2,
            "expected rollover, got {} children",
            manifest.segments.len()
        );
    }

    #[test]
    fn append_publishes_a_new_generation() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut lex = HashMap::new();
        lex.insert(1, "p".into());
        let mut pubr = Q42RolloverPublisher::with_limit(dir.path(), "gen", lex.clone(), 1024 * 1024)
            .unwrap();
        pubr.push_block(0, &[quin(1)]).unwrap();
        let root = pubr.finish().unwrap();
        let extra = dir.path().join("extra.q42");
        let mut w = StreamingQ42VolumeWriter::new(&lex).unwrap();
        w.push_block(1, &[quin(2)]).unwrap();
        w.finish(&extra).unwrap();
        append_segment_to_root(&root, &extra).unwrap();
        let volume = Q42Volume::open(&root).unwrap();
        let manifest = volume.volume_manifest().unwrap().unwrap();
        assert_eq!(manifest.generation, 2);
        assert_eq!(manifest.segments.len(), 2);
    }
}
