//! MoE top-k router and gating logic (Work Package F9).
//!
//! Provides zero-heap top-k expert selection and routing weight normalization
//! conforming to the Qwen3.6-35B-A3B MoE architecture (256 routed experts, top-8 selection,
//! and shared expert pathways).

/// Maximum number of routed experts per token supported on the stack.
pub const MAX_MOE_TOPK: usize = 16;

/// Error conditions during MoE routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoeError {
    /// Provided output buffers are too small for requested top-k.
    BufferTooSmall,
    /// Number of gate logits is smaller than requested top-k.
    InsufficientExperts,
    /// Requested top-k exceeds compile-time stack capacity `MAX_MOE_TOPK`.
    TopKExceedsCapacity,
    /// Dimension mismatch during tensor execution.
    DimensionMismatch,
}

impl std::fmt::Display for MoeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "MoE output buffer is smaller than requested top-k"),
            Self::InsufficientExperts => write!(f, "Logit count is smaller than requested top-k"),
            Self::TopKExceedsCapacity => write!(f, "Requested top-k exceeds MAX_MOE_TOPK"),
            Self::DimensionMismatch => write!(f, "Dimension mismatch in MoE tensor execution"),
        }
    }
}

impl std::error::Error for MoeError {}

/// Configuration for MoE routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoeRoutingConfig {
    /// Total number of routed experts (e.g. 256 for Qwen3.6-35B).
    pub num_routed_experts: usize,
    /// Active experts routed per token (e.g. 8 for Qwen3.6-35B).
    pub num_experts_per_tok: usize,
    /// Whether to apply softmax normalization across the selected top-k.
    pub norm_topk_prob: bool,
    /// Number of shared experts evaluated on all tokens.
    pub num_shared_experts: usize,
}

impl MoeRoutingConfig {
    /// Canonical Qwen3.6-35B-A3B MoE configuration:
    /// 256 routed experts, top-8 selection, softmax-normalized, 1 shared expert.
    pub const fn qwen3_6_35b() -> Self {
        Self {
            num_routed_experts: 256,
            num_experts_per_tok: 8,
            norm_topk_prob: true,
            num_shared_experts: 1,
        }
    }
}

/// Zero-heap top-k expert selection and routing weight calculation.
///
/// Caller supplies `out_indices` and `out_weights` buffers of length >= `k`.
/// Does not perform heap allocations (zero-heap in hot paths per Rule 0).
pub fn route_topk(
    gate_logits: &[f32],
    k: usize,
    norm_topk_prob: bool,
    out_indices: &mut [u16],
    out_weights: &mut [f32],
) -> Result<usize, MoeError> {
    if k > MAX_MOE_TOPK {
        return Err(MoeError::TopKExceedsCapacity);
    }
    if out_indices.len() < k || out_weights.len() < k {
        return Err(MoeError::BufferTooSmall);
    }
    if gate_logits.len() < k {
        return Err(MoeError::InsufficientExperts);
    }

    // Stack array holding candidate (logit, expert_idx) sorted descending.
    let mut top: [(f32, u16); MAX_MOE_TOPK] = [(f32::NEG_INFINITY, 0); MAX_MOE_TOPK];

    for (idx, &logit) in gate_logits.iter().enumerate() {
        let idx_u16 = idx as u16;
        if logit > top[k - 1].0 {
            // Find insertion point
            let mut insert_pos = k - 1;
            while insert_pos > 0 && logit > top[insert_pos - 1].0 {
                top[insert_pos] = top[insert_pos - 1];
                insert_pos -= 1;
            }
            top[insert_pos] = (logit, idx_u16);
        }
    }

    if norm_topk_prob {
        // Softmax normalization across the top-k logits
        let mut max_val = f32::NEG_INFINITY;
        for i in 0..k {
            if top[i].0 > max_val {
                max_val = top[i].0;
            }
        }

        let mut sum_exp = 0.0f32;
        let mut exps: [f32; MAX_MOE_TOPK] = [0.0; MAX_MOE_TOPK];
        for i in 0..k {
            let exp_val = (top[i].0 - max_val).exp();
            exps[i] = exp_val;
            sum_exp += exp_val;
        }

        let inv_sum = if sum_exp > 0.0 { 1.0 / sum_exp } else { 0.0 };
        for i in 0..k {
            out_indices[i] = top[i].1;
            out_weights[i] = exps[i] * inv_sum;
        }
    } else {
        // Raw linear normalization: max(logit, 0) normalized
        let mut sum = 0.0f32;
        for i in 0..k {
            let positive = top[i].0.max(0.0);
            out_weights[i] = positive;
            sum += positive;
            out_indices[i] = top[i].1;
        }
        if sum > 0.0 {
            let inv = 1.0 / sum;
            for i in 0..k {
                out_weights[i] *= inv;
            }
        }
    }

    Ok(k)
}

/// Combine expert outputs using computed routing weights.
///
/// `expert_outputs` contains `k` slices of length `hidden_dim`.
/// `weights` contains `k` normalized routing weights.
/// Accumulates weighted sum into `out_combined`.
pub fn combine_expert_outputs(
    expert_outputs: &[&[f32]],
    weights: &[f32],
    hidden_dim: usize,
    out_combined: &mut [f32],
) -> Result<(), MoeError> {
    if out_combined.len() < hidden_dim {
        return Err(MoeError::BufferTooSmall);
    }
    if expert_outputs.len() != weights.len() {
        return Err(MoeError::DimensionMismatch);
    }

    // Zero out output buffer
    for elem in out_combined[..hidden_dim].iter_mut() {
        *elem = 0.0;
    }

    for (exp_idx, &weight) in weights.iter().enumerate() {
        if weight == 0.0 {
            continue;
        }
        let exp_out = expert_outputs[exp_idx];
        if exp_out.len() < hidden_dim {
            return Err(MoeError::DimensionMismatch);
        }
        for (out_elem, &exp_elem) in out_combined[..hidden_dim]
            .iter_mut()
            .zip(exp_out[..hidden_dim].iter())
        {
            *out_elem += weight * exp_elem;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_topk_selection_and_softmax() {
        // 10 experts, select top-3
        let logits = [1.0, 0.5, 4.0, 2.5, -1.0, 3.0, 0.0, 5.0, 1.2, 0.8];
        let mut indices = [0u16; 3];
        let mut weights = [0.0f32; 3];

        let count = route_topk(&logits, 3, true, &mut indices, &mut weights).unwrap();
        assert_eq!(count, 3);
        // Top 3 should be: index 7 (5.0), index 2 (4.0), index 5 (3.0)
        assert_eq!(indices, [7, 2, 5]);

        // Sum of weights should equal 1.0
        let sum: f32 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
        assert!(weights[0] > weights[1]);
        assert!(weights[1] > weights[2]);
    }

    #[test]
    fn test_qwen_top8_capacity() {
        let mut logits = [0.0f32; 256];
        logits[42] = 10.0;
        logits[100] = 9.0;
        logits[12] = 8.0;
        logits[200] = 7.0;
        logits[1] = 6.0;
        logits[99] = 5.0;
        logits[150] = 4.0;
        logits[255] = 3.0;

        let mut indices = [0u16; 8];
        let mut weights = [0.0f32; 8];

        let count = route_topk(&logits, 8, true, &mut indices, &mut weights).unwrap();
        assert_eq!(count, 8);
        assert_eq!(indices[0], 42);
        assert_eq!(indices[1], 100);
        assert_eq!(indices[2], 12);
        assert_eq!(indices[3], 200);
        assert_eq!(indices[4], 1);
        assert_eq!(indices[5], 99);
        assert_eq!(indices[6], 150);
        assert_eq!(indices[7], 255);
    }

    #[test]
    fn test_combine_expert_outputs() {
        let exp1 = [1.0, 2.0, 3.0];
        let exp2 = [4.0, 5.0, 6.0];
        let outputs = [&exp1[..], &exp2[..]];
        let weights = [0.5, 0.5];
        let mut combined = [0.0f32; 3];

        combine_expert_outputs(&outputs, &weights, 3, &mut combined).unwrap();
        assert_eq!(combined, [2.5, 3.5, 4.5]);
    }
}
