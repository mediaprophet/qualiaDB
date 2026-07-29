//! Mel-filterbank apply as a certified forge kernel.
//!
//! Projects a row-major power spectrum (`n_frames × n_bins`) onto a triangular mel
//! filterbank (`n_mel × n_bins`), producing mel energies (`n_frames × n_mel`). The
//! operation is the matrix contraction
//! `mel_out[f, m] = Σ_b spectrum[f, b] · mel_fb[m, b]`.
//!
//! Embeds [`shaders/audio_mel.wgsl`](../../../shaders/audio_mel.wgsl) via `include_str!`
//! (single source of truth), grades it against the exact CPU oracle [`mel_apply_cpu`],
//! and runs it on the auxiliary GPU circuit via [`mel_apply_forge`]. The public entry
//! point [`mel_apply`] prefers the GPU when one is present and otherwise uses the CPU
//! floor, so the call is never broken.
//!
//! Circuit placement: the forge kernel runs on the **auxiliary circuit (the iGPU when
//! present)** so the primary/discrete GPU stays free for the LLM. Device selection goes
//! through [`crate::gpu_context::device_registry::try_auxiliary_gpu`], which falls back
//! auxiliary → primary → `None`; on `None` the forge path returns
//! [`ForgeError::GpuUnavailable`] and [`mel_apply`] drops to the CPU floor — i.e. the
//! effective placement chain is auxiliary → primary → CPU.

use crate::wgsl_forge::ForgeError;

/// The mel-apply kernel source (embedded from the canonical `.wgsl`).
pub const MEL_APPLY_WGSL: &str = include_str!("../../shaders/audio_mel.wgsl");
/// Entry-point name of [`MEL_APPLY_WGSL`].
pub const MEL_APPLY_ENTRY: &str = "mel_apply";

/// Exact CPU oracle for the mel-filterbank apply. Mirrors the WGSL scalar-for-scalar:
/// for each output element `(frame, m)`, accumulates `spectrum[frame, b] · mel_fb[m, b]`
/// over `b` in increasing order. `spectrum` is row-major `n_frames × n_bins`, `mel_fb`
/// is row-major `n_mel × n_bins`; the result is row-major `n_frames × n_mel`.
pub fn mel_apply_cpu(
    spectrum: &[f32],
    mel_fb: &[f32],
    n_frames: usize,
    n_bins: usize,
    n_mel: usize,
) -> Vec<f32> {
    let mut out = vec![0.0f32; n_frames * n_mel];
    for frame in 0..n_frames {
        let spec_base = frame * n_bins;
        for m in 0..n_mel {
            let fb_base = m * n_bins;
            let mut acc = 0.0f32;
            for b in 0..n_bins {
                acc += spectrum[spec_base + b] * mel_fb[fb_base + b];
            }
            out[frame * n_mel + m] = acc;
        }
    }
    out
}

/// Run the mel-filterbank apply on the GPU and read back the result. Runs on the
/// **auxiliary GPU circuit (the iGPU when present)** to keep the primary/discrete GPU
/// free for the LLM: the device is taken from
/// [`crate::gpu_context::device_registry::try_auxiliary_gpu`] (falls back
/// auxiliary → primary → `None`) and the compute context is built with
/// [`WgpuComputeContext::from_device`] on that shared device, rather than requesting its
/// own HighPerformance adapter.
///
/// Uploads `spectrum` (binding 0, read), `mel_fb` (binding 1, read), a zeroed output
/// (binding 2, read_write) and `params = [n_frames, n_bins, n_mel]` (binding 3, read),
/// dispatches one invocation per output element, and reads back the `n_frames × n_mel`
/// result. Returns [`ForgeError::GpuUnavailable`] when no GPU circuit is available (so
/// [`mel_apply`] falls back auxiliary → primary → CPU), and [`ForgeError::GpuValidation`]
/// on a shape/length mismatch.
pub fn mel_apply_forge(
    spectrum: &[f32],
    mel_fb: &[f32],
    n_frames: usize,
    n_bins: usize,
    n_mel: usize,
) -> Result<Vec<f32>, ForgeError> {
    use crate::wgsl_forge::execute::{
        BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline,
    };
    use crate::wgsl_forge::Schedule;

    if n_frames == 0 || n_bins == 0 || n_mel == 0 {
        return Err(ForgeError::GpuValidation(format!(
            "mel_apply_forge: dimensions must be non-zero (n_frames={n_frames}, n_bins={n_bins}, n_mel={n_mel})"
        )));
    }
    if spectrum.len() != n_frames * n_bins {
        return Err(ForgeError::GpuValidation(format!(
            "mel_apply_forge: spectrum length {} != n_frames*n_bins {}",
            spectrum.len(),
            n_frames * n_bins
        )));
    }
    if mel_fb.len() != n_mel * n_bins {
        return Err(ForgeError::GpuValidation(format!(
            "mel_apply_forge: mel_fb length {} != n_mel*n_bins {}",
            mel_fb.len(),
            n_mel * n_bins
        )));
    }

    let out_len = n_frames * n_mel;
    let total_floats = spectrum.len() + mel_fb.len() + out_len + 4;
    let capacity = (total_floats * 4).max(4 << 20);
    // Take the device on the auxiliary circuit (iGPU when present), falling back
    // auxiliary → primary → None inside `try_auxiliary_gpu`. On `None` there is no GPU
    // at all: return `GpuUnavailable` so the public `mel_apply` drops to the CPU floor.
    let shared = crate::gpu_context::device_registry::try_auxiliary_gpu().ok_or_else(|| {
        ForgeError::GpuUnavailable(
            "mel_apply_forge: no GPU circuit available (auxiliary→primary both absent)".to_string(),
        )
    })?;
    let mut ctx = WgpuComputeContext::from_device(
        shared.device.clone(),
        shared.queue.clone(),
        &shared.adapter_caps,
        capacity,
    )?;

    let view_spectrum = ctx.allocate_and_write(
        bytemuck::cast_slice(spectrum),
        0,
        0,
        BindingUsage::StorageRead,
    )?;
    let view_fb = ctx.allocate_and_write(
        bytemuck::cast_slice(mel_fb),
        1,
        0,
        BindingUsage::StorageRead,
    )?;
    let zeros = vec![0.0f32; out_len];
    let view_out = ctx.allocate_and_write(
        bytemuck::cast_slice(&zeros),
        2,
        0,
        BindingUsage::StorageReadWrite,
    )?;
    let params = [n_frames as f32, n_bins as f32, n_mel as f32];
    let view_params = ctx.allocate_and_write(
        bytemuck::cast_slice(&params),
        3,
        0,
        BindingUsage::StorageRead,
    )?;

    let buffers = vec![view_spectrum, view_fb, view_out, view_params];
    let pipeline = WgpuPipeline::compile(&ctx, MEL_APPLY_WGSL, MEL_APPLY_ENTRY)?;
    let schedule = Schedule {
        workgroup_size: 64,
        ..Default::default()
    };
    pipeline.dispatch(&buffers, &schedule, out_len)?;
    let mut out = ctx.read_buffer_f32(&view_out)?;
    out.truncate(out_len);
    Ok(out)
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
    use crate::wgsl_forge::validate::validate_wgsl;

    /// The mel-apply kernel must naga-validate under naga 30 and expose the `mel_apply`
    /// entry point. Runs without a GPU — the always-on proof the shader is valid WGSL.
    #[test]
    fn mel_apply_wgsl_validates() {
        let report = validate_wgsl(MEL_APPLY_WGSL).expect("mel-apply WGSL must naga-validate");
        assert!(
            report.entry_points.iter().any(|e| e == MEL_APPLY_ENTRY),
            "validated module must expose {MEL_APPLY_ENTRY}; got {:?}",
            report.entry_points
        );
    }

    /// Hand-computed small case: n_frames=2, n_bins=4, n_mel=2 with a known filterbank.
    /// Two triangular-ish bands, each covering two of the four bins.
    #[test]
    fn mel_apply_cpu_matches_reference() {
        // spectrum: 2 frames × 4 bins.
        //   frame 0: [1, 2, 3, 4]
        //   frame 1: [5, 6, 7, 8]
        let spectrum = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        // mel_fb: 2 mel bands × 4 bins.
        //   band 0: [1, 1, 0, 0]  (low bins)
        //   band 1: [0, 0, 1, 1]  (high bins)
        let mel_fb = vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0];

        let out = mel_apply_cpu(&spectrum, &mel_fb, 2, 4, 2);

        // frame 0: band0 = 1+2 = 3 ; band1 = 3+4 = 7
        // frame 1: band0 = 5+6 = 11; band1 = 7+8 = 15
        assert_eq!(out, vec![3.0, 7.0, 11.0, 15.0]);
    }

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
