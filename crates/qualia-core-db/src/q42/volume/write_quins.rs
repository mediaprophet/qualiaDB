//! Write a caller-owned Quin slice as a canonical unified v3 `.q42`.

use std::io;
use std::path::Path;

use super::super::UnifiedVolumeBuilder;
use crate::{NQuin, QUINS_PER_BLOCK};

/// Sort `quins` by object hash, chunk into SuperBlocks, and write a unified v3 volume.
///
/// Personal / session graphs stay unmarked (no Permissive Commons flag). Empty input is rejected.
#[cfg(not(target_arch = "wasm32"))]
pub fn write_sorted_quins_volume(path: &Path, quins: &[NQuin]) -> io::Result<usize> {
    write_sorted_quins_volume_with_author(path, quins, 0)
}

/// Same as [`write_sorted_quins_volume`], with a DAG author DID on each SuperBlock commit.
#[cfg(not(target_arch = "wasm32"))]
pub fn write_sorted_quins_volume_with_author(
    path: &Path,
    quins: &[NQuin],
    author_did: u64,
) -> io::Result<usize> {
    if quins.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "refusing to write an empty Q42 volume",
        ));
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut sorted = quins.to_vec();
    sorted.sort_unstable_by_key(|q| q.object);
    let mut builder = UnifiedVolumeBuilder::with_empty_lex().with_author_did(author_did);
    for (seq, chunk) in sorted.chunks(QUINS_PER_BLOCK).enumerate() {
        builder.push_block(seq as u64, chunk)?;
    }
    builder.finish(path)?;
    Ok(sorted.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42_volume::{Q42Volume, FLAG_FIELD_POSTINGS, FLAG_FIELD_RANGES, Q42_MAGIC};

    fn quin(object: u64) -> NQuin {
        let subject = 1;
        let predicate = 2;
        let context = 3;
        let metadata = 4;
        NQuin {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity: NQuin::calculate_parity(subject, predicate, object, context, metadata),
        }
    }

    #[test]
    fn writes_unified_v3_with_indexes_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.q42");
        let written = write_sorted_quins_volume(&path, &[quin(9), quin(1), quin(5)]).unwrap();
        assert_eq!(written, 3);
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(&Q42_MAGIC));
        let volume = Q42Volume::open(&path).unwrap();
        assert_eq!({ volume.header().version }, 3);
        assert!(volume.header().flags & FLAG_FIELD_RANGES != 0);
        assert!(volume.header().flags & FLAG_FIELD_POSTINGS != 0);
        volume.verify_all_blocks().expect("ECC + BIDX");
        let loaded = volume.read_all_quins().unwrap();
        assert_eq!(loaded.len(), 3);
        assert!(loaded.windows(2).all(|w| w[0].object <= w[1].object));
    }

    #[test]
    fn empty_slice_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let err = write_sorted_quins_volume(&dir.path().join("empty.q42"), &[]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }
}
