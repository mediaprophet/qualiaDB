//! Local biometric processing policy evaluation (CCTV / pipeline stage gate).

use crate::biosense::consent::{BiosenseConsent, BiosensePurpose};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingAct {
    MotionOnly,
    FaceDetect,
    FaceEmbed,
    Identify1N,
    Rppg,
    Affect,
    RecordStore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    Permit,
    Forbid,
}

/// Fail-closed: forbid unless consent purpose allows the act.
pub fn evaluate_processing_act(consent: BiosenseConsent, act: ProcessingAct) -> PolicyDecision {
    if !consent.allow_process {
        return PolicyDecision::Forbid;
    }
    match (consent.purpose, act) {
        (_, ProcessingAct::MotionOnly) => PolicyDecision::Permit,
        (BiosensePurpose::SurveillancePolicy, ProcessingAct::FaceDetect) => PolicyDecision::Permit,
        (
            BiosensePurpose::SurveillancePolicy,
            ProcessingAct::FaceEmbed | ProcessingAct::Identify1N,
        ) => {
            PolicyDecision::Forbid // identification needs stronger grant
        }
        (BiosensePurpose::Security, ProcessingAct::FaceEmbed | ProcessingAct::Identify1N) => {
            if consent.allow_store_template {
                PolicyDecision::Permit
            } else {
                PolicyDecision::Forbid
            }
        }
        (BiosensePurpose::WellfairSelfMonitor | BiosensePurpose::Research, ProcessingAct::Rppg) => {
            PolicyDecision::Permit
        }
        (
            BiosensePurpose::Research | BiosensePurpose::WellfairSelfMonitor,
            ProcessingAct::Affect,
        ) => PolicyDecision::Permit,
        (_, ProcessingAct::RecordStore) => {
            if consent.allow_store_template || consent.allow_graph_observation {
                PolicyDecision::Permit
            } else {
                PolicyDecision::Forbid
            }
        }
        _ => PolicyDecision::Forbid,
    }
}

/// CCTV compliance: which stages may run.
pub fn cctv_stages_allowed(consent: BiosenseConsent) -> (bool, bool, bool) {
    // (motion, face_detect, face_embed)
    (
        evaluate_processing_act(consent, ProcessingAct::MotionOnly) == PolicyDecision::Permit,
        evaluate_processing_act(consent, ProcessingAct::FaceDetect) == PolicyDecision::Permit,
        evaluate_processing_act(consent, ProcessingAct::FaceEmbed) == PolicyDecision::Permit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surveillance_no_embed() {
        let c = BiosenseConsent::grant_process(BiosensePurpose::SurveillancePolicy, 1);
        assert_eq!(
            evaluate_processing_act(c, ProcessingAct::FaceEmbed),
            PolicyDecision::Forbid
        );
        assert_eq!(
            evaluate_processing_act(c, ProcessingAct::MotionOnly),
            PolicyDecision::Permit
        );
    }
}
