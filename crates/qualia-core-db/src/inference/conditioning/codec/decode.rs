//! Canonical CBOR decoding for compiled plan summary.

use super::super::budget::ConditioningBudget;
use super::super::compile::CompiledPlanSummary;
use super::super::spec::ConditioningError;
use super::CODEC_VERSION;

/// Decode a CompiledPlanSummary from Tag-4201 CBOR bytes.
/// Fails closed on malformed headers, invalid version, or incomplete payloads.
pub fn decode_plan_cbor<'a>(input: &'a [u8]) -> Result<CompiledPlanSummary<'a>, ConditioningError> {
    if input.len() < 4 + 2 + 8 + 2 + 2 + 4 + 4 + 4 {
        return Err(ConditioningError::CodecError);
    }

    // Check Tag 4201: 0xd9, 0x10, 0x69 and map header: 0xa7
    if input[0] != 0xd9 || input[1] != 0x10 || input[2] != 0x69 || input[3] != 0xa7 {
        return Err(ConditioningError::CodecError);
    }

    let mut idx = 4;

    // Field 0: Version
    let version = u16::from_be_bytes([input[idx], input[idx + 1]]);
    idx += 2;
    if version != CODEC_VERSION {
        return Err(ConditioningError::UnsupportedVersion);
    }

    // Field 1: Plan ID
    let mut plan_id_bytes = [0u8; 8];
    plan_id_bytes.copy_from_slice(&input[idx..idx + 8]);
    let plan_id = u64::from_be_bytes(plan_id_bytes);
    idx += 8;

    // Field 2: Profile ID
    let p_len = u16::from_be_bytes([input[idx], input[idx + 1]]) as usize;
    idx += 2;
    if idx + p_len > input.len() {
        return Err(ConditioningError::CodecError);
    }
    let profile_id = std::str::from_utf8(&input[idx..idx + p_len])
        .map_err(|_| ConditioningError::CodecError)?;
    idx += p_len;

    // Field 3: Objective
    if idx + 2 > input.len() {
        return Err(ConditioningError::CodecError);
    }
    let o_len = u16::from_be_bytes([input[idx], input[idx + 1]]) as usize;
    idx += 2;
    if idx + o_len > input.len() {
        return Err(ConditioningError::CodecError);
    }
    let objective = std::str::from_utf8(&input[idx..idx + o_len])
        .map_err(|_| ConditioningError::CodecError)?;
    idx += o_len;

    // Field 4: Requirements count
    if idx + 4 > input.len() {
        return Err(ConditioningError::CodecError);
    }
    let req_count = u32::from_be_bytes([input[idx], input[idx + 1], input[idx + 2], input[idx + 3]])
        as usize;
    idx += 4;

    // Field 5: Outcomes count
    if idx + 4 > input.len() {
        return Err(ConditioningError::CodecError);
    }
    let outcomes_count =
        u32::from_be_bytes([input[idx], input[idx + 1], input[idx + 2], input[idx + 3]]) as usize;
    idx += 4;

    // Field 6: Evidence count
    if idx + 4 > input.len() {
        return Err(ConditioningError::CodecError);
    }
    let evidence_count =
        u32::from_be_bytes([input[idx], input[idx + 1], input[idx + 2], input[idx + 3]]) as usize;

    Ok(CompiledPlanSummary {
        plan_id,
        profile_id,
        objective,
        requirements_count: req_count,
        outcomes_count,
        evidence_count,
        budget: ConditioningBudget {
            input_tokens: 0,
            output_tokens: 0,
            tool_rounds: 0,
            max_bytes: 0,
        },
    })
}
