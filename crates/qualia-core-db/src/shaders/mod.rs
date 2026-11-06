//! WGSL shader sources embedded in the Qualia WASM binary and canonical Etalon shader registry (`RM-03`).
//!
//! - **Root** (`*.wgsl`) — U0/U1 compute (LLM, tensor query, physics/HUD)
//! - **`wasm/`** — copies of WASM LLM inference shaders (fused attention/FFN/transformer, GEMV, dequant, top-K, ternary, LoRA)
//! - **`viewport/`** — U2 display (retired `RM-03` (`R`) and re-exported directly from [`etalon_render::shaders`])
//!
//! Canonical `etalon_nquin` and `etalon_render` shader constants are exposed here with compile-time
//! and unit-tested byte-for-byte parity guarantees.

pub mod viewport;

pub const PGA_SCLERP_EVAL_WGSL: &str = include_str!("pga_sclerp_eval.wgsl");
pub const SPRING_DAMPER_GRID_WGSL: &str = include_str!("spring_damper_grid.wgsl");
pub const HUD_GLASS_BLUR_WGSL: &str = include_str!("hud_glass_blur.wgsl");
pub const WAVE_INTERFERENCE_WGSL: &str = include_str!("wave_interference.wgsl");

/// Canonical 48-byte `NQuinWire` warp-scan sieve compute shader from [`etalon_nquin`].
pub const NQUIN_WARP_SCAN_WGSL: &str = etalon_nquin::NQUIN_WARP_SCAN_WGSL;

/// Legacy 27-word bitmask sieve compute shader retained for `platform::npu_ffi` compatibility.
pub const SIEVE_WGSL: &str = include_str!("sieve.wgsl");

/// Canonical composite Holographic 3D semantic graph shader (`spectral.wgsl` + `holographic.wgsl`) from [`etalon_render::shaders`].
pub const HOLOGRAPHIC_WGSL: &str = viewport::HOLOGRAPHIC_WGSL;

/// Canonical standalone volumetric Holographic shader from [`etalon_render::volumetric`].
#[cfg(feature = "gpu-runtime")]
pub const HOLOGRAPHIC_STANDALONE_WGSL: &str = etalon_render::HOLOGRAPHIC_WGSL;
#[cfg(not(feature = "gpu-runtime"))]
pub const HOLOGRAPHIC_STANDALONE_WGSL: &str = viewport::HOLOGRAPHIC_WGSL;

/// Canonical 4D EMF volumetric slice shader from [`etalon_render::shaders`].
pub const EMF_VOLUMETRIC_WGSL: &str = viewport::EMF_VOLUMETRIC_WGSL;

/// Canonical 64-byte Stokes v2 EMF volumetric shader from [`etalon_render::emf`].
#[cfg(feature = "gpu-runtime")]
pub const EMF_VOLUMETRIC_STOKES_WGSL: &str = etalon_render::EMF_VOLUMETRIC_WGSL;
#[cfg(not(feature = "gpu-runtime"))]
pub const EMF_VOLUMETRIC_STOKES_WGSL: &str = viewport::EMF_VOLUMETRIC_WGSL;

/// Canonical GEMM substrate compute shader.
pub const GEMM_SUBSTRATE_WGSL: &str = include_str!("gemm_substrate.wgsl");

/// Canonical top-K reduction compute shader.
pub const TOPK_REDUCTION_WGSL: &str = include_str!("topk_reduction.wgsl");

/// Canonical graph sieve field compute shader.
pub const GRAPH_SIEVE_FIELD_WGSL: &str = include_str!("graph_sieve_field.wgsl");

/// Canonical graph radix histogram compute shader.
pub const GRAPH_RADIX_HIST_WGSL: &str = include_str!("graph_radix_hist.wgsl");

/// Canonical graph radix scatter compute shader.
pub const GRAPH_RADIX_SCATTER_WGSL: &str = include_str!("graph_radix_scatter.wgsl");

/// Return the canonical [`etalon_nquin::NQUIN_WARP_SCAN_WGSL`] source.
#[inline]
pub const fn canonical_nquin_warp_scan_wgsl() -> &'static str {
    NQUIN_WARP_SCAN_WGSL
}

/// Return the canonical [`etalon_render::shaders::HOLOGRAPHIC_WGSL`] source.
#[inline]
pub const fn canonical_holographic_wgsl() -> &'static str {
    HOLOGRAPHIC_WGSL
}

/// Return the canonical [`etalon_render::shaders::EMF_VOLUMETRIC_WGSL`] source.
#[inline]
pub const fn canonical_emf_volumetric_wgsl() -> &'static str {
    EMF_VOLUMETRIC_WGSL
}

/// Zero-heap lookup of a canonical shader source by logical registry name.
#[inline]
pub fn canonical_shader_source(name: &str) -> Option<&'static str> {
    match name {
        "nquin_warp_scan.wgsl" | "nquin_warp_scan" => Some(NQUIN_WARP_SCAN_WGSL),
        "sieve.wgsl" | "sieve" => Some(SIEVE_WGSL),
        "holographic.wgsl" | "holographic" => Some(HOLOGRAPHIC_WGSL),
        "holographic_standalone.wgsl" | "holographic_standalone" => {
            Some(HOLOGRAPHIC_STANDALONE_WGSL)
        }
        "emf_volumetric.wgsl" | "emf_volumetric" => Some(EMF_VOLUMETRIC_WGSL),
        "emf_volumetric_stokes.wgsl" | "emf_volumetric_stokes" => Some(EMF_VOLUMETRIC_STOKES_WGSL),
        "spectral.wgsl" | "spectral" => Some(viewport::SPECTRAL_WGSL),
        "ambient.wgsl" | "ambient" => Some(viewport::AMBIENT_WGSL),
        "projector.wgsl" | "projector" => Some(viewport::PROJECTOR_WGSL),
        "epistemic.wgsl" | "epistemic" => Some(viewport::EPISTEMIC_WGSL),
        "screen.wgsl" | "screen" => Some(viewport::SCREEN_WGSL),
        "water.wgsl" | "water" | "water_surface.wgsl" | "water_surface" => {
            Some(viewport::WATER_WGSL)
        }
        "mesh.wgsl" | "mesh" => Some(viewport::MESH_WGSL),
        "bloom.wgsl" | "bloom" => Some(viewport::BLOOM_WGSL),
        "temporal_resolve.wgsl" | "temporal_resolve" => Some(viewport::TEMPORAL_RESOLVE_WGSL),
        "instance_cull_indirect.wgsl"
        | "instance_cull_indirect"
        | "indirect_cull.wgsl"
        | "indirect_cull" => Some(viewport::INSTANCE_CULL_INDIRECT_WGSL),
        "gemm_substrate.wgsl" | "gemm_substrate" => Some(GEMM_SUBSTRATE_WGSL),
        "topk_reduction.wgsl" | "topk_reduction" => Some(TOPK_REDUCTION_WGSL),
        "pga_sclerp_eval.wgsl" | "pga_sclerp_eval" => Some(PGA_SCLERP_EVAL_WGSL),
        "spring_damper_grid.wgsl" | "spring_damper_grid" => Some(SPRING_DAMPER_GRID_WGSL),
        "hud_glass_blur.wgsl" | "hud_glass_blur" => Some(HUD_GLASS_BLUR_WGSL),
        "wave_interference.wgsl" | "wave_interference" => Some(WAVE_INTERFERENCE_WGSL),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rm03_canonical_shader_exports_match_etalon_byte_for_byte() {
        // 1. NQuin warp-scan sieve parity with etalon_nquin
        assert_eq!(
            NQUIN_WARP_SCAN_WGSL.as_bytes(),
            etalon_nquin::NQUIN_WARP_SCAN_WGSL.as_bytes(),
            "NQUIN_WARP_SCAN_WGSL must be byte-for-byte identical to etalon_nquin::NQUIN_WARP_SCAN_WGSL"
        );
        assert_eq!(
            canonical_nquin_warp_scan_wgsl().as_bytes(),
            etalon_nquin::NQUIN_WARP_SCAN_WGSL.as_bytes()
        );

        // 2. Holographic shader parity with etalon_render
        assert_eq!(
            HOLOGRAPHIC_WGSL.as_bytes(),
            etalon_render::shaders::HOLOGRAPHIC_WGSL.as_bytes(),
            "HOLOGRAPHIC_WGSL must be byte-for-byte identical to etalon_render::shaders::HOLOGRAPHIC_WGSL"
        );
        assert_eq!(
            viewport::HOLOGRAPHIC_WGSL.as_bytes(),
            etalon_render::shaders::HOLOGRAPHIC_WGSL.as_bytes()
        );
        assert_eq!(
            HOLOGRAPHIC_STANDALONE_WGSL.as_bytes(),
            etalon_render::HOLOGRAPHIC_WGSL.as_bytes(),
            "HOLOGRAPHIC_STANDALONE_WGSL must be byte-for-byte identical to etalon_render::volumetric::HOLOGRAPHIC_WGSL"
        );
        assert_eq!(
            HOLOGRAPHIC_STANDALONE_WGSL.as_bytes(),
            crate::render::gpu::holographic::ETALON_HOLOGRAPHIC_WGSL.as_bytes()
        );

        // 3. EMF volumetric shader parity with etalon_render
        assert_eq!(
            EMF_VOLUMETRIC_WGSL.as_bytes(),
            etalon_render::shaders::EMF_VOLUMETRIC_WGSL.as_bytes(),
            "EMF_VOLUMETRIC_WGSL must be byte-for-byte identical to etalon_render::shaders::EMF_VOLUMETRIC_WGSL"
        );
        assert_eq!(
            viewport::EMF_VOLUMETRIC_WGSL.as_bytes(),
            etalon_render::shaders::EMF_VOLUMETRIC_WGSL.as_bytes()
        );
        assert_eq!(
            EMF_VOLUMETRIC_STOKES_WGSL.as_bytes(),
            etalon_render::EMF_VOLUMETRIC_WGSL.as_bytes(),
            "EMF_VOLUMETRIC_STOKES_WGSL must be byte-for-byte identical to etalon_render::emf::EMF_VOLUMETRIC_WGSL"
        );

        // 4. Full viewport shader registry byte-for-byte parity with etalon_render::shaders
        let pairs: &[(&str, &str, &str)] = &[
            ("SPECTRAL_WGSL", viewport::SPECTRAL_WGSL, etalon_render::shaders::SPECTRAL_WGSL),
            ("AMBIENT_WGSL", viewport::AMBIENT_WGSL, etalon_render::shaders::AMBIENT_WGSL),
            ("PROJECTOR_WGSL", viewport::PROJECTOR_WGSL, etalon_render::shaders::PROJECTOR_WGSL),
            ("EPISTEMIC_WGSL", viewport::EPISTEMIC_WGSL, etalon_render::shaders::EPISTEMIC_WGSL),
            ("SCREEN_WGSL", viewport::SCREEN_WGSL, etalon_render::shaders::SCREEN_WGSL),
            ("WATER_WGSL", viewport::WATER_WGSL, etalon_render::shaders::WATER_WGSL),
            ("MESH_WGSL", viewport::MESH_WGSL, etalon_render::shaders::MESH_WGSL),
            ("BLOOM_WGSL", viewport::BLOOM_WGSL, etalon_render::shaders::BLOOM_WGSL),
            ("CLUSTER_LIGHT_WGSL", viewport::CLUSTER_LIGHT_WGSL, etalon_render::shaders::CLUSTER_LIGHT_WGSL),
            ("AO_PREPASS_WGSL", viewport::AO_PREPASS_WGSL, etalon_render::shaders::AO_PREPASS_WGSL),
            ("AO_GENERATE_WGSL", viewport::AO_GENERATE_WGSL, etalon_render::shaders::AO_GENERATE_WGSL),
            ("DEFERRED_RESOLVE_WGSL", viewport::DEFERRED_RESOLVE_WGSL, etalon_render::shaders::DEFERRED_RESOLVE_WGSL),
            ("TEMPORAL_RESOLVE_WGSL", viewport::TEMPORAL_RESOLVE_WGSL, etalon_render::shaders::TEMPORAL_RESOLVE_WGSL),
            ("TEMPORAL_DEPTH_WGSL", viewport::TEMPORAL_DEPTH_WGSL, etalon_render::shaders::TEMPORAL_DEPTH_WGSL),
            ("TEMPORAL_OUTPUT_WGSL", viewport::TEMPORAL_OUTPUT_WGSL, etalon_render::shaders::TEMPORAL_OUTPUT_WGSL),
            ("TEMPORAL_MOTION_WGSL", viewport::TEMPORAL_MOTION_WGSL, etalon_render::shaders::TEMPORAL_MOTION_WGSL),
            ("TEMPORAL_REACTIVE_WGSL", viewport::TEMPORAL_REACTIVE_WGSL, etalon_render::shaders::TEMPORAL_REACTIVE_WGSL),
            ("TEMPORAL_WARP_WGSL", viewport::TEMPORAL_WARP_WGSL, etalon_render::shaders::TEMPORAL_WARP_WGSL),
            ("INSTANCE_CULL_INDIRECT_WGSL", viewport::INSTANCE_CULL_INDIRECT_WGSL, etalon_render::shaders::INSTANCE_CULL_INDIRECT_WGSL),
            ("OUTPUT_TRANSFORM_WGSL", viewport::OUTPUT_TRANSFORM_WGSL, etalon_render::shaders::OUTPUT_TRANSFORM_WGSL),
            ("SUN_SHADOW_WGSL", viewport::SUN_SHADOW_WGSL, etalon_render::shaders::SUN_SHADOW_WGSL),
            ("SKY_WGSL", viewport::SKY_WGSL, etalon_render::shaders::SKY_WGSL),
            ("PRECIPITATION_WGSL", viewport::PRECIPITATION_WGSL, etalon_render::shaders::PRECIPITATION_WGSL),
            ("FOLIAGE_LOD_WGSL", viewport::FOLIAGE_LOD_WGSL, etalon_render::shaders::FOLIAGE_LOD_WGSL),
            ("TILE_COMPACTION_WGSL", viewport::TILE_COMPACTION_WGSL, etalon_render::shaders::TILE_COMPACTION_WGSL),
            ("HYDROLOGIC_OPTICS_WGSL", viewport::HYDROLOGIC_OPTICS_WGSL, etalon_render::shaders::HYDROLOGIC_OPTICS_WGSL),
            ("SGGX_MICROFLAKE_WGSL", viewport::SGGX_MICROFLAKE_WGSL, etalon_render::shaders::SGGX_MICROFLAKE_WGSL),
            ("SWE_SOLVER_WGSL", viewport::SWE_SOLVER_WGSL, etalon_render::shaders::SWE_SOLVER_WGSL),
            ("SURFEL_INSTANCED_WGSL", viewport::SURFEL_INSTANCED_WGSL, etalon_render::shaders::SURFEL_INSTANCED_WGSL),
            ("ACOUSTIC_VOXEL_CONE_WGSL", viewport::ACOUSTIC_VOXEL_CONE_WGSL, etalon_render::shaders::ACOUSTIC_VOXEL_CONE_WGSL),
        ];
        for &(name, qualia_export, etalon_export) in pairs {
            assert_eq!(
                qualia_export.as_bytes(),
                etalon_export.as_bytes(),
                "viewport::{name} diverged from etalon_render::shaders::{name}"
            );
        }
    }

    #[test]
    fn rm03_on_disk_viewport_wgsl_files_never_diverge_from_canonical_etalon_render() {
        // RM-03 retired (`R`): `crates/qualia-core-db/src/shaders/viewport/*.wgsl` duplicates are removed
        // and all viewport shaders are sourced directly from `etalon_render::shaders::*`.
        assert_eq!(
            viewport::SPECTRAL_WGSL.as_bytes(),
            etalon_render::shaders::SPECTRAL_WGSL.as_bytes()
        );
        assert_eq!(
            viewport::HOLOGRAPHIC_WGSL.as_bytes(),
            etalon_render::shaders::HOLOGRAPHIC_WGSL.as_bytes()
        );
        assert_eq!(
            viewport::EMF_VOLUMETRIC_WGSL.as_bytes(),
            etalon_render::shaders::EMF_VOLUMETRIC_WGSL.as_bytes()
        );
        assert_eq!(
            viewport::TEMPORAL_RESOLVE_WGSL.as_bytes(),
            etalon_render::shaders::TEMPORAL_RESOLVE_WGSL.as_bytes()
        );
        assert_eq!(
            viewport::INDIRECT_CULL_WGSL.as_bytes(),
            etalon_render::shaders::INSTANCE_CULL_INDIRECT_WGSL.as_bytes()
        );
        assert_eq!(
            viewport::WATER_SURFACE_WGSL.as_bytes(),
            etalon_render::shaders::WATER_WGSL.as_bytes()
        );

        for (name, src) in [
            ("SPECTRAL_WGSL", viewport::SPECTRAL_WGSL),
            ("HOLOGRAPHIC_WGSL", viewport::HOLOGRAPHIC_WGSL),
            ("EMF_VOLUMETRIC_WGSL", viewport::EMF_VOLUMETRIC_WGSL),
            ("TEMPORAL_RESOLVE_WGSL", viewport::TEMPORAL_RESOLVE_WGSL),
            ("INDIRECT_CULL_WGSL", viewport::INDIRECT_CULL_WGSL),
            ("WATER_SURFACE_WGSL", viewport::WATER_SURFACE_WGSL),
        ] {
            let module = naga::front::wgsl::parse_str(src)
                .unwrap_or_else(|e| panic!("viewport::{name} failed Naga WGSL frontend parse: {e}"));
            let mut validator = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            );
            validator
                .validate(&module)
                .unwrap_or_else(|e| panic!("viewport::{name} failed Naga validation: {e}"));
        }
    }

    #[test]
    fn rm03_canonical_registry_lookup_and_wgsl_validation() {
        assert_eq!(
            canonical_shader_source("nquin_warp_scan.wgsl").unwrap().as_bytes(),
            NQUIN_WARP_SCAN_WGSL.as_bytes()
        );
        assert_eq!(
            canonical_shader_source("sieve.wgsl").unwrap().as_bytes(),
            SIEVE_WGSL.as_bytes()
        );
        assert_eq!(
            canonical_shader_source("holographic.wgsl").unwrap().as_bytes(),
            HOLOGRAPHIC_WGSL.as_bytes()
        );
        assert_eq!(
            canonical_shader_source("emf_volumetric.wgsl").unwrap().as_bytes(),
            EMF_VOLUMETRIC_WGSL.as_bytes()
        );
        assert!(canonical_shader_source("nonexistent.wgsl").is_none());

        for (name, wgsl) in [
            ("nquin_warp_scan.wgsl", NQUIN_WARP_SCAN_WGSL),
            ("sieve.wgsl", SIEVE_WGSL),
            ("holographic.wgsl", HOLOGRAPHIC_WGSL),
            ("holographic_standalone.wgsl", HOLOGRAPHIC_STANDALONE_WGSL),
            ("emf_volumetric.wgsl", EMF_VOLUMETRIC_WGSL),
            ("emf_volumetric_stokes.wgsl", EMF_VOLUMETRIC_STOKES_WGSL),
            ("gemm_substrate.wgsl", GEMM_SUBSTRATE_WGSL),
            ("topk_reduction.wgsl", TOPK_REDUCTION_WGSL),
        ] {
            let module = naga::front::wgsl::parse_str(wgsl)
                .unwrap_or_else(|e| panic!("RM-03 shader {name} failed WGSL parse: {e}"));
            let mut validator = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            );
            validator
                .validate(&module)
                .unwrap_or_else(|e| panic!("RM-03 shader {name} failed Naga validation: {e}"));
        }
    }
}
