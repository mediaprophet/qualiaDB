use std::sync::Arc;

use crate::gguf_bridge::QTensorEngine;
use crate::gguf_sharder::{GgufTensorIndex, GgufTokenizer};

pub(super) struct RawModel {
    pub engine: QTensorEngine,
    pub mmap: Arc<memmap2::Mmap>,
    pub tokenizer: GgufTokenizer,
    pub index: GgufTensorIndex,
    pub emb: Vec<f32>,
    pub scratch_a: Vec<f32>,
    pub scratch_b: Vec<f32>,
}

impl RawModel {
    pub fn load(model_path: &str) -> Result<Self, String> {
        let mut engine = QTensorEngine::new();
        // The checked activation path parses the format and eagerly uploads resident layer
        // weights, norms, and logits. `load_gguf` alone only maps/indexes and is therefore not a
        // valid prepared resident benchmark.
        engine.load_model_checked(model_path)?;

        let mmap = engine
            .gguf_mmap
            .clone()
            .ok_or_else(|| "model did not memory-map".to_string())?;
        let is_p64 = mmap.len() >= 4 && mmap[0..4] == *b"p64\0";
        let (tokenizer, index) = if is_p64 {
            let p64 = crate::p64_weight::P64TensorIndex::from_p64(&mmap)
                .map_err(|e| format!("P64 index: {e}"))?;
            let tokenizer =
                GgufTokenizer::from_p64_section(p64.tokenizer_bytes(&mmap)).unwrap_or_default();
            (tokenizer, p64.to_gguf_index())
        } else {
            (
                GgufTokenizer::from_gguf(&mmap),
                GgufTensorIndex::from_gguf(&mmap),
            )
        };
        let emb_dim = index.emb_dim();
        if emb_dim == 0 {
            return Err("embedding dimension is zero".into());
        }

        Ok(Self {
            engine,
            mmap,
            tokenizer,
            index,
            emb: vec![0.0; emb_dim.max(8192)],
            scratch_a: vec![0.0; 16_384],
            scratch_b: vec![0.0; 16_384],
        })
    }

    pub fn prepare_prompt(
        &mut self,
        prompt_tokens: &[u32],
        cuda_prepared: bool,
    ) -> Result<(u32, u32), String> {
        if prompt_tokens.is_empty() {
            return Err("raw decode prompt produced zero tokens".into());
        }
        self.engine.reset_kv_cache();
        let emb_dim = self.index.emb_dim();
        for (position, token_id) in prompt_tokens
            .iter()
            .copied()
            .take(prompt_tokens.len().saturating_sub(1))
            .enumerate()
        {
            if cuda_prepared {
                let token = self
                    .engine
                    .try_cuda_mega_pass_decode_token(
                        &self.index,
                        token_id,
                        &mut self.emb[..emb_dim],
                        emb_dim,
                        position as u32,
                    )
                    .ok_or_else(|| {
                        "prepared CUDA decode became ineligible during prompt ingestion".to_string()
                    })?;
                if token == u32::MAX {
                    return Err(
                        "prepared CUDA prompt pass did not own the output projection".into(),
                    );
                }
            } else {
                self.load_embedding(token_id)?;
                let _ = self.engine.dispatch_transformer_forward(
                    &self.index,
                    &mut self.emb[..emb_dim],
                    emb_dim,
                    &mut self.scratch_a,
                    &mut self.scratch_b,
                    position as u32,
                    0,
                );
            }
        }
        Ok((
            *prompt_tokens.last().unwrap(),
            prompt_tokens.len().saturating_sub(1) as u32,
        ))
    }

    pub fn load_embedding(&mut self, token_id: u32) -> Result<(), String> {
        let emb_dim = self.index.emb_dim();
        let written = self.index.dequantize_token_embedding_into(
            &self.mmap,
            token_id,
            &mut self.emb[..emb_dim],
        );
        if written == 0 {
            Err(format!("embedding lookup failed for token {token_id}"))
        } else {
            Ok(())
        }
    }
}
