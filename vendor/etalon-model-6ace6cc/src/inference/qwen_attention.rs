//! Explicit HF Qwen3.5 gated full-attention tensor contract.
use super::{
    qwen_roles::{projection, vector, Projection, Qwen35RoleError},
    ModelHyperparams, TensorIndex, TensorInfo, ARCH_QWEN35_MOE,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen35AttentionGeometry {
    pub hidden: usize,
    pub query_heads: usize,
    pub kv_heads: usize,
    pub head_dim: usize,
    pub rotary_dim: usize,
}

#[cfg(test)]
#[path = "qwen_attention_tests.rs"]
mod tests;
impl Qwen35AttentionGeometry {
    pub fn validate(self) -> Result<(), Qwen35RoleError> {
        use Qwen35RoleError::*;
        if [
            self.hidden,
            self.query_heads,
            self.kv_heads,
            self.head_dim,
            self.rotary_dim,
        ]
        .contains(&0)
            || self.query_heads % self.kv_heads != 0
            || self.rotary_dim > self.head_dim
            || self.rotary_dim % 2 != 0
        {
            return Err(Geometry);
        }
        let q = self
            .query_heads
            .checked_mul(self.head_dim)
            .ok_or(Overflow)?;
        let k = self.kv_heads.checked_mul(self.head_dim).ok_or(Overflow)?;
        q.checked_mul(2)
            .and_then(|n| n.checked_mul(self.hidden))
            .and_then(|n| n.checked_mul(4))
            .ok_or(Overflow)?;
        k.checked_mul(self.hidden)
            .and_then(|n| n.checked_mul(4))
            .ok_or(Overflow)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QkNorm {
    RawZeroCentered,
    FoldedMultiplier,
}

/// q_gate rows are per head [Q[head_dim], gate[head_dim]], matching the
/// pinned HF view(..., heads, 2*head_dim).chunk(2, -1). No biases are admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Qwen35AttentionRoles {
    pub geometry: Qwen35AttentionGeometry,
    pub q_gate: Projection,
    pub k: Projection,
    pub v: Projection,
    pub out: Projection,
    pub q_norm: TensorInfo,
    pub k_norm: TensorInfo,
    pub norm: QkNorm,
}
impl Qwen35AttentionRoles {
    pub fn validate(&self) -> Result<(), Qwen35RoleError> {
        self.geometry.validate()?;
        let g = self.geometry;
        let q = g.query_heads * g.head_dim;
        let k = g.kv_heads * g.head_dim;
        for (p, cols, rows) in [
            (self.q_gate, g.hidden, 2 * q),
            (self.k, g.hidden, k),
            (self.v, g.hidden, k),
            (self.out, q, g.hidden),
        ] {
            projection(p.weight, p.row_scale, cols, rows)?;
        }
        vector(self.q_norm, g.head_dim)?;
        vector(self.k_norm, g.head_dim)?;
        Ok(())
    }

    /// Geometry and norm convention must come from an independently admitted
    /// source profile; tensor names alone cannot establish either convention.
    pub fn resolve<H: Copy + Into<ModelHyperparams> + From<ModelHyperparams>>(
        index: &TensorIndex<H>,
        layer: u32,
        geometry: Qwen35AttentionGeometry,
        norm: QkNorm,
    ) -> Result<Self, Qwen35RoleError> {
        use Qwen35RoleError::*;
        let h: ModelHyperparams = index.hyperparams.into();
        if h.architecture != ARCH_QWEN35_MOE {
            return Err(Architecture);
        }
        geometry.validate()?;
        let g = geometry;
        if layer >= h.n_layer
            || h.n_embd as usize != g.hidden
            || h.n_head == 0
            || h.n_head as usize != g.query_heads
            || h.effective_n_kv_head() as usize != g.kv_heads
            || h.effective_head_dim() as usize != g.head_dim
        {
            return Err(Geometry);
        }
        for name in [
            "self_attn.q_proj.bias",
            "self_attn.k_proj.bias",
            "self_attn.v_proj.bias",
            "self_attn.o_proj.bias",
        ] {
            if index.qwen35_tensor(layer, name)?.is_some() {
                return Err(MixedProjectionProfile);
            }
        }
        let p = |name, cols, rows| index.qwen35_projection(layer, name, cols, rows);
        let required = |name| index.qwen35_tensor(layer, name)?.ok_or(MissingTensor);
        let roles = Self {
            geometry,
            norm,
            q_gate: p("self_attn.q_proj", g.hidden, 2 * g.query_heads * g.head_dim)?,
            k: p("self_attn.k_proj", g.hidden, g.kv_heads * g.head_dim)?,
            v: p("self_attn.v_proj", g.hidden, g.kv_heads * g.head_dim)?,
            out: p("self_attn.o_proj", g.query_heads * g.head_dim, g.hidden)?,
            q_norm: required("self_attn.q_norm.weight")?,
            k_norm: required("self_attn.k_norm.weight")?,
        };
        roles.validate()?;
        Ok(roles)
    }
}
