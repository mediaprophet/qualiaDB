//! GEMM: weight upload + raw/quantized dispatch + ternary-FFN dispatch
//! Split from gguf_bridge/mod.rs (structural refactor; no behaviour change).
use super::*;

/// XOR into the resident-weight key so f16-promoted FFN blobs never alias quant blobs.
#[cfg(not(target_arch = "wasm32"))]
const F16_PROMOTE_KEY_TAG: u64 = 0xF16E_F16E_0000_00F1;

/// Host policy and retained immutable source. Ownership equality preserves leased-model warmth
/// and prevents recycled mmap addresses from aliasing a previous model's weights.
#[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
pub(super) struct ResidentProjectionCache {
    source: Option<super::model_bytes::ModelByteHandle>,
    package: Option<Arc<crate::inference::moe::ftw_loader::FtwModelPackage>>,
    buffers: etalon_inference::projections::weight_resources::ResidentWeightCache,
    byte_budget: u64,
}
#[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
impl Default for ResidentProjectionCache {
    fn default() -> Self {
        Self {
            source: None,
            package: None,
            buffers: Default::default(),
            // Local retained-weight limit derives from the existing native host policy.
            // This ledger does not reserve or subtract usage in the global VRAM ledger.
            byte_budget: crate::gpu_context::global_vram_ledger().budget(),
        }
    }
}

impl QTensorEngine {
    /// Dequant full 2-D weight to dense f32 (row-major n_out × n_in) for CUDA TC path.
    /// Parallel over rows (rayon) — Q4 densify is cold-path once, then cached.
    #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
    pub(crate) fn dequant_weight_dense_f32(
        info: &GgufTensorInfo,
        raw: &[u8],
        n_in: usize,
        n_out: usize,
    ) -> Option<Vec<f32>> {
        etalon_inference::projections::weights::dense_f32(
            info,
            raw,
            n_in,
            n_out,
            crate::inference::cuda_lane::MAX_DENSE_ELEMS * 4,
        )
    }

    /// Pre-densify and cache a weight for mode=cuda / TC GEMM (no thrash on first token).
    ///
    /// Used by the CUDA_DECODE plan path to fill the host dense-weight cache so
    /// `try_cuda_batch_gemv` can hit without a cold dequant on first use. Skips
    /// when already cached, dims too small for WMMA pad-16, or densify fails.
    #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
    pub(crate) fn prewarm_cuda_weight(
        info: &GgufTensorInfo,
        raw: &[u8],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        if !crate::prefer_tensor_core_gemm() {
            return false;
        }
        if n_in < 16 || n_out < 16 {
            return false;
        }
        // Cap densify cost: one matrix ≤ ~64 MiB f32 (fits 3B FFN rows on A2000 headroom).
        if n_in.saturating_mul(n_out).saturating_mul(4) > 64 * 1024 * 1024 {
            return false;
        }
        let key = crate::weight_fingerprint(raw, n_in, n_out);
        if crate::dense_weight_cached(key) {
            return true;
        }
        // Dequantize directly into the 2 MiB-aligned huge-page buffer — skips
        // the intermediate Vec<f32> allocation + copy that cache_dense_weight
        // would require. Rayon parallelizes over rows.
        crate::inference::cuda_lane::cache_dense_weight_direct(key, n_in, n_out, |buf| {
            etalon_inference::projections::weights::dequant_dense_into(info, raw, n_in, buf)
        })
    }

    /// Promote a 2-D quant weight (Q4_K / SoA / Q6_K / Q8_0) to a resident **f16** buffer
    /// for the fast coop GEMV path. Returns `(buffer, ggml_type=F16, byte_len, row_elems)`.
    /// `None` when disabled, unsupported type, or OOM/dequant failure — caller keeps quant.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(crate) fn promote_matrix_to_f16_resident(
        &self,
        info: &GgufTensorInfo,
        raw: &[u8],
    ) -> Option<(etalon::Buffer, u32, u32, u32)> {
        if !crate::llm_bench::ffn_f16_enabled() {
            return None;
        }
        let f16_bytes =
            etalon_inference::projections::weights::promote_f16(info, raw, 512 * 1024 * 1024)?;
        let nbytes = f16_bytes.len();
        let n0 = info.dims[0] as usize;
        let key = (raw.as_ptr() as u64) ^ F16_PROMOTE_KEY_TAG;
        let buf = self.resident_weight_buffer(key, &f16_bytes)?;
        Some((
            buf,
            crate::ggml_quants::GGML_TYPE_F16,
            nbytes as u32,
            n0 as u32,
        ))
    }

    #[cfg(feature = "gpu-runtime")]
    /// Writes `weight-words`.
    pub(crate) fn write_weight_words(&self, raw: &[u8], max_bytes: usize) {
        etalon_inference::projections::weight_resources::write_weight_words(
            self.gpu_queue(),
            self.gemm_weight_buf.as_ref().expect("gemm weight buf"),
            raw,
            max_bytes,
        )
    }

    /// Phase 2: get-or-create the resident VRAM buffer for a weight byte-region, keyed by `key`
    /// (the region's absolute mmap address — unique per distinct weight, stable across tokens). The
    /// bytes (from the immutable mmap) are uploaded **once** on first use, then this returns a clone
    /// of the resident buffer handle (wgpu buffers are Arc-backed) to bind in place of the shared
    /// per-token `gemm_weight_buf` — eliminating the per-token weight re-upload. Buffer size is
    /// 256-aligned ≥ `raw.len()`; the shader only reads `weight_byte_len`.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(crate) fn resident_weight_buffer(&self, key: u64, raw: &[u8]) -> Option<etalon::Buffer> {
        let source = self.clone_model_bytes()?;
        let mut cache = self.gemm_resident_weights.lock().ok()?;
        let same_package = match (cache.package.as_ref(), self.ftw_package.as_ref()) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !same_package
            || !cache
                .source
                .as_ref()
                .is_some_and(|old| old.same_owner(&source))
        {
            cache.buffers.clear();
            cache.source = Some(source);
            cache.package = self.ftw_package.clone();
        }
        let mut budget =
            etalon_inference::gpu_resources::ResourceBudget::for_device(self.gpu_device());
        budget.max_buffer_bytes = cache.byte_budget;
        etalon_inference::projections::weight_resources::resident_weight_buffer(
            self.gpu_device(),
            self.gpu_queue(),
            &mut cache.buffers,
            key,
            raw,
            budget,
        )
    }

    /// Set the admitted cold native-projection/readback payload budget. This legacy Vec-returning
    /// entrypoint reports native admission exhaustion by panic instead of choosing another layout.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub fn set_projection_cold_byte_budget(&mut self, bytes: usize) {
        self.projection_cold_byte_budget = bytes;
    }

    /// Set aggregate projection-weight allowance. Admission misses retain uncached GPU/CPU fallback.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub fn set_resident_projection_byte_budget(&mut self, bytes: u64) {
        let cache = self
            .gemm_resident_weights
            .get_mut()
            .unwrap_or_else(|e| e.into_inner());
        cache.byte_budget = bytes;
        if cache.buffers.retained_bytes() > bytes {
            cache.buffers.clear();
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) fn invalidate_resident_projection_weights(&mut self) {
        let cache = self
            .gemm_resident_weights
            .get_mut()
            .unwrap_or_else(|e| e.into_inner());
        cache.buffers.clear();
        cache.source = None;
        cache.package = None;
    }

    /// Quantized GEMM from a pre-sliced weight byte range (chunk-local row indices).
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(crate) fn dispatch_gemm_raw_into(
        &self,
        info: &GgufTensorInfo,
        raw: &[u8],
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        etalon_inference::projections::ProjectionGpuEngine::new(self)
            .dispatch_gemm_raw_into(info, raw, input, out, n_in, n_out)
    }

    /// CPU GEMM when wgpu is not compiled in (qdnf / no gpu-runtime).
    #[cfg(all(not(target_arch = "wasm32"), not(feature = "gpu-runtime")))]
    pub(crate) fn dispatch_gemm_raw_into(
        &self,
        info: &GgufTensorInfo,
        raw: &[u8],
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        if n_in > input.len() || n_out > out.len() {
            return false;
        }
        stack_gemm_quant(raw, info, input, out, n_in, n_out)
    }

    /// Browser compatibility path for synchronous callers. WebGPU mapping must
    /// be awaited, so the public browser inference route uses the async GEMM
    /// dispatcher and this fallback executes the deterministic CPU kernel.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn dispatch_gemm_raw_into(
        &self,
        info: &GgufTensorInfo,
        raw: &[u8],
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        if n_in > input.len() || n_out > out.len() {
            return false;
        }
        stack_gemm_quant(raw, info, input, out, n_in, n_out)
    }

    /// A1b: dispatch one **ternary** FFN GEMM (`GGML_TYPE_TERNARY_158`). The resident 2-bit GPU
    /// kernel when the toggle is on AND the tensor is resident; otherwise the CPU base-3 oracle on
    /// the blob fetched from the mmap. Fail-closed (returns false, never garbage) on any mismatch.
    pub(crate) fn dispatch_ternary_ffn(
        &self,
        info: &GgufTensorInfo,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        if n_in > input.len() || n_out > out.len() {
            return false;
        }
        // GPU resident path (toggle on): keyed by the P64 blob offset (== info.byte_offset).
        #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
        if crate::llm_bench::ternary_ffn_enabled() {
            if let Some(res) = self.ternary_ffn.as_ref() {
                if res.gemv(
                    self.gpu_device(),
                    self.gpu_queue(),
                    info.byte_offset,
                    input,
                    out,
                    n_in,
                    n_out,
                ) {
                    return true;
                }
            }
        }
        // CPU oracle fallback (toggle off, or GPU unavailable): the SAME ternary weights via the
        // base-3 CPU GEMM — correct, slower; this is the toggle's OFF baseline.
        let mmap = match self.model_bytes() {
            Some(m) => m,
            None => return false,
        };
        let raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, self.tensor_data_offset, info)
        {
            Ok(s) => s,
            Err(_) => return false,
        };
        if raw.len() < 4 {
            return false;
        }
        let scale = f32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
        crate::ternary::ternary_gemm_cpu(
            &input[..n_in],
            &raw[4..],
            scale,
            n_in,
            n_out,
            1,
            0,
            0,
            &mut out[..n_out],
        );
        true
    }

    /// Quantized GEMM into caller `out` using reused GPU buffers (Q6_K) or CPU dequant fallback.
    pub fn dispatch_gemm_into(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        info: &GgufTensorInfo,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        if n_in > input.len() || n_out > out.len() {
            wlog(&format!(
                "[gemm_into] GUARD n_in={n_in} n_out={n_out} input={} out={}",
                input.len(),
                out.len()
            ));
            return false;
        }
        // A1b: ternary FFN tensors are not row-block quantized — route them to the dedicated ternary
        // dispatch (resident 2-bit GPU kernel / CPU oracle) before the standard fetch+GEMM path.
        if info.ggml_type == crate::ternary::GGML_TYPE_TERNARY_158 {
            return self.dispatch_ternary_ffn(info, input, out, n_in, n_out);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let raw = match self.fetch_tensor_raw_bytes(info) {
            Some(s) => s,
            None => {
                let mmap = match self.model_bytes() {
                    Some(m) => m,
                    None => return false,
                };
                match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, info) {
                    Ok(s) => s,
                    Err(_) => return false,
                }
            }
        };
        #[cfg(target_arch = "wasm32")]
        let raw = {
            let mmap = match self.model_bytes() {
                Some(m) => m,
                None => return false,
            };
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, info) {
                Ok(s) => s,
                Err(_) => return false,
            }
        };
        self.dispatch_gemm_raw_into(info, raw, input, out, n_in, n_out)
    }
}

/// Substrate-parity proof (CPU, no GPU): the LLM's quantized GEMV is *the same linear
/// operation* as the engine's dense `solvers::linear_algebra::gemm`.
///
/// The LLM forward path is not a bespoke "AI inference" kernel — its weight×activation
/// step is matrix–vector multiplication `out[i] = Σ_j W[i][j]·x[j]`, with `W` dequantized
/// on the fly. This proves it: dequantize the quantized weights to a dense matrix, run the
/// engine's `matvec` on them, and show the LLM kernel (`stack_gemm_quant`) agrees to f32
/// rounding. Together with the existing GPU↔CPU probe (`gemm_parity_probe`), this closes
/// the chain  substrate GEMM ≡ LLM CPU GEMV ≡ LLM GPU GEMV.
#[cfg(test)]
mod substrate_parity_tests {
    use crate::gguf_sharder::GgufTensorInfo;
    use crate::solvers::linear_algebra::gemm::{matvec, Transpose};

    /// Returns `(exact_err, quant_err)`:
    /// - `exact_err` = max|LLM_kernel(Q8(W)) − substrate(dequant(Q8(W)))| — should be ~f32 ε,
    ///   proving the two compute the *same* operation;
    /// - `quant_err` = max|LLM_kernel(Q8(W)) − substrate(W_original)| — the Q8 quantization cost.
    fn run(n_in: usize, n_out: usize, seed: u64) -> (f32, f32) {
        // Deterministic LCG → values in [-1, 1) (mirrors gemm_parity_probe_blocking).
        let mut s = seed | 1;
        let mut rng = move || -> f32 {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((s >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
        };

        let row_bytes = crate::llm_kernel_parity::q8_0_bytes(n_in);
        let mut raw = vec![0u8; row_bytes * n_out];
        let mut w_orig = vec![0f32; n_in * n_out]; // dense, pre-quantization (row-major n_out×n_in)
        let mut row_f32 = vec![0f32; n_in];
        for r in 0..n_out {
            for x in row_f32.iter_mut() {
                *x = rng();
            }
            w_orig[r * n_in..(r + 1) * n_in].copy_from_slice(&row_f32);
            assert!(crate::llm_kernel_parity::quantize_q8_0_from_f32(
                &row_f32,
                &mut raw[r * row_bytes..(r + 1) * row_bytes],
            ));
        }
        let input: Vec<f32> = (0..n_in).map(|_| rng()).collect();

        let info = GgufTensorInfo {
            dims: [n_in as u64, n_out as u64, 1, 1],
            n_dims: 2,
            ggml_type: crate::ggml_quants::GGML_TYPE_Q8_0,
            byte_offset: 0,
        };

        // (1) The actual LLM CPU kernel.
        let mut out_llm = vec![0f32; n_out];
        assert!(crate::gguf_bridge::stack_gemm_quant(
            &raw,
            &info,
            &input,
            &mut out_llm,
            n_in,
            n_out
        ));

        // (2) Dequantize the same quantized weights to a dense matrix, then run the
        //     engine's GEMM (matvec) on it. Same operands ⇒ must match the LLM kernel.
        let mut w_deq = vec![0f64; n_in * n_out];
        let mut deq_row = vec![0f32; n_in];
        for i in 0..n_out {
            let got = crate::ggml_quants::dequant_matrix_row_into(&raw, &info, i, &mut deq_row)
                .unwrap_or(0);
            assert_eq!(got, n_in, "dequant row {i}");
            for j in 0..n_in {
                w_deq[i * n_in + j] = deq_row[j] as f64;
            }
        }
        let x_f64: Vec<f64> = input.iter().map(|&v| v as f64).collect();
        let mut out_sub_deq = vec![0f64; n_out];
        matvec(Transpose::No, n_out, n_in, &w_deq, &x_f64, &mut out_sub_deq).unwrap();

        // (3) The engine GEMM on the ORIGINAL (pre-quant) weights — the Q8 cost reference.
        let w_orig_f64: Vec<f64> = w_orig.iter().map(|&v| v as f64).collect();
        let mut out_sub_orig = vec![0f64; n_out];
        matvec(
            Transpose::No,
            n_out,
            n_in,
            &w_orig_f64,
            &x_f64,
            &mut out_sub_orig,
        )
        .unwrap();

        let exact_err = (0..n_out)
            .map(|i| (out_llm[i] as f64 - out_sub_deq[i]).abs() as f32)
            .fold(0.0f32, f32::max);
        let quant_err = (0..n_out)
            .map(|i| (out_llm[i] as f64 - out_sub_orig[i]).abs() as f32)
            .fold(0.0f32, f32::max);
        (exact_err, quant_err)
    }

    #[test]
    fn llm_quant_gemv_is_the_substrate_gemm() {
        // Several shapes/seeds; n_in a multiple of 32 (Q8_0 block size).
        for &(n_in, n_out, seed) in &[
            (64usize, 32usize, 0xC0FFEEu64),
            (128, 96, 7),
            (256, 64, 0xBEEF),
        ] {
            let (exact_err, quant_err) = run(n_in, n_out, seed);
            // Same operation: LLM kernel == engine GEMM on identical (dequantized) weights,
            // to f32 accumulation rounding only.
            assert!(
                exact_err < 1e-4,
                "LLM GEMV diverges from substrate GEMM on identical weights: exact_err={exact_err} (n_in={n_in}, n_out={n_out})"
            );
            // Quantization is the *only* extra divergence from exact math, and it is bounded.
            assert!(
                quant_err < 0.5,
                "Q8 quantization error unexpectedly large: quant_err={quant_err}"
            );
        }
    }
}
