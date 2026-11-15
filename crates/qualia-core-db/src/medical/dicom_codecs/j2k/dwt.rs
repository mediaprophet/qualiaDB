//! ISO/IEC 15444-1 Annex E dequantization & Annex F 2D inverse 5/3 + 9/7 DWT.
use super::markers::{QuantizationParams, WaveletTransform};

pub const ALPHA_97: f64 = -1.586134342059924;
pub const BETA_97: f64 = -0.052980118572961;
pub const GAMMA_97: f64 = 0.882911075530934;
pub const DELTA_97: f64 = 0.443506852043971;
pub const K_97: f64 = 1.230174104914001;

pub fn subband_total_bitplanes(qcd: &QuantizationParams, sb_seq_idx: usize, decomp_levels: u8, nominal_bpp: u8) -> (u8, f64) {
    let gain = if sb_seq_idx == 0 {
        0u8
    } else {
        match (sb_seq_idx - 1) % 3 {
            0 | 1 => 1u8,
            _ => 2u8,
        }
    };
    let res_level = if sb_seq_idx == 0 { decomp_levels } else { decomp_levels - ((sb_seq_idx - 1) / 3) as u8 };
    let (exp_b, mant_b) = if qcd.style == 1 && !qcd.steps.is_empty() {
        let (e0, m0) = qcd.steps[0];
        let nl = decomp_levels as i32;
        let nb = res_level as i32;
        ((e0 as i32 - (nl - nb)).max(0) as u8, m0)
    } else {
        qcd.steps.get(sb_seq_idx).copied().unwrap_or((nominal_bpp + gain, 0))
    };
    let mb = qcd.guard_bits + exp_b.saturating_sub(1);
    let r_b = (nominal_bpp + gain) as i32;
    let delta_b = 2.0f64.powi(r_b - exp_b as i32) * (1.0 + mant_b as f64 / 2048.0);
    (mb, delta_b)
}

#[inline]
pub fn sym_ext(i: isize, len: usize) -> usize {
    if len <= 1 {
        return 0;
    }
    let period = (len - 1) as isize;
    let mut idx = i.rem_euclid(2 * period);
    if idx >= len as isize {
        idx = 2 * period - idx;
    }
    idx as usize
}

/// 1D reversible inverse LeGall 5/3 lifting (`cas = start_coord & 1`).
pub fn idwt_1d_53(low: &[i32], high: &[i32], out: &mut [i32], cas: usize) {
    let n = out.len();
    if n == 0 {
        return;
    }
    if n == 1 {
        out[0] = if cas == 0 { low[0] } else { high[0] >> 1 };
        return;
    }
    for i in 0..n {
        if (i + cas).is_multiple_of(2) {
            let k = i / 2;
            let h_idx = |pos: isize| -> i32 {
                let len = high.len();
                if len == 0 { return 0; }
                high[sym_ext(pos, len)]
            };
            let (hl, hr) = if cas == 0 {
                (h_idx(k as isize - 1), h_idx(k as isize))
            } else {
                (h_idx(k as isize), h_idx(k as isize + 1))
            };
            out[i] = low[k] - ((hl + hr + 2) >> 2);
        }
    }
    for i in 0..n {
        if !(i + cas).is_multiple_of(2) {
            let k = i / 2;
            let x_idx = |pos: isize| -> i32 { out[sym_ext(pos, n)] };
            out[i] = high[k] + ((x_idx(i as isize - 1) + x_idx(i as isize + 1)) >> 1);
        }
    }
}

/// 1D irreversible inverse CDF 9/7 lifting (`cas = start_coord & 1`).
pub fn idwt_1d_97(low: &[f64], high: &[f64], out: &mut [f64], cas: usize) {
    let n = out.len();
    if n == 0 {
        return;
    }
    if n == 1 {
        out[0] = if cas == 0 { low[0] } else { high[0] / K_97 };
        return;
    }
    for i in 0..n {
        let k = i / 2;
        if (i + cas).is_multiple_of(2) {
            out[i] = low[k] * K_97;
        } else {
            out[i] = high[k] / K_97;
        }
    }
    let lift = |buf: &mut [f64], parity: usize, coef: f64| {
        for i in 0..n {
            if (i + cas) % 2 == parity {
                let l = buf[sym_ext(i as isize - 1, n)];
                let r = buf[sym_ext(i as isize + 1, n)];
                buf[i] -= coef * (l + r);
            }
        }
    };
    lift(out, 0, DELTA_97);
    lift(out, 1, GAMMA_97);
    lift(out, 0, BETA_97);
    lift(out, 1, ALPHA_97);
}

#[allow(clippy::too_many_arguments)]
pub fn synthesize_level_53(
    ll: &[i32],
    hl: &[i32],
    lh: &[i32],
    hh: &[i32],
    w_ll: usize,
    h_ll: usize,
    w_hl: usize,
    h_lh: usize,
    x0: u32,
    y0: u32,
) -> Vec<i32> {
    let w = w_ll + w_hl;
    let h = h_ll + h_lh;
    let cas_x = (x0 & 1) as usize;
    let cas_y = (y0 & 1) as usize;
    let mut inter = vec![0i32; w * h];
    let mut row_tmp = vec![0i32; w];
    for y in 0..h_ll {
        let l_row = &ll[y * w_ll..(y + 1) * w_ll];
        let h_row = &hl[y * w_hl..(y + 1) * w_hl];
        idwt_1d_53(l_row, h_row, &mut row_tmp, cas_x);
        let dst_y = if cas_y == 0 { 2 * y } else { 2 * y + 1 };
        if dst_y < h {
            inter[dst_y * w..(dst_y + 1) * w].copy_from_slice(&row_tmp);
        }
    }
    for y in 0..h_lh {
        let l_row = &lh[y * w_ll..(y + 1) * w_ll];
        let h_row = &hh[y * w_hl..(y + 1) * w_hl];
        idwt_1d_53(l_row, h_row, &mut row_tmp, cas_x);
        let dst_y = if cas_y == 0 { 2 * y + 1 } else { 2 * y };
        if dst_y < h {
            inter[dst_y * w..(dst_y + 1) * w].copy_from_slice(&row_tmp);
        }
    }
    let mut col_l = vec![0i32; h_ll];
    let mut col_h = vec![0i32; h_lh];
    let mut col_out = vec![0i32; h];
    let mut result = vec![0i32; w * h];
    for x in 0..w {
        for y in 0..h_ll {
            let sy = if cas_y == 0 { 2 * y } else { 2 * y + 1 };
            col_l[y] = inter[sy * w + x];
        }
        for y in 0..h_lh {
            let sy = if cas_y == 0 { 2 * y + 1 } else { 2 * y };
            col_h[y] = inter[sy * w + x];
        }
        idwt_1d_53(&col_l, &col_h, &mut col_out, cas_y);
        for y in 0..h {
            result[y * w + x] = col_out[y];
        }
    }
    result
}

#[allow(clippy::too_many_arguments)]
pub fn synthesize_level_97(
    ll: &[f64],
    hl: &[f64],
    lh: &[f64],
    hh: &[f64],
    w_ll: usize,
    h_ll: usize,
    w_hl: usize,
    h_lh: usize,
    x0: u32,
    y0: u32,
) -> Vec<f64> {
    let w = w_ll + w_hl;
    let h = h_ll + h_lh;
    let cas_x = (x0 & 1) as usize;
    let cas_y = (y0 & 1) as usize;
    let mut inter = vec![0.0f64; w * h];
    let mut row_tmp = vec![0.0f64; w];
    for y in 0..h_ll {
        idwt_1d_97(&ll[y * w_ll..(y + 1) * w_ll], &hl[y * w_hl..(y + 1) * w_hl], &mut row_tmp, cas_x);
        let dst_y = if cas_y == 0 { 2 * y } else { 2 * y + 1 };
        if dst_y < h {
            inter[dst_y * w..(dst_y + 1) * w].copy_from_slice(&row_tmp);
        }
    }
    for y in 0..h_lh {
        idwt_1d_97(&lh[y * w_ll..(y + 1) * w_ll], &hh[y * w_hl..(y + 1) * w_hl], &mut row_tmp, cas_x);
        let dst_y = if cas_y == 0 { 2 * y + 1 } else { 2 * y };
        if dst_y < h {
            inter[dst_y * w..(dst_y + 1) * w].copy_from_slice(&row_tmp);
        }
    }
    let mut col_l = vec![0.0f64; h_ll];
    let mut col_h = vec![0.0f64; h_lh];
    let mut col_out = vec![0.0f64; h];
    let mut result = vec![0.0f64; w * h];
    for x in 0..w {
        for y in 0..h_ll {
            let sy = if cas_y == 0 { 2 * y } else { 2 * y + 1 };
            col_l[y] = inter[sy * w + x];
        }
        for y in 0..h_lh {
            let sy = if cas_y == 0 { 2 * y + 1 } else { 2 * y };
            col_h[y] = inter[sy * w + x];
        }
        idwt_1d_97(&col_l, &col_h, &mut col_out, cas_y);
        for y in 0..h {
            result[y * w + x] = col_out[y];
        }
    }
    result
}

#[allow(dead_code)]
pub fn dequantize_to_f64(q: &[i32], transform: WaveletTransform, delta_b: f64) -> Vec<f64> {
    match transform {
        WaveletTransform::Reversible53 => q.iter().map(|&v| v as f64).collect(),
        WaveletTransform::Irreversible97 => q.iter().map(|&v| v as f64 * delta_b).collect(),
    }
}
