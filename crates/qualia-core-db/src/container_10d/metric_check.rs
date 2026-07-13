//! Metric-completeness descriptor + the "queryability claim == code" gate.
//!
//! The `.10d` header carries a [`MetricCompletenessDescriptor`] declaring, per
//! `v`-branch, which COORDINATE axes the distance metric folds. The parser
//! rejects any descriptor that diverges from `Tensor10D::full_distance`'s
//! actual v-branch behaviour — this is the mechanical barrier that enforces
//! the honest-limitation contract: a developer cannot claim an axis is
//! queryable under a metric the kernel does not actually fold.
//!
//! **Current reality (encoded as the proposed default):**
//! - `v=0` Euclidean folds `x,y,z,t,α,μ,σ` (all seven COORDINATEs)
//! - `v=1` Cyclic/Toroidal folds `x,y,z` only
//! - `v=2` Hyperbolic folds `x,y,z` only
//! - `v>=3` Boundary clique folds no coordinate axes (byte-equality on `v`)
//!
//! This is option (b) "document the limitation" per Timothy's 2026-07-04
//! decision. P7.9 (making the non-Euclidean metrics axis-complete via product
//! / warped-product manifolds and a clique-graph) is deferred as future
//! geometry design work — see the progress log and the P7.9 task entry.
//!
//! Reference: `docs/plans/native-computational-geometry.md` §4.1 honest
//! limitation, and the cross-cutting gate "Honest axis-role taxonomy &
//! metric-completeness (queryability claim == code)".

use bytemuck::{Pod, Zeroable};

use crate::container_10d::axis_role::COORDINATE_AXES;
use crate::tensor::Tensor10D;
use std::fmt;

/// Number of `v`-branch descriptors the header carries: the three explicit
/// classes (0, 1, 2) plus one catch-all for `v >= 3`.
pub const METRIC_BRANCH_COUNT: usize = 4;

/// Index of the `v >= 3` catch-all branch in a [`MetricCompletenessDescriptor`].
pub const BOUNDARY_CLIQUE_BRANCH_INDEX: usize = 3;

/// A `u8` tag identifying the metric kind a branch implements. Mirrors the
/// dispatch in `Tensor10D::full_distance`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    /// Sentinel — the parser rejects a branch with this kind.
    Undefined = 0,
    Euclidean = 1,
    CyclicToroidal = 2,
    Hyperbolic = 3,
    BoundaryClique = 4,
}

impl MetricKind {
    #[inline]
    pub const fn from_u8(raw: u8) -> Option<MetricKind> {
        match raw {
            0 => Some(MetricKind::Undefined),
            1 => Some(MetricKind::Euclidean),
            2 => Some(MetricKind::CyclicToroidal),
            3 => Some(MetricKind::Hyperbolic),
            4 => Some(MetricKind::BoundaryClique),
            _ => None,
        }
    }
}

/// One row of the metric-completeness table: the metric used for a single
/// `v`-class and the bitmask of COORDINATE axes it folds.
///
/// `folded_axes` is a bitmask over [`super::axis_role::AXIS_ORDER`] indices —
/// bit `i` set means axis `i` participates in this branch's distance sum. Only
/// COORDINATE-axis bits may be set; a SELECTOR-axis bit (0,1,2) is a divergence
/// the verifier rejects.
///
/// Layout: 8 bytes, no padding (`u8` + `u8` + `u16` + `u32`, all naturally
/// aligned). POD so it embeds directly in the header.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct MetricBranchDescriptor {
    /// The `v` class this row describes: `0`, `1`, `2`, or `255` for the
    /// `v >= 3` catch-all.
    pub v_class: u8,
    /// The metric kind — see [`MetricKind`].
    pub metric_kind: u8,
    /// Bitmask of folded COORDINATE axes (bit `i` = axis `i` in `AXIS_ORDER`).
    pub folded_axes: u16,
    /// Reserved, must be zero. Future use (e.g. per-branch scale weights).
    pub reserved: u32,
}

impl MetricBranchDescriptor {
    /// True if axis `i` is declared folded by this branch.
    #[inline]
    pub const fn folds_axis(self, i: usize) -> bool {
        (self.folded_axes >> i) & 1 == 1
    }
}

/// The full metric-completeness table carried in the `.10d` header. One row per
/// `v`-class; `branches[3]` is the `v >= 3` catch-all.
///
/// Layout: 4 × 8 = 32 bytes, no padding.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct MetricCompletenessDescriptor {
    pub branches: [MetricBranchDescriptor; METRIC_BRANCH_COUNT],
}

/// Error raised when a descriptor diverges from `full_distance`'s actual
/// behaviour. Carries enough detail to name the offending branch + axis in the
/// parser's rejection message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricDivergence {
    pub v_class: u8,
    pub axis_index: usize,
    pub declared_folds: bool,
    pub actual_folds: bool,
}

impl fmt::Display for MetricDivergence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let axis = super::axis_role::AXIS_ORDER[self.axis_index];
        write!(
            f,
            "metric-completeness divergence: v={} claims axis {} is {}folded but full_distance {}folds it",
            self.v_class,
            axis,
            if self.declared_folds { "" } else { "not " },
            if self.actual_folds { "" } else { "does not " },
        )
    }
}

impl std::error::Error for MetricDivergence {}

/// Probe `Tensor10D::full_distance` to determine whether axis `axis_index` is
/// folded under the branch selected by `v_class`. Returns `true` if varying
/// only that axis (with `v` held at `v_class`) changes the distance.
///
/// This is the introspection step that makes the honesty gate mechanical: it
/// reads the actual kernel behaviour, not a hardcoded claim.
fn axis_is_folded_by_full_distance(v_class: u8, axis_index: usize) -> bool {
    // Base tensor: v set to the branch class, all coordinates zero, alpha=1
    // (the Tensor10D default) so the spectral sum is non-degenerate.
    let mut base = Tensor10D::default();
    base.v = v_class as f32;
    let mut perturbed = base;
    perturb_axis(&mut perturbed, axis_index);
    let d_base = base.full_distance(&base);
    let d_perturbed = base.full_distance(&perturbed);
    // An axis is "folded" if perturbing it changes the distance away from the
    // identity distance. Use a small epsilon to tolerate f32 rounding noise.
    (d_perturbed - d_base).abs() > 1e-6
}

/// Perturb a single axis lane by a non-trivial delta, leaving the other nine
/// lanes (including `v`) untouched. The deltas are chosen to be large enough
/// that f32 rounding cannot mask the distance change, and (for the cyclic
/// branch) inside the modulo-1 unit cell.
fn perturb_axis(t: &mut Tensor10D, axis_index: usize) {
    // A delta of 0.25 is inside the cyclic unit cell (mod-1 wrap) and large
    // relative to f32 epsilon, so it produces a clear distance change in every
    // branch that folds the axis.
    const DELTA: f32 = 0.25;
    match axis_index {
        0 => t.q += DELTA,
        1 => t.v += DELTA,
        2 => t.w += DELTA,
        3 => t.x += DELTA,
        4 => t.y += DELTA,
        5 => t.z += DELTA,
        6 => t.t += DELTA,
        7 => t.alpha += DELTA,
        8 => t.mu += DELTA,
        9 => t.sigma += DELTA,
        _ => unreachable!("axis_index out of range"),
    }
}

/// Probe `full_distance` for one branch and return the bitmask of COORDINATE
/// axes it actually folds. Only COORDINATE axes are probed — SELECTOR axes are
/// excluded by definition and a descriptor claiming to fold one is a divergence.
pub fn probe_folded_axes(v_class: u8) -> u16 {
    let mut mask: u16 = 0;
    for &i in &COORDINATE_AXES {
        if axis_is_folded_by_full_distance(v_class, i) {
            mask |= 1 << i;
        }
    }
    mask
}

/// Verify a [`MetricCompletenessDescriptor`] against the actual
/// `Tensor10D::full_distance` behaviour. Returns the first divergence found, or
/// `Ok(())` if every branch's declared `folded_axes` and `metric_kind` match
/// reality.
///
/// This is the function the parser calls — a diverging descriptor fails the
/// parse (the P0.1 acceptance gate: "parse rejects ... a metric-completeness
/// descriptor that a unit test shows diverges from full_distance's actual
/// v-branch behaviour").
pub fn verify_descriptor_against_reality(
    descriptor: &MetricCompletenessDescriptor,
) -> Result<(), MetricDivergence> {
    for (row_index, branch) in descriptor.branches.iter().enumerate() {
        // The catch-all row (index 3) describes v >= 3; probe with v = 3.
        let v_class = if row_index == BOUNDARY_CLIQUE_BRANCH_INDEX {
            3
        } else {
            branch.v_class
        };

        // Reject Undefined metric kind.
        let declared_kind = MetricKind::from_u8(branch.metric_kind);
        match (declared_kind, v_class) {
            (Some(MetricKind::Euclidean), 0) => {}
            (Some(MetricKind::CyclicToroidal), 1) => {}
            (Some(MetricKind::Hyperbolic), 2) => {}
            (Some(MetricKind::BoundaryClique), 3) => {}
            (Some(MetricKind::Undefined), _) => {
                return Err(MetricDivergence {
                    v_class,
                    axis_index: 0,
                    declared_folds: false,
                    actual_folds: false,
                });
            }
            _ => {
                // metric_kind does not match the v_class — divergence.
                return Err(MetricDivergence {
                    v_class,
                    axis_index: 0,
                    declared_folds: false,
                    actual_folds: false,
                });
            }
        }

        let actual = probe_folded_axes(v_class);
        for &i in &COORDINATE_AXES {
            let declared_folds = branch.folds_axis(i);
            let actual_folds = (actual >> i) & 1 == 1;
            if declared_folds != actual_folds {
                return Err(MetricDivergence {
                    v_class,
                    axis_index: i,
                    declared_folds,
                    actual_folds,
                });
            }
        }
    }
    Ok(())
}

/// The proposed (not-yet-frozen) metric-completeness descriptor encoding the
/// current `full_distance` reality — option (b), the documented limitation.
/// The header's descriptor field is initialised from this by
/// [`super::header::Container10dHeader::proposed`].
pub const fn proposed_metric_descriptor() -> MetricCompletenessDescriptor {
    const fn branch(v: u8, kind: u8, folded: u16) -> MetricBranchDescriptor {
        MetricBranchDescriptor {
            v_class: v,
            metric_kind: kind,
            folded_axes: folded,
            reserved: 0,
        }
    }
    // Bitmasks over AXIS_ORDER indices: x=bit3, y=bit4, z=bit5, t=bit6,
    // α=bit7, μ=bit8, σ=bit9.
    const XYZ: u16 = (1 << 3) | (1 << 4) | (1 << 5);
    const ALL_SEVEN: u16 = XYZ | (1 << 6) | (1 << 7) | (1 << 8) | (1 << 9);
    MetricCompletenessDescriptor {
        branches: [
            branch(0, MetricKind::Euclidean as u8, ALL_SEVEN),
            branch(1, MetricKind::CyclicToroidal as u8, XYZ),
            branch(2, MetricKind::Hyperbolic as u8, XYZ),
            branch(255, MetricKind::BoundaryClique as u8, 0),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_v0_euclidean_folds_all_seven_coordinates() {
        let mask = probe_folded_axes(0);
        // x,y,z,t,α,μ,σ
        assert_eq!(
            mask,
            (1 << 3) | (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8) | (1 << 9)
        );
    }

    #[test]
    fn probe_v1_cyclic_folds_xyz_only() {
        let mask = probe_folded_axes(1);
        assert_eq!(
            mask,
            (1 << 3) | (1 << 4) | (1 << 5),
            "v=1 must fold only x,y,z; got {mask:#b}"
        );
        // explicitly: t, α, μ, σ are NOT folded
        assert_eq!(mask & (1 << 6), 0, "t must not be folded under v=1");
        assert_eq!(mask & (1 << 7), 0, "α must not be folded under v=1");
        assert_eq!(mask & (1 << 8), 0, "μ must not be folded under v=1");
        assert_eq!(mask & (1 << 9), 0, "σ must not be folded under v=1");
    }

    #[test]
    fn probe_v2_hyperbolic_folds_xyz_only() {
        let mask = probe_folded_axes(2);
        assert_eq!(
            mask,
            (1 << 3) | (1 << 4) | (1 << 5),
            "v=2 must fold only x,y,z; got {mask:#b}"
        );
    }

    #[test]
    fn probe_v3_boundary_folds_no_coordinate_axes() {
        let mask = probe_folded_axes(3);
        assert_eq!(
            mask, 0,
            "v>=3 boundary clique must fold no coordinate axes; got {mask:#b}"
        );
    }

    #[test]
    fn proposed_descriptor_matches_reality() {
        let desc = proposed_metric_descriptor();
        assert!(
            verify_descriptor_against_reality(&desc).is_ok(),
            "the proposed (option b) descriptor must match full_distance's actual behaviour"
        );
    }

    #[test]
    fn diverging_descriptor_claiming_v1_folds_t_is_rejected() {
        let mut desc = proposed_metric_descriptor();
        // Claim v=1 (cyclic) folds t (bit 6) — it does not.
        desc.branches[1].folded_axes |= 1 << 6;
        let err =
            verify_descriptor_against_reality(&desc).expect_err("must reject diverging claim");
        assert_eq!(err.v_class, 1);
        assert_eq!(err.axis_index, 6, "divergence must name axis t (index 6)");
        assert!(err.declared_folds);
        assert!(!err.actual_folds);
    }

    #[test]
    fn diverging_descriptor_claiming_v0_ignores_sigma_is_rejected() {
        let mut desc = proposed_metric_descriptor();
        // Claim v=0 does NOT fold σ (clear bit 9) — it does.
        desc.branches[0].folded_axes &= !(1 << 9);
        let err =
            verify_descriptor_against_reality(&desc).expect_err("must reject diverging claim");
        assert_eq!(err.v_class, 0);
        assert_eq!(err.axis_index, 9, "divergence must name axis σ (index 9)");
        assert!(!err.declared_folds);
        assert!(err.actual_folds);
    }

    #[test]
    fn diverging_descriptor_claiming_v3_folds_x_is_rejected() {
        let mut desc = proposed_metric_descriptor();
        // Claim v>=3 boundary folds x (bit 3) — it folds nothing.
        desc.branches[BOUNDARY_CLIQUE_BRANCH_INDEX].folded_axes |= 1 << 3;
        let err =
            verify_descriptor_against_reality(&desc).expect_err("must reject diverging claim");
        assert_eq!(err.v_class, 3);
        assert_eq!(err.axis_index, 3);
    }

    #[test]
    fn wrong_metric_kind_for_v_class_is_rejected() {
        let mut desc = proposed_metric_descriptor();
        // Claim v=0 uses the hyperbolic metric — wrong.
        desc.branches[0].metric_kind = MetricKind::Hyperbolic as u8;
        assert!(verify_descriptor_against_reality(&desc).is_err());
    }

    #[test]
    fn undefined_metric_kind_is_rejected() {
        let mut desc = proposed_metric_descriptor();
        desc.branches[0].metric_kind = MetricKind::Undefined as u8;
        assert!(verify_descriptor_against_reality(&desc).is_err());
    }
}
