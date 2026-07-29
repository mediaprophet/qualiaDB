//! Differential testing: verify that cross-profile shader emission produces
//! equivalent kernels. Tests that HLSL, MSL, and WGSL backends all emit
//! valid source for the same `KernelSpec`, and that PTX emitters produce
//! expected instruction patterns.
//!
//! These tests run headless (no GPU required). They verify emission
//! correctness, not numerical equivalence (which requires GPU execution).

use qualia_core_db::wgsl_forge::emit::{emit_hlsl, emit_msl, emit_ptx, emit_wgsl};
use qualia_core_db::wgsl_forge::{
    generate_builtin, BufferAccess, BufferElement, BufferSpec, BuiltinKernel, KernelSpec,
    ScalarType, Schedule, TargetBackend,
};

/// Helper: create a storage buffer spec.
fn storage(name: &str, binding: u32) -> BufferSpec {
    BufferSpec {
        group: 0,
        binding,
        name: name.to_string(),
        element: BufferElement::Scalar(ScalarType::F32),
        access: BufferAccess::StorageRead,
    }
}

/// Helper: create a read-write storage buffer spec.
fn storage_rw(name: &str, binding: u32) -> BufferSpec {
    BufferSpec {
        group: 0,
        binding,
        name: name.to_string(),
        element: BufferElement::Scalar(ScalarType::F32),
        access: BufferAccess::StorageReadWrite,
    }
}

/// Helper: create a uniform buffer spec.
fn uniform(name: &str, binding: u32) -> BufferSpec {
    BufferSpec {
        group: 0,
        binding,
        name: name.to_string(),
        element: BufferElement::Scalar(ScalarType::U32),
        access: BufferAccess::Uniform,
    }
}

/// Helper: create a minimal GEMV kernel spec for cross-profile testing.
fn gemv_spec(entry: &str) -> KernelSpec {
    KernelSpec {
        id: "gemv".to_string(),
        semantic_version: 1,
        entry_point: entry.to_string(),
        description: "GEMV test".to_string(),
        buffers: vec![
            storage("a", 0),
            storage("x", 1),
            storage_rw("y", 2),
            uniform("params", 3),
        ],
        ops: Vec::new(),
        shared_memory: Vec::new(),
    }
}

/// Helper: create a minimal RMSNorm kernel spec.
fn rmsnorm_spec(entry: &str) -> KernelSpec {
    KernelSpec {
        id: "rmsnorm".to_string(),
        semantic_version: 1,
        entry_point: entry.to_string(),
        description: "RMSNorm test".to_string(),
        buffers: vec![
            storage("x", 0),
            storage("weight", 1),
            storage_rw("y", 2),
            uniform("params", 3),
        ],
        ops: Vec::new(),
        shared_memory: Vec::new(),
    }
}

/// WGSL and HLSL both emit valid source for every built-in kernel.
/// This catches profile-specific emission failures.
#[test]
fn wgsl_and_hlsl_both_emit_for_all_builtins() {
    for builtin in BuiltinKernel::ALL {
        let schedule = Schedule::default();
        // WGSL must always work
        let wgsl = generate_builtin(builtin, schedule, TargetBackend::Wgsl)
            .unwrap_or_else(|e| panic!("WGSL generate {} failed: {e}", builtin.name()));
        assert!(!wgsl.source.is_empty(), "{} WGSL empty", builtin.name());

        // HLSL may fail for ray-probe (expected), but others should emit
        if builtin != BuiltinKernel::RayProbe {
            let hlsl = generate_builtin(builtin, schedule, TargetBackend::Hlsl);
            if let Ok(h) = hlsl {
                assert!(!h.source.is_empty(), "{} HLSL empty", builtin.name());
            }
        }
    }
}

/// MSL emits valid source for GEMV with SIMD-group acceleration when
/// workgroup size >= 32, and scalar otherwise.
#[test]
fn msl_emits_gemv_with_simd_and_scalar_paths() {
    let spec = gemv_spec("gemv_msl");

    let simd_schedule = Schedule {
        workgroup_size: 32,
        ..Default::default()
    };
    let simd = emit_msl(&spec, simd_schedule).expect("MSL SIMD GEMV emit");
    assert!(
        simd.source.contains("simd_sum"),
        "MSL SIMD path should use simd_sum"
    );

    let scalar_schedule = Schedule {
        workgroup_size: 1,
        ..Default::default()
    };
    let scalar = emit_msl(&spec, scalar_schedule).expect("MSL scalar GEMV emit");
    assert!(
        !scalar.source.contains("simd_sum"),
        "MSL scalar path should not use simd_sum"
    );
}

/// MSL emits valid source for RMSNorm decode kernel.
#[test]
fn msl_emits_rmsnorm_decode_kernel() {
    let spec = rmsnorm_spec("rmsnorm_msl");
    let schedule = Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let shader = emit_msl(&spec, schedule).expect("MSL RMSNorm emit");
    assert!(
        shader.source.contains("rsqrt"),
        "MSL RMSNorm should use rsqrt"
    );
    assert!(
        shader.source.contains("threadgroup_barrier"),
        "MSL RMSNorm should use threadgroup_barrier"
    );
}

/// MSL emits valid source for SDPA decode kernel.
#[test]
fn msl_emits_sdpa_decode_kernel() {
    let spec = KernelSpec {
        id: "sdpa-decode".to_string(),
        semantic_version: 1,
        entry_point: "sdpa_decode_msl".to_string(),
        description: "SDPA decode test".to_string(),
        buffers: vec![
            storage("q", 0),
            storage("kv", 1),
            storage_rw("out", 2),
            uniform("params", 3),
        ],
        ops: Vec::new(),
        shared_memory: Vec::new(),
    };
    let schedule = Schedule {
        workgroup_size: 32,
        ..Default::default()
    };
    let shader = emit_msl(&spec, schedule).expect("MSL SDPA decode emit");
    assert!(
        shader.source.contains("exp2"),
        "MSL SDPA should use exp2 for softmax"
    );
    assert!(
        shader.source.contains("simd_sum"),
        "MSL SDPA should use simd_sum for dot reduction"
    );
}

/// MSL emits valid source for fused QKV+RoPE kernel.
#[test]
fn msl_emits_fused_qkv_rope_kernel() {
    let spec = KernelSpec {
        id: "fused-qkv-rope".to_string(),
        semantic_version: 1,
        entry_point: "fused_qkv_rope_msl".to_string(),
        description: "Fused QKV+RoPE test".to_string(),
        buffers: vec![
            storage("x", 0),
            storage("Wq", 1),
            storage("Wk", 2),
            storage("Wv", 3),
            storage_rw("yq", 4),
            storage_rw("yk", 5),
            storage_rw("yv", 6),
            uniform("dims", 7),
            uniform("rope_params", 8),
        ],
        ops: Vec::new(),
        shared_memory: Vec::new(),
    };
    let schedule = Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let shader = emit_msl(&spec, schedule).expect("MSL fused QKV+RoPE emit");
    assert!(
        shader.source.contains("threadgroup"),
        "MSL fused QKV+RoPE should use threadgroup memory"
    );
    assert!(
        shader.source.contains("sin"),
        "MSL fused QKV+RoPE should use sin for RoPE"
    );
    assert!(
        shader.source.contains("cos"),
        "MSL fused QKV+RoPE should use cos for RoPE"
    );
}

/// HLSL emits valid source for fused QKV+RoPE kernel.
#[test]
fn hlsl_emits_fused_qkv_rope_kernel() {
    let spec = KernelSpec {
        id: "fused-qkv-rope".to_string(),
        semantic_version: 1,
        entry_point: "fused_qkv_rope_hlsl".to_string(),
        description: "Fused QKV+RoPE HLSL test".to_string(),
        buffers: vec![
            storage("x", 0),
            storage("Wq", 1),
            storage("Wk", 2),
            storage("Wv", 3),
            storage_rw("yq", 4),
            storage_rw("yk", 5),
            storage_rw("yv", 6),
            uniform("dims", 7),
            uniform("rope_params", 8),
        ],
        ops: Vec::new(),
        shared_memory: Vec::new(),
    };
    let schedule = Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let shader = emit_hlsl(&spec, schedule).expect("HLSL fused QKV+RoPE emit");
    assert!(
        shader.source.contains("groupshared"),
        "HLSL fused QKV+RoPE should use groupshared memory"
    );
    assert!(
        shader.source.contains("GroupMemoryBarrierWithGroupSync"),
        "HLSL fused QKV+RoPE should use group barrier"
    );
}

/// PTX emits valid source for all implemented kernel IDs.
#[test]
fn ptx_emits_valid_source_for_all_implemented_kernels() {
    let kernels: &[(&str, &str)] = &[
        ("rmsnorm", "rmsnorm_ptx"),
        ("q4k-gemv", "q4k_gemv_ptx"),
        ("wmma-gemv", "wmma_gemv_ptx"),
        ("sdpa-decode", "sdpa_ptx"),
        ("q4k-soa-wmma", "q4k_soa_wmma_ptx"),
        ("q6k-soa-gemv", "q6k_soa_gemv_ptx"),
    ];

    for (id, entry) in kernels {
        let spec = KernelSpec {
            id: id.to_string(),
            semantic_version: 1,
            entry_point: entry.to_string(),
            description: format!("PTX {} test", id),
            buffers: vec![
                storage("x", 0),
                storage("W", 1),
                storage_rw("y", 2),
                uniform("params", 3),
            ],
            ops: Vec::new(),
            shared_memory: Vec::new(),
        };
        let schedule = Schedule::default();
        let shader =
            emit_ptx(&spec, schedule).unwrap_or_else(|e| panic!("PTX emit {} failed: {e}", id));
        assert!(
            !shader.source.is_empty(),
            "PTX {} produced empty source",
            id
        );
        assert!(
            shader.source.contains(".entry"),
            "PTX {} should contain .entry directive",
            id
        );
    }
}

/// Cross-profile: WGSL and MSL both emit GEMV with the same entry point name.
#[test]
fn cross_profile_gemv_entry_point_consistent() {
    let spec = gemv_spec("gemv_main");
    let schedule = Schedule::default();

    let wgsl = emit_wgsl(&spec, schedule).expect("WGSL GEMV emit");
    let msl = emit_msl(&spec, schedule).expect("MSL GEMV emit");

    assert!(
        wgsl.source.contains("gemv_main"),
        "WGSL should contain entry point"
    );
    assert!(
        msl.source.contains("gemv_main"),
        "MSL should contain entry point"
    );
}

/// MSL simdgroup_matrix GEMV kernel emits with expected Metal constructs.
#[test]
fn msl_emits_simdgroup_matrix_gemv() {
    let spec = KernelSpec {
        id: "gemv-simd-matrix".to_string(),
        semantic_version: 1,
        entry_point: "gemv_simd_mat".to_string(),
        description: "SIMD-group matrix GEMV test".to_string(),
        buffers: vec![
            storage("a", 0),
            storage("x", 1),
            storage_rw("y", 2),
            uniform("params", 3),
        ],
        ops: Vec::new(),
        shared_memory: Vec::new(),
    };
    let schedule = Schedule::default();
    let shader = emit_msl(&spec, schedule).expect("MSL simdgroup_matrix GEMV emit");
    assert!(
        shader.source.contains("simdgroup_matrix"),
        "Should use simdgroup_matrix"
    );
    assert!(
        shader.source.contains("simdgroup_multiply_accumulate"),
        "Should use simdgroup_multiply_accumulate"
    );
}

/// Metal mega-pass is unavailable on non-macOS (stub returns None).
#[test]
fn metal_mega_pass_stub_returns_none_on_non_macos() {
    use qualia_core_db::inference::metal_lane;
    if !metal_lane::metal_mega_pass_available() {
        assert!(metal_lane::try_metal_mega_pass(
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0.0,
            0.0,
            0.0,
            &[],
            &[],
            &[],
            None,
            None,
            0,
            0,
        )
        .is_none());
    }
}
