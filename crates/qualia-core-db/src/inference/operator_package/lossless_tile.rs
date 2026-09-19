//! Lossless tile codec module (Wave 8: EOS-080).
//!
//! Provides bit-exact payload tile transforms:
//! - Raw escape (uncompressed tile)
//! - Shared-tile references (deduplication)
//! - XOR deltas (bitwise difference against an anchor tile)
//! - Exponent/significand separation for 16-bit floats
//! - Zero-heap hot path decoding into caller-supplied buffers

use std::fmt;

/// Codec scheme used to compress an individual payload tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TileCodecKind {
    /// Raw uncompressed payload bytes.
    RawEscape = 0,
    /// Exact duplicate referencing an existing anchor tile.
    SharedReference = 1,
    /// Bitwise XOR delta against an anchor tile.
    XorDelta = 2,
    /// Transposed 16-bit float bytes (exponents separated from mantissas).
    SplitExpSignificand = 3,
}

impl TryFrom<u8> for TileCodecKind {
    type Error = TileCodecError;

    fn try_from(val: u8) -> Result<Self, Self::Error> {
        match val {
            0 => Ok(TileCodecKind::RawEscape),
            1 => Ok(TileCodecKind::SharedReference),
            2 => Ok(TileCodecKind::XorDelta),
            3 => Ok(TileCodecKind::SplitExpSignificand),
            _ => Err(TileCodecError::MalformedHeader),
        }
    }
}

/// Errors from tile encoding or decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileCodecError {
    BufferTooSmall,
    MalformedHeader,
    AnchorMissing,
    AnchorMismatch,
    TruncatedPayload,
    InvalidAlignment,
}

impl fmt::Display for TileCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "destination buffer too small for decoded tile"),
            Self::MalformedHeader => write!(f, "tile header malformed or unrecognized codec kind"),
            Self::AnchorMissing => write!(f, "anchor tile required for delta/reference decode was missing"),
            Self::AnchorMismatch => write!(f, "anchor tile dimensions or length mismatch"),
            Self::TruncatedPayload => write!(f, "encoded payload was truncated"),
            Self::InvalidAlignment => write!(f, "payload tile size not aligned to float element width"),
        }
    }
}

impl std::error::Error for TileCodecError {}

/// Fixed 12-byte header describing an encoded tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileHeader {
    pub kind: TileCodecKind,
    pub anchor_index: u16,
    pub original_len: u32,
    pub encoded_len: u32,
}

impl TileHeader {
    pub const BYTES: usize = 12;

    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        let mut buf = [0u8; Self::BYTES];
        buf[0] = self.kind as u8;
        buf[1] = 0; // reserved
        buf[2..4].copy_from_slice(&self.anchor_index.to_le_bytes());
        buf[4..8].copy_from_slice(&self.original_len.to_le_bytes());
        buf[8..12].copy_from_slice(&self.encoded_len.to_le_bytes());
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, TileCodecError> {
        if bytes.len() < Self::BYTES {
            return Err(TileCodecError::MalformedHeader);
        }
        let kind = TileCodecKind::try_from(bytes[0])?;
        let anchor_index = u16::from_le_bytes([bytes[2], bytes[3]]);
        let original_len = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let encoded_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        Ok(Self {
            kind,
            anchor_index,
            original_len,
            encoded_len,
        })
    }
}

/// Encode a tile as raw uncompressed escape bytes.
pub fn encode_tile_raw(data: &[u8]) -> (TileHeader, Vec<u8>) {
    let header = TileHeader {
        kind: TileCodecKind::RawEscape,
        anchor_index: 0,
        original_len: data.len() as u32,
        encoded_len: data.len() as u32,
    };
    (header, data.to_vec())
}

/// Encode a tile as a reference to an earlier anchor tile.
pub fn encode_tile_shared(anchor_index: u16, original_len: usize) -> (TileHeader, Vec<u8>) {
    let header = TileHeader {
        kind: TileCodecKind::SharedReference,
        anchor_index,
        original_len: original_len as u32,
        encoded_len: 0,
    };
    (header, Vec::new())
}

/// Encode a tile as a bitwise XOR delta against an anchor tile.
pub fn encode_tile_xor(data: &[u8], anchor: &[u8], anchor_index: u16) -> (TileHeader, Vec<u8>) {
    assert_eq!(data.len(), anchor.len(), "data and anchor must have identical size");
    let mut delta = Vec::with_capacity(data.len());
    for i in 0..data.len() {
        delta.push(data[i] ^ anchor[i]);
    }
    let header = TileHeader {
        kind: TileCodecKind::XorDelta,
        anchor_index,
        original_len: data.len() as u32,
        encoded_len: delta.len() as u32,
    };
    (header, delta)
}

/// Encode a tile of 16-bit floats (FP16/BF16) by separating low and high bytes into contiguous planes.
pub fn encode_tile_split_f16(data: &[u8]) -> Result<(TileHeader, Vec<u8>), TileCodecError> {
    if data.len() % 2 != 0 {
        return Err(TileCodecError::InvalidAlignment);
    }
    let n_floats = data.len() / 2;
    let mut encoded = vec![0u8; data.len()];

    // Plane 0: low bytes (mantissa bits); Plane 1: high bytes (sign + exponent bits)
    for i in 0..n_floats {
        encoded[i] = data[2 * i];
        encoded[n_floats + i] = data[2 * i + 1];
    }

    let header = TileHeader {
        kind: TileCodecKind::SplitExpSignificand,
        anchor_index: 0,
        original_len: data.len() as u32,
        encoded_len: encoded.len() as u32,
    };
    Ok((header, encoded))
}

/// Decode a tile into a caller-supplied buffer with zero heap allocations (hot path).
///
/// Guaranteed bit-exact reconstruction of the original tile payload.
pub fn decode_tile_into(
    header: &TileHeader,
    encoded: &[u8],
    anchor: Option<&[u8]>,
    out: &mut [u8],
) -> Result<usize, TileCodecError> {
    let orig_len = header.original_len as usize;
    if out.len() < orig_len {
        return Err(TileCodecError::BufferTooSmall);
    }

    match header.kind {
        TileCodecKind::RawEscape => {
            if encoded.len() < orig_len {
                return Err(TileCodecError::TruncatedPayload);
            }
            out[..orig_len].copy_from_slice(&encoded[..orig_len]);
            Ok(orig_len)
        }
        TileCodecKind::SharedReference => {
            let anch = anchor.ok_or(TileCodecError::AnchorMissing)?;
            if anch.len() < orig_len {
                return Err(TileCodecError::AnchorMismatch);
            }
            out[..orig_len].copy_from_slice(&anch[..orig_len]);
            Ok(orig_len)
        }
        TileCodecKind::XorDelta => {
            let anch = anchor.ok_or(TileCodecError::AnchorMissing)?;
            if anch.len() < orig_len {
                return Err(TileCodecError::AnchorMismatch);
            }
            if encoded.len() < orig_len {
                return Err(TileCodecError::TruncatedPayload);
            }
            for i in 0..orig_len {
                out[i] = encoded[i] ^ anch[i];
            }
            Ok(orig_len)
        }
        TileCodecKind::SplitExpSignificand => {
            if encoded.len() < orig_len {
                return Err(TileCodecError::TruncatedPayload);
            }
            if orig_len % 2 != 0 {
                return Err(TileCodecError::InvalidAlignment);
            }
            let n_floats = orig_len / 2;
            for i in 0..n_floats {
                out[2 * i] = encoded[i];
                out[2 * i + 1] = encoded[n_floats + i];
            }
            Ok(orig_len)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_escape_round_trip() {
        let raw = b"qualialogic_weights_q4k_escape_payload_bytes_1234567890";
        let (hdr, enc) = encode_tile_raw(raw);
        assert_eq!(hdr.kind, TileCodecKind::RawEscape);
        assert_eq!(hdr.encoded_len, raw.len() as u32);

        let mut out = [0u8; 128];
        let n = decode_tile_into(&hdr, &enc, None, &mut out).expect("decode raw");
        assert_eq!(n, raw.len());
        assert_eq!(&out[..n], raw, "exact bit-exact reconstruction");
    }

    #[test]
    fn test_shared_reference_round_trip() {
        let anchor_bytes = b"shared_anchor_tile_content_for_moe_expert_gate_00";
        let (hdr, enc) = encode_tile_shared(42, anchor_bytes.len());
        assert_eq!(hdr.kind, TileCodecKind::SharedReference);
        assert_eq!(hdr.anchor_index, 42);
        assert_eq!(enc.len(), 0);

        let mut out = [0u8; 128];
        let n = decode_tile_into(&hdr, &enc, Some(anchor_bytes), &mut out).expect("decode shared");
        assert_eq!(n, anchor_bytes.len());
        assert_eq!(&out[..n], anchor_bytes);

        // Missing anchor must fail closed
        let err = decode_tile_into(&hdr, &enc, None, &mut out);
        assert_eq!(err, Err(TileCodecError::AnchorMissing));
    }

    #[test]
    fn test_xor_delta_exact_reconstruction() {
        let mut anchor = [0u8; 64];
        let mut target = [0u8; 64];
        for i in 0..64 {
            anchor[i] = (i * 7) as u8;
            target[i] = (i * 7) as u8 ^ ((i % 5 == 0) as u8 * 0xFF);
        }

        let (hdr, enc) = encode_tile_xor(&target, &anchor, 3);
        assert_eq!(hdr.kind, TileCodecKind::XorDelta);
        assert_eq!(hdr.anchor_index, 3);

        let mut out = [0u8; 64];
        let n = decode_tile_into(&hdr, &enc, Some(&anchor), &mut out).expect("decode xor");
        assert_eq!(n, 64);
        assert_eq!(&out[..n], &target, "bit-exact round trip under XOR delta");
    }

    #[test]
    fn test_split_exp_significand_exact_reconstruction() {
        let floats: [f32; 8] = [1.0, -2.5, 0.03125, 4096.0, -0.001, 128.5, 3.1415, -65504.0];
        let mut f16_bytes = Vec::with_capacity(16);
        for &f in &floats {
            let half = half::f16::from_f32(f);
            f16_bytes.extend_from_slice(&half.to_le_bytes());
        }

        let (hdr, enc) = encode_tile_split_f16(&f16_bytes).expect("encode split f16");
        assert_eq!(hdr.kind, TileCodecKind::SplitExpSignificand);
        assert_eq!(enc.len(), f16_bytes.len());

        let mut out = [0u8; 32];
        let n = decode_tile_into(&hdr, &enc, None, &mut out).expect("decode split");
        assert_eq!(n, f16_bytes.len());
        assert_eq!(&out[..n], &f16_bytes[..], "bit-exact float byte reconstruction");
    }

    #[test]
    fn test_header_serialization_and_bounds() {
        let hdr = TileHeader {
            kind: TileCodecKind::XorDelta,
            anchor_index: 1024,
            original_len: 256,
            encoded_len: 128,
        };
        let bytes = hdr.to_bytes();
        let parsed = TileHeader::from_bytes(&bytes).expect("parse header");
        assert_eq!(hdr, parsed);

        let mut small_out = [0u8; 64];
        let dummy_enc = [0u8; 128];
        let err = decode_tile_into(&hdr, &dummy_enc, Some(&[0u8; 256]), &mut small_out);
        assert_eq!(err, Err(TileCodecError::BufferTooSmall));
    }
}
