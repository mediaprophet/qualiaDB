//! ITU-T T.87 / ISO 14495-1 JPEG-LS decoder for DICOM Transfer Syntaxes
//! `1.2.840.10008.1.2.4.80` (Lossless) and `1.2.840.10008.1.2.4.81` (Near-Lossless).
mod bitstream;
mod context;
mod headers;
#[cfg(test)]
mod tests;

pub use headers::{
    JpegLsError, JpegLsFrameInfo, JpegLsLimits, TS_JPEGLS_LOSSLESS, TS_JPEGLS_NEAR_LOSSLESS,
};

use super::encapsulated::{parse_encapsulated_part10, EncapLimits};
use bitstream::LsBitReader;
use context::ContextModel;
use headers::{derive_coding, parse_jpegls_frame};

/// Decode one bare ITU-T T.87 JPEG-LS codestream into caller-owned `output` bytes
/// (8-bit pixels as 1 byte/sample; 9..16-bit pixels as little-endian `u16`).
/// Output is never modified if validation or entropy decoding fails. Returns `near_max`.
pub fn decode_jpegls_codestream_into(
    codestream: &[u8],
    expected_width: u16,
    expected_height: u16,
    expected_components: u16,
    expected_bits_allocated: u16,
    expected_bits_stored: u16,
    lossless_only: bool,
    output: &mut [u8],
) -> Result<u8, JpegLsError> {
    let parsed = parse_jpegls_frame(codestream)?;
    if parsed.width != expected_width || parsed.height != expected_height {
        return Err(JpegLsError::DimensionMismatch);
    }
    if parsed.comp_count as u16 != expected_components {
        return Err(JpegLsError::CorruptStream("component count mismatch"));
    }
    if parsed.precision as u16 != expected_bits_stored {
        return Err(JpegLsError::BitPrecisionMismatch);
    }
    let bytes_per_sample = if expected_bits_allocated <= 8 { 1usize } else { 2usize };
    let w = parsed.width as usize;
    let h = parsed.height as usize;
    let nc = parsed.comp_count as usize;
    let need = w
        .checked_mul(h)
        .and_then(|n| n.checked_mul(nc))
        .and_then(|n| n.checked_mul(bytes_per_sample))
        .ok_or(JpegLsError::Overflow)?;
    if output.len() < need {
        return Err(JpegLsError::OutputTooSmall);
    }

    let mut planes = Vec::new();
    planes
        .try_reserve_exact(nc)
        .map_err(|_| JpegLsError::ByteLimit)?;
    for _ in 0..nc {
        let mut p = Vec::new();
        p.try_reserve_exact(w * h).map_err(|_| JpegLsError::ByteLimit)?;
        p.resize(w * h, 0i32);
        planes.push(p);
    }
    let mut decoded_mask = 0u8;
    let mut near_max = 0u8;

    for scan in &parsed.scans {
        if lossless_only && scan.near != 0 {
            return Err(JpegLsError::LosslessModeViolation);
        }
        near_max = near_max.max(scan.near);
        for &ci in &scan.comp_indices {
            if (decoded_mask & (1u8 << ci)) != 0 {
                return Err(JpegLsError::CorruptStream("duplicate component decoded across scans"));
            }
            decoded_mask |= 1u8 << ci;
        }
        let coding = derive_coding(parsed.precision, scan.near, scan.preset)?;
        let mut model = ContextModel::new(coding);
        let mut reader = LsBitReader::new(scan.entropy_and_tail);
        let mut rst_idx = 0u8;
        match scan.ilv {
            0 => {
                let ci = scan.comp_indices[0];
                let mut prev = vec![0i32; w + 2];
                let mut curr = vec![0i32; w + 2];
                let mut run_index = 0usize;
                for y in 0..h {
                    if scan.restart_interval > 0 && y > 0 && (y % scan.restart_interval as usize) == 0 {
                        reader.consume_restart(rst_idx)?;
                        rst_idx = rst_idx.wrapping_add(1);
                        model.reset_state();
                        prev.fill(0);
                        run_index = 0;
                    }
                    model.decode_line(&mut reader, &mut prev, &mut curr, w, &mut run_index)?;
                    planes[ci][y * w..(y + 1) * w].copy_from_slice(&curr[1..=w]);
                    std::mem::swap(&mut prev, &mut curr);
                }
            }
            1 => {
                let snc = scan.comp_indices.len();
                let mut prev: Vec<Vec<i32>> = (0..snc).map(|_| vec![0i32; w + 2]).collect();
                let mut curr: Vec<Vec<i32>> = (0..snc).map(|_| vec![0i32; w + 2]).collect();
                let mut run_index = vec![0usize; snc];
                for y in 0..h {
                    if scan.restart_interval > 0 && y > 0 && (y % scan.restart_interval as usize) == 0 {
                        reader.consume_restart(rst_idx)?;
                        rst_idx = rst_idx.wrapping_add(1);
                        model.reset_state();
                        for p in &mut prev { p.fill(0); }
                        run_index.fill(0);
                    }
                    for (si, &ci) in scan.comp_indices.iter().enumerate() {
                        model.decode_line(&mut reader, &mut prev[si], &mut curr[si], w, &mut run_index[si])?;
                        planes[ci][y * w..(y + 1) * w].copy_from_slice(&curr[si][1..=w]);
                        std::mem::swap(&mut prev[si], &mut curr[si]);
                    }
                }
            }
            2 => {
                let snc = scan.comp_indices.len();
                let mut prev: Vec<Vec<i32>> = (0..snc).map(|_| vec![0i32; w + 2]).collect();
                let mut curr: Vec<Vec<i32>> = (0..snc).map(|_| vec![0i32; w + 2]).collect();
                let mut run_index = 0usize;
                for y in 0..h {
                    if scan.restart_interval > 0 && y > 0 && (y % scan.restart_interval as usize) == 0 {
                        reader.consume_restart(rst_idx)?;
                        rst_idx = rst_idx.wrapping_add(1);
                        model.reset_state();
                        for p in &mut prev { p.fill(0); }
                        run_index = 0;
                    }
                    model.decode_sample_interleaved_line(&mut reader, &mut prev, &mut curr, w, &mut run_index)?;
                    for (si, &ci) in scan.comp_indices.iter().enumerate() {
                        planes[ci][y * w..(y + 1) * w].copy_from_slice(&curr[si][1..=w]);
                        std::mem::swap(&mut prev[si], &mut curr[si]);
                    }
                }
            }
            _ => return Err(JpegLsError::CorruptStream("invalid ILV")),
        }
    }

    let expected_mask = (1u8 << nc) - 1;
    if decoded_mask != expected_mask {
        return Err(JpegLsError::CorruptStream("missing JPEG-LS scan for frame component"));
    }

    // Write pixel-interleaved decoded frame atomically after full validation.
    if bytes_per_sample == 1 {
        for px in 0..w * h {
            for c in 0..nc {
                output[px * nc + c] = planes[c][px] as u8;
            }
        }
    } else {
        for px in 0..w * h {
            for c in 0..nc {
                let le = (planes[c][px] as u16).to_le_bytes();
                let dst = (px * nc + c) * 2;
                output[dst] = le[0];
                output[dst + 1] = le[1];
            }
        }
    }
    Ok(near_max)
}

/// Decode encapsulated DICOM JPEG-LS Part-10 bytes (`1.2.840.10008.1.2.4.80` / `.81`)
/// into `output`, leaving `output` untouched on any framing, budget or stream failure.
pub fn decode_jpegls_part10_into<'a>(
    part10_bytes: &'a [u8],
    output: &mut [u8],
    limits: JpegLsLimits,
) -> Result<JpegLsFrameInfo<'a>, JpegLsError> {
    let encap = parse_encapsulated_part10(
        part10_bytes,
        &[TS_JPEGLS_LOSSLESS, TS_JPEGLS_NEAR_LOSSLESS],
        EncapLimits {
            max_part10_bytes: limits.max_part10_bytes,
            max_decoded_bytes: limits.max_decoded_bytes,
            max_compressed_bytes: limits.max_compressed_bytes,
            max_frames: limits.max_frames,
            max_fragments: limits.max_fragments,
            max_width: limits.max_width,
            max_height: limits.max_height,
            max_elements: 8192,
        },
    )?;
    if !matches!(encap.samples_per_pixel, 1 | 3) {
        return Err(JpegLsError::Unsupported("JPEG-LS SamplesPerPixel must be 1 or 3"));
    }
    if encap.samples_per_pixel == 1 {
        if !matches!(encap.photometric, "MONOCHROME1" | "MONOCHROME2") || encap.planar_configuration.is_some() {
            return Err(JpegLsError::Unsupported("monochrome JPEG-LS requires MONOCHROME1/2 without PlanarConfiguration"));
        }
    } else if !matches!(encap.photometric, "RGB" | "YBR_RCT" | "YBR_ICT") || encap.planar_configuration.is_some_and(|p| p != 0) {
        return Err(JpegLsError::Unsupported("color JPEG-LS requires RGB/YBR PlanarConfiguration 0"));
    }
    if !matches!(encap.bits_allocated, 8 | 16) || !(2..=16).contains(&encap.bits_stored) || encap.bits_stored > encap.bits_allocated {
        return Err(JpegLsError::BitPrecisionMismatch);
    }
    let bps = (encap.bits_allocated / 8) as usize;
    let frame_bytes = (encap.rows as usize)
        .checked_mul(encap.columns as usize)
        .and_then(|v| v.checked_mul(encap.samples_per_pixel as usize))
        .and_then(|v| v.checked_mul(bps))
        .ok_or(JpegLsError::Overflow)?;
    let total_bytes = frame_bytes.checked_mul(encap.frames as usize).ok_or(JpegLsError::Overflow)?;
    if total_bytes > limits.max_decoded_bytes {
        return Err(JpegLsError::ByteLimit);
    }
    if output.len() < total_bytes {
        return Err(JpegLsError::OutputTooSmall);
    }

    let mut staged = Vec::new();
    staged.try_reserve_exact(total_bytes).map_err(|_| JpegLsError::ByteLimit)?;
    staged.resize(total_bytes, 0u8);
    let lossless_only = encap.transfer_syntax_uid == TS_JPEGLS_LOSSLESS;
    let mut near_max = 0u8;

    for f in 0..encap.frames as usize {
        let frag_bytes = encap.frame_bytes(f, limits.max_compressed_bytes)?;
        let dst = &mut staged[f * frame_bytes..(f + 1) * frame_bytes];
        let near = decode_jpegls_codestream_into(
            frag_bytes.as_ref(),
            encap.columns,
            encap.rows,
            encap.samples_per_pixel,
            encap.bits_allocated,
            encap.bits_stored,
            lossless_only,
            dst,
        )?;
        near_max = near_max.max(near);
    }

    output[..total_bytes].copy_from_slice(&staged);
    Ok(JpegLsFrameInfo {
        source_blake3: encap.source_blake3,
        transfer_syntax_uid: encap.transfer_syntax_uid,
        rows: encap.rows,
        columns: encap.columns,
        frames: encap.frames,
        samples_per_pixel: encap.samples_per_pixel,
        bits_allocated: encap.bits_allocated,
        bits_stored: encap.bits_stored,
        signed: encap.pixel_representation == 1,
        source_photometric: encap.photometric,
        near_max,
        bytes: total_bytes,
    })
}
