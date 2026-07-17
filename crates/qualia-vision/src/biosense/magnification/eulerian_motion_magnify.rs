//! Eulerian motion magnification — multi-scale spatial (Laplacian) × temporal band × α.
//!
//! Wu et al. style: Laplacian pyramid per frame, IIR band-pass along time per level,
//! amplify, reconstruct, add residual to original gray sequence.

use super::evm_snr_gate::{evm_snr_gate_trace, EvmRefuse, DEFAULT_EVM_MIN_SNR};
use super::gaussian_pyramid_build::{
    gaussian_pyramid_build, gaussian_pyramid_scratch_elems, PyramidLevelMeta, MAX_PYRAMID_LEVELS,
};
use super::laplacian_pyramid_build::laplacian_pyramid_build;
use super::pyramid_reconstruct::pyramid_reconstruct;
use super::temporal_bandpass::{bandpass_step, design_bandpass_iir};
use crate::cv::error::CvError;

/// Motion EVM parameters.
#[derive(Debug, Clone, Copy)]
pub struct MotionEvmParams {
    pub fps: f32,
    pub f_lo_hz: f32,
    pub f_hi_hz: f32,
    pub alpha: f32,
    /// Pyramid depth (1..=MAX_PYRAMID_LEVELS).
    pub levels: usize,
    pub min_snr: f32,
    pub require_snr: bool,
}

impl Default for MotionEvmParams {
    fn default() -> Self {
        Self {
            fps: 30.0,
            f_lo_hz: 0.4,
            f_hi_hz: 3.0,
            alpha: 20.0,
            levels: 4,
            min_snr: DEFAULT_EVM_MIN_SNR,
            require_snr: true,
        }
    }
}

/// Compatibility API: gray packed `n*w*h`, default band @ 30 fps, SNR gate off.
pub fn eulerian_motion_magnify(
    frames: &[u8],
    n_frames: usize,
    width: u32,
    height: u32,
    alpha: f32,
    out: &mut [u8],
) -> Result<(), CvError> {
    eulerian_motion_magnify_ex(
        frames,
        n_frames,
        width,
        height,
        MotionEvmParams {
            alpha,
            require_snr: false,
            ..MotionEvmParams::default()
        },
        out,
    )
    .map(|_| ())
    .map_err(|e| match e {
        EvmRefuse::BufferTooSmall => CvError::BufferTooSmall,
        EvmRefuse::EmptyInput => CvError::EmptyInput,
        _ => CvError::InvalidParameter,
    })
}

/// Multi-scale motion EVM. Returns band SNR of global mean gray (0 if gate off).
pub fn eulerian_motion_magnify_ex(
    frames: &[u8],
    n_frames: usize,
    width: u32,
    height: u32,
    params: MotionEvmParams,
    out: &mut [u8],
) -> Result<f32, EvmRefuse> {
    let w = width as usize;
    let h = height as usize;
    let px = w * h;
    if n_frames < 8 || width < 2 || height < 2 {
        return Err(EvmRefuse::InsufficientFrames {
            got: n_frames,
            need: 8,
        });
    }
    if frames.len() < n_frames * px || out.len() < n_frames * px {
        return Err(EvmRefuse::BufferTooSmall);
    }

    let levels = params.levels.clamp(1, MAX_PYRAMID_LEVELS);
    let bp = design_bandpass_iir(params.fps, params.f_lo_hz, params.f_hi_hz)?;
    let gain = params.alpha.clamp(0.0, 100.0);

    // SNR on global mean
    let mut mean_series = vec![0.0f32; n_frames];
    for t in 0..n_frames {
        let base = t * px;
        let mut s = 0.0f32;
        for p in 0..px {
            s += frames[base + p] as f32;
        }
        mean_series[t] = s / px as f32;
    }
    let mut snr = 0.0f32;
    if params.require_snr {
        let mut scratch = vec![0.0f32; n_frames];
        snr = evm_snr_gate_trace(
            &mean_series,
            params.fps,
            params.f_lo_hz,
            params.f_hi_hz,
            params.min_snr,
            &mut scratch,
        )?;
    }

    // Tier-2 cold: pyramid scratch for one frame + per-level temporal state
    let pyr_elems = gaussian_pyramid_scratch_elems(width, height, levels);
    let mut g_pack = vec![0.0f32; pyr_elems];
    let mut lap_pack = vec![0.0f32; pyr_elems];
    let mut expand = vec![0.0f32; px];
    let mut recon = vec![0.0f32; px];
    let mut work = vec![0.0f32; px];
    let mut src_f = vec![0.0f32; px];
    let mut meta = [PyramidLevelMeta::default(); MAX_PYRAMID_LEVELS];

    // Temporal IIR state per pyramid element (lo/hi)
    let mut state_lo = vec![0.0f32; pyr_elems];
    let mut state_hi = vec![0.0f32; pyr_elems];
    let mut state_init = false;

    // Accumulate amplified band into recon buffer history — process frame-by-frame:
    // For each frame: build Lap pyramid, band-pass step each coeff, reconstruct band image,
    // out = original + alpha * band.
    for t in 0..n_frames {
        let base = t * px;
        for p in 0..px {
            src_f[p] = frames[base + p] as f32;
        }
        let n_lev = gaussian_pyramid_build(&src_f, width, height, levels, &mut g_pack, &mut meta)?;
        laplacian_pyramid_build(&g_pack, &meta, n_lev, &mut lap_pack, &mut expand)?;

        if !state_init {
            state_lo[..pyr_elems].copy_from_slice(&lap_pack[..pyr_elems]);
            state_hi[..pyr_elems].copy_from_slice(&lap_pack[..pyr_elems]);
            state_init = true;
        }

        // In-place band-pass step on laplacian coefficients
        let total: usize = (0..n_lev).map(|i| meta[i].len).sum();
        for k in 0..total {
            let x = lap_pack[k];
            lap_pack[k] = bandpass_step(x, &bp, &mut state_lo[k], &mut state_hi[k]);
        }

        pyramid_reconstruct(&lap_pack, &meta, n_lev, &mut recon, &mut work)?;

        for p in 0..px {
            let v = src_f[p] + gain * recon[p];
            out[base + p] = v.clamp(0.0, 255.0) as u8;
        }
    }

    Ok(snr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_runs_multiscale() {
        let n = 40usize;
        let w = 16u32;
        let h = 16u32;
        let px = (w * h) as usize;
        let mut frames = vec![80u8; n * px];
        // Vertical edge that shifts slightly over time (sub-pixel motion proxy)
        for t in 0..n {
            let shift = ((core::f32::consts::TAU * 0.8 * t as f32 / 30.0).sin() * 1.5) as i32;
            for y in 0..h as usize {
                for x in 0..w as usize {
                    let edge = if (x as i32) > 8 + shift { 160u8 } else { 40u8 };
                    frames[t * px + y * w as usize + x] = edge;
                }
            }
        }
        let mut out = vec![0u8; n * px];
        eulerian_motion_magnify_ex(
            &frames,
            n,
            w,
            h,
            MotionEvmParams {
                require_snr: false,
                alpha: 10.0,
                levels: 3,
                ..Default::default()
            },
            &mut out,
        )
        .unwrap();
        // Mid frames should not be identical to input everywhere
        let mid = n / 2;
        let mut same = 0usize;
        for p in 0..px {
            if frames[mid * px + p] == out[mid * px + p] {
                same += 1;
            }
        }
        assert!(same < px, "expected some pixels to change under motion mag");
    }

    #[test]
    fn legacy_api_ok() {
        let n = 16;
        let w = 8u32;
        let h = 8u32;
        let px = 64;
        let f = vec![100u8; n * px];
        let mut o = vec![0u8; n * px];
        eulerian_motion_magnify(&f, n, w, h, 5.0, &mut o).unwrap();
    }
}
