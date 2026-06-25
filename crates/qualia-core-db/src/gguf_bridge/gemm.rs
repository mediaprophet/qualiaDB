//! GEMM: weight upload + raw/quantized dispatch + ternary-FFN dispatch
//! Split from gguf_bridge/mod.rs (structural refactor; no behaviour change).
use super::*;

impl QTensorEngine {
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
            let active_pipeline: &wgpu::ComputePipeline =
                if use_coop { &self.coop_gemv_pipeline } else { &self.pipeline };
            #[cfg(target_arch = "wasm32")]
            let active_pipeline: &wgpu::ComputePipeline = &self.pipeline;

            let bind_layout = active_pipeline.get_bind_group_layout(0);
            let bind_group = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
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
            });

            let mut encoder = self
                .device()
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
                if use_coop {
                    // One workgroup per output row (decode batch = 1). n_out ≤ MAX_STACK_GEMM_OUT
                    // (10240) < 65535, guarded above, so this is within the dispatch limit.
                    cpass.dispatch_workgroups(n_out as u32, 1, 1);
                } else {
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
                    let data = slice.get_mapped_range();
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
        // GPU resident path (toggle on): keyed by the `.q42` blob offset (== info.byte_offset).
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
        let raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, self.tensor_data_offset, info) {
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
            wlog(&format!("[gemm_into] GUARD n_in={n_in} n_out={n_out} input={} out={}", input.len(), out.len()));
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
