//! LZ4 block-stream compressor for browser-deployable dataset artifacts.
//!
//! Unified v2 `.q42` volumes already embed LZ4-compressed SuperBlocks; the
//! compress command copies them unchanged. Legacy v1 raw SuperBlock streams are
//! still converted to the framed transport format for the browser VFS:
//!
//! ```text
//! Per block:
//!   [block_id:   u64 LE]
//!   [comp_len:   u32 LE]
//!   [uncomp_len: u32 LE]
//!   [payload:    comp_len bytes]   — lz4_flex::compress_prepend_size output
//! ```

use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

/// Bytes per compressed chunk.  8 192 × 48 = 393 216 — the same size used by
/// the `import` pipeline, so the browser decoder is identical for both sources.
const CHUNK_SIZE: usize = 393_216;

/// SuperBlock layout constants (must match qualia-core-db).
const SUPERBLOCK_SIZE: usize = 40_960;
const SUPERBLOCK_HEADER: usize = 160;
const QUINS_PER_BLOCK: usize = 850;
const QUIN_SIZE: usize = 48;
const QUIN_DATA_PER_BLOCK: usize = QUINS_PER_BLOCK * QUIN_SIZE; // 40 800 bytes

pub struct CompressStats {
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub blocks: u64,
    pub ratio: f64,
}

/// Compress a `.q42` file. Unified v2 volumes are copied as-is; legacy v1
/// SuperBlock streams are stripped and re-framed for the browser VFS.
pub fn compress_q42(
    input: &Path,
    output: &Path,
) -> Result<CompressStats, Box<dyn std::error::Error>> {
    if qualia_core_db::q42_volume::is_v2_volume(input)? {
        fs::copy(input, output)?;
        let input_bytes = fs::metadata(input)?.len();
        let output_bytes = fs::metadata(output)?.len();
        return Ok(CompressStats {
            input_bytes,
            output_bytes,
            blocks: qualia_core_db::q42_volume::Q42Volume::open(input)?.block_count(),
            ratio: 1.0,
        });
    }

    let meta = fs::metadata(input)?;
    let input_bytes = meta.len();

    let mut reader = BufReader::new(File::open(input)?);
    let out_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output)?;
    let mut writer = BufWriter::new(out_file);

    let mut chunk: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);
    let mut block_id: u64 = 0;
    let mut sb_buf = vec![0u8; SUPERBLOCK_SIZE];

    loop {
        let n = read_exact_or_eof(&mut reader, &mut sb_buf)?;
        if n == 0 {
            break;
        }
        if n < SUPERBLOCK_SIZE {
            break;
        }

        chunk
            .extend_from_slice(&sb_buf[SUPERBLOCK_HEADER..SUPERBLOCK_HEADER + QUIN_DATA_PER_BLOCK]);

        if chunk.len() >= CHUNK_SIZE {
            block_id = write_lz4_block(&mut writer, block_id, &chunk[..CHUNK_SIZE])?;
            chunk.drain(..CHUNK_SIZE);
        }
    }

    if !chunk.is_empty() {
        block_id = write_lz4_block(&mut writer, block_id, &chunk)?;
    }

    writer.flush()?;
    let output_bytes = fs::metadata(output)?.len();

    Ok(CompressStats {
        input_bytes,
        output_bytes,
        blocks: block_id,
        ratio: input_bytes as f64 / output_bytes as f64,
    })
}

/// Compress any binary file (e.g. legacy `.lex` sidecar) as raw bytes.
pub fn compress_raw(
    input: &Path,
    output: &Path,
) -> Result<CompressStats, Box<dyn std::error::Error>> {
    let meta = fs::metadata(input)?;
    let input_bytes = meta.len();

    let mut reader = BufReader::new(File::open(input)?);
    let out_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output)?;
    let mut writer = BufWriter::new(out_file);

    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut block_id: u64 = 0;

    loop {
        let n = read_exact_or_eof(&mut reader, &mut buf)?;
        if n == 0 {
            break;
        }
        block_id = write_lz4_block(&mut writer, block_id, &buf[..n])?;
    }

    writer.flush()?;
    let output_bytes = fs::metadata(output)?.len();

    Ok(CompressStats {
        input_bytes,
        output_bytes,
        blocks: block_id,
        ratio: input_bytes as f64 / output_bytes as f64,
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

fn read_exact_or_eof(r: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut total = 0;
    while total < buf.len() {
        match r.read(&mut buf[total..])? {
            0 => break,
            n => total += n,
        }
    }
    Ok(total)
}
