//! ISO/IEC 18181-1 & 18181-2 JPEG-XL container (`JXL ` ISOBMFF + `jxlc`/`jxlp`/`jbrd`),
//! bare codestream (`FF 0A`) signature unwrapper, types, and limits.
use crate::medical::dicom_codecs::encapsulated::EncapError;
use std::borrow::Cow;

pub const TS_JXL_LOSSLESS: &str = "1.2.840.10008.1.2.4.110";
pub const TS_JXL_JPEG_RECOMPRESSION: &str = "1.2.840.10008.1.2.4.111";
pub const TS_JXL: &str = "1.2.840.10008.1.2.4.112";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JxlError {
    Encap(EncapError),
    Unsupported(&'static str),
    OutputTooSmall,
    ByteLimit,
    FrameLimit,
    DimensionLimit,
    Overflow,
    CorruptStream(&'static str),
    DimensionMismatch,
    BitPrecisionMismatch,
    LosslessModeViolation,
}

impl From<EncapError> for JxlError {
    fn from(e: EncapError) -> Self {
        Self::Encap(e)
    }
}
impl std::fmt::Display for JxlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DICOM JPEG-XL: {self:?}")
    }
}
impl std::error::Error for JxlError {}

#[derive(Clone, Copy, Debug)]
pub struct JxlLimits {
    pub max_part10_bytes: usize,
    pub max_decoded_bytes: usize,
    pub max_compressed_bytes: usize,
    pub max_frames: u32,
    pub max_fragments: usize,
    pub max_width: u16,
    pub max_height: u16,
    pub max_extra_channels: usize,
    pub max_tree_nodes: usize,
}

impl Default for JxlLimits {
    fn default() -> Self {
        Self {
            max_part10_bytes: 128 * 1024 * 1024,
            max_decoded_bytes: 256 * 1024 * 1024,
            max_compressed_bytes: 64 * 1024 * 1024,
            max_frames: 4096,
            max_fragments: 65536,
            max_width: 16384,
            max_height: 16384,
            max_extra_channels: 8,
            max_tree_nodes: 16384,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JxlEncoding {
    VarDct,
    Modular,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JxlFrameType {
    Regular,
    LfFrame,
    ReferenceOnly,
    SkipProgressive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JxlTransferFunction {
    Srgb,
    Linear,
    Pq,
    Hlg,
    Gamma(u32),
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct JxlBoxResult<'a> {
    pub codestream: Cow<'a, [u8]>,
    pub has_jbrd_box: bool,
    pub jbrd: Option<&'a [u8]>,
    pub is_container: bool,
}

#[derive(Clone, Debug)]
pub struct JxlImageHeader {
    pub width: u32,
    pub height: u32,
    pub bits_per_sample: u8,
    pub floating_point: bool,
    pub xyb_encoded: bool,
    pub is_gray: bool,
    pub num_extra_channels: u16,
    pub has_alpha: bool,
    pub transfer: JxlTransferFunction,
    pub opsin_inverse_matrix: [[f64; 3]; 3],
    pub opsin_bias: [f64; 3],
    pub cfl_default_factors: bool,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct JxlFrameHeader {
    pub frame_type: JxlFrameType,
    pub encoding: JxlEncoding,
    pub flags: u64,
    pub do_ycbcr: bool,
    pub group_size_shift: u8,
    pub num_passes: u32,
    pub is_last: bool,
    pub gab: bool,
    pub epf_iters: u8,
    pub toc_lengths: Vec<usize>,
    pub x_qm_scale: u8,
    pub b_qm_scale: u8,
}

pub const JXL_CONTAINER_SIG: [u8; 12] = [
    0x00, 0x00, 0x00, 0x0C, b'J', b'X', b'L', b' ', 0x0D, 0x0A, 0x87, 0x0A,
];

/// Default ISO/IEC 18181-1 opsin forward matrix (`M_1`) inverted analytically at runtime.
pub fn default_opsin_inverse_matrix() -> [[f64; 3]; 3] {
    let m = [
        [0.300, 0.622, 0.078],
        [0.230, 0.692, 0.078],
        [0.243_422_689_245_478, 0.204_767_444_244_968, 0.551_809_866_509_554],
    ];
    invert_3x3(m).unwrap_or([[11.031567, -9.866944, -0.164623], [-3.254147, 4.418770, -0.164623], [-3.658851, 2.712923, 1.945928]])
}

pub fn invert_3x3(m: [[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let c00 = m[1][1] * m[2][2] - m[1][2] * m[2][1];
    let c01 = -(m[1][0] * m[2][2] - m[1][2] * m[2][0]);
    let c02 = m[1][0] * m[2][1] - m[1][1] * m[2][0];
    let det = m[0][0] * c00 + m[0][1] * c01 + m[0][2] * c02;
    if det.abs() < 1e-15 || !det.is_finite() {
        return None;
    }
    let inv = 1.0 / det;
    let c10 = -(m[0][1] * m[2][2] - m[0][2] * m[2][1]);
    let c11 = m[0][0] * m[2][2] - m[0][2] * m[2][0];
    let c12 = -(m[0][0] * m[2][1] - m[0][1] * m[2][0]);
    let c20 = m[0][1] * m[1][2] - m[0][2] * m[1][1];
    let c21 = -(m[0][0] * m[1][2] - m[0][2] * m[1][0]);
    let c22 = m[0][0] * m[1][1] - m[0][1] * m[1][0];
    Some([
        [c00 * inv, c10 * inv, c20 * inv],
        [c01 * inv, c11 * inv, c21 * inv],
        [c02 * inv, c12 * inv, c22 * inv],
    ])
}

/// Unwrap ISOBMFF `JXL ` container boxes (`jxlc`, sequenced `jxlp`, and `jbrd` detection)
/// or accept a bare `FF 0A` codestream directly, subject to `max_compressed_bytes`.
pub fn unwrap_jxl_container<'a>(bytes: &'a [u8], max_compressed_bytes: usize) -> Result<JxlBoxResult<'a>, JxlError> {
    if bytes.len() > max_compressed_bytes {
        return Err(JxlError::ByteLimit);
    }
    if bytes.starts_with(&[0xFF, 0x0A]) {
        return Ok(JxlBoxResult {
            codestream: Cow::Borrowed(bytes),
            has_jbrd_box: false,
            jbrd: None,
            is_container: false,
        });
    }
    if !bytes.starts_with(&JXL_CONTAINER_SIG) {
        return Err(JxlError::CorruptStream("missing JPEG-XL FF0A or ISOBMFF JXL signature"));
    }
    let mut pos = JXL_CONTAINER_SIG.len();
    let mut jxlc: Option<&'a [u8]> = None;
    let mut jxlp_parts: Vec<(u32, &'a [u8])> = Vec::new();
    let mut jbrd: Option<&'a [u8]> = None;
    let mut saw_last_jxlp = false;

    while pos < bytes.len() {
        if pos + 8 > bytes.len() {
            return Err(JxlError::CorruptStream("truncated ISOBMFF box header"));
        }
        let size32 = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]) as usize;
        let box_type = &bytes[pos + 4..pos + 8];
        let (hdr_len, box_end) = if size32 == 1 {
            if pos + 16 > bytes.len() {
                return Err(JxlError::CorruptStream("truncated ISOBMFF 64-bit box header"));
            }
            let s64 = u64::from_be_bytes(bytes[pos + 8..pos + 16].try_into().map_err(|_| JxlError::CorruptStream("box size"))?);
            let s = usize::try_from(s64).map_err(|_| JxlError::Overflow)?;
            if s < 16 {
                return Err(JxlError::CorruptStream("invalid 64-bit box length"));
            }
            (16usize, pos.checked_add(s).ok_or(JxlError::Overflow)?)
        } else if size32 == 0 {
            (8usize, bytes.len())
        } else if size32 < 8 {
            return Err(JxlError::CorruptStream("invalid ISOBMFF box size < 8"));
        } else {
            (8usize, pos.checked_add(size32).ok_or(JxlError::Overflow)?)
        };
        if box_end > bytes.len() {
            return Err(JxlError::CorruptStream("ISOBMFF box extends past stream end"));
        }
        let payload = &bytes[pos + hdr_len..box_end];
        match box_type {
            b"jxlc" => {
                if jxlc.is_some() || !jxlp_parts.is_empty() {
                    return Err(JxlError::CorruptStream("duplicate jxlc codestream box"));
                }
                jxlc = Some(payload);
            }
            b"jxlp" => {
                if jxlc.is_some() || payload.len() < 4 {
                    return Err(JxlError::CorruptStream("invalid jxlp partial codestream box"));
                }
                let idx_raw = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                if (idx_raw & 0x8000_0000) != 0 {
                    saw_last_jxlp = true;
                }
                jxlp_parts.push((idx_raw & 0x7FFF_FFFF, &payload[4..]));
            }
            b"jbrd" => {
                if jbrd.is_some() {
                    return Err(JxlError::CorruptStream("duplicate jbrd reconstruction box"));
                }
                if payload.is_empty() {
                    return Err(JxlError::CorruptStream("empty jbrd reconstruction box"));
                }
                jbrd = Some(payload);
            }
            _ => {}
        }
        pos = box_end;
    }

    if let Some(cs) = jxlc {
        if !cs.starts_with(&[0xFF, 0x0A]) {
            return Err(JxlError::CorruptStream("jxlc payload missing FF0A signature"));
        }
        return Ok(JxlBoxResult {
            codestream: Cow::Borrowed(cs),
            has_jbrd_box: jbrd.is_some(),
            jbrd,
            is_container: true,
        });
    }
    if jxlp_parts.is_empty() || !saw_last_jxlp {
        return Err(JxlError::CorruptStream("JXL container missing jxlc/jxlp codestream"));
    }
    jxlp_parts.sort_by_key(|p| p.0);
    for (expected, (idx, _)) in jxlp_parts.iter().enumerate() {
        if *idx as usize != expected {
            return Err(JxlError::CorruptStream("non-contiguous jxlp sequence indices"));
        }
    }
    let total: usize = jxlp_parts.iter().map(|p| p.1.len()).sum();
    if total > max_compressed_bytes {
        return Err(JxlError::ByteLimit);
    }
    let mut joined = Vec::new();
    joined.try_reserve_exact(total).map_err(|_| JxlError::ByteLimit)?;
    for (_, part) in jxlp_parts {
        joined.extend_from_slice(part);
    }
    if !joined.starts_with(&[0xFF, 0x0A]) {
        return Err(JxlError::CorruptStream("reassembled jxlp missing FF0A signature"));
    }
    Ok(JxlBoxResult {
        codestream: Cow::Owned(joined),
        has_jbrd_box: jbrd.is_some(),
        jbrd,
        is_container: true,
    })
}

/// Validate ISOBMFF JXL container framing and extract the `jbrd` JPEG reconstruction box
/// payload (`.111`), failing closed if missing, empty, duplicated, or truncated.
pub fn extract_jbrd_payload<'a>(bytes: &'a [u8], limits: JxlLimits) -> Result<&'a [u8], JxlError> {
    let unwrapped = unwrap_jxl_container(bytes, limits.max_compressed_bytes)?;
    unwrapped.jbrd.ok_or(JxlError::CorruptStream("missing jbrd box for JPEG reconstruction"))
}
