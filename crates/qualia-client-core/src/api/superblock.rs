//! SuperBlock artifacts and views over unified v3 `.q42` volumes.

#![allow(non_snake_case)]

use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use qualia_core_db::q42_reader::read_c_q42_quins;
use qualia_core_db::q42_volume::{
    decode_superblock_quins, is_unified_volume, Q42Volume, SUPERBLOCK_SIZE,
};

#[derive(Debug, Clone, Serialize)]
pub struct SuperBlockArtifact {
    pub path: String,
    pub display_name: String,
    pub byte_size: u64,
    pub block_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuperBlockView {
    pub source_path: String,
    pub block_index: u64,
    pub total_blocks: u64,
    pub block_sequence_id: u64,
    pub storage_owner_did: u64,
    pub active_quin_count: u64,
    pub validation_checksum: u32,
    pub hardware_profile_flags: u32,
    pub fea_mesh_index_id: u64,
    pub raw_bytes: Vec<u8>,
    pub quins: Vec<[u64; 6]>,
}

fn decode_u64(bytes: &[u8], start: usize) -> Result<u64, String> {
    let end = start + 8;
    let slice = bytes
        .get(start..end)
        .ok_or_else(|| format!("SuperBlock truncated at byte range {start}..{end}"))?;
    let mut buf = [0u8; 8];
    buf.copy_from_slice(slice);
    Ok(u64::from_le_bytes(buf))
}

fn decode_u32(bytes: &[u8], start: usize) -> Result<u32, String> {
    let end = start + 4;
    let slice = bytes
        .get(start..end)
        .ok_or_else(|| format!("SuperBlock truncated at byte range {start}..{end}"))?;
    let mut buf = [0u8; 4];
    buf.copy_from_slice(slice);
    Ok(u32::from_le_bytes(buf))
}

fn scan_q42_artifacts(root: &Path, out: &mut Vec<SuperBlockArtifact>) -> Result<(), String> {
    if !root.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            scan_q42_artifacts(&path, out)?;
            continue;
        }
        if path.extension().and_then(|v| v.to_str()) != Some("q42") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        if name.ends_with(".c.q42") {
            continue;
        }
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        let block_count = artifact_block_count(&path, meta.len())?;
        if block_count == 0 && meta.len() < SUPERBLOCK_SIZE as u64 {
            // Empty roots still list; tiny non-volumes are skipped.
            if is_unified_volume(&path).ok() != Some(true) {
                continue;
            }
        }
        out.push(SuperBlockArtifact {
            path: path.to_string_lossy().into_owned(),
            display_name: name.to_string(),
            byte_size: meta.len(),
            block_count,
        });
    }
    Ok(())
}

fn artifact_block_count(path: &Path, byte_len: u64) -> Result<u64, String> {
    if is_unified_volume(path).ok() == Some(true) {
        let volume = Q42Volume::open(path).map_err(|e| e.to_string())?;
        return Ok(volume.block_count());
    }
    if byte_len >= SUPERBLOCK_SIZE as u64 && byte_len % SUPERBLOCK_SIZE as u64 == 0 {
        return Ok(byte_len / SUPERBLOCK_SIZE as u64);
    }
    if byte_len >= 16 {
        return Ok(1);
    }
    Ok(0)
}

pub fn list_superblock_artifacts() -> Result<Vec<SuperBlockArtifact>, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let mut out = Vec::new();
    scan_q42_artifacts(Path::new(&storage), &mut out)?;
    out.sort_by(|a, b| {
        a.display_name
            .cmp(&b.display_name)
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(out)
}

pub fn get_superblock_view(
    source_path: String,
    block_index: u64,
) -> Result<SuperBlockView, String> {
    let path = PathBuf::from(&source_path);
    if !path.is_file() {
        return Err(format!("SuperBlock source not found: {}", path.display()));
    }
    if path.extension().and_then(|v| v.to_str()) != Some("q42") {
        return Err("Block inspector expects a raw .q42 artifact".to_string());
    }

    if is_unified_volume(&path).ok() == Some(true) {
        return view_unified_volume(&path, source_path, block_index);
    }

    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    if metadata.len() >= SUPERBLOCK_SIZE as u64 && metadata.len() % SUPERBLOCK_SIZE as u64 == 0 {
        return view_legacy_raw_pages(&path, source_path, block_index, metadata.len());
    }

    view_legacy_framed(&path, source_path, block_index)
}

fn view_unified_volume(
    path: &Path,
    source_path: String,
    block_index: u64,
) -> Result<SuperBlockView, String> {
    let volume = Q42Volume::open(path).map_err(|e| e.to_string())?;
    if volume
        .volume_manifest()
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err(
            "This Q42 is a volume-set root (no local SuperBlocks). Open a data child segment."
                .into(),
        );
    }
    let total_blocks = volume.block_count();
    if total_blocks == 0 {
        return Err("Volume has no SuperBlocks".into());
    }
    if block_index >= total_blocks {
        return Err(format!(
            "Block index {block_index} is out of range for {total_blocks} blocks"
        ));
    }
    let mut raw_bytes = vec![0u8; SUPERBLOCK_SIZE];
    volume
        .read_superblock_into(block_index as usize, &mut raw_bytes)
        .map_err(|e| e.to_string())?;
    view_from_decompressed(source_path, block_index, total_blocks, raw_bytes)
}

fn view_legacy_raw_pages(
    path: &Path,
    source_path: String,
    block_index: u64,
    file_len: u64,
) -> Result<SuperBlockView, String> {
    let total_blocks = file_len / SUPERBLOCK_SIZE as u64;
    if block_index >= total_blocks {
        return Err(format!(
            "Block index {block_index} is out of range for {total_blocks} blocks"
        ));
    }
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    use std::io::Seek;
    use std::io::SeekFrom;
    file.seek(SeekFrom::Start(block_index * SUPERBLOCK_SIZE as u64))
        .map_err(|e| e.to_string())?;
    let mut raw_bytes = vec![0u8; SUPERBLOCK_SIZE];
    file.read_exact(&mut raw_bytes).map_err(|e| e.to_string())?;
    view_from_decompressed(source_path, block_index, total_blocks, raw_bytes)
}

fn view_legacy_framed(
    path: &Path,
    source_path: String,
    block_index: u64,
) -> Result<SuperBlockView, String> {
    if block_index != 0 {
        return Err("Legacy framed .q42 exposes a single logical block (index 0)".into());
    }
    let quins = read_c_q42_quins(path).map_err(|e| e.to_string())?;
    if quins.is_empty() {
        return Err("Legacy framed .q42 contained no Quins".into());
    }
    let packed: Vec<[u64; 6]> = quins
        .iter()
        .map(|q| {
            [
                q.subject,
                q.predicate,
                q.object,
                q.context,
                q.metadata,
                q.parity,
            ]
        })
        .collect();
    Ok(SuperBlockView {
        source_path,
        block_index: 0,
        total_blocks: 1,
        block_sequence_id: 0,
        storage_owner_did: 0,
        active_quin_count: packed.len() as u64,
        validation_checksum: 0,
        hardware_profile_flags: 0,
        fea_mesh_index_id: 0,
        raw_bytes: Vec::new(),
        quins: packed,
    })
}

fn view_from_decompressed(
    source_path: String,
    block_index: u64,
    total_blocks: u64,
    raw_bytes: Vec<u8>,
) -> Result<SuperBlockView, String> {
    let block_sequence_id = decode_u64(&raw_bytes, 0)?;
    let storage_owner_did = decode_u64(&raw_bytes, 8)?;
    let decoded = decode_superblock_quins(&raw_bytes).map_err(|e| e.to_string())?;
    let active_quin_count = decoded.len() as u64;
    let validation_checksum = decode_u32(&raw_bytes, 24)?;
    let hardware_profile_flags = decode_u32(&raw_bytes, 28)?;
    let fea_mesh_index_id = decode_u64(&raw_bytes, 32)?;
    let quins = decoded
        .into_iter()
        .map(|q| {
            [
                q.subject,
                q.predicate,
                q.object,
                q.context,
                q.metadata,
                q.parity,
            ]
        })
        .collect();
    Ok(SuperBlockView {
        source_path,
        block_index,
        total_blocks,
        block_sequence_id,
        storage_owner_did,
        active_quin_count,
        validation_checksum,
        hardware_profile_flags,
        fea_mesh_index_id,
        raw_bytes,
        quins,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::q42_volume::write_sorted_quins_volume;
    use qualia_core_db::NQuin;

    fn sample_quin(object: u64) -> NQuin {
        NQuin {
            subject: 11,
            predicate: 22,
            object,
            context: 33,
            metadata: 0,
            parity: NQuin::calculate_parity(11, 22, object, 33, 0),
        }
    }

    #[test]
    fn inspector_reads_unified_v3_block() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("body.q42");
        write_sorted_quins_volume(&path, &[sample_quin(7), sample_quin(3)]).unwrap();
        let view = get_superblock_view(path.to_string_lossy().into_owned(), 0).unwrap();
        assert_eq!(view.total_blocks, 1);
        assert_eq!(view.active_quin_count, 2);
        assert_eq!(view.quins.len(), 2);
        assert!(view.quins.iter().any(|q| q[2] == 3));
        assert!(view.quins.iter().any(|q| q[2] == 7));
    }
}
