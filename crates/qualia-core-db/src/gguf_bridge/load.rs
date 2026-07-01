//! Model load + residency: GGUF load, ternary-FFN resident build, resident mmap adoption,
//! resident logits upload, KV-cache sizing, decode-profiler hooks. Split from mod.rs (structural).
use super::*;

impl QTensorEngine {
    pub fn kv_cache_bytes(&self) -> u64 {
        self.kv_layout
            .as_ref()
            .map(|layout| (layout.total_f32_elems * std::mem::size_of::<f32>()) as u64)
            .unwrap_or(0)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_gguf_checked(&mut self, path: &str) -> Result<GgufLoadReport, String> {
        use std::fs::File;

        log::info!("LLM_LOAD|gguf-open|0.52|Opening GGUF file {}", path);
        let file = File::open(path).map_err(|e| {
            log::error!("GGUF mmap open failed for {}: {}", path, e);
            log::error!("LLM_LOAD|failed|1.00|Could not open GGUF: {}", e);
            e.to_string()
        })?;
        log::info!("LLM_LOAD|mmap-start|0.64|Memory-mapping GGUF into virtual memory");
        let mmap = unsafe { MmapOptions::new().map(&file) }.map_err(|e| {
            log::error!("GGUF mmap failed for {}: {}", path, e);
            log::error!("LLM_LOAD|failed|1.00|Memory map failed: {}", e);
            e.to_string()
        })?;
        let file_size = mmap.len();
        log::info!(
            "LLM_LOAD|ram-map|0.70|Mapped {:.2} GiB GGUF into system memory",
            bytes_to_gib(file_size as u64)
        );
        let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
        if index.tensor_data_start == 0
            && index.max_tensor_bytes == 0
            && index.hyperparams.n_layer == 0
        {
            let msg = "GGUF header parse failed or yielded no tensor metadata".to_string();
            log::error!("LLM_LOAD|failed|1.00|{}", msg);
            return Err(msg);
        }

        self.tensor_data_offset = index.tensor_data_start;
        self.hyperparams = index.hyperparams;
        let staging = index
            .max_layer_tensor_bytes
            .max(4096)
            .min(MAX_WGPU_WEIGHT_STAGING);
        self.ensure_gemm_buffers(staging, MAX_STACK_GEMM_OUT as u32);
        self.ensure_kv_cache(&index.hyperparams);
        self.gguf_mmap = Some(Arc::new(mmap));

        let kv_cache_bytes = self.kv_cache_bytes();
        log::info!(
            "LLM_LOAD|gguf-index|0.78|Parsed {} layers, {} attention heads",
            self.hyperparams.n_layer,
            self.hyperparams.n_head
        );
        log::info!(
            "LLM_LOAD|gguf-ready|0.92|GGUF indexed and cache arena reserved ({} MiB)",
            kv_cache_bytes / (1024 * 1024)
        );

        Ok(GgufLoadReport {
            mapped_bytes: file_size as u64,
            tensor_data_offset: self.tensor_data_offset,
            n_layer: self.hyperparams.n_layer,
            n_head: self.hyperparams.n_head,
            n_kv_head: self.hyperparams.effective_n_kv_head(),
            max_tensor_bytes: index.max_tensor_bytes,
            kv_cache_bytes,
            directml_enabled: {
                #[cfg(target_os = "windows")]
                {
                    self.dml.is_some()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    false
                }
            },
        })
    }

    /// Memory-map a GGUF file so tensor bytes are accessible without heap allocation.
    /// Call this once after `new()`, before the first `dispatch_fused_transformer_block`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_gguf(&mut self, path: &str) {
        if let Err(e) = self.load_gguf_checked(path) {
            eprintln!("[gguf_bridge] Could not load {path}: {e}");
        }
    }

    /// Memory-map and auto-detect a supported local model container.
    ///
    /// Canonical P64 is detected by its exact `p64\0` magic and adopted through
    /// the P64 validation path. All other inputs are passed to the GGUF parser,
    /// which rejects malformed or unsupported data.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_model_checked(&mut self, path: &str) -> Result<GgufLoadReport, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("open {path}: {e}"))?;
        let mmap = std::sync::Arc::new(
            unsafe { memmap2::MmapOptions::new().map(&file) }.map_err(|e| e.to_string())?,
        );
        if crate::p64_weight::has_p64_magic(&mmap[..]) {
            self.adopt_resident_p64_mmap(mmap)
        } else {
            self.adopt_resident_mmap(mmap)
        }
    }

    /// Fail-soft wrapper retained for the agent decode path.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_model(&mut self, path: &str) {
        if let Err(e) = self.load_model_checked(path) {
            eprintln!("[gguf_bridge] Could not load model {path}: {e}");
        }
    }

    /// Build the resident 2-bit ternary-FFN dispatcher from a P64 container's base-3 FFN
    /// blobs (rebaked to 2-bit + uploaded once). Returns false if there are no ternary FFN tensors
    /// or the GPU build fails — the FFN then runs the CPU oracle (`dispatch_ternary_ffn` fallback).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn build_ternary_ffn_resident(
        &mut self,
        q: &crate::p64_weight::P64TensorIndex,
    ) -> bool {
        let mmap_arc = match self.gguf_mmap.clone() {
            Some(a) => a,
            None => return false,
        };
        let data: &[u8] = &mmap_arc;
        let mut tensors: Vec<(u64, usize, usize, &[u8])> = Vec::new();
        for e in &q.entries {
            if e.dtype as u32 != crate::ternary::GGML_TYPE_TERNARY_158 {
                continue;
            }
            let (n_in, n_out) = (e.dimensions[0] as usize, e.dimensions[1] as usize);
            let (off, len) = (e.blob_offset as usize, e.blob_size as usize);
            if n_in == 0 || n_out == 0 || off + len > data.len() {
                continue;
            }
            // key = the P64 blob offset == the synthetic index's GgufTensorInfo::byte_offset.
            tensors.push((e.blob_offset as u64, n_in, n_out, &data[off..off + len]));
        }
        if tensors.is_empty() {
            return false;
        }
        match crate::ternary_gpu::TernaryFfnResident::build(
            self.gpu_device(),
            self.gpu_queue(),
            &tensors,
        ) {
            Some(r) => {
                log::info!(
                    "LLM_LOAD|ternary-ffn|0.71|resident 2-bit FFN: {} tensors, {:.1} MB",
                    r.len(),
                    r.resident_bytes() as f64 / (1024.0 * 1024.0)
                );
                self.ternary_ffn = Some(r);
                true
            }
            None => false,
        }
    }

    /// Boot from an already-mapped P64 weight container (native). Mirrors the GGUF
    /// `adopt_resident_mmap` but for the `P64` format: validates + builds a synthetic GGUF index
    /// from the manifest, points the byte source at the P64 bytes (`tensor_data_start = 0`,
    /// absolute blob offsets), reserves the GEMM/KV arenas, makes the (verbatim) output projection
    /// resident, and builds the resident 2-bit ternary-FFN dispatcher from the FFN blobs. The
    /// attention/norm/embed tensors stay at source precision and run the standard GGUF hot path.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn adopt_resident_p64_mmap(
        &mut self,
        mmap: Arc<memmap2::Mmap>,
    ) -> Result<GgufLoadReport, String> {
        let file_size = mmap.len();
        if file_size == 0 {
            return Err("Empty P64 mmap".to_string());
        }
        let q = crate::p64_weight::P64TensorIndex::from_p64(&mmap[..])?;
        let index = q.to_gguf_index();
        let hp = index.hyperparams;
        if hp.n_layer == 0 || hp.n_embd == 0 {
            return Err("P64: missing hyperparameters in header".to_string());
        }
        self.hyperparams = hp;
        self.tensor_data_offset = 0; // P64 blob offsets are absolute
        let staging = index
            .max_layer_tensor_bytes
            .max(4096)
            .min(MAX_WGPU_WEIGHT_STAGING);
        self.ensure_gemm_buffers(staging, MAX_STACK_GEMM_OUT as u32);
        self.ensure_kv_cache(&hp);
        if self.kv_layout.is_none() || self.kv_cache_cpu.is_none() {
            return Err("P64: KV cache allocation failed".to_string());
        }
        self.gguf_mmap = Some(mmap);
        if !self.mc8_upload_resident_logits(&index) {
            log::info!("LLM_LOAD|p64-logits|0.70|skipped — per-token upload fallback");
        }
        if !self.build_ternary_ffn_resident(&q) {
            log::info!(
                "LLM_LOAD|ternary-ffn|0.71|no resident set (no ternary FFN or build failed) — CPU oracle path"
            );
        }
        let kv_cache_bytes = self.kv_cache_bytes();
        Ok(GgufLoadReport {
            mapped_bytes: file_size as u64,
            tensor_data_offset: 0,
            n_layer: hp.n_layer,
            n_head: hp.n_head,
            n_kv_head: hp.effective_n_kv_head(),
            max_tensor_bytes: index.max_tensor_bytes,
            kv_cache_bytes,
            directml_enabled: {
                #[cfg(target_os = "windows")]
                {
                    self.dml.is_some()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    false
                }
            },
        })
    }

    /// Compatibility alias for the historical pre-P64 API name.
    #[deprecated(note = "use adopt_resident_p64_mmap")]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn adopt_resident_q42_mmap(
        &mut self,
        mmap: Arc<memmap2::Mmap>,
    ) -> Result<GgufLoadReport, String> {
        self.adopt_resident_p64_mmap(mmap)
    }

    /// A1b: number of resident ternary FFN tensors (0 unless a ternary P64 was adopted). Lets a
    /// test confirm the GPU resident path is actually populated (not a silent CPU-only fallback).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn ternary_ffn_resident_len(&self) -> usize {
        self.ternary_ffn.as_ref().map_or(0, |r| r.len())
    }

    /// Attach an already-mapped resident GGUF (shared with orchestrator slot).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn adopt_resident_mmap(
        &mut self,
        mmap: Arc<memmap2::Mmap>,
    ) -> Result<GgufLoadReport, String> {
        let file_size = mmap.len();
        if file_size == 0 {
            return Err("Empty GGUF mmap".to_string());
        }
        log::info!(
            "LLM_LOAD|resident-mmap|0.68|Reusing resident GGUF mapping ({:.2} GiB)",
            bytes_to_gib(file_size as u64)
        );
        let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(mmap.as_ref());
        if index.tensor_data_start == 0
            && index.max_tensor_bytes == 0
            && index.hyperparams.n_layer == 0
        {
            return Err("GGUF header parse failed or yielded no tensor metadata".to_string());
        }
        self.tensor_data_offset = index.tensor_data_start;
        self.hyperparams = index.hyperparams;
        let staging = index
            .max_layer_tensor_bytes
            .max(4096)
            .min(MAX_WGPU_WEIGHT_STAGING);
        self.ensure_gemm_buffers(staging, MAX_STACK_GEMM_OUT as u32);
        self.ensure_kv_cache(&index.hyperparams);
        self.gguf_mmap = Some(mmap);
        // A1a step-2: make the output/logits projection resident (upload once) so the per-token
        // top-k decode binds per-chunk 256-aligned sub-ranges instead of re-uploading the whole
        // ~47 MB matrix every token (the documented decode throughput killer). Fail-soft: a false
        // return leaves `mc8_logits_resident_buf=None` and the decode keeps its per-token upload.
        if !self.mc8_upload_resident_logits(&index) {
            log::info!("LLM_LOAD|resident-logits|0.70|skipped — per-token upload fallback");
        }
        let kv_cache_bytes = self.kv_cache_bytes();
        Ok(GgufLoadReport {
            mapped_bytes: file_size as u64,
            tensor_data_offset: self.tensor_data_offset,
            n_layer: self.hyperparams.n_layer,
            n_head: self.hyperparams.n_head,
            n_kv_head: self.hyperparams.effective_n_kv_head(),
            max_tensor_bytes: index.max_tensor_bytes,
            kv_cache_bytes,
            directml_enabled: {
                #[cfg(target_os = "windows")]
                {
                    self.dml.is_some()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    false
                }
            },
        })
    }

    /// A1a step-2 (native port of Phase 5.3): upload the output/logits projection (tied
    /// `token_embd`) to a resident `STORAGE` buffer **once**, so the per-token top-k decode binds
    /// per-chunk 256-aligned sub-ranges instead of re-uploading the whole ~47 MB matrix every
    /// token (the decode throughput killer). Idempotent. Returns false (→ per-token upload
    /// fallback) if the projection is missing or its bytes don't divide evenly into rows.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn mc8_upload_resident_logits(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
    ) -> bool {
        if self.mc8_logits_resident_buf.is_some() {
            return true;
        }
        let info = match index.logits_projection_info() {
            Some(i) => i,
            None => return false,
        };
        let (_, vocab) = Self::matmul_dims(info);
        if vocab == 0 {
            return false;
        }
        // Clone the Arc so the mmap borrow does not block mutating `self` below.
        let mmap_arc = match self.gguf_mmap.clone() {
            Some(a) => a,
            None => return false,
        };
        let mmap: &[u8] = &mmap_arc;
        let raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, info)
        {
            Ok(s) => s,
            Err(_) => return false,
        };
        let total = raw.len();
        if total == 0 || total % vocab != 0 {
            return false;
        }
        let row_bytes = total / vocab;
        // VOCAB_CHUNK_ROWS is a multiple of 256, so every chunk's byte offset
        // (chunk_idx * VOCAB_CHUNK_ROWS * row_bytes) is 256-aligned for the storage binding.
        let buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("ResidentLogits"),
            size: total as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_queue().write_buffer(&buf, 0, raw);
        self.mc8_logits_resident_buf = Some(buf);
        self.mc8_logits_row_bytes = row_bytes as u32;
        log::info!(
            "LLM_LOAD|resident-logits|0.70|output projection resident once: {:.1} MB ({} rows x {} B)",
            total as f64 / (1024.0 * 1024.0),
            vocab,
            row_bytes
        );
        true
    }

    /// Decode-profiler: blocking GPU fence wait + round-trip counter. Every native sync point routes
    /// through this (via the `self.gpu_device().poll(Maintain::Wait)` → `self.poll_wait()` rewrite),
    /// so the bench can count submit→wait round-trips per token and separate synchronization stall
    /// from real kernel time. Behaviourally identical to a bare blocking poll.
    #[inline]
    pub(crate) fn poll_wait(&self) {
        let _ = self.gpu_device().poll(wgpu::PollType::wait_indefinitely());
        GPU_WAIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Decode-profiler: wall-clock for `n` EMPTY `submit → poll(Maintain::Wait)` round-trips (no
    /// compute dispatched). Isolates the fixed CPU↔GPU fence latency: if a token's forward time ≈
    /// (its round-trip count × this per-round-trip cost), the bottleneck is synchronization, not
    /// math; if forward ≫ that, the kernels themselves are slow. Does NOT touch `GPU_WAIT_COUNT`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn bench_empty_submit_roundtrip(&self, n: u32) -> u64 {
        let t = std::time::Instant::now();
        for _ in 0..n {
            let enc = self
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("EmptyRT"),
                });
            self.gpu_queue().submit(Some(enc.finish()));
            let _ = self.gpu_device().poll(wgpu::PollType::wait_indefinitely());
        }
        t.elapsed().as_nanos() as u64
    }
    #[cfg(target_arch = "wasm32")]
    pub fn adopt_resident_mmap(&mut self, mmap: Arc<[u8]>) -> Result<GgufLoadReport, String> {
        let file_size = mmap.len();
        if file_size == 0 {
            return Err("Empty GGUF mmap".to_string());
        }
        log::info!(
            "LLM_LOAD|resident-mmap|0.68|Reusing resident GGUF mapping ({:.2} GiB)",
            bytes_to_gib(file_size as u64)
        );
        let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(mmap.as_ref());
        if index.tensor_data_start == 0
            && index.max_tensor_bytes == 0
            && index.hyperparams.n_layer == 0
        {
            return Err("GGUF header parse failed or yielded no tensor metadata".to_string());
        }
        self.tensor_data_offset = index.tensor_data_start;
        self.hyperparams = index.hyperparams;
        let staging = index
            .max_layer_tensor_bytes
            .max(4096)
            .min(MAX_WGPU_WEIGHT_STAGING);
        self.ensure_gemm_buffers(staging, MAX_STACK_GEMM_OUT as u32);
        self.ensure_kv_cache(&index.hyperparams);
        if self.kv_layout.is_none() || self.kv_cache_cpu.is_none() {
            return Err("KV cache allocation failed (layout or CPU mirror missing)".to_string());
        }
        self.gguf_mmap = Some(mmap);
        // Part 3y: stage all layer weights to the GPU now (init time, before the TTFT clock),
        // so the 219 MB upload is not charged to the first token's latency.
        if !self.mc8_upload_all_resident_weights(&index) {
            wlog("[MC8] eager resident weight upload skipped at init — will retry lazily");
        }
        // Phase 5.3: also make the output/logits projection resident (eliminates the ~50 MB
        // per-token re-upload in the decode argmax).
        if !self.mc8_upload_resident_logits(&index) {
            wlog("[MC8] resident logits projection skipped at init — per-token upload fallback");
        }
        // Phase 5.4: norm weights resident (removes the per-layer norm write_buffer race).
        if !self.mc8_upload_resident_norms(&index) {
            wlog("[MC8] resident norm weights skipped at init — per-layer upload fallback");
        }
        let kv_cache_bytes = self.kv_cache_bytes();
        Ok(GgufLoadReport {
            mapped_bytes: file_size as u64,
            tensor_data_offset: self.tensor_data_offset,
            n_layer: self.hyperparams.n_layer,
            n_head: self.hyperparams.n_head,
            n_kv_head: self.hyperparams.effective_n_kv_head(),
            max_tensor_bytes: index.max_tensor_bytes,
            kv_cache_bytes,
            directml_enabled: {
                #[cfg(target_os = "windows")]
                {
                    self.dml.is_some()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    false
                }
            },
        })
    }
}
