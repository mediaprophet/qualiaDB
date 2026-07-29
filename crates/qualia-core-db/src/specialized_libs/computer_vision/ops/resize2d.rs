//! NCHW f32 nearest resize (CPU oracle).

use crate::specialized_libs::computer_vision::types::VisionError;

/// Resize each channel plane independently (nearest neighbour).
pub fn resize_nearest_nchw_f32(
    input: &[f32],
    c: usize,
    h_in: usize,
    w_in: usize,
    h_out: usize,
    w_out: usize,
    out: &mut [f32],
) -> Result<(), VisionError> {
    if c == 0 || h_in == 0 || w_in == 0 || h_out == 0 || w_out == 0 {
        return Err(VisionError::MalformedImage);
    }
    if input.len() < c * h_in * w_in || out.len() < c * h_out * w_out {
        return Err(VisionError::OutputBufferTooSmall);
    }
    for ch in 0..c {
        for oy in 0..h_out {
            let iy = (oy * h_in) / h_out;
            for ox in 0..w_out {
                let ix = (ox * w_in) / w_out;
                out[ch * h_out * w_out + oy * w_out + ox] =
                    input[ch * h_in * w_in + iy * w_in + ix];
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upscale_holds_corner() {
        let input = [1.0f32, 2.0, 3.0, 4.0];
        let mut out = [0.0f32; 16];
        resize_nearest_nchw_f32(&input, 1, 2, 2, 4, 4, &mut out).unwrap();
        assert_eq!(out[0], 1.0);
        assert_eq!(out[15], 4.0);
    }
}
