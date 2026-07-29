//! GEMM: weight upload + raw/quantized dispatch + ternary-FFN dispatch
//! Split from gguf_bridge/mod.rs (structural refactor; no behaviour change).
use super::*;

/// XOR into the resident-weight key so f16-promoted FFN blobs never alias quant blobs.
#[cfg(not(target_arch = "wasm32"))]
const F16_PROMOTE_KEY_TAG: u64 = 0xF16E_F16E_0000_00F1;

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
        use crate::ggml_quants::dequant_matrix_row_into;
        use rayon::prelude::*;
        let total = n_out.checked_mul(n_in)?;
        if total > crate::inference::cuda_lane::MAX_DENSE_ELEMS {
            return None;
        }
        let mut dense = vec![0.0f32; total];
        let ok = dense
            .par_chunks_mut(n_in)
            .enumerate()
            .map(|(r, row)| dequant_matrix_row_into(raw, info, r, row).is_ok())
            .all(|b| b);
        if ok {
            Some(dense)
        } else {
            None
        }
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
        use crate::ggml_quants::dequant_matrix_row_into;
        use rayon::prelude::*;
        crate::inference::cuda_lane::cache_dense_weight_direct(key, n_in, n_out, |buf| {
            buf.par_chunks_mut(n_in)
                .enumerate()
                .map(|(r, row)| dequant_matrix_row_into(raw, info, r, row).is_ok())
                .all(|b| b)
        })
    }

    /// Promote a 2-D quant weight (Q4_K / SoA / Q6_K / Q8_0) to a resident **f16** buffer
    /// for the fast coop GEMV path. Returns `(buffer, ggml_type=F16, byte_len, row_elems)`.
    /// `None` when disabled, unsupported type, or OOM/dequant failure — caller keeps quant.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn promote_matrix_to_f16_resident(
        &self,
        info: &GgufTensorInfo,
        raw: &[u8],
    ) -> Option<(wgpu::Buffer, u32, u32, u32)> {
        if !crate::llm_bench::ffn_f16_enabled() {
            return None;
        }
        use crate::ggml_quants::{
            dequant_matrix_row_into, GGML_TYPE_F16, GGML_TYPE_Q4_K, GGML_TYPE_Q4_K_SOA,
            GGML_TYPE_Q6_K, GGML_TYPE_Q8_0,
        };
        if !matches!(
            info.ggml_type,
            GGML_TYPE_Q4_K | GGML_TYPE_Q4_K_SOA | GGML_TYPE_Q6_K | GGML_TYPE_Q8_0
        ) {
            return None;
        }
        if info.n_dims < 2 || info.dims[0] == 0 || info.dims[1] == 0 {
            return None;
        }
        let n0 = info.dims[0] as usize; // in (row width)
        let n1 = info.dims[1] as usize; // out (rows)
        let nbytes = n0.checked_mul(n1)?.checked_mul(2)?;
        // Skip absurd expansions (e.g. accidental full-model promote) — FFN matrices are
        // typically ≤ ~200 MiB each on 3B-class models.
        if nbytes > 512 * 1024 * 1024 {
            return None;
        }
        let mut f16_bytes = vec![0u8; nbytes];
        let mut row = vec![0f32; n0];
        for r in 0..n1 {
            if dequant_matrix_row_into(raw, info, r, &mut row).is_err() {
                return None;
            }
            let base = r * n0 * 2;
            for (c, &v) in row.iter().enumerate() {
                let bits = half::f16::from_f32(v).to_le_bytes();
                f16_bytes[base + c * 2] = bits[0];
                f16_bytes[base + c * 2 + 1] = bits[1];
            }
        }
        let key = (raw.as_ptr() as u64) ^ F16_PROMOTE_KEY_TAG;
        let buf = self.resident_weight_buffer(key, &f16_bytes)?;
        Some((buf, GGML_TYPE_F16, nbytes as u32, n0 as u32))
    }

    pub(crate) fn write_weight_words(&self, raw: &[u8], max_bytes: usize) {
        let weight_buf = self.gemm_weight_buf.as_ref().expect("gemm weight buf");
        let upload = if raw.len() <= max_bytes {
            raw
        } else {
            &raw[..max_bytes]
        };
        self.gpu_queue().write_buffer(weight_buf, 0, upload);
    }

    /// Phase 2: get-or-create the resident VRAM buffer for a weight byte-region, keyed by `key`
    /// (the region's absolute mmap address — unique per distinct weight, stable across tokens). The
    /// bytes (from the immutable mmap) are uploaded **once** on first use, then this returns a clone
    /// of the resident buffer handle (wgpu buffers are Arc-backed) to bind in place of the shared
    /// per-token `gemm_weight_buf` — eliminating the per-token weight re-upload. Buffer size is
    /// 256-aligned ≥ `raw.len()`; the shader only reads `weight_byte_len`.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn resident_weight_buffer(&self, key: u64, raw: &[u8]) -> Option<wgpu::Buffer> {
        let mut map = self.gemm_resident_weights.lock().ok()?;
        if let Some(b) = map.get(&key) {
            return Some(b.clone());
        }
        let size = (((raw.len() + 255) & !255).max(4)) as wgpu::BufferAddress;
        let buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("ResidentWeight"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_queue().write_buffer(&buf, 0, raw);
        map.insert(key, buf.clone());
        Some(buf)
    }

    /// Quantized GEMM from a pre-sliced weight byte range (chunk-local row indices).
    #[cfg(not(target_arch = "wasm32"))]
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

        // Mode=cuda acceleration (fail closed → wgpu path below).
        #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
        {
            if crate::prefer_tensor_core_gemm() {
                // M2b: on-device Q4_K SoA dequant-GEMV (no host densify) — preferred for .soa.p64.
                // This path has CPU differential tests; keep it on for mode=cuda.
                if info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA
                    && crate::try_q4k_soa_gemv(n_in, n_out, &input[..n_in], raw, &mut out[..n_out])
                {
                    return true;
                }
                // Dense densify + CUDA GEMM for F16/Q8/etc. was measured **incoherent** on
                // single-token decode (pad-to-16 + f16-WMMA reduced path → garbage tokens,
                // 2026-07-24 SmolLM f16 package). Default OFF for decode GEMV (always batch=1
                // here). Opt-in: QUALIA_LLM_CUDA_TC_DECODE=1 (lab only). Prefill should use
                // multi-token dispatch or the resident mega-pass, not this broken default.
                let densify_tc_decode = matches!(
                    std::env::var("QUALIA_LLM_CUDA_TC_DECODE").ok().as_deref(),
                    Some("1") | Some("true")
                );
                if densify_tc_decode
                    && n_in >= 16
                    && n_out >= 16
                    && n_in.saturating_mul(n_out) <= crate::inference::cuda_lane::MAX_DENSE_ELEMS
                {
                    let key = crate::weight_fingerprint(raw, n_in, n_out);
                    if crate::try_cuda_batch_gemv_cached_only(
                        key,
                        &input[..n_in],
                        1,
                        &mut out[..n_out],
                    ) {
                        return true;
                    }
                    if let Some(dense) = Self::dequant_weight_dense_f32(info, raw, n_in, n_out) {
                        if crate::try_cuda_batch_gemv_cached(
                            key,
                            &input[..n_in],
                            1,
                            n_in,
                            n_out,
                            &dense,
                            &mut out[..n_out],
                        ) {
                            return true;
                        }
                    }
                }
            }
        }

        let weight_bytes = raw.len();
        // GEMM shader supports a wider quant set than the legacy `ggml_gpu_quant_supported` (Q4_K/Q6_K)
        // — notably Q8_0, which was silently falling back to the CPU `stack_gemm_quant` below (the FFN
        // bottleneck for Q8_0 models). The guards (size/buffer caps) still fail closed → CPU fallback.
        if ggml_gpu_gemm_supported(info.ggml_type)
            && n_in <= MAX_STACK_GEMM_IN
            && n_out <= self.gemm_max_out_dim as usize
            && weight_bytes <= self.max_tensor_bytes
            && self.gemm_input_buf.is_some()
        {
            let params = GemmGpuParams {
                n_in: n_in as u32,
                n_out: n_out as u32,
                weight_ggml_type: info.ggml_type,
                weight_row_elems: info.dims[0] as u32,
                weight_byte_len: raw.len() as u32,
                n_batch: 1,
                in_row_stride: 0,
                out_row_stride: 0,
            };
            let input_buf = self.gemm_input_buf.as_ref().unwrap();
            let weight_buf = self.gemm_weight_buf.as_ref().unwrap();
            let output_buf = self.gemm_output_buf.as_ref().unwrap();
            let params_buf = self.gemm_params_buf.as_ref().unwrap();
            let staging = self.gemm_output_staging.as_ref().unwrap();

            self.gpu_queue()
                .write_buffer(input_buf, 0, bytemuck::cast_slice(&input[..n_in]));
            // Phase 2 (native): bind this tensor's resident VRAM buffer (uploaded once, keyed by
            // byte_offset) instead of re-uploading the weight into the shared gemm_weight_buf every
            // token. On wasm the resident path is the MC8 arena, so this stays the per-token upload.
            #[cfg(not(target_arch = "wasm32"))]
            let resident = if crate::llm_bench::resident_weights_enabled() {
                // Key on the chunk's absolute mmap address, NOT `info.byte_offset`: the output
                // projection passes the SAME whole-tensor `info` (byte_offset == header size) for
                // every vocab chunk, so byte_offset aliases all chunks to one buffer (wrong logits).
                // `raw` is a slice of the immutable, lifetime-stable mmap → its start address is
                // unique per distinct weight region and identical across tokens.
                self.resident_weight_buffer(raw.as_ptr() as u64, raw)
            } else {
                None
            };
            #[cfg(target_arch = "wasm32")]
            let resident: Option<wgpu::Buffer> = None;
            let weight_binding: &wgpu::Buffer = match resident.as_ref() {
                Some(r) => r,
                None => {
                    self.write_weight_words(raw, self.max_tensor_bytes);
                    weight_buf
                }
            };
            self.gpu_queue()
                .write_buffer(params_buf, 0, bytemuck::bytes_of(&params));

            // 0.0.21: select the cooperative GEMV kernel (one workgroup/row, coalesced + shared-mem
            // reduction) when enabled, else the naive 1-thread/row kernel. Same group-0 bindings, so
            // only the pipeline + dispatch geometry differ. The bind group must be built from the
            // ACTIVE pipeline's auto-layout.
            #[cfg(not(target_arch = "wasm32"))]
            let use_coop = crate::llm_bench::coop_gemv_enabled();
            #[cfg(target_arch = "wasm32")]
            let use_coop = false;
            #[cfg(not(target_arch = "wasm32"))]
            let use_mr = use_coop
                && info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA
                && n_out >= 512;
            #[cfg(not(target_arch = "wasm32"))]
            let active_pipeline: &wgpu::ComputePipeline = if use_mr {
                &self.coop_gemv_mr_pipeline
            } else if use_coop {
                &self.coop_gemv_pipeline
            } else {
                &self.pipeline
            };
            #[cfg(target_arch = "wasm32")]
            let use_mmv_q8_0 =
                info.ggml_type == crate::ggml_quants::GGML_TYPE_Q8_0 && (n_in % 32 == 0);
            #[cfg(target_arch = "wasm32")]
            let active_pipeline: &wgpu::ComputePipeline = if use_mmv_q8_0 {
                &self.mmv_q8_0_pipeline
            } else {
                &self.pipeline
            };
            #[cfg(target_arch = "wasm32")]
            let use_mr = false;

            #[cfg(not(target_arch = "wasm32"))]
            let bind_layout = self.native_gemm_bind_layout(use_coop).clone();
            #[cfg(target_arch = "wasm32")]
            let bind_layout = active_pipeline.get_bind_group_layout(0);
            // CoopGemvBGL is 5-slot (binding 4 = residual). Dummy residual = input.
            let bind_group = if use_coop {
                self.gpu_device()
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("LayerGemmBindGroup"),
                        layout: &bind_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: input_buf.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: weight_binding.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: params_buf.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: output_buf.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 4,
                                resource: input_buf.as_entire_binding(),
                            },
                        ],
                    })
            } else {
                self.gpu_device()
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("LayerGemmBindGroup"),
                        layout: &bind_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: input_buf.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: weight_binding.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: params_buf.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: output_buf.as_entire_binding(),
                            },
                        ],
                    })
            };

            let mut encoder =
                self.device()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("LayerGemmEncoder"),
                    });
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: crate::llm_gpu_profiler::pass_writes_both(),
                });
                cpass.set_pipeline(active_pipeline);
                cpass.set_bind_group(0, &bind_group, &[]);
                if use_mr {
                    cpass.dispatch_workgroups(
                        crate::llm_bench::coop_gemv_workgroups(n_out as u32),
                        1,
                        1,
                    );
                } else if use_coop {
                    cpass.dispatch_workgroups(n_out as u32, 1, 1);
                } else {
                    #[cfg(target_arch = "wasm32")]
                    if use_mmv_q8_0 {
                        cpass.dispatch_workgroups((n_out as u32 + 3) / 4, 1, 1);
                    } else {
                        cpass.dispatch_workgroups((n_out as u32 + 63) / 64, 1, 1);
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    cpass.dispatch_workgroups((n_out as u32 + 63) / 64, 1, 1);
                }
            }
            let out_bytes = (n_out * 4) as wgpu::BufferAddress;
            encoder.copy_buffer_to_buffer(output_buf, 0, staging, 0, out_bytes);
            crate::llm_gpu_profiler::resolve(&mut encoder);
            self.gpu_queue().submit(Some(encoder.finish()));
            crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Gemm);

            let slice = staging.slice(..out_bytes);
            let (tx, rx) = futures_channel::oneshot::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            self.poll_wait();
            #[cfg(not(target_arch = "wasm32"))]
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                if handle.block_on(rx).ok().map(|m| m.is_ok()).unwrap_or(false) {
                    let data = slice
                        .get_mapped_range()
                        .expect("wgpu buffer map_range failed");
                    let floats: &[f32] = bytemuck::cast_slice(&data);
                    out[..n_out].copy_from_slice(&floats[..n_out]);
                    drop(data);
                    staging.unmap();
                    return true;
                }
            }
            let _ = staging.unmap();
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
        #[cfg(not(target_arch = "wasm32"))]
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
        let mmap = match self.gguf_mmap.as_deref() {
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
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, info)
        {
            Ok(s) => s,
            Err(_) => return false,
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
