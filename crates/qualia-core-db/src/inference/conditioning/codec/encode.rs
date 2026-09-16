//! Canonical CBOR encoding for compiled plan summary.

use super::super::compile::CompiledPlanSummary;
use super::super::spec::ConditioningError;
use super::CODEC_VERSION;

/// Encode a CompiledPlanSummary into a caller-supplied buffer using Tag-4201 format.
/// Returns bytes written. Fails with OutputBufferFull if caller buffer is insufficient.
pub fn encode_plan_cbor<'a>(
    plan: &CompiledPlanSummary<'a>,
    out: &mut [u8],
) -> Result<usize, ConditioningError> {
    // Magic 4-byte prefix: 0xD9, 0x10, 0x69 (Tag 4201 in CBOR: 0xd9, 0x10, 0x69)
    // plus map of 7 entries: 0xA7
    let p_bytes = plan.profile_id.as_bytes();
    let o_bytes = plan.objective.as_bytes();

    let total_len = 4
        + 2 + 2 // version
        + 1 + 8 // plan_id
        + 2 + p_bytes.len() // profile_id
        + 2 + o_bytes.len() // objective
        + 1 + 4 // req_count
        + 1 + 4 // outcomes_count
        + 1 + 4; // evidence_count

    if out.len() < total_len {
        return Err(ConditioningError::OutputBufferFull);
    }

    let mut idx = 0;
    out[idx] = 0xd9;
    out[idx + 1] = 0x10;
    out[idx + 2] = 0x69; // Tag 4201
    out[idx + 3] = 0xa7; // map of 7 items
    idx += 4;

    // Field 0: Version
    out[idx..idx + 2].copy_from_slice(&CODEC_VERSION.to_be_bytes());
    idx += 2;

    // Field 1: Plan ID
    out[idx..idx + 8].copy_from_slice(&plan.plan_id.to_be_bytes());
    idx += 8;

    // Field 2: Profile ID
    let p_len = p_bytes.len() as u16;
    out[idx..idx + 2].copy_from_slice(&p_len.to_be_bytes());
    idx += 2;
    out[idx..idx + p_bytes.len()].copy_from_slice(p_bytes);
    idx += p_bytes.len();

    // Field 3: Objective
    let o_len = o_bytes.len() as u16;
    out[idx..idx + 2].copy_from_slice(&o_len.to_be_bytes());
    idx += 2;
    out[idx..idx + o_bytes.len()].copy_from_slice(o_bytes);
    idx += o_bytes.len();

    // Field 4: Requirements count
    out[idx..idx + 4].copy_from_slice(&(plan.requirements_count as u32).to_be_bytes());
    idx += 4;

    // Field 5: Outcomes count
    out[idx..idx + 4].copy_from_slice(&(plan.outcomes_count as u32).to_be_bytes());
    idx += 4;

    // Field 6: Evidence count
    out[idx..idx + 4].copy_from_slice(&(plan.evidence_count as u32).to_be_bytes());
    idx += 4;

    Ok(idx)
}
