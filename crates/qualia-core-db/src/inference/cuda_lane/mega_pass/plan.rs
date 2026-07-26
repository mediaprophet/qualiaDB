//! Hot validation of cold-prepared layer handles.

use super::super::device::MultiWeightDevice;
use super::MegaPassPlanView;
use crate::ggml_quants::ggml_row_bytes;

pub(super) const MAX_MEGA_PASS_LAYERS: usize = 128;

#[derive(Clone, Copy, Default)]
pub(super) struct LayerWeightKeys {
    pub kq: u64,
    pub kk: u64,
    pub kvw: u64,
    pub ko: u64,
    pub kg: u64,
    pub ku: u64,
    pub kd: u64,
}

pub(super) fn collect_layer_keys(
    dev: &MultiWeightDevice,
    plan: &impl MegaPassPlanView,
    weight_type: u32,
    n_embd: usize,
    n_layer: usize,
    out: &mut [LayerWeightKeys; MAX_MEGA_PASS_LAYERS],
) -> Option<()> {
    for (layer_index, output) in out.iter_mut().enumerate().take(n_layer) {
        let dims = plan.layer_dims(layer_index)?;
        let weights = plan.layer_weights(layer_index)?;
        let row_in = ggml_row_bytes(weight_type, n_embd)?;
        let row_o = ggml_row_bytes(weight_type, dims.o_in)?;
        let row_gate = ggml_row_bytes(weight_type, dims.gate_in)?;
        let row_down = ggml_row_bytes(weight_type, dims.down_in)?;

        if weights.q_raw.len() < row_in.saturating_mul(dims.q_out)
            || weights.k_raw.len() < row_in.saturating_mul(dims.kv_out)
            || weights.v_raw.len() < row_in.saturating_mul(dims.kv_out)
            || weights.o_raw.len() < row_o.saturating_mul(dims.o_out)
            || weights.gate_raw.len() < row_gate.saturating_mul(dims.gate_out)
            || weights.up_raw.len() < row_gate.saturating_mul(dims.up_out)
            || weights.down_raw.len() < row_down.saturating_mul(dims.down_out)
        {
            return None;
        }

        let [kq, kk, kvw, ko, kg, ku, kd] = plan.layer_weight_keys(layer_index)?;
        let [attention_norm, ffn_norm] = plan.layer_norm_keys(layer_index)?;
        if [kq, kk, kvw, ko, kg, ku, kd, attention_norm, ffn_norm]
            .iter()
            .any(|key| !dev.weights.contains_key(key))
        {
            log::warn!("mega_pass|prepared_weight_missing|layer={layer_index}");
            return None;
        }
        *output = LayerWeightKeys {
            kq,
            kk,
            kvw,
            ko,
            kg,
            ku,
            kd,
        };
    }
    Some(())
}
