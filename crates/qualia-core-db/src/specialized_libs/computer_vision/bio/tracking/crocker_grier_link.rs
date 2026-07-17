//! Crocker–Grier style 2D particle linking across frames (trackpy / IDL lineage).
//!
//! Links detections `(x, y, frame)` with:
//! - **max distance** — Euclidean gate between consecutive frames
//! - **assignment** — greedy nearest-neighbour, or LAP-lite (cost-sorted edges)
//! - **memory / miss gap** — a track may skip up to `memory` frames without a hit
//! - **fixed max tracks** — overflow births rejected deterministically (`track_id = 0`)
//!
//! Pure Rust; fixed slot table + stack scratch on the link path.
//! Reference: Crocker & Grier, *J. Colloid Interface Sci.* (1996); trackpy `link`.

use super::particle_features::ParticleCentroid;

/// Maximum simultaneous active tracks.
pub const MAX_PARTICLE_TRACKS: usize = 64;

/// Max detections considered per frame (association scratch).
pub const MAX_FRAME_DETS: usize = 256;

/// Sentinel: detection not assigned a track.
pub const NO_TRACK_ID: u32 = 0;

/// One linked particle observation (detection + assigned track id).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkedParticle {
    pub x: f32,
    pub y: f32,
    pub frame: u32,
    pub area: u32,
    pub track_id: u32,
}

impl LinkedParticle {
    pub const EMPTY: Self = Self {
        x: 0.0,
        y: 0.0,
        frame: 0,
        area: 0,
        track_id: NO_TRACK_ID,
    };
}

impl Default for LinkedParticle {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Compact detection for the free-function linker (x, y, frame only).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Detection2 {
    pub x: f32,
    pub y: f32,
    pub frame: u32,
}

/// Per-detection link result for [`crocker_grier_link`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrackLink {
    pub track_id: u32,
    pub det_index: usize,
}

/// Parameters for Crocker–Grier linking.
#[derive(Debug, Clone, Copy)]
pub struct CrockerGrierParams {
    /// Maximum link distance (pixels) between candidate pairs.
    pub max_distance: f32,
    /// Frames a track may go unmatched before expiry (`memory` in trackpy).
    pub memory: u32,
    /// When true, use cost-sorted conflict resolution (LAP-lite); else pure greedy.
    pub use_lap_lite: bool,
}

impl Default for CrockerGrierParams {
    fn default() -> Self {
        Self {
            max_distance: 5.0,
            memory: 0,
            use_lap_lite: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TrackSlot {
    active: bool,
    id: u32,
    x: f32,
    y: f32,
    last_frame: u32,
    miss_count: u32,
}

impl TrackSlot {
    const EMPTY: Self = Self {
        active: false,
        id: 0,
        x: 0.0,
        y: 0.0,
        last_frame: 0,
        miss_count: 0,
    };
}

/// Bounded Crocker–Grier particle tracker.
#[derive(Debug, Clone)]
pub struct CrockerGrierLinker {
    slots: [TrackSlot; MAX_PARTICLE_TRACKS],
    next_id: u32,
    pub params: CrockerGrierParams,
}

impl Default for CrockerGrierLinker {
    fn default() -> Self {
        Self::new(CrockerGrierParams::default())
    }
}

impl CrockerGrierLinker {
    pub fn new(params: CrockerGrierParams) -> Self {
        Self {
            slots: [TrackSlot::EMPTY; MAX_PARTICLE_TRACKS],
            next_id: 1,
            params,
        }
    }

    pub fn active_track_count(&self) -> usize {
        self.slots.iter().filter(|s| s.active).count()
    }

    /// Assign track ids for one frame of centroids.
    ///
    /// `dets[0..n]` are particles in the current frame (any order).
    /// Writes into `out[0..n_out]` as [`LinkedParticle`]; returns count written
    /// (`min(n, out.len())`).
    pub fn update(
        &mut self,
        dets: &[ParticleCentroid],
        n: usize,
        out: &mut [LinkedParticle],
    ) -> usize {
        let n = n.min(dets.len()).min(MAX_FRAME_DETS).min(out.len());
        if n == 0 {
            return 0;
        }

        let max_d2 = self.params.max_distance * self.params.max_distance;

        let mut track_x = [0.0f32; MAX_PARTICLE_TRACKS];
        let mut track_y = [0.0f32; MAX_PARTICLE_TRACKS];
        let mut track_active = [false; MAX_PARTICLE_TRACKS];
        let mut track_last = [0u32; MAX_PARTICLE_TRACKS];
        for ti in 0..MAX_PARTICLE_TRACKS {
            let s = &self.slots[ti];
            if s.active {
                track_active[ti] = true;
                track_x[ti] = s.x;
                track_y[ti] = s.y;
                track_last[ti] = s.last_frame;
            }
        }

        let mut track_used = [false; MAX_PARTICLE_TRACKS];
        let mut det_matched = [false; MAX_FRAME_DETS];
        let mut det_to_track = [u16::MAX; MAX_FRAME_DETS];
        let memory = self.params.memory;

        if self.params.use_lap_lite {
            associate_lap_lite(
                dets,
                n,
                &track_x,
                &track_y,
                &track_active,
                &track_last,
                memory,
                max_d2,
                &mut track_used,
                &mut det_matched,
                &mut det_to_track,
            );
        } else {
            associate_greedy(
                dets,
                n,
                &track_x,
                &track_y,
                &track_active,
                &track_last,
                memory,
                max_d2,
                &mut track_used,
                &mut det_matched,
                &mut det_to_track,
            );
        }

        for di in 0..n {
            if !det_matched[di] {
                continue;
            }
            let ti = det_to_track[di] as usize;
            if ti >= MAX_PARTICLE_TRACKS || !self.slots[ti].active {
                continue;
            }
            let d = dets[di];
            let slot = &mut self.slots[ti];
            slot.x = d.x;
            slot.y = d.y;
            slot.last_frame = d.frame;
            slot.miss_count = 0;
            out[di] = LinkedParticle {
                x: d.x,
                y: d.y,
                frame: d.frame,
                area: d.area,
                track_id: slot.id,
            };
        }

        for di in 0..n {
            if det_matched[di] {
                continue;
            }
            let d = dets[di];
            if let Some(ti) = self.slots.iter().position(|s| !s.active) {
                let id = self.next_id;
                self.next_id = self.next_id.wrapping_add(1).max(1);
                self.slots[ti] = TrackSlot {
                    active: true,
                    id,
                    x: d.x,
                    y: d.y,
                    last_frame: d.frame,
                    miss_count: 0,
                };
                out[di] = LinkedParticle {
                    x: d.x,
                    y: d.y,
                    frame: d.frame,
                    area: d.area,
                    track_id: id,
                };
            } else {
                out[di] = LinkedParticle {
                    x: d.x,
                    y: d.y,
                    frame: d.frame,
                    area: d.area,
                    track_id: NO_TRACK_ID,
                };
            }
        }

        let mut max_frame = dets[0].frame;
        for di in 1..n {
            if dets[di].frame > max_frame {
                max_frame = dets[di].frame;
            }
        }
        for (ti, slot) in self.slots.iter_mut().enumerate() {
            if !slot.active || track_used[ti] {
                continue;
            }
            let gap = max_frame.saturating_sub(slot.last_frame);
            if gap == 0 {
                continue;
            }
            slot.miss_count = gap;
            if slot.miss_count > memory {
                *slot = TrackSlot::EMPTY;
            }
        }

        n
    }

    /// Age all tracks for an empty frame (no detections).
    pub fn age_empty_frame(&mut self, frame: u32) {
        let memory = self.params.memory;
        for slot in self.slots.iter_mut() {
            if !slot.active {
                continue;
            }
            let gap = frame.saturating_sub(slot.last_frame);
            if gap > memory {
                *slot = TrackSlot::EMPTY;
            } else {
                slot.miss_count = gap;
            }
        }
    }

    /// Link a multi-frame detection list. Groups by frame, runs [`Self::update`] per group.
    pub fn link_sequence(
        &mut self,
        dets: &[ParticleCentroid],
        n: usize,
        out: &mut [LinkedParticle],
    ) -> usize {
        let n = n.min(dets.len()).min(out.len());
        if n == 0 {
            return 0;
        }

        let mut frames = [0u32; MAX_FRAME_DETS];
        let mut n_frames = 0usize;
        for i in 0..n {
            let f = dets[i].frame;
            let mut seen = false;
            for j in 0..n_frames {
                if frames[j] == f {
                    seen = true;
                    break;
                }
            }
            if !seen && n_frames < MAX_FRAME_DETS {
                frames[n_frames] = f;
                n_frames += 1;
            }
        }
        for i in 0..n_frames {
            let mut best = i;
            for j in (i + 1)..n_frames {
                if frames[j] < frames[best] {
                    best = j;
                }
            }
            frames.swap(i, best);
        }

        let mut total = 0usize;
        let mut frame_buf = [ParticleCentroid::EMPTY; MAX_FRAME_DETS];
        let mut link_buf = [LinkedParticle::EMPTY; MAX_FRAME_DETS];

        for fi in 0..n_frames {
            let f = frames[fi];
            let mut m = 0usize;
            let mut src_idx = [0u16; MAX_FRAME_DETS];
            for i in 0..n {
                if dets[i].frame == f && m < MAX_FRAME_DETS {
                    frame_buf[m] = dets[i];
                    src_idx[m] = i as u16;
                    m += 1;
                }
            }
            let linked = self.update(&frame_buf, m, &mut link_buf);
            for k in 0..linked {
                let oi = src_idx[k] as usize;
                if oi < out.len() {
                    out[oi] = link_buf[k];
                    total = total.saturating_add(1);
                }
            }
        }
        total
    }
}

/// Greedy: for each detection (index order), take nearest free track under max_d2.
fn associate_greedy(
    dets: &[ParticleCentroid],
    n: usize,
    track_x: &[f32; MAX_PARTICLE_TRACKS],
    track_y: &[f32; MAX_PARTICLE_TRACKS],
    track_active: &[bool; MAX_PARTICLE_TRACKS],
    track_last: &[u32; MAX_PARTICLE_TRACKS],
    memory: u32,
    max_d2: f32,
    track_used: &mut [bool; MAX_PARTICLE_TRACKS],
    det_matched: &mut [bool],
    det_to_track: &mut [u16],
) {
    for di in 0..n {
        let d = dets[di];
        let mut best_ti: Option<usize> = None;
        let mut best_d2 = max_d2 + 1.0; // exclusive upper; accept d2 <= max_d2
        for ti in 0..MAX_PARTICLE_TRACKS {
            if !track_active[ti] || track_used[ti] {
                continue;
            }
            let gap = d.frame.saturating_sub(track_last[ti]);
            if gap < 1 || gap > memory + 1 {
                continue;
            }
            let dx = d.x - track_x[ti];
            let dy = d.y - track_y[ti];
            let d2 = dx * dx + dy * dy;
            if d2 <= max_d2 && d2 < best_d2 {
                best_d2 = d2;
                best_ti = Some(ti);
            }
        }
        if let Some(ti) = best_ti {
            track_used[ti] = true;
            det_matched[di] = true;
            det_to_track[di] = ti as u16;
        }
    }
}

/// LAP-lite: collect candidate edges, sort by cost, greedily accept non-conflicting.
fn associate_lap_lite(
    dets: &[ParticleCentroid],
    n: usize,
    track_x: &[f32; MAX_PARTICLE_TRACKS],
    track_y: &[f32; MAX_PARTICLE_TRACKS],
    track_active: &[bool; MAX_PARTICLE_TRACKS],
    track_last: &[u32; MAX_PARTICLE_TRACKS],
    memory: u32,
    max_d2: f32,
    track_used: &mut [bool; MAX_PARTICLE_TRACKS],
    det_matched: &mut [bool],
    det_to_track: &mut [u16],
) {
    const MAX_EDGES: usize = MAX_FRAME_DETS * 8;
    let mut costs = [f32::MAX; MAX_EDGES];
    let mut edge_di = [0u16; MAX_EDGES];
    let mut edge_ti = [0u16; MAX_EDGES];
    let mut n_edges = 0usize;

    for di in 0..n {
        let d = dets[di];
        for ti in 0..MAX_PARTICLE_TRACKS {
            if !track_active[ti] {
                continue;
            }
            let gap = d.frame.saturating_sub(track_last[ti]);
            if gap < 1 || gap > memory + 1 {
                continue;
            }
            let dx = d.x - track_x[ti];
            let dy = d.y - track_y[ti];
            let d2 = dx * dx + dy * dy;
            if d2 <= max_d2 && n_edges < MAX_EDGES {
                costs[n_edges] = d2;
                edge_di[n_edges] = di as u16;
                edge_ti[n_edges] = ti as u16;
                n_edges += 1;
            }
        }
    }

    let mut order = [0u16; MAX_EDGES];
    for i in 0..n_edges {
        order[i] = i as u16;
    }
    for i in 0..n_edges {
        let mut best = i;
        for j in (i + 1)..n_edges {
            if costs[order[j] as usize] < costs[order[best] as usize] {
                best = j;
            }
        }
        order.swap(i, best);
    }

    for oi in 0..n_edges {
        let e = order[oi] as usize;
        let di = edge_di[e] as usize;
        let ti = edge_ti[e] as usize;
        if det_matched[di] || track_used[ti] {
            continue;
        }
        track_used[ti] = true;
        det_matched[di] = true;
        det_to_track[di] = ti as u16;
    }
}

/// One-shot convenience: link a full detection list with fresh linker state.
pub fn link_particles(
    dets: &[ParticleCentroid],
    n: usize,
    params: CrockerGrierParams,
    out: &mut [LinkedParticle],
) -> usize {
    let mut linker = CrockerGrierLinker::new(params);
    linker.link_sequence(dets, n, out)
}

/// Free-function Crocker–Grier link over compact [`Detection2`] rows.
///
/// Defaults: `memory = 1` (allows one skipped frame), greedy assignment.
/// Writes `out[i]` for each input detection; returns `dets.len()` on success.
pub fn crocker_grier_link(
    dets: &[Detection2],
    max_dist: f32,
    out: &mut [TrackLink],
) -> Result<usize, crate::specialized_libs::computer_vision::cv::error::CvError> {
    if out.len() < dets.len() {
        return Err(crate::specialized_libs::computer_vision::cv::error::CvError::BufferTooSmall);
    }
    let n = dets.len().min(MAX_FRAME_DETS);
    if n == 0 {
        return Ok(0);
    }
    let mut cents = [ParticleCentroid::EMPTY; MAX_FRAME_DETS];
    for i in 0..n {
        cents[i] = ParticleCentroid {
            x: dets[i].x,
            y: dets[i].y,
            frame: dets[i].frame,
            area: 1,
            label: 0,
        };
    }
    let mut linked = [LinkedParticle::EMPTY; MAX_FRAME_DETS];
    let params = CrockerGrierParams {
        max_distance: max_dist,
        memory: 1,
        use_lap_lite: false,
    };
    let got = link_particles(&cents, n, params, &mut linked);
    for i in 0..got {
        out[i] = TrackLink {
            track_id: linked[i].track_id,
            det_index: i,
        };
    }
    // Zero remaining out slots when n < out.len() is caller's concern.
    Ok(got)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32, frame: u32) -> ParticleCentroid {
        ParticleCentroid {
            x,
            y,
            frame,
            area: 1,
            label: 0,
        }
    }

    #[test]
    fn same_particle_keeps_track_id() {
        let dets = [p(10.0, 10.0, 0), p(11.0, 10.5, 1), p(12.0, 11.0, 2)];
        let mut out = [LinkedParticle::EMPTY; 3];
        let n = link_particles(
            &dets,
            3,
            CrockerGrierParams {
                max_distance: 3.0,
                memory: 0,
                use_lap_lite: false,
            },
            &mut out,
        );
        assert_eq!(n, 3);
        assert_ne!(out[0].track_id, NO_TRACK_ID);
        assert_eq!(out[0].track_id, out[1].track_id);
        assert_eq!(out[1].track_id, out[2].track_id);
    }

    #[test]
    fn two_particles_two_tracks() {
        let dets = [
            p(0.0, 0.0, 0),
            p(100.0, 100.0, 0),
            p(1.0, 0.0, 1),
            p(101.0, 100.0, 1),
        ];
        let mut out = [LinkedParticle::EMPTY; 4];
        let n = link_particles(
            &dets,
            4,
            CrockerGrierParams {
                max_distance: 5.0,
                memory: 0,
                use_lap_lite: false,
            },
            &mut out,
        );
        assert_eq!(n, 4);
        assert_ne!(out[0].track_id, out[1].track_id);
        assert_eq!(out[0].track_id, out[2].track_id);
        assert_eq!(out[1].track_id, out[3].track_id);
    }

    #[test]
    fn far_detection_starts_new_track() {
        let dets = [p(0.0, 0.0, 0), p(100.0, 100.0, 1)];
        let mut out = [LinkedParticle::EMPTY; 2];
        let n = link_particles(
            &dets,
            2,
            CrockerGrierParams {
                max_distance: 5.0,
                memory: 0,
                use_lap_lite: false,
            },
            &mut out,
        );
        assert_eq!(n, 2);
        assert_ne!(out[0].track_id, out[1].track_id);
        assert_ne!(out[0].track_id, NO_TRACK_ID);
        assert_ne!(out[1].track_id, NO_TRACK_ID);
    }

    #[test]
    fn memory_gap_relinks() {
        let dets = [p(0.0, 0.0, 0), p(1.0, 0.0, 2)];
        let mut out = [LinkedParticle::EMPTY; 2];
        let n = link_particles(
            &dets,
            2,
            CrockerGrierParams {
                max_distance: 5.0,
                memory: 1,
                use_lap_lite: false,
            },
            &mut out,
        );
        assert_eq!(n, 2);
        assert_eq!(out[0].track_id, out[1].track_id);
    }

    #[test]
    fn memory_zero_expires_on_skip() {
        let dets = [p(0.0, 0.0, 0), p(1.0, 0.0, 2)];
        let mut out = [LinkedParticle::EMPTY; 2];
        let n = link_particles(
            &dets,
            2,
            CrockerGrierParams {
                max_distance: 5.0,
                memory: 0,
                use_lap_lite: false,
            },
            &mut out,
        );
        assert_eq!(n, 2);
        assert_ne!(out[0].track_id, out[1].track_id);
    }

    #[test]
    fn lap_lite_resolves_conflict() {
        let dets = [
            p(0.0, 0.0, 0),
            p(10.0, 0.0, 0),
            p(1.0, 0.0, 1),
            p(9.0, 0.0, 1),
        ];
        let mut out = [LinkedParticle::EMPTY; 4];
        let n = link_particles(
            &dets,
            4,
            CrockerGrierParams {
                max_distance: 5.0,
                memory: 0,
                use_lap_lite: true,
            },
            &mut out,
        );
        assert_eq!(n, 4);
        assert_eq!(out[0].track_id, out[2].track_id);
        assert_eq!(out[1].track_id, out[3].track_id);
        assert_ne!(out[0].track_id, out[1].track_id);
    }

    #[test]
    fn overflow_is_deterministic() {
        let mut linker = CrockerGrierLinker::new(CrockerGrierParams {
            max_distance: 0.5,
            memory: 100,
            use_lap_lite: false,
        });
        let mut dets = [ParticleCentroid::EMPTY; MAX_PARTICLE_TRACKS + 1];
        for i in 0..MAX_PARTICLE_TRACKS {
            dets[i] = p((i as f32) * 10.0, 0.0, 0);
        }
        let mut out = [LinkedParticle::EMPTY; MAX_PARTICLE_TRACKS + 1];
        let n = linker.update(&dets, MAX_PARTICLE_TRACKS, &mut out);
        assert_eq!(n, MAX_PARTICLE_TRACKS);
        assert_eq!(linker.active_track_count(), MAX_PARTICLE_TRACKS);
        for i in 0..MAX_PARTICLE_TRACKS {
            assert_ne!(out[i].track_id, NO_TRACK_ID);
        }
        let extra = [p(10_000.0, 10_000.0, 1)];
        let mut out2 = [LinkedParticle::EMPTY; 1];
        linker.update(&extra, 1, &mut out2);
        assert_eq!(out2[0].track_id, NO_TRACK_ID);
        assert_eq!(linker.active_track_count(), MAX_PARTICLE_TRACKS);
    }

    #[test]
    fn empty_sequence_zero() {
        let dets: [ParticleCentroid; 0] = [];
        let mut out = [LinkedParticle::EMPTY; 1];
        assert_eq!(
            link_particles(&dets, 0, CrockerGrierParams::default(), &mut out),
            0
        );
    }

    #[test]
    fn age_empty_frame_expires() {
        let mut linker = CrockerGrierLinker::new(CrockerGrierParams {
            max_distance: 5.0,
            memory: 1,
            use_lap_lite: false,
        });
        let dets = [p(0.0, 0.0, 0)];
        let mut out = [LinkedParticle::EMPTY; 1];
        linker.update(&dets, 1, &mut out);
        assert_eq!(linker.active_track_count(), 1);
        linker.age_empty_frame(1);
        assert_eq!(linker.active_track_count(), 1);
        linker.age_empty_frame(2);
        assert_eq!(linker.active_track_count(), 0);
    }

    #[test]
    fn free_fn_links_moving_point() {
        let dets = [
            Detection2 {
                x: 0.0,
                y: 0.0,
                frame: 0,
            },
            Detection2 {
                x: 1.0,
                y: 0.0,
                frame: 1,
            },
            Detection2 {
                x: 2.0,
                y: 0.0,
                frame: 2,
            },
        ];
        let mut out = [TrackLink::default(); 3];
        crocker_grier_link(&dets, 2.0, &mut out).unwrap();
        assert_eq!(out[0].track_id, out[1].track_id);
        assert_eq!(out[1].track_id, out[2].track_id);
        assert_ne!(out[0].track_id, NO_TRACK_ID);
    }
}
