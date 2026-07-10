//! Output projection: chunked argmax + GPU top-k, output RMSNorm
//! Split from gguf_bridge/mod.rs (structural refactor; no behaviour change).
use super::*;

impl QTensorEngine {
    /// Chunked vocabulary projection with streaming argmax (zero heap, stack chunk buffer only).
    /// `max_chunks`: `0` sweeps the full vocabulary; otherwise caps chunk iterations (tests).
    pub fn dispatch_output_argmax_chunked(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        chunk_logits: &mut [f32],
        max_chunks: u32,
        sieve_mask: Option<&crate::neuro_symbolic_sieve::SieveStateMask>,
    ) -> Option<StreamingArgmaxResult> {
        let info = index.logits_projection_info()?;
        let (n_in, vocab_size) = Self::matmul_dims(info);
        if n_in == 0 || vocab_size == 0 || n_in > emb_dim || n_in > hidden.len() {
            return None;
        }
        if chunk_logits.len() < VOCAB_CHUNK_ROWS {
            return None;
        }
        let mmap = self.gguf_mmap.as_deref()?;
        let full_chunks = vocab_size.div_ceil(VOCAB_CHUNK_ROWS);
        let n_chunks = if max_chunks == 0 {
            full_chunks
        } else {
            (max_chunks as usize).min(full_chunks)
        };
        let mut best_token_id = 0u32;
        let mut max_logit = f32::NEG_INFINITY;

        for chunk_idx in 0..n_chunks {
            let row_start = chunk_idx * VOCAB_CHUNK_ROWS;
            let chunk_rows = VOCAB_CHUNK_ROWS.min(vocab_size - row_start);
            let raw = crate::ggml_quants::fetch_tensor_row_range_bytes(
                mmap,
                index.tensor_data_start,
                info,
                row_start,
                chunk_rows,
            )
            .ok()?;
            if !self.dispatch_gemm_raw_into(
                info,
                raw,
                &hidden[..n_in],
                &mut chunk_logits[..chunk_rows],
                n_in,
                chunk_rows,
            ) {
                return None;
            }
            if let Some(mask) = sieve_mask {
                update_streaming_argmax_sieved(
                    &chunk_logits[..chunk_rows],
                    chunk_rows,
                    chunk_idx,
                    Some(mask),
                    &mut best_token_id,
                    &mut max_logit,
                );
            } else {
                update_streaming_argmax(
                    &chunk_logits[..chunk_rows],
                    chunk_rows,
                    chunk_idx,
                    &mut best_token_id,
                    &mut max_logit,
                );
            }
            scrub_f32_volatile(&mut chunk_logits[..chunk_rows], chunk_rows);
        }

        if max_logit == f32::NEG_INFINITY {
            return None;
        }
        Some(StreamingArgmaxResult {
            best_token_id,
            max_logit,
        })
    }

    /// A1a: create the persistent GPU top-k pipeline + small candidate/staging buffers (once).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn init_output_topk(&mut self) {
        let shader = self
            .gpu_device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("output_topk"),
                source: wgpu::ShaderSource::Wgsl(crate::topk::TOPK_REDUCTION_WGSL.into()),
            });
        let pipeline =
            self.gpu_device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("output_topk_pipeline"),
                    layout: None,
                    module: &shader,
                    entry_point: Some("topk_block"),
                    compilation_options: Default::default(),
                    cache: self.native_pipeline_cache_ref(),
                });
        let pipeline_layout = pipeline.get_bind_group_layout(0);
        // Multi-chunk mega-pass: hold candidates for a full large-vocab sweep (≤256k ids),
        // k=1 → one cand per block; oversize for TOPK_MAX_K headroom on smaller vocabs.
        const MAX_VOCAB_RESIDENT: usize = 262_144;
        let max_blocks = (MAX_VOCAB_RESIDENT / crate::topk::TOPK_BLOCK_SIZE).max(1);
        let cand_bytes =
            ((max_blocks * crate::topk::TOPK_MAX_K).max(1) * 4) as wgpu::BufferAddress;
        self.topk_cand_val_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("TopkCandVal"),
            size: cand_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));
        self.topk_cand_idx_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("TopkCandIdx"),
            size: cand_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));
        self.topk_cand_staging = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("TopkCandStaging"),
            size: cand_bytes * 2, // packed: [val .. | idx ..]
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.topk_params_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("TopkParams"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.output_topk_bind_layout = Some(pipeline_layout);
        self.output_topk_pipeline = Some(pipeline);
    }

    /// Native decode fast path: output projection plus GPU block argmax (`k=1`) with one tiny
    /// candidate readback after all vocab chunks have been submitted. This avoids the full
    /// chunk-logit readback in [`Self::dispatch_output_argmax_chunked`] and avoids heap allocation
    /// in the decode loop.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn dispatch_output_top1_chunked(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
    ) -> Option<StreamingArgmaxResult> {
        let info = index.logits_projection_info()?;
        let (n_in, vocab_size) = Self::matmul_dims(info);
        if n_in == 0 || vocab_size == 0 || n_in > emb_dim || n_in > hidden.len() {
            return None;
        }
        let topk_pipeline = self.output_topk_pipeline.as_ref()?;
        let input_buf = self.gemm_input_buf.as_ref()?;
        let weight_buf = self.gemm_weight_buf.as_ref()?;
        let output_buf = self.gemm_output_buf.as_ref()?;
        let params_buf = self.gemm_params_buf.as_ref()?;
        let topk_params_buf = self.topk_params_buf.as_ref()?;
        let cand_val = self.topk_cand_val_buf.as_ref()?;
        let cand_idx = self.topk_cand_idx_buf.as_ref()?;
        let staging = self.topk_cand_staging.as_ref()?;
        let mmap = self.gguf_mmap.as_deref()?;

        let use_coop = crate::llm_bench::coop_gemv_enabled();
        let use_mr = use_coop
            && info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA;
        let gemm_pipeline: &wgpu::ComputePipeline = if use_mr {
            &self.coop_gemv_mr_pipeline
        } else if use_coop {
            &self.coop_gemv_pipeline
        } else {
            &self.pipeline
        };
        let gemm_layout = self.native_gemm_bind_layout(use_coop).clone();
        let topk_layout = self.output_topk_bind_layout.as_ref()?;
        let topk_bind = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Top1Bind"),
                layout: topk_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: output_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: topk_params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: cand_val.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: cand_idx.as_entire_binding(),
                    },
                ],
            });
        let block_size = crate::topk::TOPK_BLOCK_SIZE;
        let full_chunks = vocab_size.div_ceil(VOCAB_CHUNK_ROWS);
        let total_cands = vocab_size.div_ceil(block_size);
        let cand_capacity = VOCAB_CHUNK_ROWS
            .div_ceil(block_size)
            .max(1)
            .saturating_mul(crate::topk::TOPK_MAX_K);
        if total_cands == 0 || total_cands > cand_capacity {
            return None;
        }
        let resident_logits = self.mc8_logits_resident_buf.as_ref();
        let resident_row_bytes = self.mc8_logits_row_bytes as u64;

        self.gpu_queue()
            .write_buffer(input_buf, 0, bytemuck::cast_slice(&hidden[..n_in]));

        // Fast path: resident logits → ONE submit for all vocab chunks (no per-chunk fence).
        // Shared uniform buffers force multi-submit when weights must be re-uploaded each chunk.
        if let Some(res_buf) = resident_logits {
            if !(ggml_gpu_gemm_supported(info.ggml_type) && n_in <= MAX_STACK_GEMM_IN) {
                return None;
            }
            // 256-byte aligned slots (wgpu min uniform offset) for per-chunk params.
            const SLOT: usize = 256;
            let gemm_slot = std::mem::size_of::<GemmGpuParams>().max(32);
            let mut gemm_slab = vec![0u8; full_chunks * SLOT];
            let mut topk_slab = vec![0u8; full_chunks * SLOT];
            let mut chunk_meta: Vec<(u32, u32, u32)> = Vec::with_capacity(full_chunks); // rows, weight_bytes, n_blocks
            for chunk_idx in 0..full_chunks {
                let row_start = chunk_idx * VOCAB_CHUNK_ROWS;
                let chunk_rows = VOCAB_CHUNK_ROWS.min(vocab_size - row_start);
                if chunk_rows > self.gemm_max_out_dim as usize {
                    return None;
                }
                let weight_byte_len = (chunk_rows as u64 * resident_row_bytes) as u32;
                let gparams = GemmGpuParams {
                    n_in: n_in as u32,
                    n_out: chunk_rows as u32,
                    weight_ggml_type: info.ggml_type,
                    weight_row_elems: info.dims[0] as u32,
                    weight_byte_len,
                    n_batch: 1,
                    in_row_stride: 0,
                    out_row_stride: 0,
                };
                let gp = bytemuck::bytes_of(&gparams);
                let go = chunk_idx * SLOT;
                gemm_slab[go..go + gemm_slot.min(gp.len())].copy_from_slice(&gp[..gemm_slot.min(gp.len())]);
                let tparams = crate::topk::topk_params_bytes(chunk_rows as u32, 1, block_size as u32);
                topk_slab[go..go + tparams.len()].copy_from_slice(&tparams);
                let num_blocks = chunk_rows.div_ceil(block_size) as u32;
                chunk_meta.push((chunk_rows as u32, weight_byte_len, num_blocks));
            }
            // Upload slabs once; bind with per-chunk offsets via dedicated per-chunk param buffers
            // (auto layouts rarely allow dynamic offsets). Reuse gemm_params / topk_params only for
            // the first slot write pattern: create a single multi-slot buffer pair for the fused pass.
            let gemm_multi = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("Top1GemmParamsMulti"),
                size: gemm_slab.len() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let topk_multi = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("Top1TopkParamsMulti"),
                size: topk_slab.len() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.gpu_queue()
                .write_buffer(&gemm_multi, 0, &gemm_slab);
            self.gpu_queue()
                .write_buffer(&topk_multi, 0, &topk_slab);

            let mut encoder =
                self.device()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Top1FusedEncoder"),
                    });
            let mut cand_offset = 0usize;
            for (chunk_idx, &(chunk_rows, _wlen, num_blocks)) in chunk_meta.iter().enumerate() {
                let row_start = chunk_idx * VOCAB_CHUNK_ROWS;
                let byte_len = chunk_rows as u64 * resident_row_bytes;
                let weight_resource = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: res_buf,
                    offset: row_start as u64 * resident_row_bytes,
                    size: std::num::NonZeroU64::new(byte_len),
                });
                let go = (chunk_idx * SLOT) as u64;
                let gsize = std::num::NonZeroU64::new(std::mem::size_of::<GemmGpuParams>() as u64);
                let tsize = std::num::NonZeroU64::new(16);
                let mut gemm_entries = vec![
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: weight_resource,
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &gemm_multi,
                            offset: go,
                            size: gsize,
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: output_buf.as_entire_binding(),
                    },
                ];
                if use_coop {
                    gemm_entries.push(wgpu::BindGroupEntry {
                        binding: 4,
                        resource: input_buf.as_entire_binding(),
                    });
                }
                let gemm_bind = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Top1GemmBindFused"),
                    layout: &gemm_layout,
                    entries: &gemm_entries,
                });
                let topk_bind_c = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Top1TopkBindFused"),
                    layout: topk_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: output_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &topk_multi,
                                offset: go,
                                size: tsize,
                            }),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: cand_val.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: cand_idx.as_entire_binding(),
                        },
                    ],
                });
                {
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Top1GemmPass"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(gemm_pipeline);
                    cpass.set_bind_group(0, &gemm_bind, &[]);
                    if use_mr {
                        cpass.dispatch_workgroups(
                            crate::llm_bench::coop_gemv_workgroups(chunk_rows),
                            1,
                            1,
                        );
                    } else if use_coop {
                        cpass.dispatch_workgroups(chunk_rows, 1, 1);
                    } else {
                        cpass.dispatch_workgroups((chunk_rows + 63) / 64, 1, 1);
                    }
                }
                {
                    let mut tpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Top1ReducePass"),
                        timestamp_writes: None,
                    });
                    tpass.set_pipeline(topk_pipeline);
                    tpass.set_bind_group(0, &topk_bind_c, &[]);
                    tpass.dispatch_workgroups(num_blocks, 1, 1);
                }
                let cand_count = num_blocks as usize;
                let cand_bytes = (cand_count * 4) as wgpu::BufferAddress;
                let val_dst = (cand_offset * 4) as wgpu::BufferAddress;
                let idx_dst = ((total_cands + cand_offset) * 4) as wgpu::BufferAddress;
                encoder.copy_buffer_to_buffer(cand_val, 0, staging, val_dst, cand_bytes);
                encoder.copy_buffer_to_buffer(cand_idx, 0, staging, idx_dst, cand_bytes);
                cand_offset += cand_count;
            }
            crate::llm_gpu_profiler::resolve(&mut encoder);
            self.gpu_queue().submit(Some(encoder.finish()));
            crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::OutputTopk);
        } else {
            // Legacy: per-chunk upload + submit (shared weight staging buffer).
            let mut cand_offset = 0usize;
            for chunk_idx in 0..full_chunks {
                let row_start = chunk_idx * VOCAB_CHUNK_ROWS;
                let chunk_rows = VOCAB_CHUNK_ROWS.min(vocab_size - row_start);
                let raw = crate::ggml_quants::fetch_tensor_row_range_bytes(
                    mmap,
                    index.tensor_data_start,
                    info,
                    row_start,
                    chunk_rows,
                )
                .ok()?;
                if !(ggml_gpu_gemm_supported(info.ggml_type)
                    && n_in <= MAX_STACK_GEMM_IN
                    && chunk_rows <= self.gemm_max_out_dim as usize
                    && raw.len() <= self.max_tensor_bytes)
                {
                    return None;
                }
                let byte_len = raw.len() as u32;
                let resident = if crate::llm_bench::resident_weights_enabled() {
                    self.resident_weight_buffer(raw.as_ptr() as u64, raw)
                } else {
                    None
                };
                let weight_binding: &wgpu::Buffer = match resident.as_ref() {
                    Some(r) => r,
                    None => {
                        self.write_weight_words(raw, self.max_tensor_bytes);
                        weight_buf
                    }
                };

                let gparams = GemmGpuParams {
                    n_in: n_in as u32,
                    n_out: chunk_rows as u32,
                    weight_ggml_type: info.ggml_type,
                    weight_row_elems: info.dims[0] as u32,
                    weight_byte_len: byte_len,
                    n_batch: 1,
                    in_row_stride: 0,
                    out_row_stride: 0,
                };
                self.gpu_queue()
                    .write_buffer(params_buf, 0, bytemuck::bytes_of(&gparams));
                let tparams =
                    crate::topk::topk_params_bytes(chunk_rows as u32, 1, block_size as u32);
                self.gpu_queue().write_buffer(topk_params_buf, 0, &tparams);

                let num_blocks = chunk_rows.div_ceil(block_size);
                let cand_count = num_blocks;

                let mut gemm_entries = vec![
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
                ];
                if use_coop {
                    gemm_entries.push(wgpu::BindGroupEntry {
                        binding: 4,
                        resource: input_buf.as_entire_binding(),
                    });
                }
                let gemm_bind = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Top1GemmBind"),
                    layout: &gemm_layout,
                    entries: &gemm_entries,
                });
                let mut encoder =
                    self.device()
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Top1Encoder"),
                        });
                {
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Top1GemmPass"),
                        timestamp_writes: crate::llm_gpu_profiler::pass_writes_begin(),
                    });
                    cpass.set_pipeline(gemm_pipeline);
                    cpass.set_bind_group(0, &gemm_bind, &[]);
                    if use_mr {
                        cpass.dispatch_workgroups(
                            crate::llm_bench::coop_gemv_workgroups(chunk_rows as u32),
                            1,
                            1,
                        );
                    } else if use_coop {
                        cpass.dispatch_workgroups(chunk_rows as u32, 1, 1);
                    } else {
                        cpass.dispatch_workgroups((chunk_rows as u32 + 63) / 64, 1, 1);
                    }
                }
                {
                    let mut tpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Top1ReducePass"),
                        timestamp_writes: crate::llm_gpu_profiler::pass_writes_end(),
                    });
                    tpass.set_pipeline(topk_pipeline);
                    tpass.set_bind_group(0, &topk_bind, &[]);
                    tpass.dispatch_workgroups(num_blocks as u32, 1, 1);
                }
                let cand_bytes = (cand_count * 4) as wgpu::BufferAddress;
                let val_dst = (cand_offset * 4) as wgpu::BufferAddress;
                let idx_dst = ((total_cands + cand_offset) * 4) as wgpu::BufferAddress;
                encoder.copy_buffer_to_buffer(cand_val, 0, staging, val_dst, cand_bytes);
                encoder.copy_buffer_to_buffer(cand_idx, 0, staging, idx_dst, cand_bytes);
                crate::llm_gpu_profiler::resolve(&mut encoder);
                self.gpu_queue().submit(Some(encoder.finish()));
                crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::OutputTopk);
                cand_offset += cand_count;
            }
        }

        let map_bytes = (total_cands * 8) as wgpu::BufferAddress;
        let slice = staging.slice(..map_bytes);
        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.poll_wait();
        let mapped_ok = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.block_on(rx).ok().map(|m| m.is_ok()).unwrap_or(false)
        } else {
            false
        };
        if !mapped_ok {
            let _ = staging.unmap();
            return None;
        }

        let mut best_token_id = 0u32;
        let mut max_logit = f32::NEG_INFINITY;
        {
            let data = slice.get_mapped_range();
            let val_bytes = total_cands * 4;
            let vals: &[f32] = bytemuck::cast_slice(&data[..val_bytes]);
            let idxs: &[u32] = bytemuck::cast_slice(&data[val_bytes..val_bytes * 2]);
            let mut offset = 0usize;
            for chunk_idx in 0..full_chunks {
                let row_start = chunk_idx * VOCAB_CHUNK_ROWS;
                let chunk_rows = VOCAB_CHUNK_ROWS.min(vocab_size - row_start);
                let cand_count = chunk_rows.div_ceil(block_size);
                for i in 0..cand_count {
                    let pos = offset + i;
                    let v = vals[pos];
                    let token_id = row_start as u32 + idxs[pos];
                    if v > f32::NEG_INFINITY
                        && (v > max_logit || (v == max_logit && token_id < best_token_id))
                    {
                        max_logit = v;
                        best_token_id = token_id;
                    }
                }
                offset += cand_count;
            }
        }
        staging.unmap();

        if max_logit == f32::NEG_INFINITY {
            None
        } else {
            Some(StreamingArgmaxResult {
                best_token_id,
                max_logit,
            })
        }
    }

    /// A1a: GPU top-k over the output projection — the logits stay on-GPU (`gemm_output_buf`), the
    /// top-k kernel reduces them per chunk, and only K `(id, logit)` candidates are read back (vs the
    /// 196 KB/token full-logit readback + CPU argmax in `dispatch_output_argmax_chunked`). Returns the
    /// merged global top-K, or `None` to signal the caller to fall back to the argmax path. v1: no
    /// sieve coupling (caller routes here only when no sieve mask is active).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn dispatch_output_topk_chunked(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        k: usize,
    ) -> Option<Vec<crate::topk::TopKItem>> {
        let info = index.logits_projection_info()?;
        let (n_in, vocab_size) = Self::matmul_dims(info);
        if n_in == 0 || vocab_size == 0 || n_in > emb_dim || n_in > hidden.len() {
            return None;
        }
        let pipeline = self.output_topk_pipeline.as_ref()?;
        let topk_layout = self.output_topk_bind_layout.as_ref()?;
        let input_buf = self.gemm_input_buf.as_ref()?;
        let weight_buf = self.gemm_weight_buf.as_ref()?;
        let output_buf = self.gemm_output_buf.as_ref()?;
        let params_buf = self.gemm_params_buf.as_ref()?;
        let topk_params_buf = self.topk_params_buf.as_ref()?;
        let cand_val = self.topk_cand_val_buf.as_ref()?;
        let cand_idx = self.topk_cand_idx_buf.as_ref()?;
        let staging = self.topk_cand_staging.as_ref()?;
        let mmap = self.gguf_mmap.as_deref()?;

        let k = k.clamp(1, crate::topk::TOPK_MAX_K);
        let block_size = crate::topk::TOPK_BLOCK_SIZE;
        let full_chunks = vocab_size.div_ceil(VOCAB_CHUNK_ROWS);
        let use_coop = crate::llm_bench::coop_gemv_enabled();
        let use_mr = use_coop
            && info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K_SOA;
        let gemm_pipeline: &wgpu::ComputePipeline = if use_mr {
            &self.coop_gemv_mr_pipeline
        } else if use_coop {
            &self.coop_gemv_pipeline
        } else {
            &self.pipeline
        };
        let gemm_layout = self.native_gemm_bind_layout(use_coop).clone();
        let topk_bind = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("TopkBind"),
                layout: topk_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: output_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: topk_params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: cand_val.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: cand_idx.as_entire_binding(),
                    },
                ],
            });

        let mut all_val: Vec<f32> = Vec::new();
        let mut all_idx: Vec<u32> = Vec::new();

        // A1a step-2: when the output projection is resident (uploaded once at init), bind the
        // per-chunk sub-range - zero per-token upload. `VOCAB_CHUNK_ROWS` is a multiple of 256, so
        // every chunk offset is storage-binding aligned. The bound bytes, quant, shader and params
        // are identical to the per-chunk-upload fallback, so logits are byte-for-byte equal.
        let resident_logits = self.mc8_logits_resident_buf.as_ref();
        let resident_row_bytes = self.mc8_logits_row_bytes as u64;

        for chunk_idx in 0..full_chunks {
            let row_start = chunk_idx * VOCAB_CHUNK_ROWS;
            let chunk_rows = VOCAB_CHUNK_ROWS.min(vocab_size - row_start);

            let (weight_resource, weight_byte_len) = if let Some(buf) = resident_logits {
                // Only the GPU-quant fast path is supported; otherwise signal fallback to argmax.
                if !(ggml_gpu_gemm_supported(info.ggml_type)
                    && n_in <= MAX_STACK_GEMM_IN
                    && chunk_rows <= self.gemm_max_out_dim as usize)
                {
                    return None;
                }
                let byte_len = chunk_rows as u64 * resident_row_bytes;
                let res = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: buf,
                    offset: row_start as u64 * resident_row_bytes,
                    size: std::num::NonZeroU64::new(byte_len),
                });
                (res, byte_len as u32)
            } else {
                let raw = crate::ggml_quants::fetch_tensor_row_range_bytes(
                    mmap,
                    index.tensor_data_start,
                    info,
                    row_start,
                    chunk_rows,
                )
                .ok()?;
                if !(ggml_gpu_gemm_supported(info.ggml_type)
                    && n_in <= MAX_STACK_GEMM_IN
                    && chunk_rows <= self.gemm_max_out_dim as usize
                    && raw.len() <= self.max_tensor_bytes)
                {
                    return None;
                }
                let byte_len = raw.len() as u32;
                self.write_weight_words(raw, self.max_tensor_bytes);
                (weight_buf.as_entire_binding(), byte_len)
            };

            let gparams = GemmGpuParams {
                n_in: n_in as u32,
                n_out: chunk_rows as u32,
                weight_ggml_type: info.ggml_type,
                weight_row_elems: info.dims[0] as u32,
                weight_byte_len,
                n_batch: 1,
                in_row_stride: 0,
                out_row_stride: 0,
            };
            self.gpu_queue()
                .write_buffer(input_buf, 0, bytemuck::cast_slice(&hidden[..n_in]));
            self.gpu_queue()
                .write_buffer(params_buf, 0, bytemuck::bytes_of(&gparams));
            let tparams =
                crate::topk::topk_params_bytes(chunk_rows as u32, k as u32, block_size as u32);
            self.gpu_queue().write_buffer(topk_params_buf, 0, &tparams);

            let num_blocks = chunk_rows.div_ceil(block_size);
            let cand_count = num_blocks * k;

            let mut gemm_entries = vec![
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weight_resource,
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buf.as_entire_binding(),
                },
            ];
            if use_coop {
                gemm_entries.push(wgpu::BindGroupEntry {
                    binding: 4,
                    resource: input_buf.as_entire_binding(),
                });
            }
            let gemm_bind = self
                .gpu_device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("TopkGemmBind"),
                    layout: &gemm_layout,
                    entries: &gemm_entries,
                });
            let mut encoder =
                self.device()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("TopkEncoder"),
                    });
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("TopkGemmPass"),
                    timestamp_writes: crate::llm_gpu_profiler::pass_writes_begin(),
                });
                cpass.set_pipeline(gemm_pipeline);
                cpass.set_bind_group(0, &gemm_bind, &[]);
                if use_mr {
                    cpass.dispatch_workgroups(
                        crate::llm_bench::coop_gemv_workgroups(chunk_rows as u32),
                        1,
                        1,
                    );
                } else if use_coop {
                    cpass.dispatch_workgroups(chunk_rows as u32, 1, 1);
                } else {
                    cpass.dispatch_workgroups((chunk_rows as u32 + 63) / 64, 1, 1);
                }
            }
            {
                let mut tpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("TopkReducePass"),
                    timestamp_writes: crate::llm_gpu_profiler::pass_writes_end(),
                });
                tpass.set_pipeline(pipeline);
                tpass.set_bind_group(0, &topk_bind, &[]);
                tpass.dispatch_workgroups(num_blocks as u32, 1, 1);
            }
            let cand_bytes = (cand_count * 4) as wgpu::BufferAddress;
            encoder.copy_buffer_to_buffer(cand_val, 0, staging, 0, cand_bytes);
            encoder.copy_buffer_to_buffer(cand_idx, 0, staging, cand_bytes, cand_bytes);
            crate::llm_gpu_profiler::resolve(&mut encoder);
            self.gpu_queue().submit(Some(encoder.finish()));
            crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::OutputTopk);

            let map_bytes = cand_bytes * 2;
            let slice = staging.slice(..map_bytes);
            let (tx, rx) = futures_channel::oneshot::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            self.poll_wait();
            let mapped_ok = if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.block_on(rx).ok().map(|m| m.is_ok()).unwrap_or(false)
            } else {
                false
            };
            if !mapped_ok {
                let _ = staging.unmap();
                return None;
            }
            {
                let data = slice.get_mapped_range();
                let vals: &[f32] = bytemuck::cast_slice(&data[..cand_count * 4]);
                let idxs: &[u32] = bytemuck::cast_slice(&data[cand_count * 4..cand_count * 8]);
                for i in 0..cand_count {
                    let v = vals[i];
                    if v > f32::NEG_INFINITY {
                        all_val.push(v);
                        all_idx.push(row_start as u32 + idxs[i]);
                    }
                }
            }
            staging.unmap();
        }

        let top = crate::topk::merge_block_candidates(&all_val, &all_idx, k, None);
        if top.is_empty() {
            None
        } else {
            Some(top)
        }
    }

    /// Final `output_norm` RMSNorm in-place before vocabulary projection (Pre-Norm LLM tail).
    /// REQUIRED on all targets — native previously skipped it → logits from an un-normed hidden.
    pub fn apply_output_norm_inplace(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &mut [f32],
        emb_dim: usize,
    ) -> bool {
        let info = match index.output_norm_info() {
            Some(i) => i,
            None => return true,
        };
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let n_embd = index.hyperparams.n_embd as usize;
        let n = emb_dim.min(n_embd).min(hidden.len());
        let mut norm_w = [0f32; MAX_HIDDEN_DIM];
        if dequant_norm_row_into(mmap, index.tensor_data_start, info, &mut norm_w) < n {
            return false;
        }
        rms_norm_inplace(&mut hidden[..n], &norm_w[..n], RMS_NORM_EPS);
        true
    }
}
