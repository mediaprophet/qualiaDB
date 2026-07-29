//! SuperBlock artifacts and views

#![allow(non_snake_case)]

use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

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
        if meta.len() < qualia_core_db::BLOCK_MULTIPLIER_SIZE as u64 {
            continue;
        }
        out.push(SuperBlockArtifact {
            path: path.to_string_lossy().into_owned(),
            display_name: name.to_string(),
            byte_size: meta.len(),
            block_count: meta.len() / qualia_core_db::BLOCK_MULTIPLIER_SIZE as u64,
        });
    }
    Ok(())
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
    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    let block_size = qualia_core_db::BLOCK_MULTIPLIER_SIZE as u64;
    let total_blocks = metadata.len() / block_size;
    if total_blocks == 0 {
        return Err("Artifact does not contain a full 40 KB SuperBlock".to_string());
    }
    if block_index >= total_blocks {
        return Err(format!(
            "Block index {block_index} is out of range for {} blocks",
            total_blocks
        ));
    }

    let mut file = fs::File::open(&path).map_err(|e| e.to_string())?;
    let offset = block_index * block_size;
    use std::io::Seek;
    use std::io::SeekFrom;
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| e.to_string())?;
    let mut raw_bytes = vec![0u8; qualia_core_db::BLOCK_MULTIPLIER_SIZE];
    file.read_exact(&mut raw_bytes).map_err(|e| e.to_string())?;

    let block_sequence_id = decode_u64(&raw_bytes, 0)?;
    let storage_owner_did = decode_u64(&raw_bytes, 8)?;
    let active_quin_count = decode_u64(&raw_bytes, 16)?.min(qualia_core_db::QUINS_PER_BLOCK as u64);
    let validation_checksum = decode_u32(&raw_bytes, 24)?;
    let hardware_profile_flags = decode_u32(&raw_bytes, 28)?;
    let fea_mesh_index_id = decode_u64(&raw_bytes, 32)?;

    let mut quins = Vec::with_capacity(active_quin_count as usize);
    let header_size = 160usize;
    for i in 0..active_quin_count as usize {
        let start = header_size + (i * 48);
        quins.push([
            decode_u64(&raw_bytes, start)?,
            decode_u64(&raw_bytes, start + 8)?,
            decode_u64(&raw_bytes, start + 16)?,
            decode_u64(&raw_bytes, start + 24)?,
            decode_u64(&raw_bytes, start + 32)?,
            decode_u64(&raw_bytes, start + 40)?,
        ]);
    }

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
