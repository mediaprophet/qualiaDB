use super::*;

impl<H: From<ModelHyperparams>> TensorIndex<H> {
    pub(super) fn parse_kv_hyperparams(
        key: &str,
        vtype: u32,
        mmap: &[u8],
        pos: &mut usize,
    ) -> ModelHyperparams {
        let mut patch = ModelHyperparams::default();
        // general.architecture = STRING
        if key == "general.architecture" && vtype == 8 {
            if *pos + 8 <= mmap.len() {
                let n =
                    u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8])) as usize;
                *pos += 8;
                if *pos + n <= mmap.len() {
                    let s = std::str::from_utf8(&mmap[*pos..*pos + n]).unwrap_or("");
                    patch.architecture = parse_architecture_id(s);
                    *pos += n;
                }
            }
            return patch;
        }
        if key.ends_with("rope.freq_base") && !key.contains("swa") {
            match vtype {
                6 if *pos + 4 <= mmap.len() => {
                    let bits =
                        u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
                    *pos += 4;
                    patch.rope_freq_base = f32::from_bits(bits);
                }
                12 if *pos + 8 <= mmap.len() => {
                    let bits =
                        u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
                    *pos += 8;
                    patch.rope_freq_base = f64::from_bits(bits) as f32;
                }
                _ => {
                    let _ = gguf_skip_value(mmap, pos, vtype);
                }
            }
            return patch;
        }
        if key.ends_with("final_logit_softcapping") || key.ends_with("attention.logit_softcapping")
        {
            match vtype {
                6 if *pos + 4 <= mmap.len() => {
                    let bits =
                        u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
                    *pos += 4;
                    patch.logit_softcap = f32::from_bits(bits);
                    if patch.logit_softcap > 0.0 {
                        patch.arch_flags |= ARCH_FLAG_HAS_SOFTCAP;
                    }
                }
                12 if *pos + 8 <= mmap.len() => {
                    let bits =
                        u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
                    *pos += 8;
                    patch.logit_softcap = f64::from_bits(bits) as f32;
                    if patch.logit_softcap > 0.0 {
                        patch.arch_flags |= ARCH_FLAG_HAS_SOFTCAP;
                    }
                }
                _ => {
                    let _ = gguf_skip_value(mmap, pos, vtype);
                }
            }
            return patch;
        }
        if key.ends_with("rope.scale_linear") || key.ends_with("rope.scaling.factor") {
            match vtype {
                6 if *pos + 4 <= mmap.len() => {
                    let bits =
                        u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
                    *pos += 4;
                    patch.rope_scale = f32::from_bits(bits);
                }
                12 if *pos + 8 <= mmap.len() => {
                    let bits =
                        u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
                    *pos += 8;
                    patch.rope_scale = f64::from_bits(bits) as f32;
                }
                4 if *pos + 4 <= mmap.len() => {
                    patch.rope_scale =
                        u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]))
                            as f32;
                    *pos += 4;
                }
                _ => {
                    let _ = gguf_skip_value(mmap, pos, vtype);
                }
            }
            return patch;
        }
        if vtype != 4 {
            let _ = gguf_skip_value(mmap, pos, vtype);
            return patch;
        }
        if *pos + 4 > mmap.len() {
            return patch;
        }
        let v = u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
        *pos += 4;
        if key.ends_with("block_count") {
            patch.n_layer = v;
        } else if key.ends_with("embedding_length") && !key.contains("per_layer") {
            patch.n_embd = v;
        } else if key.ends_with("attention.head_count") && !key.contains("kv") {
            patch.n_head = v;
        } else if key.contains("head_count_kv") || key.contains("n_kv_head") {
            patch.n_kv_head = v;
        } else if key.ends_with("attention.key_length_swa")
            || key.ends_with("attention.value_length_swa")
        {
            // Prefer key_length_swa; value_length_swa is the same for Gemma 4.
            if patch.head_dim_swa == 0 {
                patch.head_dim_swa = v;
            }
            if v > 0 {
                patch.arch_flags |= ARCH_FLAG_HAS_SWA;
            }
        } else if key.ends_with("attention.key_length") || key.ends_with("attention.value_length") {
            if !key.contains("swa") && patch.head_dim == 0 {
                patch.head_dim = v;
            }
        } else if key.ends_with("attention.sliding_window") {
            patch.sliding_window = v;
            if v > 0 {
                patch.arch_flags |= ARCH_FLAG_HAS_SWA;
            }
        } else if key.ends_with("attention.shared_kv_layers") {
            patch.shared_kv_layers = v;
            if v > 0 {
                patch.arch_flags |= ARCH_FLAG_HAS_SHARED_KV;
            }
        } else if key.ends_with("ssm.conv_kernel") {
            patch.ssm_conv_kernel = v;
        } else if key.ends_with("ssm.state_size") {
            patch.ssm_state_size = v;
        } else if key.ends_with("ssm.group_count") {
            patch.ssm_group_count = v;
        } else if key.ends_with("ssm.time_step_rank") {
            patch.ssm_time_step_rank = v;
        } else if key.ends_with("ssm.inner_size") {
            patch.ssm_inner_size = v;
        } else if key.ends_with("full_attention_interval") {
            patch.full_attention_interval = v;
        }
        patch
    }

    /// Consume Qwen4Exp PLE metadata into its fixed-capacity runtime contract.
    /// Returns `true` only when this function consumed the value.
    pub(super) fn parse_ple_config_kv(
        key: &str,
        vtype: u32,
        mmap: &[u8],
        pos: &mut usize,
        config: &mut Qwen4ExpPleConfig,
    ) -> bool {
        fn scalar_u32(vtype: u32, mmap: &[u8], pos: &mut usize, field: &mut u32) -> bool {
            if vtype != 4 || *pos + 4 > mmap.len() {
                let _ = gguf_skip_value(mmap, pos, vtype);
                return false;
            }
            *field = u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
            *pos += 4;
            true
        }

        if key.ends_with("ple.ngram_size") {
            return scalar_u32(vtype, mmap, pos, &mut config.ngram_size);
        }
        if key.ends_with("ple.heads_per_ngram") {
            return scalar_u32(vtype, mmap, pos, &mut config.heads_per_ngram);
        }
        if key.ends_with("ple.eos_token_id") {
            return scalar_u32(vtype, mmap, pos, &mut config.eos_token_id);
        }
        if key.ends_with("embedding_length_per_layer_input") {
            return scalar_u32(vtype, mmap, pos, &mut config.embedding_row_width);
        }

        let target: Option<(&mut [i64], &mut u32)> = if key.ends_with("ple.layer_multipliers") {
            Some((&mut config.layer_multipliers, &mut config.multiplier_count))
        } else if key.ends_with("ple.head_offsets") {
            Some((&mut config.head_offsets, &mut config.head_count))
        } else if key.ends_with("ple.head_vocab_sizes") {
            Some((&mut config.head_vocab_sizes, &mut config.head_count))
        } else {
            None
        };
        if let Some((target, count)) = target {
            if vtype != 9 || *pos + 12 > mmap.len() {
                let _ = gguf_skip_value(mmap, pos, vtype);
                return false;
            }
            let element_type =
                u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
            *pos += 4;
            let items = u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
            *pos += 8;
            if element_type != 10
                || items > target.len() as u64
                || *pos + items as usize * 8 > mmap.len()
            {
                for _ in 0..items {
                    if gguf_skip_value(mmap, pos, element_type).is_none() {
                        break;
                    }
                }
                return true;
            }
            for slot in target.iter_mut().take(items as usize) {
                *slot = i64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
                *pos += 8;
            }
            *count = items as u32;
            return true;
        }

        if key.ends_with("ple.layers") {
            if vtype != 9 || *pos + 12 > mmap.len() {
                let _ = gguf_skip_value(mmap, pos, vtype);
                return false;
            }
            let element_type =
                u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
            *pos += 4;
            let items = u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
            *pos += 8;
            if element_type != 5
                || items > config.layers.len() as u64
                || *pos + items as usize * 4 > mmap.len()
            {
                for _ in 0..items {
                    if gguf_skip_value(mmap, pos, element_type).is_none() {
                        break;
                    }
                }
                return true;
            }
            for slot in config.layers.iter_mut().take(items as usize) {
                *slot = u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
                *pos += 4;
            }
            config.layer_count = items as u32;
            return true;
        }
        false
    }
}
