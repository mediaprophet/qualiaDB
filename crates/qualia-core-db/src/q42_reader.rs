//! Read LZ4-compressed `.q42` block files produced by `ingest::streaming_import_rdf`.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::QualiaQuin;

/// Decompress all quin slots from a `.q42` block file.
pub fn read_q42_quins(path: &Path) -> std::io::Result<Vec<QualiaQuin>> {
    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let mut offset = 0u64;
    let mut quins = Vec::new();
    let quin_size = std::mem::size_of::<QualiaQuin>();

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
            let quin: QualiaQuin =
                unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const QualiaQuin) };
            quins.push(quin);
        }
    }

    Ok(quins)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{q_hash, QualiaQuin};
    use std::io::Write;

    fn write_test_q42(path: &Path, quins: &[QualiaQuin]) {
        let mut file = File::create(path).unwrap();
        let bytes: Vec<u8> = quins
            .iter()
            .flat_map(|q| bytemuck::bytes_of(q).iter().copied())
            .collect();
        let compressed = lz4_flex::compress_prepend_size(&bytes);
        file.write_all(&0u64.to_le_bytes()).unwrap();
        file.write_all(&(compressed.len() as u32).to_le_bytes()).unwrap();
        file.write_all(&(bytes.len() as u32).to_le_bytes()).unwrap();
        file.write_all(&compressed).unwrap();
    }

    #[test]
    fn roundtrip_read_q42() {
        let dir = std::env::temp_dir().join(format!("q42-read-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.q42");
        let s = q_hash("ex:subject");
        let p = q_hash("ex:predicate");
        let o = q_hash("ex:object");
        let quin = QualiaQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: s ^ p ^ o,
        };
        write_test_q42(&path, &[quin]);
        let loaded = read_q42_quins(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].subject, s);
        let _ = std::fs::remove_dir_all(dir);
    }
}
