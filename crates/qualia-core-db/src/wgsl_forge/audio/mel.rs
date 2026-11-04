//! Product mel placement: auxiliary GPU, then primary GPU, then the CPU oracle.
use crate::wgsl_forge::ForgeError;
pub use etalon_wgsl_forge::numerical::audio::mel::{mel_apply_cpu, mel_apply_into, MelShape, PreparedMel, MEL_APPLY_ENTRY, MEL_APPLY_WGSL};

pub fn mel_apply_forge(spectrum: &[f32], mel_fb: &[f32], n_frames: usize,
                       n_bins: usize, n_mel: usize) -> Result<Vec<f32>, ForgeError> {
    use crate::wgsl_forge::execute::WgpuComputeContext;
    use etalon_wgsl_forge::numerical::audio::mel::{mel_apply_with_context, mel_gpu_capacity};
    let capacity = mel_gpu_capacity(spectrum, mel_fb, MelShape { n_frames, n_bins, n_mel })?;
    let shared = crate::gpu_context::device_registry::try_auxiliary_gpu().ok_or_else(|| {
        ForgeError::GpuUnavailable(
            "mel_apply_forge: no GPU circuit available (auxiliary→primary both absent)".to_string(),
        )
    })?;
    let mut ctx = WgpuComputeContext::from_device(
        shared.device.clone(), shared.queue.clone(), &shared.adapter_caps, capacity,
    )?;
    mel_apply_with_context(&mut ctx, spectrum, mel_fb, n_frames, n_bins, n_mel)
}

/// Public entry point: run the mel-filterbank apply on the best path available on this
/// machine. If a wgpu adapter is present ([`caps().wgpu`]), try [`mel_apply_forge`] and
/// return its result on success; otherwise (no adapter, or a runtime GPU failure) fall
/// back to the exact CPU oracle [`mel_apply_cpu`], so the call is never broken.
///
/// [`caps().wgpu`]: crate::wgsl_forge::dispatch::caps
pub fn mel_apply(
    spectrum: &[f32],
    mel_fb: &[f32],
    n_frames: usize,
    n_bins: usize,
    n_mel: usize,
) -> Vec<f32> {
    if crate::wgsl_forge::dispatch::caps().wgpu {
        if let Ok(out) = mel_apply_forge(spectrum, mel_fb, n_frames, n_bins, n_mel) {
            return out;
        }
        // Forge path was eligible but failed at runtime — fall through to the CPU floor
        // rather than propagating, so the call is never broken.
    }
    mel_apply_cpu(spectrum, mel_fb, n_frames, n_bins, n_mel)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The public entry point must agree with the CPU oracle. Works on GPU-less boxes
    /// via the fallback (always runs) and on GPU boxes via the forge path.
    #[test]
    fn mel_apply_public_matches_cpu() {
        let (n_frames, n_bins, n_mel) = (3usize, 5usize, 4usize);
        let spectrum: Vec<f32> = (0..n_frames * n_bins)
            .map(|k| (k as f32) * 0.5 - 3.0)
            .collect();
        // A deterministic overlapping-triangle-ish filterbank.
        let mut mel_fb = vec![0.0f32; n_mel * n_bins];
        for m in 0..n_mel {
            for b in 0..n_bins {
                mel_fb[m * n_bins + b] = ((m + b) as f32 % 3.0) * 0.25;
            }
        }
        let public = mel_apply(&spectrum, &mel_fb, n_frames, n_bins, n_mel);
        let oracle = mel_apply_cpu(&spectrum, &mel_fb, n_frames, n_bins, n_mel);
        assert_eq!(public.len(), oracle.len());
        for (p, o) in public.iter().zip(oracle.iter()) {
            let tol = 1e-3 * o.abs().max(1.0);
            assert!((p - o).abs() <= tol, "public/CPU mismatch: {p} vs {o}");
        }
    }

    /// GPU certify: the kernel on a real adapter must match the CPU oracle within f32
    /// tolerance over a deterministic multi-frame scene. Skips cleanly with no device.
    #[test]
    #[serial_test::serial(gpu)]
    fn mel_gpu_matches_oracle() {
        if !crate::wgsl_forge::test_gpu_available() {
            return;
        }
        // Report which circuit/adapter the forge path resolved to (cheap, non-asserting:
        // CI may expose only one GPU, so this is diagnostic only).
        if let Some(shared) = crate::gpu_context::device_registry::try_auxiliary_gpu() {
            let caps = &shared.adapter_caps;
            eprintln!(
                "mel_gpu_matches_oracle: forge on adapter '{}' ({:?}, {:?})",
                caps.name, caps.device_type, caps.backend
            );
        }
        let (n_frames, n_bins, n_mel) = (17usize, 33usize, 12usize);
        let spectrum: Vec<f32> = (0..n_frames * n_bins)
            .map(|k| ((k as f32) * 0.017).sin().abs() + 0.001)
            .collect();
        // Deterministic triangular filterbank: band m peaks around bin proportional to m.
        let mut mel_fb = vec![0.0f32; n_mel * n_bins];
        for m in 0..n_mel {
            let centre = (m as f32 + 1.0) * (n_bins as f32) / (n_mel as f32 + 1.0);
            for b in 0..n_bins {
                let w = 1.0 - ((b as f32 - centre).abs() / 3.0);
                mel_fb[m * n_bins + b] = w.max(0.0);
            }
        }
        let expected = mel_apply_cpu(&spectrum, &mel_fb, n_frames, n_bins, n_mel);
        let gpu = mel_apply_forge(&spectrum, &mel_fb, n_frames, n_bins, n_mel)
            .expect("mel_apply_forge on an available device");
        assert_eq!(gpu.len(), expected.len());
        for (g, e) in gpu.iter().zip(expected.iter()) {
            let tol = 1e-3 * e.abs().max(1.0);
            assert!((g - e).abs() <= tol, "GPU/CPU mismatch: {g} vs {e}");
        }
    }
}
