//! Explicit bounded Gemma4 per-layer metadata; scalar wrappers cannot encode KV8/2.
use etalon_model::gguf::{KvCursor, KvView, MetadataView, ParseLimits, VersionPolicy};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(crate) struct Profile {
    pub hidden: usize,
    pub layers: usize,
    pub heads: usize,
    pub kv_heads: Vec<usize>,
    pub sliding: Vec<bool>,
    pub head_dim: usize,
    pub swa_dim: usize,
    pub window: usize,
    pub shared: usize,
    pub ple: usize,
    pub ff: Vec<usize>,
    pub experts: usize,
    pub topk: usize,
    pub expert_ff: usize,
    pub rope_dim: usize,
    pub swa_rope_dim: usize,
    pub rope_base: f32,
    pub swa_base: f32,
    pub epsilon: f32,
    pub softcap: f32,
    pub trained_context: usize,
}
impl Profile {
    pub fn parse(bytes: &[u8]) -> Result<Option<Self>, String> {
        let limits = ParseLimits {
            max_metadata_bytes: 32 * 1024 * 1024,
            max_work: 2_000_000,
            max_nesting_depth: 4,
            versions: VersionPolicy::V2V3,
        };
        let mut found = false;
        for item in KvCursor::new(bytes, limits).map_err(|e| e.to_string())? {
            let v = item.map_err(|e| e.to_string())?;
            if v.key == b"general.architecture" {
                if v.value_type != 8 || v.encoded_value.get(8..) != Some(b"gemma4".as_slice()) {
                    return Ok(None);
                }
                found = true;
                break;
            }
        }
        if !found {
            return Ok(None);
        }
        let view = MetadataView::parse(bytes, limits).map_err(|e| e.to_string())?;
        if view.header().kv_count > 256 || view.header().tensor_count > 4096 {
            return Err("Gemma metadata count bound".into());
        }
        let mut kv = BTreeMap::new();
        for item in view.kvs() {
            let v = item.map_err(|e| e.to_string())?;
            if kv.insert(v.key, v).is_some() {
                return Err("duplicate Gemma metadata".into());
            }
        }
        let get = |suffix: &str| -> Result<KvView<'_>, String> {
            kv.get(format!("gemma4.{suffix}").as_bytes())
                .copied()
                .ok_or_else(|| format!("missing Gemma {suffix}"))
        };
        let number = |suffix: &str| -> Result<usize, String> {
            let a = numbers(get(suffix)?)?;
            if a.len() != 1 {
                return Err("Gemma expected scalar".into());
            }
            Ok(a[0])
        };
        let float = |suffix: &str| -> Result<f32, String> {
            let v = get(suffix)?;
            if v.value_type != 6 {
                return Err("Gemma expected f32".into());
            }
            let x = f32::from_le_bytes(v.encoded_value.try_into().map_err(|_| "Gemma f32")?);
            if !x.is_finite() {
                return Err("Gemma nonfinite metadata".into());
            }
            Ok(x)
        };
        let layers = number("block_count")?;
        if layers == 0 || layers > 64 {
            return Err("Gemma layer bound".into());
        }
        let per_layer = |suffix: &str| -> Result<Vec<usize>, String> {
            let v = numbers(get(suffix)?)?;
            if v.len() == 1 {
                Ok(vec![v[0]; layers])
            } else if v.len() == layers {
                Ok(v)
            } else {
                Err("Gemma per-layer metadata length".into())
            }
        };
        let heads = number("attention.head_count")?;
        let pattern = per_layer("attention.sliding_window_pattern")?;
        if pattern.iter().any(|n| *n > 1) {
            return Err("Gemma invalid sliding pattern".into());
        }
        let optional_number = |key: &str| -> Result<usize, String> {
            if kv.contains_key(format!("gemma4.{key}").as_bytes()) {
                number(key)
            } else {
                Ok(0)
            }
        };
        let p = Self {
            hidden: number("embedding_length")?,
            layers,
            heads,
            kv_heads: per_layer("attention.head_count_kv")?,
            sliding: pattern.iter().map(|n| *n == 1).collect(),
            head_dim: number("attention.key_length")?,
            swa_dim: number("attention.key_length_swa")?,
            window: number("attention.sliding_window")?,
            shared: optional_number("attention.shared_kv_layers")?,
            ple: number("embedding_length_per_layer_input")?,
            ff: per_layer("feed_forward_length")?,
            experts: optional_number("expert_count")?,
            topk: optional_number("expert_used_count")?,
            expert_ff: optional_number("expert_feed_forward_length")?,
            rope_dim: number("rope.dimension_count")?,
            swa_rope_dim: number("rope.dimension_count_swa")?,
            rope_base: float("rope.freq_base")?,
            swa_base: float("rope.freq_base_swa")?,
            epsilon: float("attention.layer_norm_rms_epsilon")?,
            softcap: float("final_logit_softcapping")?,
            trained_context: number("context_length")?,
        };
        if p.layers == 0
            || p.layers > 64
            || p.hidden == 0
            || p.hidden > 4096
            || p.heads == 0
            || p.heads > 32
            || p.head_dim == 0
            || p.head_dim > 512
            || p.swa_dim == 0
            || p.swa_dim > 512
            || p.heads * p.head_dim > 8192
            || p.heads * p.swa_dim > 8192
            || p.kv_heads
                .iter()
                .enumerate()
                .any(|(i, k)| *k == 0 || p.heads % k != 0 || k * p.dim(i) > 4096)
            || p.window == 0
            || p.window > p.trained_context
            || p.shared >= p.layers
            || p.layers - p.shared < 2
            || p.ple > 512
            || p.ple * p.layers > 16384
            || p.ff
                .iter()
                .any(|n| *n == 0 || *n > crate::gguf_bridge::MAX_STACK_GEMM_DIM)
            || p.experts > 256
            || p.topk > 16
            || (p.experts == 0) != (p.topk == 0)
            || p.topk > p.experts
            || (p.experts > 0 && (p.expert_ff == 0 || p.expert_ff > 4096))
            || p.rope_dim == 0
            || p.rope_dim > p.head_dim
            || p.rope_dim % 2 != 0
            || p.swa_rope_dim == 0
            || p.swa_rope_dim > p.swa_dim
            || p.swa_rope_dim % 2 != 0
            || p.rope_base <= 0.
            || p.swa_base <= 0.
            || p.epsilon <= 0.
            || p.softcap <= 0.
            || number("attention.value_length")? != p.head_dim
            || number("attention.value_length_swa")? != p.swa_dim
        {
            return Err("unsupported Gemma4 geometry".into());
        }
        if kv.keys().any(|k| {
            k.starts_with(b"gemma4.rope.scaling.")
                || *k == b"gemma4.rope.scale_linear".as_slice()
                || k.windows(8).any(|s| s == b"hadamard")
        }) {
            return Err("unsupported transformed Gemma4 profile".into());
        }
        Ok(Some(p))
    }
    pub fn dim(&self, layer: usize) -> usize {
        if self.sliding[layer] {
            self.swa_dim
        } else {
            self.head_dim
        }
    }
}
fn numbers(v: KvView<'_>) -> Result<Vec<usize>, String> {
    let (kind, count, raw) = if v.value_type == 9 {
        let b = v.encoded_value;
        if b.len() < 12 {
            return Err("Gemma array header".into());
        }
        (
            u32::from_le_bytes(b[..4].try_into().unwrap()),
            u64::from_le_bytes(b[4..12].try_into().unwrap()) as usize,
            &b[12..],
        )
    } else {
        (v.value_type, 1, v.encoded_value)
    };
    if count == 0 || count > 64 {
        return Err("Gemma metadata array bound".into());
    }
    let width = match kind {
        4 | 5 => 4,
        7 => 1,
        _ => return Err("Gemma metadata integer type".into()),
    };
    if raw.len() != count * width {
        return Err("Gemma metadata integer length".into());
    }
    let mut out = Vec::with_capacity(count);
    for b in raw.chunks_exact(width) {
        let n = if width == 1 {
            b[0] as u32
        } else {
            u32::from_le_bytes(b.try_into().unwrap())
        };
        if kind == 5 && n > i32::MAX as u32 {
            return Err("negative Gemma geometry".into());
        }
        out.push(n as usize);
    }
    Ok(out)
}
