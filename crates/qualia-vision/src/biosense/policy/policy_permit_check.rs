//! Local federated policy permit-check (D4.04).
//!
//! The honest LOCAL answer to a federated question — "is this camera's biometric
//! use permitted?" — by composing the existing fail-closed policy engine
//! [`evaluate_processing_act`]. It does NOT reimplement the consent/purpose matrix.
//!
//! FEDERATION NOTE: the transport that carries such a question between peers (a
//! signed FED ask/answer over the mixnet) is future work. This function is the
//! purely local determination a node makes for a camera it controls; a federated
//! wrapper would attach identity/provenance around this same answer. Until that
//! exists, the answer is honest but local-only — it speaks for THIS node's policy,
//! not a network-wide grant.

use crate::biosense::consent::BiosenseConsent;
use crate::biosense::policy::{evaluate_processing_act, PolicyDecision, ProcessingAct};

/// Answer to a "may this camera perform `act`?" check: the decision, a static
/// human-readable reason, and the camera it pertains to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermitAnswer {
    pub decision: PolicyDecision,
    pub reason: &'static str,
    pub camera_id: u64,
}

/// Fail-closed permit check for a camera + processing act under a consent record.
///
/// Composes [`evaluate_processing_act`] and attaches a clear static reason per
/// outcome. No consent → `Forbid`. Never panics; no `unwrap`.
pub fn policy_permit_check(
    camera_id: u64,
    act: ProcessingAct,
    consent: BiosenseConsent,
) -> PermitAnswer {
    let decision = evaluate_processing_act(consent, act);
    let reason = reason_for(act, decision, consent.allow_process);
    PermitAnswer {
        decision,
        reason,
        camera_id,
    }
}

/// Map (act, decision) to an honest, non-empty static reason string.
fn reason_for(act: ProcessingAct, decision: PolicyDecision, allow_process: bool) -> &'static str {
    match decision {
        PolicyDecision::Permit => match act {
            ProcessingAct::MotionOnly => "motion-only processing permitted",
            ProcessingAct::FaceDetect => {
                "face detection permitted under surveillance-policy purpose"
            }
            ProcessingAct::FaceEmbed | ProcessingAct::Identify1N => {
                "identification permitted: purpose-bound security grant with template consent"
            }
            ProcessingAct::Rppg => "rPPG permitted for self-monitoring / research purpose",
            ProcessingAct::Affect => {
                "affect estimation permitted for self-monitoring / research purpose"
            }
            ProcessingAct::RecordStore => {
                "record storage permitted by template or graph-observation consent"
            }
        },
        PolicyDecision::Forbid => {
            if !allow_process {
                return "fail-closed: no processing consent for this camera";
            }
            match act {
                ProcessingAct::FaceEmbed | ProcessingAct::Identify1N => {
                    "identification requires a purpose-bound security grant"
                }
                ProcessingAct::FaceDetect => {
                    "face detection requires a surveillance-policy purpose"
                }
                ProcessingAct::Rppg => "rPPG requires a self-monitoring or research purpose",
                ProcessingAct::Affect => {
                    "affect estimation requires a self-monitoring or research purpose"
                }
                ProcessingAct::RecordStore => {
                    "record storage requires template or graph-observation consent"
                }
                // MotionOnly is always permitted; unreachable in the Forbid arm.
                ProcessingAct::MotionOnly => {
                    "motion-only processing not permitted for this consent"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::consent::BiosensePurpose;

    #[test]
    fn denied_consent_forbids_with_reason() {
        let consent = BiosenseConsent::denied(BiosensePurpose::Security);
        let a = policy_permit_check(0xCA51, ProcessingAct::FaceEmbed, consent);
        assert_eq!(a.decision, PolicyDecision::Forbid);
        assert_eq!(a.camera_id, 0xCA51);
        assert_eq!(
            a.reason,
            "fail-closed: no processing consent for this camera"
        );
        assert!(!a.reason.is_empty());
    }

    #[test]
    fn security_template_grant_permits_face_embed() {
        let consent = BiosenseConsent::grant_security_template(0xABC);
        let a = policy_permit_check(0xCA52, ProcessingAct::FaceEmbed, consent);
        assert_eq!(a.decision, PolicyDecision::Permit);
        assert_eq!(a.camera_id, 0xCA52);
        assert!(!a.reason.is_empty());
    }

    #[test]
    fn surveillance_face_embed_forbidden_needs_security_grant() {
        let consent = BiosenseConsent::grant_process(BiosensePurpose::SurveillancePolicy, 1);
        let a = policy_permit_check(0xCA53, ProcessingAct::FaceEmbed, consent);
        assert_eq!(a.decision, PolicyDecision::Forbid);
        assert_eq!(
            a.reason,
            "identification requires a purpose-bound security grant"
        );
    }

    #[test]
    fn motion_only_always_permitted() {
        // Even a surveillance-policy (non-security) consent permits motion-only.
        let consent = BiosenseConsent::grant_process(BiosensePurpose::SurveillancePolicy, 1);
        let a = policy_permit_check(0xCA54, ProcessingAct::MotionOnly, consent);
        assert_eq!(a.decision, PolicyDecision::Permit);
        assert_eq!(a.reason, "motion-only processing permitted");
        assert_eq!(a.camera_id, 0xCA54);
    }

    #[test]
    fn every_answer_carries_camera_id_and_nonempty_reason() {
        let grants = [
            BiosenseConsent::denied(BiosensePurpose::Research),
            BiosenseConsent::grant_process(BiosensePurpose::WellfairSelfMonitor, 2),
            BiosenseConsent::grant_security_template(3),
        ];
        let acts = [
            ProcessingAct::MotionOnly,
            ProcessingAct::FaceDetect,
            ProcessingAct::FaceEmbed,
            ProcessingAct::Identify1N,
            ProcessingAct::Rppg,
            ProcessingAct::Affect,
            ProcessingAct::RecordStore,
        ];
        for (i, &c) in grants.iter().enumerate() {
            for &act in &acts {
                let cam = 0x1000 + i as u64;
                let a = policy_permit_check(cam, act, c);
                assert_eq!(a.camera_id, cam);
                assert!(!a.reason.is_empty(), "reason must be non-empty for {act:?}");
            }
        }
    }
}
