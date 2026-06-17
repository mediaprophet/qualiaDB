//! Phenomenal viewport CI contract — WGSL bindings, binary layout, and regression oracles.
//!
//! Run via `cargo test -p qualia-core-db phenomenal_contract --lib` or
//! `node docs/tests/phenomenal-verify.mjs`.

use crate::shaders::viewport::{AMBIENT_WGSL, BLOOM_WGSL, PROJECTOR_WGSL};
use crate::tensor::buffer_export::{
    TensorBufferHeader, TENSOR_HEADER_BYTES, TENSOR_STRIDE,
};
use crate::portal_acoustic::{sigma_to_center_frequency_hz, sigma_to_wavelength_nm, ACOUSTIC_UNIFORM_FLOAT_COUNT};
use crate::portal_control::{PortalControlCommand, CONTROL_RING_CAP, ICP_MAGIC_BIT};
use crate::portal_spectral::sigma_to_cie_xyz;
use crate::tensor::Tensor10D;

/// Rust `portal_gpu` projector camera bind group (group 0).
pub const PROJECTOR_GROUP0_BINDINGS: &[u32] = &[0, 1];
/// Rust `portal_gpu` projector tensor SOA bind group (group 1).
pub const PROJECTOR_GROUP1_BINDINGS: &[u32] = &[0];
/// Rust ambient layout — binding 4 is reserved for `ObserverStandpoint` (not yet in WGSL).
pub const AMBIENT_GROUP0_BINDINGS: &[u32] = &[0, 1, 2, 3, 4];
pub const BLOOM_GROUP0_BINDINGS: &[u32] = &[0, 1, 2, 3, 4];

/// Parse `@group(G) @binding(B)` declarations from WGSL source lines.
pub fn parse_wgsl_bindings(source: &str) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for line in source.lines() {
        if !line.contains("@group(") || !line.contains("@binding(") {
            continue;
        }
        let Some(group) = parse_u32_after(line, "@group(") else {
            continue;
        };
        let Some(binding) = parse_u32_after(line, "@binding(") else {
            continue;
        };
        if !out.contains(&(group, binding)) {
            out.push((group, binding));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    out
}

fn parse_u32_after(line: &str, token: &str) -> Option<u32> {
    let rest = line.split(token).nth(1)?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Every WGSL-declared binding must exist in the Rust bind-group layout manifest.
pub fn assert_wgsl_bindings_covered(
    wgsl_source: &str,
    group: u32,
    rust_bindings: &[u32],
) -> Result<(), String> {
    let wgsl: Vec<u32> = parse_wgsl_bindings(wgsl_source)
        .into_iter()
        .filter(|(g, _)| *g == group)
        .map(|(_, b)| b)
        .collect();
    for b in wgsl {
        if !rust_bindings.contains(&b) {
            return Err(format!(
                "WGSL group({group}) binding({b}) missing from Rust layout {rust_bindings:?}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
fn validate_wgsl_smoke(label: &str, source: &str) {
    use naga::front::wgsl::Frontend;

    // Parse-only smoke: catches syntax regressions on native CI. Full layout validation
    // is enforced by `CameraUniform`/`ObserverStandpoint` size tests below and by
    // `cargo check --target wasm32-unknown-unknown --features portal` (wgpu pipeline create).
    Frontend::new()
        .parse(source)
        .unwrap_or_else(|e| panic!("{label}: WGSL parse failed: {e:?}"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu_context::{
        ambient_draw_instances_for_mode, ComputeUniverse, OperationalMode, UniverseOrchestrator,
    };
    use crate::portal_pga::motor_rq_gated;
    use crate::portal_telemetry::{
        CameraUniform, ObserverStandpoint, ParticleInstance, SystemTelemetry,
        STANDPOINT_DID, STANDPOINT_EPHEMERAL, STANDPOINT_SPECTATOR, STANDPOINT_VAULT,
    };
    use crate::audio::acoustic_plane::{AcousticUniform, SONIC_RING_CAP};
    use crate::audio::acoustic_sab::{init_acoustic_sab, ACOUSTIC_SAB_BYTES};
    use crate::audio::hrtf::binaural_from_position;
    use crate::audio::audio_spectral_sheet::SPECTRAL_PREVIEW_BINS;
    use crate::sonic_token::SonicToken;
    use crate::tensor::buffer_export::tensor_node_count;

    const IDENTITY_ROTOR: [f32; 4] = [1.0, 0.0, 0.0, 0.0];

    #[test]
    fn phenomenal_shader_modules_parse() {
        validate_wgsl_smoke("ambient", AMBIENT_WGSL);
        validate_wgsl_smoke("projector", PROJECTOR_WGSL);
        validate_wgsl_smoke("bloom", BLOOM_WGSL);
    }

    #[test]
    fn phenomenal_binding_layout_matches_wgsl() {
        assert_wgsl_bindings_covered(AMBIENT_WGSL, 0, AMBIENT_GROUP0_BINDINGS)
            .expect("ambient bindings");
        assert_wgsl_bindings_covered(PROJECTOR_WGSL, 0, PROJECTOR_GROUP0_BINDINGS)
            .expect("projector group0");
        assert_wgsl_bindings_covered(PROJECTOR_WGSL, 1, PROJECTOR_GROUP1_BINDINGS)
            .expect("projector group1");
        assert_wgsl_bindings_covered(BLOOM_WGSL, 0, BLOOM_GROUP0_BINDINGS).expect("bloom");
    }

    #[test]
    fn phenomenal_uniform_struct_sizes_match_wgsl() {
        assert_eq!(std::mem::size_of::<TensorBufferHeader>(), 32);
        assert_eq!(TENSOR_HEADER_BYTES, 32);
        assert_eq!(std::mem::size_of::<Tensor10D>(), 40);
        assert_eq!(TENSOR_STRIDE, 40);
        assert_eq!(std::mem::size_of::<SystemTelemetry>(), 48);
        assert_eq!(std::mem::size_of::<ParticleInstance>(), 16);
        assert_eq!(std::mem::size_of::<CameraUniform>(), 128);
        assert_eq!(std::mem::size_of::<ObserverStandpoint>(), 128);
    }

    #[test]
    fn phenomenal_tensor_header_stride_matches_gpu_upload() {
        let tensors = [Tensor10D::ground_truth(0.0, 0.0, 0.1, 0.2, 0.3, 0.0, 1.0, 0.0, 0.5)];
        let need = TensorBufferHeader::total_bytes(tensors.len());
        let mut buf = vec![0u8; need];
        crate::tensor::buffer_export::write_tensor_buffer(&tensors, &mut buf).unwrap();
        let (header, header_len) =
            crate::tensor::buffer_export::parse_header(&buf).expect("header");
        assert_eq!(header_len, TENSOR_HEADER_BYTES);
        assert_eq!(header.stride as usize, TENSOR_STRIDE);
        assert_eq!(tensor_node_count(&buf).unwrap(), 1);
        // `PortalGpu::upload_tensor_buffer` skips the 32 B header when binding SOA storage.
        assert_eq!(
            header_len + header.node_count as usize * TENSOR_STRIDE,
            buf.len()
        );
    }

    #[test]
    fn phenomenal_standpoint_rq_motor_identity_gate() {
        for class in [
            STANDPOINT_SPECTATOR,
            STANDPOINT_EPHEMERAL,
            STANDPOINT_DID,
            STANDPOINT_VAULT,
        ] {
            assert_eq!(
                motor_rq_gated(0.0, 0.5, 1.0, 1.0, class, 1.0),
                IDENTITY_ROTOR,
                "collapsed q class={class}"
            );
        }
        assert_eq!(
            motor_rq_gated(0.9, 0.5, 1.0, 1.0, STANDPOINT_DID, 0.0),
            IDENTITY_ROTOR,
            "DID epistemic aperture 0"
        );
        assert_eq!(
            motor_rq_gated(0.9, 0.5, 1.0, 1.0, STANDPOINT_VAULT, 1.0),
            IDENTITY_ROTOR,
            "vault always identity"
        );
        let active = motor_rq_gated(0.5, 0.25, 2.0, 1.0, STANDPOINT_SPECTATOR, 1.0);
        assert!(
            (active[0] - 1.0).abs() > 1e-4 || active[1].abs() > 1e-4,
            "sandbox q should spin: {active:?}"
        );
    }

    #[test]
    fn phenomenal_vram_ledger_full_mode_draws_above_eco_cap() {
        let resident = 50_000_u32;
        let full = ambient_draw_instances_for_mode(resident, OperationalMode::Full);
        let eco = ambient_draw_instances_for_mode(resident, OperationalMode::Eco);
        assert!(full > 8_000, "Full mode must draw >8k instances");
        assert_eq!(eco, 8_000, "Eco mode caps at 8k");
        assert_eq!(
            UniverseOrchestrator::from_total_budget(
                6 * 1024 * 1024 * 1024,
                OperationalMode::Full
            )
            .max_particles(ComputeUniverse::Viewport, OperationalMode::Full),
            50_000
        );
    }

    #[test]
    fn phenomenal_vram_ledger_pressure_step_down() {
        let local = crate::gpu_context::VramLedger::new(1000);
        local.record_tensor(500);
        assert_eq!(local.mode(), OperationalMode::Full);
        local.record_kv_cache(300);
        assert_eq!(local.mode(), OperationalMode::Eco);
        local.record_render(200);
        assert_eq!(local.mode(), OperationalMode::Reserve);
    }

    #[test]
    fn phenomenal_acoustic_uniform_layout() {
        assert_eq!(std::mem::size_of::<SonicToken>(), 8);
        assert_eq!(SONIC_RING_CAP, 128);
        let uniform = AcousticUniform::default();
        let bytes = bytemuck::bytes_of(&uniform);
        assert_eq!(bytes.len(), std::mem::size_of::<AcousticUniform>());
        // 18 scalars (binaural + STFT frame) + 64 preview bins
        assert_eq!(std::mem::size_of::<AcousticUniform>(), 72 + SPECTRAL_PREVIEW_BINS * 4);
        assert_eq!(std::mem::size_of::<AcousticUniform>(), 328);
    }

    #[test]
    fn phenomenal_sigma_visual_audio_parity() {
        for i in 0..=10 {
            let sigma = i as f32 / 10.0;
            let lambda = sigma_to_wavelength_nm(sigma);
            assert!(lambda >= 400.0 && lambda <= 700.0);
            let _xyz = sigma_to_cie_xyz(sigma);
            let hz = sigma_to_center_frequency_hz(sigma);
            assert!(hz >= 55.0 && hz <= 8_000.0);
            let hz2 = sigma_to_center_frequency_hz(sigma + 1.0);
            assert!((hz - hz2).abs() < 1e-3, "σ fract parity");
        }
        assert_eq!(ACOUSTIC_UNIFORM_FLOAT_COUNT, 82);
    }

    #[test]
    fn phenomenal_hrtf_and_sab_layout() {
        let g = binaural_from_position([1.0, 0.0, -1.0], 0.0);
        assert!(g.gain_r > g.gain_l);
        let mut sab = [0u8; ACOUSTIC_SAB_BYTES];
        assert!(init_acoustic_sab(&mut sab));
        assert_eq!(ACOUSTIC_SAB_BYTES, 1024);
    }

    #[test]
    fn phenomenal_icp_command_layout() {
        let cmd = PortalControlCommand::navigate_index(9);
        assert_eq!(std::mem::size_of::<PortalControlCommand>(), 8);
        assert!((cmd.raw & ICP_MAGIC_BIT) != 0);
        assert_eq!(cmd.tensor_or_menu_index(), 9);
        assert!(CONTROL_RING_CAP >= 64);
    }

    #[test]
    fn phenomenal_u3_aliases_u1_partition() {
        let orch = UniverseOrchestrator::from_total_budget_full(10_000);
        let u1 = orch.partition(ComputeUniverse::Tensor10D).ledger_range;
        let u3 = orch.partition(ComputeUniverse::AcousticPlane).ledger_range;
        assert_eq!(u1.offset, u3.offset);
        assert_eq!(u1.size, u3.size);
        assert_eq!(
            orch.effective_mode(ComputeUniverse::AcousticPlane, OperationalMode::Reserve),
            OperationalMode::Reserve
        );
    }
}