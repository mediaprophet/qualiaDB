//! Qwen hybrid linear-attention and GatedDeltaNet recurrent state execution (Work Package F11).
//!
//! Provides zero-heap convolution and recurrent delta-rule updates for Qwen3.6-35B-A3B
//! hybrid linear-attention layers.

pub const CAUSAL_CONV_KERNEL: usize = 4;

/// Errors during Qwen recurrent state execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QwenStateError {
    BufferTooSmall,
    DimensionMismatch,
    InvalidKernelSize,
}

impl std::fmt::Display for QwenStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "Output or state buffer is too small"),
            Self::DimensionMismatch => write!(f, "Dimension mismatch in GatedDeltaNet recurrence"),
            Self::InvalidKernelSize => write!(f, "Invalid causal convolution kernel size"),
        }
    }
}

impl std::error::Error for QwenStateError {}

/// Update causal 1D depthwise convolution state and produce convolved output.
///
/// `conv_state`: History buffer of length `(CAUSAL_CONV_KERNEL - 1) * channels`.
/// `conv_weights`: Filter weights of length `CAUSAL_CONV_KERNEL * channels`.
/// `input`: Current token input features of length `channels`.
/// `out`: Output features of length `channels`.
///
/// Zero-heap: modifies `conv_state` in place and writes to caller-supplied `out`.
pub fn step_causal_conv1d(
    conv_state: &mut [f32],
    conv_weights: &[f32],
    input: &[f32],
    out: &mut [f32],
) -> Result<(), QwenStateError> {
    let channels = input.len();
    let history_len = (CAUSAL_CONV_KERNEL - 1) * channels;

    if conv_state.len() < history_len || out.len() < channels || conv_weights.len() < CAUSAL_CONV_KERNEL * channels {
        return Err(QwenStateError::BufferTooSmall);
    }

    for c in 0..channels {
        // Compute convolution dot product:
        // y[c] = W[c, 0] * history[c, 0] + W[c, 1] * history[c, 1] + W[c, 2] * history[c, 2] + W[c, 3] * input[c]
        let w_offset = c * CAUSAL_CONV_KERNEL;
        let s_offset = c * (CAUSAL_CONV_KERNEL - 1);

        let h0 = conv_state[s_offset];
        let h1 = conv_state[s_offset + 1];
        let h2 = conv_state[s_offset + 2];
        let cur = input[c];

        out[c] = conv_weights[w_offset] * h0
            + conv_weights[w_offset + 1] * h1
            + conv_weights[w_offset + 2] * h2
            + conv_weights[w_offset + 3] * cur;

        // Shift history left
        conv_state[s_offset] = h1;
        conv_state[s_offset + 1] = h2;
        conv_state[s_offset + 2] = cur;
    }

    Ok(())
}

/// Execute one step of the GatedDeltaNet recurrent state update and projection.
///
/// Mathematical formulation:
/// 1. Retrieve query projection: $v_{\text{pred}} = S_{t-1} \cdot k_t$
/// 2. Compute error / innovation: $e_t = v_t - v_{\text{pred}}$
/// 3. Update state with decay $\alpha$ and learning rate $\beta$:
///    $S_t = S_{t-1} \odot \alpha_t + \beta_t (e_t \otimes k_t)$
/// 4. Output projection: $y_t = S_t \cdot q_t$
///
/// `state`: Persistent matrix of dimensions `d_state * d_head`.
/// `q`: Query vector of length `d_state`.
/// `k`: Key vector of length `d_state`.
/// `v`: Value vector of length `d_head`.
/// `alpha`: Decay factor per head (length `d_head` or 1).
/// `beta`: Update gate per head (length `d_head` or 1).
/// `out`: Output vector of length `d_head`.
pub fn step_gated_deltanet(
    state: &mut [f32],
    q: &[f32],
    k: &[f32],
    v: &[f32],
    alpha: &[f32],
    beta: &[f32],
    d_state: usize,
    d_head: usize,
    out: &mut [f32],
) -> Result<(), QwenStateError> {
    let state_elements = d_state * d_head;
    if state.len() < state_elements || q.len() < d_state || k.len() < d_state || v.len() < d_head || out.len() < d_head {
        return Err(QwenStateError::BufferTooSmall);
    }

    let alpha_val = if !alpha.is_empty() { alpha[0] } else { 1.0 };
    let beta_val = if !beta.is_empty() { beta[0] } else { 1.0 };

    // 1. Compute predicted v: v_pred[h] = sum_s (S[s, h] * k[s])
    for h in 0..d_head {
        let mut v_pred = 0.0f32;
        for s in 0..d_state {
            v_pred += state[s * d_head + h] * k[s];
        }

        let err = v[h] - v_pred;

        // 2. Update state: S[s, h] = S[s, h] * alpha + beta * err * k[s]
        for s in 0..d_state {
            let idx = s * d_head + h;
            state[idx] = state[idx] * alpha_val + beta_val * err * k[s];
        }
    }

    // 3. Compute output: out[h] = sum_s (S[s, h] * q[s])
    for h in 0..d_head {
        let mut y = 0.0f32;
        for s in 0..d_state {
            y += state[s * d_head + h] * q[s];
        }
        out[h] = y;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_causal_conv1d() {
        // 1 channel, kernel size 4
        let mut state = [0.0f32; 3];
        let weights = [0.1, 0.2, 0.3, 0.4];
        let mut out = [0.0f32; 1];

        // Step 1: input = 10.0
        step_causal_conv1d(&mut state, &weights, &[10.0], &mut out).unwrap();
        // out = 0.4 * 10 = 4.0
        assert_eq!(out[0], 4.0);
        assert_eq!(state, [0.0, 0.0, 10.0]);

        // Step 2: input = 20.0
        step_causal_conv1d(&mut state, &weights, &[20.0], &mut out).unwrap();
        // out = 0.3 * 10.0 + 0.4 * 20.0 = 3.0 + 8.0 = 11.0
        assert_eq!(out[0], 11.0);
        assert_eq!(state, [0.0, 10.0, 20.0]);
    }

    #[test]
    fn test_step_gated_deltanet() {
        // d_state = 2, d_head = 2
        let mut state = [0.0f32; 4];
        let q = [1.0, 0.0];
        let k = [1.0, 0.0];
        let v = [5.0, 10.0];
        let alpha = [1.0];
        let beta = [1.0];
        let mut out = [0.0f32; 2];

        // Step 1: empty state, v_pred = 0, err = v
        // S[0, 0] becomes 0 + 1 * 5 * 1 = 5
        // S[0, 1] becomes 0 + 1 * 10 * 1 = 10
        // out = S * q: out[0] = 5 * 1 = 5, out[1] = 10 * 1 = 10
        step_gated_deltanet(&mut state, &q, &k, &v, &alpha, &beta, 2, 2, &mut out).unwrap();

        assert_eq!(out[0], 5.0);
        assert_eq!(out[1], 10.0);
    }
}
