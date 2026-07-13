//! Per-kernel GPU timing for the native LLM forward/decode path (W2 / D17).
//!
//! Wraps each LLM compute pass with `TIMESTAMP_QUERY` writes, resolves the query set,
//! and accumulates GPU-internal nanoseconds per [`Phase`]. Gated by a runtime flag
//! (`QUALIA_LLM_GPU_PROFILE=1` or [`set_enabled`]) so production decode pays nothing:
//! when disabled, the `pass_writes_*` helpers return `None` and the pass is byte-identical
//! to before (`timestamp_writes: None`).
//!
//! Requires the shared device to have negotiated `TIMESTAMP_QUERY`
//! (see [`crate::gpu_context::SharedGpuContext::timestamps_supported`]); degrades to a
//! no-op otherwise, so it is safe to call unconditionally from every dispatch site.
//!
//! **Honesty note:** the per-phase nanoseconds are GPU-internal (begin→end of the pass) and
//! individually accurate. A *profiling* run serialises per-op readback, so the headline
//! tok/s of a profiled run is perturbed — report the per-phase split, not the profiled tok/s.
//!
//! Not re-entrant: it uses one shared 2-slot query set, which the single LLM engine thread's
//! per-op blocking readback serialises. That matches how decode actually runs.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Distinct kernel families on the LLM forward/decode path.
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Quantized token-embedding lookup/dequant.
    Embedding = 0,
    /// Projection + FFN matmuls (incl. the ternary-FFN dispatch).
    Gemm = 1,
    /// Fused attention (Q·K, softmax, ·V).
    Attention = 2,
    /// Single-pass fused transformer-block shader.
    FusedBlock = 3,
    /// lm_head logits + top-k / argmax sampling.
    OutputTopk = 4,
}

impl Phase {
    pub const COUNT: usize = 5;
    pub const ALL: [Phase; Self::COUNT] = [
        Phase::Embedding,
        Phase::Gemm,
        Phase::Attention,
        Phase::FusedBlock,
        Phase::OutputTopk,
    ];

    #[inline]
    pub fn label(self) -> &'static str {
        match self {
            Phase::Embedding => "embedding",
            Phase::Gemm => "gemm",
            Phase::Attention => "attention",
            Phase::FusedBlock => "fused_block",
            Phase::OutputTopk => "output_topk",
        }
    }
}

static ENABLED: AtomicBool = AtomicBool::new(false);
static ACC_NS: [AtomicU64; Phase::COUNT] = [const { AtomicU64::new(0) }; Phase::COUNT];
static ACC_CALLS: [AtomicU64; Phase::COUNT] = [const { AtomicU64::new(0) }; Phase::COUNT];

/// Force GPU pass profiling on/off at runtime (bench-only; production leaves it off).
#[inline]
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

/// One-shot `QUALIA_LLM_GPU_PROFILE` env opt-in (cached; no per-call allocation).
fn env_opt_in() -> bool {
    static ENV: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENV.get_or_init(|| {
        std::env::var("QUALIA_LLM_GPU_PROFILE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    })
}

/// True when profiling is requested AND the shared device negotiated `TIMESTAMP_QUERY`.
#[inline]
pub fn enabled() -> bool {
    if !(ENABLED.load(Ordering::Relaxed) || env_opt_in()) {
        return false;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::gpu_context::shared_gpu().timestamps_supported
    }
    #[cfg(target_arch = "wasm32")]
    {
        false
    }
}

// ── Native timestamp resources + accumulation ─────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
struct TsResources {
    qset: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    staging: wgpu::Buffer,
}

#[cfg(not(target_arch = "wasm32"))]
const TS_BYTES: u64 = 2 * std::mem::size_of::<u64>() as u64; // begin + end

#[cfg(not(target_arch = "wasm32"))]
fn resources() -> &'static TsResources {
    use std::sync::OnceLock;
    static R: OnceLock<TsResources> = OnceLock::new();
    R.get_or_init(|| {
        let device = &crate::gpu_context::shared_gpu().device;
        let qset = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("llm-ts-qset"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("llm-ts-resolve"),
            size: TS_BYTES,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("llm-ts-staging"),
            size: TS_BYTES,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        TsResources {
            qset,
            resolve,
            staging,
        }
    })
}

/// `timestamp_writes` for a single-pass kernel (writes both begin and end on this pass).
/// Returns `None` (zero overhead) when profiling is off — drop straight into the descriptor.
#[cfg(not(target_arch = "wasm32"))]
pub fn pass_writes_both() -> Option<wgpu::ComputePassTimestampWrites<'static>> {
    if !enabled() {
        return None;
    }
    Some(wgpu::ComputePassTimestampWrites {
        query_set: &resources().qset,
        beginning_of_pass_write_index: Some(0),
        end_of_pass_write_index: Some(1),
    })
}

/// `timestamp_writes` for the FIRST pass of a multi-pass kernel (begin only).
#[cfg(not(target_arch = "wasm32"))]
pub fn pass_writes_begin() -> Option<wgpu::ComputePassTimestampWrites<'static>> {
    if !enabled() {
        return None;
    }
    Some(wgpu::ComputePassTimestampWrites {
        query_set: &resources().qset,
        beginning_of_pass_write_index: Some(0),
        end_of_pass_write_index: None,
    })
}

/// `timestamp_writes` for the LAST pass of a multi-pass kernel (end only).
#[cfg(not(target_arch = "wasm32"))]
pub fn pass_writes_end() -> Option<wgpu::ComputePassTimestampWrites<'static>> {
    if !enabled() {
        return None;
    }
    Some(wgpu::ComputePassTimestampWrites {
        query_set: &resources().qset,
        beginning_of_pass_write_index: None,
        end_of_pass_write_index: Some(1),
    })
}

/// Encode the query-set resolve into `encoder` (call after the pass, before `finish()`).
/// No-op when profiling is off.
#[cfg(not(target_arch = "wasm32"))]
pub fn resolve(encoder: &mut wgpu::CommandEncoder) {
    if !enabled() {
        return;
    }
    let r = resources();
    encoder.resolve_query_set(&r.qset, 0..2, &r.resolve, 0);
    encoder.copy_buffer_to_buffer(&r.resolve, 0, &r.staging, 0, TS_BYTES);
}

/// Read the resolved timestamps and add `end - begin` ns to `phase`'s accumulator.
/// Call after the kernel's submit + device poll. No-op when profiling is off.
#[cfg(not(target_arch = "wasm32"))]
pub fn accumulate(phase: Phase) {
    if !enabled() {
        return;
    }
    let ctx = crate::gpu_context::shared_gpu();
    let r = resources();
    let slice = r.staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res);
    });
    // poll(Wait) blocks until the GPU is done and drives the map callback.
    let _ = ctx.device.poll(wgpu::PollType::wait_indefinitely());
    if rx.recv().map(|r| r.is_ok()).unwrap_or(false) {
        let data = slice.get_mapped_range().expect("wgpu buffer map_range failed");
        let ts: &[u64] = bytemuck::cast_slice(&data);
        if ts.len() >= 2 {
            let ticks = ts[1].saturating_sub(ts[0]);
            let ns = (ticks as f64 * ctx.timestamp_period_ns as f64) as u64;
            ACC_NS[phase as usize].fetch_add(ns, Ordering::Relaxed);
            ACC_CALLS[phase as usize].fetch_add(1, Ordering::Relaxed);
        }
        drop(data);
    }
    r.staging.unmap();
}

#[cfg(target_arch = "wasm32")]
pub fn pass_writes_both() -> Option<wgpu::ComputePassTimestampWrites<'static>> {
    None
}
#[cfg(target_arch = "wasm32")]
pub fn pass_writes_begin() -> Option<wgpu::ComputePassTimestampWrites<'static>> {
    None
}
#[cfg(target_arch = "wasm32")]
pub fn pass_writes_end() -> Option<wgpu::ComputePassTimestampWrites<'static>> {
    None
}
#[cfg(target_arch = "wasm32")]
pub fn resolve(_encoder: &mut wgpu::CommandEncoder) {}
#[cfg(target_arch = "wasm32")]
pub fn accumulate(_phase: Phase) {}

// ── Snapshot / reset for the bench ────────────────────────────────────────────

/// Accumulated GPU time for one phase across all dispatches since the last [`reset`].
#[derive(Debug, Clone, Copy)]
pub struct PhaseTiming {
    pub phase: Phase,
    pub total_ns: u64,
    pub calls: u64,
}

impl PhaseTiming {
    #[inline]
    pub fn micros(&self) -> f64 {
        self.total_ns as f64 / 1000.0
    }
}

/// Snapshot all phase accumulators (for the bench JSON).
pub fn snapshot() -> [PhaseTiming; Phase::COUNT] {
    let mut out = [PhaseTiming {
        phase: Phase::Embedding,
        total_ns: 0,
        calls: 0,
    }; Phase::COUNT];
    for (i, p) in Phase::ALL.iter().enumerate() {
        out[i] = PhaseTiming {
            phase: *p,
            total_ns: ACC_NS[i].load(Ordering::Relaxed),
            calls: ACC_CALLS[i].load(Ordering::Relaxed),
        };
    }
    out
}

/// Zero all phase accumulators (call before a measured run).
pub fn reset() {
    for i in 0..Phase::COUNT {
        ACC_NS[i].store(0, Ordering::Relaxed);
        ACC_CALLS[i].store(0, Ordering::Relaxed);
    }
}

/// True if any phase recorded GPU time since the last reset (test/diagnostic helper).
pub fn any_recorded() -> bool {
    (0..Phase::COUNT).any(|i| ACC_CALLS[i].load(Ordering::Relaxed) > 0)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    // Trivial compute kernel with enough arithmetic to register measurable GPU time.
    const SELF_TEST_WGSL: &str = r#"
@group(0) @binding(0) var<storage, read_write> data: array<u32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i < arrayLength(&data)) {
        var acc = data[i];
        for (var k: u32 = 0u; k < 4096u; k = k + 1u) {
            acc = acc * 1664525u + 1013904223u;
        }
        data[i] = acc;
    }
}
"#;

    /// Proves the end-to-end timestamp path on real hardware: enable → wrap a real
    /// compute pass → resolve → accumulate → assert non-zero GPU time. Named with `gpu`
    /// so `--skip gpu` excludes it on adapters/CI without a timestamp-capable device.
    #[test]
    fn gpu_timestamp_self_test_records_nonzero() {
        let ctx = crate::gpu_context::shared_gpu();
        if !ctx.timestamps_supported {
            eprintln!("SKIP gpu_timestamp_self_test: adapter has no TIMESTAMP_QUERY");
            return;
        }
        let device = &ctx.device;
        let queue = &ctx.queue;

        set_enabled(true);
        reset();

        let n: u64 = 65_536;
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("prof-self-test-buf"),
            size: n * 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("prof-self-test-shader"),
            source: wgpu::ShaderSource::Wgsl(SELF_TEST_WGSL.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("prof-self-test-pipe"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let bgl = pipeline.get_bind_group_layout(0);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("prof-self-test-bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buf.as_entire_binding(),
            }],
        });

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("prof-self-test-pass"),
                timestamp_writes: pass_writes_both(),
            });
            cpass.set_pipeline(&pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups((n as u32 + 63) / 64, 1, 1);
        }
        resolve(&mut encoder);
        queue.submit(Some(encoder.finish()));
        accumulate(Phase::Gemm);

        assert!(
            any_recorded(),
            "profiler recorded no timestamps on a TIMESTAMP_QUERY device"
        );
        let snap = snapshot();
        let gemm = snap
            .iter()
            .find(|t| matches!(t.phase, Phase::Gemm))
            .copied()
            .expect("gemm phase present");
        eprintln!(
            "gpu_timestamp_self_test: gemm pass = {:.3} µs ({} ns, {} call)",
            gemm.micros(),
            gemm.total_ns,
            gemm.calls
        );
        assert_eq!(gemm.calls, 1, "expected exactly one recorded pass");
        assert!(
            gemm.total_ns > 0,
            "expected non-zero GPU time for a 4096-iter kernel"
        );

        set_enabled(false);
    }

    #[test]
    fn disabled_profiler_is_noop() {
        set_enabled(false);
        reset();
        // With profiling off, the pass-writes helpers must return None (zero overhead).
        assert!(pass_writes_both().is_none());
        assert!(pass_writes_begin().is_none());
        assert!(pass_writes_end().is_none());
        accumulate(Phase::Attention);
        assert!(!any_recorded());
    }
}
