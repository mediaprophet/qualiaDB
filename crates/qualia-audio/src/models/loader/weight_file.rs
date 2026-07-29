//! Minimal P64-style learned-head weight blob format + bounded parser (AU-LEARNED).
//!
//! This is the on-disk container every learned audio head loads from. It is deliberately tiny
//! and self-describing: a magic tag, a format version, a small `dims` shape vector, and a flat
//! `f32` payload. Real weights are gated on the principal (HA1 corpus / HA6 model); the format
//! here lets us (a) round-trip a synthetic/test blob and (b) fail closed when a blob is absent
//! or malformed. Parsing is **cold** (a `Vec` is allowed); inference over the parsed blob is the
//! hot, caller-buffered path and lives in the head modules.
//!
//! Wire layout (all little-endian):
//! ```text
//!   magic   : u32   = "QP64"
//!   version : u32
//!   ndims   : u32   (<= MAX_DIMS)
//!   dims    : [u32; ndims]
//!   ndata   : u32   (<= MAX_DATA)
//!   data    : [f32; ndata]
//! ```
//! The parser requires the byte slice to end exactly after `data` — trailing bytes are rejected,
//! so a truncated or padded blob cannot be silently accepted.

use crate::types::AudioError;

/// Blob magic: `QP64` little-endian.
pub const WEIGHT_MAGIC: u32 = u32::from_le_bytes([b'Q', b'P', b'6', b'4']);
/// Current blob format version.
pub const WEIGHT_VERSION: u32 = 1;
/// Upper bound on `dims` length (bounds parsing; a shape vector is always tiny).
pub const MAX_DIMS: usize = 8;
/// Upper bound on `data` length in `f32` elements (~4M floats = 16 MiB; bounds parsing).
pub const MAX_DATA: usize = 1 << 22;

/// A parsed learned-head weight blob. Cold-path owned (`Vec`); heads borrow it during inference.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightBlob {
    pub magic: u32,
    pub version: u32,
    /// Logical tensor shape(s) — interpretation is head-specific (e.g. `[classes, features]`).
    pub dims: Vec<u32>,
    /// Flat weight payload (row-major per head convention).
    pub data: Vec<f32>,
}

impl WeightBlob {
    /// Product of `dims` (saturating). Heads use this to validate `data` length against shape.
    pub fn dim_product(&self) -> usize {
        self.dims
            .iter()
            .fold(1usize, |acc, &d| acc.saturating_mul(d as usize))
    }
}

#[inline]
fn read_u32(bytes: &[u8], off: usize) -> Result<u32, AudioError> {
    let end = off.checked_add(4).ok_or(AudioError::MalformedAudio)?;
    let slice = bytes.get(off..end).ok_or(AudioError::MalformedAudio)?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// Parse a weight blob from bytes. Bounded and strict: unknown magic → `BackendUnavailable`
/// (treated as "no usable weights" so the head fails closed); any structural problem
/// (short read, oversized counts, trailing bytes) → `MalformedAudio`. Never partially accepts.
pub fn parse_weight_blob(bytes: &[u8]) -> Result<WeightBlob, AudioError> {
    let magic = read_u32(bytes, 0)?;
    if magic != WEIGHT_MAGIC {
        return Err(AudioError::BackendUnavailable);
    }
    let version = read_u32(bytes, 4)?;
    if version != WEIGHT_VERSION {
        return Err(AudioError::BackendUnavailable);
    }

    let ndims = read_u32(bytes, 8)? as usize;
    if ndims > MAX_DIMS {
        return Err(AudioError::MalformedAudio);
    }
    let mut dims = Vec::with_capacity(ndims);
    let mut off = 12;
    for _ in 0..ndims {
        dims.push(read_u32(bytes, off)?);
        off += 4;
    }

    let ndata = read_u32(bytes, off)? as usize;
    off += 4;
    if ndata > MAX_DATA {
        return Err(AudioError::MalformedAudio);
    }
    let data_bytes = ndata.checked_mul(4).ok_or(AudioError::MalformedAudio)?;
    let data_end = off
        .checked_add(data_bytes)
        .ok_or(AudioError::MalformedAudio)?;
    // Strict: the blob must end exactly after `data` (no truncation, no trailing padding).
    if data_end != bytes.len() {
        return Err(AudioError::MalformedAudio);
    }
    let mut data = Vec::with_capacity(ndata);
    let mut d = off;
    for _ in 0..ndata {
        let w = f32::from_le_bytes([bytes[d], bytes[d + 1], bytes[d + 2], bytes[d + 3]]);
        data.push(w);
        d += 4;
    }

    Ok(WeightBlob {
        magic,
        version,
        dims,
        data,
    })
}

/// Serialize a weight blob (cold path — tests and fixture generation). Inverse of
/// [`parse_weight_blob`].
pub fn write_weight_blob(blob: &WeightBlob) -> Vec<u8> {
    let mut v = Vec::with_capacity(16 + blob.dims.len() * 4 + blob.data.len() * 4);
    v.extend_from_slice(&blob.magic.to_le_bytes());
    v.extend_from_slice(&blob.version.to_le_bytes());
    v.extend_from_slice(&(blob.dims.len() as u32).to_le_bytes());
    for &d in &blob.dims {
        v.extend_from_slice(&d.to_le_bytes());
    }
    v.extend_from_slice(&(blob.data.len() as u32).to_le_bytes());
    for &f in &blob.data {
        v.extend_from_slice(&f.to_le_bytes());
    }
    v
}

/// Convenience: build a well-formed blob with the current magic/version.
pub fn make_blob(dims: Vec<u32>, data: Vec<f32>) -> WeightBlob {
    WeightBlob {
        magic: WEIGHT_MAGIC,
        version: WEIGHT_VERSION,
        dims,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let blob = make_blob(vec![2, 3], vec![0.1, -0.2, 0.3, 0.4, -0.5, 0.6]);
        let bytes = write_weight_blob(&blob);
        let back = parse_weight_blob(&bytes).expect("parse");
        assert_eq!(back, blob);
        assert_eq!(back.dim_product(), 6);
    }

    #[test]
    fn empty_data_ok() {
        let blob = make_blob(vec![], vec![]);
        let bytes = write_weight_blob(&blob);
        let back = parse_weight_blob(&bytes).expect("parse");
        assert_eq!(back, blob);
        assert_eq!(back.dim_product(), 1);
    }

    #[test]
    fn bad_magic_is_backend_unavailable() {
        let mut bytes = write_weight_blob(&make_blob(vec![1], vec![1.0]));
        bytes[0] ^= 0xFF;
        assert_eq!(
            parse_weight_blob(&bytes),
            Err(AudioError::BackendUnavailable)
        );
    }

    #[test]
    fn wrong_version_is_backend_unavailable() {
        let mut blob = make_blob(vec![1], vec![1.0]);
        blob.version = 999;
        let bytes = write_weight_blob(&blob);
        assert_eq!(
            parse_weight_blob(&bytes),
            Err(AudioError::BackendUnavailable)
        );
    }

    #[test]
    fn truncated_is_malformed() {
        let bytes = write_weight_blob(&make_blob(vec![2, 2], vec![1.0, 2.0, 3.0, 4.0]));
        let short = &bytes[..bytes.len() - 3];
        assert_eq!(parse_weight_blob(short), Err(AudioError::MalformedAudio));
    }

    #[test]
    fn trailing_bytes_rejected() {
        let mut bytes = write_weight_blob(&make_blob(vec![1], vec![1.0]));
        bytes.push(0);
        assert_eq!(parse_weight_blob(&bytes), Err(AudioError::MalformedAudio));
    }

    #[test]
    fn oversized_ndims_rejected() {
        // magic, version, ndims = huge
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&WEIGHT_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&WEIGHT_VERSION.to_le_bytes());
        bytes.extend_from_slice(&(MAX_DIMS as u32 + 1).to_le_bytes());
        assert_eq!(parse_weight_blob(&bytes), Err(AudioError::MalformedAudio));
    }

    #[test]
    fn empty_input_malformed() {
        assert_eq!(parse_weight_blob(&[]), Err(AudioError::MalformedAudio));
    }
}
