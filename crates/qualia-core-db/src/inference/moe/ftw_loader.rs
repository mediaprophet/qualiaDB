//! FTW (FreeToken Weight) format loader and multi-shard memory mapper (Work Package F9/F10).
//!
//! Loads `freetoken_weight.json` and memory-maps `.ftw` binary shards, providing
//! zero-copy access to shared weights and MoE expert banks (256 experts per layer).

use memmap2::Mmap;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use crate::inference::gguf_sharder::{
    GgufHyperparams, GgufTensorIndex, GgufTensorInfo, ARCH_QWEN2, DEFAULT_ROPE_FREQ_BASE,
};
use crate::inference::safetensor_loader::ModelJsonConfig;
use crate::inference::tensor_roles::name_to_role;

#[derive(Debug, Clone, Deserialize)]
pub struct FtwTensorEntry {
    pub name: String,
    pub kind: String,
    pub dtype: String,
    pub shape: Vec<u64>,
    pub global_off: u64,
    pub nbytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FtwShardEntry {
    pub file: String,
    pub global_off: u64,
    pub nbytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FtwManifest {
    pub format: String,
    pub version: u32,
    pub total_bytes: u64,
    pub tensors: Vec<FtwTensorEntry>,
    pub shards: Vec<FtwShardEntry>,
    #[serde(default)]
    pub quant_format: String,
    #[serde(default)]
    pub expert_bank_num_layers: Option<u32>,
}

/// Slices within the memory-mapped shards for a single expert in a given layer.
#[derive(Debug, Clone, Copy, Default)]
pub struct FtwExpertBankSlice {
    pub shard_idx: usize,
    pub gate_up_packed_off: usize,
    pub gate_up_packed_len: usize,
    pub gate_up_scale_off: usize,
    pub gate_up_scale_len: usize,
    pub gate_up_global_off: usize,
    pub gate_up_global_len: usize,
    pub down_packed_off: usize,
    pub down_packed_len: usize,
    pub down_scale_off: usize,
    pub down_scale_len: usize,
    pub down_global_off: usize,
    pub down_global_len: usize,
}

/// Zero-copy byte slices for one expert's NVFP4 tensors.
pub struct FtwExpertData<'a> {
    pub gate_up_packed: &'a [u8],
    pub gate_up_scale: &'a [u8],
    pub gate_up_global: &'a [u8],
    pub down_packed: &'a [u8],
    pub down_scale: &'a [u8],
    pub down_global: &'a [u8],
}

impl<'a> FtwExpertData<'a> {
    /// Convert zero-copy slice into typed `ExpertWeightView` for zero-heap SwiGLU computation.
    pub fn to_view(
        &self,
        emb_dim: usize,
        intermediate_dim: usize,
    ) -> super::dispatch::ExpertWeightView<'a> {
        let gate_up_global = if self.gate_up_global.len() >= 4 {
            let mut b = [0u8; 4];
            b.copy_from_slice(&self.gate_up_global[..4]);
            f32::from_le_bytes(b)
        } else {
            1.0
        };
        let down_global = if self.down_global.len() >= 4 {
            let mut b = [0u8; 4];
            b.copy_from_slice(&self.down_global[..4]);
            f32::from_le_bytes(b)
        } else {
            1.0
        };

        super::dispatch::ExpertWeightView {
            gate_up_packed: self.gate_up_packed,
            gate_up_scale: self.gate_up_scale,
            gate_up_global,
            down_packed: self.down_packed,
            down_scale: self.down_scale,
            down_global,
            intermediate_dim,
            emb_dim,
        }
    }
}

use std::sync::Arc;

/// Container owning memory-mapped FTW shards and tensor index.
pub struct FtwModelPackage {
    pub manifest: FtwManifest,
    pub shards: Vec<Arc<Mmap>>,
    pub tensor_index: GgufTensorIndex,
    pub expert_slices: HashMap<(u16, u16), FtwExpertBankSlice>,
    tensor_locations: HashMap<String, (usize, usize, usize)>, // name -> (shard_idx, local_off, len)
}

impl FtwModelPackage {
    /// Load an FTW model package from a directory containing `freetoken_weight.json` and `.ftw` shards.
    pub fn open_from_dir(dir: &Path) -> Result<Self, String> {
        let manifest_path = dir.join("freetoken_weight.json");
        if !manifest_path.is_file() {
            return Err(format!(
                "FTW manifest not found at {}",
                manifest_path.display()
            ));
        }
        let manifest_bytes = std::fs::read(&manifest_path)
            .map_err(|e| format!("Failed to read {}: {}", manifest_path.display(), e))?;
        let manifest: FtwManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|e| format!("Failed to parse FTW manifest: {}", e))?;

        // Open and mmap each shard
        let mut shards = Vec::with_capacity(manifest.shards.len());
        for s in &manifest.shards {
            let shard_path = dir.join(&s.file);
            let file = File::open(&shard_path)
                .map_err(|e| format!("Failed to open shard {}: {}", shard_path.display(), e))?;
            let mmap = unsafe { Mmap::map(&file) }
                .map_err(|e| format!("Failed to mmap shard {}: {}", shard_path.display(), e))?;
            shards.push(Arc::new(mmap));
        }

        // Parse config.json if available
        let config_path = dir.join("config.json");
        let parsed_cfg = if config_path.is_file() {
            std::fs::read_to_string(&config_path)
                .ok()
                .and_then(|s| ModelJsonConfig::from_json_str(&s))
        } else {
            None
        };

        let mut tensor_locations = HashMap::new();
        let mut named_tensors = Vec::new();
        let mut bank_tensors: HashMap<String, (usize, usize, usize)> = HashMap::new();

        let mut inferred_layers = 0u32;
        let mut inferred_embd = 0u32;

        for t in &manifest.tensors {
            // Find which shard contains t.global_off
            let mut shard_idx = None;
            let mut local_off = 0usize;
            for (idx, s) in manifest.shards.iter().enumerate() {
                let start = s.global_off;
                let end = start + s.nbytes;
                if t.global_off >= start && t.global_off < end {
                    shard_idx = Some(idx);
                    local_off = (t.global_off - start) as usize;
                    break;
                }
            }
            let s_idx = match shard_idx {
                Some(i) => i,
                None => continue,
            };
            let len = t.nbytes as usize;

            if t.kind == "experts_bank" {
                bank_tensors.insert(t.name.clone(), (s_idx, local_off, len));
                continue;
            }

            tensor_locations.insert(t.name.clone(), (s_idx, local_off, len));

            let ggml_type = match t.dtype.as_str() {
                "bfloat16" | "bf16" => crate::ggml_quants::GGML_TYPE_BF16,
                "float16" | "f16" => crate::ggml_quants::GGML_TYPE_F16,
                "float32" | "f32" => crate::ggml_quants::GGML_TYPE_F32,
                "uint8" => crate::ggml_quants::GGML_TYPE_Q8_0,
                _ => crate::ggml_quants::GGML_TYPE_Q8_0,
            };

            let mut dims = [0u64; 4];
            for (i, &d) in t.shape.iter().rev().enumerate().take(4) {
                dims[i] = d;
            }
            let n_dims = t.shape.len().min(4) as u32;

            let info = GgufTensorInfo {
                dims,
                n_dims,
                ggml_type,
                byte_offset: t.global_off,
            };

            // Canonicalize to standard GGUF tensor names for QTensorEngine compatibility
            if let Some(r) = name_to_role(&t.name) {
                if r.layer == crate::p64_weight::P64_LAYER_GLOBAL {
                    match r.role {
                        crate::p64_weight::P64_ROLE_TOKEN_EMBD => {
                            if t.shape.len() >= 2 {
                                inferred_embd = t.shape[1] as u32;
                            }
                            named_tensors.push((b"token_embd.weight".to_vec(), info));
                            tensor_locations
                                .insert("token_embd.weight".to_string(), (s_idx, local_off, len));
                        }
                        crate::p64_weight::P64_ROLE_OUTPUT => {
                            named_tensors.push((b"output.weight".to_vec(), info));
                            tensor_locations
                                .insert("output.weight".to_string(), (s_idx, local_off, len));
                        }
                        crate::p64_weight::P64_ROLE_OUTPUT_NORM => {
                            named_tensors.push((b"output_norm.weight".to_vec(), info));
                            tensor_locations
                                .insert("output_norm.weight".to_string(), (s_idx, local_off, len));
                        }
                        _ => {}
                    }
                } else {
                    inferred_layers = inferred_layers.max((r.layer + 1) as u32);
                    if let Some(suffix) = crate::inference::safetensor_loader::role_suffix(r.role) {
                        let mut buf = [0u8; 96];
                        let blk_len = crate::gguf_sharder::write_blk_tensor_name(
                            r.layer as u32,
                            suffix,
                            &mut buf,
                        );
                        if blk_len > 0 {
                            named_tensors.push((buf[..blk_len].to_vec(), info));
                            if let Ok(c_str) = std::str::from_utf8(&buf[..blk_len]) {
                                tensor_locations.insert(c_str.to_string(), (s_idx, local_off, len));
                            }
                        }
                    }
                }
            }

            // Always retain the original HuggingFace / FreeToken tensor name
            named_tensors.push((t.name.as_bytes().to_vec(), info));

            if t.name.ends_with("embed_tokens.weight") && t.shape.len() >= 2 {
                inferred_embd = t.shape[1] as u32;
            }
        }

        // Build expert bank slices per (layer, expert_id)
        let num_layers = manifest
            .expert_bank_num_layers
            .unwrap_or(inferred_layers)
            .max(1);
        let mut expert_slices = HashMap::new();

        for layer in 0..num_layers {
            let layer_tag = format!("#L{:05}", layer);
            let gup_key = format!("gate_up_packed{}", layer_tag);
            let gus_key = format!("gate_up_scale{}", layer_tag);
            let gug_key = format!("gate_up_global{}", layer_tag);
            let dnp_key = format!("down_packed{}", layer_tag);
            let dns_key = format!("down_scale{}", layer_tag);
            let dng_key = format!("down_global{}", layer_tag);

            if let (Some(&gup), Some(&gus), Some(&gug), Some(&dnp), Some(&dns), Some(&dng)) = (
                bank_tensors.get(&gup_key),
                bank_tensors.get(&gus_key),
                bank_tensors.get(&gug_key),
                bank_tensors.get(&dnp_key),
                bank_tensors.get(&dns_key),
                bank_tensors.get(&dng_key),
            ) {
                // 256 experts per layer
                let num_experts = 256usize;
                let gup_step = gup.2 / num_experts;
                let gus_step = gus.2 / num_experts;
                let gug_step = gug.2 / num_experts;
                let dnp_step = dnp.2 / num_experts;
                let dns_step = dns.2 / num_experts;
                let dng_step = dng.2 / num_experts;

                for e in 0..num_experts {
                    let slice = FtwExpertBankSlice {
                        shard_idx: gup.0,
                        gate_up_packed_off: gup.1 + e * gup_step,
                        gate_up_packed_len: gup_step,
                        gate_up_scale_off: gus.1 + e * gus_step,
                        gate_up_scale_len: gus_step,
                        gate_up_global_off: gug.1 + e * gug_step,
                        gate_up_global_len: gug_step,
                        down_packed_off: dnp.1 + e * dnp_step,
                        down_packed_len: dnp_step,
                        down_scale_off: dns.1 + e * dns_step,
                        down_scale_len: dns_step,
                        down_global_off: dng.1 + e * dng_step,
                        down_global_len: dng_step,
                    };
                    expert_slices.insert((layer as u16, e as u16), slice);
                }
            }
        }

        let n_layer = parsed_cfg
            .as_ref()
            .and_then(|c| c.num_hidden_layers)
            .unwrap_or(inferred_layers);
        let n_embd = parsed_cfg
            .as_ref()
            .and_then(|c| c.hidden_size)
            .unwrap_or(inferred_embd);
        let head_dim = parsed_cfg.as_ref().and_then(|c| c.head_dim).unwrap_or(64);
        let n_head = parsed_cfg
            .as_ref()
            .and_then(|c| c.num_attention_heads)
            .unwrap_or(if head_dim > 0 { n_embd / head_dim } else { 16 });
        let n_kv_head = parsed_cfg
            .as_ref()
            .and_then(|c| c.num_key_value_heads)
            .unwrap_or(n_head);
        let rope_freq_base = parsed_cfg
            .as_ref()
            .and_then(|c| c.rope_theta)
            .unwrap_or(DEFAULT_ROPE_FREQ_BASE);

        let hyperparams = GgufHyperparams {
            n_layer,
            n_embd,
            n_head,
            n_kv_head,
            rope_freq_base,
            rope_scale: 1.0,
            head_dim,
            head_dim_swa: head_dim,
            sliding_window: 0,
            shared_kv_layers: 0,
            logit_softcap: 0.0,
            ssm_conv_kernel: 0,
            ssm_state_size: 0,
            ssm_group_count: 0,
            ssm_time_step_rank: 0,
            ssm_inner_size: 0,
            full_attention_interval: 0,
            architecture: ARCH_QWEN2,
            arch_flags: 0,
        };

        let references: Vec<(&[u8], GgufTensorInfo)> = named_tensors
            .iter()
            .map(|(name, info)| (name.as_slice(), *info))
            .collect();

        let tensor_index = GgufTensorIndex::from_components(&references, hyperparams, 0);

        Ok(Self {
            manifest,
            shards,
            tensor_index,
            expert_slices,
            tensor_locations,
        })
    }

    /// Read raw bytes for a shared weight tensor by name.
    pub fn fetch_tensor_bytes(&self, name: &str) -> Option<&[u8]> {
        let &(shard_idx, off, len) = self.tensor_locations.get(name)?;
        let shard = self.shards.get(shard_idx)?;
        shard.get(off..off + len)
    }

    /// Get zero-copy byte slices for one expert in a given layer.
    pub fn get_expert_data(&self, layer: u16, expert_idx: u16) -> Option<FtwExpertData<'_>> {
        let slice = self.expert_slices.get(&(layer, expert_idx))?;
        let shard = self.shards.get(slice.shard_idx)?;

        Some(FtwExpertData {
            gate_up_packed: shard.get(
                slice.gate_up_packed_off..slice.gate_up_packed_off + slice.gate_up_packed_len,
            )?,
            gate_up_scale: shard
                .get(slice.gate_up_scale_off..slice.gate_up_scale_off + slice.gate_up_scale_len)?,
            gate_up_global: shard.get(
                slice.gate_up_global_off..slice.gate_up_global_off + slice.gate_up_global_len,
            )?,
            down_packed: shard
                .get(slice.down_packed_off..slice.down_packed_off + slice.down_packed_len)?,
            down_scale: shard
                .get(slice.down_scale_off..slice.down_scale_off + slice.down_scale_len)?,
            down_global: shard
                .get(slice.down_global_off..slice.down_global_off + slice.down_global_len)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ftw_manifest_parsing() {
        let raw = r#"{
            "format": "freetoken_weight",
            "version": 1,
            "total_bytes": 1024,
            "tensors": [
                { "name": "model.embed_tokens.weight", "kind": "weight", "dtype": "bfloat16", "shape": [10, 64], "global_off": 0, "nbytes": 1280 }
            ],
            "shards": [
                { "file": "freetoken-00000.ftw", "global_off": 0, "nbytes": 1280 }
            ]
        }"#;

        let manifest: FtwManifest = serde_json::from_str(raw).expect("manifest parsed");
        assert_eq!(manifest.format, "freetoken_weight");
        assert_eq!(manifest.tensors.len(), 1);
        assert_eq!(manifest.shards.len(), 1);
    }

    #[test]
    fn test_ftw_synthetic_package_open() {
        let temp_dir = tempfile::tempdir().unwrap();
        let ftw_path = temp_dir.path().join("freetoken-00000.ftw");
        let dummy_bytes = vec![0u8; 1280];
        std::fs::write(&ftw_path, &dummy_bytes).unwrap();

        let manifest_json = r#"{
            "format": "freetoken_weight",
            "version": 1,
            "total_bytes": 1280,
            "tensors": [
                { "name": "model.embed_tokens.weight", "kind": "weight", "dtype": "bfloat16", "shape": [10, 64], "global_off": 0, "nbytes": 1280 }
            ],
            "shards": [
                { "file": "freetoken-00000.ftw", "global_off": 0, "nbytes": 1280 }
            ]
        }"#;
        std::fs::write(temp_dir.path().join("freetoken_weight.json"), manifest_json).unwrap();

        let config_json = r#"{
            "model_type": "qwen3_5_moe",
            "text_config": {
                "hidden_size": 64,
                "num_hidden_layers": 40,
                "num_attention_heads": 8,
                "num_key_value_heads": 2
            }
        }"#;
        std::fs::write(temp_dir.path().join("config.json"), config_json).unwrap();

        let pkg = FtwModelPackage::open_from_dir(temp_dir.path()).expect("package open");
        assert_eq!(pkg.tensor_index.emb_dim(), 64);
        assert_eq!(pkg.tensor_index.hyperparams.n_layer, 40);
        assert_eq!(pkg.tensor_index.hyperparams.n_head, 8);
        assert_eq!(pkg.tensor_index.hyperparams.n_kv_head, 2);
        assert!(pkg.fetch_tensor_bytes("token_embd.weight").is_some());
        assert!(pkg
            .fetch_tensor_bytes("model.embed_tokens.weight")
            .is_some());
    }

    #[test]
    fn test_ftw_physical_qwen_package_if_present() {
        let p = Path::new(r#"E:\LLM_Models\Qwen3.6-35B-A3B-NVFP4"#);
        if !p.exists() || !p.join("freetoken_weight.json").exists() {
            return;
        }

        let pkg = FtwModelPackage::open_from_dir(p).expect("real package open");
        assert_eq!(pkg.manifest.format, "freetoken_weight");
        assert!(!pkg.shards.is_empty());
        assert!(pkg.tensor_index.emb_dim() > 0);
        
        let exp_0_0 = pkg.get_expert_data(0, 0);
        assert!(exp_0_0.is_some(), "layer 0 expert 0 data present");
        let exp_data = exp_0_0.unwrap();
        assert!(!exp_data.gate_up_packed.is_empty());
        assert!(!exp_data.down_packed.is_empty());

        let view = exp_data.to_view(pkg.tensor_index.emb_dim() as usize, 1024);
        assert!(!view.gate_up_packed.is_empty());
        assert!(!view.down_packed.is_empty());
    }
}

