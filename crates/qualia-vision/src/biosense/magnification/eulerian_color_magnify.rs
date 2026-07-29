//! Eulerian colour magnification — thin public API over YIQ-class excellence path.
//!
//! Prefer [`super::colour_evm_yiq::colour_evm_yiq`] when you need explicit fps/band/SNR.

use super::colour_evm_yiq::{colour_evm_yiq, ColourEvmParams};
use super::evm_snr_gate::EvmRefuse;
use crate::cv::error::CvError;

/// Amplify temporal chrominance band of packed RGB frames into `out`.
///
/// Compatibility wrapper: uses default HR band (0.7–4 Hz @ 30 fps), SNR gate on,
/// and mean-chroma path (no large plane scratch). `alpha` maps to chroma gain.
///
/// For full control (fps, band, pixel path, SNR threshold) call `colour_evm_yiq`.
pub fn eulerian_color_magnify(
    frames: &[u8],
    n_frames: usize,
    width: u32,
    height: u32,
    alpha: f32,
    out: &mut [u8],
) -> Result<(), CvError> {
    eulerian_color_magnify_ex(
        frames,
        n_frames,
        width,
        height,
        ColourEvmParams {
            alpha_chroma: alpha,
            require_snr: false, // legacy API must not surprise callers with abstain
            ..ColourEvmParams::default()
        },
        out,
    )
    .map(|_| ())
    .map_err(evm_to_cv)
}

/// Excellence entry: returns measured band SNR (0 if SNR gate disabled).
pub fn eulerian_color_magnify_ex(
    frames: &[u8],
    n_frames: usize,
    width: u32,
    height: u32,
    params: ColourEvmParams,
    out: &mut [u8],
) -> Result<f32, EvmRefuse> {
    let mut trace = vec![0.0f32; n_frames.max(1)];
    let mut bi = Vec::new();
    let mut bq = Vec::new();
    colour_evm_yiq(
        frames, n_frames, width, height, params, out, &mut trace, &mut bi, &mut bq,
    )
}

fn evm_to_cv(e: EvmRefuse) -> CvError {
    match e {
        EvmRefuse::BufferTooSmall => CvError::BufferTooSmall,
        EvmRefuse::EmptyInput => CvError::EmptyInput,
        EvmRefuse::InvalidParameter
        | EvmRefuse::ConsentDenied
        | EvmRefuse::SnrTooLow { .. }
        | EvmRefuse::InsufficientFrames { .. } => CvError::InvalidParameter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs() {
        let n = 16;
        let px = 4 * 3;
        let mut f = vec![100u8; n * px];
        for i in 0..n {
            let phase = (core::f32::consts::TAU * 1.0 * i as f32 / 30.0).sin();
            f[i * px] = (100.0 + 5.0 * phase) as u8;
            f[i * px + 2] = (100.0 - 4.0 * phase) as u8;
        }
        let mut o = vec![0u8; n * px];
        eulerian_color_magnify(&f, n, 2, 2, 10.0, &mut o).unwrap();
    }

    #[test]
    fn ex_with_snr_gate() {
        let n = 90;
        let w = 2u32;
        let h = 2u32;
        let fb = 4 * 3;
        let mut f = vec![128u8; n * fb];
        for t in 0..n {
            let phase = (core::f32::consts::TAU * 1.5 * t as f32 / 30.0).sin();
            for p in 0..4 {
                let o = t * fb + p * 3;
                f[o] = (128.0 + 10.0 * phase) as u8;
                f[o + 2] = (128.0 - 8.0 * phase) as u8;
            }
        }
        let mut o = vec![0u8; n * fb];
        let snr = eulerian_color_magnify_ex(
            &f,
            n,
            w,
            h,
            ColourEvmParams {
                require_snr: true,
                min_snr: 0.05,
                alpha_chroma: 15.0,
                ..Default::default()
            },
            &mut o,
        )
        .unwrap();
        assert!(snr > 0.0);
    }
}
