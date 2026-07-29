//! Phase 5 / V5 — bounded multi-object tracker (ByteTrack-class association).
//!
//! Pure Rust, no training, no Python. Fixed slot table — no heap on the hot path.
//!
//! # Association (ByteTrack-style two-stage IoU)
//! 1. **High-score** detections associate first (`iou_thresh`).
//! 2. **Low-score** detections recover remaining unmatched tracks (`low_iou_thresh`);
//!    they never spawn new tracks.
//! 3. Unmatched high-score detections birth **Tentative** tracks.
//! 4. After `min_hits` successful associations a track becomes **Confirmed**.
//! 5. Unmatched tracks age; Tentative dies faster; Confirmed expires after `max_miss`.
//!
//! Overflow: when all slots full, new detections keep `track_id = 0` and set
//! `FLAG_TRACK_OVERFLOW` (deterministic; no eviction of live tracks).

mod associate_greedy_iou;
mod score_tier_of;

pub use associate_greedy_iou::{associate_greedy_iou, NO_TRACK};
pub use score_tier_of::{score_tier_of, ScoreTier};

use crate::types::Detection;

use associate_greedy_iou::associate_greedy_iou as assoc_iou;
use score_tier_of::score_tier_of as tier_of;

/// Maximum simultaneous tracks (caller-visible bound).
pub const MAX_TRACKS: usize = 32;

/// Max detections considered per frame (fixed association scratch).
pub const MAX_ASSOC_DETS: usize = 256;

/// Detection flag: track table full; this box was not assigned a track id.
pub const FLAG_TRACK_OVERFLOW: u32 = 1 << 8;

/// Default high-score floor (~0.5 in score_u16 units).
pub const DEFAULT_HIGH_SCORE_U16: u16 = 32_768;
/// Default low-score floor (~0.1) — band for second-stage recovery only.
pub const DEFAULT_LOW_SCORE_U16: u16 = 6_554;
/// Default hits required before Tentative → Confirmed.
pub const DEFAULT_MIN_HITS: u32 = 2;

/// Lifecycle of a resident track slot.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackState {
    /// Newborn; may be dropped on first miss before confirmation.
    Tentative = 0,
    /// Survived `min_hits` associations; ages with `max_miss`.
    Confirmed = 1,
}

#[derive(Debug, Clone, Copy)]
struct TrackSlot {
    active: bool,
    state: TrackState,
    id: u32,
    class_hash: u64,
    x_min_u16: u16,
    y_min_u16: u16,
    x_max_u16: u16,
    y_max_u16: u16,
    last_frame: u32,
    /// Frames since last match (0 = matched this update).
    miss_count: u32,
    /// Successful associations including birth frame.
    hit_count: u32,
}

impl TrackSlot {
    const EMPTY: Self = Self {
        active: false,
        state: TrackState::Tentative,
        id: 0,
        class_hash: 0,
        x_min_u16: 0,
        y_min_u16: 0,
        x_max_u16: 0,
        y_max_u16: 0,
        last_frame: 0,
        miss_count: 0,
        hit_count: 0,
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

/// Bounded ByteTrack-class IoU tracker over a frame sequence.
#[derive(Debug, Clone)]
pub struct BoundedTracker {
    slots: [TrackSlot; MAX_TRACKS],
    next_id: u32,
    /// Minimum IoU for high-score (first) association.
    pub iou_thresh: f32,
    /// Minimum IoU for low-score (second / recovery) association.
    pub low_iou_thresh: f32,
    /// Drop a **Confirmed** track after this many consecutive unmatched frames.
    pub max_miss: u32,
    /// Hits needed to promote Tentative → Confirmed.
    pub min_hits: u32,
    /// Score ≥ this → High tier (first match + birth).
    pub high_score_u16: u16,
    /// Low ≤ score < high → Low tier (recovery only).
    pub low_score_u16: u16,
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
            low_iou_thresh: 0.25,
            max_miss: 5,
            min_hits: DEFAULT_MIN_HITS,
            high_score_u16: DEFAULT_HIGH_SCORE_U16,
            low_score_u16: DEFAULT_LOW_SCORE_U16,
            class_gated: true,
        }
    }

    pub fn active_track_count(&self) -> usize {
        self.slots.iter().filter(|s| s.active).count()
    }

    pub fn confirmed_track_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.active && s.state == TrackState::Confirmed)
            .count()
    }

    /// Assign `track_id` on each of the first `n` detections in place.
    ///
    /// Unmatched / overflow / reject-tier detections get `track_id = 0`
    /// (and `FLAG_TRACK_OVERFLOW` when the table is full for a high-score birth).
    pub fn update(&mut self, frame_index: u32, detections: &mut [Detection], n: usize) {
        let n = n.min(detections.len()).min(MAX_ASSOC_DETS);

        // Score-descending order (selection sort — fixed buffer, deterministic).
        let mut order = [0u16; MAX_ASSOC_DETS];
        for i in 0..n {
            order[i] = i as u16;
        }
        for i in 0..n {
            let mut best = i;
            for j in (i + 1)..n {
                if detections[order[j] as usize].score_u16
                    > detections[order[best] as usize].score_u16
                {
                    best = j;
                }
            }
            order.swap(i, best);
        }

        // Snapshot track geometry for pure association helpers.
        let mut track_boxes = [Detection::empty(); MAX_TRACKS];
        let mut track_class = [0u64; MAX_TRACKS];
        let mut track_active = [false; MAX_TRACKS];
        for ti in 0..MAX_TRACKS {
            let slot = &self.slots[ti];
            if slot.active {
                track_active[ti] = true;
                track_class[ti] = slot.class_hash;
                track_boxes[ti] = slot.as_box();
            }
        }

        let mut track_used = [false; MAX_TRACKS];
        let mut det_matched = [false; MAX_ASSOC_DETS];
        let mut det_to_track = [NO_TRACK; MAX_ASSOC_DETS];
        let mut high_eligible = [false; MAX_ASSOC_DETS];
        let mut low_eligible = [false; MAX_ASSOC_DETS];

        for di in 0..n {
            let det = detections[di];
            if det.class_hash == 0 && det.score_u16 == 0 {
                continue;
            }
            match tier_of(det.score_u16, self.high_score_u16, self.low_score_u16) {
                ScoreTier::High => high_eligible[di] = true,
                ScoreTier::Low => low_eligible[di] = true,
                ScoreTier::Reject => {}
            }
        }

        // Stage 1 — high-score IoU match.
        let _ = assoc_iou(
            detections,
            &order,
            n,
            &high_eligible,
            &track_boxes,
            &track_class,
            &track_active,
            &mut track_used,
            self.class_gated,
            self.iou_thresh,
            &mut det_matched,
            &mut det_to_track,
        );

        // Stage 2 — low-score recovery of remaining tracks (no birth later).
        let _ = assoc_iou(
            detections,
            &order,
            n,
            &low_eligible,
            &track_boxes,
            &track_class,
            &track_active,
            &mut track_used,
            self.class_gated,
            self.low_iou_thresh,
            &mut det_matched,
            &mut det_to_track,
        );

        // Apply matches: update geometry, hits, lifecycle.
        for di in 0..n {
            if !det_matched[di] {
                continue;
            }
            let ti = det_to_track[di] as usize;
            if ti >= MAX_TRACKS || !self.slots[ti].active {
                continue;
            }
            let det = detections[di];
            let slot = &mut self.slots[ti];
            slot.x_min_u16 = det.x_min_u16;
            slot.y_min_u16 = det.y_min_u16;
            slot.x_max_u16 = det.x_max_u16;
            slot.y_max_u16 = det.y_max_u16;
            slot.last_frame = frame_index;
            slot.miss_count = 0;
            slot.hit_count = slot.hit_count.saturating_add(1);
            if slot.state == TrackState::Tentative && slot.hit_count >= self.min_hits {
                slot.state = TrackState::Confirmed;
            }
            detections[di].track_id = slot.id;
            detections[di].frame_index = frame_index;
        }

        // Birth: unmatched high-score detections only (score order).
        for oi in 0..n {
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
            // Low / Reject: no new track.
            if !high_eligible[di] {
                detections[di].track_id = 0;
                detections[di].frame_index = frame_index;
                continue;
            }
            if let Some(ti) = self.slots.iter().position(|s| !s.active) {
                let id = self.next_id;
                self.next_id = self.next_id.wrapping_add(1).max(1);
                self.slots[ti] = TrackSlot {
                    active: true,
                    state: if self.min_hits <= 1 {
                        TrackState::Confirmed
                    } else {
                        TrackState::Tentative
                    },
                    id,
                    class_hash: det.class_hash,
                    x_min_u16: det.x_min_u16,
                    y_min_u16: det.y_min_u16,
                    x_max_u16: det.x_max_u16,
                    y_max_u16: det.y_max_u16,
                    last_frame: frame_index,
                    miss_count: 0,
                    hit_count: 1,
                };
                detections[di].track_id = id;
                detections[di].frame_index = frame_index;
            } else {
                // Overflow: leave track_id 0, flag, do not evict.
                detections[di].track_id = 0;
                detections[di].frame_index = frame_index;
                detections[di].flags |= FLAG_TRACK_OVERFLOW;
            }
        }

        // Age unmatched tracks (skip matched and same-frame births).
        for (ti, slot) in self.slots.iter_mut().enumerate() {
            if !slot.active || track_used[ti] || slot.last_frame == frame_index {
                continue;
            }
            slot.miss_count = slot.miss_count.saturating_add(1);
            let expire = match slot.state {
                // Tentative dies on first miss (strict newborn gate).
                TrackState::Tentative => true,
                TrackState::Confirmed => slot.miss_count > self.max_miss,
            };
            if expire {
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
        // Second hit promotes Tentative → Confirmed when min_hits=2.
        assert_eq!(t.confirmed_track_count(), 1);
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
    fn two_objects_keep_ids_across_frames() {
        let mut t = BoundedTracker::new();
        let mut f0 = [
            box_at(0x11, 0, 0, 10_000, 10_000, 50_000),
            box_at(0x22, 40_000, 40_000, 55_000, 55_000, 49_000),
        ];
        t.update(0, &mut f0, 2);
        let id_a = f0[0].track_id;
        let id_b = f0[1].track_id;
        assert_ne!(id_a, 0);
        assert_ne!(id_b, 0);
        assert_ne!(id_a, id_b);

        // Move each slightly; identities must stick.
        let mut f1 = [
            box_at(0x11, 200, 100, 10_200, 10_100, 48_000),
            box_at(0x22, 40_200, 40_100, 55_200, 55_100, 47_000),
        ];
        t.update(1, &mut f1, 2);
        assert_eq!(f1[0].track_id, id_a);
        assert_eq!(f1[1].track_id, id_b);
        assert_eq!(t.active_track_count(), 2);
        assert_eq!(t.confirmed_track_count(), 2);
    }

    #[test]
    fn overflow_is_deterministic() {
        let mut t = BoundedTracker::new();
        t.max_miss = 100;
        t.min_hits = 1; // keep all slots Confirmed so empty frames do not purge tentative
                        // Fill all tracks with non-overlapping boxes (distinct classes, high score).
        for i in 0..MAX_TRACKS {
            let x = (i as u16).wrapping_mul(2000);
            let mut f = [box_at(
                0x100 + i as u64,
                x,
                0,
                x.saturating_add(500),
                500,
                40_000,
            )];
            t.update(i as u32, &mut f, 1);
            assert_ne!(f[0].track_id, 0, "slot {i}");
        }
        assert_eq!(t.active_track_count(), MAX_TRACKS);
        // One more high-score det must attempt birth → overflow (low-score never births).
        let mut extra = [box_at(0xDEAD, 60_000, 60_000, 65_000, 65_000, 45_000)];
        t.update(MAX_TRACKS as u32, &mut extra, 1);
        assert_eq!(extra[0].track_id, 0);
        assert_ne!(extra[0].flags & FLAG_TRACK_OVERFLOW, 0);
        // Second overflow still deterministic.
        let mut extra2 = [box_at(0xBEEF, 58_000, 58_000, 63_000, 63_000, 46_000)];
        t.update(MAX_TRACKS as u32 + 1, &mut extra2, 1);
        assert_eq!(extra2[0].track_id, 0);
        assert_ne!(extra2[0].flags & FLAG_TRACK_OVERFLOW, 0);
        assert_eq!(t.active_track_count(), MAX_TRACKS);
    }

    #[test]
    fn miss_expires_track() {
        let mut t = BoundedTracker::new();
        t.max_miss = 2;
        t.min_hits = 1; // confirmed immediately so we test confirmed aging
        let mut f0 = [box_at(0xBB, 1000, 1000, 4000, 4000, 50_000)];
        t.update(0, &mut f0, 1);
        assert_eq!(t.active_track_count(), 1);
        assert_eq!(t.confirmed_track_count(), 1);
        // Empty frames.
        let mut empty = [];
        t.update(1, &mut empty, 0);
        t.update(2, &mut empty, 0);
        t.update(3, &mut empty, 0);
        assert_eq!(t.active_track_count(), 0);
    }

    #[test]
    fn tentative_expires_on_first_miss() {
        let mut t = BoundedTracker::new();
        t.min_hits = 3;
        t.max_miss = 10;
        let mut f0 = [box_at(0xCC, 1000, 1000, 4000, 4000, 50_000)];
        t.update(0, &mut f0, 1);
        assert_eq!(t.active_track_count(), 1);
        assert_eq!(t.confirmed_track_count(), 0);
        let mut empty = [];
        t.update(1, &mut empty, 0);
        assert_eq!(
            t.active_track_count(),
            0,
            "tentative must die on first miss"
        );
    }

    #[test]
    fn low_score_recovers_unmatched_track() {
        let mut t = BoundedTracker::new();
        t.min_hits = 1;
        t.high_score_u16 = 40_000;
        t.low_score_u16 = 5_000;
        t.low_iou_thresh = 0.2;

        let mut f0 = [box_at(0xDD, 1000, 1000, 5000, 5000, 50_000)];
        t.update(0, &mut f0, 1);
        let id = f0[0].track_id;
        assert_ne!(id, 0);

        // High-score miss; low-score overlapping det should recover the track.
        let mut f1 = [box_at(0xDD, 1200, 1100, 5200, 5100, 10_000)];
        t.update(1, &mut f1, 1);
        assert_eq!(f1[0].track_id, id);
        assert_eq!(t.active_track_count(), 1);
    }

    #[test]
    fn low_score_does_not_spawn() {
        let mut t = BoundedTracker::new();
        t.high_score_u16 = 40_000;
        t.low_score_u16 = 5_000;
        let mut f0 = [box_at(0xEE, 1000, 1000, 5000, 5000, 10_000)];
        t.update(0, &mut f0, 1);
        assert_eq!(f0[0].track_id, 0);
        assert_eq!(t.active_track_count(), 0);
    }
}
