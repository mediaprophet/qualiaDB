//! Zero-heap SHACL/ShEx validation engine for holistic twins and geometry assets (§2.8 / Appended Spec).
//!
//! Validates `NQuin` slices against structural invariants defined in `schemas/holistic-twin.shex`
//! and `schemas/geometry-asset.shacl.ttl` without dynamic heap allocations.

use crate::entity_view::bifurcation::BifurcationContext64;
use crate::q42::spatial_predicates::*;
use crate::q_hash;
use crate::NQuin;

/// Violations discovered during holistic twin and geometry asset validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryShaclViolation {
    /// Parity verification failed.
    InvalidParity { subject: u64, predicate: u64 },
    /// Quin sensitivity exceeded context ceiling.
    SensitivityCeilingExceeded { declared: u8, max_permitted: u8 },
    /// Quantum branch exceeded permitted ceiling.
    QuantumRangeViolation { branch_q: u32, allowed_q_max: u32 },
    /// Scale tier is out of valid range [0..=7].
    ScaleTierOutOfRange { tier: u8 },
    /// Topology class is out of valid range [0..=6].
    TopologyClassOutOfRange { class_id: u8 },
    /// Vertex count is out of valid range [1..=2^22].
    VertexCountOutOfRange { count: u32, max: u32 },
    /// Triangle count is out of valid range [1..=2^22].
    TriangleCountOutOfRange { count: u32, max: u32 },
    /// Bounding box min coordinate exceeds max along the given axis (0=x, 1=y, 2=z).
    InvertedBbox { axis: u8 },
    /// Claimed CRC-32C did not match container digest.
    CompiledDigestMismatch { claimed: u32, actual: u32 },
}

/// Conformance verdict for holistic twin SHACL/ShEx validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GeometryShaclVerdict {
    /// True if zero violations were recorded.
    pub is_valid: bool,
    /// Total count of violations written into output slice.
    pub violations_count: usize,
    /// Total count of quins evaluated.
    pub checked_quins_count: usize,
    /// Verified compiled digest (if observed).
    pub verified_digest: u32,
}

const P_VERTEX_COUNT: u64 = q_hash("geo:vertexCount");
const P_TRIANGLE_COUNT: u64 = q_hash("geo:triangleCount");
const P_COMPILED_DIGEST: u64 = q_hash("geo:compiledDigest");

/// Zero-heap conformance validator over an `NQuin` slice with optional `BifurcationContext64`
/// and container CRC-32C verification.
pub fn validate_holistic_twin_quins_zero_heap(
    quins: &[NQuin],
    bifurcation_ctx: Option<&BifurcationContext64>,
    actual_container_crc32c: Option<u32>,
    violations_out: &mut [GeometryShaclViolation],
) -> GeometryShaclVerdict {
    let mut written = 0;
    let mut verified_digest = 0u32;

    let mut record_violation = |v: GeometryShaclViolation, w: &mut usize| {
        if *w < violations_out.len() {
            violations_out[*w] = v;
            *w += 1;
        }
    };

    let mut bbox_min_obj: Option<u64> = None;
    let mut bbox_max_obj: Option<u64> = None;

    for q in quins {
        // 1. ECC Parity
        if !q.verify_ecc_parity() {
            record_violation(
                GeometryShaclViolation::InvalidParity {
                    subject: q.subject,
                    predicate: q.predicate,
                },
                &mut written,
            );
        }

        // 2. Sensitivity Ceiling
        if let Some(ctx) = bifurcation_ctx {
            let sens = q.get_sensitivity_byte();
            if sens > ctx.max_sensitivity_class {
                record_violation(
                    GeometryShaclViolation::SensitivityCeilingExceeded {
                        declared: sens,
                        max_permitted: ctx.max_sensitivity_class,
                    },
                    &mut written,
                );
            }
        }

        // 3. Predicate Invariants
        match q.predicate {
            PRED_GEO_HAS_SCALE_TIER => {
                let tier = (q.object & 0xFF) as u8;
                if tier > 7 {
                    record_violation(
                        GeometryShaclViolation::ScaleTierOutOfRange { tier },
                        &mut written,
                    );
                }
            }
            PRED_GEO_HAS_TOPOLOGY_CLASS => {
                let class_id = (q.object & 0xFF) as u8;
                if class_id > 6 {
                    record_violation(
                        GeometryShaclViolation::TopologyClassOutOfRange { class_id },
                        &mut written,
                    );
                }
            }
            PRED_EPI_HAS_QUANTUM_RANGE => {
                let branch_q = q.object as u32;
                if let Some(ctx) = bifurcation_ctx {
                    if branch_q > ctx.allowed_q_max {
                        record_violation(
                            GeometryShaclViolation::QuantumRangeViolation {
                                branch_q,
                                allowed_q_max: ctx.allowed_q_max,
                            },
                            &mut written,
                        );
                    }
                }
            }
            P_VERTEX_COUNT => {
                let count = q.object as u32;
                if count == 0 || count > (1 << 22) {
                    record_violation(
                        GeometryShaclViolation::VertexCountOutOfRange {
                            count,
                            max: 1 << 22,
                        },
                        &mut written,
                    );
                }
            }
            P_TRIANGLE_COUNT => {
                let count = q.object as u32;
                if count == 0 || count > (1 << 22) {
                    record_violation(
                        GeometryShaclViolation::TriangleCountOutOfRange {
                            count,
                            max: 1 << 22,
                        },
                        &mut written,
                    );
                }
            }
            P_COMPILED_DIGEST | PRED_Q42_HAS_CONTAINER_10D => {
                let claimed = q.object as u32;
                verified_digest = claimed;
                if let Some(actual) = actual_container_crc32c {
                    if claimed != actual {
                        record_violation(
                            GeometryShaclViolation::CompiledDigestMismatch { claimed, actual },
                            &mut written,
                        );
                    }
                }
            }
            PRED_GEO_HAS_PACKED_BBOX_MIN => {
                bbox_min_obj = Some(q.object);
            }
            PRED_GEO_HAS_PACKED_BBOX_MAX => {
                bbox_max_obj = Some(q.object);
            }
            _ => {}
        }
    }

    // 4. Bounding Box Inversion Check
    if let (Some(min_raw), Some(max_raw)) = (bbox_min_obj, bbox_max_obj) {
        let (min_x, min_y, min_z, _) = unpack_coord_with_scale(min_raw);
        let (max_x, max_y, max_z, _) = unpack_coord_with_scale(max_raw);
        if min_x > max_x {
            record_violation(GeometryShaclViolation::InvertedBbox { axis: 0 }, &mut written);
        }
        if min_y > max_y {
            record_violation(GeometryShaclViolation::InvertedBbox { axis: 1 }, &mut written);
        }
        if min_z > max_z {
            record_violation(GeometryShaclViolation::InvertedBbox { axis: 2 }, &mut written);
        }
    }

    GeometryShaclVerdict {
        is_valid: written == 0,
        violations_count: written,
        checked_quins_count: quins.len(),
        verified_digest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_quin(subject: u64, pred: u64, obj: u64, ctx: u64, meta: u64) -> NQuin {
        let parity = NQuin::calculate_parity(subject, pred, obj, ctx, meta);
        NQuin {
            subject,
            predicate: pred,
            object: obj,
            context: ctx,
            metadata: meta,
            parity,
        }
    }

    #[test]
    fn valid_holistic_twin_quins_pass() {
        let q1 = make_valid_quin(1, PRED_GEO_HAS_SCALE_TIER, 3, 0, 0);
        let q2 = make_valid_quin(1, PRED_GEO_HAS_TOPOLOGY_CLASS, 2, 0, 0);
        let q3 = make_valid_quin(1, P_COMPILED_DIGEST, 0xAABBCCDD, 0, 0);

        let quins = [q1, q2, q3];
        let mut violations = [GeometryShaclViolation::ScaleTierOutOfRange { tier: 0 }; 8];

        let verdict = validate_holistic_twin_quins_zero_heap(
            &quins,
            None,
            Some(0xAABBCCDD),
            &mut violations,
        );
        assert!(verdict.is_valid);
        assert_eq!(verdict.violations_count, 0);
        assert_eq!(verdict.checked_quins_count, 3);
        assert_eq!(verdict.verified_digest, 0xAABBCCDD);
    }

    #[test]
    fn invalid_scale_tier_is_caught() {
        let q = make_valid_quin(1, PRED_GEO_HAS_SCALE_TIER, 9, 0, 0);
        let quins = [q];
        let mut violations = [GeometryShaclViolation::ScaleTierOutOfRange { tier: 0 }; 4];

        let verdict = validate_holistic_twin_quins_zero_heap(
            &quins,
            None,
            None,
            &mut violations,
        );
        assert!(!verdict.is_valid);
        assert_eq!(verdict.violations_count, 1);
        assert_eq!(
            violations[0],
            GeometryShaclViolation::ScaleTierOutOfRange { tier: 9 }
        );
    }

    #[test]
    fn compiled_digest_mismatch_is_caught() {
        let q = make_valid_quin(1, P_COMPILED_DIGEST, 0x11112222, 0, 0);
        let quins = [q];
        let mut violations = [GeometryShaclViolation::ScaleTierOutOfRange { tier: 0 }; 4];

        let verdict = validate_holistic_twin_quins_zero_heap(
            &quins,
            None,
            Some(0x33334444),
            &mut violations,
        );
        assert!(!verdict.is_valid);
        assert_eq!(verdict.violations_count, 1);
        assert_eq!(
            violations[0],
            GeometryShaclViolation::CompiledDigestMismatch {
                claimed: 0x11112222,
                actual: 0x33334444,
            }
        );
    }

    #[test]
    fn inverted_bbox_is_caught() {
        let min_raw = pack_coord_with_scale(10.0, 0.0, 0.0, 0);
        let max_raw = pack_coord_with_scale(5.0, 0.0, 0.0, 0); // min_x > max_x

        let q1 = make_valid_quin(1, PRED_GEO_HAS_PACKED_BBOX_MIN, min_raw, 0, 0);
        let q2 = make_valid_quin(1, PRED_GEO_HAS_PACKED_BBOX_MAX, max_raw, 0, 0);

        let quins = [q1, q2];
        let mut violations = [GeometryShaclViolation::ScaleTierOutOfRange { tier: 0 }; 4];

        let verdict = validate_holistic_twin_quins_zero_heap(
            &quins,
            None,
            None,
            &mut violations,
        );
        assert!(!verdict.is_valid);
        assert_eq!(verdict.violations_count, 1);
        assert_eq!(violations[0], GeometryShaclViolation::InvertedBbox { axis: 0 });
    }
}
