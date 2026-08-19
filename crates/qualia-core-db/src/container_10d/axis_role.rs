//! Normative axis-role taxonomy for the `.10d` container header.
//!
//! Each of the ten tensor axes `[q, v, w, x, y, z, t, alpha, mu, sigma]` is
//! declared in the header with one of the roles below. The taxonomy is the
//! prerequisite for honest queryability: a `COORDINATE` participates in
//! distance, a `SELECTOR` is excluded from every distance sum, and a `CARRIER`
//! is the in-band provenance lane. `mu` carries a dual role (it is both a
//! measured coordinate and the provenance carrier) and so gets its own variant
//! `CoordinateCarrier`.
//!
//! **Status: PROPOSED, not yet frozen.** Timothy Charles Holborn confirmed the
//! Option A assignment on 2026-07-04 as the baseline to encode, but the
//! taxonomy is not normatively frozen in the `.10d` v1 spec until the P0.7
//! conformance vectors land. The parser in [`crate::container_10d::header`]
//! accepts any non-`Undefined` assignment today; a future task can tighten it
//! to reject deviations from the frozen table once blessed.
//!
//! **t vs μ Reconciliation (T67, resolved 2026-08-19):**
//!
//! The earlier disagreement between this file and
//! `crates/qualia-core-db/src/tensor/mod.rs` has been reconciled:
//!
//! - **`t`** = **Coordinate Time** — the 4th dimension of spacetime. It is
//!   a `Coordinate` axis that participates in distance queries alongside
//!   `x`, `y`, `z`, `α`, `σ`. It is NOT the provenance carrier. The
//!   earlier "Provenance Ledger" label on `t` in `tensor/mod.rs` was
//!   incorrect and has been corrected.
//!
//! - **`μ`** = **Provenance Weight / Carrier** — epistemic metadata lane.
//!   It is the `CoordinateCarrier`: a dual-role axis that is both a
//!   measured coordinate AND the in-band provenance carrier. This is
//!   distinct from `t` (coordinate time). The provenance/consent lane
//!   is `μ`, not `t`.
//!
//! - These are different concepts that happen to both be associated with
//!   the 10th lane position. The 10D tensor has lanes for pose/query;
//!   the 10D epistemic manifold has lanes for attention/epistemic geometry.
//!
//! Cross-reference: T14 (name the two tens) and T15 (resolve t vs μ).
//!
//! Reference: `docs/plans/native-computational-geometry.md` §4.1, and the
//! execution plan's ⚑ "Axis-role taxonomy sign-off" curation datum.

/// Canonical axis order — matches the `Tensor10D` field order exactly so a
/// `Tensor10D` can be reinterpreted as ten `f32` lanes in this order without
/// re-shuffling. Index `i` here is index `i` into `axis_roles` in the header
/// and into the `folded_axes` bitmask of [`super::metric_check`].
///
/// The three spectral/epistemic axes use their normative Greek letters (`α`,
/// `μ`, `σ`) — the vocabulary the `.10d` spec and the AGENTS.md bit layout
/// define — not ASCII substitutes. The `Tensor10D` Rust struct fields are
/// named `alpha`/`mu`/`sigma` (Rust identifier convention), but the normative
/// axis names in the header, error messages, and this table are `α`/`μ`/`σ`.
pub const AXIS_ORDER: [&str; 10] = ["q", "v", "w", "x", "y", "z", "t", "α", "μ", "σ"];

/// Axis indices for the seven COORDINATE lanes (the ones that may participate
/// in distance). `μ` is included here because it is a coordinate in addition
/// to being the carrier.
pub const COORDINATE_AXES: [usize; 7] = [3, 4, 5, 6, 7, 8, 9]; // x,y,z,t,α,μ,σ

/// Axis indices for the three SELECTOR lanes (excluded from every distance sum).
pub const SELECTOR_AXES: [usize; 3] = [0, 1, 2]; // q,v,w

/// Index of `μ` — the dual-role coordinate + provenance carrier.
pub const MU_AXIS: usize = 8;

/// The role a single tensor axis plays in the `.10d` distance/query model.
///
/// Encoded as a `u8` in the header's `axis_roles[10]` array. `Undefined` is the
/// sentinel the parser rejects — a header with any axis left undefined fails
/// closed (the "missing-or-undefined axis role" acceptance gate).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisRole {
    /// Sentinel for "no role assigned". The parser rejects any header carrying
    /// this value — every axis must declare a concrete role.
    Undefined = 0,
    /// Excluded from every distance sum. `q`, `v`, `w`.
    Selector = 1,
    /// Participates in distance. `x`, `y`, `z`, `t`, `α`, `σ`.
    Coordinate = 2,
    /// In-band provenance lane only (not a coordinate). Reserved for a future
    /// taxonomy where `μ` is carrier-only; not used by the proposed table.
    Carrier = 3,
    /// Dual-role: a `COORDINATE` that is also the in-band provenance `CARRIER`.
    /// This is `μ`'s role under the proposed Option A taxonomy — it preserves
    /// the foundational promise that provenance/consent can exert geometric
    /// proximity or weight during queries.
    CoordinateCarrier = 4,
}

impl AxisRole {
    /// True if this role participates in distance (pure coordinate or the
    /// dual-role coordinate+carrier).
    #[inline]
    pub const fn is_coordinate(self) -> bool {
        matches!(self, AxisRole::Coordinate | AxisRole::CoordinateCarrier)
    }

    /// True if this role carries the provenance lane.
    #[inline]
    pub const fn is_carrier(self) -> bool {
        matches!(self, AxisRole::Carrier | AxisRole::CoordinateCarrier)
    }

    /// Decode a raw `u8` back to an `AxisRole`, or `None` if the value is not a
    /// defined variant. Used by the parser.
    #[inline]
    pub const fn from_u8(raw: u8) -> Option<AxisRole> {
        match raw {
            0 => Some(AxisRole::Undefined),
            1 => Some(AxisRole::Selector),
            2 => Some(AxisRole::Coordinate),
            3 => Some(AxisRole::Carrier),
            4 => Some(AxisRole::CoordinateCarrier),
            _ => None,
        }
    }
}

/// The proposed (not-yet-frozen) normative axis-role table, indexed by
/// `AXIS_ORDER`. This is the Option A assignment Timothy confirmed on
/// 2026-07-04:
///
/// - `q, v, w` → `Selector`
/// - `x, y, z, t, α, σ` → `Coordinate`
/// - `μ` → `CoordinateCarrier` (dual-role: coordinate + provenance carrier)
///
/// The header's `axis_roles` field is initialised from this table by
/// [`super::header::Container10dHeader::proposed`].
pub const PROPOSED_AXIS_ROLES: [AxisRole; 10] = [
    AxisRole::Selector,          // q
    AxisRole::Selector,          // v
    AxisRole::Selector,          // w
    AxisRole::Coordinate,        // x
    AxisRole::Coordinate,        // y
    AxisRole::Coordinate,        // z
    AxisRole::Coordinate,        // t
    AxisRole::Coordinate,        // α
    AxisRole::CoordinateCarrier, // μ
    AxisRole::Coordinate,        // σ
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposed_table_matches_option_a_confirmation() {
        // q,v,w are selectors
        for &i in &SELECTOR_AXES {
            assert_eq!(
                PROPOSED_AXIS_ROLES[i],
                AxisRole::Selector,
                "axis {} should be Selector",
                AXIS_ORDER[i]
            );
        }
        // x,y,z,t,α,σ are pure coordinates
        for &i in &[3usize, 4, 5, 6, 7, 9] {
            assert_eq!(
                PROPOSED_AXIS_ROLES[i],
                AxisRole::Coordinate,
                "axis {} should be Coordinate",
                AXIS_ORDER[i]
            );
        }
        // μ is the dual-role coordinate+carrier
        assert_eq!(PROPOSED_AXIS_ROLES[MU_AXIS], AxisRole::CoordinateCarrier);
        assert!(PROPOSED_AXIS_ROLES[MU_AXIS].is_coordinate());
        assert!(PROPOSED_AXIS_ROLES[MU_AXIS].is_carrier());
    }

    #[test]
    fn axis_role_round_trips_through_u8() {
        for raw in 0u8..=4 {
            let role = AxisRole::from_u8(raw).expect("0..=4 are defined");
            assert_eq!(role as u8, raw);
        }
        assert!(AxisRole::from_u8(5).is_none());
        assert!(AxisRole::from_u8(255).is_none());
    }

    #[test]
    fn coordinate_axes_cover_seven_lanes_including_mu() {
        assert_eq!(COORDINATE_AXES.len(), 7);
        assert!(
            COORDINATE_AXES.contains(&MU_AXIS),
            "mu is a coordinate under Option A"
        );
    }

    #[test]
    fn t67_reconciled_comment_present() {
        // T67: verify the reconciled comment block is present in this file.
        let source = include_str!("axis_role.rs");
        assert!(source.contains("t vs μ Reconciliation"), "missing reconciliation header");
        assert!(source.contains("Coordinate Time"), "missing t = Coordinate Time");
        assert!(source.contains("Provenance Weight / Carrier"), "missing μ = provenance carrier");
        assert!(source.contains("T67"), "missing T67 cross-reference");
    }

    #[test]
    fn t67_tensor10d_comments_reconciled() {
        // T67: verify the Tensor10D field comments in tensor/mod.rs are reconciled.
        let source = include_str!("../tensor/mod.rs");
        assert!(source.contains("Coordinate Time"), "tensor/mod.rs t field not reconciled");
        assert!(source.contains("Provenance Weight / Carrier"), "tensor/mod.rs mu field not reconciled");
        assert!(source.contains("T67"), "tensor/mod.rs missing T67 cross-reference");
        // The t field should be labeled "Coordinate Time" in its doc comment.
        let t_idx = source.find("pub t: f32").unwrap();
        let before_t = &source[..t_idx];
        assert!(before_t.contains("Coordinate Time"),
            "tensor/mod.rs t field doc should say 'Coordinate Time'");
    }
}
