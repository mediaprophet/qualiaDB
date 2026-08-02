//! Direct GGML Q8_0 matrix-vector multiplication.
//!
//! The browser hot path consumes packed weights directly instead of first
//! materialising every row as f32. WASM builds use explicit SIMD128; the
//! scalar implementation is retained as a differential oracle on native CI.

const BLOCK_ELEMS: usize = 32;
const BLOCK_BYTES: usize = 34;

pub(crate) fn q8_0_gemv_into(
    raw: &[u8],
    input: &[f32],
    out: &mut [f32],
    n_in: usize,
    n_out: usize,
) -> bool {
    if n_in == 0
        || n_out == 0
        || !n_in.is_multiple_of(BLOCK_ELEMS)
        || input.len() < n_in
        || out.len() < n_out
    {
        return false;
    }
    let row_bytes = (n_in / BLOCK_ELEMS) * BLOCK_BYTES;
    let Some(required) = row_bytes.checked_mul(n_out) else {
        return false;
    };
    if raw.len() < required {
        return false;
    }

    #[cfg(target_arch = "wasm32")]
    unsafe {
        q8_0_gemv_simd(raw, input, out, n_in, n_out, row_bytes);
    }
    #[cfg(not(target_arch = "wasm32"))]
    q8_0_gemv_scalar(raw, input, out, n_in, n_out, row_bytes);
    true
}

#[cfg(not(target_arch = "wasm32"))]
fn q8_0_gemv_scalar(
    raw: &[u8],
    input: &[f32],
    out: &mut [f32],
    n_in: usize,
    n_out: usize,
    row_bytes: usize,
) {
    for (row, dst) in out[..n_out].iter_mut().enumerate() {
        let mut sum = 0.0f32;
        for block in 0..(n_in / BLOCK_ELEMS) {
            let base = row * row_bytes + block * BLOCK_BYTES;
            let scale =
                half::f16::from_bits(u16::from_le_bytes([raw[base], raw[base + 1]])).to_f32();
            for lane in 0..BLOCK_ELEMS {
                let weight = raw[base + 2 + lane] as i8 as f32;
                sum = weight.mul_add(input[block * BLOCK_ELEMS + lane] * scale, sum);
            }
        }
        *dst = sum;
    }
}

#[cfg(target_arch = "wasm32")]
unsafe fn q8_0_gemv_simd(
    raw: &[u8],
    input: &[f32],
    out: &mut [f32],
    n_in: usize,
    n_out: usize,
    row_bytes: usize,
) {
    use core::arch::wasm32::*;

    for (row, dst) in out[..n_out].iter_mut().enumerate() {
        let mut acc = f32x4_splat(0.0);
        for block in 0..(n_in / BLOCK_ELEMS) {
            let base = row * row_bytes + block * BLOCK_BYTES;
            let scale =
                half::f16::from_bits(u16::from_le_bytes([raw[base], raw[base + 1]])).to_f32();
            let scale4 = f32x4_splat(scale);
            let x_base = block * BLOCK_ELEMS;

            for half_block in 0..2 {
                let q_offset = base + 2 + half_block * 16;
                let x_offset = x_base + half_block * 16;
                let packed = unsafe { v128_load(raw.as_ptr().add(q_offset).cast()) };
                let low = i16x8_extend_low_i8x16(packed);
                let high = i16x8_extend_high_i8x16(packed);
                let q4 = [
                    i32x4_extend_low_i16x8(low),
                    i32x4_extend_high_i16x8(low),
                    i32x4_extend_low_i16x8(high),
                    i32x4_extend_high_i16x8(high),
                ];
                for (quarter, quantized) in q4.into_iter().enumerate() {
                    let x = unsafe { v128_load(input.as_ptr().add(x_offset + quarter * 4).cast()) };
                    let weights = f32x4_convert_i32x4(quantized);
                    acc = f32x4_add(acc, f32x4_mul(f32x4_mul(weights, x), scale4));
                }
            }
        }
        let mut lanes = [0.0f32; 4];
        unsafe { v128_store(lanes.as_mut_ptr().cast(), acc) };
        *dst = lanes.into_iter().sum();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_q8_matches_dequantized_rows() {
        const N_IN: usize = 64;
        const N_OUT: usize = 3;
        let mut raw = [0u8; N_OUT * 2 * BLOCK_BYTES];
        for row in 0..N_OUT {
            for block in 0..2 {
                let base = row * 2 * BLOCK_BYTES + block * BLOCK_BYTES;
                raw[base..base + 2].copy_from_slice(
                    &half::f16::from_f32(0.005 + row as f32 * 0.002)
                        .to_bits()
                        .to_le_bytes(),
                );
                for lane in 0..BLOCK_ELEMS {
                    raw[base + 2 + lane] =
                        (((row * 23 + block * 11 + lane * 5) % 255) as i16 - 127) as i8 as u8;
                }
            }
        }
        let input = core::array::from_fn::<_, N_IN, _>(|i| (i as f32 * 0.07).sin());
        let mut got = [0.0f32; N_OUT];
        assert!(q8_0_gemv_into(&raw, &input, &mut got, N_IN, N_OUT));

        let info = crate::gguf_sharder::GgufTensorInfo {
            dims: [N_IN as u64, N_OUT as u64, 1, 1],
            n_dims: 2,
            ggml_type: crate::ggml_quants::GGML_TYPE_Q8_0,
            byte_offset: 0,
        };
        let mut row = [0.0f32; N_IN];
        for output_row in 0..N_OUT {
            assert_eq!(
                crate::ggml_quants::dequant_matrix_row_into(&raw, &info, output_row, &mut row,)
                    .unwrap(),
                N_IN,
            );
            let expected = row.iter().zip(input).map(|(w, x)| w * x).sum::<f32>();
            assert!((got[output_row] - expected).abs() <= 2.0e-4);
        }
    }
}
