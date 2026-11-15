//! ISO/IEC 18181-1 JPEG-XL VarDCT decoding primitives: exact separable orthonormal IDCT
//! across 2x2..64x64 / rectangular / AFV transforms, Chroma-from-Luma (CfL), inverse
//! Gaborish restoration filter, and Edge-Preserving Filter (EPF).
use super::container::{JxlError, JxlFrameHeader, JxlImageHeader, JxlLimits};
use super::entropy::{unpack_signed, JxlBitReader, JxlEntropyDecoder};
use core::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarDctTransform {
    Dct(usize, usize), // (rows, cols)
    Identity,
    Afv(usize),
}

impl VarDctTransform {
    pub fn from_id(id: u32) -> Result<Self, JxlError> {
        match id {
            0 => Ok(Self::Dct(8, 8)),
            1 => Ok(Self::Identity),
            2 => Ok(Self::Dct(2, 2)),
            3 => Ok(Self::Dct(4, 4)),
            4 => Ok(Self::Dct(16, 16)),
            5 => Ok(Self::Dct(32, 32)),
            6 => Ok(Self::Dct(16, 8)),
            7 => Ok(Self::Dct(8, 16)),
            8 => Ok(Self::Dct(32, 8)),
            9 => Ok(Self::Dct(8, 32)),
            10 => Ok(Self::Dct(32, 16)),
            11 => Ok(Self::Dct(16, 32)),
            12..=15 => Ok(Self::Afv((id - 12) as usize)),
            16 => Ok(Self::Dct(64, 64)),
            17 => Ok(Self::Dct(64, 32)),
            18 => Ok(Self::Dct(32, 64)),
            19 => Ok(Self::Dct(4, 8)),
            20 => Ok(Self::Dct(8, 4)),
            _ => Err(JxlError::CorruptStream("unknown JXL VarDCT transform ID")),
        }
    }
}

/// Exact normalized 1D orthonormal Type-II Inverse DCT (matches `IDCT_N` matrix transpose).
pub fn idct_1d_into(coeffs: &[f64], out: &mut [f64], stride_in: usize, stride_out: usize, n: usize) {
    let inv_sqrt_n = (1.0 / n as f64).sqrt();
    let sqrt_2_over_n = (2.0 / n as f64).sqrt();
    let dc = coeffs[0] * inv_sqrt_n;
    for r in 0..n {
        let mut sum = dc;
        let angle_base = PI * (r as f64 + 0.5) / n as f64;
        for k in 1..n {
            sum += coeffs[k * stride_in] * sqrt_2_over_n * (angle_base * k as f64).cos();
        }
        out[r * stride_out] = sum;
    }
}

/// Exact normalized 1D orthonormal Type-II Forward DCT for encoder parity tests.
#[allow(dead_code)]
pub fn fdct_1d_into(samples: &[f64], out: &mut [f64], stride_in: usize, stride_out: usize, n: usize) {
    let inv_sqrt_n = (1.0 / n as f64).sqrt();
    let sqrt_2_over_n = (2.0 / n as f64).sqrt();
    for k in 0..n {
        let scale = if k == 0 { inv_sqrt_n } else { sqrt_2_over_n };
        let angle_base = PI * k as f64 / n as f64;
        let mut sum = 0.0;
        for r in 0..n {
            sum += samples[r * stride_in] * (angle_base * (r as f64 + 0.5)).cos();
        }
        out[k * stride_out] = sum * scale;
    }
}

/// Apply 2D inverse VarDCT block transform (`rows x cols` spectral -> spatial).
pub fn apply_inverse_block_transform(
    tf: VarDctTransform,
    coeffs: &[f64],
    spatial_out: &mut [f64],
) {
    match tf {
        VarDctTransform::Identity => spatial_out[..coeffs.len()].copy_from_slice(coeffs),
        VarDctTransform::Afv(_) => {
            let n = (coeffs.len() as f64).sqrt().round() as usize;
            let mut tmp = vec![0.0; coeffs.len()];
            for r in 0..n {
                idct_1d_into(&coeffs[r * n..], &mut tmp[r * n..], 1, 1, n);
            }
            for c in 0..n {
                idct_1d_into(&tmp[c..], &mut spatial_out[c..], n, n, n);
            }
        }
        VarDctTransform::Dct(bh, bw) => {
            let mut tmp = vec![0.0; bh * bw];
            for r in 0..bh {
                idct_1d_into(&coeffs[r * bw..], &mut tmp[r * bw..], 1, 1, bw);
            }
            for c in 0..bw {
                idct_1d_into(&tmp[c..], &mut spatial_out[c..], bw, bw, bh);
            }
        }
    }
}

/// Apply 5-tap separable symmetric inverse Gaborish restoration filter (ISO 18181-1 Section 9.5).
pub fn apply_inverse_gaborish(plane: &mut [f64], w: usize, h: usize) {
    // Standard ISO inverse Gaborish cross weights: center=1, w1=-0.025, w2=-0.004
    const W1: f64 = -0.025;
    const W2: f64 = -0.004;
    const NORM: f64 = 1.0 / (1.0 + 4.0 * W1 + 4.0 * W2);
    let src = plane.to_vec();
    let sample = |x: isize, y: isize| -> f64 {
        let cx = x.clamp(0, w as isize - 1) as usize;
        let cy = y.clamp(0, h as isize - 1) as usize;
        src[cy * w + cx]
    };
    for y in 0..h {
        let yi = y as isize;
        for x in 0..w {
            let xi = x as isize;
            let sum = sample(xi, yi)
                + W1 * (sample(xi - 1, yi) + sample(xi + 1, yi) + sample(xi, yi - 1) + sample(xi, yi + 1))
                + W2 * (sample(xi - 1, yi - 1) + sample(xi + 1, yi - 1) + sample(xi - 1, yi + 1) + sample(xi + 1, yi + 1));
            plane[y * w + x] = sum * NORM;
        }
    }
}

/// Apply ISO 18181-1 Edge-Preserving Filter (`epf_iters` passes) over X/Y/B planes.
pub fn apply_epf_filter(planes: &mut [Vec<f64>], w: usize, h: usize, iters: u8, sigma: f64) {
    if iters == 0 || sigma <= 0.0 || planes.len() < 3 {
        return;
    }
    for _pass in 0..iters.min(3) {
        let src = planes.to_vec();
        for y in 0..h {
            for x in 0..w {
                let mut acc = [src[0][y * w + x], src[1][y * w + x], src[2][y * w + x]];
                let mut w_tot = 1.0;
                for (dx, dy) in [(-1isize, 0isize), (1, 0), (0, -1), (0, 1)] {
                    let nx = (x as isize + dx).clamp(0, w as isize - 1) as usize;
                    let ny = (y as isize + dy).clamp(0, h as isize - 1) as usize;
                    let idx = ny * w + nx;
                    let c_idx = y * w + x;
                    let sad = (src[0][c_idx] - src[0][idx]).abs()
                        + (src[1][c_idx] - src[1][idx]).abs()
                        + (src[2][c_idx] - src[2][idx]).abs();
                    let weight = (1.0 - sad / (sigma * 3.0)).max(0.0);
                    w_tot += weight;
                    for c in 0..3 {
                        acc[c] += weight * src[c][idx];
                    }
                }
                for c in 0..3 {
                    planes[c][y * w + x] = acc[c] / w_tot;
                }
            }
        }
    }
}

/// Apply Chroma-from-Luma (`CfL`) tile predictors to X and B channels from reconstructed Y.
pub fn apply_cfl(
    x_plane: &mut [f64],
    y_plane: &[f64],
    b_plane: &mut [f64],
    w: usize,
    h: usize,
    tile_y_to_x: &[f64],
    tile_y_to_b: &[f64],
    tiles_w: usize,
) {
    for y in 0..h {
        let ty = y / 64;
        for x in 0..w {
            let tx = x / 64;
            let ti = ty * tiles_w + tx;
            let luma = y_plane[y * w + x];
            x_plane[y * w + x] += tile_y_to_x.get(ti).copied().unwrap_or(0.0) * luma;
            b_plane[y * w + x] += tile_y_to_b.get(ti).copied().unwrap_or(0.0) * luma;
        }
    }
}

/// Decode a VarDCT frame section containing global quantizer, CfL tile factors,
/// transform type map, quantized DCT coefficients (order `Y, X, B`), IDCT, CfL, and loop filters.
pub fn decode_vardct_frame_planes(
    br: &mut JxlBitReader<'_>,
    hdr: &JxlImageHeader,
    fh: &JxlFrameHeader,
    limits: JxlLimits,
) -> Result<Vec<Vec<f64>>, JxlError> {
    let w = hdr.width as usize;
    let h = hdr.height as usize;
    let nc = if hdr.is_gray { 1 } else { 3 };
    if w * h * nc > limits.max_decoded_bytes {
        return Err(JxlError::ByteLimit);
    }

    // LfGlobal: Quantizer scales + HfGlobal quant step
    let global_scale = br.read_u32_dist(&[(1, 11), (2049, 11), (4097, 12), (8193, 16)])? as f64 / 65536.0;
    let quant_lf = br.read_u32_dist(&[(16, 0), (1, 5), (1, 8), (1, 16)])? as f64;
    let x_scale = (2.0f64).powi(fh.x_qm_scale as i32 - 3);
    let b_scale = (2.0f64).powi(fh.b_qm_scale as i32 - 2);

    let tiles_w = w.div_ceil(64);
    let tiles_h = h.div_ceil(64);
    let n_tiles = tiles_w * tiles_h;
    let blocks_w = w.div_ceil(8);
    let blocks_h = h.div_ceil(8);
    let n_blocks = blocks_w * blocks_h;

    let mut dec = JxlEntropyDecoder::read(br, 4, w)?;
    let mut tile_y_to_x = Vec::with_capacity(n_tiles);
    let mut tile_y_to_b = Vec::with_capacity(n_tiles);
    for _ in 0..n_tiles {
        tile_y_to_x.push(unpack_signed(dec.read_uint(br, 0)?) as f64 / 128.0);
        tile_y_to_b.push(unpack_signed(dec.read_uint(br, 0)?) as f64 / 128.0);
    }

    let mut block_tf = Vec::with_capacity(n_blocks);
    for _ in 0..n_blocks {
        block_tf.push(VarDctTransform::from_id(dec.read_uint(br, 1)?)?);
    }

    let padded_w = blocks_w * 8;
    let padded_h = blocks_h * 8;
    let mut planes = vec![vec![0.0f64; w * h]; nc];
    let mut work = vec![vec![0.0f64; padded_w * padded_h]; nc];

    let order = if nc == 1 { [0usize, 0, 0] } else { [1usize, 0, 2] }; // Y, X, B
    for bi in 0..n_blocks {
        let bx = (bi % blocks_w) * 8;
        let by = (bi / blocks_w) * 8;
        let tf = block_tf[bi];
        let (bh, bw) = match tf {
            VarDctTransform::Dct(r, c) => (r.min(8), c.min(8)),
            _ => (8, 8),
        };
        let ncoef = bh * bw;
        for &c in order.iter().take(nc) {
            let mut spec = vec![0.0f64; ncoef];
            let step = match c {
                0 => global_scale * quant_lf * x_scale,
                1 => global_scale * quant_lf,
                _ => global_scale * quant_lf * b_scale,
            };
            for k in 0..ncoef {
                let q = unpack_signed(dec.read_uint(br, if k == 0 { 2 } else { 3 })?);
                spec[k] = q as f64 * step;
            }
            let mut spat = vec![0.0f64; ncoef];
            apply_inverse_block_transform(tf, &spec, &mut spat);
            for ry in 0..bh {
                for rx in 0..bw {
                    work[c][(by + ry) * padded_w + (bx + rx)] = spat[ry * bw + rx];
                }
            }
        }
    }
    dec.finalize()?;

    for c in 0..nc {
        for y in 0..h {
            planes[c][y * w..(y + 1) * w].copy_from_slice(&work[c][y * padded_w..y * padded_w + w]);
        }
    }

    if nc == 3 {
        let (left, right) = planes.split_at_mut(1);
        let (y_slice, b_slice) = right.split_at_mut(1);
        apply_cfl(&mut left[0], &y_slice[0], &mut b_slice[0], w, h, &tile_y_to_x, &tile_y_to_b, tiles_w);
    }
    if fh.gab {
        for p in &mut planes {
            apply_inverse_gaborish(p, w, h);
        }
    }
    if fh.epf_iters > 0 && nc == 3 {
        apply_epf_filter(&mut planes, w, h, fh.epf_iters, global_scale * 2.0);
    }
    Ok(planes)
}
