use super::*;

pub(super) fn dequant_f32(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    let need = n_elems * 4;
    if raw.len() < need {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for i in 0..n_elems {
        out[i] = f32::from_le_bytes(raw[i * 4..i * 4 + 4].try_into().unwrap_or([0; 4]));
    }
    Ok(n_elems)
}

pub(super) fn dequant_f16(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    let need = n_elems * 2;
    if raw.len() < need {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for i in 0..n_elems {
        out[i] =
            half::f16::from_le_bytes(raw[i * 2..i * 2 + 2].try_into().unwrap_or([0; 2])).to_f32();
    }
    Ok(n_elems)
}

/// BF16 → f32: shift 16-bit code into the high half of an f32 bit pattern.
pub(super) fn dequant_bf16(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    let need = n_elems * 2;
    if raw.len() < need {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for i in 0..n_elems {
        let bits = u16::from_le_bytes(raw[i * 2..i * 2 + 2].try_into().unwrap_or([0; 2]));
        out[i] = f32::from_bits((bits as u32) << 16);
    }
    Ok(n_elems)
}

/// FP8 E4M3 → f32 using exact float arithmetic.
pub(super) fn dequant_fp8_e4m3(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    if raw.len() < n_elems {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for i in 0..n_elems {
        out[i] = super::fp8::fp8_e4m3_to_f32(raw[i]);
    }
    Ok(n_elems)
}
