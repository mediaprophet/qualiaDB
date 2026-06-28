//! FFN: pre-norm SwiGLU/ReLU-gated block (sync + async)
//! Split from gguf_bridge/mod.rs (structural refactor; no behaviour change).
use super::*;

impl QTensorEngine {
    /// Pre-norm FFN: RMSNorm(hidden) → SwiGLU (wasm) or ReLU-gated (native) → residual add.
    pub(crate) fn dispatch_ffn_block_pre_norm(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        // Phase 3: try the single-submit fused FFN first (one round-trip/layer). Requires resident
        // weights; on any ineligibility it returns false and we fall through to the per-GEMM path.
        // 0.0.22: fused FFN now selects the cooperative GEMV entry point for its gate/up/down GEMMs
        // when enabled, so the FFN gets both wins: one readback per layer and the parallel row
        // reduction kernel.
        #[cfg(not(target_arch = "wasm32"))]
        if crate::llm_bench::resident_weights_enabled() && crate::llm_bench::ffn_fusion_enabled() {
            if self.dispatch_ffn_fused_resident(index, hidden, emb_dim, tensors, scratch_a) {
                return true;
            }
        }
        // SwiGLU FFN with ffn_norm pre-norm — REQUIRED on all targets (native previously ran a
        // norm-less ReLU chain on the raw residual → exponential blow-up to inf).
        {
            let mut norm_w_ffn = [0f32; MAX_HIDDEN_DIM];
            let mut h_norm_ffn = [0f32; MAX_HIDDEN_DIM];
            let ffn_input = prepare_pre_norm_input(
                &hidden[..emb_dim],
                emb_dim,
                tensors.ffn_norm.as_ref(),
                self.gguf_mmap.as_deref().map(|m| &m[..]),
                index.tensor_data_start,
                &mut h_norm_ffn,
                &mut norm_w_ffn,
            );
            let gate_info = match tensors.ffn_gate.as_ref() {
                Some(i) => i,
                None => return false,
            };
            let up_info = match tensors.ffn_up.as_ref() {
                Some(i) => i,
                None => return false,
            };
            let down_info = match tensors.ffn_down.as_ref() {
                Some(i) => i,
                None => return false,
            };
            let (gate_in, n_ffn) = Self::matmul_dims(gate_info);
            let (up_in, up_out) = Self::matmul_dims(up_info);
            let (dn_in, dn_out) = Self::matmul_dims(down_info);
            if gate_in > emb_dim
                || up_in != gate_in
                || up_out != n_ffn
                || dn_in != n_ffn
                || n_ffn > MAX_STACK_GEMM_DIM
                || dn_out > scratch_a.len()
            {
                return false;
            }
            // AWQ step 1 (calibration only; no-op in production): record per-input-channel salience
            // at the post-ffn_norm FFN input — the activation that scales the gate/up projections.
            crate::llm_awq::record_ffn_input(&ffn_input[..gate_in]);
            let mut gate_buf = [0f32; MAX_STACK_GEMM_DIM];
            let mut up_buf = [0f32; MAX_STACK_GEMM_DIM];
            if !self.dispatch_gemm_into(
                index,
                gate_info,
                &ffn_input[..gate_in],
                &mut gate_buf[..n_ffn],
                gate_in,
                n_ffn,
            ) {
                return false;
            }
            if !self.dispatch_gemm_into(
                index,
                up_info,
                &ffn_input[..up_in],
                &mut up_buf[..n_ffn],
                up_in,
                n_ffn,
            ) {
                return false;
            }
            silu_inplace(&mut gate_buf[..n_ffn], n_ffn);
            for i in 0..n_ffn {
                gate_buf[i] *= up_buf[i];
            }
            if !self.dispatch_gemm_into(
                index,
                down_info,
                &gate_buf[..dn_in],
                scratch_a,
                dn_in,
                dn_out,
            ) {
                return false;
            }
            add_residual_inplace(
                &mut hidden[..emb_dim],
                &scratch_a[..dn_out],
                emb_dim.min(dn_out),
            );
            return true;
        }
    }

    /// Phase 3: the entire pre-norm FFN — gate GEMM → up GEMM → GPU SiLU·mul → down GEMM → CPU
    /// residual — in ONE command submit, with the n_ffn intermediates kept in VRAM. Replaces the
    /// three separate GEMM dispatches of [`Self::dispatch_ffn_block_pre_norm`] (each a submit +
    /// blocking readback, with a CPU SiLU·mul between) with a single submit→wait round-trip per
    /// layer. Binds resident weight buffers (Phase 2), so it requires resident weights; returns
    /// `false` (→ caller falls back to the per-GEMM path) on any ineligibility or map failure.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn dispatch_ffn_fused_resident(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
    ) -> bool {
        // Clone the mmap Arc so weight slices don't borrow `self` (we mutate `self` below).
        let mmap_arc = match self.gguf_mmap.clone() {
            Some(a) => a,
            None => return false,
        };
        let mmap: &[u8] = &mmap_arc;

        let gate_info = match tensors.ffn_gate.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let up_info = match tensors.ffn_up.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let down_info = match tensors.ffn_down.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let (gate_in, n_ffn) = Self::matmul_dims(gate_info);
        let (up_in, up_out) = Self::matmul_dims(up_info);
        let (dn_in, dn_out) = Self::matmul_dims(down_info);
        // Same shape contract as the per-GEMM path.
        if gate_in > emb_dim
            || up_in != gate_in
            || up_out != n_ffn
            || dn_in != n_ffn
            || n_ffn > MAX_STACK_GEMM_DIM
            || dn_out > scratch_a.len()
        {
            return false;
        }
        // GPU-eligibility: all three must run the GPU GEMM (else fall back so behaviour matches).
        if !(ggml_gpu_gemm_supported(gate_info.ggml_type)
            && ggml_gpu_gemm_supported(up_info.ggml_type)
            && ggml_gpu_gemm_supported(down_info.ggml_type))
        {
            return false;
        }
        if n_ffn > self.gemm_max_out_dim as usize
            || dn_out > self.gemm_max_out_dim as usize
            || gate_in > self.gemm_max_input_floats as usize
            || n_ffn > self.gemm_max_input_floats as usize
        {
            return false;
        }
        let gate_raw = match crate::ggml_quants::fetch_tensor_bytes(
            mmap,
            index.tensor_data_start,
            gate_info,
        ) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let up_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, up_info) {
                Ok(s) => s,
                Err(_) => return false,
            };
        let down_raw = match crate::ggml_quants::fetch_tensor_bytes(
            mmap,
            index.tensor_data_start,
            down_info,
        ) {
            Ok(s) => s,
            Err(_) => return false,
        };
        if gate_raw.len() > self.max_tensor_bytes
            || up_raw.len() > self.max_tensor_bytes
            || down_raw.len() > self.max_tensor_bytes
        {
            return false;
        }

        // CPU pre-norm (cheap), then upload the FFN input once.
        let mut norm_w_ffn = [0f32; MAX_HIDDEN_DIM];
        let mut h_norm_ffn = [0f32; MAX_HIDDEN_DIM];
        let ffn_input = prepare_pre_norm_input(
            &hidden[..emb_dim],
            emb_dim,
            tensors.ffn_norm.as_ref(),
            Some(mmap),
            index.tensor_data_start,
            &mut h_norm_ffn,
            &mut norm_w_ffn,
        );
        // AWQ calibration hook (no-op in production) — same as the per-GEMM path.
        crate::llm_awq::record_ffn_input(&ffn_input[..gate_in]);

        // Resident weight buffers (Phase 2). Bail to the per-GEMM path if any is unavailable.
        let rg = match self.resident_weight_buffer(gate_raw.as_ptr() as u64, gate_raw) {
            Some(b) => b,
            None => return false,
        };
        let ru = match self.resident_weight_buffer(up_raw.as_ptr() as u64, up_raw) {
            Some(b) => b,
            None => return false,
        };
        let rd = match self.resident_weight_buffer(down_raw.as_ptr() as u64, down_raw) {
            Some(b) => b,
            None => return false,
        };

        // GEMM params at 256-aligned uniform sub-ranges: gate@0, up@256, down@512.
        const SLOT: wgpu::BufferAddress = 256;
        let p_gate = GemmGpuParams {
            n_in: gate_in as u32,
            n_out: n_ffn as u32,
            weight_ggml_type: gate_info.ggml_type,
            weight_row_elems: gate_info.dims[0] as u32,
            weight_byte_len: gate_raw.len() as u32,
            n_batch: 1,
            in_row_stride: 0,
            out_row_stride: 0,
        };
        let p_up = GemmGpuParams {
            n_in: up_in as u32,
            n_out: n_ffn as u32,
            weight_ggml_type: up_info.ggml_type,
            weight_row_elems: up_info.dims[0] as u32,
            weight_byte_len: up_raw.len() as u32,
            n_batch: 1,
            in_row_stride: 0,
            out_row_stride: 0,
        };
        let p_down = GemmGpuParams {
            n_in: n_ffn as u32,
            n_out: dn_out as u32,
            weight_ggml_type: down_info.ggml_type,
            weight_row_elems: down_info.dims[0] as u32,
            weight_byte_len: down_raw.len() as u32,
            n_batch: 1,
            in_row_stride: 0,
            out_row_stride: 0,
        };
        let p_silu = ElemGpuParams {
            n: n_ffn as u32,
            batch: 1,
            op: ELEM_OP_SILU_MUL,
            eps: RMS_NORM_EPS,
            a_row_stride: 0,
            b_row_stride: 0,
            out_row_stride: 0,
            a_slot: 0,
            b_slot: 0,
            out_slot: 0,
            _pad: 0,
        };

        // Lazily create the 3-slot fused GEMM params uniform (&mut self — done before the shared borrows).
        if self.ffn_fused_params.is_none() {
            self.ffn_fused_params =
                Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                    label: Some("FfnFusedGemmParams"),
                    size: SLOT * 3,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
        }

        // Work buffers: in/G/U/S distinct; D (down output) reuses the now-dead input buffer — its
        // only readers (gate/up) are encoded before D's write, so wgpu serializes with a barrier.
        let (params_buf, in_buf, g_buf, u_buf, s_buf, staging, elem_params) = match (
            self.ffn_fused_params.as_ref(),
            self.gemm_input_buf.as_ref(),
            self.gemm_aux_buf.as_ref(),
            self.gemm_ffn_buf.as_ref(),
            self.gemm_output_buf.as_ref(),
            self.gemm_output_staging.as_ref(),
            self.elem_params_buf.as_ref(),
        ) {
            (Some(p), Some(i), Some(g), Some(u), Some(s), Some(st), Some(e)) => {
                (p, i, g, u, s, st, e)
            }
            _ => return false,
        };
        let d_buf = in_buf;
        let device = self.gpu_device();

        self.gpu_queue()
            .write_buffer(params_buf, 0, bytemuck::bytes_of(&p_gate));
        self.gpu_queue()
            .write_buffer(params_buf, SLOT, bytemuck::bytes_of(&p_up));
        self.gpu_queue()
            .write_buffer(params_buf, SLOT * 2, bytemuck::bytes_of(&p_down));
        self.gpu_queue()
            .write_buffer(elem_params, 0, bytemuck::bytes_of(&p_silu));
        self.gpu_queue()
            .write_buffer(in_buf, 0, bytemuck::cast_slice(&ffn_input[..gate_in]));

        let use_coop = crate::llm_bench::coop_gemv_enabled();
        let gemm_pipeline: &wgpu::ComputePipeline = if use_coop {
            &self.coop_gemv_pipeline
        } else {
            &self.pipeline
        };
        let gemm_layout = self.native_gemm_bind_layout(use_coop).clone();
        let elem_layout = self.elem_silu_mul_bind_layout.clone();
        let gp_sz = std::num::NonZeroU64::new(std::mem::size_of::<GemmGpuParams>() as u64);
        let ep_sz = std::num::NonZeroU64::new(std::mem::size_of::<ElemGpuParams>() as u64);
        let gemm_params_at = |slot: wgpu::BufferAddress| {
            wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: params_buf,
                offset: slot,
                size: gp_sz,
            })
        };

        let gate_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FfnGateBG"),
            layout: &gemm_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: in_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rg.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: gemm_params_at(0),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: g_buf.as_entire_binding(),
                },
            ],
        });
        let up_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FfnUpBG"),
            layout: &gemm_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: in_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: ru.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: gemm_params_at(SLOT),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: u_buf.as_entire_binding(),
                },
            ],
        });
        let silu_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FfnSiluBG"),
            layout: &elem_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: g_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: u_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: s_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: elem_params,
                        offset: 0,
                        size: ep_sz,
                    }),
                },
            ],
        });
        let down_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FfnDownBG"),
            layout: &gemm_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: s_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rd.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: gemm_params_at(SLOT * 2),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: d_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("FfnFusedEncoder"),
            });
        let gate_groups = if use_coop {
            n_ffn as u32
        } else {
            (n_ffn as u32 + 63) / 64
        };
        let up_groups = gate_groups;
        let down_groups = if use_coop {
            dn_out as u32
        } else {
            (dn_out as u32 + 63) / 64
        };
        for (idx, (label, pipe, bg, groups)) in [
            ("FfnGate", gemm_pipeline, &gate_bg, gate_groups),
            ("FfnUp", gemm_pipeline, &up_bg, up_groups),
            (
                "FfnSilu",
                &self.elem_silu_mul_pipeline,
                &silu_bg,
                (n_ffn as u32 + 63) / 64,
            ),
            ("FfnDown", gemm_pipeline, &down_bg, down_groups),
        ]
        .into_iter()
        .enumerate()
        {
            let timestamp_writes = match idx {
                0 => crate::llm_gpu_profiler::pass_writes_begin(),
                3 => crate::llm_gpu_profiler::pass_writes_end(),
                _ => None,
            };
            let mut cp = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(label),
                timestamp_writes,
            });
            cp.set_pipeline(pipe);
            cp.set_bind_group(0, bg, &[]);
            cp.dispatch_workgroups(groups, 1, 1);
        }
        let out_bytes = (dn_out * 4) as wgpu::BufferAddress;
        encoder.copy_buffer_to_buffer(d_buf, 0, staging, 0, out_bytes);
        crate::llm_gpu_profiler::resolve(&mut encoder);
        self.gpu_queue().submit(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Gemm);

        let slice = staging.slice(..out_bytes);
        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.poll_wait();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            if handle.block_on(rx).ok().map(|m| m.is_ok()).unwrap_or(false) {
                let data = slice.get_mapped_range();
                let floats: &[f32] = bytemuck::cast_slice(&data);
                scratch_a[..dn_out].copy_from_slice(&floats[..dn_out]);
                drop(data);
                staging.unmap();
                add_residual_inplace(
                    &mut hidden[..emb_dim],
                    &scratch_a[..dn_out],
                    emb_dim.min(dn_out),
                );
                return true;
            }
        }
        let _ = staging.unmap();
        false
    }

    /// Phase 2B: SwiGLU FFN block with async GEMM readback (`map_async` + `await`).
    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn dispatch_ffn_block_pre_norm_async(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        let mut norm_w_ffn = [0f32; MAX_HIDDEN_DIM];
        let mut h_norm_ffn = [0f32; MAX_HIDDEN_DIM];
        let ffn_input = prepare_pre_norm_input(
            &hidden[..emb_dim],
            emb_dim,
            tensors.ffn_norm.as_ref(),
            self.gguf_mmap.as_deref(),
            index.tensor_data_start,
            &mut h_norm_ffn,
            &mut norm_w_ffn,
        );
        let gate_info = match tensors.ffn_gate.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let up_info = match tensors.ffn_up.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let down_info = match tensors.ffn_down.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let (gate_in, n_ffn) = Self::matmul_dims(gate_info);
        let (up_in, up_out) = Self::matmul_dims(up_info);
        let (dn_in, dn_out) = Self::matmul_dims(down_info);
        if gate_in > emb_dim
            || up_in != gate_in
            || up_out != n_ffn
            || dn_in != n_ffn
            || n_ffn > MAX_STACK_GEMM_DIM
            || dn_out > scratch_a.len()
        {
            return false;
        }
        let mut gate_buf = [0f32; MAX_STACK_GEMM_DIM];
        let mut up_buf = [0f32; MAX_STACK_GEMM_DIM];
        if !self
            .dispatch_gemm_into_async(
                index,
                gate_info,
                &ffn_input[..gate_in],
                &mut gate_buf[..n_ffn],
                gate_in,
                n_ffn,
            )
            .await
        {
            return false;
        }
        if !self
            .dispatch_gemm_into_async(
                index,
                up_info,
                &ffn_input[..up_in],
                &mut up_buf[..n_ffn],
                up_in,
                n_ffn,
            )
            .await
        {
            return false;
        }
        silu_inplace(&mut gate_buf[..n_ffn], n_ffn);
        for i in 0..n_ffn {
            gate_buf[i] *= up_buf[i];
        }
        if !self
            .dispatch_gemm_into_async(
                index,
                down_info,
                &gate_buf[..dn_in],
                scratch_a,
                dn_in,
                dn_out,
            )
            .await
        {
            return false;
        }
        add_residual_inplace(
            &mut hidden[..emb_dim],
            &scratch_a[..dn_out],
            emb_dim.min(dn_out),
        );
        true
    }
}
