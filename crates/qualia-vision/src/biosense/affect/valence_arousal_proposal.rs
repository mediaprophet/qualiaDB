//! Affective **proposal** from simple image stats — never a silent fact.
//! Excellence: always emit uncertainty + non-claim flags.

use crate::biosense::consent::BiosenseConsent;
use crate::cv::buffer::RgbView;
use crate::cv::error::CvError;

#[derive(Debug, Clone, Copy)]
pub struct AffectProposal {
    /// -1..1
    pub valence: f32,
    /// 0..1
    pub arousal: f32,
    pub uncertainty: f32,
    /// Machine flag: not diagnosis / not courtroom truth.
    pub is_proposal_only: bool,
    pub method: &'static str,
}

/// Heuristic colour/brightness proxy — **low assurance**. Consent required.
pub fn valence_arousal_proposal(
    consent: BiosenseConsent,
    src: RgbView<'_>,
) -> Result<AffectProposal, CvError> {
    if !consent.may_process() {
        return Err(CvError::InvalidParameter);
    }
    let mut r = 0.0f32;
    let mut g = 0.0f32;
    let mut b = 0.0f32;
    let mut n = 0.0f32;
    for y in 0..src.height {
        for x in 0..src.width {
            let (rr, gg, bb) = src.pixel(x, y);
            r += rr as f32;
            g += gg as f32;
            b += bb as f32;
            n += 1.0;
        }
    }
    n = n.max(1.0);
    r /= n;
    g /= n;
    b /= n;
    let bright = (r + g + b) / (3.0 * 255.0);
    let warm = (r - b) / 255.0;
    Ok(AffectProposal {
        valence: warm.clamp(-1.0, 1.0) * 0.3,
        arousal: bright.clamp(0.0, 1.0) * 0.4,
        uncertainty: 0.85,
        is_proposal_only: true,
        method: "heuristic_rgb_proxy_v1_not_clinical",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::consent::{BiosenseConsent, BiosensePurpose};
    #[test]
    fn always_proposal() {
        let rgb = [200u8, 100, 50, 200, 100, 50];
        let v = RgbView::new(2, 1, 6, &rgb).unwrap();
        let c = BiosenseConsent::grant_process(BiosensePurpose::Research, 1);
        let p = valence_arousal_proposal(c, v).unwrap();
        assert!(p.is_proposal_only);
        assert!(p.uncertainty > 0.5);
    }
}
