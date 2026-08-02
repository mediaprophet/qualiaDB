//! Compact WebGPU top-1 reduction for browser decode.
//!
//! Logits stay on-device. Each vocabulary chunk is reduced to one candidate
//! per 1024-logit block and only the bounded candidate pairs are mapped.

use crate::gguf_bridge::{QTensorEngine, StreamingArgmaxResult, VOCAB_CHUNK_ROWS, await_wgpu_map};

const PARAM_STRIDE: usize = 256;
const MAX_CHUNKS: usize = 32;
const PARAM_BYTES: usize = PARAM_STRIDE * MAX_CHUNKS;

#[derive(Clone, Copy)]
pub(crate) struct BrowserTop1Plan {
    vocab_size: usize,
    chunks: usize,
    total_candidates: usize,
}

impl BrowserTop1Plan {
    pub(crate) fn new(vocab_size: usize, max_chunks: u32) -> Option<Self> {
        let full_chunks = vocab_size.div_ceil(VOCAB_CHUNK_ROWS);
        let chunks = if max_chunks == 0 {
            full_chunks
        } else {
            (max_chunks as usize).min(full_chunks)
        };
        if vocab_size == 0 || chunks == 0 || chunks > MAX_CHUNKS {
            return None;
        }
        let mut total_candidates = 0usize;
        for chunk in 0..chunks {
            total_candidates +=
                Self::rows_for(vocab_size, chunk).div_ceil(crate::topk::TOPK_BLOCK_SIZE);
        }
        Some(Self {
            vocab_size,
            chunks,
            total_candidates,
        })
    }

    pub(crate) fn chunks(self) -> usize {
        self.chunks
    }

    pub(crate) fn rows(self, chunk: usize) -> usize {
        Self::rows_for(self.vocab_size, chunk)
    }

    pub(crate) fn readback_bytes(self) -> u64 {
        (self.total_candidates * 2 * core::mem::size_of::<u32>()) as u64
    }

    fn rows_for(vocab_size: usize, chunk: usize) -> usize {
        VOCAB_CHUNK_ROWS.min(vocab_size.saturating_sub(chunk * VOCAB_CHUNK_ROWS))
    }

    fn candidate_count(self, chunk: usize) -> usize {
        self.rows(chunk).div_ceil(crate::topk::TOPK_BLOCK_SIZE)
    }

    fn candidate_offset(self, chunk: usize) -> usize {
        let mut offset = 0usize;
        for previous in 0..chunk {
            offset += self.candidate_count(previous);
        }
        offset
    }
}

impl QTensorEngine {
    pub(crate) fn browser_top1_readback_bytes(&self, vocab_size: usize) -> Option<u32> {
        self.output_topk_pipeline.as_ref()?;
        self.mc8_logits_resident_buf.as_ref()?;
        let plan = BrowserTop1Plan::new(vocab_size, 0)?;
        u32::try_from(plan.readback_bytes()).ok()
    }

    pub(crate) fn prepare_browser_top1(&self, plan: BrowserTop1Plan) -> bool {
        let Some(params) = self.topk_params_buf.as_ref() else {
            return false;
        };
        if params.size() < PARAM_BYTES as u64 {
            return false;
        }
        let mut slab = [0u8; PARAM_BYTES];
        for chunk in 0..plan.chunks() {
            let bytes = crate::topk::topk_params_bytes(
                plan.rows(chunk) as u32,
                1,
                crate::topk::TOPK_BLOCK_SIZE as u32,
            );
            let offset = chunk * PARAM_STRIDE;
            slab[offset..offset + bytes.len()].copy_from_slice(&bytes);
        }
        self.gpu_queue()
            .write_buffer(params, 0, &slab[..plan.chunks() * PARAM_STRIDE]);
        true
    }

    pub(crate) fn encode_browser_top1_chunk(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        logits: &wgpu::Buffer,
        staging: &wgpu::Buffer,
        plan: BrowserTop1Plan,
        chunk: usize,
    ) -> bool {
        let (Some(pipeline), Some(layout), Some(params), Some(values), Some(indices)) = (
            self.output_topk_pipeline.as_ref(),
            self.output_topk_bind_layout.as_ref(),
            self.topk_params_buf.as_ref(),
            self.topk_cand_val_buf.as_ref(),
            self.topk_cand_idx_buf.as_ref(),
        ) else {
            return false;
        };
        if chunk >= plan.chunks() {
            return false;
        }
        let param_offset = (chunk * PARAM_STRIDE) as u64;
        let binding = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: params,
            offset: param_offset,
            size: core::num::NonZeroU64::new(16),
        });
        let bind = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("BrowserTop1Bind"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: logits.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: binding,
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: values.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: indices.as_entire_binding(),
                    },
                ],
            });
        let candidates = plan.candidate_count(chunk);
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("BrowserTop1Reduce"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(candidates as u32, 1, 1);
        }
        let bytes = (candidates * 4) as u64;
        let offset = plan.candidate_offset(chunk) as u64 * 4;
        let index_offset = (plan.total_candidates as u64 * 4) + offset;
        if index_offset + bytes > staging.size() {
            return false;
        }
        encoder.copy_buffer_to_buffer(values, 0, staging, offset, bytes);
        encoder.copy_buffer_to_buffer(indices, 0, staging, index_offset, bytes);
        true
    }

    pub(crate) async fn read_browser_top1(
        &self,
        staging: &wgpu::Buffer,
        plan: BrowserTop1Plan,
    ) -> Option<StreamingArgmaxResult> {
        let slice = staging.slice(..plan.readback_bytes());
        if !await_wgpu_map(slice).await {
            let _ = staging.unmap();
            return None;
        }
        let data = match slice.get_mapped_range() {
            Ok(data) => data,
            Err(_) => {
                staging.unmap();
                return None;
            }
        };
        let value_bytes = plan.total_candidates * 4;
        let values: &[f32] = bytemuck::cast_slice(&data[..value_bytes]);
        let indices: &[u32] = bytemuck::cast_slice(&data[value_bytes..value_bytes * 2]);
        let mut best_token_id = 0u32;
        let mut max_logit = f32::NEG_INFINITY;
        for chunk in 0..plan.chunks() {
            let offset = plan.candidate_offset(chunk);
            for candidate in 0..plan.candidate_count(chunk) {
                let position = offset + candidate;
                let token_id = (chunk * VOCAB_CHUNK_ROWS) as u32 + indices[position];
                let value = values[position];
                if value > f32::NEG_INFINITY
                    && (value > max_logit || (value == max_logit && token_id < best_token_id))
                {
                    best_token_id = token_id;
                    max_logit = value;
                }
            }
        }
        drop(data);
        staging.unmap();
        (max_logit > f32::NEG_INFINITY).then_some(StreamingArgmaxResult {
            best_token_id,
            max_logit,
        })
    }
}
