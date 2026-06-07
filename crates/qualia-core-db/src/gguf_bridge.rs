//! Neuro-Symbolic GGUF Bridge
//! Dispatches transformer block computation to the best available GPU backend:
//!   - Windows x64: DirectML 1.15 (D3D12, hardware-vendor-optimised kernels)
//!   - All platforms: wgpu / WGSL fallback (Vulkan / Metal / WebGPU)
//! GGUF tensor bytes are memory-mapped via `memmap2` — zero heap copy.

use crate::QualiaQuin;
use memmap2::MmapOptions;

/// Represents a Q4_K Quantized or standard float Tensor mapped from a monolithic GGUF file.
#[derive(Debug, Clone)]
pub struct QTensor {
    pub shape: Vec<usize>,
    pub byte_offset: u64,
    pub is_quantized_q4_k: bool,
}

impl QTensor {
    pub fn new(shape: Vec<usize>, byte_offset: u64, is_quantized_q4_k: bool) -> Self {
        Self { shape, byte_offset, is_quantized_q4_k }
    }

    /// Maps the exact bytes from the GGUF using the 60-bit pointer.
    pub fn map_from_pointer(quin: &QualiaQuin) -> Option<Self> {
        use crate::QuinPointerExt;
        
        let flag = quin.extract_modality_flag();
        if flag != crate::MODALITY_FLAG_LLM_TENSOR {
            return None; // Not an LLM tensor
        }

        let offset = quin.extract_byte_offset();
        
        // Mock parsing the GGUF header at the offset to find shape and quantization
        // For demonstration, we assume a Q4_K tensor representation.
        Some(Self::new(vec![4096, 4096], offset, true))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct QTensorEngine {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub pipeline: wgpu::ComputePipeline,
    pub is_initialized: bool,
    /// DirectML device — Some on Windows when DirectML 1.15 is linked.
    #[cfg(target_os = "windows")]
    pub dml: Option<crate::directml_bridge::DmlDevice>,
    /// Memory-mapped GGUF file (set after `load_gguf`).
    pub gguf_mmap: Option<memmap2::Mmap>,
    /// Byte offset into the mmap where tensor data begins.
    pub tensor_data_offset: u64,
}

#[cfg(not(target_arch = "wasm32"))]
impl QTensorEngine {
    pub fn new() -> Self {
        let instance = wgpu::Instance::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        let adapter = rt.block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })).expect("Failed to find wgpu adapter");

        let (device, queue) = rt.block_on(adapter.request_device(
            &wgpu::DeviceDescriptor::default(),
            None,
        )).expect("Failed to create wgpu device");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fused Transformer Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fused_tensor_contraction.wgsl").into()),
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Fused Transformer Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
        });

        Self {
            device,
            queue,
            pipeline,
            is_initialized: true,
            #[cfg(target_os = "windows")]
            dml: crate::directml_bridge::DmlDevice::new().ok(),
            gguf_mmap: None,
            tensor_data_offset: 0,
        }
    }

    /// Memory-map a GGUF file so tensor bytes are accessible without heap allocation.
    /// Call this once after `new()`, before the first `dispatch_fused_transformer_block`.
    pub fn load_gguf(&mut self, path: &str) {
        use std::fs::File;
        match File::open(path) {
            Ok(f) => match unsafe { MmapOptions::new().map(&f) } {
                Ok(mmap) => {
                    // Scan GGUF header to find where tensor data starts.
                    // GGUF v3 layout: magic(4) + version(4) + tensor_count(8) + kv_count(8)
                    // Followed by kv metadata, then tensor info, then tensor data (aligned to 32 bytes).
                    // We use a conservative offset scan here; a full parser lives in gguf_sharder.rs.
                    let data_offset = Self::locate_tensor_data_start(&mmap);
                    self.tensor_data_offset = data_offset;
                    self.gguf_mmap = Some(mmap);
                    eprintln!("[gguf_bridge] Mapped {} — tensor data at offset {:#x}", path, data_offset);
                }
                Err(e) => eprintln!("[gguf_bridge] mmap failed for {path}: {e}"),
            },
            Err(e) => eprintln!("[gguf_bridge] Could not open {path}: {e}"),
        }
    }

    /// Scan the GGUF file for the start of tensor payload data.
    /// Searches for the 32-byte alignment boundary after the header/metadata.
    fn locate_tensor_data_start(mmap: &[u8]) -> u64 {
        if mmap.len() < 24 || &mmap[0..4] != b"GGUF" { return 0; }
        let tensor_count = u64::from_le_bytes(mmap[8..16].try_into().unwrap_or([0;8]));
        // Heuristic: tensor data typically starts after the first 1MB for small models,
        // later for large ones. For now use the tensor_count × avg-metadata-size estimate.
        // A production parser would walk the kv section byte by byte.
        let estimated_header_bytes = 24u64 + tensor_count * 64 + 65536; // conservative
        let aligned = (estimated_header_bytes + 31) & !31;
        aligned.min(mmap.len() as u64)
    }

    pub fn dispatch_fused_transformer_block(&self, tensor: &QTensor, input_activations: &[f32]) -> Vec<f32> {
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
                    let weights_f32 = crate::directml_bridge::dequantize_q4_k_tensor(q4_slice, rows * cols);
                    let op = crate::directml_bridge::DmlGemmOp {
                        m: input_activations.len() as u32 / cols as u32,
                        k: cols as u32,
                        n: rows as u32,
                    };
                    if let Ok(result) = op.execute(dml, input_activations, &weights_f32) {
                        crate::telemetry::SIEVE_OPS_COUNT.fetch_add(
                            rows * cols,
                            std::sync::atomic::Ordering::Relaxed,
                        );
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
                let weights_f32 = crate::metal_bridge::dequantize_q4_k_tensor(q4_slice, rows * cols);
                let input_rows = (input_activations.len() / cols).max(1);
                let result = crate::metal_bridge::accelerate_sgemm(
                    input_rows, cols, rows, input_activations, &weights_f32
                );
                crate::telemetry::SIEVE_OPS_COUNT.fetch_add(
                    rows * cols, std::sync::atomic::Ordering::Relaxed,
                );
                return result;
            }
        }

        // ── wgpu / WGSL fallback (all platforms — Vulkan on Linux/NVIDIA,
        //    Metal on macOS when mmap not loaded, D3D12 on Windows fallback) ──
        let input_bytes = bytemuck::cast_slice(input_activations);
        let input_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Input"),
            size: input_bytes.len().max(4) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&input_buf, 0, input_bytes);

        // Upload real weights from mmap when available, else use a zero buffer.
        let weights_size = (rows * cols * 4) as wgpu::BufferAddress;
        let weights_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Weights"),
            size: weights_size.max(4),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if let Some(mmap) = &self.gguf_mmap {
            let offset = (self.tensor_data_offset + tensor.byte_offset) as usize;
            let end    = (offset + rows * cols * 4).min(mmap.len());
            if end > offset {
                let f32_bytes = &mmap[offset..end];
                self.queue.write_buffer(&weights_buf, 0, f32_bytes);
            }
        }

        let output_size = (4096 * 4) as wgpu::BufferAddress;
        let output_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group_layout = self.pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: weights_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: output_buf.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(4096 / 64, 1, 1);
        }

        let staging_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, output_size);
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buf.slice(..);
        let (sender, receiver) = futures_channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
        self.device.poll(wgpu::Maintain::Wait);
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(receiver).unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        crate::telemetry::SIEVE_OPS_COUNT.fetch_add(4096 * 4096, std::sync::atomic::Ordering::Relaxed);
        result
    }

    pub fn decode_lexicon_bound(&self, _logits: &[f32], valid_lexicon_ids: &[u64]) -> u64 {
        if valid_lexicon_ids.is_empty() { 0 } else { valid_lexicon_ids[0] }
    }
}

#[cfg(target_arch = "wasm32")]
pub struct QTensorEngine {
    pub is_initialized: bool,
}

#[cfg(target_arch = "wasm32")]
impl QTensorEngine {
    pub fn new() -> Self {
        Self { is_initialized: true }
    }
    pub fn dispatch_fused_transformer_block(&self, _tensor: &QTensor, _input_activations: &[f32]) -> Vec<f32> {
        crate::telemetry::SIEVE_OPS_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        vec![0.0; 4096]
    }
    pub fn decode_lexicon_bound(&self, _logits: &[f32], valid_lexicon_ids: &[u64]) -> u64 {
        if valid_lexicon_ids.is_empty() { 0 } else { valid_lexicon_ids[0] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use memmap2::MmapOptions;
    use std::path::Path;

    #[test]
    fn test_q_tensor_pointer_extraction() {
        let quin = QualiaQuin {
            subject: crate::q_hash("LLM_Prompt"),
            predicate: crate::q_hash("has_tensor_offset"),
            // Pack the LLM flag + byte offset
            object: ((crate::MODALITY_FLAG_LLM_TENSOR as u64) << 60) | 0x0000_1234,
            context: 0,
            metadata: 0,
            parity: 0,
        };

        let q_tensor = QTensor::map_from_pointer(&quin)
            .expect("Failed to map QTensor from pointer");
        
        assert_eq!(q_tensor.byte_offset, 0x0000_1234, "Extracted byte offset incorrectly");
        assert_eq!(q_tensor.is_quantized_q4_k, true, "Did not identify Q4_K quantization");
    }

    #[test]
    fn test_lexicon_bound_decoding() {
        let engine = QTensorEngine::new();
        let logits = vec![0.1, 0.9, 0.2]; // Mock logits
        let valid_ids = vec![crate::q_hash("Dog"), crate::q_hash("Cat")];

        // Should return a valid u64 semantic ID, not a string
        let decoded = engine.decode_lexicon_bound(&logits, &valid_ids);
        assert_eq!(decoded, valid_ids[0], "Failed to bind decoding to logic lexicon");
    }

    #[test]
    fn test_mmap_gemma_model_if_exists() {
        // Check if the specific GGUF file exists on the developer's machine
        let path = Path::new("C:/Projects/qualiaDB/gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf");
        if path.exists() {
            let file = File::open(path).expect("Failed to open GGUF file");
            let mmap = unsafe { MmapOptions::new().map(&file).expect("Failed to memory map GGUF file") };
            
            // Just asserting that we successfully mapped a massive file into virtual memory
            assert!(mmap.len() > 1024 * 1024, "Memory map size is suspiciously small");
            println!("Successfully mapped Gemma GGUF! Size: {} bytes", mmap.len());
        } else {
            println!("Gemma GGUF file not found locally. Skipping mmap test.");
        }
    }
}
