//! Engine init/lifecycle: GPU device/queue accessors, try_new, new, KV-cache + GEMM-buffer
//! allocation/reset. Split from gguf_bridge/mod.rs (structural; no behaviour change).
use super::*;

impl QTensorEngine {
    /// Resolve the MC8 elementwise GPU pipeline for a given opcode.
    pub(crate) fn elem_gpu_pipeline(&self, op: u32) -> Option<&wgpu::ComputePipeline> {
        use super::gpu_params::{ELEM_OP_ADD_RESIDUAL, ELEM_OP_RMS_NORM};
        match op {
            ELEM_OP_RMS_NORM => Some(&self.elem_rms_norm_pipeline),
            ELEM_OP_ADD_RESIDUAL => Some(&self.elem_add_residual_pipeline),
            _ => None,
        }
    }
    pub(crate) fn gpu_device(&self) -> &wgpu::Device {
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
    pub(crate) fn gpu_queue(&self) -> &wgpu::Queue {
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
    pub fn device(&self) -> &wgpu::Device {
        self.gpu_device()
    }

    /// Shared process-wide wgpu queue.
    #[inline]
    pub fn queue(&self) -> &wgpu::Queue {
        self.gpu_queue()
    }

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
            let instance = wgpu::Instance::default();
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .map_err(|e| format!("Failed to find wgpu adapter: {e}"))?;
            adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await
                .map_err(|e| e.to_string())?
        };
        #[cfg(target_arch = "wasm32")]
        let device = &wasm_device;
        #[cfg(target_arch = "wasm32")]
        let queue = &wasm_queue;

        #[cfg(not(target_arch = "wasm32"))]
        let native_pipeline_cache = create_native_pipeline_cache(device);
        #[cfg(target_arch = "wasm32")]
        let native_pipeline_cache: Option<wgpu::PipelineCache> = None;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fused Transformer Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/fused_transformer.wgsl").into(),
            ),
        });

        // Shared explicit 5-slot layout (bindings 0-3 + residual 4). Module-scope
        // `residual` in fused_transformer.wgsl requires binding 4 on every entry point.
        // Plain GEMV call sites bind a dummy residual (often the input buffer).
        #[cfg(not(target_arch = "wasm32"))]
        let coop_gemv_residual_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("CoopGemvBGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        #[cfg(not(target_arch = "wasm32"))]
        let coop_gemv_bind_layout = coop_gemv_residual_bind_layout.clone();
        #[cfg(not(target_arch = "wasm32"))]
        let coop_gemv_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("CoopGemvPL"),
                bind_group_layouts: &[Some(&coop_gemv_bind_layout)],
                immediate_size: 0,
            });
        #[cfg(not(target_arch = "wasm32"))]
        let coop_gemv_residual_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("CoopGemvResidualPL"),
                bind_group_layouts: &[Some(&coop_gemv_residual_bind_layout)],
                immediate_size: 0,
            });

        #[cfg(target_arch = "wasm32")]
        let mc8_gemm_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MC8GemmBGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: std::num::NonZeroU64::new(MC8_UNIFORM_ALIGN as u64),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        #[cfg(target_arch = "wasm32")]
        let mc8_gemm_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("MC8GemmPL"),
                bind_group_layouts: &[Some(&mc8_gemm_bind_layout)],
                immediate_size: 0,
            });

        #[cfg(target_arch = "wasm32")]
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Fused Transformer Pipeline"),
            layout: Some(&mc8_gemm_pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: native_pipeline_cache.as_ref(),
        });
        #[cfg(not(target_arch = "wasm32"))]
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Fused Transformer Pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: native_pipeline_cache.as_ref(),
        });
        // 0.0.21: second pipeline over the SAME shader module — the cooperative one-workgroup-per-row
        // GEMV. Auto-layout; same group-0 bindings as `main`. Native only.
        //
        // When the adapter advertises SUBGROUP, build the wave-reduction variant (`coop_gemv_sg` in
        // coop_gemv_subgroup.wgsl, concatenated after the base with `enable subgroups;`) which
        // replaces the 8-step barrier-synced shared-memory tree reduction with one `subgroupAdd` per
        // subgroup. Identical group-0 bindings + (n_out,1,1) dispatch → a drop-in for this field, so
        // every call site and the derived `coop_gemv_bind_layout` pick it up transparently. Adapters
        // without subgroups (and wasm) keep the universal shared-memory `coop_gemv`.
        // All coop GEMV entry points share the explicit 5-slot CoopGemvBGL so bind
        // groups are interchangeable across single-row / multi-row / residual / warp
        // (no exclusive-pipeline auto-layout traps).
        #[cfg(not(target_arch = "wasm32"))]
        let (coop_gemv_pipeline, coop_gemv_residual_pipeline) =
            if device.features().contains(wgpu::Features::SUBGROUP) {
                // Note: do NOT inject `enable subgroups;` — naga/wgpu 29 rejects it.
                let sg_src = format!(
                    "{}\n{}",
                    include_str!("../shaders/fused_transformer.wgsl"),
                    include_str!("../shaders/coop_gemv_subgroup.wgsl"),
                );
                let sg_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Coop GEMV Subgroup Shader"),
                    source: wgpu::ShaderSource::Wgsl(sg_src.into()),
                });
                let gemv = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Coop GEMV SG Pipeline"),
                    layout: Some(&coop_gemv_pipeline_layout),
                    module: &sg_module,
                    entry_point: Some("coop_gemv_sg"),
                    compilation_options: Default::default(),
                    cache: native_pipeline_cache.as_ref(),
                });
                let resid = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Coop GEMV Residual SG Pipeline"),
                    layout: Some(&coop_gemv_residual_pipeline_layout),
                    module: &sg_module,
                    entry_point: Some("coop_gemv_residual_sg"),
                    compilation_options: Default::default(),
                    cache: native_pipeline_cache.as_ref(),
                });
                (gemv, resid)
            } else {
                let gemv = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Coop GEMV Pipeline"),
                    layout: Some(&coop_gemv_pipeline_layout),
                    module: &shader,
                    entry_point: Some("coop_gemv"),
                    compilation_options: Default::default(),
                    cache: native_pipeline_cache.as_ref(),
                });
                let resid = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Coop GEMV Residual Pipeline"),
                    layout: Some(&coop_gemv_residual_pipeline_layout),
                    module: &shader,
                    entry_point: Some("coop_gemv_residual"),
                    compilation_options: Default::default(),
                    cache: native_pipeline_cache.as_ref(),
                });
                (gemv, resid)
            };
        #[cfg(not(target_arch = "wasm32"))]
        let coop_gemv_mr_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Coop GEMV Multi-Row Pipeline"),
                layout: Some(&coop_gemv_pipeline_layout),
                module: &shader,
                entry_point: Some("coop_gemv_mr"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });
        #[cfg(not(target_arch = "wasm32"))]
        let coop_gemv_residual_mr_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Coop GEMV Residual Multi-Row Pipeline"),
                layout: Some(&coop_gemv_residual_pipeline_layout),
                module: &shader,
                entry_point: Some("coop_gemv_residual_mr"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });
        #[cfg(not(target_arch = "wasm32"))]
        let coop_gemv_warp_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Coop GEMV Warp Pipeline"),
                layout: Some(&coop_gemv_pipeline_layout),
                module: &shader,
                entry_point: Some("coop_gemv_warp"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });
        #[cfg(not(target_arch = "wasm32"))]
        let coop_gemv_residual_warp_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Coop GEMV Residual Warp Pipeline"),
                layout: Some(&coop_gemv_residual_pipeline_layout),
                module: &shader,
                entry_point: Some("coop_gemv_residual_warp"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });

        let mock_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Mock Fused Contraction Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/fused_tensor_contraction.wgsl").into(),
            ),
        });
        let mock_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Mock Fused Contraction Pipeline"),
            layout: None,
            module: &mock_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: native_pipeline_cache.as_ref(),
        });

        let emb_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Quantized Embedding Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/quantized_embedding.wgsl").into(),
            ),
        });
        let embedding_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Quantized Embedding Pipeline"),
            layout: None,
            module: &emb_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: native_pipeline_cache.as_ref(),
        });

        let attn_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fused Attention Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/fused_attention.wgsl").into(),
            ),
        });
        #[cfg(target_arch = "wasm32")]
        let mc8_attn_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MC8AttnBGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: std::num::NonZeroU64::new(MC8_UNIFORM_ALIGN as u64),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        #[cfg(target_arch = "wasm32")]
        let mc8_attn_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("MC8AttnPL"),
                bind_group_layouts: &[Some(&mc8_attn_bind_layout)],
                immediate_size: 0,
            });
        #[cfg(target_arch = "wasm32")]
        let attention_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Fused Attention Pipeline"),
            layout: Some(&mc8_attn_pipeline_layout),
            module: &attn_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: native_pipeline_cache.as_ref(),
        });
        #[cfg(not(target_arch = "wasm32"))]
        let attention_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Fused Attention Pipeline"),
            layout: None,
            module: &attn_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: native_pipeline_cache.as_ref(),
        });

        let elem_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Wasm Elementwise Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/wasm_elementwise.wgsl").into(),
            ),
        });
        #[cfg(target_arch = "wasm32")]
        let mc8_elem_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MC8ElemBGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: std::num::NonZeroU64::new(MC8_UNIFORM_ALIGN as u64),
                        },
                        count: None,
                    },
                ],
            });
        #[cfg(target_arch = "wasm32")]
        let mc8_elem_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("MC8ElemPL"),
                bind_group_layouts: &[Some(&mc8_elem_bind_layout)],
                immediate_size: 0,
            });
        #[cfg(target_arch = "wasm32")]
        let elem_rms_norm_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ElemRmsNorm"),
                layout: Some(&mc8_elem_pipeline_layout),
                module: &elem_shader,
                entry_point: Some("rms_norm_batch"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });
        #[cfg(target_arch = "wasm32")]
        let elem_silu_mul_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ElemSiluMul"),
                layout: Some(&mc8_elem_pipeline_layout),
                module: &elem_shader,
                entry_point: Some("silu_mul_main"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });
        #[cfg(target_arch = "wasm32")]
        let elem_add_residual_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ElemAddResidual"),
                layout: Some(&mc8_elem_pipeline_layout),
                module: &elem_shader,
                entry_point: Some("add_residual_main"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });
        #[cfg(not(target_arch = "wasm32"))]
        let elem_rms_norm_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ElemRmsNorm"),
                layout: None,
                module: &elem_shader,
                entry_point: Some("rms_norm_batch"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });
        #[cfg(not(target_arch = "wasm32"))]
        let elem_silu_mul_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ElemSiluMul"),
                layout: None,
                module: &elem_shader,
                entry_point: Some("silu_mul_main"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });
        #[cfg(not(target_arch = "wasm32"))]
        let elem_add_residual_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ElemAddResidual"),
                layout: None,
                module: &elem_shader,
                entry_point: Some("add_residual_main"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });

        // Phase 5 — Fused FFN expansion pipeline (gate · SiLU · up in one dispatch).
        // The dequant math is authored once in `dequant_template.wgsl` and instantiated
        // per weight role here (Rust-side modular WGSL composition), so the proven GEMM
        // path in `fused_transformer.wgsl` is untouched.
        #[cfg(target_arch = "wasm32")]
        let mc8_ffn_fused_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MC8FfnFusedBGL"),
                entries: &[
                    // 0: ffn_input (normalized hidden, storage read)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 1: gate_words (quantized gate weight, storage read)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 2: up_words (quantized up weight, storage read)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 3: params (GemmParams, dynamic uniform offset — gate's staged params)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: std::num::NonZeroU64::new(MC8_UNIFORM_ALIGN as u64),
                        },
                        count: None,
                    },
                    // 4: ffn_output (silu(gate)·up intermediate, storage read_write)
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        #[cfg(target_arch = "wasm32")]
        let mc8_ffn_fused_pipeline = {
            // Modular WGSL: shared scaffold + per-role dequant instances composed at runtime.
            let tpl = include_str!("../shaders/dequant_template.wgsl");
            let gate_fns = tpl.replace("$W", "gate_words").replace("$S", "_gate");
            let up_fns = tpl.replace("$W", "up_words").replace("$S", "_up");
            let base = include_str!("../shaders/fused_ffn.wgsl");
            // Inject the per-role dequant math at the marker (between shared helpers and
            // the entry point) so declarations precede their uses.
            let src = base.replace("// @@DEQUANT_FUNCTIONS@@", &format!("{gate_fns}\n{up_fns}"));
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("FusedFFNExpansion"),
                source: wgpu::ShaderSource::Wgsl(src.into()),
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("MC8FfnFusedPL"),
                bind_group_layouts: &[Some(&mc8_ffn_fused_bind_layout)],
                immediate_size: 0,
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("FusedFFNExpansionPipeline"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some("fused_ffn_expansion"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            })
        };

        // Native T-A1 — same fused_ffn.wgsl, static uniform (no dynamic offset).
        // Wired into resident_decode mega-pass when gate/up share a supported quant type.
        #[cfg(not(target_arch = "wasm32"))]
        let ffn_fused_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("NativeFfnFusedBGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: std::num::NonZeroU64::new(std::mem::size_of::<
                                GemmGpuParams,
                            >(
                            )
                                as u64),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        #[cfg(not(target_arch = "wasm32"))]
        let (ffn_fused_pipeline, ffn_fused_coop_pipeline, ffn_fused_mr_pipeline, ffn_fused_warp_pipeline) = {
            let tpl = include_str!("../shaders/dequant_template.wgsl");
            let gate_fns = tpl.replace("$W", "gate_words").replace("$S", "_gate");
            let up_fns = tpl.replace("$W", "up_words").replace("$S", "_up");
            let base = include_str!("../shaders/fused_ffn.wgsl");
            let src = base.replace("// @@DEQUANT_FUNCTIONS@@", &format!("{gate_fns}\n{up_fns}"));
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("NativeFusedFFNExpansion"),
                source: wgpu::ShaderSource::Wgsl(src.into()),
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("NativeFfnFusedPL"),
                bind_group_layouts: &[Some(&ffn_fused_bind_layout)],
                immediate_size: 0,
            });
            let naive = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("NativeFusedFFNExpansionPipeline"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some("fused_ffn_expansion"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });
            // Prefer subgroup reduction when available (same as coop_gemv_sg).
            let coop_ep = if device.features().contains(wgpu::Features::SUBGROUP) {
                "coop_fused_ffn_sg"
            } else {
                "coop_fused_ffn_expansion"
            };
            let coop = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("NativeCoopFusedFFNExpansionPipeline"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some(coop_ep),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });
            // Multi-row fused FFN (4 rows/WG, one K-sweep) — Q4_K_SOA 3B lever.
            let coop_mr = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("NativeCoopFusedFFNMultiRowPipeline"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some("coop_fused_ffn_mr"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });
            // Warp fused FFN (32 thr/row, 8 cols/lane).
            let coop_warp = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("NativeCoopFusedFFNWarpPipeline"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some("coop_fused_ffn_warp"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });
            log::info!("LLM_LOAD|fused_ffn|coop_entry={coop_ep}|mr=coop_fused_ffn_mr|warp=coop_fused_ffn_warp");
            (naive, coop, coop_mr, coop_warp)
        };

        // Dual K+V GEMV (shared act) — mega-kernel slice for resident decode.
        #[cfg(not(target_arch = "wasm32"))]
        let dual_gemv_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("DualGemvBGL"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        #[cfg(not(target_arch = "wasm32"))]
        let (dual_gemv_pipeline, dual_gemv_mr_pipeline) = {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Dual GEMV Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../shaders/dual_gemv.wgsl").into(),
                ),
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("DualGemvPL"),
                bind_group_layouts: &[Some(&dual_gemv_bind_layout)],
                immediate_size: 0,
            });
            let dual = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Dual GEMV Pipeline"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some("coop_gemv_dual"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });
            let dual_mr = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Dual GEMV Multi-Row Pipeline"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some("coop_gemv_dual_mr"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            });
            (dual, dual_mr)
        };

        // Triple Q+K+V GEMV (shared act, GQA) — resident mega-pass: 3 dispatches → 1.
        #[cfg(not(target_arch = "wasm32"))]
        let triple_gemv_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("TripleGemvBGL"),
                entries: &[
                    // 0 input, 1 Wq, 2 params, 3 out_q, 4 Wk, 5 out_k, 6 Wv, 7 out_v
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        #[cfg(not(target_arch = "wasm32"))]
        let triple_gemv_pipeline = {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Triple GEMV Shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../shaders/triple_gemv.wgsl").into(),
                ),
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("TripleGemvPL"),
                bind_group_layouts: &[Some(&triple_gemv_bind_layout)],
                immediate_size: 0,
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Triple GEMV Pipeline"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some("coop_gemv_triple"),
                compilation_options: Default::default(),
                cache: native_pipeline_cache.as_ref(),
            })
        };

        // DirectML is a *second* D3D12 device next to wgpu. Always-on init competed for
        // A2000 VRAM and added driver overhead while the resident decode path uses wgpu.
        // Opt in with QUALIA_DIRECTML=1 (or QUALIA_LLM_DIRECTML=1). Default: wgpu-only.
        #[cfg(target_os = "windows")]
        let dml_status = {
            let want = matches!(
                std::env::var("QUALIA_DIRECTML")
                    .or_else(|_| std::env::var("QUALIA_LLM_DIRECTML"))
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
        let pipeline_bind_layout = pipeline.get_bind_group_layout(0);
        // Keep the explicit non-exclusive CoopGemvBGL (created above). Do NOT replace with
        // pipeline.get_bind_group_layout — that yields exclusive layouts and breaks multi-row.
        #[cfg(not(target_arch = "wasm32"))]
        let embedding_bind_layout = embedding_pipeline.get_bind_group_layout(0);
        #[cfg(not(target_arch = "wasm32"))]
        let attention_bind_layout = attention_pipeline.get_bind_group_layout(0);
        #[cfg(not(target_arch = "wasm32"))]
        let elem_silu_mul_bind_layout = elem_silu_mul_pipeline.get_bind_group_layout(0);
        #[cfg(not(target_arch = "wasm32"))]
        let attention_kv_gemm_params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("AttentionKvGemmParams"),
            size: 256 * 2,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        #[cfg(not(target_arch = "wasm32"))]
        let attention_kv_params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("AttentionKvWriteParams"),
            size: 256 * 2,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let engine = Self {
            #[cfg(target_arch = "wasm32")]
            device: wasm_device,
            #[cfg(target_arch = "wasm32")]
            queue: wasm_queue,
            pipeline,
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
            gguf_mmap: None,
            #[cfg(target_arch = "wasm32")]
            p64_resident: None,
            #[cfg(not(target_arch = "wasm32"))]
            p64_index: None,
            #[cfg(not(target_arch = "wasm32"))]
            tensor_index_cache: None,
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
            #[cfg(not(target_arch = "wasm32"))]
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
            ffn_fused_mr_pipeline,
            ffn_fused_warp_pipeline,
            dual_gemv_pipeline,
            dual_gemv_mr_pipeline,
            dual_gemv_bind_layout,
            triple_gemv_pipeline,
            triple_gemv_bind_layout,
            mc8_logits_resident_buf: None,
            mc8_logits_row_bytes: 0,
            #[cfg(not(target_arch = "wasm32"))]
            ternary_ffn: None,
            #[cfg(not(target_arch = "wasm32"))]
            resident_decode: super::resident_decode::ResidentDecodeState::Unbuilt,
            #[cfg(not(target_arch = "wasm32"))]
            prefill_arena: super::prefill_arena::PrefillArenaState::Unbuilt,
            #[cfg(not(target_arch = "wasm32"))]
            verify_arena: super::verify_arena::VerifyArenaState::Unbuilt,
            #[cfg(not(target_arch = "wasm32"))]
            gemm_resident_weights: std::sync::Mutex::new(std::collections::HashMap::new()),
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

    #[cfg(not(target_arch = "wasm32"))]
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
        let bytes = (layout.total_f32_elems * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        // Native: honour U0 VRAM ledger pins. WASM: always allocate CPU mirror + wgpu storage
        // (ledger models host adapter VRAM; browser WebGPU has separate limits).
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
                return;
            }
        }
        let gpu = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("StaticKvCacheArena"),
            size: bytes.max(4),
            // COPY_SRC: MC8 pt3e L0 probe reads K/V slots via pipeline_read_kv_head.
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let cpu = vec![0f32; layout.total_f32_elems].into_boxed_slice();
        let attn_params_bytes = {
            #[cfg(target_arch = "wasm32")]
            {
                (MC8_MAX_ATTN_UNIFORM_CHUNK_SLOTS * MC8_UNIFORM_ALIGN) as wgpu::BufferAddress
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::mem::size_of::<AttentionGpuParams>() as wgpu::BufferAddress
            }
        };
        self.attention_params_buf =
            Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("AttentionParams"),
                size: attn_params_bytes.max(4),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        let mask_bytes = (MAX_ATTN_MASK_UPLOAD_WORDS * std::mem::size_of::<u32>()).max(4);
        self.attention_mask_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("AttentionKvMaskBatch"),
            size: mask_bytes as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
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

    /// Zero the static KV arena at the start of a new decode context (zero heap in decode).
    pub fn reset_kv_cache(&mut self) {
        let Some(layout) = self.kv_layout.as_ref() else {
            return;
        };
        let n = layout.total_f32_elems;
        if let Some(cpu) = self.kv_cache_cpu.as_mut() {
            for v in cpu.iter_mut().take(n) {
                unsafe { core::ptr::write_volatile(v, 0.0) };
            }
        }
        if let (Some(cpu), Some(gpu)) = (self.kv_cache_cpu.as_ref(), self.kv_cache_gpu.as_ref()) {
            self.gpu_queue()
                .write_buffer(gpu, 0, bytemuck::cast_slice(&cpu[..n]));
        }
        // W5b Phase 4b: the zero above wiped the atoms tail — re-seed it (dict mode only).
        #[cfg(not(target_arch = "wasm32"))]
        self.upload_dict_atoms();
    }

    /// W5b Phase 4b: write the installed dictionary atoms into the tail of each layer's arena slice
    /// (after that layer's code region). No-op unless dict mode is active and atoms are installed.
    #[cfg(not(target_arch = "wasm32"))]
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

    pub fn get_kv_cache_cpu(&self) -> Option<&[f32]> {
        self.kv_cache_cpu.as_deref()
    }

    pub fn set_kv_cache_cpu(&mut self, data: &[f32]) {
        let Some(layout) = self.kv_layout.as_ref() else {
            return;
        };
        let n = layout.total_f32_elems;
        if data.len() < n {
            return;
        }
        if let Some(cpu) = self.kv_cache_cpu.as_mut() {
            cpu[..n].copy_from_slice(&data[..n]);
        }
        if let (Some(cpu), Some(gpu)) = (self.kv_cache_cpu.as_ref(), self.kv_cache_gpu.as_ref()) {
            self.gpu_queue()
                .write_buffer(gpu, 0, bytemuck::cast_slice(&cpu[..n]));
        }
    }

    /// Read the entire GPU KV-cache arena back to host as `f32` (native, cold path — forge KV capture,
    /// not the decode hot path). The returned flat buffer is interpretable via `KvCacheLayout::
    /// k_index`/`v_index` **only when the layout is f32** (int8 KV disabled at load); with an int8
    /// layout the bytes are packed i8 lanes + scales and this returns `None`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_kv_cache_gpu(&self) -> Option<Vec<f32>> {
        let gpu = self.kv_cache_gpu.as_ref()?;
        let layout = self.kv_layout.as_ref()?;
        if layout.int8 {
            return None;
        }
        let n = layout.total_f32_elems;
        let size = (n * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let staging = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("KvCacheReadback"),
            size: size.max(4),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder =
            self.gpu_device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("KvReadback"),
                });
        encoder.copy_buffer_to_buffer(gpu, 0, &staging, 0, size);
        self.gpu_queue().submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |v| {
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
        let data = slice.get_mapped_range();
        let out: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging.unmap();
        Some(out)
    }

    /// Decode the current f32 KV cache into per-layer K and V vectors for token positions `0..n_tokens`
    /// (capped at `max_per_layer` per layer per stream). The **GPU-readback** capture route for the
    /// sparse-KV-dictionary go/no-go — reads the real decode-path K/V straight from VRAM, no CPU
    /// reference forward. Returns `None` on an int8 layout or if readback fails.
    #[cfg(not(target_arch = "wasm32"))]
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

    pub(crate) fn ensure_gemm_buffers(&mut self, max_weight_bytes: usize, max_out_dim: u32) {
        // A1a: build the persistent GPU top-k pipeline + candidate buffers once (additive; the
        // existing argmax path is unaffected whether or not this succeeds).
        #[cfg(not(target_arch = "wasm32"))]
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
        let w_bytes = max_weight_bytes.max(4) as wgpu::BufferAddress;
        let in_bytes = (need_input * 4) as wgpu::BufferAddress;
        let out_bytes = (max_out_dim as usize * 4).max(4) as wgpu::BufferAddress;
        self.gemm_input_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmInput"),
            size: in_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));
        self.gemm_weight_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmWeight"),
            size: w_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        #[cfg(target_arch = "wasm32")]
        {
            let weight_usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
            let mk = |label: &str| {
                self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: w_bytes,
                    usage: weight_usage,
                    mapped_at_creation: false,
                })
            };
            let qkv_k = mk("MC8WeightAttnK");
            let qkv_v = mk("MC8WeightAttnV");
            let qkv_q = mk("MC8WeightAttnQ");
            let o_proj = mk("MC8WeightOProj");
            let gate = mk("MC8WeightGate");
            let up = mk("MC8WeightUp");
            let down = mk("MC8WeightDown");
            let weight_b = mk("LayerGemmWeightB");
            self.mc8_weight_arena = Some(Mc8WeightArenaBufs {
                qkv_k,
                qkv_v,
                qkv_q,
                o_proj,
                gate,
                up,
                down,
            });
            self.gemm_weight_buf_b = Some(weight_b);
        }
        self.gemm_output_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmOutput"),
            size: out_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        let gemm_params_bytes = {
            #[cfg(target_arch = "wasm32")]
            {
                (MC8_MAX_GEMM_UNIFORM_CHUNK_SLOTS * MC8_UNIFORM_ALIGN) as wgpu::BufferAddress
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::mem::size_of::<GemmGpuParams>() as wgpu::BufferAddress
            }
        };
        self.gemm_params_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmParams"),
            size: gemm_params_bytes.max(4),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        #[cfg(target_arch = "wasm32")]
        let staging_bytes = out_bytes.max((65536 * 4) as wgpu::BufferAddress);
        #[cfg(not(target_arch = "wasm32"))]
        let staging_bytes = out_bytes;
        self.gemm_output_staging = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmStaging"),
            size: staging_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.gemm_aux_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmAux"),
            size: out_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));
        self.gemm_ffn_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmFfnUp"),
            size: out_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));
        #[cfg(target_arch = "wasm32")]
        {
            self.prefill_scratch_buf =
                Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                    label: Some("PrefillBatchScratch"),
                    size: in_bytes,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }));
            // Per-token row: norm + gate + up + save (see encode_prefill_q_ffn_tail_fused).
            let work_row_floats =
                (MAX_HIDDEN_DIM + 2 * max_out_dim as usize + MAX_HIDDEN_DIM).max(4);
            let work_bytes =
                (PREFILL_CHUNK_SIZE * work_row_floats * 4).max(4) as wgpu::BufferAddress;
            let work_usage = wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC;
            self.prefill_work_buf_a =
                Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                    label: Some("PrefillBatchWorkA"),
                    size: work_bytes,
                    usage: work_usage,
                    mapped_at_creation: false,
                }));
            self.prefill_work_buf_b =
                Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                    label: Some("PrefillBatchWorkB"),
                    size: work_bytes,
                    usage: work_usage,
                    mapped_at_creation: false,
                }));
            // Phase 5.5: Q/K/V projection scratch (parallel-GEMM output). work_bytes ≥ q_dim×tokens.
            self.mc8_q_proj_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("MC8QProj"),
                size: work_bytes,
                usage: work_usage,
                mapped_at_creation: false,
            }));
            self.mc8_k_proj_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("MC8KProj"),
                size: work_bytes,
                usage: work_usage,
                mapped_at_creation: false,
            }));
            self.mc8_v_proj_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("MC8VProj"),
                size: work_bytes,
                usage: work_usage,
                mapped_at_creation: false,
            }));
        }
        let elem_params_bytes = {
            #[cfg(target_arch = "wasm32")]
            {
                (MC8_MAX_ELEM_UNIFORM_CHUNK_SLOTS * MC8_UNIFORM_ALIGN) as wgpu::BufferAddress
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::mem::size_of::<ElemGpuParams>() as wgpu::BufferAddress
            }
        };
        self.elem_params_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("ElemParams"),
            size: elem_params_bytes.max(4),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        let norm_bytes = (MAX_HIDDEN_DIM * 4) as wgpu::BufferAddress;
        self.norm_weight_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("NormWeights"),
            size: norm_bytes.max(4),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.gemm_max_out_dim = max_out_dim;
        self.gemm_max_input_floats = need_input;
        self.max_tensor_bytes = max_weight_bytes;
    }
}
