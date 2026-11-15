//! Mixed FTW shared/routed feed-forward execution using admitted immutable bytes.
use super::{session::SessionError, Plan};
use etalon_inference::ordinary::qwen35::Host;
use etalon_model::inference::qwen_roles::Projection;
use qualia_inference_kernel::moe::{
    dispatch::silu,
    routing::{route_topk, MAX_MOE_TOPK},
};

#[allow(clippy::too_many_arguments)]
pub(super) fn execute(
    plan: &Plan,
    index: usize,
    host: &impl Host,
    input: &[f32],
    router: &mut [f32],
    gate_up: &mut [f32],
    _gate: &mut [f32],
    _up: &mut [f32],
    swiglu: &mut [f32],
    expert: &mut [f32],
    combined: &mut [f32],
) -> Result<(), SessionError> {
    let p = &plan.profile;
    let layer = &plan.layers[index];
    router.fill(f32::NAN);
    host.gemm(
        &Projection {
            weight: layer.router,
            row_scale: None,
        },
        input,
        router,
    )?;
    super::session::finite(router)?;
    let mut ids = [0u16; MAX_MOE_TOPK];
    let mut weights = [0f32; MAX_MOE_TOPK];
    let count = route_topk(router, p.topk, true, &mut ids, &mut weights)
        .map_err(|_| SessionError::Shape)?;
    super::session::finite(&weights[..count])?;
    combined.fill(0.0);
    for i in 0..count {
        let data = plan
            .pkg
            .get_expert_data(index as u16, ids[i])
            .ok_or(SessionError::Provider)?;
        // The numerical kernel slices its admitted matrices directly. Check the
        // borrowed bank lengths before entry, never let malformed metadata panic.
        let gu = p.intermediate.checked_mul(2).ok_or(SessionError::Shape)?;
        let packed = gu.checked_mul(p.hidden / 2).ok_or(SessionError::Shape)?;
        let scales = gu.checked_mul(p.hidden / 16).ok_or(SessionError::Shape)?;
        let down_packed = p
            .hidden
            .checked_mul(p.intermediate / 2)
            .ok_or(SessionError::Shape)?;
        let down_scales = p
            .hidden
            .checked_mul(p.intermediate / 16)
            .ok_or(SessionError::Shape)?;
        if p.hidden % 16 != 0
            || p.intermediate % 16 != 0
            || data.gate_up_packed.len() != packed
            || data.gate_up_scale.len() != scales
            || data.down_packed.len() != down_packed
            || data.down_scale.len() != down_scales
            || data.gate_up_global.len() != gu * 2
            || data.down_global.len() != p.hidden * 2
        {
            return Err(SessionError::Shape);
        }
        bank_gemv(
            input,
            data.gate_up_packed,
            data.gate_up_scale,
            data.gate_up_global,
            gate_up,
        )?;
        for j in 0..p.intermediate {
            swiglu[j] = silu(gate_up[j]) * gate_up[p.intermediate + j];
        }
        super::session::finite(swiglu)?;
        expert.fill(f32::NAN);
        bank_gemv(
            swiglu,
            data.down_packed,
            data.down_scale,
            data.down_global,
            expert,
        )?;
        super::session::finite(expert)?;
        for (sum, value) in combined.iter_mut().zip(&*expert) {
            *sum += weights[i] * value;
        }
    }
    // Shared expert is gated independently; routed weights already sum to one.
    let mut shared_gate = [f32::NAN];
    host.gemm(
        &Projection {
            weight: layer.shared_gate,
            row_scale: None,
        },
        input,
        &mut shared_gate,
    )?;
    super::session::finite(&shared_gate)?;
    let shared_weight = 1.0 / (1.0 + (-shared_gate[0]).exp());
    layer
        .shared_up
        .gemv(&plan.pkg, input, gate_up)
        .map_err(|_| SessionError::Provider)?;
    for i in 0..p.intermediate {
        swiglu[i] = silu(gate_up[i]) * gate_up[p.intermediate + i];
    }
    super::session::finite(swiglu)?;
    layer
        .shared_down
        .gemv(&plan.pkg, swiglu, expert)
        .map_err(|_| SessionError::Provider)?;
    super::session::finite(expert)?;
    for (sum, value) in combined.iter_mut().zip(&*expert) {
        *sum += shared_weight * value;
    }
    super::session::finite(combined)
}

/// The FTW bank stores an independent F16 global multiplier for every row.
/// A scalar-global ExpertWeightView would silently reinterpret two F16 values
/// as one F32 and therefore cannot represent this exporter contract.
fn bank_gemv(
    input: &[f32],
    packed: &[u8],
    scales: &[u8],
    globals: &[u8],
    out: &mut [f32],
) -> Result<(), SessionError> {
    if input.len() > 16384 {
        return Err(SessionError::Shape);
    }
    super::nvfp4_rows::gemv(input, packed, scales, globals, out).map_err(|error| match error {
        super::nvfp4_rows::RowError::Shape => SessionError::Shape,
        super::nvfp4_rows::RowError::NonFinite => SessionError::Numerical,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expert_rows_use_independent_f16_global_multipliers() {
        let packed = [0x22; 16];
        let scales = [0x38; 2];
        let mut globals = [0u8; 4];
        globals[..2].copy_from_slice(&half::f16::from_f32(2.0).to_le_bytes());
        globals[2..].copy_from_slice(&half::f16::from_f32(3.0).to_le_bytes());
        let mut out = [0f32; 2];
        bank_gemv(&[1.0; 16], &packed, &scales, &globals, &mut out).unwrap();
        assert_eq!(out, [32.0, 48.0]);
        assert_eq!(
            bank_gemv(&[1.0; 16], &packed, &[0x7f, 0x38], &globals, &mut out),
            Err(SessionError::Numerical)
        );
        assert_eq!(
            bank_gemv(&[1.0; 16], &packed, &[0xff, 0x38], &globals, &mut out),
            Err(SessionError::Numerical)
        );
        assert_eq!(
            bank_gemv(&[1.0; 16], &packed, &scales, &globals[..2], &mut out),
            Err(SessionError::Shape)
        );
    }
}
