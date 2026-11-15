//! Nested realm context hashing, floating-origin sector handoffs, and time dilation (OCS §7, §14).
//!
//! Reference: OCS Specification v2.2.0 §7, §14.

use crate::value::Value;
use std::collections::BTreeMap;

/// A nested realm level (OCS §7).
///
/// Realities can be nested: a holodeck simulation inside a starship,
/// a dream level inside a character, etc. Each level has its own
/// coordinate system, clock, and paraconsistent isolation context.
#[derive(Debug, Clone, PartialEq)]
pub struct NestedRealm {
    /// The nesting depth (0 = physical base, 1 = first nested, etc.)
    pub depth: u32,
    /// The USRI of this realm.
    pub realm_usri: String,
    /// Time dilation factor ζ = dτ_nested / dτ_parent (OCS §7.1).
    /// ζ > 1 = accelerated (e.g. dream time), ζ < 1 = decelerated.
    pub time_dilation: f64,
    /// The parent realm's context hash (for isolation).
    pub parent_context: u64,
}

impl NestedRealm {
    /// Creates a new `NestedRealm`.
    pub fn new(depth: u32, realm_usri: &str, time_dilation: f64, parent_context: u64) -> Self {
        Self {
            depth,
            realm_usri: realm_usri.into(),
            time_dilation,
            parent_context,
        }
    }

    /// Compute the paraconsistent isolation context hash (OCS §7.1).
    ///
    /// Uses non-commutative FNV-1a folding over `(parent_context.to_le_bytes(),
    /// realm_usri.as_bytes(), depth.to_le_bytes())` so ordered paths `[A, B]`
    /// and `[B, A]` produce distinct context hashes.
    pub fn context_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in self.parent_context.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        for b in self.realm_usri.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        for b in self.depth.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    /// Convert parent proper time to nested proper time (OCS §7.1).
    /// τ_nested = τ_parent * ζ
    pub fn parent_to_nested_time(&self, parent_tau_secs: f64) -> f64 {
        parent_tau_secs * self.time_dilation
    }

    /// Convert nested proper time to parent proper time.
    pub fn nested_to_parent_time(&self, nested_tau_secs: f64) -> f64 {
        nested_tau_secs / self.time_dilation
    }

    /// Converts to value.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("depth".into(), Value::U64(self.depth as u64));
        rec.insert("realm_usri".into(), Value::String(self.realm_usri.clone()));
        rec.insert("time_dilation".into(), Value::F64(self.time_dilation));
        rec.insert("context_hash".into(), Value::U64(self.context_hash()));
        Value::Record(rec)
    }
}

/// A stack of nested realms (OCS §7).
#[derive(Debug, Clone)]
pub struct NestingStack {
    realms: Vec<NestedRealm>,
}

impl NestingStack {
    /// Creates a new `NestingStack`.
    pub fn new() -> Self {
        Self { realms: Vec::new() }
    }

    /// Push a new nested realm onto the stack.
    pub fn push(&mut self, realm_usri: &str, time_dilation: f64) -> &NestedRealm {
        let depth = self.realms.len() as u32;
        let parent_context = self.realms.last().map(|r| r.context_hash()).unwrap_or(0); // Physical base has context 0
        self.realms.push(NestedRealm::new(
            depth,
            realm_usri,
            time_dilation,
            parent_context,
        ));
        self.realms.last().unwrap()
    }

    /// Pop the top realm off the stack.
    pub fn pop(&mut self) -> Option<NestedRealm> {
        self.realms.pop()
    }

    /// Current nesting depth.
    pub fn depth(&self) -> u32 {
        self.realms.len() as u32
    }

    /// Get the current (top) realm.
    pub fn current(&self) -> Option<&NestedRealm> {
        self.realms.last()
    }

    /// Compute the cumulative time dilation from base to current level.
    pub fn cumulative_time_dilation(&self) -> f64 {
        self.realms.iter().map(|r| r.time_dilation).product()
    }

    /// Convert base time to time at the current nesting level.
    pub fn base_to_current_time(&self, base_tau_secs: f64) -> f64 {
        base_tau_secs * self.cumulative_time_dilation()
    }
}

impl Default for NestingStack {
    fn default() -> Self {
        Self::new()
    }
}

/// Hierarchical Floating-Origin reference frame with HEALPix + Octree sector indexing
/// and split sub-millimeter local residual (OCS §3.2, §14, Packet OCS-11).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingOriginFrame {
    /// Deterministic FNV-1a identifier for the current sector anchor.
    pub anchor_id: u64,
    /// Angular HEALPix cell index on the unit sphere (`12 * nside^2` tessellation).
    pub healpix_index: u64,
    /// Discrete 3D octree sector coordinates in units of `sector_radius_m`.
    pub octree_cell: [i64; 3],
    /// Coarse position within the current sector (resets to `[0.0, 0.0, 0.0]` on sector handoff).
    pub local_origin_m: [f64; 3],
    /// Independently constructed sub-millimeter local residual preserved across handoffs.
    pub residual_m: [f64; 3],
    /// Number of sector boundary re-anchorings performed.
    pub reanchor_count: u64,
}

impl FloatingOriginFrame {
    /// Create a new floating-origin frame at a given sector cell and sub-millimeter residual.
    pub fn new(octree_cell: [i64; 3], residual_m: [f64; 3]) -> Self {
        let healpix_index = compute_healpix_index(octree_cell, 64);
        let anchor_id = compute_sector_anchor_id(octree_cell, healpix_index);
        Self {
            anchor_id,
            healpix_index,
            octree_cell,
            local_origin_m: [0.0, 0.0, 0.0],
            residual_m,
            reanchor_count: 0,
        }
    }
}

/// Compute a deterministic HEALPix pixel index (`12 * nside^2`) from a 3D sector vector.
pub fn compute_healpix_index(octree_cell: [i64; 3], nside: u64) -> u64 {
    let nside = nside.max(1);
    let x = octree_cell[0] as f64;
    let y = octree_cell[1] as f64;
    let z = octree_cell[2] as f64;
    let r = (x * x + y * y + z * z).sqrt();
    if r == 0.0 {
        return 0;
    }
    let cos_theta = (z / r).clamp(-1.0, 1.0);
    let phi = y.atan2(x).rem_euclid(2.0 * std::f64::consts::PI);
    let base_pix = ((phi / (2.0 * std::f64::consts::PI)) * 12.0).floor() as u64 % 12;
    let z_band = (((1.0 - cos_theta) * 0.5) * (nside as f64)).floor() as u64 % nside;
    let az_band = ((phi / (2.0 * std::f64::consts::PI)) * (nside as f64)).floor() as u64 % nside;
    base_pix * nside * nside + z_band * nside + az_band
}

fn compute_sector_anchor_id(octree_cell: [i64; 3], healpix_index: u64) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for coord in octree_cell {
        for b in coord.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    for b in healpix_index.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Advance a [`FloatingOriginFrame`] by `step_delta_m` within a sector of radius `sector_radius_m`.
///
/// Automatically transitions the entity to the adjacent `healpix_index` / `octree_cell`
/// and resets `local_origin_m` to `(0.0, 0.0, 0.0)` when crossing a sector boundary.
pub fn reanchor_origin(
    current: &FloatingOriginFrame,
    step_delta_m: [f64; 3],
    sector_radius_m: f64,
) -> FloatingOriginFrame {
    let radius = sector_radius_m.abs().max(1.0);
    let candidate = [
        current.local_origin_m[0] + step_delta_m[0],
        current.local_origin_m[1] + step_delta_m[1],
        current.local_origin_m[2] + step_delta_m[2],
    ];
    let dist_sq = candidate[0] * candidate[0]
        + candidate[1] * candidate[1]
        + candidate[2] * candidate[2];
    if dist_sq >= radius * radius {
        let mut next_cell = current.octree_cell;
        let mut moved = false;
        for i in 0..3 {
            let steps = (candidate[i] / radius).trunc() as i64;
            if steps != 0 {
                next_cell[i] = next_cell[i].saturating_add(steps);
                moved = true;
            }
        }
        if !moved {
            let mut dom = 0;
            if candidate[1].abs() > candidate[dom].abs() {
                dom = 1;
            }
            if candidate[2].abs() > candidate[dom].abs() {
                dom = 2;
            }
            let dir = if candidate[dom] >= 0.0 { 1 } else { -1 };
            next_cell[dom] = next_cell[dom].saturating_add(dir);
        }
        let healpix_index = compute_healpix_index(next_cell, 64);
        let anchor_id = compute_sector_anchor_id(next_cell, healpix_index);
        FloatingOriginFrame {
            anchor_id,
            healpix_index,
            octree_cell: next_cell,
            local_origin_m: [0.0, 0.0, 0.0],
            residual_m: current.residual_m,
            reanchor_count: current.reanchor_count + 1,
        }
    } else {
        FloatingOriginFrame {
            anchor_id: current.anchor_id,
            healpix_index: current.healpix_index,
            octree_cell: current.octree_cell,
            local_origin_m: candidate,
            residual_m: current.residual_m,
            reanchor_count: current.reanchor_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_realm_context_hash() {
        let r = NestedRealm::new(
            1,
            "urn:omni:v1:simulation:holodeck:vicorian-london",
            20.0,
            0,
        );
        let h = r.context_hash();
        assert_eq!(h, r.context_hash());
        assert_ne!(h, 0);
    }

    #[test]
    fn context_hash_isolates_levels() {
        let r1 = NestedRealm::new(1, "urn:omni:v1:simulation:A", 1.0, 0);
        let r2 = NestedRealm::new(1, "urn:omni:v1:simulation:B", 1.0, 0);
        assert_ne!(r1.context_hash(), r2.context_hash());
    }

    #[test]
    fn time_dilation_accelerated() {
        let r = NestedRealm::new(1, "dream", 20.0, 0);
        assert!((r.parent_to_nested_time(1.0) - 20.0).abs() < 1e-10);
        assert!((r.nested_to_parent_time(20.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn time_dilation_decelerated() {
        let r = NestedRealm::new(1, "slow", 0.1, 0);
        assert!((r.parent_to_nested_time(1.0) - 0.1).abs() < 1e-10);
    }

    #[test]
    fn nesting_stack_push_pop() {
        let mut stack = NestingStack::new();
        assert_eq!(stack.depth(), 0);
        stack.push("level1", 10.0);
        assert_eq!(stack.depth(), 1);
        stack.push("level2", 2.0);
        assert_eq!(stack.depth(), 2);
        stack.pop();
        assert_eq!(stack.depth(), 1);
    }

    #[test]
    fn cumulative_time_dilation() {
        let mut stack = NestingStack::new();
        stack.push("level1", 10.0);
        stack.push("level2", 2.0);
        assert!((stack.cumulative_time_dilation() - 20.0).abs() < 1e-10);
    }

    #[test]
    fn base_to_current_time() {
        let mut stack = NestingStack::new();
        stack.push("dream", 20.0);
        assert!((stack.base_to_current_time(1.0) - 20.0).abs() < 1e-10);
    }

    #[test]
    fn nesting_context_chains() {
        let mut stack = NestingStack::new();
        stack.push("level1", 1.0);
        let ctx1 = stack.current().unwrap().context_hash();
        stack.push("level2", 1.0);
        let ctx2 = stack.current().unwrap().context_hash();
        assert_ne!(ctx1, ctx2);
        assert_ne!(ctx1, 0);
        assert_ne!(ctx2, 0);
    }

    #[test]
    fn nested_realm_to_value() {
        let r = NestedRealm::new(2, "urn:omni:v1:simulation:test", 5.0, 12345);
        let v = r.to_value();
        match v {
            Value::Record(rec) => {
                assert_eq!(rec.get("depth"), Some(&Value::U64(2)));
                assert_eq!(rec.get("time_dilation"), Some(&Value::F64(5.0)));
                assert!(rec.contains_key("context_hash"));
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn nested_context_hash_is_non_commutative() {
        let mut stack_ab = NestingStack::new();
        stack_ab.push("urn:omni:v1:simulation:A", 1.0);
        stack_ab.push("urn:omni:v1:simulation:B", 1.0);
        let hash_ab = stack_ab.current().unwrap().context_hash();

        let mut stack_ba = NestingStack::new();
        stack_ba.push("urn:omni:v1:simulation:B", 1.0);
        stack_ba.push("urn:omni:v1:simulation:A", 1.0);
        let hash_ba = stack_ba.current().unwrap().context_hash();

        assert_ne!(
            hash_ab, hash_ba,
            "ordered realm paths [A, B] and [B, A] must not collide"
        );
    }

    #[test]
    fn earth_to_alpha_centauri_floating_origin_preserves_sub_mm_residual() {
        // Alpha Centauri distance: 4.37 light-years ≈ 4.1343e16 m.
        const ALPHA_CENTAURI_M: f64 = 4.1343e16;
        let ingested_residual_m = [0.000_42, -0.000_19, 0.000_73]; // 0.42 mm, -0.19 mm, 0.73 mm

        // 1. Prove naive global f64 coordinate addition loses sub-meter (and sub-mm) precision:
        let naive_global = std::hint::black_box(ALPHA_CENTAURI_M + ingested_residual_m[0]);
        let naive_recovered = naive_global - ALPHA_CENTAURI_M;
        let naive_err = (naive_recovered - ingested_residual_m[0]).abs();
        assert!(
            naive_err > 1e-4,
            "naive global f64 at 4.13e16 m loses sub-millimeter precision (err = {naive_err})"
        );

        // 2. Simulate trajectory across 100 sector handoffs from Earth orbit to Alpha Centauri:
        let sector_radius_m = 4.1343e14; // 100 sectors along trajectory
        let step_delta_m = [sector_radius_m, 1.0e13, -2.0e13];
        let mut frame = FloatingOriginFrame::new([0, 0, 0], ingested_residual_m);
        let initial_anchor = frame.anchor_id;

        for _ in 0..100 {
            frame = reanchor_origin(&frame, step_delta_m, sector_radius_m);
            assert_eq!(frame.local_origin_m, [0.0, 0.0, 0.0]);
        }

        assert_eq!(frame.reanchor_count, 100);
        assert_eq!(frame.octree_cell[0], 100);
        assert_ne!(frame.anchor_id, initial_anchor);
        assert_ne!(frame.healpix_index, 0);
        for axis in 0..3 {
            let err = (frame.residual_m[axis] - ingested_residual_m[axis]).abs();
            assert!(
                err < 1e-12,
                "FloatingOriginFrame must preserve sub-millimeter residual on axis {axis}, got err {err}"
            );
        }
    }
}
