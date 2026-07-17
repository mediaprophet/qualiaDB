//! NCHW f32 Conv2D (CPU oracle).

use crate::types::VisionError;

/// 2D convolution, NCHW layout, single batch (N=1).
///
/// - `input`: `[C_in * H * W]`
/// - `weight`: `[C_out * C_in * KH * KW]` (no groups)
/// - `bias`: optional `[C_out]` or empty
/// - `output`: `[C_out * H_out * W_out]`
#[allow(clippy::too_many_arguments)]
pub fn conv2d_nchw_f32(
    input: &[f32],
    c_in: usize,
    h: usize,
    w: usize,
    weight: &[f32],
    c_out: usize,
    kh: usize,
    kw: usize,
    bias: &[f32],
    stride_h: usize,
    stride_w: usize,
    pad_h: usize,
    pad_w: usize,
    out: &mut [f32],
) -> Result<(usize, usize), VisionError> {
    if c_in == 0 || c_out == 0 || h == 0 || w == 0 || kh == 0 || kw == 0 {
        return Err(VisionError::MalformedImage);
    }
    if stride_h == 0 || stride_w == 0 {
        return Err(VisionError::MalformedImage);
    }
    let h_out = (h + 2 * pad_h - kh) / stride_h + 1;
    let w_out = (w + 2 * pad_w - kw) / stride_w + 1;
    if input.len() < c_in * h * w {
        return Err(VisionError::MalformedImage);
    }
    if weight.len() < c_out * c_in * kh * kw {
        return Err(VisionError::MalformedImage);
    }
    if !bias.is_empty() && bias.len() < c_out {
        return Err(VisionError::MalformedImage);
    }
    if out.len() < c_out * h_out * w_out {
        return Err(VisionError::OutputBufferTooSmall);
    }
    for oc in 0..c_out {
        for oh in 0..h_out {
            for ow in 0..w_out {
                let mut acc = if bias.is_empty() { 0.0 } else { bias[oc] };
                for ic in 0..c_in {
                    for ky in 0..kh {
                        for kx in 0..kw {
                            let ih = oh * stride_h + ky;
                            let iw = ow * stride_w + kx;
                            if ih < pad_h || iw < pad_w {
                                continue;
                            }
                            let ih = ih - pad_h;
                            let iw = iw - pad_w;
                            if ih >= h || iw >= w {
                                continue;
                            }
                            let iv = input[ic * h * w + ih * w + iw];
                            let wv = weight
                                [oc * (c_in * kh * kw) + ic * (kh * kw) + ky * kw + kx];
                            acc += iv * wv;
                        }
                    }
                }
                out[oc * h_out * w_out + oh * w_out + ow] = acc;
            }
        }
    }
    Ok((h_out, w_out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_1x1() {
        // 1x1 conv identity on 1 channel 2x2
        let input = [1.0f32, 2.0, 3.0, 4.0];
        let weight = [1.0f32];
        let mut out = [0.0f32; 4];
        let (ho, wo) = conv2d_nchw_f32(
            &input, 1, 2, 2, &weight, 1, 1, 1, &[], 1, 1, 0, 0, &mut out,
        )
        .unwrap();
        assert_eq!((ho, wo), (2, 2));
        assert_eq!(out, input);
    }
}
