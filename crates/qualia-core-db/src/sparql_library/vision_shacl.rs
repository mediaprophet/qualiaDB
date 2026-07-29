//! SHACL-style validation for visual observation graphs (design §6.3.8 / first-release).
//!
//! Caller-buffered, pure checks on NQuin slices. Does not claim certified model quality —
//! only structural integrity of epistemic claims (source, model digest, region bounds, score).

use crate::q_hash;
use crate::NQuin;

/// Predicate IRIs (must match `qualia-vision` semantic + client vision_ingest).
pub const P_VISUAL_OBSERVATION: &str = "https://ns.webizen.org/q42/VisualObservation";
pub const P_PROPOSES_CLASS: &str = "https://ns.webizen.org/q42/proposesClass";
pub const P_HAS_BBOX: &str = "https://ns.webizen.org/q42/hasBoundingBox";
pub const P_HAS_TRACK: &str = "https://ns.webizen.org/q42/hasTrackId";
pub const P_MODEL_DIGEST: &str = "https://ns.webizen.org/q42/modelDigest";
pub const P_HUMAN_REJECTS: &str = "https://ns.webizen.org/q42/humanRejects";
pub const P_HUMAN_CORRECTS: &str = "https://ns.webizen.org/q42/humanCorrectsClass";

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionShaclViolation {
    MissingModelDigest = 1,
    ObservationWithoutClass = 2,
    InvalidBbox = 3,
    ScoreOutOfRange = 4,
    EmptyGraph = 5,
    OrphanClassProposal = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisionShaclReport {
    pub ok: bool,
    pub observation_count: u32,
    pub class_count: u32,
    pub bbox_count: u32,
    pub model_digest_count: u32,
    pub human_attestation_count: u32,
    pub violation: Option<VisionShaclViolation>,
}

impl VisionShaclReport {
    pub const EMPTY_OK: Self = Self {
        ok: false,
        observation_count: 0,
        class_count: 0,
        bbox_count: 0,
        model_digest_count: 0,
        human_attestation_count: 0,
        violation: Some(VisionShaclViolation::EmptyGraph),
    };
}

#[inline]
fn unpack_bbox(v: u64) -> (u16, u16, u16, u16) {
    (
        (v & 0xFFFF) as u16,
        ((v >> 16) & 0xFFFF) as u16,
        ((v >> 32) & 0xFFFF) as u16,
        ((v >> 48) & 0xFFFF) as u16,
    )
}

/// Validate a vision observation bundle (media hash as subject of observations optional).
pub fn validate_vision_observation_graph(quins: &[NQuin]) -> VisionShaclReport {
    if quins.is_empty() {
        return VisionShaclReport::EMPTY_OK;
    }

    let p_obs = q_hash(P_VISUAL_OBSERVATION);
    let p_class = q_hash(P_PROPOSES_CLASS);
    let p_bbox = q_hash(P_HAS_BBOX);
    let p_track = q_hash(P_HAS_TRACK);
    let p_model = q_hash(P_MODEL_DIGEST);
    let p_rej = q_hash(P_HUMAN_REJECTS);
    let p_corr = q_hash(P_HUMAN_CORRECTS);

    let mut report = VisionShaclReport {
        ok: true,
        observation_count: 0,
        class_count: 0,
        bbox_count: 0,
        model_digest_count: 0,
        human_attestation_count: 0,
        violation: None,
    };

    // Collect instance hashes from observations (fixed stack buffer).
    let mut instances = [0u64; 64];
    let mut n_inst = 0usize;

    for q in quins {
        if q.predicate == p_model {
            report.model_digest_count = report.model_digest_count.saturating_add(1);
            if q.object == 0 {
                report.ok = false;
                report.violation = Some(VisionShaclViolation::MissingModelDigest);
            }
        } else if q.predicate == p_obs {
            report.observation_count = report.observation_count.saturating_add(1);
            if n_inst < instances.len() {
                instances[n_inst] = q.object;
                n_inst += 1;
            }
            // score in metadata low 16
            let score = (q.metadata & 0xFFFF) as u32;
            if score > 65535 {
                report.ok = false;
                report.violation = Some(VisionShaclViolation::ScoreOutOfRange);
            }
        } else if q.predicate == p_class {
            report.class_count = report.class_count.saturating_add(1);
            if q.object == 0 {
                report.ok = false;
                report.violation = Some(VisionShaclViolation::ObservationWithoutClass);
            }
        } else if q.predicate == p_bbox {
            report.bbox_count = report.bbox_count.saturating_add(1);
            let (x0, y0, x1, y1) = unpack_bbox(q.object);
            if x1 < x0 || y1 < y0 {
                report.ok = false;
                report.violation = Some(VisionShaclViolation::InvalidBbox);
            }
        } else if q.predicate == p_track {
            // track id may be 0 (untracked) — allowed
        } else if q.predicate == p_rej || q.predicate == p_corr {
            report.human_attestation_count = report.human_attestation_count.saturating_add(1);
        }
    }

    if report.model_digest_count == 0 && report.observation_count > 0 {
        report.ok = false;
        report.violation = Some(VisionShaclViolation::MissingModelDigest);
    }

    // Each observation instance should have a class proposal (when we have room to check).
    if report.ok && report.observation_count > 0 {
        for i in 0..n_inst {
            let inst = instances[i];
            let has_class = quins
                .iter()
                .any(|q| q.predicate == p_class && q.subject == inst);
            if !has_class {
                report.ok = false;
                report.violation = Some(VisionShaclViolation::ObservationWithoutClass);
                break;
            }
        }
    }

    // Orphan class with no observation is a soft fail only if we have classes without any obs.
    if report.ok && report.class_count > 0 && report.observation_count == 0 {
        report.ok = false;
        report.violation = Some(VisionShaclViolation::OrphanClassProposal);
    }

    report
}

/// Constraints description for tooling (hashes only).
pub fn vision_shape_predicate_hashes(out: &mut [u64]) -> usize {
    let preds = [
        q_hash(P_MODEL_DIGEST),
        q_hash(P_VISUAL_OBSERVATION),
        q_hash(P_PROPOSES_CLASS),
        q_hash(P_HAS_BBOX),
        q_hash(P_HAS_TRACK),
        q_hash(P_HUMAN_REJECTS),
        q_hash(P_HUMAN_CORRECTS),
    ];
    let n = preds.len().min(out.len());
    out[..n].copy_from_slice(&preds[..n]);
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(s: u64, p: u64, o: u64, c: u64, m: u64) -> NQuin {
        NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: c,
            metadata: m,
            parity: s ^ p ^ o ^ c ^ m,
        }
    }

    #[test]
    fn valid_bundle_passes() {
        let media = 9u64;
        let inst = 2u64;
        let model = 7u64;
        let bbox = (0u64) | (0u64 << 16) | (1000u64 << 32) | (2000u64 << 48);
        let quins = [
            q(media, q_hash(P_MODEL_DIGEST), model, 1, 64),
            q(media, q_hash(P_VISUAL_OBSERVATION), inst, 1, 1000),
            q(inst, q_hash(P_PROPOSES_CLASS), 99, 1, 1000),
            q(inst, q_hash(P_HAS_BBOX), bbox, 1, 1000),
        ];
        let r = validate_vision_observation_graph(&quins);
        assert!(r.ok, "{r:?}");
        assert_eq!(r.observation_count, 1);
        assert_eq!(r.model_digest_count, 1);
    }

    #[test]
    fn missing_digest_fails() {
        let quins = [q(9, q_hash(P_VISUAL_OBSERVATION), 2, 1, 1000)];
        let r = validate_vision_observation_graph(&quins);
        assert!(!r.ok);
        assert_eq!(r.violation, Some(VisionShaclViolation::MissingModelDigest));
    }

    #[test]
    fn invalid_bbox_fails() {
        // x1 < x0
        let bbox = (5000u64) | (0u64 << 16) | (100u64 << 32) | (2000u64 << 48);
        let quins = [
            q(9, q_hash(P_MODEL_DIGEST), 7, 1, 1),
            q(9, q_hash(P_VISUAL_OBSERVATION), 2, 1, 1),
            q(2, q_hash(P_PROPOSES_CLASS), 99, 1, 1),
            q(2, q_hash(P_HAS_BBOX), bbox, 1, 1),
        ];
        let r = validate_vision_observation_graph(&quins);
        assert!(!r.ok);
        assert_eq!(r.violation, Some(VisionShaclViolation::InvalidBbox));
    }

    #[test]
    fn human_reject_does_not_require_erasing_machine() {
        let quins = [
            q(9, q_hash(P_MODEL_DIGEST), 7, 1, 1),
            q(9, q_hash(P_VISUAL_OBSERVATION), 2, 1, 1),
            q(2, q_hash(P_PROPOSES_CLASS), 99, 1, 1),
            q(0xD1D, q_hash(P_HUMAN_REJECTS), 2, 1, 0),
        ];
        let r = validate_vision_observation_graph(&quins);
        assert!(r.ok);
        assert_eq!(r.human_attestation_count, 1);
        assert_eq!(r.class_count, 1); // machine claim retained
    }
}
