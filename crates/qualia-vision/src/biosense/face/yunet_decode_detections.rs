//! Decode YuNet-style face detection tensors into fixed detection boxes.
//!
//! Runtime-agnostic: caller supplies model output floats (from ORT/tract/other).
//! Weight file is MIT (OpenCV Zoo YuNet) — PermissiveReady, not commercial-gated.

use crate::cv::error::CvError;

/// One face box in normalized image coordinates (0..1) or pixels if scale applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub score: f32,
}

/// YuNet multi-stride heads emit prior-decoded boxes; this decoder accepts a flat
/// list of rows `[x, y, w, h, score, ...pad]` with `stride` floats per row.
pub fn yunet_decode_detections(
    rows: &[f32],
    stride: usize,
    score_idx: usize,
    min_score: f32,
    out: &mut [FaceBox],
) -> Result<usize, CvError> {
    if stride < 5 || score_idx >= stride {
        return Err(CvError::InvalidParameter);
    }
    if rows.is_empty() {
        return Ok(0);
    }
    if rows.len() % stride != 0 {
        return Err(CvError::DimensionMismatch);
    }
    let n_rows = rows.len() / stride;
    let mut written = 0usize;
    for i in 0..n_rows {
        if written >= out.len() {
            break;
        }
        let base = i * stride;
        let score = rows[base + score_idx];
        if score < min_score {
            continue;
        }
        out[written] = FaceBox {
            x: rows[base],
            y: rows[base + 1],
            w: rows[base + 2],
            h: rows[base + 3],
            score,
        };
        written += 1;
    }
    Ok(written)
}

/// Greedy IoU NMS into `out` (caller buffer); returns kept count.
pub fn face_nms(boxes: &[FaceBox], iou_thr: f32, out: &mut [FaceBox]) -> usize {
    let mut order: [usize; 256] = [0; 256];
    let n = boxes.len().min(256);
    for i in 0..n {
        order[i] = i;
    }
    // Simple selection sort by score desc (n small).
    for i in 0..n {
        let mut best = i;
        for j in (i + 1)..n {
            if boxes[order[j]].score > boxes[order[best]].score {
                best = j;
            }
        }
        order.swap(i, best);
    }
    let mut kept = 0usize;
    let mut suppressed = [false; 256];
    for a in 0..n {
        let ia = order[a];
        if suppressed[ia] {
            continue;
        }
        if kept >= out.len() {
            break;
        }
        out[kept] = boxes[ia];
        kept += 1;
        for b in (a + 1)..n {
            let ib = order[b];
            if suppressed[ib] {
                continue;
            }
            if iou(boxes[ia], boxes[ib]) > iou_thr {
                suppressed[ib] = true;
            }
        }
    }
    kept
}

fn iou(a: FaceBox, b: FaceBox) -> f32 {
    let ax2 = a.x + a.w;
    let ay2 = a.y + a.h;
    let bx2 = b.x + b.w;
    let by2 = b.y + b.h;
    let ix1 = a.x.max(b.x);
    let iy1 = a.y.max(b.y);
    let ix2 = ax2.min(bx2);
    let iy2 = ay2.min(by2);
    let iw = (ix2 - ix1).max(0.0);
    let ih = (iy2 - iy1).max(0.0);
    let inter = iw * ih;
    let uni = a.w * a.h + b.w * b.h - inter;
    if uni <= 1e-9 {
        0.0
    } else {
        inter / uni
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_filters_score() {
        let rows = [0.1, 0.1, 0.2, 0.2, 0.9, 0.0, 0.5, 0.5, 0.1, 0.1, 0.05, 0.0];
        let mut out = [FaceBox {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            score: 0.0,
        }; 4];
        let n = yunet_decode_detections(&rows, 6, 4, 0.2, &mut out).unwrap();
        assert_eq!(n, 1);
        assert!((out[0].score - 0.9).abs() < 1e-5);
    }

    #[test]
    fn nms_suppresses_overlap() {
        let boxes = [
            FaceBox {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
                score: 0.9,
            },
            FaceBox {
                x: 0.1,
                y: 0.1,
                w: 1.0,
                h: 1.0,
                score: 0.8,
            },
        ];
        let mut out = [FaceBox {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            score: 0.0,
        }; 2];
        let k = face_nms(&boxes, 0.3, &mut out);
        assert_eq!(k, 1);
    }
}
