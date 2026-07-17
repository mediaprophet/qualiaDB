//! Affect proposals from MediaPipe-class blendshape proxies (no AffectNet weights).
//! Always proposal-only with high uncertainty.

use crate::biosense::consent::BiosenseConsent;
use crate::biosense::affect::valence_arousal_proposal::AffectProposal;
use crate::cv::error::CvError;

/// Minimal blendshape subset (filled by mesh adapter when MediaPipe is wired).
#[derive(Debug, Clone, Copy, Default)]
pub struct BlendshapeProxy {
    pub mouth_smile_left: f32,
    pub mouth_smile_right: f32,
    pub brow_inner_up: f32,
    pub brow_down: f32,
    pub jaw_open: f32,
    pub eye_wide: f32,
}

/// Map blendshapes → valence/arousal **proposals** (heuristic Path A in commercial model pack).
pub fn blendshape_affect_proposal(
    consent: BiosenseConsent,
    bs: BlendshapeProxy,
) -> Result<AffectProposal, CvError> {
    if !consent.may_process() {
        return Err(CvError::InvalidParameter);
    }
    let smile = 0.5 * (bs.mouth_smile_left + bs.mouth_smile_right);
    let valence = (smile - bs.brow_down * 0.5).clamp(-1.0, 1.0) * 0.5;
    let arousal = (bs.jaw_open * 0.3 + bs.eye_wide * 0.3 + bs.brow_inner_up * 0.2).clamp(0.0, 1.0);
    Ok(AffectProposal {
        valence,
        arousal,
        uncertainty: 0.75,
        is_proposal_only: true,
        method: "mediapipe_blendshape_heuristic_v1_not_clinical",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::consent::{BiosenseConsent, BiosensePurpose};
    #[test]
    fn smile_raises_valence() {
        let c = BiosenseConsent::grant_process(BiosensePurpose::Research, 1);
        let p = blendshape_affect_proposal(
            c,
            BlendshapeProxy {
                mouth_smile_left: 0.9,
                mouth_smile_right: 0.9,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(p.valence > 0.0);
        assert!(p.is_proposal_only);
    }
}
