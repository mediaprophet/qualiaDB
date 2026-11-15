//! ISO/IEC 18181-1 JPEG-XL Modular coding: MA decision tree, self-correcting Weighted
//! predictor (WP), context selection, and multi-channel modular section decoding.
use super::container::{JxlError, JxlImageHeader, JxlLimits};
use super::entropy::{unpack_signed, JxlBitReader, JxlEntropyDecoder};
use super::modular_transforms::{
    apply_inverse_transforms, eval_fixed_predictor, ModularChannel, ModularTransform, SqueezeStep,
};

#[cfg(test)]
pub use super::modular_transforms::{apply_inverse_rct, smooth_tendency};

#[derive(Clone, Debug)]
pub enum MaNode {
    Decision {
        property: u8,
        split_val: i64,
        left: usize,
        right: usize,
    },
    Leaf {
        ctx: usize,
        predictor: u8,
        offset: i64,
        multiplier: i64,
    },
}

pub fn parse_ma_tree(br: &mut JxlBitReader<'_>, max_nodes: usize) -> Result<(Vec<MaNode>, usize), JxlError> {
    let mut dec = JxlEntropyDecoder::read(br, 6, 1)?;
    let mut nodes: Vec<MaNode> = Vec::new();
    let mut to_decode = 1usize;
    let mut max_ctx = 0usize;

    while to_decode > 0 {
        if nodes.len() >= max_nodes {
            return Err(JxlError::ByteLimit);
        }
        to_decode -= 1;
        let prop = dec.read_uint(br, 1)? as usize;
        if prop > 0 {
            let split_val = unpack_signed(dec.read_uint(br, 2)?);
            let left = nodes.len() + 1 + to_decode;
            let right = left + 1;
            nodes.push(MaNode::Decision {
                property: (prop - 1).min(15) as u8,
                split_val,
                left,
                right,
            });
            to_decode += 2;
        } else {
            let ctx = dec.read_uint(br, 0)? as usize;
            max_ctx = max_ctx.max(ctx);
            let predictor = dec.read_uint(br, 2)?.min(14) as u8;
            let offset = unpack_signed(dec.read_uint(br, 3)?);
            let mul_log = dec.read_uint(br, 4)?.min(30);
            let mul_bits = dec.read_uint(br, 5)? as i64;
            let multiplier = (mul_bits + 1) << mul_log;
            nodes.push(MaNode::Leaf { ctx, predictor, offset, multiplier });
        }
    }
    dec.finalize()?;
    Ok((nodes, max_ctx + 1))
}

struct WpState {
    err_row: Vec<[i64; 4]>,
    weights: [i64; 4],
}

impl WpState {
    fn new(width: usize) -> Self {
        Self { err_row: vec![[0i64; 4]; width + 2], weights: [1 << 10; 4] }
    }

    fn predict(&self, x: usize, n: i64, w: i64, nw: i64, ne: i64, w_true_err: i64) -> (i64, i64, [i64; 4]) {
        let ew = if x > 0 { self.err_row[x - 1] } else { [0; 4] };
        let en = self.err_row[x];
        let ene = self.err_row[x + 1];
        let p0 = w + ne - n;
        let p1 = n - ((en[1] + ew[1] + ene[1]) >> 5).clamp(-8, 8);
        let p2 = w - ((w_true_err + ew[2]) >> 5).clamp(-8, 8);
        let p3 = n + w - nw;
        let preds = [p0, p1, p2, p3];
        let mut w_sum = 0i64;
        let mut num = 0i64;
        for i in 0..4 {
            let weight = (self.weights[i] / (1 + ((ew[i].abs() + en[i].abs()) >> 2))).max(1);
            w_sum += weight;
            num += weight * preds[i];
        }
        let max_err = ew.iter().chain(en.iter()).map(|e| e.abs()).max().unwrap_or(0);
        let pred = (num + (w_sum >> 1)) / w_sum.max(1);
        (pred, max_err, preds)
    }

    fn update(&mut self, x: usize, actual: i64, preds: [i64; 4]) {
        let mut errs = [0i64; 4];
        for i in 0..4 {
            let e = (actual - preds[i]).abs().min(1 << 20);
            errs[i] = e;
            if e < 8 {
                self.weights[i] = (self.weights[i] + (1 << 8)).min(1 << 18);
            } else {
                self.weights[i] = (self.weights[i] - (self.weights[i] >> 4)).max(16);
            }
        }
        self.err_row[x] = errs;
    }
}

pub fn decode_modular_channel(
    br: &mut JxlBitReader<'_>,
    dec: &mut JxlEntropyDecoder,
    tree: &[MaNode],
    ch_idx: usize,
    group_id: usize,
    ch: &mut ModularChannel,
) -> Result<(), JxlError> {
    let (w, h) = (ch.width, ch.height);
    let mut wp = WpState::new(w);
    let mut prev_err_row = vec![0i64; w];

    for y in 0..h {
        let mut prev_w_err = 0i64;
        for x in 0..w {
            let n_val = if y > 0 { ch.data[(y - 1) * w + x] } else { 0 };
            let w_val = if x > 0 { ch.data[y * w + x - 1] } else { n_val };
            let nw_val = if x > 0 && y > 0 { ch.data[(y - 1) * w + x - 1] } else { w_val };
            let ne_val = if x + 1 < w && y > 0 { ch.data[(y - 1) * w + x + 1] } else { n_val };
            let ww_val = if x >= 2 { ch.data[y * w + x - 2] } else { w_val };
            let nn_val = if y >= 2 { ch.data[(y - 2) * w + x] } else { n_val };
            let nee_val = if x + 2 < w && y > 0 { ch.data[(y - 1) * w + x + 2] } else { ne_val };

            let (wp_pred, wp_err, wp_preds) = wp.predict(x, n_val, w_val, nw_val, ne_val, prev_w_err);
            let props: [i64; 16] = [
                ch_idx as i64, group_id as i64, y as i64, x as i64,
                n_val.abs(), w_val.abs(), n_val, w_val, prev_err_row[x],
                w_val + n_val - nw_val, w_val - nw_val, nw_val - n_val,
                n_val - ne_val, n_val - nn_val, w_val - ww_val, wp_err,
            ];
            let mut node_idx = 0usize;
            let leaf = loop {
                match tree.get(node_idx) {
                    Some(&MaNode::Decision { property, split_val, left, right }) => {
                        node_idx = if props[property as usize] > split_val { left } else { right };
                    }
                    Some(&MaNode::Leaf { ctx, predictor, offset, multiplier }) => {
                        break (ctx, predictor, offset, multiplier);
                    }
                    None => return Err(JxlError::CorruptStream("invalid JXL MA tree traversal")),
                }
            };
            let res = unpack_signed(dec.read_uint(br, leaf.0)?) * leaf.3;
            let pred = eval_fixed_predictor(leaf.1, wp_pred, w_val, n_val, nw_val, ne_val, ww_val, nn_val, nee_val) + leaf.2;
            let val = pred.saturating_add(res);
            ch.data[y * w + x] = val;
            prev_w_err = val - w_val;
            prev_err_row[x] = val - (w_val + n_val - nw_val);
            wp.update(x, val, wp_preds);
        }
    }
    Ok(())
}

/// Decode a Modular-encoded codestream section into channel `f64` planes.
pub fn decode_modular_frame_planes(
    br: &mut JxlBitReader<'_>,
    hdr: &JxlImageHeader,
    limits: JxlLimits,
) -> Result<Vec<Vec<f64>>, JxlError> {
    let nc = if hdr.is_gray { 1 } else { 3 } + hdr.num_extra_channels as usize;
    let have_global_tree = br.read_bool()?;
    let (tree, num_ctx) = if have_global_tree {
        parse_ma_tree(br, limits.max_tree_nodes)?
    } else {
        (vec![MaNode::Leaf { ctx: 0, predictor: 5, offset: 0, multiplier: 1 }], 1)
    };
    let num_transforms = br.read_u32_dist(&[(0, 0), (1, 0), (2, 4), (18, 8)])? as usize;
    let mut transforms = Vec::with_capacity(num_transforms);
    let mut chan_dims: Vec<(usize, usize)> = vec![(hdr.width as usize, hdr.height as usize); nc];

    for _ in 0..num_transforms {
        let tr = br.read_bits(2)?;
        match tr {
            0 => {
                let begin_c = br.read_u32_dist(&[(0, 3), (8, 3), (72, 10), (1096, 13)])? as usize;
                let rct_type = br.read_u32_dist(&[(6, 0), (0, 2), (2, 4), (10, 6)])? as u8;
                if begin_c + 3 > chan_dims.len() {
                    return Err(JxlError::CorruptStream("JXL RCT channel out of range"));
                }
                transforms.push(ModularTransform::Rct { begin_c, rct_type });
            }
            1 => {
                let begin_c = br.read_u32_dist(&[(0, 3), (8, 3), (72, 10), (1096, 13)])? as usize;
                let num_c = br.read_u32_dist(&[(3, 0), (4, 0), (1, 1), (5, 13)])? as usize;
                let nb_colours = br.read_u32_dist(&[(0, 8), (256, 10), (1280, 12), (5376, 16)])? as usize;
                let nb_deltas = br.read_u32_dist(&[(0, 0), (1, 8), (257, 10), (1281, 16)])? as usize;
                let d_pred = br.read_bits(4)? as u8;
                if begin_c + num_c > chan_dims.len() || num_c == 0 {
                    return Err(JxlError::CorruptStream("JXL Palette channel range invalid"));
                }
                let prev_dim = chan_dims[begin_c];
                chan_dims.drain(begin_c..begin_c + num_c);
                chan_dims.insert(begin_c, (nb_colours + nb_deltas, num_c));
                chan_dims.insert(begin_c + 1, prev_dim);
                transforms.push(ModularTransform::Palette { begin_c, num_c, nb_colours, nb_deltas, d_pred });
            }
            2 => {
                let num_steps = br.read_u32_dist(&[(0, 0), (1, 4), (9, 6), (41, 8)])? as usize;
                let mut steps = Vec::with_capacity(num_steps);
                for _ in 0..num_steps {
                    let horizontal = br.read_bool()?;
                    let in_place = br.read_bool()?;
                    let begin_c = br.read_u32_dist(&[(0, 3), (8, 3), (72, 10), (1096, 13)])? as usize;
                    let num_c = br.read_u32_dist(&[(1, 0), (2, 0), (3, 0), (4, 4)])? as usize;
                    if begin_c + num_c > chan_dims.len() {
                        return Err(JxlError::CorruptStream("JXL Squeeze channel range invalid"));
                    }
                    for ci in (begin_c..begin_c + num_c).rev() {
                        let (cw, ch) = chan_dims[ci];
                        if horizontal {
                            chan_dims[ci] = (cw.div_ceil(2), ch);
                            chan_dims.insert(ci + num_c, (cw / 2, ch));
                        } else {
                            chan_dims[ci] = (cw, ch.div_ceil(2));
                            chan_dims.insert(ci + num_c, (cw, ch / 2));
                        }
                    }
                    steps.push(SqueezeStep { horizontal, in_place, begin_c, num_c });
                }
                transforms.push(ModularTransform::Squeeze { steps });
            }
            _ => return Err(JxlError::Unsupported("unknown JXL Modular transform ID")),
        }
    }

    let mut dec = JxlEntropyDecoder::read(br, num_ctx, hdr.width as usize)?;
    let mut channels = Vec::with_capacity(chan_dims.len());
    for (ci, &(cw, ch)) in chan_dims.iter().enumerate() {
        let mut mc = ModularChannel::new(cw, ch, 0, 0)?;
        decode_modular_channel(br, &mut dec, &tree, ci, 0, &mut mc)?;
        channels.push(mc);
    }
    dec.finalize()?;

    apply_inverse_transforms(&mut channels, &transforms)?;
    Ok(channels
        .into_iter()
        .map(|c| c.data.into_iter().map(|v| v as f64).collect())
        .collect())
}
