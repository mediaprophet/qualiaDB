//! ISO/IEC 15444-1 Annex G inverse color transforms (RCT / ICT), DC level shift and sample packing.
use super::markers::{ComponentSiz, WaveletTransform};

pub fn apply_mct_and_level_shift(
    planes_f64: &mut [Vec<f64>],
    mct: u8,
    transform: WaveletTransform,
    comps: &[ComponentSiz],
) {
    if mct != 0 && planes_f64.len() >= 3 {
        let len = planes_f64[0].len();
        let (p0, rest) = planes_f64.split_at_mut(1);
        let (p1, p2) = rest.split_at_mut(1);
        match transform {
            WaveletTransform::Reversible53 => {
                for i in 0..len {
                    let y0 = p0[0][i].round() as i64;
                    let cb = p1[0][i].round() as i64;
                    let cr = p2[0][i].round() as i64;
                    let g = y0 - ((cb + cr) >> 2);
                    let r = cr + g;
                    let b = cb + g;
                    p0[0][i] = r as f64;
                    p1[0][i] = g as f64;
                    p2[0][i] = b as f64;
                }
            }
            WaveletTransform::Irreversible97 => {
                for i in 0..len {
                    let y = p0[0][i];
                    let cb = p1[0][i];
                    let cr = p2[0][i];
                    p0[0][i] = y + 1.402 * cr;
                    p1[0][i] = y - 0.34413 * cb - 0.71414 * cr;
                    p2[0][i] = y + 1.772 * cb;
                }
            }
        }
    }
    for (ci, plane) in planes_f64.iter_mut().enumerate() {
        let c = &comps[ci];
        if !c.signed {
            let shift = (1u64 << (c.precision - 1)) as f64;
            for v in plane.iter_mut() {
                *v += shift;
            }
        }
    }
}

pub fn pack_tile_into_output(
    planes_f64: &[Vec<f64>],
    tx0: usize,
    ty0: usize,
    tw: usize,
    th: usize,
    img_w: usize,
    comps: &[ComponentSiz],
    bytes_per_sample: usize,
    output: &mut [u8],
) {
    let nc = comps.len();
    for y in 0..th {
        for x in 0..tw {
            let src_idx = y * tw + x;
            let gx = tx0 + x;
            let gy = ty0 + y;
            let dst_px = (gy * img_w + gx) * nc;
            for c in 0..nc {
                let comp = &comps[c];
                let raw = planes_f64[c][src_idx].round() as i64;
                let clamped = if comp.signed {
                    let min_v = -(1i64 << (comp.precision - 1));
                    let max_v = (1i64 << (comp.precision - 1)) - 1;
                    raw.clamp(min_v, max_v)
                } else {
                    let max_v = (1i64 << comp.precision) - 1;
                    raw.clamp(0, max_v)
                };
                if bytes_per_sample == 1 {
                    output[dst_px + c] = (clamped as i16) as u8;
                } else {
                    let le = (clamped as i16 as u16).to_le_bytes();
                    let off = (dst_px + c) * 2;
                    output[off] = le[0];
                    output[off + 1] = le[1];
                }
            }
        }
    }
}
