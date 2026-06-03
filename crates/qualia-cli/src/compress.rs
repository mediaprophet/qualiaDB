//! LZ4 block-stream compressor for browser-deployable dataset artifacts.
//!
//! Produces the format the browser VFS expects:
//!
//! ```text
//! Per block:
//!   [block_id:   u64 LE]   — monotonically increasing
//!   [comp_len:   u32 LE]   — byte length of the lz4_flex payload that follows
//!   [uncomp_len: u32 LE]   — byte length of the original chunk
//!   [payload:    comp_len bytes]   — lz4_flex::compress_prepend_size output
//!                                    (first 4 bytes = uncomp_len LE, then LZ4 block)
//! ```
//!
//! For `.q42` SuperBlock input, the 160-byte block headers are stripped so the
//! decompressed output is a flat sequence of 48-byte Quins — no header offsets
//! needed in the browser scanner.
//!
//! For any other input (e.g. `.lex`) the raw bytes are chunked and compressed.

use std::fs::{File, OpenOptions};
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
    pub input_bytes:  u64,
    pub output_bytes: u64,
    pub blocks:       u64,
    pub ratio:        f64,
}

/// Compress a `.q42` SuperBlock file: strip 160-byte headers, emit raw Quins
/// chunked and LZ4-compressed.
pub fn compress_q42(input: &Path, output: &Path) -> Result<CompressStats, Box<dyn std::error::Error>> {
    let meta = std::fs::metadata(input)?;
    let input_bytes = meta.len();

    let mut reader = BufReader::new(File::open(input)?);
    let out_file = OpenOptions::new().create(true).write(true).truncate(true).open(output)?;
    let mut writer = BufWriter::new(out_file);

    let mut chunk: Vec<u8> = Vec::with_capacity(CHUNK_SIZE);
    let mut block_id: u64 = 0;
    let mut sb_buf = vec![0u8; SUPERBLOCK_SIZE];

    loop {
        let n = read_exact_or_eof(&mut reader, &mut sb_buf)?;
        if n == 0 { break; }
        if n < SUPERBLOCK_SIZE { break; } // partial final block — skip

        // Skip the 160-byte header, take only Quin data.
        chunk.extend_from_slice(&sb_buf[SUPERBLOCK_HEADER..SUPERBLOCK_HEADER + QUIN_DATA_PER_BLOCK]);

        if chunk.len() >= CHUNK_SIZE {
            block_id = write_lz4_block(&mut writer, block_id, &chunk[..CHUNK_SIZE])?;
            chunk.drain(..CHUNK_SIZE);
        }
    }

    // Flush remaining Quins (last partial chunk).
    if !chunk.is_empty() {
        block_id = write_lz4_block(&mut writer, block_id, &chunk)?;
    }

    writer.flush()?;
    let output_bytes = std::fs::metadata(output)?.len();

    Ok(CompressStats {
        input_bytes,
        output_bytes,
        blocks: block_id,
        ratio: input_bytes as f64 / output_bytes as f64,
    })
}

/// Compress any binary file (e.g. `.lex`) as raw bytes.
pub fn compress_raw(input: &Path, output: &Path) -> Result<CompressStats, Box<dyn std::error::Error>> {
    let meta = std::fs::metadata(input)?;
    let input_bytes = meta.len();

    let mut reader = BufReader::new(File::open(input)?);
    let out_file = OpenOptions::new().create(true).write(true).truncate(true).open(output)?;
    let mut writer = BufWriter::new(out_file);

    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut block_id: u64 = 0;

    loop {
        let n = read_exact_or_eof(&mut reader, &mut buf)?;
        if n == 0 { break; }
        block_id = write_lz4_block(&mut writer, block_id, &buf[..n])?;
    }

    writer.flush()?;
    let output_bytes = std::fs::metadata(output)?.len();

    Ok(CompressStats {
        input_bytes,
        output_bytes,
        blocks: block_id,
        ratio: input_bytes as f64 / output_bytes as f64,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Write one LZ4 block (16-byte header + lz4_flex payload).
/// Returns the next block_id.
fn write_lz4_block(
    w: &mut impl Write,
    block_id: u64,
    data: &[u8],
) -> std::io::Result<u64> {
    let compressed = lz4_flex::compress_prepend_size(data);
    w.write_all(&block_id.to_le_bytes())?;
    w.write_all(&(compressed.len() as u32).to_le_bytes())?;
    w.write_all(&(data.len() as u32).to_le_bytes())?;
    w.write_all(&compressed)?;
    Ok(block_id + 1)
}

/// Read exactly `buf.len()` bytes, or return the number of bytes read if EOF.
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
