//! Quantized token embedding + fused transformer block dispatch
//! Split from gguf_bridge/mod.rs (structural refactor; no behaviour change).
use super::*;

impl QTensorEngine {
    /// Upload raw quantized embedding bytes to the GPU and matmul without CPU dequant.
    /// Returns `None` when the GGML type has no WGSL kernel (caller uses CPU fallback).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn dispatch_quantized_token_embedding(
        &self,
        raw_embd: &[u8],
        ggml_type: u32,
        n_embd: u32,
        weight_tensor: &QTensor,
    ) -> Option<Vec<f32>> {
        if ggml_type != crate::ggml_quants::GGML_TYPE_Q6_K || raw_embd.is_empty() || n_embd == 0 {
            return None;
        }

        let n_output = weight_tensor
            .shape
            .first()
            .copied()
            .unwrap_or(n_embd as usize) as u32;
        let n_embd_u = n_embd;
        let weights_elems = (n_output as usize).saturating_mul(n_embd as usize);

        let params = EmbeddingGpuParams {
            n_embd: n_embd_u,
            ggml_type,
            n_output,
            raw_byte_len: raw_embd.len() as u32,
        };

        // WGSL storage uses u32 words; pad mmap slice to 4-byte alignment.
        let word_bytes = raw_embd.len().div_ceil(4) * 4;
        let embd_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("QuantizedEmbeddingBytes"),
            size: word_bytes.max(4) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if raw_embd.len() == word_bytes {
            self.gpu_queue().write_buffer(&embd_buf, 0, raw_embd);
        } else {
            const MAX_EMB_ROW_PAD: usize = 8192;
            if word_bytes > MAX_EMB_ROW_PAD {
                return None;
            }
            let mut padded = [0u8; MAX_EMB_ROW_PAD];
            padded[..raw_embd.len()].copy_from_slice(raw_embd);
            self.gpu_queue()
                .write_buffer(&embd_buf, 0, &padded[..word_bytes]);
        }

        let params_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("EmbeddingParams"),
            size: std::mem::size_of::<EmbeddingGpuParams>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_queue()
            .write_buffer(&params_buf, 0, bytemuck::bytes_of(&params));

        let weights_size = (weights_elems * 4).max(4) as wgpu::BufferAddress;
        let weights_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("EmbeddingWeights"),
            size: weights_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if let Some(mmap) = &self.gguf_mmap {
            let offset = (self.tensor_data_offset + weight_tensor.byte_offset) as usize;
            let end = (offset + weights_elems * 4).min(mmap.len());
            if end > offset {
                self.gpu_queue()
                    .write_buffer(&weights_buf, 0, &mmap[offset..end]);
            }
        }

        let output_size = (n_output as usize * 4).max(4) as wgpu::BufferAddress;
        let output_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("EmbeddingOutput"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        #[cfg(not(target_arch = "wasm32"))]
        let bind_layout = self.embedding_bind_layout.clone();
        #[cfg(target_arch = "wasm32")]
        let bind_layout = self.embedding_pipeline.get_bind_group_layout(0);
        let bind_group = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("QuantizedEmbeddingBindGroup"),
                layout: &bind_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: embd_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: weights_buf.as_entire_binding(),
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
                label: Some("QuantizedEmbeddingEncoder"),
            });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("QuantizedEmbeddingPass"),
                timestamp_writes: crate::llm_gpu_profiler::pass_writes_both(),
            });
            cpass.set_pipeline(&self.embedding_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups((n_output + 63) / 64, 1, 1);
        }

        let staging_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("EmbeddingStaging"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, output_size);
        crate::llm_gpu_profiler::resolve(&mut encoder);
        self.gpu_queue().submit(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Embedding);

        let buffer_slice = staging_buf.slice(..);
        let (sender, receiver) = futures_channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
            let _ = sender.send(v);
        });
        self.poll_wait();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let handle = tokio::runtime::Handle::try_current().unwrap_or_else(|_| {
                let rt = Box::leak(Box::new(tokio::runtime::Runtime::new().unwrap()));
                rt.handle().clone()
            });
            if handle.block_on(receiver).ok()?.is_err() {
                return None;
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            return None;
        }

        let data = buffer_slice
            .get_mapped_range()
            .expect("wgpu buffer map_range failed");
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        crate::telemetry::SIEVE_OPS_COUNT
            .fetch_add(weights_elems, std::sync::atomic::Ordering::Relaxed);
        Some(result)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn dispatch_fused_transformer_block(
        &self,
        tensor: &QTensor,
        input_activations: &[f32],
    ) -> Vec<f32> {
        let rows = tensor.shape.get(0).copied().unwrap_or(4096);
        let cols = tensor.shape.get(1).copied().unwrap_or(4096);

        // ── DirectML path (Windows) ───────────────────────────────────────────
        #[cfg(target_os = "windows")]
        if let Some(dml) = &self.dml {
            if let Some(mmap) = &self.gguf_mmap {
                let offset = self.tensor_data_offset + tensor.byte_offset;
                let q4_bytes_needed = (rows * cols / crate::directml_bridge::Q4_K_BLOCK_SIZE)
                    * crate::directml_bridge::Q4_K_BLOCK_BYTES;
                if (offset as usize + q4_bytes_needed) <= mmap.len() {
                    let q4_slice = &mmap[offset as usize..offset as usize + q4_bytes_needed];
                    let weights_f32 =
                        crate::directml_bridge::dequantize_q4_k_tensor(q4_slice, rows * cols);
                    let op = crate::directml_bridge::DmlGemmOp {
                        m: input_activations.len() as u32 / cols as u32,
                        k: cols as u32,
                        n: rows as u32,
                    };
                    if let Ok(result) = op.execute(dml, input_activations, &weights_f32) {
                        crate::telemetry::SIEVE_OPS_COUNT
                            .fetch_add(rows * cols, std::sync::atomic::Ordering::Relaxed);
                        return result;
                    }
                }
            }
        }

        // ── Accelerate BLAS path (macOS / Apple Silicon AMX) ─────────────────────
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        if let Some(mmap) = &self.gguf_mmap {
            let offset = (self.tensor_data_offset + tensor.byte_offset) as usize;
            let q4_bytes_needed = (rows * cols / crate::metal_bridge::Q4_K_BLOCK_SIZE)
                * crate::metal_bridge::Q4_K_BLOCK_BYTES;
            if offset + q4_bytes_needed <= mmap.len() {
                let q4_slice = &mmap[offset..offset + q4_bytes_needed];
                let weights_f32 =
                    crate::metal_bridge::dequantize_q4_k_tensor(q4_slice, rows * cols);
                let input_rows = (input_activations.len() / cols).max(1);
                let result = crate::metal_bridge::accelerate_sgemm(
                    input_rows,
                    cols,
                    rows,
                    input_activations,
                    &weights_f32,
                );
                crate::telemetry::SIEVE_OPS_COUNT
                    .fetch_add(rows * cols, std::sync::atomic::Ordering::Relaxed);
                return result;
            }
        }

        // ── wgpu / WGSL fallback (all platforms — Vulkan on Linux/NVIDIA,
        //    Metal on macOS when mmap not loaded, D3D12 on Windows fallback) ──
        let input_bytes = bytemuck::cast_slice(input_activations);
        let input_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Input"),
            size: input_bytes.len().max(4) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_queue().write_buffer(&input_buf, 0, input_bytes);

        // Upload real weights from mmap when available, else use a zero buffer.
        let weights_size = (rows * cols * 4) as wgpu::BufferAddress;
        let weights_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Weights"),
            size: weights_size.max(4),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if let Some(mmap) = &self.gguf_mmap {
            let offset = (self.tensor_data_offset + tensor.byte_offset) as usize;
            let end = (offset + rows * cols * 4).min(mmap.len());
            if end > offset {
                let f32_bytes = &mmap[offset..end];
                self.gpu_queue().write_buffer(&weights_buf, 0, f32_bytes);
            }
        }

        let output_size = (rows * 4).max(4) as wgpu::BufferAddress;
        let output_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Upload GemmGpuParams for fused_transformer.wgsl (binding 2).
        let gemm_params = GemmGpuParams {
            n_in: cols as u32,
            n_out: rows as u32,
            weight_ggml_type: if tensor.is_quantized_q4_k { 12 } else { 14 },
            weight_row_elems: cols as u32,
            weight_byte_len: (rows * cols * 4) as u32,
            n_batch: 1,
            in_row_stride: 0,
            out_row_stride: 0,
        };
        let params_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("TransformerParams"),
            size: std::mem::size_of::<GemmGpuParams>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_queue()
            .write_buffer(&params_buf, 0, bytemuck::bytes_of(&gemm_params));

        // The bind group MUST be built from the SAME pipeline it is dispatched
        // under (below). Both `pipeline` (Fused Transformer) and `mock_pipeline`
        // (Mock Fused Contraction) are created with `layout: None`, i.e. wgpu
        // *exclusive* auto-derived layouts — a bind group from one is rejected by
        // a dispatch that set the other ("exclusive pipelines don't match"). The
        // real path (`gguf_mmap.is_some()`) dispatches `self.pipeline`; the mock
        // path (no model, i.e. tests) dispatches `self.mock_pipeline`, so it must
        // take the mock pipeline's own group-0 layout.
        let use_mock = self.gguf_mmap.is_none();
        #[cfg(not(target_arch = "wasm32"))]
        let bind_group_layout = if use_mock {
            self.mock_pipeline.get_bind_group_layout(0)
        } else {
            self.pipeline_bind_layout.clone()
        };
        #[cfg(target_arch = "wasm32")]
        let bind_group_layout = if use_mock {
            self.mock_pipeline.get_bind_group_layout(0)
        } else {
            self.pipeline.get_bind_group_layout(0)
        };
        let bind_group = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: weights_buf.as_entire_binding(),
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
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: crate::llm_gpu_profiler::pass_writes_both(),
            });
            // Same selector as the bind-group layout above — they must agree.
            let pipeline = if use_mock {
                &self.mock_pipeline
            } else {
                &self.pipeline
            };
            cpass.set_pipeline(pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups((rows as u32 + 63) / 64, 1, 1);
        }

        let staging_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, output_size);
        crate::llm_gpu_profiler::resolve(&mut encoder);
        self.gpu_queue().submit(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::FusedBlock);

        let buffer_slice = staging_buf.slice(..);
        let (sender, receiver) = futures_channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
        self.poll_wait();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let handle = tokio::runtime::Handle::try_current().unwrap_or_else(|_| {
                let rt = Box::leak(Box::new(tokio::runtime::Runtime::new().unwrap()));
                rt.handle().clone()
            });
            handle.block_on(receiver).unwrap().unwrap();
        }

        let data = buffer_slice
            .get_mapped_range()
            .expect("wgpu buffer map_range failed");
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        crate::telemetry::SIEVE_OPS_COUNT
            .fetch_add(rows * cols, std::sync::atomic::Ordering::Relaxed);
        result
    }

    /// Browser inference must await WebGPU buffer mapping. The synchronous entry
    /// point therefore reports the documented CPU-fallback sentinel.
    #[cfg(target_arch = "wasm32")]
    pub fn dispatch_quantized_token_embedding(
        &self,
        raw_embd: &[u8],
        ggml_type: u32,
        n_embd: u32,
        weight_tensor: &QTensor,
    ) -> Option<Vec<f32>> {
        wlog(&format!(
            "[embedding] synchronous browser dispatch unavailable (bytes={}, type={}, dim={}, tensor_offset={})",
            raw_embd.len(),
            ggml_type,
            n_embd,
            weight_tensor.byte_offset
        ));
        None
    }

    /// A synchronous GPU readback cannot make progress on the browser event
    /// loop. Callers must use the async inference surface instead.
    #[cfg(target_arch = "wasm32")]
    pub fn dispatch_fused_transformer_block(
        &self,
        tensor: &QTensor,
        input_activations: &[f32],
    ) -> Vec<f32> {
        panic!(
            "synchronous browser transformer dispatch is unsupported for tensor at byte offset {} ({} activations); use inferWasmAsync/inferWasmStreaming",
            tensor.byte_offset,
            input_activations.len()
        );
    }
}
