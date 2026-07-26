//! Allocation-free scalar oracle for canonical GGML Q8_0 matrix-vector multiplication.

pub const Q8_0_BLOCK_ELEMS: usize = 32;
pub const Q8_0_BLOCK_BYTES: usize = 34;

/// Decode a row-major GGML Q8_0 matrix and multiply it by `x`.
///
/// Returns `false` on an invalid shape or undersized buffer. `n_in` must be a multiple of the
/// canonical 32-element Q8_0 block width.
pub fn q8_0_gemv_oracle_into(
    n_in: usize,
    n_out: usize,
    x: &[f32],
    weight_raw: &[u8],
    out: &mut [f32],
) -> bool {
    if n_in == 0
        || n_out == 0
        || !n_in.is_multiple_of(Q8_0_BLOCK_ELEMS)
        || x.len() < n_in
        || out.len() < n_out
    {
        return false;
    }
    let row_bytes = (n_in / Q8_0_BLOCK_ELEMS) * Q8_0_BLOCK_BYTES;
    let Some(required) = row_bytes.checked_mul(n_out) else {
        return false;
    };
    if weight_raw.len() < required {
        return false;
    }
    for (row, dst) in out[..n_out].iter_mut().enumerate() {
        let mut sum = 0.0f32;
        let row_base = row * row_bytes;
        for block in 0..(n_in / Q8_0_BLOCK_ELEMS) {
            let base = row_base + block * Q8_0_BLOCK_BYTES;
            let scale =
                half::f16::from_bits(u16::from_le_bytes([weight_raw[base], weight_raw[base + 1]]))
                    .to_f32();
            for lane in 0..Q8_0_BLOCK_ELEMS {
                let q = weight_raw[base + 2 + lane] as i8 as f32;
                sum = q.mul_add(scale * x[block * Q8_0_BLOCK_ELEMS + lane], sum);
            }
        }
        *dst = sum;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oracle_decodes_multiple_rows_and_blocks() {
        let n_in = 64;
        let n_out = 3;
        let mut raw = [0u8; 3 * 2 * Q8_0_BLOCK_BYTES];
        for row in 0..n_out {
            for block in 0..2 {
                let base = row * 2 * Q8_0_BLOCK_BYTES + block * Q8_0_BLOCK_BYTES;
                raw[base..base + 2].copy_from_slice(
                    &half::f16::from_f32(0.25 * (row + 1) as f32)
                        .to_bits()
                        .to_le_bytes(),
                );
                for lane in 0..32 {
                    raw[base + 2 + lane] = ((lane as i8 % 9) - 4) as u8;
                }
            }
        }
        let x = [0.5f32; 64];
        let mut got = [0.0f32; 3];
        assert!(q8_0_gemv_oracle_into(n_in, n_out, &x, &raw, &mut got));
        assert_eq!(got, [-2.5, -5.0, -7.5]);
    }

    #[test]
    fn oracle_rejects_partial_blocks_and_short_storage() {
        assert!(!q8_0_gemv_oracle_into(
            33,
            1,
            &[0.0; 33],
            &[0; 68],
            &mut [0.0]
        ));
        assert!(!q8_0_gemv_oracle_into(
            32,
            2,
            &[0.0; 32],
            &[0; 34],
            &mut [0.0; 2]
        ));
    }

    #[test]
    #[serial_test::serial]
    fn cuda_matches_oracle_when_available() {
        use crate::inference_modes::{set_inference_mode, InferenceMode};

        if std::env::var("QUALIA_SKIP_CUDA").is_ok() {
            return;
        }
        let n_in = 256usize;
        let n_out = 17usize;
        let row_bytes = (n_in / Q8_0_BLOCK_ELEMS) * Q8_0_BLOCK_BYTES;
        let mut raw = vec![0u8; row_bytes * n_out];
        for row in 0..n_out {
            for block in 0..(n_in / Q8_0_BLOCK_ELEMS) {
                let base = row * row_bytes + block * Q8_0_BLOCK_BYTES;
                let scale = half::f16::from_f32(0.003 + row as f32 * 0.0002);
                raw[base..base + 2].copy_from_slice(&scale.to_bits().to_le_bytes());
                for lane in 0..Q8_0_BLOCK_ELEMS {
                    raw[base + 2 + lane] =
                        (((row * 13 + block * 7 + lane * 3) % 255) as i16 - 127) as i8 as u8;
                }
            }
        }
        let x: Vec<f32> = (0..n_in)
            .map(|index| ((index as f32 * 0.071).sin() * 0.75) + 0.1)
            .collect();
        let mut expected = vec![0.0f32; n_out];
        assert!(q8_0_gemv_oracle_into(n_in, n_out, &x, &raw, &mut expected));

        set_inference_mode(InferenceMode::CudaTc);
        let mut actual = vec![0.0f32; n_out];
        let executed = super::super::run::try_q8_0_cuda_gemv(n_in, n_out, &x, &raw, &mut actual);
        set_inference_mode(InferenceMode::Portable);
        if !executed {
            eprintln!("q8_0_gemv: CUDA unavailable - skipped differential");
            return;
        }
        for (row, (reference, observed)) in expected.iter().zip(&actual).enumerate() {
            let tolerance = 2.0e-3 * reference.abs().max(1.0);
            assert!(
                (reference - observed).abs() <= tolerance,
                "row {row}: oracle={reference} cuda={observed} tolerance={tolerance}"
            );
        }
    }
}
