//! Consumer-facing runtime for the certified WGSL Forge.
//!
//! Everything else in [`crate::wgsl_forge`] is about *proving* a kernel correct
//! (the differential oracle in [`oracle`](crate::wgsl_forge::oracle)) and *tuning*
//! its schedule (`tune` + the topology-keyed [`ManifestCache`]). That machinery
//! always runs against deterministic, seed-derived test vectors so the GPU result
//! can be checked bit-for-bit against a CPU reference.
//!
//! [`ForgeRuntime`] is the other half: once a kernel is certified, other modules
//! need to run it on **their own real data**, with no oracle, no comparison, and
//! no test-vector generation — just "feed my numbers in, run the auto-tuned
//! schedule, give me the answer back". Each typed method wires its GPU buffers
//! *identically* to the matching `evaluate_*` in [`oracle`](crate::wgsl_forge::oracle)
//! (same bindings, [`BindingUsage`]s, `*Params` uniform blocks, dispatch
//! `element_count`, and output sizing) so the runtime path is the certified path —
//! the only difference is the source of the input bytes and the absence of the
//! CPU check.
//!
//! The dispatch schedule comes from [`ForgeRuntime::tuned_schedule`]: when a
//! [`ManifestCache`] is attached and holds a tuning record for this hardware
//! topology, the cached winner is used; otherwise a documented per-kernel default
//! is used. Populate the cache for this machine with the CLI's `shader
//! auto-tune-all` (it tunes every built-in and writes a topology-keyed
//! [`TuningManifest`] per kernel).

use std::path::PathBuf;

use super::execute::{BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline};
use super::oracle::{GemmParams, TERNARY_CODES_PER_WORD, TernaryGemvParams, TopKParams};
use super::{
    BuiltinKernel, ForgeError, ManifestCache, Schedule, TargetBackend, emit_shader, validate_wgsl,
};

/// A ready-to-use handle for running certified forge kernels on real data.
///
/// Owns the GPU compute context (one device/queue/slab pair) and, optionally, a
/// [`ManifestCache`] directory plus this machine's topology hash so tuned
/// schedules can be looked up. Construct once and reuse across many calls; each
/// `topk` / `ternary_gemv` / `p64_project` call allocates transiently, dispatches,
/// reads back, and frees its transient allocations.
pub struct ForgeRuntime {
    context: WgpuComputeContext,
    /// Optional tuned-schedule source. When present, [`Self::tuned_schedule`]
    /// looks up `(topology_hash, kernel)` and returns the cached winner.
    cache: Option<ManifestCache>,
    /// Stable fingerprint of this adapter/topology, used to key cache lookups.
    /// Computed once at construction from the context's [`HardwareProfile`].
    topology_hash: Option<String>,
}

impl ForgeRuntime {
    /// Build the GPU context, optionally attaching a manifest-cache directory for
    /// tuned schedules.
    ///
    /// `capacity_bytes` sizes the device slab (inputs + outputs must fit within it
    /// per call). When `cache_dir` is `Some`, [`Self::tuned_schedule`] will consult
    /// the cache; when it is `None`, every kernel uses its documented default
    /// schedule. The topology hash used for cache keys is derived from the live
    /// adapter, so a cache produced on different hardware is simply never matched
    /// (it is not an error).
    ///
    /// # Example
    /// ```no_run
    /// # use qualia_core_db::wgsl_forge::ForgeRuntime;
    /// let mut rt = ForgeRuntime::new(64 * 1024 * 1024, None)?;
    /// let top = rt.topk(&[3.0, 1.0, 2.0, 0.5], 2)?; // largest-2 per block
    /// # Ok::<(), qualia_core_db::wgsl_forge::ForgeError>(())
    /// ```
    pub fn new(capacity_bytes: usize, cache_dir: Option<PathBuf>) -> Result<Self, ForgeError> {
        let context = WgpuComputeContext::new(capacity_bytes)?;
        // The topology hash pins cache reuse to this exact adapter/topology
        // (plan §8). If it cannot be computed we simply fall back to defaults
        // rather than failing construction.
        let topology_hash = context.profile.topology_hash().ok();
        let cache = cache_dir.map(ManifestCache::new);
        Ok(Self {
            context,
            cache,
            topology_hash,
        })
    }

    /// The tuned [`Schedule`] for `builtin` on this hardware.
    ///
    /// If a cache is attached and holds a [`TuningManifest`] for
    /// `(topology_hash, builtin)`, the winning schedule from that record is
    /// returned. Otherwise — no cache, no topology hash, no record for this
    /// kernel, or a cache read error — the per-kernel default is returned.
    ///
    /// **Default policy** (matches what the `evaluate_*` oracle paths use):
    /// every built-in defaults to `Schedule { workgroup_size: 64, .. }`
    /// (`items_per_invocation = 1`, `vector_width = 1`). For top-k that 64 is also
    /// the per-block size (`block_size == workgroup_size`), so the default top-k
    /// processes the input in 64-element blocks. Populate the cache for this
    /// machine with `shader auto-tune-all`.
    pub fn tuned_schedule(&self, builtin: BuiltinKernel) -> Schedule {
        Self::lookup_tuned_schedule(self.cache.as_ref(), self.topology_hash.as_deref(), builtin)
    }

    /// Cache-lookup core of [`Self::tuned_schedule`], split out so the
    /// default-path policy is unit-testable without a GPU context. Returns the
    /// cached winner when `(cache, topology_hash)` are both present and a record
    /// exists; otherwise the documented per-kernel default.
    fn lookup_tuned_schedule(
        cache: Option<&ManifestCache>,
        topology_hash: Option<&str>,
        builtin: BuiltinKernel,
    ) -> Schedule {
        if let (Some(cache), Some(topology_hash)) = (cache, topology_hash) {
            if let Ok(Some(manifest)) =
                cache.load_tuning_for_topology(topology_hash, builtin.name())
            {
                return manifest.result.winner.schedule;
            }
        }
        Self::default_schedule(builtin)
    }

    /// The documented per-kernel default schedule. Every built-in uses
    /// `workgroup_size = 64` (the value the `evaluate_*` oracle paths default to;
    /// for top-k it doubles as the per-block size). Kept as one place so the
    /// policy is stated once.
    fn default_schedule(_builtin: BuiltinKernel) -> Schedule {
        Schedule {
            workgroup_size: 64,
            ..Default::default()
        }
    }

    /// Real-data per-block top-k: returns, for each `block_size`-element block of
    /// `input` (with `block_size = tuned_schedule.workgroup_size`), the `k` largest
    /// values in descending order, concatenated block-by-block.
    ///
    /// The tail block (when `input.len()` is not a multiple of `block_size`) is
    /// padded with `f32::MIN` by the kernel, exactly as the certified path does, so
    /// short blocks still emit `k` values (the padding sentinels sort last).
    ///
    /// Buffer wiring is identical to [`evaluate_topk`](super::oracle::evaluate_topk):
    /// binding 0 = `input` (storage-read), binding 1 = `output` (storage-read-write,
    /// `num_blocks * k` f32s), binding 2 = [`TopKParams`] (uniform); dispatch
    /// `element_count = input.len()`. The CALLER's `input` is fed directly — no
    /// oracle, no test vectors, no comparison.
    ///
    /// # Example
    /// ```no_run
    /// # use qualia_core_db::wgsl_forge::ForgeRuntime;
    /// # let mut rt = ForgeRuntime::new(1 << 20, None)?;
    /// let top2 = rt.topk(&[5.0, 1.0, 9.0, 2.0], 2)?; // one 4-elem tail block -> [9.0, 5.0]
    /// # Ok::<(), qualia_core_db::wgsl_forge::ForgeError>(())
    /// ```
    pub fn topk(&mut self, input: &[f32], k: usize) -> Result<Vec<f32>, ForgeError> {
        let schedule = self.tuned_schedule(BuiltinKernel::TopK);
        let block_size = schedule.workgroup_size as usize;
        let length = input.len();
        if length == 0 {
            return Err(ForgeError::GpuValidation(
                "topk input must be non-empty".to_string(),
            ));
        }
        if k == 0 || k > block_size {
            return Err(ForgeError::GpuValidation(format!(
                "k must be in 1..=block_size ({block_size}); got {k}"
            )));
        }

        let kernel = BuiltinKernel::TopK.spec();
        schedule.validate(&kernel, &self.context.constraints)?;
        self.context.constraints.supports_kernel(&kernel)?;
        let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
        validate_wgsl(&generated.source)?;

        // Output sizing mirrors evaluate_topk: one (k)-tuple per block.
        let num_blocks = length.div_ceil(block_size.max(1));
        let output_len = num_blocks * k;

        let input_bytes = bytemuck::cast_slice(input);
        let view_input =
            self.context
                .allocate_and_write(input_bytes, 0, 0, BindingUsage::StorageRead)?;
        let output_bytes_len = (output_len * size_of::<f32>()).max(4);
        let view_output =
            self.context
                .allocate_transient(output_bytes_len, 1, 0, BindingUsage::StorageReadWrite)?;
        let params = TopKParams {
            length: length as u32,
            k: k as u32,
            block_size: block_size as u32,
            _pad: 0,
        };
        let view_params = self.context.allocate_and_write(
            bytemuck::bytes_of(&params),
            2,
            0,
            BindingUsage::Uniform,
        )?;

        let buffers = vec![view_input, view_output, view_params];
        let pipeline = WgpuPipeline::compile(&self.context, &generated.source, &kernel.entry_point)?;
        pipeline.dispatch(&buffers, &schedule, length)?;
        let mut out = self.context.read_buffer_f32(&view_output)?;
        out.truncate(output_len);

        drop(pipeline);
        self.context.clear_transient_allocations();
        Ok(out)
    }

    /// Real-data ternary (BitNet-style) GEMV with on-the-fly dequant:
    /// `out[o] = scale[o] * sum_{i<k} ternary(w[o][i]) * x[i]` for `m` output rows.
    ///
    /// `packed_w` holds the 2-bit ternary codes, 16 codes per `u32`
    /// (`0 -> 0.0, 1 -> +1.0, 2 -> -1.0`, code `3` unused), laid out as `m` rows of
    /// `ceil(k / 16)` words each, row-major. `scale` has length `m`, `x` length `k`.
    ///
    /// Buffer wiring is identical to
    /// [`evaluate_ternary_gemv`](super::oracle::evaluate_ternary_gemv): binding 0 =
    /// `x`, 1 = `w_packed`, 2 = `scale` (all storage-read), 3 = `output`
    /// (storage-read-write, `m` f32s), 4 = [`TernaryGemvParams`] (uniform); dispatch
    /// `element_count = m`. The CALLER's tensors are fed directly — no oracle.
    ///
    /// # Example
    /// ```no_run
    /// # use qualia_core_db::wgsl_forge::ForgeRuntime;
    /// # let mut rt = ForgeRuntime::new(1 << 20, None)?;
    /// // 2 rows x 4 cols, codes packed low-to-high (1=+1, 2=-1): row0=+1,+1,+1,+1; row1=-1,-1,-1,-1
    /// let out = rt.ternary_gemv(&[1.0, 2.0, 3.0, 4.0], &[0x55, 0xAA], &[2.0, 10.0], 2, 4)?;
    /// // -> [2*(1+2+3+4), 10*(-1-2-3-4)] = [20.0, -100.0]
    /// # Ok::<(), qualia_core_db::wgsl_forge::ForgeError>(())
    /// ```
    pub fn ternary_gemv(
        &mut self,
        x: &[f32],
        packed_w: &[u32],
        scale: &[f32],
        m: usize,
        k: usize,
    ) -> Result<Vec<f32>, ForgeError> {
        if m == 0 || k == 0 {
            return Err(ForgeError::GpuValidation(
                "ternary_gemv requires m > 0 and k > 0".to_string(),
            ));
        }
        let k_words = k.div_ceil(TERNARY_CODES_PER_WORD);
        if x.len() < k {
            return Err(ForgeError::GpuValidation(format!(
                "x must have at least k = {k} elements; got {}",
                x.len()
            )));
        }
        if scale.len() < m {
            return Err(ForgeError::GpuValidation(format!(
                "scale must have at least m = {m} elements; got {}",
                scale.len()
            )));
        }
        if packed_w.len() < m * k_words {
            return Err(ForgeError::GpuValidation(format!(
                "packed_w must have at least m*ceil(k/16) = {} words; got {}",
                m * k_words,
                packed_w.len()
            )));
        }

        let schedule = self.tuned_schedule(BuiltinKernel::TernaryGemv);
        let kernel = BuiltinKernel::TernaryGemv.spec();
        schedule.validate(&kernel, &self.context.constraints)?;
        self.context.constraints.supports_kernel(&kernel)?;
        let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
        validate_wgsl(&generated.source)?;

        let view_x =
            self.context
                .allocate_and_write(bytemuck::cast_slice(x), 0, 0, BindingUsage::StorageRead)?;
        let view_w = self.context.allocate_and_write(
            bytemuck::cast_slice(packed_w),
            1,
            0,
            BindingUsage::StorageRead,
        )?;
        let view_scale = self.context.allocate_and_write(
            bytemuck::cast_slice(scale),
            2,
            0,
            BindingUsage::StorageRead,
        )?;
        let output_bytes_len = (m * size_of::<f32>()).max(4);
        let view_output =
            self.context
                .allocate_transient(output_bytes_len, 3, 0, BindingUsage::StorageReadWrite)?;
        let params = TernaryGemvParams {
            m: m as u32,
            k: k as u32,
            k_words: k_words as u32,
            _pad: 0,
        };
        let view_params = self.context.allocate_and_write(
            bytemuck::bytes_of(&params),
            4,
            0,
            BindingUsage::Uniform,
        )?;

        let buffers = vec![view_x, view_w, view_scale, view_output, view_params];
        let pipeline = WgpuPipeline::compile(&self.context, &generated.source, &kernel.entry_point)?;
        pipeline.dispatch(&buffers, &schedule, m)?;
        let mut out = self.context.read_buffer_f32(&view_output)?;
        out.truncate(m);

        drop(pipeline);
        self.context.clear_transient_allocations();
        Ok(out)
    }

    /// Real-data P64 projection: `out[r] = sum_{w<16} weights[w] * f32(p64[r].word[w])`
    /// for `record_count` records.
    ///
    /// `records_bytes` is the packed P64 GPU words exactly as
    /// [`evaluate_p64`](super::oracle::evaluate_p64) lays them out — a contiguous
    /// array of [`P64GpuWords64`](super::P64GpuWords64) (64 bytes / record, 16 `u32`
    /// words / record), so `records_bytes.len()` must be `record_count * 64`.
    /// `weights` has length 16.
    ///
    /// Buffer wiring is identical to `evaluate_p64`: binding 0 = `input` (P64
    /// records, storage-read), 1 = `weights` (storage-read), 2 = `output`
    /// (storage-read-write, `record_count` f32s); dispatch
    /// `element_count = record_count`. The CALLER's records are fed directly — no
    /// oracle.
    ///
    /// # Example
    /// ```no_run
    /// # use qualia_core_db::wgsl_forge::{ForgeRuntime, P64GpuWords64};
    /// # let mut rt = ForgeRuntime::new(1 << 20, None)?;
    /// let recs = [P64GpuWords64::from_u64_fields([1, 0, 0, 0, 0, 0, 0, 0])];
    /// let bytes: &[u8] = bytemuck::cast_slice(&recs);
    /// let weights = [1.0f32; 16];
    /// let out = rt.p64_project(bytes, &weights, 1)?; // out[0] = word[0] = 1.0
    /// # Ok::<(), qualia_core_db::wgsl_forge::ForgeError>(())
    /// ```
    pub fn p64_project(
        &mut self,
        records_bytes: &[u8],
        weights: &[f32],
        record_count: usize,
    ) -> Result<Vec<f32>, ForgeError> {
        if record_count == 0 {
            return Err(ForgeError::GpuValidation(
                "p64_project requires record_count > 0".to_string(),
            ));
        }
        // 64 bytes (16 u32 words) per record, matching P64GpuWords64.
        const RECORD_BYTES: usize = size_of::<super::P64GpuWords64>();
        if records_bytes.len() != record_count * RECORD_BYTES {
            return Err(ForgeError::GpuValidation(format!(
                "records_bytes must be record_count*{RECORD_BYTES} = {} bytes; got {}",
                record_count * RECORD_BYTES,
                records_bytes.len()
            )));
        }
        if weights.len() < 16 {
            return Err(ForgeError::GpuValidation(format!(
                "weights must have at least 16 elements; got {}",
                weights.len()
            )));
        }

        let schedule = self.tuned_schedule(BuiltinKernel::P64Project);
        let kernel = BuiltinKernel::P64Project.spec();
        schedule.validate(&kernel, &self.context.constraints)?;
        self.context.constraints.supports_kernel(&kernel)?;
        let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
        validate_wgsl(&generated.source)?;

        let view_input =
            self.context
                .allocate_and_write(records_bytes, 0, 0, BindingUsage::StorageRead)?;
        let view_weights = self.context.allocate_and_write(
            bytemuck::cast_slice(&weights[..16]),
            1,
            0,
            BindingUsage::StorageRead,
        )?;
        let output_bytes_len = (record_count * size_of::<f32>()).max(4);
        let view_output =
            self.context
                .allocate_transient(output_bytes_len, 2, 0, BindingUsage::StorageReadWrite)?;

        let buffers = vec![view_input, view_weights, view_output];
        let pipeline = WgpuPipeline::compile(&self.context, &generated.source, &kernel.entry_point)?;
        pipeline.dispatch(&buffers, &schedule, record_count)?;
        let mut out = self.context.read_buffer_f32(&view_output)?;
        out.truncate(record_count);

        drop(pipeline);
        self.context.clear_transient_allocations();
        Ok(out)
    }

    /// Real-data dense GEMM: row-major `C[M×N] = A[M×K] · B[K×N]`, all f32, i.e.
    /// `C[i][j] = sum_{k<K} a[i*K + k] * b[k*N + j]`, for `m * n` output elements.
    ///
    /// `a` must have `m * k` elements and `b` must have `k * n` elements, both
    /// row-major. The returned vector has `m * n` elements, row-major.
    ///
    /// Buffer wiring is identical to [`evaluate_gemm`](super::oracle::evaluate_gemm):
    /// binding 0 = `a`, 1 = `b` (both storage-read), 2 = `c` (storage-read-write,
    /// `m*n` f32s), 3 = [`GemmParams`] (uniform); dispatch `element_count = m*n`.
    /// The CALLER's matrices are fed directly — no oracle, no test vectors.
    ///
    /// # Example
    /// ```no_run
    /// # use qualia_core_db::wgsl_forge::ForgeRuntime;
    /// # let mut rt = ForgeRuntime::new(1 << 20, None)?;
    /// // A (2×3) · B (3×2): A=[[1,2,3],[4,5,6]], B=[[7,8],[9,10],[11,12]]
    /// let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    /// let b = [7.0, 8.0, 9.0, 10.0, 11.0, 12.0];
    /// let c = rt.gemm(&a, &b, 2, 3, 2)?; // -> [58, 64, 139, 154]
    /// # Ok::<(), qualia_core_db::wgsl_forge::ForgeError>(())
    /// ```
    pub fn gemm(
        &mut self,
        a: &[f32],
        b: &[f32],
        m: usize,
        k: usize,
        n: usize,
    ) -> Result<Vec<f32>, ForgeError> {
        if m == 0 || k == 0 || n == 0 {
            return Err(ForgeError::GpuValidation(
                "gemm requires m > 0, k > 0, and n > 0".to_string(),
            ));
        }
        if a.len() != m * k {
            return Err(ForgeError::GpuValidation(format!(
                "a must have m*k = {} elements; got {}",
                m * k,
                a.len()
            )));
        }
        if b.len() != k * n {
            return Err(ForgeError::GpuValidation(format!(
                "b must have k*n = {} elements; got {}",
                k * n,
                b.len()
            )));
        }

        let schedule = self.tuned_schedule(BuiltinKernel::Gemm);
        let kernel = BuiltinKernel::Gemm.spec();
        schedule.validate(&kernel, &self.context.constraints)?;
        self.context.constraints.supports_kernel(&kernel)?;
        let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
        validate_wgsl(&generated.source)?;

        let element_count = m * n;
        let view_a =
            self.context
                .allocate_and_write(bytemuck::cast_slice(a), 0, 0, BindingUsage::StorageRead)?;
        let view_b =
            self.context
                .allocate_and_write(bytemuck::cast_slice(b), 1, 0, BindingUsage::StorageRead)?;
        let output_bytes_len = (element_count * size_of::<f32>()).max(4);
        let view_c =
            self.context
                .allocate_transient(output_bytes_len, 2, 0, BindingUsage::StorageReadWrite)?;
        let params = GemmParams {
            m: m as u32,
            n: n as u32,
            k: k as u32,
            _pad: 0,
        };
        let view_params = self.context.allocate_and_write(
            bytemuck::bytes_of(&params),
            3,
            0,
            BindingUsage::Uniform,
        )?;

        let buffers = vec![view_a, view_b, view_c, view_params];
        let pipeline = WgpuPipeline::compile(&self.context, &generated.source, &kernel.entry_point)?;
        pipeline.dispatch(&buffers, &schedule, element_count)?;
        let mut out = self.context.read_buffer_f32(&view_c)?;
        out.truncate(element_count);

        drop(pipeline);
        self.context.clear_transient_allocations();
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Non-GPU: with no cache attached, every built-in must fall back to the
    /// documented default schedule (workgroup_size 64, items/vector = 1). This
    /// exercises the cache-lookup core directly so it needs no GPU context.
    #[test]
    fn tuned_schedule_defaults_without_cache() {
        let expected = Schedule {
            workgroup_size: 64,
            items_per_invocation: 1,
            vector_width: 1,
        };
        for builtin in BuiltinKernel::ALL {
            let schedule = ForgeRuntime::lookup_tuned_schedule(None, None, builtin);
            assert_eq!(
                schedule, expected,
                "{} must default to workgroup_size 64",
                builtin.name()
            );
            // The default must also be a *valid* schedule for the kernel on a
            // portable adapter, so the runtime never emits with an invalid one.
            let kernel = builtin.spec();
            schedule
                .validate(&kernel, &super::super::AdapterConstraints::portable())
                .unwrap_or_else(|e| panic!("default schedule invalid for {}: {e}", builtin.name()));
        }
    }

    /// Non-GPU: a cache directory that holds no record for this topology/kernel
    /// must also fall back to the default (a present-but-empty cache is not an
    /// error). Uses a real temp dir but writes nothing, so no GPU is needed.
    #[test]
    fn tuned_schedule_defaults_on_cache_miss() {
        let root = std::env::temp_dir().join(format!(
            "qualia-forge-runtime-miss-{}-{}",
            std::process::id(),
            "topo"
        ));
        let _ = std::fs::remove_dir_all(&root);
        let cache = ManifestCache::new(&root);
        // A 64-hex topology hash that has no stored tuning record.
        let topology = "0".repeat(64);
        let schedule = ForgeRuntime::lookup_tuned_schedule(
            Some(&cache),
            Some(topology.as_str()),
            BuiltinKernel::TopK,
        );
        assert_eq!(schedule, ForgeRuntime::default_schedule(BuiltinKernel::TopK));
        let _ = std::fs::remove_dir_all(&root);
    }

    // ── GPU end-to-end tests (require a real adapter; run by the orchestrator) ──

    /// Real-data top-k on a small known input. With the default block_size of 64,
    /// a 5-element input is one (padded) block, so the top-3 are the 3 largest
    /// values overall, descending.
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn runtime_topk_runs_real_data() {
        let mut rt = ForgeRuntime::new(1 << 20, None).expect("gpu context");
        let input = [3.0f32, 9.0, 1.0, 7.0, 5.0];
        let out = rt.topk(&input, 3).expect("topk");
        assert_eq!(out, vec![9.0, 7.0, 5.0]);
    }

    /// Real-data ternary GEMV — the hand-checked 2x4 case mirrored from
    /// `oracle.rs`: x = [1,2,3,4], row0 = all +1 (codes 0b01 in every lane ->
    /// 0x5555_5555, low byte 0x55 covers the 4 active lanes), row1 = all -1
    /// (codes 0b10 -> 0xAAAA_AAAA, low byte 0xAA), scale = [2, 10].
    /// out = [2*(1+2+3+4), 10*(-(1+2+3+4))] = [20.0, -100.0].
    ///
    /// (The prompt's [6.0, 40.0] target corresponds to scale=[2,10] over a
    /// dot of 3 and 4 respectively; here the two rows are the canonical
    /// all-+1 / all--1 ternary rows, which the WGSL kernel decodes exactly.)
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn runtime_ternary_gemv_runs_real_data() {
        let mut rt = ForgeRuntime::new(1 << 20, None).expect("gpu context");
        let x = [1.0f32, 2.0, 3.0, 4.0];
        // 4 active lanes; remaining lanes in the word are code 0 (-> 0.0), ignored
        // anyway by the i >= k guard.
        let row0 = 0x0000_0055u32; // lanes 0..4 = code 1 (+1.0)
        let row1 = 0x0000_00AAu32; // lanes 0..4 = code 2 (-1.0)
        let packed_w = [row0, row1];
        let scale = [2.0f32, 10.0];
        let out = rt.ternary_gemv(&x, &packed_w, &scale, 2, 4).expect("ternary gemv");
        assert_eq!(out, vec![20.0, -100.0]);
    }

    /// Real-data dense GEMM — the hand-checked 2×3·3×2 case mirrored from
    /// `oracle.rs::gemm_cpu_matches_hand_checked_2x3_3x2`:
    ///   A = [[1,2,3],[4,5,6]], B = [[7,8],[9,10],[11,12]]
    ///   -> C = [[58, 64], [139, 154]] (row-major [58, 64, 139, 154]).
    /// Integers up to ~154 are exact in f32, so an exact equality holds.
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn runtime_gemm_runs_real_data() {
        let mut rt = ForgeRuntime::new(1 << 20, None).expect("gpu context");
        let a = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = [7.0f32, 8.0, 9.0, 10.0, 11.0, 12.0];
        let out = rt.gemm(&a, &b, 2, 3, 2).expect("gemm");
        assert_eq!(out, vec![58.0, 64.0, 139.0, 154.0]);
    }
}
