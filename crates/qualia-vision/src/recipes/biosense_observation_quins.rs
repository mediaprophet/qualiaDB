//! Compile biosense HR results into epistemic VisionQuin observations (E1).
//!
//! Not ground truth — proposals with confidence in metadata low bits.
//! Fail-closed: consent + deontic `ProcessingAct::Rppg` must Permit.

use crate::biosense::{
    evaluate_processing_act, BiosenseConsent, HrEstimate, PolicyDecision, ProcessingAct,
};
use crate::cv::error::CvError;
use crate::semantic::{q_hash, VisionQuin, CTX_VISION};

const P_PROPOSES_HR: &str = "https://ns.webizen.org/q42/proposesHeartRateBpm";
const P_HR_CONFIDENCE: &str = "https://ns.webizen.org/q42/heartRateConfidence";
const P_HR_SNR: &str = "https://ns.webizen.org/q42/heartRateSnr";

/// Pack f32 into object field (low 32 bits of bits pattern) + reserved float-family tag.
fn pack_f32_object(v: f32) -> u64 {
    (v.to_bits() as u64) | (0x5u64 << 60)
}

/// Emit observation quins for an HR estimate after deontic gate.
///
/// Returns number written (3 on success), or error if consent/policy denies
/// or the output buffer is too small.
pub fn compile_hr_observation_quins(
    consent: BiosenseConsent,
    media_subject: u64,
    hr: &HrEstimate,
    out: &mut [VisionQuin],
) -> Result<usize, CvError> {
    if !consent.may_process() {
        return Err(CvError::InvalidParameter);
    }
    // Deontic: rPPG is only permitted for WellfairSelfMonitor / Research purposes.
    let decision = evaluate_processing_act(consent, ProcessingAct::Rppg);
    if decision != PolicyDecision::Permit {
        return Err(CvError::InvalidParameter);
    }
    if out.len() < 3 {
        return Err(CvError::BufferTooSmall);
    }

    let ctx = q_hash(CTX_VISION);
    let conf_meta = ((hr.confidence.clamp(0.0, 1.0) * 255.0) as u64).min(255);
    out[0] = VisionQuin::with_parity(
        media_subject,
        q_hash(P_PROPOSES_HR),
        pack_f32_object(hr.bpm),
        ctx,
        conf_meta,
    );
    out[1] = VisionQuin::with_parity(
        media_subject,
        q_hash(P_HR_CONFIDENCE),
        pack_f32_object(hr.confidence),
        ctx,
        0,
    );
    out[2] = VisionQuin::with_parity(
        media_subject,
        q_hash(P_HR_SNR),
        pack_f32_object(hr.snr),
        ctx,
        0,
    );
    Ok(3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::{BiosenseConsent, BiosensePurpose, HrEstimate};

    #[test]
    fn no_consent_fails() {
        let hr = HrEstimate {
            bpm: 60.0,
            confidence: 0.9,
            snr: 5.0,
        };
        let mut out = [VisionQuin::with_parity(0, 0, 0, 0, 0); 8];
        let c = BiosenseConsent::denied(BiosensePurpose::WellfairSelfMonitor);
        assert!(compile_hr_observation_quins(c, 1, &hr, &mut out).is_err());
    }

    #[test]
    fn security_purpose_forbids_rppg() {
        let hr = HrEstimate {
            bpm: 72.0,
            confidence: 0.8,
            snr: 4.0,
        };
        let mut out = [VisionQuin::with_parity(0, 0, 0, 0, 0); 8];
        let c = BiosenseConsent::grant_security_template(1);
        assert!(compile_hr_observation_quins(c, 0xABC, &hr, &mut out).is_err());
    }

    #[test]
    fn self_monitor_emits_three() {
        let hr = HrEstimate {
            bpm: 72.0,
            confidence: 0.8,
            snr: 4.0,
        };
        let mut out = [VisionQuin::with_parity(0, 0, 0, 0, 0); 8];
        let c = BiosenseConsent::grant_process(BiosensePurpose::WellfairSelfMonitor, 42);
        let n = compile_hr_observation_quins(c, 0xABC, &hr, &mut out).expect("permit");
        assert_eq!(n, 3);
        assert_ne!(out[0].parity, 0);
        assert_ne!(out[0].predicate, 0);
    }

    #[test]
    fn buffer_too_small() {
        let hr = HrEstimate {
            bpm: 60.0,
            confidence: 0.5,
            snr: 2.0,
        };
        let mut out = [VisionQuin::with_parity(0, 0, 0, 0, 0); 2];
        let c = BiosenseConsent::grant_process(BiosensePurpose::Research, 1);
        assert!(matches!(
            compile_hr_observation_quins(c, 1, &hr, &mut out),
            Err(CvError::BufferTooSmall)
        ));
    }
}
