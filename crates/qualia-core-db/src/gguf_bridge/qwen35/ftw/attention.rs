//! Architecture-owned attention admission and allocation-free execution.
//! Packing, norm storage and rotary geometry arrive from the admitted exporter
//! profile; neither tensor names nor a successful matrix multiply establish them.
use super::Profile;
use crate::inference::moe::ftw_loader::FtwModelPackage;
use etalon_inference::ordinary::qwen35::{self, full_attention, Error, Host};
use etalon_model::inference::{
    qwen_attention::Qwen35AttentionRoles,
    qwen_roles::{Projection, Qwen35Roles},
    TensorInfo, WeightEncoding,
};

fn tensor(pkg: &FtwModelPackage, layer: u32, suffix: &str) -> Result<Option<TensorInfo>, String> {
    let name = format!("model.layers.{layer}.{suffix}");
    let mut matches = pkg
        .manifest
        .tensors
        .iter()
        .filter(|entry| entry.name == name);
    let Some(entry) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(format!("ambiguous Qwen tensor {name}"));
    }
    if entry.shape.is_empty() || entry.shape.len() > 4 || entry.shape.contains(&0) {
        return Err(format!("invalid Qwen tensor shape {name}"));
    }
    let ty = match entry.dtype.as_str() {
        "float32" => crate::ggml_quants::GGML_TYPE_F32,
        "float16" => crate::ggml_quants::GGML_TYPE_F16,
        "bfloat16" => crate::ggml_quants::GGML_TYPE_BF16,
        "float8_e4m3fn" => crate::ggml_quants::GGML_TYPE_FP8_E4M3,
        _ => return Err(format!("unsupported Qwen attention tensor dtype {name}")),
    };
    let mut dims = [0; 4];
    for (dst, &value) in dims.iter_mut().zip(entry.shape.iter().rev()) {
        *dst = value;
    }
    let info = TensorInfo {
        dims,
        n_dims: entry.shape.len() as u32,
        ggml_type: ty,
        byte_offset: entry.global_off,
    };
    let bytes = crate::ggml_quants::tensor_byte_len(&info).ok_or("Qwen tensor size overflow")?;
    if bytes as u64 != entry.nbytes
        || pkg
            .fetch_by_global_offset(info.byte_offset, bytes)
            .is_none()
    {
        return Err(format!("Qwen attention tensor bounds {name}"));
    }
    Ok(Some(info))
}
fn projection(
    pkg: &FtwModelPackage,
    layer: u32,
    stem: &str,
    cols: usize,
    rows: usize,
) -> Result<Projection, String> {
    let weight = tensor(pkg, layer, &format!("{stem}.weight"))?.ok_or("missing Qwen projection")?;
    let row_scale = tensor(pkg, layer, &format!("{stem}.weight_scale"))?;
    if weight.n_dims != 2 || weight.dims[..2] != [cols as u64, rows as u64] {
        return Err("Qwen attention projection shape".into());
    }
    match weight.encoding() {
        WeightEncoding::Float8E4M3 => {
            let scale = row_scale.ok_or("missing Qwen FP8 row scales")?;
            if scale.n_dims != 1
                || scale.dims[0] != rows as u64
                || !matches!(
                    scale.encoding(),
                    WeightEncoding::IeeeF32 | WeightEncoding::IeeeF16 | WeightEncoding::BrainF16
                )
            {
                return Err("Qwen FP8 row scale shape/encoding".into());
            }
        }
        WeightEncoding::IeeeF32 | WeightEncoding::IeeeF16 | WeightEncoding::BrainF16
            if row_scale.is_none() => {}
        _ => return Err("unsupported Qwen attention projection encoding".into()),
    }
    Ok(Projection { weight, row_scale })
}

#[derive(Clone, Copy, Debug)]
pub enum AttentionPlan {
    Linear(Qwen35Roles),
    Full(Qwen35AttentionRoles),
}

#[cfg(test)]
#[path = "attention_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AttentionRequirements {
    pub history: usize,
    pub recurrent: usize,
    /// Float count for each of keys and values, not their sum.
    pub kv: usize,
    pub workspace: usize,
}

pub fn resolve(
    pkg: &FtwModelPackage,
    profile: &Profile,
    layer: usize,
    is_linear: bool,
) -> Result<AttentionPlan, String> {
    let layer = u32::try_from(layer).map_err(|_| "Qwen layer index overflow")?;
    let index = &pkg.tensor_index;
    if is_linear {
        let roles = Qwen35Roles::resolve(index, layer, profile.packing)
            .map_err(|e| format!("Qwen linear layer {layer} roles: {e:?}"))?;
        if roles.geometry != profile.linear_geometry {
            return Err(format!("Qwen linear layer {layer} geometry mismatch"));
        }
        // A mixed checkpoint must not silently choose one of two layer kinds.
        if tensor(pkg, layer, "self_attn.q_proj.weight")
            .map_err(|e| format!("Qwen layer {layer} attention ambiguity: {e:?}"))?
            .is_some()
            || tensor(pkg, layer, "self_attn.qkv_proj.weight")?.is_some()
        {
            return Err(format!("Qwen linear layer {layer} also has full attention"));
        }
        Ok(AttentionPlan::Linear(roles))
    } else {
        let g = profile.attention_geometry;
        g.validate()
            .map_err(|e| format!("Qwen attention geometry: {e:?}"))?;
        let query = g
            .query_heads
            .checked_mul(g.head_dim)
            .ok_or("Qwen query overflow")?;
        let kv = g
            .kv_heads
            .checked_mul(g.head_dim)
            .ok_or("Qwen KV overflow")?;
        let qg = query.checked_mul(2).ok_or("Qwen query gate overflow")?;
        let rows = kv
            .checked_mul(2)
            .and_then(|n| n.checked_add(qg))
            .ok_or("Qwen QKV overflow")?;
        // FreeToken's mixed ModelOpt exporter concatenates Q/gate, K, V rows,
        // with Q/gate interleaved per query head. Slice the scale vector at
        // exactly the same row boundaries; never reinterpret packed columns.
        let fused = projection(pkg, layer, "self_attn.qkv_proj", g.hidden, rows)
            .map_err(|e| format!("Qwen fused attention layer {layer}: {e:?}"))?;
        let required = |name| {
            tensor(pkg, layer, name)
                .map_err(|e| format!("Qwen attention layer {layer}: {e:?}"))?
                .ok_or_else(|| format!("Qwen attention layer {layer} missing {name}"))
        };
        let roles = Qwen35AttentionRoles {
            geometry: g,
            q_gate: row_range(fused, 0, qg)?,
            k: row_range(fused, qg, kv)?,
            v: row_range(fused, qg + kv, kv)?,
            out: projection(pkg, layer, "self_attn.o_proj", query, g.hidden)
                .map_err(|e| format!("Qwen attention output layer {layer}: {e:?}"))?,
            q_norm: required("self_attn.q_norm.weight")?,
            k_norm: required("self_attn.k_norm.weight")?,
            norm: profile.ordinary_norm,
        };
        roles
            .validate()
            .map_err(|e| format!("Qwen attention layer {layer} roles: {e:?}"))?;
        for name in [
            "linear_attn.in_proj_qkvz.weight",
            "linear_attn.in_proj_qkv.weight",
            "self_attn.q_proj.weight",
            "self_attn.k_proj.weight",
            "self_attn.v_proj.weight",
            "self_attn.qkv_proj.bias",
            "self_attn.q_proj.bias",
            "self_attn.k_proj.bias",
            "self_attn.v_proj.bias",
            "self_attn.o_proj.bias",
        ] {
            if tensor(pkg, layer, name)
                .map_err(|e| format!("Qwen layer {layer} linear ambiguity: {e:?}"))?
                .is_some()
            {
                return Err(format!(
                    "Qwen full attention layer {layer} has unsupported mixed role {name}"
                ));
            }
        }
        Ok(AttentionPlan::Full(roles))
    }
}

/// Cold descriptor slicing includes its independently stored row scales.
fn row_range(mut projection: Projection, start: usize, rows: usize) -> Result<Projection, String> {
    if rows == 0
        || start
            .checked_add(rows)
            .is_none_or(|end| end > projection.rows())
    {
        return Err("Qwen fused projection row bounds".into());
    }
    let width = match projection.weight.encoding() {
        WeightEncoding::IeeeF32 => 4,
        WeightEncoding::IeeeF16 | WeightEncoding::BrainF16 => 2,
        WeightEncoding::Float8E4M3 => 1,
        _ => return Err("Qwen fused projection encoding".into()),
    };
    let offset = start
        .checked_mul(projection.cols())
        .and_then(|n| n.checked_mul(width))
        .and_then(|n| u64::try_from(n).ok())
        .ok_or("Qwen fused projection offset overflow")?;
    projection.weight.byte_offset = projection
        .weight
        .byte_offset
        .checked_add(offset)
        .ok_or("Qwen fused projection address overflow")?;
    projection.weight.dims[1] = rows as u64;
    if let Some(scale) = &mut projection.row_scale {
        // qwen35_projection validates a scalar F32/F16/BF16 scale per row.
        let width = match scale.encoding() {
            WeightEncoding::IeeeF32 => 4,
            WeightEncoding::IeeeF16 | WeightEncoding::BrainF16 => 2,
            _ => return Err("Qwen fused scale encoding".into()),
        };
        let offset = start
            .checked_mul(width)
            .and_then(|n| u64::try_from(n).ok())
            .ok_or("Qwen fused scale offset overflow")?;
        scale.byte_offset = scale
            .byte_offset
            .checked_add(offset)
            .ok_or("Qwen fused scale address overflow")?;
        scale.dims[0] = rows as u64;
    }
    Ok(projection)
}

impl AttentionPlan {
    pub fn requirements(&self, max_context: usize) -> Result<AttentionRequirements, Error> {
        match self {
            Self::Linear(roles) => {
                let l = qwen35::WorkspaceLayout::checked(roles.geometry)?;
                Ok(AttentionRequirements {
                    history: l.history,
                    recurrent: l.recurrent,
                    kv: 0,
                    workspace: l.floats,
                })
            }
            Self::Full(roles) => {
                let l = full_attention::WorkspaceLayout::checked(roles.geometry, max_context)?;
                Ok(AttentionRequirements {
                    history: 0,
                    recurrent: 0,
                    kv: l.kv.checked_mul(max_context).ok_or(Error::Overflow)?,
                    workspace: l.floats,
                })
            }
        }
    }

    /// The layer kernels publish only on success. The containing token owner
    /// must poison a partially executed multi-layer token and require reset.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        host: &impl Host,
        normalized: &[f32],
        hidden: &mut [f32],
        history: &mut [f32],
        recurrent: &mut [f32],
        keys: &mut [f32],
        values: &mut [f32],
        position: usize,
        inv_freq: &[f32],
        epsilon: f32,
        workspace: &mut [f32],
    ) -> Result<(), Error> {
        match self {
            Self::Linear(roles) => {
                if !keys.is_empty() || !values.is_empty() {
                    return Err(Error::Shape);
                }
                qwen35::execute_normalized(
                    host, roles, normalized, hidden, history, recurrent, epsilon, workspace,
                )
            }
            Self::Full(roles) => {
                if !history.is_empty() || !recurrent.is_empty() {
                    return Err(Error::Shape);
                }
                let mut cursor = position;
                full_attention::execute_normalized(
                    host,
                    roles,
                    normalized,
                    hidden,
                    keys,
                    values,
                    &mut cursor,
                    full_attention::TextRope { position, inv_freq },
                    epsilon,
                    workspace,
                )
            }
        }
    }
}
