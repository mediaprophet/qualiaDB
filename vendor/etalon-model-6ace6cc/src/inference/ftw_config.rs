use super::{ModelHyperparams, ARCH_QWEN2, ARCH_QWEN35_MOE, DEFAULT_ROPE_FREQ_BASE};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct FtwConfig {
    #[serde(flatten)]
    pub fields: FtwTextConfig,
    pub text_config: Option<FtwTextConfig>,
}

#[derive(Debug, Default, Deserialize)]
pub struct FtwTextConfig {
    pub model_type: Option<String>,
    pub num_hidden_layers: Option<u32>,
    pub hidden_size: Option<u32>,
    pub num_attention_heads: Option<u32>,
    pub num_key_value_heads: Option<u32>,
    pub head_dim: Option<u32>,
    pub rope_theta: Option<f32>,
    pub rope_parameters: Option<FtwRopeConfig>,
    pub linear_conv_kernel_dim: Option<u32>,
    pub linear_key_head_dim: Option<u32>,
    pub linear_value_head_dim: Option<u32>,
    pub linear_num_key_heads: Option<u32>,
    pub linear_num_value_heads: Option<u32>,
    pub full_attention_interval: Option<u32>,
    pub layer_types: Option<Vec<String>>,
    pub num_experts: Option<u32>,
    pub num_experts_per_tok: Option<u32>,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct FtwRopeConfig {
    pub rope_theta: Option<f32>,
}

impl FtwConfig {
    pub fn moe_routing(&self) -> Result<super::moe_routing::MoeRoutingSettings, &'static str> {
        let c = self.text_config.as_ref().unwrap_or(&self.fields);
        if self.text_config.is_some()
            && (self.fields.num_experts.is_some() && self.fields.num_experts != c.num_experts
                || self.fields.num_experts_per_tok.is_some()
                    && self.fields.num_experts_per_tok != c.num_experts_per_tok)
        {
            return Err("FTW conflicting nested routing");
        }
        let settings = super::moe_routing::MoeRoutingSettings {
            expert_count: c.num_experts,
            experts_per_token: c.num_experts_per_tok,
        };
        super::moe_metadata::validate(settings).map_err(|_| "FTW invalid routing geometry")?;
        Ok(settings)
    }

    pub fn hyperparams(
        &self,
        inferred_layers: u32,
        inferred_embd: u32,
        has_hybrid_tensors: bool,
    ) -> Result<ModelHyperparams, &'static str> {
        self.moe_routing()?;
        let c = self.text_config.as_ref().unwrap_or(&self.fields);
        if self.text_config.is_some() {
            macro_rules! check_duplicate {
                ($($field:ident),+ $(,)?) => { $(
                    if self.fields.$field.is_some() && self.fields.$field != c.$field {
                        return Err("FTW conflicting nested geometry");
                    }
                )+ };
            }
            check_duplicate!(
                hidden_size,
                num_hidden_layers,
                num_attention_heads,
                num_key_value_heads,
                head_dim,
                rope_theta,
                rope_parameters,
                linear_conv_kernel_dim,
                linear_key_head_dim,
                linear_value_head_dim,
                linear_num_key_heads,
                linear_num_value_heads,
                full_attention_interval,
                layer_types,
                num_experts,
                num_experts_per_tok
            );
        }
        let kind = c
            .model_type
            .as_deref()
            .or(self.fields.model_type.as_deref());
        let hybrid = match kind {
            Some("qwen3_5_moe" | "qwen3_5_moe_text") => true,
            None | Some("qwen" | "qwen2" | "qwen3") => false,
            _ => return Err("FTW unsupported model_type"),
        };
        if let (Some(outer), Some(inner)) = (
            self.fields.model_type.as_deref(),
            self.text_config
                .as_ref()
                .and_then(|t| t.model_type.as_deref()),
        ) {
            if outer != inner && !(outer == "qwen3_5_moe" && inner == "qwen3_5_moe_text") {
                return Err("FTW conflicting model_type declarations");
            }
        }
        if !hybrid
            && (has_hybrid_tensors
                || c.linear_conv_kernel_dim.is_some()
                || c.linear_key_head_dim.is_some()
                || c.linear_value_head_dim.is_some()
                || c.linear_num_key_heads.is_some()
                || c.linear_num_value_heads.is_some()
                || c.full_attention_interval.is_some()
                || c.layer_types.is_some())
        {
            return Err("FTW hybrid metadata requires qwen3_5_moe model_type");
        }
        let n_layer = c.num_hidden_layers.unwrap_or(inferred_layers);
        let n_embd = c.hidden_size.unwrap_or(inferred_embd);
        if (inferred_layers != 0 && n_layer != inferred_layers)
            || (inferred_embd != 0 && n_embd != inferred_embd)
        {
            return Err("FTW config disagrees with tensor geometry");
        }
        let head_dim = c.head_dim.unwrap_or(64);
        let n_head = c
            .num_attention_heads
            .unwrap_or(n_embd.checked_div(head_dim).unwrap_or(0));
        let n_kv_head = c.num_key_value_heads.unwrap_or(n_head);
        if n_embd == 0 || head_dim == 0 || n_head == 0 || n_kv_head == 0 || n_head % n_kv_head != 0
        {
            return Err("FTW invalid attention geometry");
        }
        let rope_freq_base = c
            .rope_theta
            .or(c.rope_parameters.as_ref().and_then(|r| r.rope_theta))
            .unwrap_or(DEFAULT_ROPE_FREQ_BASE);
        if !rope_freq_base.is_finite() || rope_freq_base <= 0.0 {
            return Err("FTW invalid rope_theta");
        }
        let mut h = ModelHyperparams {
            n_layer,
            n_embd,
            n_head,
            n_kv_head,
            head_dim,
            head_dim_swa: head_dim,
            rope_freq_base,
            rope_scale: 1.0,
            architecture: ARCH_QWEN2,
            ..Default::default()
        };
        if hybrid {
            let required =
                |v: Option<u32>| v.filter(|&n| n > 0).ok_or("FTW incomplete hybrid geometry");
            if n_layer == 0 {
                return Err("FTW incomplete hybrid geometry");
            }
            required(c.num_hidden_layers)?;
            required(c.hidden_size)?;
            required(c.num_attention_heads)?;
            required(c.num_key_value_heads)?;
            required(c.head_dim)?;
            h.architecture = ARCH_QWEN35_MOE;
            h.ssm_conv_kernel = required(c.linear_conv_kernel_dim)?;
            h.ssm_state_size = required(c.linear_key_head_dim)?;
            h.ssm_group_count = required(c.linear_num_key_heads)?;
            h.ssm_time_step_rank = required(c.linear_num_value_heads)?;
            let value_dim = required(c.linear_value_head_dim)?;
            h.ssm_inner_size = h
                .ssm_time_step_rank
                .checked_mul(value_dim)
                .ok_or("FTW hybrid geometry overflow")?;
            h.full_attention_interval = required(c.full_attention_interval)?;
            if h.ssm_conv_kernel != 4
                || h.ssm_state_size > 256
                || h.ssm_state_size != value_dim
                || value_dim > 256
                || h.ssm_inner_size > 8192
                || h.ssm_time_step_rank % h.ssm_group_count != 0
                || h.full_attention_interval > n_layer
            {
                return Err("FTW unsupported hybrid geometry");
            }
            if required(c.num_experts)? != 256 || required(c.num_experts_per_tok)? != 8 {
                return Err("FTW unsupported expert-bank geometry");
            }
            if let Some(types) = &c.layer_types {
                if types.len() != n_layer as usize
                    || types.iter().enumerate().any(|(i, t)| {
                        let expected = if (i + 1) % h.full_attention_interval as usize == 0 {
                            "full_attention"
                        } else {
                            "linear_attention"
                        };
                        t != expected
                    })
                {
                    return Err("FTW inconsistent hybrid layer schedule");
                }
            }
        }
        Ok(h)
    }
}
