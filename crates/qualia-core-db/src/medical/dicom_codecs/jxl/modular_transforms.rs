//! ISO/IEC 18181-1 JPEG-XL Modular transforms: permutation Reversible Color Transform
//! (RCT, including YCgCo), Palette+delta expansion, and smooth Haar Squeeze inversion.
use super::container::JxlError;

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct SqueezeStep {
    pub horizontal: bool,
    pub in_place: bool,
    pub begin_c: usize,
    pub num_c: usize,
}

#[derive(Clone, Debug)]
pub enum ModularTransform {
    Rct {
        begin_c: usize,
        rct_type: u8,
    },
    Palette {
        begin_c: usize,
        num_c: usize,
        nb_colours: usize,
        nb_deltas: usize,
        d_pred: u8,
    },
    Squeeze {
        steps: Vec<SqueezeStep>,
    },
}

#[derive(Clone, Debug)]
pub struct ModularChannel {
    pub width: usize,
    pub height: usize,
    pub hshift: i32,
    pub vshift: i32,
    pub data: Vec<i64>,
}

impl ModularChannel {
    pub fn new(width: usize, height: usize, hshift: i32, vshift: i32) -> Result<Self, JxlError> {
        let len = width.checked_mul(height).ok_or(JxlError::Overflow)?;
        let mut data = Vec::new();
        data.try_reserve_exact(len).map_err(|_| JxlError::ByteLimit)?;
        data.resize(len, 0i64);
        Ok(Self { width, height, hshift, vshift, data })
    }
}

pub fn eval_fixed_predictor(
    pred_id: u8,
    wp_pred: i64,
    w: i64,
    n: i64,
    nw: i64,
    ne: i64,
    ww: i64,
    nn: i64,
    nee: i64,
) -> i64 {
    match pred_id {
        0 => 0,
        1 => w,
        2 => n,
        3 => (w + n) >> 1,
        4 => {
            let g = n + w - nw;
            if (g - n).abs() < (g - w).abs() { n } else { w }
        }
        5 => {
            let g = n + w - nw;
            g.clamp(n.min(w), n.max(w))
        }
        6 => wp_pred,
        7 => ne,
        8 => nw,
        9 => ww,
        10 => (w + nw) >> 1,
        11 => (n + nw) >> 1,
        12 => (n + ne) >> 1,
        13 => (6 * n - 2 * nn + 7 * w + ww + nee + 3 * ne + 8) >> 4,
        _ => (n + w + ne + nw) >> 2,
    }
}

/// Smooth non-linear tendency term for ISO/IEC 18181-1 H.6.2 Squeeze Haar wavelet.
pub fn smooth_tendency(b: i64, a: i64, n: i64) -> i64 {
    let mut diff = 0i64;
    if (b >= a && a >= n) || (b <= a && a <= n) {
        let abs_val = (4 * b - 3 * n - a + 6).abs() / 12;
        let cap = (2 * (a - b).abs()).min(2 * (n - a).abs());
        diff = abs_val.min(cap);
        if b < n {
            diff = -diff;
        }
    }
    diff
}

pub fn inv_hsqueeze_pair(avg: &[i64], res: &[i64], out: &mut [i64], w: usize, h: usize, w0: usize, w1: usize) {
    for y in 0..h {
        let a_row = &avg[y * w0..(y + 1) * w0];
        let r_row = &res[y * w1..(y + 1) * w1];
        let d_row = &mut out[y * w..(y + 1) * w];
        for x in 0..w1 {
            let a = a_row[x];
            let b = if x > 0 { a_row[x - 1] } else { a };
            let n = if x + 1 < w0 { a_row[x + 1] } else { a };
            let diff = r_row[x] + smooth_tendency(b, a, n);
            let second = a - (diff >> 1);
            d_row[2 * x] = second + diff;
            d_row[2 * x + 1] = second;
        }
        if w % 2 == 1 {
            d_row[w - 1] = a_row[w0 - 1];
        }
    }
}

pub fn inv_vsqueeze_pair(avg: &[i64], res: &[i64], out: &mut [i64], w: usize, h: usize, h0: usize, h1: usize) {
    for x in 0..w {
        for y in 0..h1 {
            let a = avg[y * w + x];
            let b = if y > 0 { avg[(y - 1) * w + x] } else { a };
            let n = if y + 1 < h0 { avg[(y + 1) * w + x] } else { a };
            let diff = res[y * w + x] + smooth_tendency(b, a, n);
            let second = a - (diff >> 1);
            out[(2 * y) * w + x] = second + diff;
            out[(2 * y + 1) * w + x] = second;
        }
        if h % 2 == 1 {
            out[(h - 1) * w + x] = avg[(h0 - 1) * w + x];
        }
    }
}

pub fn apply_inverse_rct(channels: &mut [ModularChannel], begin_c: usize, rct_type: u8) -> Result<(), JxlError> {
    if begin_c + 3 > channels.len() || rct_type > 41 {
        return Err(JxlError::CorruptStream("invalid JXL RCT transform"));
    }
    let perm = (rct_type / 7) as usize;
    let sub = rct_type % 7;
    let (p0, p1, p2) = match perm {
        0 => (0, 1, 2),
        1 => (1, 2, 0),
        2 => (2, 0, 1),
        3 => (0, 2, 1),
        4 => (1, 0, 2),
        _ => (2, 1, 0),
    };
    let len = channels[begin_c].data.len();
    for i in 0..len {
        let a = channels[begin_c].data[i];
        let b = channels[begin_c + 1].data[i];
        let c = channels[begin_c + 2].data[i];
        let (v0, v1, v2) = match sub {
            0 => (a, b, c),
            1 => (a, b, c + a),
            2 => (a, b + a, c),
            3 => (a, b + a, c + a),
            4 => {
                let r = b + a;
                (a, r, c + ((a + r) >> 1))
            }
            5 => {
                let bl = c + a;
                (a, b + ((a + bl) >> 1), bl)
            }
            _ => {
                let t = a - (c >> 1);
                let g = c + t;
                let bl = t - (b >> 1);
                let r = bl + b;
                (r, g, bl)
            }
        };
        let vals = [v0, v1, v2];
        channels[begin_c].data[i] = vals[p0];
        channels[begin_c + 1].data[i] = vals[p1];
        channels[begin_c + 2].data[i] = vals[p2];
    }
    Ok(())
}

pub fn apply_inverse_transforms(
    channels: &mut Vec<ModularChannel>,
    transforms: &[ModularTransform],
) -> Result<(), JxlError> {
    for tf in transforms.iter().rev() {
        match tf {
            ModularTransform::Rct { begin_c, rct_type } => apply_inverse_rct(channels, *begin_c, *rct_type)?,
            ModularTransform::Palette { begin_c, num_c, nb_colours, nb_deltas, d_pred } => {
                if *begin_c + 1 >= channels.len() {
                    return Err(JxlError::CorruptStream("JXL Palette channels missing"));
                }
                let pal_ch = channels.remove(*begin_c);
                let src = channels.remove(*begin_c);
                let total_entries = *nb_colours + *nb_deltas;
                let mut palette = vec![vec![0i64; total_entries]; *num_c];
                for r in 0..*num_c {
                    for c in 0..total_entries {
                        palette[r][c] = pal_ch.data.get(r * total_entries + c).copied().unwrap_or(0);
                    }
                }
                let (w, h) = (src.width, src.height);
                let mut expanded = Vec::with_capacity(*num_c);
                for _ in 0..*num_c {
                    expanded.push(ModularChannel::new(w, h, src.hshift, src.vshift)?);
                }
                for y in 0..h {
                    for x in 0..w {
                        let idx = src.data[y * w + x];
                        for c in 0..*num_c {
                            let val = if idx >= 0 && (idx as usize) < *nb_deltas {
                                let wc = if x > 0 { expanded[c].data[y * w + x - 1] } else { 0 };
                                let nc = if y > 0 { expanded[c].data[(y - 1) * w + x] } else { 0 };
                                let nwc = if x > 0 && y > 0 { expanded[c].data[(y - 1) * w + x - 1] } else { wc };
                                let nec = if x + 1 < w && y > 0 { expanded[c].data[(y - 1) * w + x + 1] } else { nc };
                                palette[c][idx as usize] + eval_fixed_predictor(*d_pred, wc, wc, nc, nwc, nec, wc, nc, nec)
                            } else {
                                let pi = (idx - *nb_deltas as i64).clamp(0, (*nb_colours as i64).saturating_sub(1)) as usize;
                                palette[c][*nb_deltas + pi]
                            };
                            expanded[c].data[y * w + x] = val;
                        }
                    }
                }
                for (off, ec) in expanded.into_iter().enumerate() {
                    channels.insert(*begin_c + off, ec);
                }
            }
            ModularTransform::Squeeze { steps } => {
                for step in steps.iter().rev() {
                    for c_off in 0..step.num_c {
                        let ci = step.begin_c + c_off;
                        let ri = ci + step.num_c;
                        if ri >= channels.len() {
                            return Err(JxlError::CorruptStream("Squeeze residual channel missing"));
                        }
                        let avg = &channels[ci];
                        let res = &channels[ri];
                        if step.horizontal {
                            let (w, h) = (avg.width + res.width, avg.height);
                            let mut out = vec![0i64; w * h];
                            inv_hsqueeze_pair(&avg.data, &res.data, &mut out, w, h, avg.width, res.width);
                            channels[ci] = ModularChannel { width: w, height: h, hshift: avg.hshift, vshift: avg.vshift, data: out };
                        } else {
                            let (w, h) = (avg.width, avg.height + res.height);
                            let mut out = vec![0i64; w * h];
                            inv_vsqueeze_pair(&avg.data, &res.data, &mut out, w, h, avg.height, res.height);
                            channels[ci] = ModularChannel { width: w, height: h, hshift: avg.hshift, vshift: avg.vshift, data: out };
                        }
                        channels.remove(ri);
                    }
                }
            }
        }
    }
    Ok(())
}
