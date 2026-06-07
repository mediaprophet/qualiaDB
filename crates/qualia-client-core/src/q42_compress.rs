//! LZ4 distribution artifacts — `.c.q42` for browser / WebTorrent deploy.
//!
//! This module currently preserves a transitional compatibility path where some
//! ingest outputs are already framed as LZ4 transport blocks. That behavior
//! should not be confused with the canonical raw `.q42` SuperBlock container.
//! Long term, `.c.q42` should be derived from raw `.q42`, not treated as the
//! same artifact under a different filename.

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use serde::Serialize;

const CHUNK_SIZE: usize = 393_216;

#[derive(Debug, Clone, Serialize)]
pub struct CompressStats {
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub blocks: u64,
    pub ratio: f64,
}

/// Finalize `{ontology_id}.q42` as `{ontology_id}.c.q42` for sharing.
///
/// Transitional note: this currently assumes the input is already a legacy
/// framed LZ4 transport artifact. It is therefore a compatibility shim, not
/// the canonical raw-`.q42` to `.c.q42` conversion path described by the new
/// internal format draft.
pub fn finalize_c_q42(q42_path: &Path, c_q42_path: &Path) -> Result<CompressStats, String> {
    if !q42_path.is_file() {
        return Err(format!("Missing .q42 artifact: {}", q42_path.display()));
    }
    if let Some(parent) = c_q42_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::copy(q42_path, c_q42_path).map_err(|e| e.to_string())?;
    let input_bytes = fs::metadata(q42_path).map_err(|e| e.to_string())?.len();
    let output_bytes = fs::metadata(c_q42_path).map_err(|e| e.to_string())?.len();
    let blocks = output_bytes.saturating_div(CHUNK_SIZE as u64).max(1);
    Ok(CompressStats {
        input_bytes,
        output_bytes,
        blocks,
        ratio: if output_bytes > 0 {
            input_bytes as f64 / output_bytes as f64
        } else {
            1.0
        },
    })
}

/// Extra LZ4 pass for non-ingest binaries (e.g. lexicon sidecars).
pub fn compress_raw_file(input: &Path, output: &Path) -> Result<CompressStats, String> {
    let input_bytes = fs::metadata(input).map_err(|e| e.to_string())?.len();
    let mut reader = fs::File::open(input).map_err(|e| e.to_string())?;
    let mut writer = fs::File::create(output).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut block_id: u64 = 0;
    loop {
        let n = read_partial(&mut reader, &mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        block_id = write_lz4_block(&mut writer, block_id, &buf[..n]).map_err(|e| e.to_string())?;
    }
    writer.flush().map_err(|e| e.to_string())?;
    let output_bytes = fs::metadata(output).map_err(|e| e.to_string())?.len();
    Ok(CompressStats {
        input_bytes,
        output_bytes,
        blocks: block_id,
        ratio: if output_bytes > 0 {
            input_bytes as f64 / output_bytes as f64
        } else {
            1.0
        },
    })
}

fn write_lz4_block(w: &mut impl Write, block_id: u64, data: &[u8]) -> std::io::Result<u64> {
    let compressed = lz4_flex::compress_prepend_size(data);
    w.write_all(&block_id.to_le_bytes())?;
    w.write_all(&(compressed.len() as u32).to_le_bytes())?;
    w.write_all(&(data.len() as u32).to_le_bytes())?;
    w.write_all(&compressed)?;
    Ok(block_id + 1)
}

fn read_partial(r: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut total = 0;
    while total < buf.len() {
        match r.read(&mut buf[total..])? {
            0 => break,
            n => total += n,
        }
    }
    Ok(total)
}
