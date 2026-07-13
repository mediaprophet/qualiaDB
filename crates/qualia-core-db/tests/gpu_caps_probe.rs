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
    use wgpu::Features as F;
    let instance = wgpu::Instance::default();
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
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
    let chosen = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
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
                yn(f.contains(wgpu::Features::EXPERIMENTAL_COOPERATIVE_MATRIX)),
                yn(f.contains(wgpu::Features::SUBGROUP)),
                yn(f.contains(wgpu::Features::SHADER_F16))
            );
        }
        Err(e) => eprintln!("[caps] default adapter selection failed: {e}"),
    }
    eprintln!("════════════════════════════════════════════════════════════════════");
}
