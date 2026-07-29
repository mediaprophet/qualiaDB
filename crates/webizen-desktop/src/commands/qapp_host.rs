//! Webizen Host API (qApp Message Bus)

#![allow(non_snake_case)]

use super::qapp_telemetry::{
    qapp_slug, ForgeComputeProbe, ForgeKernelProbe, ForgePhysicsCertification, QappAnalysisRequest,
    QappAnalysisResult, QualiaComputeProfile,
};
use super::*;
use tauri::{command, Manager, State};

// ── Webizen Host API (qApp Message Bus) ──────────────────────────────────────

pub struct HostApiState(pub crate::companion_gateway::HostApiHandle);

#[tauri::command]
pub fn submit_record(
    app: tauri::AppHandle,
    qapp_id: String,
    envelope: wellfare_core::record::RecordEnvelope,
    source: String,
) -> Result<usize, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |api_guard| {
        if let Some(host_api) = api_guard.as_mut() {
            host_api.submit_record(&qapp_id, envelope, &source)
        } else {
            Err("Host API not initialized".into())
        }
    })?
}

fn qapp_evidence_score(key: &str, value: &str) -> f32 {
    let hash = qualia_core_db::q_hash(&format!("{key}={value}"));
    let upper = (hash >> 40) as u32;
    (upper as f32) / ((1u32 << 24) as f32)
}

fn qapp_graph_for_scores(
    channel_count: usize,
) -> Result<qualia_core_db::wgsl_forge::ir::graph::ComputeGraph, String> {
    use qualia_core_db::wgsl_forge::ir::graph::{
        Axis, ComputeGraph, DType, OpNode, RedKind, Shape, TensorRef,
    };
    use qualia_core_db::wgsl_forge::Schedule;

    let mut graph = ComputeGraph::new();
    let input_len = channel_count.max(1) as u32;
    let input = TensorRef::external(Shape::new(&[input_len]), DType::F32);
    let out = graph
        .push(
            OpNode::Reduce {
                op: RedKind::Mean,
                axis: Axis::Last,
            },
            &[input],
            Shape::scalar(),
            DType::F32,
            Schedule::default(),
        )
        .map_err(|e| e.to_string())?;
    graph.mark_output(out);
    Ok(graph)
}

fn qapp_content_quins(
    request: &QappAnalysisRequest,
    canonical: &str,
    scores: &[f32],
) -> Vec<qualia_core_db::NQuin> {
    let context = qualia_core_db::q_hash(canonical);
    let subject = qualia_core_db::q_hash(&request.discipline);
    let notes_len = if request.notes.trim().is_empty() {
        0
    } else {
        1
    };
    let mut quins = Vec::with_capacity(request.fields.len() + notes_len);
    let mut score_idx = 0usize;

    for (key, value) in &request.fields {
        if value.trim().is_empty() {
            continue;
        }
        let score = scores.get(score_idx).copied().unwrap_or_default();
        score_idx += 1;
        quins.push(qualia_core_db::NQuin {
            subject,
            predicate: qualia_core_db::q_hash(key),
            object: qualia_core_db::q_hash(value),
            context,
            metadata: score.to_bits() as u64,
            parity: (score_idx as u64).wrapping_sub(1),
        });
    }

    if !request.notes.trim().is_empty() {
        let score = scores.get(score_idx).copied().unwrap_or_default();
        quins.push(qualia_core_db::NQuin {
            subject,
            predicate: qualia_core_db::q_hash("notes"),
            object: qualia_core_db::q_hash(request.notes.trim()),
            context,
            metadata: score.to_bits() as u64,
            parity: score_idx as u64,
        });
    }

    quins
}

fn hex32(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn deterministic_nbody_state() -> [f32; 64] {
    let mut state = [0.0_f32; 64];
    for particle in 0..8 {
        let base = particle * 8;
        let phase = particle as f32 * std::f32::consts::FRAC_PI_4;
        state[base] = phase.cos() * 3.0;
        state[base + 1] = phase.sin() * 3.0;
        state[base + 2] = (particle as f32 - 3.5) * 0.18;
        state[base + 3] = -phase.sin() * 0.12;
        state[base + 4] = phase.cos() * 0.12;
        state[base + 5] = 0.0;
        state[base + 6] = 1.0 + (particle % 3) as f32 * 0.25;
        state[base + 7] = if particle % 2 == 0 { 1.0 } else { -1.0 };
    }
    state
}

fn total_momentum(state: &[f32]) -> [f32; 3] {
    let mut momentum = [0.0_f32; 3];
    for particle in state.chunks_exact(8) {
        let mass = particle[6];
        momentum[0] += particle[3] * mass;
        momentum[1] += particle[4] * mass;
        momentum[2] += particle[5] * mass;
    }
    momentum
}

fn build_forge_physics_certification(run_gpu: bool) -> ForgePhysicsCertification {
    use qualia_core_db::wgsl_forge::physics::kinematics::{
        nbody_step_cpu, nbody_step_gpu, KIN_STRIDE,
    };
    use std::panic::{catch_unwind, AssertUnwindSafe};

    const DT: f32 = 0.005;
    const SOFTENING: f32 = 0.01;
    const COUPLING: f32 = 1.0;
    const CERTIFICATION_TOLERANCE: f32 = 1.0e-3;

    let state = deterministic_nbody_state();
    let started = std::time::Instant::now();
    // QualiaDB's certification API returns Vec buffers. Those bounded allocations,
    // plus the result Vec used by Tauri serialization, stay at this explicit command
    // boundary and never enter the render loop.
    let oracle = nbody_step_cpu(&state, DT, SOFTENING, COUPLING);
    let gpu_result = if run_gpu {
        catch_unwind(AssertUnwindSafe(|| {
            nbody_step_gpu(&state, DT, SOFTENING, COUPLING)
        }))
        .ok()
        .and_then(Result::ok)
    } else {
        None
    };

    let (output, backend, certified, max_abs_error, note) = match gpu_result {
        Some(gpu) => {
            let max_error = gpu
                .iter()
                .zip(&oracle)
                .map(|(actual, expected)| (actual - expected).abs())
                .fold(0.0_f32, f32::max);
            let passed = max_error <= CERTIFICATION_TOLERANCE;
            (
                gpu,
                "wgpu-forge".to_string(),
                passed,
                max_error,
                if passed {
                    "Forge WGPU kinematics matched the scalar CPU oracle.".to_string()
                } else {
                    format!(
                        "Forge WGPU result exceeded the {CERTIFICATION_TOLERANCE:.1e} certification tolerance."
                    )
                },
            )
        }
        None => (
            oracle.clone(),
            "cpu-oracle".to_string(),
            false,
            0.0,
            if run_gpu {
                "WGPU execution was unavailable; returned the deterministic CPU oracle without claiming GPU certification.".to_string()
            } else {
                "CPU oracle path used for deterministic verification.".to_string()
            },
        ),
    };

    let before = total_momentum(&state);
    let after = total_momentum(&output);
    let momentum_drift = ((after[0] - before[0]).powi(2)
        + (after[1] - before[1]).powi(2)
        + (after[2] - before[2]).powi(2))
    .sqrt();
    let result_fingerprint = output.iter().fold(0xcbf29ce484222325_u64, |acc, value| {
        acc.rotate_left(5) ^ value.to_bits() as u64
    });
    let provenance_quin = qualia_core_db::NQuin {
        subject: qualia_core_db::q_hash("webizen:physics-simulator"),
        predicate: qualia_core_db::q_hash("forge:nbody-step"),
        object: result_fingerprint,
        context: qualia_core_db::q_hash(qualia_core_db::ENGINE_VERSION),
        metadata: max_abs_error.to_bits() as u64,
        parity: (state.len() / KIN_STRIDE) as u64,
    };
    let root = qualia_core_db::wgsl_forge::ir::graph_merkle_root(&[provenance_quin]);
    let sample_positions = output
        .chunks_exact(KIN_STRIDE)
        .take(4)
        .map(|particle| [particle[0], particle[1], particle[2]])
        .collect();

    ForgePhysicsCertification {
        engine_version: qualia_core_db::ENGINE_VERSION.to_string(),
        forge_schema_version: qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION,
        kernel: "kinematics.nbody_step".to_string(),
        backend,
        particle_count: state.len() / KIN_STRIDE,
        certified,
        max_abs_error,
        momentum_drift,
        elapsed_ms: started.elapsed().as_secs_f64() * 1_000.0,
        q42_provenance: format!("q42:{}", hex32(root)),
        sample_positions,
        note,
    }
}

fn max_abs_error(actual: &[f32], expected: &[f32]) -> f32 {
    actual
        .iter()
        .zip(expected)
        .map(|(actual, expected)| (actual - expected).abs())
        .fold(0.0_f32, f32::max)
}

fn fingerprint_f32(values: &[f32]) -> u64 {
    values.iter().fold(0xcbf29ce484222325_u64, |acc, value| {
        acc.rotate_left(5) ^ value.to_bits() as u64
    })
}

fn build_forge_compute_probe() -> Result<ForgeComputeProbe, String> {
    use qualia_core_db::wgsl_forge::ForgeRuntime;

    const TOLERANCE: f32 = 1.0e-3;
    const SLAB_BYTES: usize = 8 * 1024 * 1024;

    // ForgeRuntime owns transient Vec-backed upload/readback buffers. This explicit,
    // user-triggered diagnostics boundary keeps those allocations out of Webizen's
    // render, diffusion, and 10D resident-substrate hot paths.
    let initialization_started = std::time::Instant::now();
    let mut runtime = ForgeRuntime::new(SLAB_BYTES, None).map_err(|err| err.to_string())?;
    let initialization_ms = initialization_started.elapsed().as_secs_f64() * 1_000.0;
    let mut kernels = Vec::with_capacity(3);
    let mut provenance_quins = Vec::with_capacity(3);

    let topk_input: Vec<f32> = (0..64)
        .map(|index| ((index * 37 % 101) as f32) - 50.0)
        .collect();
    let mut topk_expected = topk_input.clone();
    topk_expected.sort_by(|left, right| right.total_cmp(left));
    topk_expected.truncate(4);
    let started = std::time::Instant::now();
    let topk_output = runtime
        .topk(&topk_input, 4)
        .map_err(|err| format!("Forge Top-K failed: {err}"))?;
    let topk_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let topk_error = max_abs_error(&topk_output, &topk_expected);
    kernels.push(ForgeKernelProbe {
        kernel: "topk".to_string(),
        shape: "64 → 4".to_string(),
        output_elements: topk_output.len(),
        elapsed_ms: topk_ms,
        max_abs_error: topk_error,
        certified: topk_error <= TOLERANCE,
    });
    provenance_quins.push(qualia_core_db::NQuin {
        subject: qualia_core_db::q_hash("webizen:benchmark-harness"),
        predicate: qualia_core_db::q_hash("forge:topk"),
        object: fingerprint_f32(&topk_output),
        context: qualia_core_db::q_hash(qualia_core_db::ENGINE_VERSION),
        metadata: topk_error.to_bits() as u64,
        parity: topk_output.len() as u64,
    });

    const M: usize = 16;
    const K: usize = 16;
    const N: usize = 16;
    let matrix_a: Vec<f32> = (0..M * K)
        .map(|index| ((index * 13 % 29) as f32 - 14.0) / 7.0)
        .collect();
    let matrix_b: Vec<f32> = (0..K * N)
        .map(|index| ((index * 17 % 31) as f32 - 15.0) / 8.0)
        .collect();
    let mut gemm_expected = vec![0.0_f32; M * N];
    for row in 0..M {
        for column in 0..N {
            let mut sum = 0.0_f32;
            for inner in 0..K {
                sum += matrix_a[row * K + inner] * matrix_b[inner * N + column];
            }
            gemm_expected[row * N + column] = sum;
        }
    }
    let started = std::time::Instant::now();
    let gemm_output = runtime
        .gemm(&matrix_a, &matrix_b, M, K, N)
        .map_err(|err| format!("Forge GEMM failed: {err}"))?;
    let gemm_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let gemm_error = max_abs_error(&gemm_output, &gemm_expected);
    kernels.push(ForgeKernelProbe {
        kernel: "gemm".to_string(),
        shape: "16×16 · 16×16".to_string(),
        output_elements: gemm_output.len(),
        elapsed_ms: gemm_ms,
        max_abs_error: gemm_error,
        certified: gemm_error <= TOLERANCE,
    });
    provenance_quins.push(qualia_core_db::NQuin {
        subject: qualia_core_db::q_hash("webizen:benchmark-harness"),
        predicate: qualia_core_db::q_hash("forge:gemm"),
        object: fingerprint_f32(&gemm_output),
        context: qualia_core_db::q_hash(qualia_core_db::ENGINE_VERSION),
        metadata: gemm_error.to_bits() as u64,
        parity: gemm_output.len() as u64,
    });

    const FFT_POINTS: usize = 64;
    let mut fft_input = vec![0.0_f32; FFT_POINTS * 2];
    fft_input[0] = 1.0;
    let mut fft_expected = vec![0.0_f32; FFT_POINTS * 2];
    for point in 0..FFT_POINTS {
        fft_expected[point * 2] = 1.0;
    }
    let started = std::time::Instant::now();
    let fft_output = runtime
        .fft(&fft_input)
        .map_err(|err| format!("Forge FFT failed: {err}"))?;
    let fft_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let fft_error = max_abs_error(&fft_output, &fft_expected);
    kernels.push(ForgeKernelProbe {
        kernel: "fft".to_string(),
        shape: "64 complex points".to_string(),
        output_elements: fft_output.len(),
        elapsed_ms: fft_ms,
        max_abs_error: fft_error,
        certified: fft_error <= TOLERANCE,
    });
    provenance_quins.push(qualia_core_db::NQuin {
        subject: qualia_core_db::q_hash("webizen:benchmark-harness"),
        predicate: qualia_core_db::q_hash("forge:fft"),
        object: fingerprint_f32(&fft_output),
        context: qualia_core_db::q_hash(qualia_core_db::ENGINE_VERSION),
        metadata: fft_error.to_bits() as u64,
        parity: fft_output.len() as u64,
    });

    let total_kernel_ms = kernels.iter().map(|probe| probe.elapsed_ms).sum();
    let all_certified = kernels.iter().all(|probe| probe.certified);
    let root = qualia_core_db::wgsl_forge::ir::graph_merkle_root(&provenance_quins);

    Ok(ForgeComputeProbe {
        engine_version: qualia_core_db::ENGINE_VERSION.to_string(),
        forge_schema_version: qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION,
        backend: "wgpu-forge-runtime".to_string(),
        initialization_ms,
        total_kernel_ms,
        all_certified,
        q42_provenance: format!("q42:{}", hex32(root)),
        kernels,
        note: "Real-data ForgeRuntime diagnostic; timings are per-call diagnostics, not LLM throughput or an end-to-end application benchmark.".to_string(),
    })
}

#[command]
pub fn qapp_analyze(request: QappAnalysisRequest) -> Result<QappAnalysisResult, String> {
    // This boundary deliberately uses QualiaDB's emit-time graph Vecs/Q42 serialization.
    // The allocation is confined to the command surface, not a render/runtime hot path.
    let mut canonical = String::new();
    canonical.push_str(&request.discipline);

    let mut assertions = Vec::new();
    let mut scores = Vec::new();
    for (key, value) in &request.fields {
        if value.trim().is_empty() {
            continue;
        }
        canonical.push('|');
        canonical.push_str(key);
        canonical.push('=');
        canonical.push_str(value);
        assertions.push(format!(
            "{} :{} \"{}\" .",
            request.discipline,
            qapp_slug(key),
            value
        ));
        scores.push(qapp_evidence_score(key, value));
    }
    if !request.notes.trim().is_empty() {
        canonical.push_str("|notes=");
        canonical.push_str(request.notes.trim());
        assertions.push(format!(
            "{} :hasNote \"{}\" .",
            request.discipline,
            request.notes.trim()
        ));
        scores.push(qapp_evidence_score("notes", request.notes.trim()));
    }

    if scores.is_empty() {
        scores.push(qapp_evidence_score("empty", &request.discipline));
    }

    let graph = qapp_graph_for_scores(scores.len())?;
    let graph_nodes = graph.len();
    let evidence = qualia_core_db::wgsl_forge::graph_ops::executor::execute_graph_cpu(
        &graph,
        &[scores.clone()],
    )
    .map_err(|e| e.to_string())?
    .first()
    .copied()
    .unwrap_or_default();
    let mut quins =
        qualia_core_db::wgsl_forge::ir::serialize_graph(&graph).map_err(|e| e.to_string())?;
    quins.extend(qapp_content_quins(&request, &canonical, &scores));
    let merkle_root = qualia_core_db::wgsl_forge::ir::graph_merkle_root(&quins);

    Ok(QappAnalysisResult {
        summary: format!(
            "{} analysis derived {} assertion(s); Forge DAG reduced {} evidence channel(s) into q42 Merkle provenance.",
            request.discipline, assertions.len(), scores.len()
        ),
        assertions,
        provenance_hash: format!("q42:{}", hex32(merkle_root)),
        engine: format!(
            "qualia-core-db/{} forge-schema-{}",
            qualia_core_db::ENGINE_VERSION,
            qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION
        ),
        graph_nodes,
        q42_quins: quins.len(),
        evidence_weight: evidence,
        forge_schema_version: qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION,
    })
}

#[command]
pub async fn certify_forge_physics() -> Result<ForgePhysicsCertification, String> {
    tauri::async_runtime::spawn_blocking(|| build_forge_physics_certification(true))
        .await
        .map_err(|err| format!("Forge physics worker failed: {err}"))
}

#[command]
pub async fn run_forge_compute_probe() -> Result<ForgeComputeProbe, String> {
    tauri::async_runtime::spawn_blocking(build_forge_compute_probe)
        .await
        .map_err(|err| format!("Forge compute worker failed: {err}"))?
}

#[command]
pub fn get_qualia_compute_profile() -> QualiaComputeProfile {
    use qualia_core_db::gpu_context::{
        qualia_backend_override, recommend_inference_backend, shared_gpu,
    };
    use qualia_core_db::wgsl_forge::{
        resolve_execution_backend, TargetBackend, CUDARC_API_VERSION, FORGE_SCHEMA_VERSION,
        NAGA_API_VERSION, WGPU_API_VERSION,
    };
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let backend_override = qualia_backend_override().map(|backends| format!("{backends:?}"));
    let gpu = catch_unwind(AssertUnwindSafe(shared_gpu));

    match gpu {
        Ok(gpu) => {
            let caps = &gpu.adapter_caps;
            let preferred = match caps.backend_label() {
                "vulkan" => TargetBackend::Spirv,
                "dx12" => TargetBackend::Hlsl,
                "metal" => TargetBackend::Msl,
                _ => TargetBackend::Wgsl,
            };
            let (active, fallback_note) = resolve_execution_backend(preferred, |target| {
                matches!(
                    (caps.backend_label(), target),
                    ("vulkan", TargetBackend::Spirv)
                        | ("dx12", TargetBackend::Hlsl)
                        | ("metal", TargetBackend::Msl)
                )
            });

            QualiaComputeProfile {
                engine_version: qualia_core_db::ENGINE_VERSION.to_string(),
                forge_schema_version: FORGE_SCHEMA_VERSION,
                wgpu_api_version: WGPU_API_VERSION.to_string(),
                naga_api_version: NAGA_API_VERSION.to_string(),
                cudarc_api_version: CUDARC_API_VERSION.to_string(),
                backend_override,
                adapter_name: caps.name.clone(),
                backend: caps.backend_label().to_string(),
                device_type: caps.device_type_label().to_string(),
                vendor_hex: format!("0x{:04x}", caps.vendor),
                device_hex: format!("0x{:04x}", caps.device),
                driver: caps.driver.clone(),
                driver_info: caps.driver_info.clone(),
                recommendation: recommend_inference_backend(caps).to_string(),
                preferred_forge_target: format!("{preferred:?}"),
                active_forge_target: format!("{active:?}"),
                fallback_note,
                features: caps.features.compact_flags(),
                enabled_features: gpu.enabled_features.compact_flags(),
                subgroup_range: format!("{}..{}", caps.subgroup_min_size, caps.subgroup_max_size),
                cooperative_matrix_tile_count: caps.cooperative_matrix_tile_count,
                max_buffer_size_mib: caps.limits.max_buffer_size / (1024 * 1024),
                max_storage_buffer_binding_size_mib: caps.limits.max_storage_buffer_binding_size
                    / (1024 * 1024),
                max_compute_workgroup_storage_size: caps.limits.max_compute_workgroup_storage_size,
                max_compute_invocations_per_workgroup: caps
                    .limits
                    .max_compute_invocations_per_workgroup,
                max_compute_workgroup_size_x: caps.limits.max_compute_workgroup_size_x,
                max_compute_workgroups_per_dimension: caps
                    .limits
                    .max_compute_workgroups_per_dimension,
                timestamps_supported: gpu.timestamps_supported,
                timestamp_period_ns: gpu.timestamp_period_ns,
                q42_graph_bridge: true,
                available_modules: vec![
                    "forge_graph_cpu".to_string(),
                    "q42_graph_bridge".to_string(),
                    "physics_kinematics".to_string(),
                    "molecular_dynamics".to_string(),
                    "audio_stft".to_string(),
                    "audio_cqt".to_string(),
                    "audio_hrtf".to_string(),
                ],
            }
        }
        Err(_) => QualiaComputeProfile {
            engine_version: qualia_core_db::ENGINE_VERSION.to_string(),
            forge_schema_version: FORGE_SCHEMA_VERSION,
            wgpu_api_version: WGPU_API_VERSION.to_string(),
            naga_api_version: NAGA_API_VERSION.to_string(),
            cudarc_api_version: CUDARC_API_VERSION.to_string(),
            backend_override,
            adapter_name: "unavailable".to_string(),
            backend: "unavailable".to_string(),
            device_type: "unknown".to_string(),
            vendor_hex: "0x0000".to_string(),
            device_hex: "0x0000".to_string(),
            driver: "unavailable".to_string(),
            driver_info: "shared GPU initialization failed".to_string(),
            recommendation: "CPU/portable WGSL fallback until a wgpu adapter is available"
                .to_string(),
            preferred_forge_target: format!("{:?}", TargetBackend::Wgsl),
            active_forge_target: format!("{:?}", TargetBackend::Wgsl),
            fallback_note: Some(
                "shared GPU initialization failed; reporting portable Forge floor".to_string(),
            ),
            features: String::new(),
            enabled_features: String::new(),
            subgroup_range: "0..0".to_string(),
            cooperative_matrix_tile_count: 0,
            max_buffer_size_mib: 0,
            max_storage_buffer_binding_size_mib: 0,
            max_compute_workgroup_storage_size: 0,
            max_compute_invocations_per_workgroup: 0,
            max_compute_workgroup_size_x: 0,
            max_compute_workgroups_per_dimension: 0,
            timestamps_supported: false,
            timestamp_period_ns: 0.0,
            q42_graph_bridge: true,
            available_modules: vec![
                "forge_graph_cpu".to_string(),
                "q42_graph_bridge".to_string(),
            ],
        },
    }
}

#[cfg(test)]
mod qapp_analysis_tests {
    use super::*;

    #[test]
    fn qapp_analyze_is_deterministic_and_q42_addressed() {
        let request = QappAnalysisRequest {
            discipline: "Anatomy".to_string(),
            fields: vec![
                ("Structure".to_string(), "larynx".to_string()),
                ("Empty".to_string(), " ".to_string()),
                ("Frame".to_string(), "10D epithelial context".to_string()),
            ],
            notes: "preserve provenance through Forge graph bridge".to_string(),
        };

        let a = qapp_analyze(request.clone()).expect("analysis succeeds");
        let b = qapp_analyze(request).expect("analysis is repeatable");

        assert_eq!(a.provenance_hash, b.provenance_hash);
        assert!(a.provenance_hash.starts_with("q42:"));
        assert_eq!(a.provenance_hash.len(), 68);
        assert_eq!(a.graph_nodes, 1);
        assert!(a.q42_quins > a.assertions.len());
        assert_eq!(
            a.forge_schema_version,
            qualia_core_db::wgsl_forge::FORGE_SCHEMA_VERSION
        );
    }

    #[test]
    fn forge_physics_cpu_oracle_is_q42_addressed_and_bounded() {
        let result = build_forge_physics_certification(false);

        assert_eq!(result.backend, "cpu-oracle");
        assert!(!result.certified);
        assert_eq!(result.particle_count, 8);
        assert_eq!(result.sample_positions.len(), 4);
        assert!(result.momentum_drift.is_finite());
        assert!(result.q42_provenance.starts_with("q42:"));
        assert_eq!(result.q42_provenance.len(), 68);
    }

    #[test]
    #[ignore = "requires a WGPU adapter"]
    fn forge_physics_wgpu_matches_cpu_oracle() {
        let result = build_forge_physics_certification(true);

        assert_eq!(result.backend, "wgpu-forge", "{}", result.note);
        assert!(result.certified, "{}", result.note);
        assert!(result.max_abs_error <= 1.0e-3);
    }

    #[test]
    fn forge_probe_error_and_fingerprint_helpers_are_deterministic() {
        let actual = [1.0_f32, 2.25, -4.0, 8.0];
        let expected = [1.0_f32, 2.0, -4.5, 8.0];

        assert_eq!(max_abs_error(&actual, &expected), 0.5);
        assert_eq!(fingerprint_f32(&actual), fingerprint_f32(&actual));
        assert_ne!(fingerprint_f32(&actual), fingerprint_f32(&expected));
    }

    #[test]
    #[ignore = "requires a WGPU adapter"]
    fn forge_real_data_compute_probe_certifies() {
        let result = build_forge_compute_probe().expect("Forge compute probe");

        assert!(result.all_certified, "{:?}", result.kernels);
        assert_eq!(result.kernels.len(), 3);
        assert!(result.q42_provenance.starts_with("q42:"));
        assert_eq!(result.q42_provenance.len(), 68);
    }
}

#[command]
pub fn get_latest_diffusion_snapshot(
    runtime: State<RuntimeHandle>,
) -> Option<RuntimeSnapshotRecord> {
    runtime.latest_snapshot()
}

#[command]
pub fn reconfigure_diffusion(
    _runtime: State<RuntimeHandle>,
    _config: DiffusionConfigInput,
) -> Result<(), String> {
    Ok(())
}

#[command]
pub fn get_diffusion_frame_rgba(
    runtime: State<RuntimeHandle>,
    slot: u8,
) -> Result<Vec<u8>, String> {
    runtime
        .frame_rgba(slot)
        .ok_or_else(|| format!("diffusion frame slot {} is not available", slot))
}

#[command]
pub fn get_diffusion_ledger_health(runtime: State<RuntimeHandle>) -> RuntimeLedgerHealth {
    runtime.ledger_health()
}

#[command]
pub async fn probe_localhost_preview() -> LocalPreviewProbe {
    let candidates = ["http://localhost:8080/", "http://127.0.0.1:8080/"];

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(1200))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return LocalPreviewProbe {
                target_url: candidates[0].to_string(),
                reachable: false,
                status_code: None,
                detail: format!("probe client failed: {err}"),
            }
        }
    };

    let mut last_error = "preview endpoint did not respond".to_string();

    for candidate in candidates {
        match client.get(candidate).send().await {
            Ok(response) => {
                return LocalPreviewProbe {
                    target_url: candidate.to_string(),
                    reachable: true,
                    status_code: Some(response.status().as_u16()),
                    detail: "preview endpoint responded".to_string(),
                }
            }
            Err(err) => {
                last_error = err.to_string();
            }
        }
    }

    LocalPreviewProbe {
        target_url: candidates[0].to_string(),
        reachable: false,
        status_code: None,
        detail: last_error,
    }
}
