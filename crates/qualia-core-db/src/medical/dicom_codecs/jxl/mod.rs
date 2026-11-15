//! ISO/IEC 18181-1 / 18181-2 JPEG-XL decoder for DICOM Transfer Syntaxes
//! `1.2.840.10008.1.2.4.110` (JPEG XL Lossless),
//! `1.2.840.10008.1.2.4.111` (JPEG XL JPEG Recompression), and
//! `1.2.840.10008.1.2.4.112` (JPEG XL).
mod color;
mod container;
mod entropy;
mod entropy_tables;
mod headers;
pub mod jbrd_reconstruct;
mod modular;
mod modular_transforms;
mod vardct;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod jxl_jbrd_tests;

pub use container::{
    extract_jbrd_payload, JxlEncoding, JxlError, JxlFrameType, JxlImageHeader, JxlLimits,
    JxlTransferFunction, TS_JXL, TS_JXL_JPEG_RECOMPRESSION, TS_JXL_LOSSLESS,
};
pub use jbrd_reconstruct::{
    encode_jbrd_box_from_jpeg, reconstruct_jpeg_from_jbrd, reconstruct_jpeg_from_jxl_container,
    JbrdExtractedCoeffs,
};

use super::encapsulated::{parse_encapsulated_part10, EncapLimits};
use color::pack_jxl_planes_into_output;
use container::unwrap_jxl_container;
use entropy::JxlBitReader;
use headers::{parse_frame_header, parse_image_header};
use modular::decode_modular_frame_planes;
use vardct::decode_vardct_frame_planes;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JxlFrameInfo<'a> {
    pub source_blake3: [u8; 32],
    pub transfer_syntax_uid: &'a str,
    pub rows: u16,
    pub columns: u16,
    pub frames: u32,
    pub samples_per_pixel: u16,
    pub bits_allocated: u16,
    pub bits_stored: u16,
    pub signed: bool,
    pub source_photometric: &'a str,
    pub encoding: JxlEncoding,
    pub has_jbrd_box: bool,
    pub bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JxlDecodedCodestream {
    pub encoding: JxlEncoding,
    pub has_jbrd_box: bool,
}

/// Decode one bare `FF 0A` codestream OR ISOBMFF `JXL ` container into `output`
/// (`bits_allocated = 8 | 16`). Fails closed leaving `output` untouched on any error.
pub fn decode_jxl_codestream_into(
    bytes: &[u8],
    expected_width: u16,
    expected_height: u16,
    expected_nc: u16,
    expected_ba: u16,
    expected_bs: u16,
    lossless_only: bool,
    limits: JxlLimits,
    output: &mut [u8],
) -> Result<JxlDecodedCodestream, JxlError> {
    let unwrapped = unwrap_jxl_container(bytes, limits.max_compressed_bytes)?;
    let cs = unwrapped.codestream.as_ref();
    let mut br = JxlBitReader::new(cs);
    let hdr = parse_image_header(&mut br, limits)?;
    if hdr.width != expected_width as u32 || hdr.height != expected_height as u32 {
        return Err(JxlError::DimensionMismatch);
    }
    let cs_nc = if hdr.is_gray { 1 } else { 3 } + hdr.num_extra_channels;
    if cs_nc != expected_nc {
        return Err(JxlError::CorruptStream("JXL component count mismatch"));
    }
    if hdr.bits_per_sample as u16 != expected_bs || expected_bs > expected_ba || !matches!(expected_ba, 8 | 16) {
        return Err(JxlError::BitPrecisionMismatch);
    }
    if hdr.floating_point {
        return Err(JxlError::Unsupported("floating-point JXL pixel depth unsupported for integer PixelData"));
    }
    let fh = parse_frame_header(&mut br, &hdr)?;
    if lossless_only && fh.encoding != JxlEncoding::Modular {
        return Err(JxlError::LosslessModeViolation);
    }
    if fh.frame_type != JxlFrameType::Regular {
        return Err(JxlError::Unsupported("non-regular JXL frame type unsupported"));
    }
    let bps = if expected_ba <= 8 { 1usize } else { 2usize };
    let need = (hdr.width as usize)
        .checked_mul(hdr.height as usize)
        .and_then(|v| v.checked_mul(expected_nc as usize))
        .and_then(|v| v.checked_mul(bps))
        .ok_or(JxlError::Overflow)?;
    if need > limits.max_decoded_bytes {
        return Err(JxlError::ByteLimit);
    }
    if output.len() < need {
        return Err(JxlError::OutputTooSmall);
    }

    br.skip_pad_to_byte()?;
    let byte_offset = br.bit_pos() / 8;
    let body_end = if let Some(&toc0) = fh.toc_lengths.first() {
        byte_offset.checked_add(toc0).ok_or(JxlError::Overflow)?
    } else {
        cs.len()
    };
    if body_end > cs.len() {
        return Err(JxlError::CorruptStream("JXL TOC section exceeds codestream size"));
    }
    let mut section_br = JxlBitReader::new(&cs[byte_offset..body_end]);
    let (planes, is_mod) = match fh.encoding {
        JxlEncoding::Modular => (decode_modular_frame_planes(&mut section_br, &hdr, limits)?, true),
        JxlEncoding::VarDct => (decode_vardct_frame_planes(&mut section_br, &hdr, &fh, limits)?, false),
    };

    let mut staged = vec![0u8; need];
    pack_jxl_planes_into_output(&hdr, &planes, is_mod, expected_nc, expected_ba, expected_bs, &mut staged)?;
    output[..need].copy_from_slice(&staged);
    Ok(JxlDecodedCodestream { encoding: fh.encoding, has_jbrd_box: unwrapped.has_jbrd_box })
}

/// Decode encapsulated DICOM Part-10 JPEG-XL Transfer Syntaxes (`.110`, `.111`, `.112`).
pub fn decode_jxl_part10_into<'a>(
    part10_bytes: &'a [u8],
    output: &mut [u8],
    limits: JxlLimits,
) -> Result<JxlFrameInfo<'a>, JxlError> {
    let encap = parse_encapsulated_part10(
        part10_bytes,
        &[TS_JXL_LOSSLESS, TS_JXL_JPEG_RECOMPRESSION, TS_JXL],
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
    if !matches!(encap.samples_per_pixel, 1 | 3 | 4) {
        return Err(JxlError::Unsupported("JPEG-XL DICOM supports 1, 3, or 4 samples per pixel"));
    }
    if encap.samples_per_pixel == 1 {
        if !matches!(encap.photometric, "MONOCHROME1" | "MONOCHROME2") {
            return Err(JxlError::Unsupported("monochrome JPEG-XL requires MONOCHROME1/2"));
        }
    } else if !matches!(encap.photometric, "RGB" | "RGBA") || encap.planar_configuration.is_some_and(|p| p != 0) {
        return Err(JxlError::Unsupported("color JPEG-XL requires RGB/RGBA PlanarConfiguration 0"));
    }
    if !matches!(encap.bits_allocated, 8 | 16) || encap.bits_stored == 0 || encap.bits_stored > encap.bits_allocated {
        return Err(JxlError::BitPrecisionMismatch);
    }
    let bps = if encap.bits_allocated <= 8 { 1usize } else { 2usize };
    let frame_bytes = (encap.rows as usize)
        .checked_mul(encap.columns as usize)
        .and_then(|v| v.checked_mul(encap.samples_per_pixel as usize))
        .and_then(|v| v.checked_mul(bps))
        .ok_or(JxlError::Overflow)?;
    let total_bytes = frame_bytes.checked_mul(encap.frames as usize).ok_or(JxlError::Overflow)?;
    if total_bytes > limits.max_decoded_bytes {
        return Err(JxlError::ByteLimit);
    }
    if output.len() < total_bytes {
        return Err(JxlError::OutputTooSmall);
    }

    let lossless_only = encap.transfer_syntax_uid == TS_JXL_LOSSLESS;
    let mut staged = vec![0u8; total_bytes];
    let mut enc_out = JxlEncoding::Modular;
    let mut has_jbrd = false;

    for f in 0..encap.frames as usize {
        let fb = encap.frame_bytes(f, limits.max_compressed_bytes)?;
        let res = decode_jxl_codestream_into(
            fb.as_ref(),
            encap.columns,
            encap.rows,
            encap.samples_per_pixel,
            encap.bits_allocated,
            encap.bits_stored,
            lossless_only,
            limits,
            &mut staged[f * frame_bytes..(f + 1) * frame_bytes],
        )?;
        enc_out = res.encoding;
        has_jbrd |= res.has_jbrd_box;
    }

    output[..total_bytes].copy_from_slice(&staged);
    Ok(JxlFrameInfo {
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
        encoding: enc_out,
        has_jbrd_box: has_jbrd,
        bytes: total_bytes,
    })
}
