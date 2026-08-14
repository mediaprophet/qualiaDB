//! Read legacy framed LZ4 transport artifacts produced by
//! `ingest::streaming_import_rdf`.
//!
//! This module does **not** read canonical raw `.q42` SuperBlock containers.
//! It reads the older 16-byte-header + LZ4-payload framing that should now be
//! treated as a `.c.q42`-style transport artifact during the migration window.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::NQuin;

/// Decompress all quin slots from a legacy framed compressed transport file.
///
/// Prefer this function only for `.c.q42`-style payloads or other explicitly
/// legacy framed artifacts. Canonical raw `.q42` readers should operate on
/// 40,960-byte `QualiaSuperBlock` pages directly.
pub fn read_c_q42_quins(path: &Path) -> std::io::Result<Vec<NQuin>> {
    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let mut offset = 0u64;
    let mut quins = Vec::new();
    let quin_size = std::mem::size_of::<NQuin>();

    while offset < file_len {
        let mut header = [0u8; 16];
        if file.read_exact(&mut header).is_err() {
            break;
        }
        offset += 16;

        let compressed_len = u32::from_le_bytes(header[8..12].try_into().unwrap()) as usize;
        let mut compressed = vec![0u8; compressed_len];
        file.read_exact(&mut compressed)?;
        offset += compressed_len as u64;

        let uncompressed = lz4_flex::decompress_size_prepended(&compressed)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        for chunk in uncompressed.chunks_exact(quin_size) {
            let quin: NQuin = unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const NQuin) };
            quins.push(quin);
        }
    }

    Ok(quins)
}

/// Legacy compatibility wrapper.
///
/// Prefer canonical v3 Q42 containers (including front-embedded logical
/// volume roots), then fall back to the explicitly legacy framed transport.
/// This keeps existing daemon/SPARQL cold-load callers working while they
/// migrate to block-oriented query sources.
pub fn read_q42_quins(path: &Path) -> std::io::Result<Vec<NQuin>> {
    if let Ok(volume) = crate::q42_volume::Q42Volume::open(path) {
        if volume.volume_manifest()?.is_some() {
            let set = crate::q42_volume::Q42VolumeSet::open_root(path)?;
            let mut quins = Vec::new();
            for segment in set.segments() {
                quins.extend(segment.read_all_quins()?);
            }
            return Ok(quins);
        }
        return volume.read_all_quins();
    }
    read_c_q42_quins(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{q_hash, NQuin};
    use std::io::Write;

    fn write_test_c_q42(path: &Path, quins: &[NQuin]) {
        let mut file = File::create(path).unwrap();
        let bytes: Vec<u8> = quins
            .iter()
            .flat_map(|q| bytemuck::bytes_of(q).iter().copied())
            .collect();
        let compressed = lz4_flex::compress_prepend_size(&bytes);
        file.write_all(&0u64.to_le_bytes()).unwrap();
        file.write_all(&(compressed.len() as u32).to_le_bytes())
            .unwrap();
        file.write_all(&(bytes.len() as u32).to_le_bytes()).unwrap();
        file.write_all(&compressed).unwrap();
    }

    #[test]
    fn roundtrip_read_c_q42() {
        let dir = std::env::temp_dir().join(format!("q42-read-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.c.q42");
        let s = q_hash("ex:subject");
        let p = q_hash("ex:predicate");
        let o = q_hash("ex:object");
        let quin = NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: s ^ p ^ o,
        };
        write_test_c_q42(&path, &[quin]);
        let loaded = read_c_q42_quins(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].subject, s);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn canonical_volume_reader_dispatches_to_unified_v3() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("canonical.q42");
        let quin = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        crate::q42_volume::write_unified_volume(
            &path,
            &std::collections::HashMap::new(),
            &[(3, 3)],
            &[vec![quin]],
        )
        .unwrap();
        assert_eq!(read_q42_quins(&path).unwrap(), vec![quin]);
    }
}
