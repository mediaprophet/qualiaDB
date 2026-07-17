//! Consent-gated EVM entry points — fail closed without `BiosenseConsent::may_process`.

use super::colour_evm_yiq::ColourEvmParams;
use super::eulerian_color_magnify::eulerian_color_magnify_ex;
use super::eulerian_motion_magnify::{eulerian_motion_magnify_ex, MotionEvmParams};
use super::evm_snr_gate::EvmRefuse;
use crate::biosense::consent::BiosenseConsent;

/// Colour EVM only when consent allows processing.
pub fn eulerian_color_magnify_consented(
    consent: BiosenseConsent,
    frames: &[u8],
    n_frames: usize,
    width: u32,
    height: u32,
    params: ColourEvmParams,
    out: &mut [u8],
) -> Result<f32, EvmRefuse> {
    if !consent.may_process() {
        return Err(EvmRefuse::ConsentDenied);
    }
    eulerian_color_magnify_ex(frames, n_frames, width, height, params, out)
}

/// Motion EVM only when consent allows processing.
pub fn eulerian_motion_magnify_consented(
    consent: BiosenseConsent,
    frames: &[u8],
    n_frames: usize,
    width: u32,
    height: u32,
    params: MotionEvmParams,
    out: &mut [u8],
) -> Result<f32, EvmRefuse> {
    if !consent.may_process() {
        return Err(EvmRefuse::ConsentDenied);
    }
    eulerian_motion_magnify_ex(frames, n_frames, width, height, params, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::consent::{BiosenseConsent, BiosensePurpose};

    #[test]
    fn denies_without_consent() {
        let n = 16;
        let px = 4;
        let f = vec![10u8; n * px];
        let mut o = vec![0u8; n * px];
        let c = BiosenseConsent::denied(BiosensePurpose::Research);
        let r = eulerian_motion_magnify_consented(
            c,
            &f,
            n,
            2,
            2,
            MotionEvmParams {
                require_snr: false,
                ..Default::default()
            },
            &mut o,
        );
        assert_eq!(r, Err(EvmRefuse::ConsentDenied));
    }

    #[test]
    fn allows_with_consent() {
        let n = 16;
        let px = 64;
        let f = vec![80u8; n * px];
        let mut o = vec![0u8; n * px];
        let c = BiosenseConsent::grant_process(BiosensePurpose::Research, 1);
        eulerian_motion_magnify_consented(
            c,
            &f,
            n,
            8,
            8,
            MotionEvmParams {
                require_snr: false,
                levels: 2,
                ..Default::default()
            },
            &mut o,
        )
        .unwrap();
    }
}
