//! Phase 5 / V5 — bounded multi-object tracker (IoU + class affinity).
//!
//! Fixed slot table — no heap. Overflow: when all slots full, new detections
//! keep `track_id = 0` and set `FLAG_TRACK_OVERFLOW` (deterministic, no eviction
//! of older tracks unless they expire by age).

use crate::preprocess::iou_u16;
use crate::types::Detection;

/// Maximum simultaneous tracks (caller-visible bound).
pub const MAX_TRACKS: usize = 32;

/// Detection flag: track table full; this box was not assigned a track id.
pub const FLAG_TRACK_OVERFLOW: u32 = 1 << 8;

#[derive(Debug, Clone, Copy)]
struct TrackSlot {
    active: bool,
    id: u32,
    class_hash: u64,
    x_min_u16: u16,
    y_min_u16: u16,
    x_max_u16: u16,
    y_max_u16: u16,
    last_frame: u32,
    /// Frames since last match (0 = matched this update).
    miss_count: u32,
}

impl TrackSlot {
    const EMPTY: Self = Self {
        active: false,
        id: 0,
        class_hash: 0,
        x_min_u16: 0,
        y_min_u16: 0,
        x_max_u16: 0,
        y_max_u16: 0,
        last_frame: 0,
        miss_count: 0,
    };

    fn as_box(&self) -> Detection {
        let mut d = Detection::empty();
        d.class_hash = self.class_hash;
        d.x_min_u16 = self.x_min_u16;
        d.y_min_u16 = self.y_min_u16;
        d.x_max_u16 = self.x_max_u16;
        d.y_max_u16 = self.y_max_u16;
        d.track_id = self.id;
        d
    }
}

/// Bounded IoU tracker over a frame sequence.
#[derive(Debug, Clone)]
pub struct BoundedTracker {
    slots: [TrackSlot; MAX_TRACKS],
    next_id: u32,
    /// Minimum IoU to associate a detection with a track.
    pub iou_thresh: f32,
    /// Drop track after this many consecutive unmatched frames.
    pub max_miss: u32,
    /// If true, only match same class_hash.
    pub class_gated: bool,
}

impl Default for BoundedTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl BoundedTracker {
    pub fn new() -> Self {
        Self {
            slots: [TrackSlot::EMPTY; MAX_TRACKS],
            next_id: 1,
            iou_thresh: 0.3,
            max_miss: 5,
            class_gated: true,
        }
    }

    pub fn active_track_count(&self) -> usize {
        self.slots.iter().filter(|s| s.active).count()
    }

    /// Assign `track_id` on each of the first `n` detections in place.
    /// Unmatched / overflow detections get track_id 0 (and FLAG_TRACK_OVERFLOW when table full).
    pub fn update(&mut self, frame_index: u32, detections: &mut [Detection], n: usize) {
        let n = n.min(detections.len());
        // Greedy: for each detection (score order would be better; sort indices by score).
        let mut order = [0u16; 256];
        let n_ord = n.min(256);
        for i in 0..n_ord {
            order[i] = i as u16;
        }
        for i in 0..n_ord {
            let mut best = i;
            for j in (i + 1)..n_ord {
                if detections[order[j] as usize].score_u16
                    > detections[order[best] as usize].score_u16
                {
                    best = j;
                }
            }
            order.swap(i, best);
        }

        // Which track slots already matched this frame.
        let mut track_used = [false; MAX_TRACKS];
        let mut det_matched = [false; 256];

        for oi in 0..n_ord {
            let di = order[oi] as usize;
            let det = detections[di];
            if det.class_hash == 0 && det.score_u16 == 0 {
                continue;
            }
            let mut best_ti: Option<usize> = None;
            let mut best_iou = self.iou_thresh;
            for (ti, slot) in self.slots.iter().enumerate() {
                if !slot.active || track_used[ti] {
                    continue;
                }
                if self.class_gated && slot.class_hash != det.class_hash {
                    continue;
                }
                let tb = slot.as_box();
                let iou = iou_u16(&det, &tb);
                if iou > best_iou {
                    best_iou = iou;
                    best_ti = Some(ti);
                }
            }
            if let Some(ti) = best_ti {
                track_used[ti] = true;
                det_matched[di] = true;
                let slot = &mut self.slots[ti];
                slot.x_min_u16 = det.x_min_u16;
                slot.y_min_u16 = det.y_min_u16;
                slot.x_max_u16 = det.x_max_u16;
                slot.y_max_u16 = det.y_max_u16;
                slot.last_frame = frame_index;
                slot.miss_count = 0;
                detections[di].track_id = slot.id;
                detections[di].frame_index = frame_index;
            }
        }

        // Spawn tracks for unmatched detections.
        for oi in 0..n_ord {
            let di = order[oi] as usize;
            if det_matched[di] {
                continue;
            }
            let det = detections[di];
            if det.class_hash == 0 && det.score_u16 == 0 {
                detections[di].track_id = 0;
                detections[di].frame_index = frame_index;
                continue;
            }
            if let Some(ti) = self.slots.iter().position(|s| !s.active) {
                let id = self.next_id;
                self.next_id = self.next_id.wrapping_add(1).max(1);
                self.slots[ti] = TrackSlot {
                    active: true,
                    id,
                    class_hash: det.class_hash,
                    x_min_u16: det.x_min_u16,
                    y_min_u16: det.y_min_u16,
                    x_max_u16: det.x_max_u16,
                    y_max_u16: det.y_max_u16,
                    last_frame: frame_index,
                    miss_count: 0,
                };
                detections[di].track_id = id;
                detections[di].frame_index = frame_index;
            } else {
                // Overflow policy: leave track_id 0, flag, do not evict.
                detections[di].track_id = 0;
                detections[di].frame_index = frame_index;
                detections[di].flags |= FLAG_TRACK_OVERFLOW;
            }
        }

        // Age unmatched tracks.
        for (ti, slot) in self.slots.iter_mut().enumerate() {
            if !slot.active || track_used[ti] {
                continue;
            }
            slot.miss_count = slot.miss_count.saturating_add(1);
            if slot.miss_count > self.max_miss {
                *slot = TrackSlot::EMPTY;
            }
        }
    }

    /// Run tracker over a sequence of per-frame detection slices.
    /// `frames[f]` is (detections buffer, count). Mutates each buffer's track_id.
    pub fn track_sequence(&mut self, frames: &mut [(&mut [Detection], usize)]) {
        for (fi, (dets, n)) in frames.iter_mut().enumerate() {
            self.update(fi as u32, dets, *n);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_at(class: u64, x0: u16, y0: u16, x1: u16, y1: u16, score: u16) -> Detection {
        let mut d = Detection::empty();
        d.class_hash = class;
        d.instance_hash = class ^ (x0 as u64);
        d.score_u16 = score;
        d.x_min_u16 = x0;
        d.y_min_u16 = y0;
        d.x_max_u16 = x1;
        d.y_max_u16 = y1;
        d
    }

    #[test]
    fn same_object_keeps_track_id() {
        let mut t = BoundedTracker::new();
        let mut f0 = [box_at(0xAA, 1000, 1000, 5000, 5000, 50_000)];
        t.update(0, &mut f0, 1);
        let id0 = f0[0].track_id;
        assert_ne!(id0, 0);

        // Slightly moved same class — should match.
        let mut f1 = [box_at(0xAA, 1200, 1100, 5200, 5100, 48_000)];
        t.update(1, &mut f1, 1);
        assert_eq!(f1[0].track_id, id0);
        assert_eq!(t.active_track_count(), 1);
    }

    #[test]
    fn two_objects_two_tracks() {
        let mut t = BoundedTracker::new();
        let mut f0 = [
            box_at(0x11, 0, 0, 10_000, 10_000, 50_000),
            box_at(0x22, 40_000, 40_000, 55_000, 55_000, 49_000),
        ];
        t.update(0, &mut f0, 2);
        assert_ne!(f0[0].track_id, 0);
        assert_ne!(f0[1].track_id, 0);
        assert_ne!(f0[0].track_id, f0[1].track_id);
        assert_eq!(t.active_track_count(), 2);
    }

    #[test]
    fn overflow_is_deterministic() {
        let mut t = BoundedTracker::new();
        t.max_miss = 100;
        // Fill all tracks with non-overlapping boxes.
        for i in 0..MAX_TRACKS {
            let x = (i as u16).wrapping_mul(2000);
            let mut f = [box_at(0x100 + i as u64, x, 0, x.saturating_add(500), 500, 40_000)];
            t.update(i as u32, &mut f, 1);
            assert_ne!(f[0].track_id, 0, "slot {i}");
        }
        assert_eq!(t.active_track_count(), MAX_TRACKS);
        // One more should overflow.
        let mut extra = [box_at(0xDEAD, 60_000, 60_000, 65_000, 65_000, 30_000)];
        t.update(MAX_TRACKS as u32, &mut extra, 1);
        assert_eq!(extra[0].track_id, 0);
        assert_ne!(extra[0].flags & FLAG_TRACK_OVERFLOW, 0);
    }

    #[test]
    fn miss_expires_track() {
        let mut t = BoundedTracker::new();
        t.max_miss = 2;
        let mut f0 = [box_at(0xBB, 1000, 1000, 4000, 4000, 50_000)];
        t.update(0, &mut f0, 1);
        assert_eq!(t.active_track_count(), 1);
        // Empty frames.
        let mut empty = [];
        t.update(1, &mut empty, 0);
        t.update(2, &mut empty, 0);
        t.update(3, &mut empty, 0);
        assert_eq!(t.active_track_count(), 0);
    }
}
