//! Architecture-owned Qwen3.5 recurrent roles; input is already normalized.

use super::{ModelHyperparams, TensorIndex, TensorInfo, WeightEncoding, ARCH_QWEN35_MOE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qwen35RoleError {
    Architecture,
    Geometry,
    Overflow,
    MissingTensor,
    AmbiguousTensor,
    MixedProjectionProfile,
    Shape,
    UnsupportedEncoding,
    MissingRowScale,
    IncompatibleRowScale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen35Geometry {
    pub hidden: usize,
    pub key_heads: usize,
    pub value_heads: usize,
    pub key_dim: usize,
    pub value_dim: usize,
    pub kernel: usize,
    pub qk_width: usize,
    pub value_width: usize,
    pub channels: usize,
    pub history_floats: usize,
    pub state_floats: usize,
}

impl Qwen35Geometry {
    pub fn validate(&self) -> Result<(), Qwen35RoleError> {
        let cvt = |v| u32::try_from(v).map_err(|_| Qwen35RoleError::Overflow);
        let expected = Self::from_hyperparams(ModelHyperparams {
            architecture: ARCH_QWEN35_MOE,
            n_embd: cvt(self.hidden)?,
            ssm_group_count: cvt(self.key_heads)?,
            ssm_time_step_rank: cvt(self.value_heads)?,
            ssm_state_size: cvt(self.key_dim)?,
            ssm_inner_size: cvt(self.value_width)?,
            ssm_conv_kernel: cvt(self.kernel)?,
            ..Default::default()
        })?;
        if *self != expected {
            return Err(Qwen35RoleError::Geometry);
        }
        Ok(())
    }

    pub fn from_hyperparams(h: ModelHyperparams) -> Result<Self, Qwen35RoleError> {
        use Qwen35RoleError::*;
        if h.architecture != ARCH_QWEN35_MOE {
            return Err(Architecture);
        }
        let hidden = h.n_embd as usize;
        let key_heads = h.ssm_group_count as usize;
        let value_heads = h.ssm_time_step_rank as usize;
        let key_dim = h.ssm_state_size as usize;
        let value_width = h.ssm_inner_size as usize;
        let kernel = h.ssm_conv_kernel as usize;
        if [hidden, key_heads, value_heads, key_dim, value_width, kernel].contains(&0)
            || value_width % value_heads != 0
            || value_heads % key_heads != 0
        {
            return Err(Geometry);
        }
        let value_dim = value_width / value_heads;
        let qk_width = key_heads.checked_mul(key_dim).ok_or(Overflow)?;
        let channels = qk_width
            .checked_mul(2)
            .and_then(|v| v.checked_add(value_width))
            .ok_or(Overflow)?;
        let history_floats = channels.checked_mul(kernel - 1).ok_or(Overflow)?;
        let state_floats = value_width.checked_mul(key_dim).ok_or(Overflow)?;
        // Ensure the complete FP32 recurrent storage is representable in bytes.
        history_floats
            .checked_add(state_floats)
            .and_then(|v| v.checked_mul(4))
            .ok_or(Overflow)?;
        Ok(Self {
            hidden,
            key_heads,
            value_heads,
            key_dim,
            value_dim,
            kernel,
            qk_width,
            value_width,
            channels,
            history_floats,
            state_floats,
        })
    }
}

/// Fused ordering requires an independently verified exporter contract. Names and
/// dimensions do not establish that contract, including for local FTW assets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qwen35Packing {
    Separate,
    /// QKVZ global blocks and BA global blocks, in that order.
    FusedGlobal,
    /// Per key head: Q,K,V,Z; per key head: B,A (each with Hv/Hk rows).
    FusedKeyHeadGrouped,
}

pub use super::projection::Projection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qwen35Projections {
    Separate {
        qkv: Projection,
        z: Projection,
        a: Projection,
        b: Projection,
    },
    Fused {
        qkvz: Projection,
        ba: Projection,
        packing: Qwen35Packing,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen35Roles {
    pub geometry: Qwen35Geometry,
    pub projections: Qwen35Projections,
    pub conv: TensorInfo,
    pub a_log: TensorInfo,
    pub dt_bias: TensorInfo,
    /// Gated RMS norm weights multiply directly (no ordinary RMSNorm +1).
    pub norm: TensorInfo,
    pub out: Projection,
}

fn scalar_encoding(info: TensorInfo) -> bool {
    matches!(
        info.encoding(),
        WeightEncoding::IeeeF32 | WeightEncoding::IeeeF16 | WeightEncoding::BrainF16
    )
}

pub(super) fn vector(info: TensorInfo, len: usize) -> Result<TensorInfo, Qwen35RoleError> {
    if info.n_dims != 1 || info.dims[0] != len as u64 {
        return Err(Qwen35RoleError::Shape);
    }
    if !scalar_encoding(info) {
        return Err(Qwen35RoleError::UnsupportedEncoding);
    }
    Ok(info)
}

pub(super) fn projection(
    weight: TensorInfo,
    row_scale: Option<TensorInfo>,
    cols: usize,
    rows: usize,
) -> Result<Projection, Qwen35RoleError> {
    use Qwen35RoleError::*;
    if weight.n_dims != 2 || weight.dims[..2] != [cols as u64, rows as u64] {
        return Err(Shape);
    }
    cols.checked_mul(rows).ok_or(Overflow)?;
    match weight.encoding() {
        WeightEncoding::Float8E4M3 => {
            let scale = row_scale.ok_or(MissingRowScale)?;
            vector(scale, rows).map_err(|_| IncompatibleRowScale)?;
        }
        WeightEncoding::IeeeF32 | WeightEncoding::IeeeF16 | WeightEncoding::BrainF16 => {
            if row_scale.is_some() {
                return Err(IncompatibleRowScale);
            }
        }
        _ => return Err(UnsupportedEncoding),
    }
    Ok(Projection { weight, row_scale })
}

impl<H: Copy + Into<ModelHyperparams> + From<ModelHyperparams>> TensorIndex<H> {
    pub(super) fn qwen35_tensor(
        &self,
        layer: u32,
        suffix: &str,
    ) -> Result<Option<TensorInfo>, Qwen35RoleError> {
        let mut found = None;
        let mut name = [0u8; 128];
        for writer in [
            super::write_hf_layer_tensor_name,
            super::write_blk_tensor_name,
        ] {
            let n = writer(layer, suffix.as_bytes(), &mut name);
            if let Some(info) = self.tensor_info(&name[..n]) {
                if found.is_some_and(|old| old != info) {
                    return Err(Qwen35RoleError::AmbiguousTensor);
                }
                found = Some(info);
            }
            let hash = super::tensor_name_hash(&name[..n]);
            for &(_, info) in self.entries.iter().filter(|(h, _)| *h == hash) {
                if found.is_some_and(|old| old != info) {
                    return Err(Qwen35RoleError::AmbiguousTensor);
                }
                found = Some(info);
            }
        }
        Ok(found)
    }

    pub(super) fn qwen35_projection(
        &self,
        layer: u32,
        stem: &str,
        cols: usize,
        rows: usize,
    ) -> Result<Projection, Qwen35RoleError> {
        let mut name = [0u8; 96];
        let weight = append(stem, ".weight", &mut name)?;
        let weight = self
            .qwen35_tensor(layer, weight)?
            .ok_or(Qwen35RoleError::MissingTensor)?;
        let scale = append(stem, ".weight_scale", &mut name)?;
        let scale = self.qwen35_tensor(layer, scale)?;
        projection(weight, scale, cols, rows)
    }
}

fn append<'a>(stem: &str, tail: &str, out: &'a mut [u8]) -> Result<&'a str, Qwen35RoleError> {
    let len = stem.len() + tail.len();
    if len > out.len() {
        return Err(Qwen35RoleError::Overflow);
    }
    out[..stem.len()].copy_from_slice(stem.as_bytes());
    out[stem.len()..len].copy_from_slice(tail.as_bytes());
    core::str::from_utf8(&out[..len]).map_err(|_| Qwen35RoleError::MissingTensor)
}

impl Qwen35Roles {
    /// Revalidate caller-constructed descriptors before any state mutation.
    pub fn validate(&self) -> Result<(), Qwen35RoleError> {
        use Qwen35RoleError::*;
        self.geometry.validate()?;
        let g = self.geometry;
        let p =
            |p: Projection, cols, rows| projection(p.weight, p.row_scale, cols, rows).map(|_| ());
        match self.projections {
            Qwen35Projections::Separate { qkv, z, a, b } => {
                p(qkv, g.hidden, g.channels)?;
                p(z, g.hidden, g.value_width)?;
                p(a, g.hidden, g.value_heads)?;
                p(b, g.hidden, g.value_heads)?;
            }
            Qwen35Projections::Fused { qkvz, ba, packing } => {
                if packing == Qwen35Packing::Separate {
                    return Err(MixedProjectionProfile);
                }
                p(
                    qkvz,
                    g.hidden,
                    g.channels.checked_add(g.value_width).ok_or(Overflow)?,
                )?;
                p(ba, g.hidden, g.value_heads.checked_mul(2).ok_or(Overflow)?)?;
            }
        }
        if self.conv.n_dims != 3 || self.conv.dims[..3] != [g.kernel as u64, 1, g.channels as u64] {
            return Err(Shape);
        }
        if !scalar_encoding(self.conv) {
            return Err(UnsupportedEncoding);
        }
        vector(self.a_log, g.value_heads)?;
        vector(self.dt_bias, g.value_heads)?;
        vector(self.norm, g.value_dim)?;
        p(self.out, g.value_width, g.hidden)
    }

    pub fn resolve<H: Copy + Into<ModelHyperparams> + From<ModelHyperparams>>(
        index: &TensorIndex<H>,
        layer: u32,
        packing: Qwen35Packing,
    ) -> Result<Self, Qwen35RoleError> {
        use Qwen35RoleError::*;
        let geometry = Qwen35Geometry::from_hyperparams(index.hyperparams.into())?;
        let g = geometry;
        let separate = [
            "linear_attn.in_proj_qkv.weight",
            "linear_attn.in_proj_z.weight",
            "linear_attn.in_proj_a.weight",
            "linear_attn.in_proj_b.weight",
        ];
        let fused = [
            "linear_attn.in_proj_qkvz.weight",
            "linear_attn.in_proj_ba.weight",
        ];
        let other = if packing == Qwen35Packing::Separate {
            fused.as_slice()
        } else {
            separate.as_slice()
        };
        for suffix in other {
            if index.qwen35_tensor(layer, suffix)?.is_some() {
                return Err(MixedProjectionProfile);
            }
        }
        let p = |stem, cols, rows| index.qwen35_projection(layer, stem, cols, rows);
        let projections = match packing {
            Qwen35Packing::Separate => Qwen35Projections::Separate {
                qkv: p("linear_attn.in_proj_qkv", g.hidden, g.channels)?,
                z: p("linear_attn.in_proj_z", g.hidden, g.value_width)?,
                a: p("linear_attn.in_proj_a", g.hidden, g.value_heads)?,
                b: p("linear_attn.in_proj_b", g.hidden, g.value_heads)?,
            },
            _ => Qwen35Projections::Fused {
                qkvz: p(
                    "linear_attn.in_proj_qkvz",
                    g.hidden,
                    g.channels.checked_add(g.value_width).ok_or(Overflow)?,
                )?,
                ba: p(
                    "linear_attn.in_proj_ba",
                    g.hidden,
                    g.value_heads.checked_mul(2).ok_or(Overflow)?,
                )?,
                packing,
            },
        };
        let required = |suffix| index.qwen35_tensor(layer, suffix)?.ok_or(MissingTensor);
        let conv = required("linear_attn.conv1d.weight")?;
        if conv.n_dims != 3 || conv.dims[..3] != [g.kernel as u64, 1, g.channels as u64] {
            return Err(Shape);
        }
        if !scalar_encoding(conv) {
            return Err(UnsupportedEncoding);
        }
        Ok(Self {
            geometry,
            projections,
            conv,
            a_log: vector(required("linear_attn.A_log")?, g.value_heads)?,
            dt_bias: vector(required("linear_attn.dt_bias")?, g.value_heads)?,
            norm: vector(required("linear_attn.norm.weight")?, g.value_dim)?,
            out: p("linear_attn.out_proj", g.value_width, g.hidden)?,
        })
    }
}

#[cfg(test)]
#[path = "qwen_roles_tests.rs"]
mod tests;
