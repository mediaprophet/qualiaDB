//! Cold-path NQuin → Tensor10D baking (ingest / encode tier).
//!
//! Hot-path queries read baked SOA volumes; this module runs at ingest only.

use crate::{q_hash, NQuin};

use super::Tensor10D;

pub const PRED_GEO_VERTEX: u64 = q_hash("geo:hasVertex");
pub const PRED_GEO_PACKED_CENTROID: u64 = q_hash("geo:hasPackedCentroid");
pub const PRED_GEO_PACKED_BBOX_MIN: u64 = q_hash("geo:hasPackedBboxMin");
pub const PRED_GEO_PACKED_BBOX_MAX: u64 = q_hash("geo:hasPackedBboxMax");

/// Bake-time link to mmap STFT/CQT sidecar (`spectral/audio/{hash}.bin`).
pub const PRED_HAS_SPECTRAL_SHEET: u64 = q_hash("q42:hasSpectralSheet");

/// Scale tier for multi-scale spatial coordinate packing (§2.3)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScaleTier {
    /// 1 mm (10^-3 m) per LSB — backward-compatible legacy default
    MillimeterDefault = 0,
    /// 10^-9 m
    Nanometer = 1,
    /// 10^-6 m
    Micrometer = 2,
    /// 10^0 m (1 meter)
    Meter = 3,
    /// 10^3 m (1 kilometer)
    Kilometer = 4,
    /// 10^6 m (1 megameter)
    Megameter = 5,
    /// 10^9 m (1 gigameter / astronomical)
    Astronomical = 6,
    /// 10^12 m (1 terameter / parsec tier)
    Parsec = 7,
}

impl Default for ScaleTier {
    fn default() -> Self {
        ScaleTier::MillimeterDefault
    }
}

impl ScaleTier {
    /// Map canonical scale tier exponent S in [0..7] (step = 10^(3*S - 9)) to ScaleTier.
    pub fn from_canonical_exponent_tier(s: u8) -> Self {
        match s {
            0 => ScaleTier::Nanometer,
            1 => ScaleTier::Micrometer,
            2 => ScaleTier::MillimeterDefault,
            3 => ScaleTier::Meter,
            4 => ScaleTier::Kilometer,
            5 => ScaleTier::Megameter,
            6 => ScaleTier::Astronomical,
            7 => ScaleTier::Parsec,
            _ => ScaleTier::MillimeterDefault,
        }
    }

    /// Decode raw 3-bit value (bits 60..62).
    pub fn from_u8(raw: u8) -> Self {
        match raw & 0x7 {
            0 => ScaleTier::MillimeterDefault,
            1 => ScaleTier::Nanometer,
            2 => ScaleTier::Micrometer,
            3 => ScaleTier::Meter,
            4 => ScaleTier::Kilometer,
            5 => ScaleTier::Megameter,
            6 => ScaleTier::Astronomical,
            7 => ScaleTier::Parsec,
            _ => ScaleTier::MillimeterDefault,
        }
    }

    /// Metric step in meters per LSB.
    pub fn step_meters(self) -> f64 {
        match self {
            ScaleTier::MillimeterDefault => 1e-3,
            ScaleTier::Nanometer => 1e-9,
            ScaleTier::Micrometer => 1e-6,
            ScaleTier::Meter => 1.0,
            ScaleTier::Kilometer => 1e3,
            ScaleTier::Megameter => 1e6,
            ScaleTier::Astronomical => 1e9,
            ScaleTier::Parsec => 1e12,
        }
    }
}

/// Encode metric coordinates in meters into a 64-bit packed object with scale tier.
#[inline]
pub fn encode_scaled_coord(x_m: f64, y_m: f64, z_m: f64, tier: ScaleTier) -> u64 {
    let step = tier.step_meters();
    let xi = (x_m / step).round().clamp(-524288.0, 524287.0) as i64 & 0xFFFFF;
    let yi = (y_m / step).round().clamp(-524288.0, 524287.0) as i64 & 0xFFFFF;
    let zi = (z_m / step).round().clamp(-524288.0, 524287.0) as i64 & 0xFFFFF;
    let tier_bits = ((tier as u8 as u64) & 0x7) << 60;
    tier_bits | ((xi as u64) << 40) | ((yi as u64) << 20) | (zi as u64)
}

/// Decode a 64-bit packed object into coordinates in meters along with its scale tier.
#[inline]
pub fn decode_scaled_coord(object: u64) -> (f64, f64, f64, ScaleTier) {
    let tier_raw = ((object >> 60) & 0x7) as u8;
    let tier = ScaleTier::from_u8(tier_raw);
    let step = tier.step_meters();
    let xi = sign_extend_20(((object >> 40) & 0xFFFFF) as u32);
    let yi = sign_extend_20(((object >> 20) & 0xFFFFF) as u32);
    let zi = sign_extend_20((object & 0xFFFFF) as u32);
    (xi as f64 * step, yi as f64 * step, zi as f64 * step, tier)
}

/// Migrate a legacy unscaled millimeter Quin to a target scale tier.
pub fn migrate_unscaled_mm_quin(quin: &NQuin, target_tier: ScaleTier) -> NQuin {
    let (x, y, z) = decode_packed_coord(quin.object);
    let new_object = encode_scaled_coord(x as f64, y as f64, z as f64, target_tier);
    let mut updated = *quin;
    updated.object = new_object;
    updated.parity = updated.subject ^ updated.predicate ^ updated.object ^ updated.context;
    updated
}

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
///
/// When bits 60..62 are 0 (ScaleTier::MillimeterDefault), returns (xi / 1000.0, yi / 1000.0, zi / 1000.0).
/// When bits 60..62 are 1..7, scales by the corresponding tier step in meters.
#[inline]
pub fn decode_packed_coord(object: u64) -> (f32, f32, f32) {
    let xi = sign_extend_20(((object >> 40) & 0xfffff) as u32);
    let yi = sign_extend_20(((object >> 20) & 0xfffff) as u32);
    let zi = sign_extend_20((object & 0xfffff) as u32);
    let tier_raw = ((object >> 60) & 0x7) as u8;
    if tier_raw == 0 {
        (xi as f32 / 1000.0, yi as f32 / 1000.0, zi as f32 / 1000.0)
    } else {
        let step = ScaleTier::from_u8(tier_raw).step_meters() as f32;
        (xi as f32 * step, yi as f32 * step, zi as f32 * step)
    }
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

/// True when the Quin carries a baked geo coordinate payload (not a hash-proxy xyz).
#[inline]
pub fn is_geo_vertex_quin(nquin: &NQuin) -> bool {
    let masked_7f = nquin.predicate & 0x7FFF_FFFF_FFFF_FF00;
    let masked_0f = nquin.predicate & 0x0FFF_FFFF_FFFF_FFFF;
    let matches = |pred: u64| {
        masked_7f == pred || masked_0f == (pred & 0x0FFF_FFFF_FFFF_FFFF)
    };
    matches(PRED_GEO_VERTEX)
        || matches(PRED_GEO_PACKED_CENTROID)
        || matches(PRED_GEO_PACKED_BBOX_MIN)
        || matches(PRED_GEO_PACKED_BBOX_MAX)
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

    #[test]
    fn test_multi_scale_packing_across_all_eight_tiers() {
        let tiers = [
            ScaleTier::MillimeterDefault,
            ScaleTier::Nanometer,
            ScaleTier::Micrometer,
            ScaleTier::Meter,
            ScaleTier::Kilometer,
            ScaleTier::Megameter,
            ScaleTier::Astronomical,
            ScaleTier::Parsec,
        ];

        for tier in tiers {
            let step = tier.step_meters();
            let xm = 42.0 * step;
            let ym = -17.0 * step;
            let zm = 100.0 * step;

            let packed = encode_scaled_coord(xm, ym, zm, tier);
            let (dx, dy, dz, decoded_tier) = decode_scaled_coord(packed);

            assert_eq!(decoded_tier, tier);
            assert!((dx - xm).abs() / step < 0.5, "Tier {tier:?} x error");
            assert!((dy - ym).abs() / step < 0.5, "Tier {tier:?} y error");
            assert!((dz - zm).abs() / step < 0.5, "Tier {tier:?} z error");

            // Also test via decode_packed_coord
            let (px, py, pz) = decode_packed_coord(packed);
            assert!((px as f64 - xm).abs() / step < 0.5);
            assert!((py as f64 - ym).abs() / step < 0.5);
            assert!((pz as f64 - zm).abs() / step < 0.5);
        }
    }

    #[test]
    fn test_canonical_exponent_mapping() {
        assert_eq!(ScaleTier::from_canonical_exponent_tier(0), ScaleTier::Nanometer);
        assert_eq!(ScaleTier::from_canonical_exponent_tier(1), ScaleTier::Micrometer);
        assert_eq!(ScaleTier::from_canonical_exponent_tier(2), ScaleTier::MillimeterDefault);
        assert_eq!(ScaleTier::from_canonical_exponent_tier(3), ScaleTier::Meter);
        assert_eq!(ScaleTier::from_canonical_exponent_tier(4), ScaleTier::Kilometer);
        assert_eq!(ScaleTier::from_canonical_exponent_tier(5), ScaleTier::Megameter);
        assert_eq!(ScaleTier::from_canonical_exponent_tier(6), ScaleTier::Astronomical);
        assert_eq!(ScaleTier::from_canonical_exponent_tier(7), ScaleTier::Parsec);
    }

    #[test]
    fn test_geo_predicates_and_migration() {
        let preds = [
            PRED_GEO_VERTEX,
            PRED_GEO_PACKED_CENTROID,
            PRED_GEO_PACKED_BBOX_MIN,
            PRED_GEO_PACKED_BBOX_MAX,
        ];
        for pred in preds {
            let mut q = NQuin::default();
            q.predicate = pred;
            assert!(is_geo_vertex_quin(&q));
        }

        // Migration from unscaled mm to meter tier
        let mut q = NQuin::default();
        q.predicate = PRED_GEO_PACKED_CENTROID;
        q.object = pack_coord(5.0, -10.0, 15.0); // 5m, -10m, 15m in mm
        let migrated = migrate_unscaled_mm_quin(&q, ScaleTier::Meter);

        let (x, y, z, tier) = decode_scaled_coord(migrated.object);
        assert_eq!(tier, ScaleTier::Meter);
        assert!((x - 5.0).abs() < 1e-3);
        assert!((y - (-10.0)).abs() < 1e-3);
        assert!((z - 15.0).abs() < 1e-3);
    }
}
