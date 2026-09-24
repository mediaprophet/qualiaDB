//! Shared Mixture-of-Experts (MoE) executable operators (Wave 7: EOS-070 / EOS-071).
//!
//! Implements:
//! - EOS-070: Independent expert conversion and execution through the unified
//!   executable operator interface with zero heap in hot paths.
//! - EOS-071: Clustered shared gate/up/down operators with nonlinear state isolation
//!   and down-projection pre-accumulation per Design Plan §8.

#[allow(unused_imports)]
use qualia_inference_kernel::operators::{
    apply_q4k_lookup, q4k_lookup_workspace_floats, validate_operator, AccumKind, MatrixView,
    MatrixViewMut, OperatorDescriptor, OperatorError, OperatorKind, OperatorWorkspace, PayloadView,
    ScaleLayout, Q4K_SUPERBLOCK_ELEMS,
};

/// SiLU (Swish) activation function: x * sigmoid(x).
#[inline(always)]
pub fn silu(x: f32) -> f32 {
    x / (1.0 + (-x).exp())
}

/// A low-rank residual factor delta: delta_W * x = A * (B * x).
#[derive(Debug, Clone)]
pub struct LowRankDelta {
    pub a: Vec<f32>, // out_dim x rank (row-major)
    pub b: Vec<f32>, // rank x in_dim (row-major)
    pub in_dim: usize,
    pub out_dim: usize,
    pub rank: usize,
}

impl LowRankDelta {
    pub fn new(in_dim: usize, out_dim: usize, rank: usize, a: Vec<f32>, b: Vec<f32>) -> Self {
        assert_eq!(a.len(), out_dim * rank);
        assert_eq!(b.len(), rank * in_dim);
        Self {
            a,
            b,
            in_dim,
            out_dim,
            rank,
        }
    }

    /// Apply delta_W * x into out: out[i] += sum_r A[i, r] * (sum_j B[r, j] * x[j]).
    pub fn apply_add(&self, input: &[f32], rank_buf: &mut [f32], out: &mut [f32]) {
        assert!(rank_buf.len() >= self.rank);
        assert!(input.len() >= self.in_dim);
        assert!(out.len() >= self.out_dim);

        // B * x -> rank_buf
        for r in 0..self.rank {
            let row = &self.b[r * self.in_dim..(r + 1) * self.in_dim];
            let mut acc = 0.0f32;
            for j in 0..self.in_dim {
                acc += row[j] * input[j];
            }
            rank_buf[r] = acc;
        }

        // A * rank_buf -> add to out
        for i in 0..self.out_dim {
            let row = &self.a[i * self.rank..(i + 1) * self.rank];
            let mut acc = 0.0f32;
            for r in 0..self.rank {
                acc += row[r] * rank_buf[r];
            }
            out[i] += acc;
        }
    }
}

/// Execute a single matrix multiplication using Q4_K lookup.
pub fn gemv_q4k(
    raw_weights: &[u8],
    in_features: usize,
    out_features: usize,
    input: &[f32],
    output: &mut [f32],
    workspace_numeric: &mut [f32],
) -> Result<(), OperatorError> {
    let desc = OperatorDescriptor {
        kind: OperatorKind::Q4KBitPlane,
        in_features: in_features as u32,
        out_features: out_features as u32,
        batch_hint: 1,
        tile_elems: Q4K_SUPERBLOCK_ELEMS,
        scale_layout: ScaleLayout::GgmlQ4K,
        accum: AccumKind::F32,
        max_workspace_bytes: 0,
        representation_digest: 1,
    };
    let payloads = [PayloadView { bytes: raw_weights }];
    let op_view = validate_operator(&desc, &payloads)?;

    let in_view = MatrixView {
        data: input,
        rows: 1,
        cols: in_features,
    };
    let mut out_view = MatrixViewMut {
        data: output,
        rows: 1,
        cols: out_features,
    };
    let mut byte_scratch = [];
    let ws = OperatorWorkspace {
        numeric: workspace_numeric,
        bytes: &mut byte_scratch,
    };

    apply_q4k_lookup(
        &op_view,
        in_features,
        out_features,
        in_view,
        &mut out_view,
        ws,
    )
}

/// Independent SwiGLU expert weights (EOS-070).
pub struct IndependentExpertWeights<'a> {
    pub gate_bytes: &'a [u8],
    pub up_bytes: &'a [u8],
    pub down_bytes: &'a [u8],
    pub model_dim: usize,
    pub hidden_dim: usize,
}

impl<'a> IndependentExpertWeights<'a> {
    /// Evaluate one SwiGLU expert: y = down(silu(gate(x)) * up(x)).
    pub fn evaluate(
        &self,
        input: &[f32],
        gate_buf: &mut [f32],
        up_buf: &mut [f32],
        hidden_buf: &mut [f32],
        out: &mut [f32],
        ws_gate_up: &mut [f32],
        ws_down: &mut [f32],
    ) -> Result<(), OperatorError> {
        if input.len() < self.model_dim || out.len() < self.model_dim {
            return Err(OperatorError::WorkspaceTooSmall);
        }
        if gate_buf.len() < self.hidden_dim
            || up_buf.len() < self.hidden_dim
            || hidden_buf.len() < self.hidden_dim
        {
            return Err(OperatorError::WorkspaceTooSmall);
        }

        gemv_q4k(
            self.gate_bytes,
            self.model_dim,
            self.hidden_dim,
            input,
            gate_buf,
            ws_gate_up,
        )?;
        gemv_q4k(
            self.up_bytes,
            self.model_dim,
            self.hidden_dim,
            input,
            up_buf,
            ws_gate_up,
        )?;

        for i in 0..self.hidden_dim {
            hidden_buf[i] = silu(gate_buf[i]) * up_buf[i];
        }

        gemv_q4k(
            self.down_bytes,
            self.hidden_dim,
            self.model_dim,
            hidden_buf,
            out,
            ws_down,
        )?;
        Ok(())
    }
}

/// A cluster anchor containing shared base weights and per-expert deltas (EOS-071).
#[derive(Debug, Clone)]
pub struct ClusterAnchor {
    pub cluster_id: usize,
    pub model_dim: usize,
    pub hidden_dim: usize,
    pub base_gate: Vec<u8>,
    pub base_up: Vec<u8>,
    pub base_down: Vec<u8>,
    pub expert_ids: Vec<usize>,
    /// Optional low-rank deltas for each expert: (expert_id -> deltas)
    pub expert_gate_deltas: Vec<Option<LowRankDelta>>,
    pub expert_up_deltas: Vec<Option<LowRankDelta>>,
    pub expert_down_deltas: Vec<Option<LowRankDelta>>,
}

/// Clustered MoE operator holding cluster anchors with shared base projections and per-expert deltas (EOS-071).
#[derive(Debug, Clone)]
pub struct ClusteredMoEOperator {
    pub clusters: Vec<ClusterAnchor>,
}

impl ClusteredMoEOperator {
    pub fn new(clusters: Vec<ClusterAnchor>) -> Self {
        Self { clusters }
    }

    pub fn dispatch(
        &self,
        routed_experts: &[RoutedExpert],
        input: &[f32],
        output: &mut [f32],
        scratch: &mut ClusteredMoeScratch<'_>,
    ) -> Result<(), OperatorError> {
        dispatch_clustered_moe_step(&self.clusters, routed_experts, input, output, scratch)
    }
}


/// Routed expert execution item: (expert_index, gating_weight).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoutedExpert {
    pub expert_id: usize,
    pub weight: f32,
}

/// Scratch buffers required for zero-heap clustered MoE evaluation.
pub struct ClusteredMoeScratch<'a> {
    pub gate_buf: &'a mut [f32],
    pub up_buf: &'a mut [f32],
    pub hidden_buf: &'a mut [f32],
    pub cluster_accum_hidden: &'a mut [f32],
    pub cluster_down_out: &'a mut [f32],
    pub rank_scratch: &'a mut [f32],
    pub ws_gate_up: &'a mut [f32],
    pub ws_down: &'a mut [f32],
}

/// Execute clustered MoE dispatch step with down-projection pre-accumulation (EOS-071).
///
/// Mathematical identity: y_c = D_c (sum_{e in c} g_e h_e) + sum_{e in c} g_e delta_D_e h_e,
/// where h_e = SiLU(G_e x) * (U_e x) stays strictly expert-specific.
pub fn dispatch_clustered_moe_step(
    clusters: &[ClusterAnchor],
    routed_experts: &[RoutedExpert],
    input: &[f32],
    output: &mut [f32],
    scratch: &mut ClusteredMoeScratch<'_>,
) -> Result<(), OperatorError> {
    output.fill(0.0);

    for cluster in clusters {
        let mut cluster_has_active = false;
        scratch.cluster_accum_hidden[..cluster.hidden_dim].fill(0.0);

        for routed in routed_experts {
            let expert_pos = cluster
                .expert_ids
                .iter()
                .position(|&id| id == routed.expert_id);
            let local_idx = match expert_pos {
                Some(idx) => idx,
                None => continue, // Expert not in this cluster
            };

            cluster_has_active = true;

            // 1. Gate projection: G_e x = G_c x + delta_G_e x
            gemv_q4k(
                &cluster.base_gate,
                cluster.model_dim,
                cluster.hidden_dim,
                input,
                &mut scratch.gate_buf[..cluster.hidden_dim],
                scratch.ws_gate_up,
            )?;
            if let Some(ref delta_g) = cluster.expert_gate_deltas[local_idx] {
                delta_g.apply_add(
                    input,
                    scratch.rank_scratch,
                    &mut scratch.gate_buf[..cluster.hidden_dim],
                );
            }

            // 2. Up projection: U_e x = U_c x + delta_U_e x
            gemv_q4k(
                &cluster.base_up,
                cluster.model_dim,
                cluster.hidden_dim,
                input,
                &mut scratch.up_buf[..cluster.hidden_dim],
                scratch.ws_gate_up,
            )?;
            if let Some(ref delta_u) = cluster.expert_up_deltas[local_idx] {
                delta_u.apply_add(
                    input,
                    scratch.rank_scratch,
                    &mut scratch.up_buf[..cluster.hidden_dim],
                );
            }

            // 3. Nonlinear activation: h_e = SiLU(G_e x) * (U_e x) — strictly expert-specific!
            for i in 0..cluster.hidden_dim {
                let h_e = silu(scratch.gate_buf[i]) * scratch.up_buf[i];
                scratch.hidden_buf[i] = h_e;
                // Accumulate weighted hidden state for down projection: sum_{e in c} g_e h_e
                scratch.cluster_accum_hidden[i] += routed.weight * h_e;
            }

            // 4. Down delta projection if present: add g_e * delta_D_e * h_e directly to output
            if let Some(ref delta_d) = cluster.expert_down_deltas[local_idx] {
                // Reuse the caller-owned down buffer.  The shared down projection below
                // overwrites it after all expert-specific deltas have been accumulated.
                // Keeping this temporary in supplied scratch is essential: this function
                // is called once per decode step and must never allocate.
                scratch.cluster_down_out[..cluster.model_dim].fill(0.0);
                delta_d.apply_add(
                    &scratch.hidden_buf[..cluster.hidden_dim],
                    scratch.rank_scratch,
                    &mut scratch.cluster_down_out[..cluster.model_dim],
                );
                for m in 0..cluster.model_dim {
                    output[m] += routed.weight * scratch.cluster_down_out[m];
                }
            }
        }

        if cluster_has_active {
            // 5. Shared cluster down projection: D_c (sum_{e in c} g_e h_e) — computed ONCE per cluster!
            gemv_q4k(
                &cluster.base_down,
                cluster.hidden_dim,
                cluster.model_dim,
                &scratch.cluster_accum_hidden[..cluster.hidden_dim],
                &mut scratch.cluster_down_out[..cluster.model_dim],
                scratch.ws_down,
            )?;

            for m in 0..cluster.model_dim {
                output[m] += scratch.cluster_down_out[m];
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_synthetic_q4k_blocks(n_blocks: usize, seed: u8) -> Vec<u8> {
        let mut raw = Vec::with_capacity(n_blocks * 144);
        for b in 0..n_blocks {
            let mut blk = [0u8; 144];
            blk[0] = 0x00;
            blk[1] = 0x14; // d
            blk[2] = 0x00;
            blk[3] = 0x0c; // dmin
            for i in 4..16 {
                blk[i] = ((seed as usize + b + i) & 0x3f) as u8;
            }
            for i in 16..144 {
                blk[i] = seed.wrapping_add((b as u8).wrapping_add(i as u8));
            }
            raw.extend_from_slice(&blk);
        }
        raw
    }

    #[test]
    fn test_independent_expert_operator_swiglu() {
        let model_dim = 256;
        let hidden_dim = 256;
        let gate = make_synthetic_q4k_blocks((hidden_dim * model_dim) / 256, 11);
        let up = make_synthetic_q4k_blocks((hidden_dim * model_dim) / 256, 12);
        let down = make_synthetic_q4k_blocks((model_dim * hidden_dim) / 256, 13);

        let expert = IndependentExpertWeights {
            gate_bytes: &gate,
            up_bytes: &up,
            down_bytes: &down,
            model_dim,
            hidden_dim,
        };

        let mut input = vec![0.0f32; model_dim];
        for (i, v) in input.iter_mut().enumerate() {
            *v = (i as f32 * 0.03).cos();
        }

        let mut gate_buf = vec![0.0f32; hidden_dim];
        let mut up_buf = vec![0.0f32; hidden_dim];
        let mut hidden_buf = vec![0.0f32; hidden_dim];
        let mut out = vec![0.0f32; model_dim];

        let ws_gate_len = q4k_lookup_workspace_floats(model_dim).unwrap();
        let ws_down_len = q4k_lookup_workspace_floats(hidden_dim).unwrap();
        let mut ws_gate = vec![0.0f32; ws_gate_len];
        let mut ws_down = vec![0.0f32; ws_down_len];

        expert
            .evaluate(
                &input,
                &mut gate_buf,
                &mut up_buf,
                &mut hidden_buf,
                &mut out,
                &mut ws_gate,
                &mut ws_down,
            )
            .expect("expert eval");

        assert!(
            out.iter().any(|&x| x.abs() > 1e-5),
            "expert produced non-zero output"
        );
    }

    #[test]
    fn test_clustered_moe_mathematical_equivalence() {
        let model_dim = 256;
        let hidden_dim = 256;
        let base_gate = make_synthetic_q4k_blocks((hidden_dim * model_dim) / 256, 21);
        let base_up = make_synthetic_q4k_blocks((hidden_dim * model_dim) / 256, 22);
        let base_down = make_synthetic_q4k_blocks((model_dim * hidden_dim) / 256, 23);

        let cluster = ClusterAnchor {
            cluster_id: 0,
            model_dim,
            hidden_dim,
            base_gate: base_gate.clone(),
            base_up: base_up.clone(),
            base_down: base_down.clone(),
            expert_ids: vec![0, 1],
            expert_gate_deltas: vec![None, None],
            expert_up_deltas: vec![None, None],
            expert_down_deltas: vec![None, None],
        };

        let mut input = vec![0.0f32; model_dim];
        for (i, v) in input.iter_mut().enumerate() {
            *v = ((i % 64) as f32 * 0.05).sin();
        }

        let routed = [
            RoutedExpert {
                expert_id: 0,
                weight: 0.6,
            },
            RoutedExpert {
                expert_id: 1,
                weight: 0.4,
            },
        ];

        // 1. Independent evaluation
        let expert = IndependentExpertWeights {
            gate_bytes: &base_gate,
            up_bytes: &base_up,
            down_bytes: &base_down,
            model_dim,
            hidden_dim,
        };
        let ws_gate_len = q4k_lookup_workspace_floats(model_dim).unwrap();
        let ws_down_len = q4k_lookup_workspace_floats(hidden_dim).unwrap();
        let mut ws_gate = vec![0.0f32; ws_gate_len];
        let mut ws_down = vec![0.0f32; ws_down_len];
        let mut gb = vec![0.0f32; hidden_dim];
        let mut ub = vec![0.0f32; hidden_dim];
        let mut hb = vec![0.0f32; hidden_dim];
        let mut single_out = vec![0.0f32; model_dim];
        expert
            .evaluate(
                &input,
                &mut gb,
                &mut ub,
                &mut hb,
                &mut single_out,
                &mut ws_gate,
                &mut ws_down,
            )
            .unwrap();

        let mut expected_out = vec![0.0f32; model_dim];
        for m in 0..model_dim {
            // Both experts share base weights, so sum of weights (0.6 + 0.4 = 1.0) times single_out
            expected_out[m] = (routed[0].weight + routed[1].weight) * single_out[m];
        }

        // 2. Clustered evaluation with down-projection pre-accumulation
        let mut cluster_out = vec![0.0f32; model_dim];
        let mut c_gb = vec![0.0f32; hidden_dim];
        let mut c_ub = vec![0.0f32; hidden_dim];
        let mut c_hb = vec![0.0f32; hidden_dim];
        let mut c_accum = vec![0.0f32; hidden_dim];
        let mut c_down = vec![0.0f32; model_dim];
        let mut rank_s = vec![0.0f32; 16];
        let mut c_wsg = vec![0.0f32; ws_gate_len];
        let mut c_wsd = vec![0.0f32; ws_down_len];

        let mut scratch = ClusteredMoeScratch {
            gate_buf: &mut c_gb,
            up_buf: &mut c_ub,
            hidden_buf: &mut c_hb,
            cluster_accum_hidden: &mut c_accum,
            cluster_down_out: &mut c_down,
            rank_scratch: &mut rank_s,
            ws_gate_up: &mut c_wsg,
            ws_down: &mut c_wsd,
        };

        dispatch_clustered_moe_step(&[cluster], &routed, &input, &mut cluster_out, &mut scratch)
            .unwrap();

        for m in 0..model_dim {
            let diff = (expected_out[m] - cluster_out[m]).abs();
            let scale = expected_out[m].abs().max(cluster_out[m].abs()).max(1.0);
            let rel = diff / scale;
            assert!(
                rel < 1e-4,
                "clustered output element {m} mismatch: expected={} clustered={} rel={rel}",
                expected_out[m],
                cluster_out[m]
            );
        }
    }

    #[test]
    fn test_clustered_moe_with_deltas() {
        let model_dim = 256;
        let hidden_dim = 256;
        let base_gate = make_synthetic_q4k_blocks((hidden_dim * model_dim) / 256, 31);
        let base_up = make_synthetic_q4k_blocks((hidden_dim * model_dim) / 256, 32);
        let base_down = make_synthetic_q4k_blocks((model_dim * hidden_dim) / 256, 33);

        let delta_gate = LowRankDelta::new(
            model_dim,
            hidden_dim,
            2,
            vec![0.01f32; hidden_dim * 2],
            vec![0.02f32; 2 * model_dim],
        );
        let delta_down = LowRankDelta::new(
            hidden_dim,
            model_dim,
            2,
            vec![0.005f32; model_dim * 2],
            vec![0.005f32; 2 * hidden_dim],
        );

        let cluster = ClusterAnchor {
            cluster_id: 0,
            model_dim,
            hidden_dim,
            base_gate,
            base_up,
            base_down,
            expert_ids: vec![0],
            expert_gate_deltas: vec![Some(delta_gate)],
            expert_up_deltas: vec![None],
            expert_down_deltas: vec![Some(delta_down)],
        };

        let input = vec![0.1f32; model_dim];
        let routed = [RoutedExpert {
            expert_id: 0,
            weight: 1.0,
        }];

        let ws_gate_len = q4k_lookup_workspace_floats(model_dim).unwrap();
        let ws_down_len = q4k_lookup_workspace_floats(hidden_dim).unwrap();
        let mut out = vec![0.0f32; model_dim];
        let mut gb = vec![0.0f32; hidden_dim];
        let mut ub = vec![0.0f32; hidden_dim];
        let mut hb = vec![0.0f32; hidden_dim];
        let mut accum = vec![0.0f32; hidden_dim];
        let mut c_down = vec![0.0f32; model_dim];
        let mut rank_s = vec![0.0f32; 16];
        let mut wsg = vec![0.0f32; ws_gate_len];
        let mut wsd = vec![0.0f32; ws_down_len];

        let mut scratch = ClusteredMoeScratch {
            gate_buf: &mut gb,
            up_buf: &mut ub,
            hidden_buf: &mut hb,
            cluster_accum_hidden: &mut accum,
            cluster_down_out: &mut c_down,
            rank_scratch: &mut rank_s,
            ws_gate_up: &mut wsg,
            ws_down: &mut wsd,
        };

        dispatch_clustered_moe_step(&[cluster], &routed, &input, &mut out, &mut scratch).unwrap();
        assert!(
            out.iter().any(|&v| v.abs() > 1e-5),
            "delta-augmented clustered MoE produced non-zero output"
        );
    }

    #[test]
    fn test_clustered_moe_operator_wrapper() {
        let model_dim = 256;
        let hidden_dim = 256;
        let base_gate = make_synthetic_q4k_blocks((hidden_dim * model_dim) / 256, 41);
        let base_up = make_synthetic_q4k_blocks((hidden_dim * model_dim) / 256, 42);
        let base_down = make_synthetic_q4k_blocks((model_dim * hidden_dim) / 256, 43);

        let cluster = ClusterAnchor {
            cluster_id: 0,
            model_dim,
            hidden_dim,
            base_gate,
            base_up,
            base_down,
            expert_ids: vec![0],
            expert_gate_deltas: vec![None],
            expert_up_deltas: vec![None],
            expert_down_deltas: vec![None],
        };

        let op = ClusteredMoEOperator::new(vec![cluster]);
        let input = vec![0.2f32; model_dim];
        let routed = [RoutedExpert {
            expert_id: 0,
            weight: 0.8,
        }];

        let ws_gate_len = q4k_lookup_workspace_floats(model_dim).unwrap();
        let ws_down_len = q4k_lookup_workspace_floats(hidden_dim).unwrap();
        let mut out = vec![0.0f32; model_dim];
        let mut gb = vec![0.0f32; hidden_dim];
        let mut ub = vec![0.0f32; hidden_dim];
        let mut hb = vec![0.0f32; hidden_dim];
        let mut accum = vec![0.0f32; hidden_dim];
        let mut c_down = vec![0.0f32; model_dim];
        let mut rank_s = vec![0.0f32; 16];
        let mut wsg = vec![0.0f32; ws_gate_len];
        let mut wsd = vec![0.0f32; ws_down_len];

        let mut scratch = ClusteredMoeScratch {
            gate_buf: &mut gb,
            up_buf: &mut ub,
            hidden_buf: &mut hb,
            cluster_accum_hidden: &mut accum,
            cluster_down_out: &mut c_down,
            rank_scratch: &mut rank_s,
            ws_gate_up: &mut wsg,
            ws_down: &mut wsd,
        };

        op.dispatch(&routed, &input, &mut out, &mut scratch).unwrap();
        assert!(out.iter().any(|&v| v.abs() > 1e-5));
    }
}

