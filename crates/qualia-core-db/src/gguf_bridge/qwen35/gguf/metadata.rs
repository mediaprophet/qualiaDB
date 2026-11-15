use crate::gguf_sharder::GgufTensorIndex;
use etalon_model::gguf::{KvCursor, MetadataView, ParseLimits, VersionPolicy};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Profile {
    pub hidden: usize,
    pub ff: usize,
    pub layers: usize,
    pub interval: usize,
    pub kh: usize,
    pub vh: usize,
    pub kd: usize,
    pub vd: usize,
    pub channels: usize,
    pub value: usize,
    pub epsilon: f32,
}
impl Profile {
    pub fn parse(bytes: &[u8], index: &mut GgufTensorIndex) -> Result<Option<Self>, String> {
        let limits = ParseLimits {
            max_metadata_bytes: 32 * 1024 * 1024,
            max_work: 2_000_000,
            max_nesting_depth: 4,
            versions: VersionPolicy::V2V3,
        };
        let mut is_qwen = false;
        for item in KvCursor::new(bytes, limits).map_err(|e| e.to_string())? {
            let kv = item.map_err(|e| e.to_string())?;
            if kv.key == b"general.architecture" {
                if kv.value_type != 8 || kv.encoded_value.get(8..) != Some(b"qwen35".as_slice()) {
                    return Ok(None);
                }
                is_qwen = true;
                break;
            }
        }
        if !is_qwen {
            return Ok(None);
        }
        let view = MetadataView::parse(bytes, limits).map_err(|e| e.to_string())?;
        let mut seen = std::collections::BTreeSet::new();
        let (mut rope, mut epsilon, mut sections, mut value_dim, mut ff) =
            (None, None, false, None, None);
        for item in view.kvs() {
            let kv = item.map_err(|e| e.to_string())?;
            if !seen.insert(kv.key) {
                return Err("duplicate GGUF metadata key".into());
            }
            match kv.key {
                b"qwen35.feed_forward_length" => ff = u32_value(kv.value_type, kv.encoded_value),
                b"qwen35.rope.dimension_count" => rope = u32_value(kv.value_type, kv.encoded_value),
                b"qwen35.attention.layer_norm_rms_epsilon" if kv.value_type == 6 => {
                    epsilon = kv.encoded_value.try_into().ok().map(f32::from_le_bytes);
                }
                b"qwen35.attention.value_length" => {
                    value_dim = u32_value(kv.value_type, kv.encoded_value)
                }
                b"qwen35.rope.dimension_sections" => {
                    let v = kv.encoded_value;
                    sections = kv.value_type == 9
                        && v.len() == 28
                        && u32::from_le_bytes(v[..4].try_into().unwrap()) == 5
                        && u64::from_le_bytes(v[4..12].try_into().unwrap()) == 4
                        && v[12..] == [11i32, 11, 10, 0].map(i32::to_le_bytes).concat();
                }
                key if key.windows(8).any(|x| x == b"hadamard")
                    || key.starts_with(b"qwen35.rope.scaling.") =>
                {
                    return Err("unsupported Qwen GGUF transform/scaled RoPE profile".into());
                }
                _ => {}
            }
        }
        let h = &mut index.hyperparams;
        h.rope_dim = rope.ok_or("missing Qwen partial RoPE")?;
        h.rms_norm_eps = epsilon.ok_or("missing Qwen norm epsilon")?;
        let hidden = h.n_embd as usize;
        let kh = h.ssm_group_count as usize;
        let vh = h.ssm_time_step_rank as usize;
        let kd = h.ssm_state_size as usize;
        let value = h.ssm_inner_size as usize;
        if hidden == 0
            || hidden > 4096
            || h.n_layer == 0
            || h.n_layer > 128
            || kh == 0
            || kh > 64
            || vh == 0
            || vh > 64
            || vh % kh != 0
            || kd == 0
            || kd > 256
            || value == 0
            || value > 4096
            || value % vh != 0
            || value / vh > 256
            || h.ssm_conv_kernel != 4
            || h.full_attention_interval == 0
            || h.full_attention_interval > h.n_layer
            || h.n_layer % h.full_attention_interval != 0
            || h.n_head == 0
            || h.effective_n_kv_head() == 0
            || h.n_head % h.effective_n_kv_head() != 0
            || h.n_head as usize * h.head_dim() as usize > 4096
            || h.effective_n_kv_head() as usize * h.head_dim() as usize > 4096
            || h.head_dim() > 512
            || value_dim != Some(h.head_dim())
            || h.rope_dim != 64
            || h.rope_dim > h.head_dim()
            || !sections
            || !h.rms_norm_eps.is_finite()
            || h.rms_norm_eps <= 0.
            || !h.rope_freq_base.is_finite()
            || h.rope_freq_base <= 0.
            || h.shared_kv_layers != 0
            || h.sliding_window != 0
            || h.logit_softcap != 0.
        {
            return Err("unsupported dense Qwen3.5 GGUF geometry".into());
        }
        let channels = 2 * kh * kd + value;
        if channels > 8192 {
            return Err("Qwen convolution channel bound".into());
        }
        Ok(Some(Self {
            hidden,
            ff: ff
                .filter(|n| *n > 0 && *n <= 32768)
                .ok_or("unsupported Qwen FFN width")? as usize,
            layers: h.n_layer as usize,
            interval: h.full_attention_interval as usize,
            kh,
            vh,
            kd,
            vd: value / vh,
            channels,
            value,
            epsilon: h.rms_norm_eps,
        }))
    }
    pub fn state_stride(self) -> usize {
        3 * self.channels + self.kd * self.vd * self.vh
    }
    pub fn linear(self, layer: usize) -> bool {
        (layer + 1) % self.interval != 0
    }
}
fn u32_value(kind: u32, bytes: &[u8]) -> Option<u32> {
    (kind == 4)
        .then(|| bytes.try_into().ok().map(u32::from_le_bytes))
        .flatten()
}
