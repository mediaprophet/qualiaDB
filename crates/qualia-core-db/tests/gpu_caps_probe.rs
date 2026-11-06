//! GPU performance-capability AUDIT — the comprehensive "what does this hardware + wgpu 29 give us
//! for decode throughput?" survey (TENSOR_CORES_DISCUSSION.md, "consider ALL hardware elements").
//!
//! Run:  cargo test -p qualia-core-db --test gpu_caps_probe -- --nocapture
//!
//! Reports, per enumerated adapter: backend, the matmul/decode-relevant wgpu-29 FEATURES
//! (cooperative-matrix, subgroups, f16, int64, timestamps, ray-query, pipeline-cache), the
//! subgroup size range, the cooperative-matrix tile descriptors the driver reports, and the
//! compute LIMITS that bound tile size / dispatch. This is "measure then route" input — it always
//! passes; it reports.

#![cfg(not(target_arch = "wasm32"))]

fn yn(b: bool) -> &'static str {
    if b {
        "YES"
    } else {
        "no"
    }
}

#[test]
fn gpu_caps_probe() {
    use etalon::Features as F;
    let instance = etalon::Instance::default();
    let adapters = pollster::block_on(instance.enumerate_adapters(etalon::Backends::all()));
    if adapters.is_empty() {
        eprintln!("[caps] no wgpu adapters enumerated on this host");
        return;
    }

    for adapter in &adapters {
        let info = adapter.get_info();
        let f = adapter.features();
        let lim = adapter.limits();
        let coop_tiles = adapter.cooperative_matrix_properties();

        eprintln!("════════════════════════════════════════════════════════════════════");
        eprintln!(
            "[caps] {} | {:?} | {:?}",
            info.name, info.backend, info.device_type
        );
        eprintln!("[caps] driver: {} {}", info.driver, info.driver_info);
        eprintln!(
            "[caps] subgroup size range: {}..{}",
            info.subgroup_min_size, info.subgroup_max_size
        );
        eprintln!("[caps] ── matmul / decode features ──");
        eprintln!(
            "[caps]   COOPERATIVE_MATRIX (tensor cores) : {}  ({} tiles)",
            yn(f.contains(F::EXPERIMENTAL_COOPERATIVE_MATRIX)),
            coop_tiles.len()
        );
        eprintln!(
            "[caps]   SUBGROUP (warp ops)               : {}",
            yn(f.contains(F::SUBGROUP))
        );
        eprintln!(
            "[caps]   SUBGROUP_BARRIER                  : {}",
            yn(f.contains(F::SUBGROUP_BARRIER))
        );
        eprintln!(
            "[caps]   SHADER_F16 (half precision)       : {}",
            yn(f.contains(F::SHADER_F16))
        );
        eprintln!(
            "[caps]   SHADER_INT64                      : {}",
            yn(f.contains(F::SHADER_INT64))
        );
        eprintln!("[caps] ── profiling / misc ──");
        eprintln!(
            "[caps]   TIMESTAMP_QUERY                   : {}",
            yn(f.contains(F::TIMESTAMP_QUERY))
        );
        eprintln!(
            "[caps]   TIMESTAMP_QUERY_INSIDE_PASSES     : {}",
            yn(f.contains(F::TIMESTAMP_QUERY_INSIDE_PASSES))
        );
        eprintln!(
            "[caps]   PIPELINE_STATISTICS_QUERY         : {}",
            yn(f.contains(F::PIPELINE_STATISTICS_QUERY))
        );
        eprintln!(
            "[caps]   PIPELINE_CACHE                    : {}",
            yn(f.contains(F::PIPELINE_CACHE))
        );
        eprintln!(
            "[caps]   EXPERIMENTAL_RAY_QUERY (RT cores) : {}",
            yn(f.contains(F::EXPERIMENTAL_RAY_QUERY))
        );
        eprintln!("[caps] ── compute limits (tile / dispatch bounds) ──");
        eprintln!(
            "[caps]   workgroup_storage_size            : {} bytes",
            lim.max_compute_workgroup_storage_size
        );
        eprintln!(
            "[caps]   invocations_per_workgroup         : {}",
            lim.max_compute_invocations_per_workgroup
        );
        eprintln!(
            "[caps]   workgroup_size_x                  : {}",
            lim.max_compute_workgroup_size_x
        );
        eprintln!(
            "[caps]   workgroups_per_dimension          : {}",
            lim.max_compute_workgroups_per_dimension
        );
        eprintln!(
            "[caps]   storage_buffer_binding_size       : {} MiB",
            lim.max_storage_buffer_binding_size / (1024 * 1024)
        );
        eprintln!(
            "[caps]   max_buffer_size                   : {} MiB",
            lim.max_buffer_size / (1024 * 1024)
        );
        for (i, t) in coop_tiles.iter().enumerate() {
            eprintln!(
                "[caps]   coop tile[{i}]: {}x{}x{} ab={:?} cr={:?}",
                t.m_size, t.n_size, t.k_size, t.ab_type, t.cr_type
            );
        }
    }
    eprintln!("════════════════════════════════════════════════════════════════════");

    // DECISIVE: what does the inference path ACTUALLY get? Replicate init_shared_gpu_async's
    // selection (Instance::default() + HighPerformance) and report the chosen backend + features.
    let chosen = pollster::block_on(instance.request_adapter(&etalon::RequestAdapterOptions {
        power_preference: etalon::PowerPreference::HighPerformance,
        ..Default::default()
    }));
    match chosen {
        Ok(a) => {
            let info = a.get_info();
            let f = a.features();
            eprintln!("[caps] *** DEFAULT inference adapter (HighPerformance) ***");
            eprintln!(
                "[caps]   {} | {:?} | {:?}",
                info.name, info.backend, info.device_type
            );
            eprintln!(
                "[caps]   coop_matrix={} subgroup={} f16={}  ← what decode runs on TODAY",
                yn(f.contains(etalon::Features::EXPERIMENTAL_COOPERATIVE_MATRIX)),
                yn(f.contains(etalon::Features::SUBGROUP)),
                yn(f.contains(etalon::Features::SHADER_F16))
            );
        }
        Err(e) => eprintln!("[caps] default adapter selection failed: {e}"),
    }
    eprintln!("════════════════════════════════════════════════════════════════════");
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DeviceQualificationOutcome {
    SoftwareWebGpuPass {
        adapter_name: &'static str,
        passed_checks: u32,
        gpu_qualified: bool,
        hardware_gpu_qualified: bool,
    },
    HardwareGpuPass {
        adapter_name: &'static str,
        backend: &'static str,
        passed_checks: u32,
        gpu_qualified: bool,
        hardware_gpu_qualified: bool,
    },
    CpuFallbackOnly {
        cpu_passed_checks: u32,
        gpu_unsupported_checks: u32,
        gpu_qualified: bool,
        hardware_gpu_qualified: bool,
    },
    BrowserRequestRejected {
        browser_error: String,
        gpu_qualified: bool,
        hardware_gpu_qualified: bool,
    },
}

fn classify_device_qualification(
    adapter_available: bool,
    is_software_adapter: bool,
    adapter_name: &'static str,
    backend: &'static str,
    passed_checks: u32,
    gpu_unsupported_checks: u32,
    browser_error: Option<&str>,
) -> DeviceQualificationOutcome {
    if let Some(err) = browser_error {
        return DeviceQualificationOutcome::BrowserRequestRejected {
            browser_error: err.to_string(),
            gpu_qualified: false,
            hardware_gpu_qualified: false,
        };
    }
    if !adapter_available || gpu_unsupported_checks > 0 {
        return DeviceQualificationOutcome::CpuFallbackOnly {
            cpu_passed_checks: passed_checks,
            gpu_unsupported_checks,
            gpu_qualified: false,
            hardware_gpu_qualified: false,
        };
    }
    if is_software_adapter {
        DeviceQualificationOutcome::SoftwareWebGpuPass {
            adapter_name,
            passed_checks,
            gpu_qualified: true,
            hardware_gpu_qualified: false,
        }
    } else {
        DeviceQualificationOutcome::HardwareGpuPass {
            adapter_name,
            backend,
            passed_checks,
            gpu_qualified: true,
            hardware_gpu_qualified: true,
        }
    }
}

#[test]
fn browser_and_device_qualification_matrix_preserves_honest_fallback_states() {
    use qualia_core_db::{q_hash, NQuin};

    // 1. Software WebGPU pass (e.g. SwiftShader / llvmpipe): WebGPU works, but hardware_gpu_qualified is false.
    let sw = classify_device_qualification(true, true, "SwiftShader", "vulkan", 13, 0, None);
    assert_eq!(
        sw,
        DeviceQualificationOutcome::SoftwareWebGpuPass {
            adapter_name: "SwiftShader",
            passed_checks: 13,
            gpu_qualified: true,
            hardware_gpu_qualified: false,
        }
    );

    // 2. Hardware GPU pass (e.g. Vulkan / Metal / DX12 discrete or integrated GPU).
    let hw = classify_device_qualification(
        true,
        false,
        "Apple M3 Max",
        "metal",
        13,
        0,
        None,
    );
    assert_eq!(
        hw,
        DeviceQualificationOutcome::HardwareGpuPass {
            adapter_name: "Apple M3 Max",
            backend: "metal",
            passed_checks: 13,
            gpu_qualified: true,
            hardware_gpu_qualified: true,
        }
    );

    // 3. CPU fallback only (2 CPU pass / 11 GPU unsupported -> gpu_qualified == false).
    let cpu_only = classify_device_qualification(false, false, "none", "none", 2, 11, None);
    assert_eq!(
        cpu_only,
        DeviceQualificationOutcome::CpuFallbackOnly {
            cpu_passed_checks: 2,
            gpu_unsupported_checks: 11,
            gpu_qualified: false,
            hardware_gpu_qualified: false,
        }
    );

    // 4. Browser request rejected (preserves exact browser error string and marks gpu_qualified == false).
    let rejected_msg = "NotAllowedError: navigator.gpu.requestAdapter rejected by browser policy";
    let rejected = classify_device_qualification(
        false,
        false,
        "none",
        "wasm-webgpu",
        0,
        11,
        Some(rejected_msg),
    );
    match &rejected {
        DeviceQualificationOutcome::BrowserRequestRejected {
            browser_error,
            gpu_qualified,
            hardware_gpu_qualified,
        } => {
            assert_eq!(browser_error, rejected_msg);
            assert!(!gpu_qualified);
            assert!(!hardware_gpu_qualified);
        }
        other => panic!("expected BrowserRequestRejected, got {other:?}"),
    }

    // Emit canonical 5-field XOR parity audit NQuin recording the qualification state.
    let s = q_hash("urn:qualia:etalon:qualification:device");
    let p = q_hash("urn:qualia:etalon:qualification:cpuFallbackOnly");
    let o = q_hash(rejected_msg);
    let c = q_hash("urn:qualia:idf:plane:Claim");
    let m = (2u64 << 32) | 11u64;
    let audit_quin = NQuin {
        subject: s,
        predicate: p,
        object: o,
        context: c,
        metadata: m,
        parity: NQuin::calculate_parity(s, p, o, c, m),
    };
    assert!(audit_quin.verify_ecc_parity());
}
