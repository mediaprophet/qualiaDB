//! Cold-path NQuin → Tensor10D baking (ingest / encode tier).
//!
//! Hot-path queries read baked SOA volumes; this module runs at ingest only.

use crate::{q_hash, NQuin};

use super::Tensor10D;

pub const PRED_GEO_VERTEX: u64 = q_hash("geo:hasVertex");
/// Bake-time link to mmap STFT/CQT sidecar (`spectral/audio/{hash}.bin`).
pub const PRED_HAS_SPECTRAL_SHEET: u64 = q_hash("q42:hasSpectralSheet");

/// Relative sidecar path under storage root — zero-heap (`spectral/audio/{hash:016x}.bin`).
pub fn audio_sidecar_relpath(content_hash: u64, out: &mut [u8]) -> usize {
    const PREFIX: &[u8] = b"spectral/audio/";
    const SUFFIX: &[u8] = b".bin";
    let hex = format_hash16(content_hash);
    let need = PREFIX.len() + 16 + SUFFIX.len();
    if out.len() < need {
        return 0;
    }
    out[..PREFIX.len()].copy_from_slice(PREFIX);
    out[PREFIX.len()..PREFIX.len() + 16].copy_from_slice(&hex);
    out[PREFIX.len() + 16..need].copy_from_slice(SUFFIX);
    need
}

#[inline]
fn format_hash16(h: u64) -> [u8; 16] {
    let mut buf = [b'0'; 16];
    let nibbles = [
        ((h >> 60) & 0xf) as u8,
        ((h >> 56) & 0xf) as u8,
        ((h >> 52) & 0xf) as u8,
        ((h >> 48) & 0xf) as u8,
        ((h >> 44) & 0xf) as u8,
        ((h >> 40) & 0xf) as u8,
        ((h >> 36) & 0xf) as u8,
        ((h >> 32) & 0xf) as u8,
        ((h >> 28) & 0xf) as u8,
        ((h >> 24) & 0xf) as u8,
        ((h >> 20) & 0xf) as u8,
        ((h >> 16) & 0xf) as u8,
        ((h >> 12) & 0xf) as u8,
        ((h >> 8) & 0xf) as u8,
        ((h >> 4) & 0xf) as u8,
        (h & 0xf) as u8,
    ];
    for (i, n) in nibbles.iter().enumerate() {
        buf[i] = match *n {
            0..=9 => b'0' + *n,
            _ => b'a' + (*n - 10),
        };
    }
    buf
}

/// σ sheet index from baked NQuin object when `q42:hasSpectralSheet` is present.
#[inline]
pub fn sigma_sheet_index_from_nquin(nquin: &NQuin) -> Option<u32> {
    let pred = nquin.predicate & 0x0FFF_FFFF_FFFF_FFFF;
    if pred != (PRED_HAS_SPECTRAL_SHEET & 0x0FFF_FFFF_FFFF_FFFF) {
        return None;
    }
    let idx = (nquin.object & 0x0FFF_FFFF_FFFF_FFFF) as u32;
    Some(idx)
}

/// Decode `spatial_encode_wasm` packed coordinates from an object field.
#[inline]
pub fn decode_packed_coord(object: u64) -> (f32, f32, f32) {
    let xi = sign_extend_20(((object >> 40) & 0xfffff) as u32);
    let yi = sign_extend_20(((object >> 20) & 0xfffff) as u32);
    let zi = sign_extend_20((object & 0xfffff) as u32);
    (xi as f32 / 1000.0, yi as f32 / 1000.0, zi as f32 / 1000.0)
}

#[inline]
fn sign_extend_20(v: u32) -> i32 {
    let v = v & 0xfffff;
    if v & 0x8_0000 != 0 {
        (v | !0xfffff) as i32
    } else {
        v as i32
    }
}

/// True when the Quin carries a baked geo vertex payload (not a hash-proxy xyz).
#[inline]
pub fn is_geo_vertex_quin(nquin: &NQuin) -> bool {
    (nquin.predicate & 0x7FFF_FFFF_FFFF_FF00) == PRED_GEO_VERTEX
        || (nquin.predicate & 0x0FFF_FFFF_FFFF_FFFF) == (PRED_GEO_VERTEX & 0x0FFF_FFFF_FFFF_FFFF)
}

/// Semantic xyz: packed geo coords when present, else legacy hash spread.
#[inline]
pub fn semantic_xyz_from_nquin(nquin: &NQuin) -> (f32, f32, f32) {
    if is_geo_vertex_quin(nquin) && (nquin.object >> 63) == 0 {
        return decode_packed_coord(nquin.object);
    }
    let hash = nquin.object & 0x0FFF_FFFF_FFFF_FFFF;
    let x = (hash & 0xFFFF) as f32 / 65535.0;
    let y = ((hash >> 16) & 0xFFFF) as f32 / 65535.0;
    let z = ((hash >> 32) & 0xFFFF) as f32 / 65535.0;
    (x, y, z)
}

/// Bake a single NQuin into a ground-truth Tensor10D node.
#[inline]
pub fn bake_quin_to_tensor(nquin: &NQuin) -> Tensor10D {
    let q = if (nquin.metadata >> 60) & 0xF != 0 {
        0.25
    } else {
        0.0
    };
    let v = ((nquin.context >> 32) & 0x7) as f32;
    let w = ((nquin.context >> 40) & 0xF) as f32;
    let (x, y, z) = semantic_xyz_from_nquin(nquin);
    let t = ((nquin.metadata >> 32) & 0x1FFF_FFFF) as f32;
    let payload = nquin.metadata & 0xFFFF_FFFF;
    let alpha = (payload & 0xFF) as f32 / 255.0;
    let mu = ((payload >> 8) & 0xFF) as f32 / 255.0;
    let sigma = ((payload >> 16) & 0xFF) as f32 / 255.0;
    if q > 0.0 {
        Tensor10D::parallel_context(q, v, w, x, y, z, t, alpha.max(0.1), mu, sigma)
    } else {
        Tensor10D::ground_truth(v, w, x, y, z, t, alpha.max(0.1), mu, sigma)
    }
}

/// Write baked tensors into caller buffer; returns count written.
pub fn bake_quins_into(quins: &[NQuin], out: &mut [Tensor10D]) -> usize {
    let n = quins.len().min(out.len());
    for i in 0..n {
        out[i] = bake_quin_to_tensor(&quins[i]);
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NQuin;

    fn pack_coord(x: f32, y: f32, z: f32) -> u64 {
        let xi = (x * 1000.0).round() as i64 & 0xfffff;
        let yi = (y * 1000.0).round() as i64 & 0xfffff;
        let zi = (z * 1000.0).round() as i64 & 0xfffff;
        ((xi as u64) << 40) | ((yi as u64) << 20) | (zi as u64)
    }

    #[test]
    fn packed_coord_round_trip() {
        let (x, y, z) = decode_packed_coord(pack_coord(1.25, -2.5, 3.75));
        assert!((x - 1.25).abs() < 0.002);
        assert!((y + 2.5).abs() < 0.002);
        assert!((z - 3.75).abs() < 0.002);
    }

    #[test]
    fn audio_sidecar_path_format() {
        let mut buf = [0u8; 64];
        let n = audio_sidecar_relpath(0xabc_def01_2345_6789, &mut buf);
        assert_eq!(n, b"spectral/audio/".len() + 16 + b".bin".len());
        let path = std::str::from_utf8(&buf[..n]).unwrap();
        assert!(path.starts_with("spectral/audio/"));
        assert!(path.ends_with(".bin"));
    }

    #[test]
    fn spectral_sheet_predicate_extracts_index() {
        let mut q = NQuin::default();
        q.predicate = PRED_HAS_SPECTRAL_SHEET;
        q.object = 42;
        assert_eq!(sigma_sheet_index_from_nquin(&q), Some(42));
    }

    #[test]
    fn geo_vertex_bake_uses_packed_xyz() {
        let mut q = NQuin::default();
        q.predicate = PRED_GEO_VERTEX;
        q.object = pack_coord(0.1, 0.2, 0.3);
        q.metadata = 42 << 32;
        let t = bake_quin_to_tensor(&q);
        assert!((t.x - 0.1).abs() < 0.002);
        assert!((t.y - 0.2).abs() < 0.002);
        assert!((t.z - 0.3).abs() < 0.002);
        assert!(t.is_ground_truth());
    }
}
