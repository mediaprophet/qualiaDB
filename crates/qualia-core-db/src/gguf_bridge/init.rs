//! Engine init/lifecycle: GPU device/queue accessors, try_new, new, KV-cache + GEMM-buffer
//! allocation/reset. Split from gguf_bridge/mod.rs (structural; no behaviour change).
use super::*;

impl QTensorEngine {
    /// Resolve the MC8 elementwise GPU pipeline for a given opcode.
    #[cfg(feature = "gpu-runtime")]
    pub(crate) fn elem_gpu_pipeline(&self, op: u32) -> Option<&etalon::ComputePipeline> {
        use super::gpu_params::{ELEM_OP_ADD_RESIDUAL, ELEM_OP_RMS_NORM};
        match op {
            ELEM_OP_RMS_NORM => Some(&self.elem_rms_norm_pipeline),
            ELEM_OP_ADD_RESIDUAL => Some(&self.elem_add_residual_pipeline),
            _ => None,
        }
    }
    #[cfg(feature = "gpu-runtime")]
    /// wgpu device.
    pub(crate) fn gpu_device(&self) -> &etalon::Device {
        #[cfg(target_arch = "wasm32")]
        {
            return &self.device;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            &crate::gpu_context::shared_gpu().device
        }
    }

    #[inline]
    #[cfg(feature = "gpu-runtime")]
    /// wgpu queue.
    pub(crate) fn gpu_queue(&self) -> &etalon::Queue {
        #[cfg(target_arch = "wasm32")]
        {
            return &self.queue;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            &crate::gpu_context::shared_gpu().queue
        }
    }

    /// Shared process-wide wgpu device (LLM + render coexistence).
    #[inline]
    #[cfg(feature = "gpu-runtime")]
    pub fn device(&self) -> &etalon::Device {
        self.gpu_device()
    }

    /// Shared process-wide wgpu queue.
    #[inline]
    #[cfg(feature = "gpu-runtime")]
    pub fn queue(&self) -> &etalon::Queue {
        self.gpu_queue()
    }

    #[cfg(feature = "gpu-runtime")]
    /// Tries to build the engine; error when GPU init fails.
    pub async fn try_new() -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        log::info!(
            "LLM_LOAD|engine-init|0.10|Initializing native GGUF runtime (shared GpuContext)"
        );
        #[cfg(target_arch = "wasm32")]
        log::info!("LLM_LOAD|engine-init|0.10|Initializing WASM GGUF runtime");

        #[cfg(not(target_arch = "wasm32"))]
        let shared = crate::gpu_context::shared_gpu();
        #[cfg(not(target_arch = "wasm32"))]
        let device = &shared.device;
        #[cfg(not(target_arch = "wasm32"))]
        let _queue = &shared.queue;
        #[cfg(not(target_arch = "wasm32"))]
        log::info!("LLM_LOAD|gpu-device|0.35|Reusing process-wide wgpu device");
        // NVIDIA: create CUDA multi-weight context early so the GPU leaves idle clocks
        // even for portable/FastVerify resident decode (measured 3B ~1.5 → ~7 tok/s).
        #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
        {
            let _ = crate::warm_cuda_context();
        }

        #[cfg(target_arch = "wasm32")]
        let (wasm_device, wasm_queue) = {
            crate::gguf_bridge::wasm_yield::phase("Acquiring shared WebGPU device…").await;
            crate::gpu_context::ensure_shared_gpu().await?;
            let shared = crate::gpu_context::try_shared_gpu().ok_or_else(|| {
                "shared WebGPU device missing after ensure_shared_gpu".to_string()
            })?;
            let info = shared.adapter.get_info();
            log::info!(
                "LLM_LOAD|webgpu-adapter|0.20|name={} backend={:?} device={:?} vendor={:?}",
                info.name,
                info.backend,
                info.device,
                info.vendor
            );
            (shared.device.clone(), shared.queue.clone())
        };
        #[cfg(target_arch = "wasm32")]
        let device = &wasm_device;
        // One yield after device so the UI paints before pipeline compile freezes the main thread.
        #[cfg(target_arch = "wasm32")]
        crate::gguf_bridge::wasm_yield::phase(
            "Compiling compute pipelines (UI may pause 30–90s on phones — leave tab open)…",
        )
        .await;
        #[cfg(not(target_arch = "wasm32"))]
        let native_pipeline_cache = create_native_pipeline_cache(device);
        #[cfg(target_arch = "wasm32")]
        let native_pipeline_cache: Option<etalon::PipelineCache> = None;

        let resources = etalon_inference::gpu_resources::EngineResources::build(
            device,
            &native_pipeline_cache,
            etalon_inference::gpu_resources::PipelineOptions {
                subgroups: device.features().contains(etalon::Features::SUBGROUP),
            },
            etalon_inference::gpu_resources::ResourceBudget::for_device(device),
            |_| Ok(()),
        )
        .map_err(|error| format!("inference GPU resources: {error:?}"))?;
        let etalon_inference::gpu_resources::EngineResources {
            gemv,
            common,
            ffn,
            projections,
            #[cfg(not(target_arch = "wasm32"))]
            attention_kv_gemm_params,
            #[cfg(not(target_arch = "wasm32"))]
            attention_kv_params,
        } = resources;
        let etalon_inference::gpu_resources::GemvPipelines {
            pipeline,
            #[cfg(target_arch = "wasm32")]
            mmv_q8_0_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_residual_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_bind_layout,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_residual_bind_layout,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_mr_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_residual_mr_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_warp_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_residual_warp_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            mock_pipeline,
            #[cfg(target_arch = "wasm32")]
            mc8_gemm_bind_layout,
            #[cfg(not(target_arch = "wasm32"))]
            pipeline_bind_layout,
        } = gemv;
        let etalon_inference::gpu_resources::CommonPipelines {
            embedding_pipeline,
            attention_pipeline,
            elem_rms_norm_pipeline,
            elem_silu_mul_pipeline,
            elem_add_residual_pipeline,
            #[cfg(target_arch = "wasm32")]
            mc8_attn_bind_layout,
            #[cfg(target_arch = "wasm32")]
            mc8_elem_bind_layout,
            #[cfg(not(target_arch = "wasm32"))]
            embedding_bind_layout,
            #[cfg(not(target_arch = "wasm32"))]
            attention_bind_layout,
            #[cfg(not(target_arch = "wasm32"))]
            elem_silu_mul_bind_layout,
        } = common;
        let etalon_inference::gpu_resources::FfnPipelines {
            #[cfg(target_arch = "wasm32")]
            mc8_ffn_fused_bind_layout,
            #[cfg(target_arch = "wasm32")]
            mc8_ffn_fused_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            ffn_fused_bind_layout,
            #[cfg(not(target_arch = "wasm32"))]
            ffn_fused_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            ffn_fused_coop_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            ffn_fused_mr_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            ffn_fused_warp_pipeline,
        } = ffn;
        let etalon_inference::gpu_resources::ProjectionsPipelines {
            #[cfg(not(target_arch = "wasm32"))]
            dual_gemv_bind_layout,
            #[cfg(not(target_arch = "wasm32"))]
            dual_gemv_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            dual_gemv_mr_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            triple_gemv_bind_layout,
            #[cfg(not(target_arch = "wasm32"))]
            triple_gemv_pipeline,
        } = projections;
        #[cfg(not(target_arch = "wasm32"))]
        log::info!(
            "LLM_LOAD|fused_ffn|coop_entry={}|mr=coop_fused_ffn_mr|warp=coop_fused_ffn_warp",
            if device.features().contains(etalon::Features::SUBGROUP) {
                "coop_fused_ffn_sg"
            } else {
                "coop_fused_ffn_expansion"
            }
        );

        // DirectML is a *second* D3D12 device next to wgpu. Always-on init competed for
        // A2000 VRAM and added driver overhead while the resident decode path uses wgpu.
        // Opt in with QUALIA_DIRECTML=1 (or QUALIA_LLM_DIRECTML=1). Default: wgpu-only.
        #[cfg(target_os = "windows")]
        let dml_status = {
            let want = matches!(
                crate::gguf_bridge::execution_env::var("QUALIA_DIRECTML")
                    .or_else(|_| crate::gguf_bridge::execution_env::var("QUALIA_LLM_DIRECTML"))
                    .ok()
                    .as_deref(),
                Some("1") | Some("true") | Some("on")
            );
            if !want {
                log::info!(
                    "LLM_LOAD|gpu-backend|0.45|DirectML deferred (set QUALIA_DIRECTML=1 to enable second D3D12 device)"
                );
                None
            } else {
                match crate::directml_bridge::DmlDevice::new() {
                    Ok(device) => {
                        log::info!(
                            "DirectML device initialization: Ok({})",
                            device.adapter_desc
                        );
                        log::info!(
                            "LLM_LOAD|gpu-backend|0.45|DirectML ready on {}",
                            device.adapter_desc
                        );
                        log::info!(
                            "LLM_LOAD|gpu-route|0.48|Streaming weights through DirectML with {:.1} GiB VRAM free",
                            bytes_to_gib(
                                device
                                    .local_budget_bytes
                                    .saturating_sub(device.local_usage_bytes)
                            )
                        );
                        Some(device)
                    }
                    Err(err) => {
                        log::warn!("DirectML device initialization failed: {:?}", err);
                        log::info!(
                            "LLM_LOAD|gpu-backend|0.45|DirectML unavailable; using wgpu fallback"
                        );
                        None
                    }
                }
            }
        };

        #[cfg(not(target_os = "windows"))]
        {
            log::info!("LLM_LOAD|gpu-backend|0.45|Using wgpu fallback backend for native compute");
        }

        #[cfg(not(target_arch = "wasm32"))]
        let projection_cold_byte_budget = crate::gpu_context::global_vram_ledger().budget();
        #[cfg(target_os = "windows")]
        let projection_cold_byte_budget =
            dml_status
                .as_ref()
                .map_or(projection_cold_byte_budget, |device| {
                    device
                        .local_budget_bytes
                        .saturating_sub(device.local_usage_bytes)
                });
        #[cfg(not(target_arch = "wasm32"))]
        let projection_cold_byte_budget = usize::try_from(projection_cold_byte_budget).unwrap_or(0);

        let engine = Self {
            #[cfg(target_arch = "wasm32")]
            device: wasm_device,
            #[cfg(target_arch = "wasm32")]
            queue: wasm_queue,
            pipeline,
            #[cfg(target_arch = "wasm32")]
            mmv_q8_0_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            native_pipeline_cache,
            #[cfg(not(target_arch = "wasm32"))]
            pipeline_bind_layout,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_bind_layout,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_mr_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_residual_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_residual_bind_layout,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_residual_mr_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_warp_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            coop_gemv_residual_warp_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            mock_pipeline,
            embedding_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            embedding_bind_layout,
            attention_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            attention_bind_layout,
            is_initialized: true,
            #[cfg(target_os = "windows")]
            dml: dml_status,
            #[cfg(not(target_arch = "wasm32"))]
            prepared_native_bytes: None,
            #[cfg(not(target_arch = "wasm32"))]
            prepared_native_sources: None,
            #[cfg(not(target_arch = "wasm32"))]
            qwen35_ftw: None,
            gguf_mmap: None,
            #[cfg(target_arch = "wasm32")]
            cached_tokenizer: None,
            #[cfg(target_arch = "wasm32")]
            cached_tensor_index: None,
            #[cfg(target_arch = "wasm32")]
            cached_token_embd: None,
            #[cfg(target_arch = "wasm32")]
            p64_resident: None,
            #[cfg(not(target_arch = "wasm32"))]
            p64_index: None,
            #[cfg(not(target_arch = "wasm32"))]
            tensor_index_cache: None,
            #[cfg(not(target_arch = "wasm32"))]
            ftw_package: None,
            tensor_data_offset: 0,
            hyperparams: crate::gguf_sharder::GgufHyperparams::default(),
            max_tensor_bytes: 0,
            gemm_input_buf: None,
            gemm_weight_buf: None,
            #[cfg(target_arch = "wasm32")]
            mc8_weight_arena: None,
            #[cfg(target_arch = "wasm32")]
            mc8_weights_resident: false,
            #[cfg(target_arch = "wasm32")]
            mc8_weight_role_stride: [0u64; 7],
            #[cfg(target_arch = "wasm32")]
            gemm_weight_buf_b: None,
            gemm_output_buf: None,
            gemm_params_buf: None,
            gemm_output_staging: None,
            output_topk_pipeline: None,
            output_topk_bind_layout: None,
            topk_cand_val_buf: None,
            topk_cand_idx_buf: None,
            topk_cand_staging: None,
            topk_params_buf: None,
            gemm_aux_buf: None,
            gemm_ffn_buf: None,
            #[cfg(target_arch = "wasm32")]
            prefill_scratch_buf: None,
            #[cfg(target_arch = "wasm32")]
            prefill_work_buf_a: None,
            #[cfg(target_arch = "wasm32")]
            prefill_work_buf_b: None,
            #[cfg(target_arch = "wasm32")]
            mc8_q_proj_buf: None,
            #[cfg(target_arch = "wasm32")]
            mc8_k_proj_buf: None,
            #[cfg(target_arch = "wasm32")]
            mc8_v_proj_buf: None,
            gemm_max_out_dim: MAX_STACK_GEMM_OUT as u32,
            gemm_max_input_floats: 0,
            kv_layout: None,
            kv_cache_gpu: None,
            kv_cache_cpu: None,
            attention_params_buf: None,
            attention_mask_buf: None,
            elem_rms_norm_pipeline,
            elem_silu_mul_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            elem_silu_mul_bind_layout,
            elem_add_residual_pipeline,
            elem_params_buf: None,
            norm_weight_buf: None,
            #[cfg(target_arch = "wasm32")]
            mc8_gemm_bind_layout,
            #[cfg(target_arch = "wasm32")]
            mc8_elem_bind_layout,
            #[cfg(target_arch = "wasm32")]
            mc8_attn_bind_layout,
            #[cfg(target_arch = "wasm32")]
            mc8_ffn_fused_bind_layout,
            #[cfg(target_arch = "wasm32")]
            mc8_ffn_fused_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            ffn_fused_bind_layout,
            #[cfg(not(target_arch = "wasm32"))]
            ffn_fused_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            ffn_fused_coop_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            ffn_fused_mr_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            ffn_fused_warp_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            dual_gemv_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            dual_gemv_mr_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            dual_gemv_bind_layout,
            #[cfg(not(target_arch = "wasm32"))]
            triple_gemv_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            triple_gemv_bind_layout,
            mc8_logits_resident_buf: None,
            mc8_logits_row_bytes: 0,
            #[cfg(not(target_arch = "wasm32"))]
            ternary_ffn: None,
            #[cfg(not(target_arch = "wasm32"))]
            resident_decode: super::resident_decode::ResidentDecodeState::Unbuilt,
            #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
            cuda_decode_plan: super::cuda_decode_plan::CudaDecodePlanState::Unbuilt,
            #[cfg(not(target_arch = "wasm32"))]
            prefill_arena: super::prefill_arena::PrefillArenaState::Unbuilt,
            #[cfg(not(target_arch = "wasm32"))]
            verify_arena: super::verify_arena::VerifyArenaState::Unbuilt,
            #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
            projection_cold_byte_budget,
            #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
            projection_readback: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            gemm_resident_weights: std::sync::Mutex::new(
                super::gemm::ResidentProjectionCache::default(),
            ),
            #[cfg(not(target_arch = "wasm32"))]
            ffn_fused_params: None,
            #[cfg(not(target_arch = "wasm32"))]
            attention_kv_gemm_params: Some(attention_kv_gemm_params),
            #[cfg(not(target_arch = "wasm32"))]
            attention_kv_params: Some(attention_kv_params),
            #[cfg(target_arch = "wasm32")]
            mc8_norm_resident_buf: None,
            #[cfg(target_arch = "wasm32")]
            mc8_norm_stride: 0,
            #[cfg(target_arch = "wasm32")]
            mc8_bg_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
            ssm_recurrent_state: Box::new([]),
            ssm_state_byte_budget: DEFAULT_SSM_STATE_BYTE_BUDGET,
        };
        // Exercise the CPU elementwise oracle (ReLU) once at engine init so the fallback
        // path stays linked when GPU elem kernels are unavailable.
        let mut relu_probe = [-1.0f32, 2.0];
        let _ =
            super::cpu_ops::apply_cpu_elem_op(super::gpu_params::ELEM_OP_RELU, &mut relu_probe, 2);
        let _ = super::gpu_params::elem_op_label(super::gpu_params::ELEM_OP_RMS_NORM);
        let _ = engine.elem_gpu_pipeline(super::gpu_params::ELEM_OP_RMS_NORM);
        let _ = engine.elem_gpu_pipeline(super::gpu_params::ELEM_OP_ADD_RESIDUAL);
        Ok(engine)
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    /// Builds the engine; panics when GPU init fails.
    pub fn new() -> Self {
        let handle = tokio::runtime::Handle::try_current().unwrap_or_else(|_| {
            let rt = Box::leak(Box::new(tokio::runtime::Runtime::new().unwrap()));
            rt.handle().clone()
        });
        tokio::task::block_in_place(|| {
            handle
                .block_on(Self::try_new())
                .expect("Failed to initialize native GGUF engine")
        })
    }

    /// CPU-only engine: mmap + hyperparams + dequant/GEMM. No wgpu, no device.
    #[cfg(not(feature = "gpu-runtime"))]
    fn cpu_runtime() -> Self {
        Self {
            is_initialized: true,
            #[cfg(target_os = "windows")]
            dml: None,
            #[cfg(not(target_arch = "wasm32"))]
            prepared_native_bytes: None,
            #[cfg(not(target_arch = "wasm32"))]
            prepared_native_sources: None,
            #[cfg(not(target_arch = "wasm32"))]
            qwen35_ftw: None,
            gguf_mmap: None,
            #[cfg(target_arch = "wasm32")]
            cached_tokenizer: None,
            #[cfg(target_arch = "wasm32")]
            cached_tensor_index: None,
            #[cfg(target_arch = "wasm32")]
            cached_token_embd: None,
            #[cfg(target_arch = "wasm32")]
            p64_resident: None,
            #[cfg(not(target_arch = "wasm32"))]
            p64_index: None,
            #[cfg(not(target_arch = "wasm32"))]
            tensor_index_cache: None,
            #[cfg(not(target_arch = "wasm32"))]
            ftw_package: None,
            tensor_data_offset: 0,
            hyperparams: crate::gguf_sharder::GgufHyperparams::default(),
            max_tensor_bytes: 0,
            #[cfg(target_arch = "wasm32")]
            mc8_weights_resident: false,
            #[cfg(target_arch = "wasm32")]
            mc8_weight_role_stride: [0u64; 7],
            gemm_max_out_dim: MAX_STACK_GEMM_OUT as u32,
            gemm_max_input_floats: 0,
            kv_layout: None,
            kv_cache_cpu: None,
            mc8_logits_row_bytes: 0,
            #[cfg(target_arch = "wasm32")]
            mc8_norm_stride: 0,
            #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
            cuda_decode_plan: super::cuda_decode_plan::CudaDecodePlanState::Unbuilt,
            ssm_recurrent_state: Box::new([]),
            ssm_state_byte_budget: DEFAULT_SSM_STATE_BYTE_BUDGET,
        }
    }

    #[cfg(not(feature = "gpu-runtime"))]
    /// Tries to build the engine; error when GPU init fails.
    pub async fn try_new() -> Result<Self, String> {
        log::info!(
            "LLM_LOAD|engine-init|0.10|Initializing CPU-only GGUF runtime (gpu-runtime off)"
        );
        Ok(Self::cpu_runtime())
    }

    #[cfg(not(feature = "gpu-runtime"))]
    /// Builds the engine; panics when GPU init fails.
    pub fn new() -> Self {
        Self::cpu_runtime()
    }

    #[cfg(feature = "gpu-runtime")]
    /// Ensures the KV cache is allocated.
    pub(crate) fn ensure_kv_cache(&mut self, h: &crate::gguf_sharder::GgufHyperparams) {
        let layout = match KvCacheLayout::from_hyperparams(h) {
            Some(l) => l,
            None => {
                #[cfg(target_arch = "wasm32")]
                wlog(
                    "[kv_cache] FAILED from_hyperparams (zero dims or exceeds KV_CACHE_MAX_BYTES)",
                );
                return;
            }
        };
        let bytes = (layout.total_f32_elems * std::mem::size_of::<f32>()) as etalon::BufferAddress;
        let attn_params_bytes = {
            #[cfg(target_arch = "wasm32")]
            {
                (MC8_MAX_ATTN_UNIFORM_CHUNK_SLOTS * MC8_UNIFORM_ALIGN) as etalon::BufferAddress
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::mem::size_of::<AttentionGpuParams>() as etalon::BufferAddress
            }
        };
        use etalon_inference::gpu_resources::{KvBuffers, KvGeometry, ResourceBudget};
        let resources = match KvBuffers::build(
            self.gpu_device(),
            KvGeometry {
                cache_bytes: bytes,
                attention_params_bytes: attn_params_bytes,
                mask_bytes: (MAX_ATTN_MASK_UPLOAD_WORDS * std::mem::size_of::<u32>()).max(4) as u64,
            },
            ResourceBudget::for_device(self.gpu_device()),
            |_| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let ledger = crate::gpu_context::global_vram_ledger();
                    let orch = crate::gpu_context::universe_orchestrator();
                    if !ledger.can_allocate_in_universe(
                        &orch,
                        crate::gpu_context::ComputeUniverse::LlmInference,
                        bytes,
                    ) {
                        log::warn!(
                    "LLM_LOAD|kv-cache|denied|U0 budget {:.1} MiB used, need {:.1} MiB (mode {:?})",
                    ledger.universe_used_bytes(crate::gpu_context::ComputeUniverse::LlmInference)
                        as f64
                        / (1024.0 * 1024.0),
                    bytes as f64 / (1024.0 * 1024.0),
                    orch.active_mode,
                );
                        return Err(etalon_inference::gpu_resources::ResourceError::PolicyDenied);
                    }
                }
                Ok(())
            },
        ) {
            Ok(resources) => resources,
            Err(error) => {
                if !matches!(
                    error,
                    etalon_inference::gpu_resources::ResourceError::PolicyDenied
                ) && (h.architecture == crate::gguf_sharder::ARCH_GEMMA4
                    || h.ssm_conv_kernel > 0
                    || (h.arch_flags & crate::gguf_sharder::ARCH_FLAG_HAS_QK_NORM) != 0
                    || crate::llm_bench::cpu_attention_enabled())
                {
                    let cpu = vec![0f32; layout.total_f32_elems].into_boxed_slice();
                    self.kv_layout = Some(layout);
                    self.kv_cache_cpu = Some(cpu);
                    self.kv_cache_gpu = None;
                    log::info!(
                        "LLM_LOAD|kv-cache|0.86|Reserved {:.1} MiB host KV cache (context {})",
                        bytes as f64 / (1024.0 * 1024.0),
                        layout.max_context,
                    );
                    return;
                }
                log::warn!("inference KV resource admission failed: {error:?}");
                return;
            }
        };
        let gpu = resources.cache;
        let cpu = vec![0f32; layout.total_f32_elems].into_boxed_slice();
        self.attention_params_buf = Some(resources.attention_params);
        self.attention_mask_buf = Some(resources.mask);
        self.kv_layout = Some(layout);
        self.kv_cache_gpu = Some(gpu);
        self.kv_cache_cpu = Some(cpu);
        // W5b Phase 4b: seed the dictionary atoms into each layer's arena tail (after the codes).
        #[cfg(not(target_arch = "wasm32"))]
        self.upload_dict_atoms();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let ledger = crate::gpu_context::global_vram_ledger();
            ledger.record_kv_cache(bytes);
        }
        log::info!(
            "LLM_LOAD|kv-cache|0.86|Reserved {:.1} MiB KV cache (GPU + CPU mirror, context {})",
            bytes as f64 / (1024.0 * 1024.0),
            layout.max_context,
        );
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!(
            "[gguf_bridge] KV arena {} slots ({:.1} MiB, {}), context={}",
            layout.total_f32_elems,
            bytes as f64 / (1024.0 * 1024.0),
            if layout.dict_k > 0 {
                "dict-coded"
            } else if layout.int8 {
                "int8+scale"
            } else {
                "f32"
            },
            layout.max_context,
        );
    }

    /// CPU-only KV arena: layout + host mirror. No wgpu buffers.
    #[cfg(not(feature = "gpu-runtime"))]
    pub(crate) fn ensure_kv_cache(&mut self, h: &crate::gguf_sharder::GgufHyperparams) {
        let layout = match KvCacheLayout::from_hyperparams(h) {
            Some(l) => l,
            None => return,
        };
        let total = layout.total_f32_elems;
        let max_context = layout.max_context;
        let cpu = vec![0f32; total].into_boxed_slice();
        self.kv_layout = Some(layout);
        self.kv_cache_cpu = Some(cpu);
        log::info!(
            "LLM_LOAD|kv-cache|0.86|Reserved {:.1} MiB CPU KV cache (context {})",
            (total * std::mem::size_of::<f32>()) as f64 / (1024.0 * 1024.0),
            max_context,
        );
    }

    /// Zero the static KV arena at the start of a new decode context (zero heap in decode).
    pub fn reset_kv_cache(&mut self) {
        self.reset_recurrent_state();
        let Some(layout) = self.kv_layout.as_ref() else {
            return;
        };
        let n = layout.total_f32_elems;
        if let Some(cpu) = self.kv_cache_cpu.as_mut() {
            for v in cpu.iter_mut().take(n) {
                unsafe { core::ptr::write_volatile(v, 0.0) };
            }
        }
        #[cfg(feature = "gpu-runtime")]
        if let (Some(cpu), Some(gpu)) = (self.kv_cache_cpu.as_ref(), self.kv_cache_gpu.as_ref()) {
            self.gpu_queue()
                .write_buffer(gpu, 0, bytemuck::cast_slice(&cpu[..n]));
        }
        // W5b Phase 4b: the zero above wiped the atoms tail — re-seed it (dict mode only).
        #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
        self.upload_dict_atoms();
    }

    /// W5b Phase 4b: write the installed dictionary atoms into the tail of each layer's arena slice
    /// (after that layer's code region). No-op unless dict mode is active and atoms are installed.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(crate) fn upload_dict_atoms(&self) {
        let Some(layout) = self.kv_layout.as_ref() else {
            return;
        };
        if layout.dict_k == 0 {
            return;
        }
        let (Some(buf), Some((flat, na, hd))) = (
            self.kv_cache_gpu.as_ref(),
            crate::kv_dict_runtime::atoms_flat(),
        ) else {
            return;
        };
        let code_region = (layout.max_context * 2 * layout.n_kv_head * layout.dict_k) as usize;
        let per_layer_atoms = 2 * na * hd;
        let ls = layout.layer_stride as usize;
        for l in 0..layout.n_layer as usize {
            let s = l * per_layer_atoms;
            let e = ((l + 1) * per_layer_atoms).min(flat.len());
            if e <= s {
                break;
            }
            let dst_word = l * ls + code_region;
            self.gpu_queue().write_buffer(
                buf,
                (dst_word * 4) as u64,
                bytemuck::cast_slice(&flat[s..e]),
            );
        }
    }

    /// Returns `kv_cache_cpu`.
    pub fn get_kv_cache_cpu(&self) -> Option<&[f32]> {
        self.kv_cache_cpu.as_deref()
    }

    /// Sets `kv_cache_cpu`.
    pub fn set_kv_cache_cpu(&mut self, data: &[f32]) -> bool {
        let Some(layout) = self.kv_layout.as_ref() else {
            return false;
        };
        let n = layout.total_f32_elems;
        if data.len() < n {
            return false;
        }
        let Some(cpu) = self.kv_cache_cpu.as_mut() else {
            return false;
        };
        cpu[..n].copy_from_slice(&data[..n]);
        #[cfg(feature = "gpu-runtime")]
        if let (Some(cpu), Some(gpu)) = (self.kv_cache_cpu.as_ref(), self.kv_cache_gpu.as_ref()) {
            self.gpu_queue()
                .write_buffer(gpu, 0, bytemuck::cast_slice(&cpu[..n]));
        }
        true
    }

    /// Read the entire GPU KV-cache arena back to host as `f32` (native, cold path — forge KV capture,
    /// not the decode hot path). The returned flat buffer is interpretable via `KvCacheLayout::
    /// k_index`/`v_index` **only when the layout is f32** (int8 KV disabled at load); with an int8
    /// layout the bytes are packed i8 lanes + scales and this returns `None`.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub fn read_kv_cache_gpu(&self) -> Option<Vec<f32>> {
        let gpu = self.kv_cache_gpu.as_ref()?;
        let layout = self.kv_layout.as_ref()?;
        if layout.int8 {
            return None;
        }
        let n = layout.total_f32_elems;
        let size = (n * std::mem::size_of::<f32>()) as etalon::BufferAddress;
        let staging = self.gpu_device().create_buffer(&etalon::BufferDescriptor {
            label: Some("KvCacheReadback"),
            size: size.max(4),
            usage: etalon::BufferUsages::MAP_READ | etalon::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder =
            self.gpu_device()
                .create_command_encoder(&etalon::CommandEncoderDescriptor {
                    label: Some("KvReadback"),
                });
        encoder.copy_buffer_to_buffer(gpu, 0, &staging, 0, size);
        self.submit_commands(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(etalon::MapMode::Read, move |v| {
            let _ = tx.send(v);
        });
        self.poll_wait();
        let handle = tokio::runtime::Handle::try_current().unwrap_or_else(|_| {
            let rt = Box::leak(Box::new(tokio::runtime::Runtime::new().unwrap()));
            rt.handle().clone()
        });
        if handle.block_on(rx).ok()?.is_err() {
            return None;
        }
        let data = slice
            .get_mapped_range()
            .expect("wgpu buffer map_range failed");
        let out: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging.unmap();
        Some(out)
    }

    /// Decode the current f32 KV cache into per-layer K and V vectors for token positions `0..n_tokens`
    /// (capped at `max_per_layer` per layer per stream). The **GPU-readback** capture route for the
    /// sparse-KV-dictionary go/no-go — reads the real decode-path K/V straight from VRAM, no CPU
    /// reference forward. Returns `None` on an int8 layout or if readback fails.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub fn capture_kv_f32(
        &self,
        n_tokens: u32,
        max_per_layer: usize,
    ) -> Option<crate::kv_capture::KvCapture> {
        let layout = *self.kv_layout.as_ref()?;
        let flat = self.read_kv_cache_gpu()?;
        let n_layer = layout.n_layer as usize;
        let n_kv = layout.n_kv_head as usize;
        let head_dim = layout.head_dim as usize;
        if head_dim == 0 || n_kv == 0 || n_layer == 0 {
            return None;
        }
        let take = (n_tokens as usize).min(layout.max_context as usize);
        let mut k = vec![Vec::new(); n_layer];
        let mut v = vec![Vec::new(); n_layer];
        let read_vec = |base_of: &dyn Fn(u32) -> usize| -> Option<Vec<f32>> {
            let mut out = Vec::with_capacity(head_dim);
            for d in 0..head_dim {
                let idx = base_of(d as u32);
                if idx >= flat.len() {
                    return None;
                }
                out.push(flat[idx]);
            }
            Some(out)
        };
        for l in 0..n_layer {
            for pos in 0..take {
                let slot = layout.ring_slot(pos as u32);
                for hkv in 0..n_kv {
                    if k[l].len() < max_per_layer {
                        if let Some(vec_k) =
                            read_vec(&|d| layout.k_index(l as u32, slot, hkv as u32, d))
                        {
                            k[l].push(vec_k);
                        }
                    }
                    if v[l].len() < max_per_layer {
                        if let Some(vec_v) =
                            read_vec(&|d| layout.v_index(l as u32, slot, hkv as u32, d))
                        {
                            v[l].push(vec_v);
                        }
                    }
                }
            }
        }
        Some(crate::kv_capture::KvCapture { head_dim, k, v })
    }

    #[cfg(feature = "gpu-runtime")]
    /// Ensures the GEMM buffers are allocated.
    pub(crate) fn ensure_gemm_buffers(&mut self, max_weight_bytes: usize, max_out_dim: u32) {
        // A1a: build the persistent GPU top-k pipeline + candidate buffers once (additive; the
        // existing argmax path is unaffected whether or not this succeeds).
        if self.output_topk_pipeline.is_none() {
            self.init_output_topk();
        }
        let need_input = MAX_STACK_GEMM_IN.max(MAX_PREFILL_BATCH_FLOATS);
        let prefill_bufs_ready = {
            #[cfg(target_arch = "wasm32")]
            {
                self.prefill_scratch_buf.is_some()
                    && self.prefill_work_buf_a.is_some()
                    && self.prefill_work_buf_b.is_some()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                true
            }
        };
        #[cfg(target_arch = "wasm32")]
        let weight_arena_ready = self.mc8_weight_arena.is_some();
        #[cfg(not(target_arch = "wasm32"))]
        let weight_arena_ready = true;
        if self.gemm_weight_buf.is_some()
            && max_weight_bytes <= self.max_tensor_bytes
            && self.gemm_max_input_floats >= need_input
            && prefill_bufs_ready
            && weight_arena_ready
        {
            return;
        }
        use etalon_inference::gpu_resources::{GemmBuffers, GemmGeometry, ResourceBudget};
        let gemm_params_bytes = {
            #[cfg(target_arch = "wasm32")]
            {
                (MC8_MAX_GEMM_UNIFORM_CHUNK_SLOTS * MC8_UNIFORM_ALIGN) as u64
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::mem::size_of::<GemmGpuParams>() as u64
            }
        };
        let elem_params_bytes = {
            #[cfg(target_arch = "wasm32")]
            {
                (MC8_MAX_ELEM_UNIFORM_CHUNK_SLOTS * MC8_UNIFORM_ALIGN) as u64
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::mem::size_of::<ElemGpuParams>() as u64
            }
        };
        let out_bytes = (u64::from(max_out_dim) * 4).max(4);
        let resources = GemmBuffers::build(
            self.gpu_device(),
            GemmGeometry {
                weight_bytes: max_weight_bytes.max(4) as u64,
                input_bytes: (need_input * 4) as u64,
                output_bytes: out_bytes,
                gemm_params_bytes,
                elem_params_bytes,
                norm_bytes: (MAX_HIDDEN_DIM * 4) as u64,
                staging_bytes: if cfg!(target_arch = "wasm32") {
                    out_bytes.max(65536 * 4)
                } else {
                    out_bytes
                },
                #[cfg(target_arch = "wasm32")]
                prefill_work_bytes: (PREFILL_CHUNK_SIZE
                    * (MAX_HIDDEN_DIM + 2 * max_out_dim as usize + MAX_HIDDEN_DIM).max(4)
                    * 4)
                .max(4) as u64,
            },
            ResourceBudget::for_device(self.gpu_device()),
            |_| Ok(()),
        )
        .unwrap_or_else(|error| panic!("inference GEMM resources: {error:?}"));
        self.gemm_input_buf = Some(resources.input);
        self.gemm_weight_buf = Some(resources.weight);
        self.gemm_output_buf = Some(resources.output);
        self.gemm_params_buf = Some(resources.params);
        self.gemm_output_staging = Some(resources.staging);
        self.gemm_aux_buf = Some(resources.aux);
        self.gemm_ffn_buf = Some(resources.ffn);
        self.elem_params_buf = Some(resources.elem_params);
        self.norm_weight_buf = Some(resources.norm);
        #[cfg(target_arch = "wasm32")]
        {
            let browser = resources.browser;
            let [qkv_k, qkv_v, qkv_q, o_proj, gate, up, down] = browser.role_weights;
            self.mc8_weight_arena = Some(Mc8WeightArenaBufs {
                qkv_k,
                qkv_v,
                qkv_q,
                o_proj,
                gate,
                up,
                down,
            });
            self.gemm_weight_buf_b = Some(browser.weight_b);
            self.prefill_scratch_buf = Some(browser.prefill_scratch);
            self.prefill_work_buf_a = Some(browser.work_a);
            self.prefill_work_buf_b = Some(browser.work_b);
            self.mc8_q_proj_buf = Some(browser.q_proj);
            self.mc8_k_proj_buf = Some(browser.k_proj);
            self.mc8_v_proj_buf = Some(browser.v_proj);
        }
        self.gemm_max_out_dim = max_out_dim;
        self.gemm_max_input_floats = need_input;
        self.max_tensor_bytes = max_weight_bytes;
    }
}

/// Compile-time proof that CPU constructors exist when wgpu is not linked.
#[cfg(not(feature = "gpu-runtime"))]
const _: fn() -> super::QTensorEngine = super::QTensorEngine::new;
