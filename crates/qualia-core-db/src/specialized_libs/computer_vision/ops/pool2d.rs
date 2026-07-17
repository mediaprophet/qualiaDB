//! NCHW f32 pooling (CPU oracle).

use crate::specialized_libs::computer_vision::types::VisionError;

#[allow(clippy::too_many_arguments)]
pub fn max_pool2d_nchw_f32(
    input: &[f32],
    c: usize,
    h: usize,
    w: usize,
    kh: usize,
    kw: usize,
    stride_h: usize,
    stride_w: usize,
    out: &mut [f32],
) -> Result<(usize, usize), VisionError> {
    pool(input, c, h, w, kh, kw, stride_h, stride_w, out, true)
}

#[allow(clippy::too_many_arguments)]
pub fn avg_pool2d_nchw_f32(
    input: &[f32],
    c: usize,
    h: usize,
    w: usize,
    kh: usize,
    kw: usize,
    stride_h: usize,
    stride_w: usize,
    out: &mut [f32],
) -> Result<(usize, usize), VisionError> {
    pool(input, c, h, w, kh, kw, stride_h, stride_w, out, false)
}

#[allow(clippy::too_many_arguments)]
fn pool(
    input: &[f32],
    c: usize,
    h: usize,
    w: usize,
    kh: usize,
    kw: usize,
    stride_h: usize,
    stride_w: usize,
    out: &mut [f32],
    is_max: bool,
) -> Result<(usize, usize), VisionError> {
    if c == 0 || h == 0 || w == 0 || kh == 0 || kw == 0 || stride_h == 0 || stride_w == 0 {
        return Err(VisionError::MalformedImage);
    }
    let h_out = (h - kh) / stride_h + 1;
    let w_out = (w - kw) / stride_w + 1;
    if input.len() < c * h * w || out.len() < c * h_out * w_out {
        return Err(VisionError::OutputBufferTooSmall);
    }
    for ch in 0..c {
        for oh in 0..h_out {
            for ow in 0..w_out {
                let mut acc = if is_max { f32::NEG_INFINITY } else { 0.0 };
                let mut n = 0u32;
                for ky in 0..kh {
                    for kx in 0..kw {
                        let ih = oh * stride_h + ky;
                        let iw = ow * stride_w + kx;
                        let v = input[ch * h * w + ih * w + iw];
                        if is_max {
                            acc = acc.max(v);
                        } else {
                            acc += v;
                            n += 1;
                        }
                    }
                }
                if !is_max {
                    acc /= n.max(1) as f32;
                }
                out[ch * h_out * w_out + oh * w_out + ow] = acc;
            }
        }
    }
    Ok((h_out, w_out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_pool_2x2() {
        let input = [1.0f32, 2.0, 3.0, 4.0];
        let mut out = [0.0f32; 1];
        max_pool2d_nchw_f32(&input, 1, 2, 2, 2, 2, 2, 2, &mut out).unwrap();
        assert_eq!(out[0], 4.0);
    }
}
