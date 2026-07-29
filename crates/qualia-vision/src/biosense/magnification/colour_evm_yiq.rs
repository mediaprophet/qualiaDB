//! Colour Eulerian magnification in YIQ-class chrominance (I/Q), not raw RGB.
//!
//! Pipeline: RGB→YIQ → temporal band-pass on I and Q → amplify → YIQ→RGB.
//! Default path uses global-mean chrominance (Tier-2 series); optional full
//! per-pixel planes when `band_i`/`band_q` are large enough.

use super::evm_snr_gate::{evm_snr_gate_trace, EvmRefuse, DEFAULT_EVM_MIN_SNR};
use super::temporal_bandpass::{design_bandpass_iir, temporal_bandpass_series, BandpassIir};

/// Parameters for colour EVM (Wu-style chrominance band amplify).
#[derive(Debug, Clone, Copy)]
pub struct ColourEvmParams {
    pub fps: f32,
    pub f_lo_hz: f32,
    pub f_hi_hz: f32,
    /// Chrominance gain (I/Q).
    pub alpha_chroma: f32,
    /// Luma gain (usually 0).
    pub alpha_luma: f32,
    pub min_snr: f32,
    /// If true, gate on global mean-I temporal series SNR and refuse invent.
    pub require_snr: bool,
}

impl Default for ColourEvmParams {
    fn default() -> Self {
        Self {
            fps: 30.0,
            f_lo_hz: 0.7,
            f_hi_hz: 4.0,
            alpha_chroma: 50.0,
            alpha_luma: 0.0,
            min_snr: DEFAULT_EVM_MIN_SNR,
            require_snr: true,
        }
    }
}

#[inline]
fn rgb_to_yiq(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let i = 0.596 * r - 0.274 * g - 0.322 * b;
    let q = 0.211 * r - 0.523 * g + 0.312 * b;
    (y, i, q)
}

#[inline]
fn yiq_to_rgb(y: f32, i: f32, q: f32) -> (f32, f32, f32) {
    let r = y + 0.956 * i + 0.621 * q;
    let g = y - 0.272 * i - 0.647 * q;
    let b = y - 1.106 * i + 1.703 * q;
    (r, g, b)
}

#[inline]
fn clamp_u8(v: f32) -> u8 {
    v.clamp(0.0, 255.0) as u8
}

/// Amplify chrominance temporal band of an RGB frame stack into `out`.
///
/// `frames` / `out`: packed `n_frames * width * height * 3` RGB8.
///
/// Scratch (caller-owned, Tier-2 OK):
/// - `trace_scratch`: ≥ `n_frames`
/// - `band_i`, `band_q`: ≥ `n_frames * px` for pixel path; shorter → mean path
pub fn colour_evm_yiq(
    frames: &[u8],
    n_frames: usize,
    width: u32,
    height: u32,
    params: ColourEvmParams,
    out: &mut [u8],
    trace_scratch: &mut [f32],
    band_i: &mut [f32],
    band_q: &mut [f32],
) -> Result<f32, EvmRefuse> {
    let w = width as usize;
    let h = height as usize;
    let px = w * h;
    let frame_bytes = px * 3;
    if n_frames < 8 || width == 0 || height == 0 {
        return Err(EvmRefuse::InsufficientFrames {
            got: n_frames,
            need: 8,
        });
    }
    if frames.len() < n_frames * frame_bytes || out.len() < n_frames * frame_bytes {
        return Err(EvmRefuse::BufferTooSmall);
    }
    if trace_scratch.len() < n_frames {
        return Err(EvmRefuse::BufferTooSmall);
    }

    let bp = design_bandpass_iir(params.fps, params.f_lo_hz, params.f_hi_hz)?;

    for t in 0..n_frames {
        let base = t * frame_bytes;
        let mut si = 0.0f32;
        for p in 0..px {
            let o = base + p * 3;
            let (_y, i, _q) =
                rgb_to_yiq(frames[o] as f32, frames[o + 1] as f32, frames[o + 2] as f32);
            si += i;
        }
        trace_scratch[t] = si / px as f32;
    }

    let mut snr = 0.0f32;
    if params.require_snr {
        let mut snr_scratch = vec![0.0f32; n_frames];
        snr = evm_snr_gate_trace(
            &trace_scratch[..n_frames],
            params.fps,
            params.f_lo_hz,
            params.f_hi_hz,
            params.min_snr,
            &mut snr_scratch,
        )?;
    }

    let a_c = params.alpha_chroma.clamp(0.0, 200.0);
    let a_y = params.alpha_luma.clamp(0.0, 50.0);
    let use_pixel = band_i.len() >= n_frames * px && band_q.len() >= n_frames * px;

    if use_pixel {
        colour_evm_pixel_path(
            frames,
            n_frames,
            px,
            frame_bytes,
            &bp,
            a_c,
            a_y,
            out,
            band_i,
            band_q,
        )?;
    } else {
        colour_evm_mean_path(
            frames,
            n_frames,
            px,
            frame_bytes,
            &bp,
            a_c,
            trace_scratch,
            out,
        )?;
    }
    Ok(snr)
}

fn colour_evm_mean_path(
    frames: &[u8],
    n_frames: usize,
    px: usize,
    frame_bytes: usize,
    bp: &BandpassIir,
    alpha: f32,
    mean_i: &[f32],
    out: &mut [u8],
) -> Result<(), EvmRefuse> {
    let mut band = vec![0.0f32; n_frames];
    temporal_bandpass_series(&mean_i[..n_frames], bp, &mut band)?;
    let mut mean_q = vec![0.0f32; n_frames];
    for t in 0..n_frames {
        let base = t * frame_bytes;
        let mut sq = 0.0f32;
        for p in 0..px {
            let o = base + p * 3;
            let (_y, _i, q) =
                rgb_to_yiq(frames[o] as f32, frames[o + 1] as f32, frames[o + 2] as f32);
            sq += q;
        }
        mean_q[t] = sq / px as f32;
    }
    let mut band_q = vec![0.0f32; n_frames];
    temporal_bandpass_series(&mean_q, bp, &mut band_q)?;

    for t in 0..n_frames {
        let base = t * frame_bytes;
        let di = alpha * band[t];
        let dq = alpha * band_q[t];
        for p in 0..px {
            let o = base + p * 3;
            let (y, i, q) =
                rgb_to_yiq(frames[o] as f32, frames[o + 1] as f32, frames[o + 2] as f32);
            let (r, g, b) = yiq_to_rgb(y, i + di, q + dq);
            out[o] = clamp_u8(r);
            out[o + 1] = clamp_u8(g);
            out[o + 2] = clamp_u8(b);
        }
    }
    Ok(())
}

fn colour_evm_pixel_path(
    frames: &[u8],
    n_frames: usize,
    px: usize,
    frame_bytes: usize,
    bp: &BandpassIir,
    a_c: f32,
    a_y: f32,
    out: &mut [u8],
    band_i: &mut [f32],
    band_q: &mut [f32],
) -> Result<(), EvmRefuse> {
    let plane_need = n_frames * px;
    for t in 0..n_frames {
        let base = t * frame_bytes;
        let pbase = t * px;
        for p in 0..px {
            let o = base + p * 3;
            let (_y, i, q) =
                rgb_to_yiq(frames[o] as f32, frames[o + 1] as f32, frames[o + 2] as f32);
            band_i[pbase + p] = i;
            band_q[pbase + p] = q;
        }
    }

    let i_src: Vec<f32> = band_i[..plane_need].to_vec();
    let q_src: Vec<f32> = band_q[..plane_need].to_vec();

    for p in 0..px {
        let mut state_lo_i = i_src[p];
        let mut state_hi_i = i_src[p];
        let mut state_lo_q = q_src[p];
        let mut state_hi_q = q_src[p];
        for t in 0..n_frames {
            let idx = t * px + p;
            band_i[idx] = super::temporal_bandpass::bandpass_step(
                i_src[idx],
                bp,
                &mut state_lo_i,
                &mut state_hi_i,
            );
            band_q[idx] = super::temporal_bandpass::bandpass_step(
                q_src[idx],
                bp,
                &mut state_lo_q,
                &mut state_hi_q,
            );
        }
    }

    for t in 0..n_frames {
        let base = t * frame_bytes;
        let pbase = t * px;
        for p in 0..px {
            let o = base + p * 3;
            let (y, i, q) =
                rgb_to_yiq(frames[o] as f32, frames[o + 1] as f32, frames[o + 2] as f32);
            let (r, g, b) = yiq_to_rgb(
                y + a_y * 0.0,
                i + a_c * band_i[pbase + p],
                q + a_c * band_q[pbase + p],
            );
            out[o] = clamp_u8(r);
            out[o + 1] = clamp_u8(g);
            out[o + 2] = clamp_u8(b);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_evm_runs_on_pulsing_chroma() {
        let n = 60usize;
        let w = 4u32;
        let h = 4u32;
        let px = (w * h) as usize;
        let fb = px * 3;
        let fps = 30.0f32;
        let mut frames = vec![128u8; n * fb];
        for t in 0..n {
            let phase = (core::f32::consts::TAU * 1.2 * t as f32 / fps).sin();
            let r = (128.0 + 8.0 * phase).clamp(0.0, 255.0) as u8;
            let g = 120u8;
            let b = (128.0 - 6.0 * phase).clamp(0.0, 255.0) as u8;
            for p in 0..px {
                let o = t * fb + p * 3;
                frames[o] = r;
                frames[o + 1] = g;
                frames[o + 2] = b;
            }
        }
        let mut out = vec![0u8; n * fb];
        let mut trace = vec![0.0f32; n];
        let mut bi = Vec::new();
        let mut bq = Vec::new();
        let params = ColourEvmParams {
            fps,
            f_lo_hz: 0.7,
            f_hi_hz: 4.0,
            alpha_chroma: 20.0,
            alpha_luma: 0.0,
            min_snr: 0.05,
            require_snr: true,
        };
        let snr = colour_evm_yiq(
            &frames, n, w, h, params, &mut out, &mut trace, &mut bi, &mut bq,
        )
        .unwrap();
        assert!(snr >= 0.0);
        let mut diff = 0u32;
        for i in 0..frames.len() {
            diff = diff.saturating_add(frames[i].abs_diff(out[i]) as u32);
        }
        assert!(diff > 0, "expected magnification change");
    }
}
