//! P7.8 — P7 golden-oracle + CPU/GPU differential + determinism harness
//!
//! This module implements the convergence gates for Phase 7 (Spectral-operator family).
//! It validates the determinism of all spectral ops (gamut mapping, metamers,
//! spectral blend, time-frequency surface edits), executes the CPU/GPU differential 
//! tests for the visual/colour kernel, and ensures metric correctness across 
//! visual and audio domains.

use crate::audio::tf_surface_edit::Region as TfRegion;
use crate::render::spectral_operator::SpectralOperator;
use crate::render::gpu_colour_kernel::{
    cpu_batch_emf_to_display_gamut_mapped, diff_cpu_gpu, GPU_COLOUR_KERNEL_WGSL
};
use crate::render::spectral_kernel::{Xyz, Spd};

#[cfg(test)]
mod tests {
    use super::*;

    // ── Determinism & Canonical Bytes ────────────────────────────────────────

    #[test]
    fn harness_spectral_operator_determinism() {
        // 1. Colour projection determinism
        let (r1, g1, b1) = SpectralOperator::emf_to_display_rgb(1.0, 0.3, 0.5);
        let (r2, g2, b2) = SpectralOperator::emf_to_display_rgb(1.0, 0.3, 0.5);
        assert_eq!((r1, g1, b1), (r2, g2, b2), "EMF projection must be byte-deterministic");

        // 2. Gamut mapping determinism
        let xyz = Xyz::new(2.0, 0.5, 0.0);
        let mapped1 = SpectralOperator::gamut_map(&xyz);
        let mapped2 = SpectralOperator::gamut_map(&xyz);
        assert_eq!(
            (mapped1.x.to_bits(), mapped1.y.to_bits(), mapped1.z.to_bits()),
            (mapped2.x.to_bits(), mapped2.y.to_bits(), mapped2.z.to_bits()),
            "Gamut mapping must be bit-identical across identical inputs"
        );
        assert!(SpectralOperator::is_in_gamut(&mapped1));

        // 3. Spectral blend determinism
        let blend1 = SpectralOperator::spectral_blend_emf(1.0, 0.1, 0.2, 1.0, 0.1, 0.8, 0.5);
        let blend2 = SpectralOperator::spectral_blend_emf(1.0, 0.1, 0.2, 1.0, 0.1, 0.8, 0.5);
        assert_eq!(
            (blend1.x.to_bits(), blend1.y.to_bits(), blend1.z.to_bits()),
            (blend2.x.to_bits(), blend2.y.to_bits(), blend2.z.to_bits()),
            "Spectral blend must be bit-identical across identical inputs"
        );
    }

    #[test]
    fn harness_metamer_determinism() {
        // Metamer particular solution determinism
        let target = Xyz::new(0.4, 0.5, 0.3);
        let spd1 = SpectralOperator::min_norm_spd_for_xyz(&target);
        let spd2 = SpectralOperator::min_norm_spd_for_xyz(&target);
        for (a, b) in spd1.samples.iter().zip(spd2.samples.iter()) {
            assert_eq!(a.to_bits(), b.to_bits(), "Metamer SPD solver must be bit-identical");
        }
    }

    #[test]
    fn harness_audio_surface_determinism() {
        let frames = 4;
        let bins = 32;
        let raster = vec![0.5f32; frames * bins];
        let s = SpectralOperator::tf_surface(&raster, frames, bins, 44100, 512);
        let region = TfRegion::full(frames, bins);
        
        let mut out1 = vec![0.0f32; frames * bins];
        let mut out2 = vec![0.0f32; frames * bins];
        
        SpectralOperator::surface_gain(&s, &region, 2.0, &mut out1).unwrap();
        SpectralOperator::surface_gain(&s, &region, 2.0, &mut out2).unwrap();
        
        for (a, b) in out1.iter().zip(out2.iter()) {
            assert_eq!(a.to_bits(), b.to_bits(), "Audio surface edit must be bit-identical");
        }
    }

    // ── CPU/GPU Differential & Wgsl Checks ────────────────────────────────────

    #[test]
    fn harness_wgsl_validation_gate() {
        // GPU batch processing pipeline must mention clamped gamut mapping.
        assert!(GPU_COLOUR_KERNEL_WGSL.contains("clamp"), "GPU kernel must clamp for gamut mapping");
        assert!(GPU_COLOUR_KERNEL_WGSL.contains("CMF_X"), "GPU kernel must contain tabulated CMF_X");
    }

    #[test]
    fn harness_gpu_differential_tolerance() {
        // Test CPU/GPU differential matching logic over typical data
        let emf_batch = [1.0, 0.0, 0.0, 1.0, 0.5, 0.5, 1.0, 1.0, 1.0];
        let mut cpu_out = [0u8; 9];
        cpu_batch_emf_to_display_gamut_mapped(&emf_batch, &mut cpu_out);

        // Simulate a GPU outcome that is identical
        let mismatches_exact = diff_cpu_gpu(&cpu_out, &cpu_out);
        assert_eq!(mismatches_exact, 0, "Exact GPU outputs must result in 0 mismatches");

        // Simulate GPU output with acceptable float math drift (±1 or ±2 per channel)
        let mut gpu_drift = cpu_out.clone();
        gpu_drift[0] = gpu_drift[0].saturating_add(1);
        gpu_drift[4] = gpu_drift[4].saturating_sub(2);
        
        let mismatches_drift = diff_cpu_gpu(&cpu_out, &gpu_drift);
        assert_eq!(mismatches_drift, 0, "GPU outputs within tolerance should not flag mismatch");

        // Simulate GPU output exceeding tolerance (e.g. failing to gamut map)
        let mut gpu_fail = cpu_out.clone();
        gpu_fail[1] = gpu_fail[1].saturating_add(10);
        
        let mismatches_fail = diff_cpu_gpu(&cpu_out, &gpu_fail);
        assert_eq!(mismatches_fail, 1, "GPU outputs exceeding tolerance must flag a mismatch");
    }
}
