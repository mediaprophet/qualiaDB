//! Exact-order borrowed NVFP4 row operations, without row dequantization scratch.
//! The lookup only replaces exact finite E4M3 decoding. Weight scaling, input
//! multiplication, and left-to-right accumulation retain the reference order.
use qualia_inference_kernel::moe::nvfp4::E2M1_TABLE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowError {
    Shape,
    NonFinite,
}

const fn scales() -> [f32; 256] {
    let mut result = [0.0; 256];
    let mut i = 0;
    while i < 256 {
        let code = i as u8;
        let exponent = (code >> 3) & 15;
        let mantissa = code & 7;
        // NaN codes are rejected before indexing; their table slots are inert.
        let absolute = if exponent == 0 {
            mantissa as f32 * (1.0 / 512.0)
        } else {
            f32::from_bits(((exponent as u32 + 120) << 23) | ((mantissa as u32) << 20))
        };
        result[i] = if code & 0x80 != 0 {
            -absolute
        } else {
            absolute
        };
        i += 1;
    }
    result
}
const SCALES: [f32; 256] = scales();

fn input_ok(input: &[f32]) -> Result<(), RowError> {
    if input.is_empty() || input.len() % 16 != 0 {
        return Err(RowError::Shape);
    }
    if input.iter().any(|x| !x.is_finite()) {
        return Err(RowError::NonFinite);
    }
    Ok(())
}

/// `input` has already been checked once for the entire matrix operation.
fn dot(input: &[f32], packed: &[u8], scales: &[u8], global: f32) -> Result<f32, RowError> {
    if packed.len() != input.len() / 2 || scales.len() != input.len() / 16 {
        return Err(RowError::Shape);
    }
    if !global.is_finite() || global <= 0.0 {
        return Err(RowError::NonFinite);
    }
    // Match Iterator::sum's floating identity, including the all-negative-zero
    // case. A +0.0 seed changes that observable IEEE-754 result.
    let mut sum: f32 = core::iter::empty::<f32>().sum();
    for (block, &code) in scales.iter().enumerate() {
        if code & 0x7f == 0x7f {
            return Err(RowError::NonFinite);
        }
        let effective = SCALES[code as usize] * global;
        if !effective.is_finite() {
            return Err(RowError::NonFinite);
        }
        let col = block * 16;
        let bytes = &packed[block * 8..(block + 1) * 8];
        for (i, &byte) in bytes.iter().enumerate() {
            let low = E2M1_TABLE[(byte & 15) as usize] * effective;
            let high = E2M1_TABLE[(byte >> 4) as usize] * effective;
            sum += input[col + 2 * i] * low;
            sum += input[col + 2 * i + 1] * high;
        }
    }
    // Non-finite products or partial sums cannot become finite again under
    // IEEE addition, so the final check also rejects intermediate overflow.
    if !sum.is_finite() {
        return Err(RowError::NonFinite);
    }
    Ok(sum)
}

pub(crate) fn row_dot(
    input: &[f32],
    packed: &[u8],
    scales: &[u8],
    global: f32,
) -> Result<f32, RowError> {
    input_ok(input)?;
    dot(input, packed, scales, global)
}

fn matrix_ok(
    input: &[f32],
    packed: &[u8],
    scales: &[u8],
    globals: &[u8],
    rows: usize,
) -> Result<(), RowError> {
    input_ok(input)?;
    if rows == 0
        || packed.len() != rows.checked_mul(input.len() / 2).ok_or(RowError::Shape)?
        || scales.len() != rows.checked_mul(input.len() / 16).ok_or(RowError::Shape)?
        || globals.len() != rows.checked_mul(2).ok_or(RowError::Shape)?
    {
        return Err(RowError::Shape);
    }
    Ok(())
}

fn matrix_row(
    input: &[f32],
    packed: &[u8],
    scales: &[u8],
    globals: &[u8],
    row: usize,
) -> Result<f32, RowError> {
    let cols = input.len();
    let global = half::f16::from_le_bytes([globals[2 * row], globals[2 * row + 1]]).to_f32();
    dot(
        input,
        &packed[row * (cols / 2)..(row + 1) * (cols / 2)],
        &scales[row * (cols / 16)..(row + 1) * (cols / 16)],
        global,
    )
}

pub(crate) fn gemv(
    input: &[f32],
    packed: &[u8],
    scales: &[u8],
    globals: &[u8],
    out: &mut [f32],
) -> Result<(), RowError> {
    matrix_ok(input, packed, scales, globals, out.len())?;
    for (row, target) in out.iter_mut().enumerate() {
        *target = matrix_row(input, packed, scales, globals, row)?;
    }
    Ok(())
}

pub(crate) fn argmax(
    input: &[f32],
    packed: &[u8],
    scales: &[u8],
    globals: &[u8],
    rows: usize,
) -> Result<(u32, f32), RowError> {
    matrix_ok(input, packed, scales, globals, rows)?;
    u32::try_from(rows).map_err(|_| RowError::Shape)?;
    let mut best = (0, f32::NEG_INFINITY);
    for row in 0..rows {
        let value = matrix_row(input, packed, scales, globals, row)?;
        if value > best.1 {
            best = (row as u32, value);
        }
    }
    Ok(best)
}

#[cfg(test)]
#[path = "nvfp4_rows_tests.rs"]
mod tests;
