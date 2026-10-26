//! Bounded RGBA8 image filters for the authoring image owner.
//!
//! Samples are encoded RGB, as in the existing image authoring operations.
//! The Gaussian convolution premultiplies RGB by alpha before filtering.

use super::{FilterPlan, ImageError, ImageFilterOp, ImageLimits};

/// Check dimensions, parameters, work, and resident buffer budget without allocating.
pub fn filter_plan(
    width: u32,
    height: u32,
    op: ImageFilterOp,
    limits: ImageLimits,
) -> Result<FilterPlan, ImageError> {
    if width == 0 || height == 0 {
        return Err(ImageError::Dimensions);
    }
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or(ImageError::Dimensions)?;
    if pixels > limits.max_pixels {
        return Err(ImageError::Budget);
    }
    let (radius, active) = match op {
        ImageFilterOp::GaussianBlur { sigma } => {
            check_range(sigma, 0.0, 16.0)?;
            (radius(sigma), sigma != 0.0)
        }
        ImageFilterOp::Sharpen { sigma, amount } => {
            check_range(sigma, 0.0, 16.0)?;
            check_range(amount, 0.0, 4.0)?;
            (radius(sigma), sigma != 0.0 && amount != 0.0)
        }
        ImageFilterOp::HueRotate { degrees } => {
            check_range(degrees, -360.0, 360.0)?;
            (0, degrees.rem_euclid(360.0) != 0.0)
        }
        ImageFilterOp::Emboss {
            azimuth_degrees,
            strength,
        } => {
            check_range(azimuth_degrees, -360.0, 360.0)?;
            check_range(strength, 0.0, 1.0)?;
            (0, strength != 0.0)
        }
    };
    let convolution = matches!(op, ImageFilterOp::GaussianBlur { .. } | ImageFilterOp::Sharpen { .. })
        && active;
    let scratch_pixels = if convolution { pixels } else { 0 };
    // Both separable passes visit every tap. The final conversion is one more
    // visit; this also bounds operations that need only one visit per pixel.
    let visits_per_pixel = if convolution {
        (radius as usize)
            .checked_mul(4)
            .and_then(|n| n.checked_add(3))
            .ok_or(ImageError::Budget)?
    } else if matches!(op, ImageFilterOp::Emboss { .. }) && active {
        5 // four luminance neighbours and one write
    } else {
        1
    };
    let work = pixels
        .checked_mul(visits_per_pixel)
        .ok_or(ImageError::Budget)?;
    if work > limits.max_work {
        return Err(ImageError::Budget);
    }
    let bytes = pixels
        .checked_mul(8) // source and caller output, four bytes each
        .and_then(|n| scratch_pixels.checked_mul(16).and_then(|s| n.checked_add(s)))
        .ok_or(ImageError::Budget)?;
    if bytes > limits.max_bytes {
        return Err(ImageError::Budget);
    }
    Ok(FilterPlan {
        scratch_pixels,
        work,
        bytes,
    })
}

/// Filter into caller-owned output. Every error is returned before output changes.
pub fn filter_rgba8_into(
    src: &[[u8; 4]],
    width: u32,
    height: u32,
    op: ImageFilterOp,
    limits: ImageLimits,
    out: &mut [[u8; 4]],
    scratch: &mut [[f32; 4]],
) -> Result<(), ImageError> {
    let plan = filter_plan(width, height, op, limits)?;
    let pixels = (width as usize) * (height as usize);
    if src.len() != pixels {
        return Err(ImageError::Storage);
    }
    if out.len() < pixels {
        return Err(ImageError::Output);
    }
    if scratch.len() < plan.scratch_pixels {
        return Err(ImageError::Scratch);
    }
    let out = &mut out[..pixels];
    match op {
        ImageFilterOp::GaussianBlur { sigma } if sigma != 0.0 => {
            let (kernel, radius) = gaussian_kernel(sigma);
            blur_horizontal(src, width as usize, height as usize, &kernel, radius, scratch);
            blur_vertical(src, width as usize, height as usize, &kernel, radius, scratch, out, 0.0);
        }
        ImageFilterOp::Sharpen { sigma, amount } if sigma != 0.0 && amount != 0.0 => {
            let (kernel, radius) = gaussian_kernel(sigma);
            blur_horizontal(src, width as usize, height as usize, &kernel, radius, scratch);
            blur_vertical(src, width as usize, height as usize, &kernel, radius, scratch, out, amount);
        }
        ImageFilterOp::HueRotate { degrees } if degrees.rem_euclid(360.0) != 0.0 => {
            hue_rotate(src, degrees, out);
        }
        ImageFilterOp::Emboss {
            azimuth_degrees,
            strength,
        } if strength != 0.0 => {
            emboss(src, width as usize, height as usize, azimuth_degrees, strength, out);
        }
        _ => out.copy_from_slice(src),
    }
    Ok(())
}

fn check_range(value: f32, lo: f32, hi: f32) -> Result<(), ImageError> {
    if value.is_finite() && value >= lo && value <= hi {
        Ok(())
    } else {
        Err(ImageError::Parameters)
    }
}

fn radius(sigma: f32) -> i32 {
    (3.0 * sigma).ceil() as i32
}

fn gaussian_kernel(sigma: f32) -> ([f32; 97], i32) {
    let r = radius(sigma);
    let mut kernel = [0.0_f32; 97]; // max radius is ceil(3 * 16) = 48
    let mut total = 0.0_f32;
    for d in -r..=r {
        let x = d as f32 / sigma;
        let k = (-0.5 * x * x).exp();
        kernel[(d + r) as usize] = k;
        total += k;
    }
    for k in &mut kernel[..=(2 * r) as usize] {
        *k /= total;
    }
    (kernel, r)
}

fn clamp_coord(value: i64, max: usize) -> usize {
    value.clamp(0, max as i64 - 1) as usize
}

fn premult(px: [u8; 4]) -> [f32; 4] {
    let a = px[3] as f32 / 255.0;
    [px[0] as f32 * a, px[1] as f32 * a, px[2] as f32 * a, a]
}

fn blur_horizontal(
    src: &[[u8; 4]],
    w: usize,
    h: usize,
    kernel: &[f32; 97],
    r: i32,
    scratch: &mut [[f32; 4]],
) {
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0_f32; 4];
            for d in -r..=r {
                let px = premult(src[y * w + clamp_coord(x as i64 + d as i64, w)]);
                let k = kernel[(d + r) as usize];
                for c in 0..4 {
                    acc[c] += px[c] * k;
                }
            }
            scratch[y * w + x] = acc;
        }
    }
}

fn blur_vertical(
    src: &[[u8; 4]],
    w: usize,
    h: usize,
    kernel: &[f32; 97],
    r: i32,
    scratch: &[[f32; 4]],
    out: &mut [[u8; 4]],
    sharpen_amount: f32,
) {
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0_f32; 4];
            for d in -r..=r {
                let px = scratch[clamp_coord(y as i64 + d as i64, h) * w + x];
                let k = kernel[(d + r) as usize];
                for c in 0..4 {
                    acc[c] += px[c] * k;
                }
            }
            let i = y * w + x;
            if sharpen_amount == 0.0 {
                let alpha = byte(acc[3] * 255.0);
                out[i][3] = alpha;
                for c in 0..3 {
                    out[i][c] = if acc[3] > 0.0 { byte(acc[c] / acc[3]) } else { 0 };
                }
            } else {
                out[i][3] = src[i][3];
                for c in 0..3 {
                    let blurred = if acc[3] > 0.0 { acc[c] / acc[3] } else { 0.0 };
                    out[i][c] = if src[i][3] == 0 {
                        0
                    } else {
                        byte(src[i][c] as f32 + sharpen_amount * (src[i][c] as f32 - blurred))
                    };
                }
            }
        }
    }
}

fn byte(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

fn hue_rotate(src: &[[u8; 4]], degrees: f32, out: &mut [[u8; 4]]) {
    let angle = degrees.to_radians();
    let (sin, cos) = angle.sin_cos();
    // SVG feColorMatrix hueRotate coefficients; alpha is passed through.
    let m = [
        [0.213 + 0.787 * cos - 0.213 * sin, 0.715 - 0.715 * cos - 0.715 * sin, 0.072 - 0.072 * cos + 0.928 * sin],
        [0.213 - 0.213 * cos + 0.143 * sin, 0.715 + 0.285 * cos + 0.140 * sin, 0.072 - 0.072 * cos - 0.283 * sin],
        [0.213 - 0.213 * cos - 0.787 * sin, 0.715 - 0.715 * cos + 0.715 * sin, 0.072 + 0.928 * cos + 0.072 * sin],
    ];
    for (input, output) in src.iter().zip(out.iter_mut()) {
        if input[0] == input[1] && input[1] == input[2] {
            *output = *input;
            continue;
        }
        for c in 0..3 {
            output[c] = byte(m[c][0] * input[0] as f32 + m[c][1] * input[1] as f32 + m[c][2] * input[2] as f32);
        }
        output[3] = input[3];
    }
}

fn luminance(px: [u8; 4]) -> f32 {
    0.2126 * px[0] as f32 + 0.7152 * px[1] as f32 + 0.0722 * px[2] as f32
}

fn emboss(src: &[[u8; 4]], w: usize, h: usize, degrees: f32, strength: f32, out: &mut [[u8; 4]]) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    for y in 0..h {
        for x in 0..w {
            let left = luminance(src[y * w + x.saturating_sub(1)]);
            let right = luminance(src[y * w + (x + 1).min(w - 1)]);
            let up = luminance(src[y.saturating_sub(1) * w + x]);
            let down = luminance(src[(y + 1).min(h - 1) * w + x]);
            let relief = (128.0 + cos * (right - left) * 0.5 + sin * (down - up) * 0.5)
                .clamp(0.0, 255.0);
            let i = y * w + x;
            for c in 0..3 {
                out[i][c] = byte(src[i][c] as f32 * (1.0 - strength) + relief * strength);
            }
            out[i][3] = src[i][3];
        }
    }
}
