//! Compatibility exports for the canonical Etalon numerical implementation.
pub use qualia_inference_kernel::recurrent::gated_deltanet::*;

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

    #[test]
    fn test_multistep_gated_deltanet_token_evolution_matches_oracle() {
        // d_state = 3, d_head = 2
        let d_state = 3;
        let d_head = 2;
        let mut state = vec![0.0f32; d_state * d_head];

        // Reference state matrix for oracle computation: S[s, h]
        let mut oracle_state = vec![0.0f32; d_state * d_head];

        // 4 tokens of query, key, value, alpha, beta
        let tokens_q = [
            [0.5, 0.2, 0.1],
            [0.1, 0.8, 0.3],
            [0.4, 0.4, 0.2],
            [0.9, 0.1, 0.0],
        ];
        let tokens_k = [
            [0.6, 0.1, 0.3],
            [0.2, 0.7, 0.1],
            [0.3, 0.3, 0.4],
            [0.8, 0.2, 0.0],
        ];
        let tokens_v = [[2.0, 4.0], [1.5, 3.0], [0.5, 2.5], [3.0, 1.0]];
        let alphas = [0.9f32, 0.85, 0.95, 0.8];
        let betas = [0.5f32, 0.6, 0.4, 0.7];

        let mut out = [0.0f32; 2];

        for t in 0..4 {
            let q = &tokens_q[t];
            let k = &tokens_k[t];
            let v = &tokens_v[t];
            let alpha = [alphas[t]];
            let beta = [betas[t]];

            // 1. Run actual step_gated_deltanet
            step_gated_deltanet(
                &mut state, q, k, v, &alpha, &beta, d_state, d_head, &mut out,
            )
            .unwrap();

            // 2. Run explicit mathematical oracle:
            // a) v_pred[h] = sum_s S_{t-1}[s, h] * k[s]
            let mut v_pred = vec![0.0f32; d_head];
            for h in 0..d_head {
                for s in 0..d_state {
                    v_pred[h] += oracle_state[s * d_head + h] * k[s];
                }
            }

            // b) err[h] = v[h] - v_pred[h]
            let mut err = vec![0.0f32; d_head];
            for h in 0..d_head {
                err[h] = v[h] - v_pred[h];
            }

            // c) S_t[s, h] = S_{t-1}[s, h] * alpha + beta * err[h] * k[s]
            for s in 0..d_state {
                for h in 0..d_head {
                    let idx = s * d_head + h;
                    oracle_state[idx] = oracle_state[idx] * alpha[0] + beta[0] * err[h] * k[s];
                }
            }

            // d) y[h] = sum_s S_t[s, h] * q[s]
            let mut expected_out = vec![0.0f32; d_head];
            for h in 0..d_head {
                for s in 0..d_state {
                    expected_out[h] += oracle_state[s * d_head + h] * q[s];
                }
            }

            // Verify state exact match
            for i in 0..(d_state * d_head) {
                assert!(
                    (state[i] - oracle_state[i]).abs() < 1e-6,
                    "Step {t} state mismatch at {i}: actual={}, expected={}",
                    state[i],
                    oracle_state[i]
                );
            }

            // Verify output exact match
            for h in 0..d_head {
                assert!(
                    (out[h] - expected_out[h]).abs() < 1e-6,
                    "Step {t} output mismatch at {h}: actual={}, expected={}",
                    out[h],
                    expected_out[h]
                );
            }
        }
    }

    #[test]
    fn test_multichannel_causal_conv1d_evolution_matches_oracle() {
        let channels = 2;
        let mut state = vec![0.0f32; (CAUSAL_CONV_KERNEL - 1) * channels];
        let weights = [
            0.1, 0.2, 0.3, 0.4, // channel 0
            0.5, 0.6, 0.7, 0.8, // channel 1
        ];

        let inputs = [[1.0, 2.0], [3.0, 4.0], [5.0, 6.0], [7.0, 8.0], [9.0, 10.0]];

        // Track full history for oracle verification: history[c] = Vec<f32>
        let mut full_history = vec![vec![0.0f32; 3]; channels];
        let mut out = [0.0f32; 2];

        for (t, inp) in inputs.iter().enumerate() {
            step_causal_conv1d(&mut state, &weights, inp, &mut out).unwrap();

            for c in 0..channels {
                full_history[c].push(inp[c]);
                let len = full_history[c].len();
                // 4-tap FIR: w0*h0 + w1*h1 + w2*h2 + w3*h3
                let w = &weights[c * 4..(c + 1) * 4];
                let h0 = full_history[c][len - 4];
                let h1 = full_history[c][len - 3];
                let h2 = full_history[c][len - 2];
                let h3 = full_history[c][len - 1];
                let expected = w[0] * h0 + w[1] * h1 + w[2] * h2 + w[3] * h3;

                assert!(
                    (out[c] - expected).abs() < 1e-6,
                    "Step {t} ch {c} mismatch: actual={}, expected={}",
                    out[c],
                    expected
                );
            }
        }
    }
}
