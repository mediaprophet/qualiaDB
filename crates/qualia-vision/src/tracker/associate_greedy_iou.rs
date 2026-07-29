//! Greedy best-IoU association between detections and free track slots.
//!
//! One ByteTrack matching stage: for each eligible detection (score order),
//! pick the free active track with the highest IoU above threshold.
//! Zero-heap: all state is caller-supplied fixed buffers.

use crate::preprocess::iou_u16;
use crate::types::Detection;

use super::MAX_TRACKS;

/// Sentinel: detection has no matched track slot.
pub const NO_TRACK: u16 = u16::MAX;

/// Greedy IoU match for one association pass (high-score or low-score stage).
///
/// - `det_order[0..n_order]` — detection indices in preferred order (usually score-desc).
/// - `det_eligible[di]` — true if this detection may participate in this stage.
/// - `track_boxes[ti]` / `track_class[ti]` — geometry and class of active tracks.
/// - `track_active[ti]` — slot occupied; `track_used[ti]` — already matched this frame.
/// - On match: sets `track_used[ti]`, `det_matched[di]`, `det_to_track[di] = ti`.
///
/// Returns the number of pairs formed.
pub fn associate_greedy_iou(
    detections: &[Detection],
    det_order: &[u16],
    n_order: usize,
    det_eligible: &[bool],
    track_boxes: &[Detection; MAX_TRACKS],
    track_class: &[u64; MAX_TRACKS],
    track_active: &[bool; MAX_TRACKS],
    track_used: &mut [bool; MAX_TRACKS],
    class_gated: bool,
    iou_thresh: f32,
    det_matched: &mut [bool],
    det_to_track: &mut [u16],
) -> usize {
    let n_order = n_order.min(det_order.len());
    let mut pairs = 0usize;

    for oi in 0..n_order {
        let di = det_order[oi] as usize;
        if di >= detections.len() || di >= det_eligible.len() || di >= det_matched.len() {
            continue;
        }
        if !det_eligible[di] || det_matched[di] {
            continue;
        }
        let det = detections[di];
        if det.class_hash == 0 && det.score_u16 == 0 {
            continue;
        }

        let mut best_ti: Option<usize> = None;
        let mut best_iou = iou_thresh;
        for ti in 0..MAX_TRACKS {
            if !track_active[ti] || track_used[ti] {
                continue;
            }
            if class_gated && track_class[ti] != det.class_hash {
                continue;
            }
            let iou = iou_u16(&det, &track_boxes[ti]);
            // Strict `>` keeps the first track on exact IoU ties (deterministic).
            if iou > best_iou {
                best_iou = iou;
                best_ti = Some(ti);
            }
        }

        if let Some(ti) = best_ti {
            track_used[ti] = true;
            det_matched[di] = true;
            if di < det_to_track.len() {
                det_to_track[di] = ti as u16;
            }
            pairs = pairs.saturating_add(1);
        }
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Detection;

    fn box_at(class: u64, x0: u16, y0: u16, x1: u16, y1: u16, score: u16) -> Detection {
        let mut d = Detection::empty();
        d.class_hash = class;
        d.score_u16 = score;
        d.x_min_u16 = x0;
        d.y_min_u16 = y0;
        d.x_max_u16 = x1;
        d.y_max_u16 = y1;
        d
    }

    #[test]
    fn matches_overlapping_same_class() {
        let dets = [
            box_at(0xAA, 1000, 1000, 5000, 5000, 50_000),
            box_at(0xBB, 40_000, 40_000, 50_000, 50_000, 40_000),
        ];
        let mut track_boxes = [Detection::empty(); MAX_TRACKS];
        let mut track_class = [0u64; MAX_TRACKS];
        let mut track_active = [false; MAX_TRACKS];
        track_boxes[0] = box_at(0xAA, 1100, 1100, 5100, 5100, 0);
        track_class[0] = 0xAA;
        track_active[0] = true;
        track_boxes[1] = box_at(0xBB, 41_000, 41_000, 51_000, 51_000, 0);
        track_class[1] = 0xBB;
        track_active[1] = true;

        let det_order = [0u16, 1];
        let det_eligible = [true, true];
        let mut track_used = [false; MAX_TRACKS];
        let mut det_matched = [false; 2];
        let mut det_to_track = [NO_TRACK; 2];

        let n = associate_greedy_iou(
            &dets,
            &det_order,
            2,
            &det_eligible,
            &track_boxes,
            &track_class,
            &track_active,
            &mut track_used,
            true,
            0.3,
            &mut det_matched,
            &mut det_to_track,
        );
        assert_eq!(n, 2);
        assert_eq!(det_to_track[0], 0);
        assert_eq!(det_to_track[1], 1);
    }

    #[test]
    fn skips_ineligible_detections() {
        let dets = [box_at(0xAA, 1000, 1000, 5000, 5000, 50_000)];
        let mut track_boxes = [Detection::empty(); MAX_TRACKS];
        let mut track_class = [0u64; MAX_TRACKS];
        let mut track_active = [false; MAX_TRACKS];
        track_boxes[0] = box_at(0xAA, 1100, 1100, 5100, 5100, 0);
        track_class[0] = 0xAA;
        track_active[0] = true;

        let det_order = [0u16];
        let det_eligible = [false];
        let mut track_used = [false; MAX_TRACKS];
        let mut det_matched = [false; 1];
        let mut det_to_track = [NO_TRACK; 1];

        let n = associate_greedy_iou(
            &dets,
            &det_order,
            1,
            &det_eligible,
            &track_boxes,
            &track_class,
            &track_active,
            &mut track_used,
            true,
            0.3,
            &mut det_matched,
            &mut det_to_track,
        );
        assert_eq!(n, 0);
        assert_eq!(det_to_track[0], NO_TRACK);
    }
}
