//! SPARQL-MM spatial+temporal query over vision OBSERVATION quins (D4.03).
//!
//! Composes the vision-native semantic engine — it does NOT reimplement region /
//! frame filtering. Two existing primitives are joined on the instance hash:
//!
//!   * [`crate::semantic::query_by_frame_range`]     — temporal filter over
//!     `VisualObservation` quins (frame index packed in metadata bits 16..48);
//!     each surviving quin carries `object = instance_hash`.
//!   * [`crate::semantic::query_instances_in_region`] — spatial filter over
//!     `hasBoundingBox` quins (box packed in the object payload); each surviving
//!     hit yields `subject = instance_hash`.
//!
//! An observation is "in the window" iff its instance appears in BOTH result sets.
//!
//! ## Why the vision-native route (not `SparqlMmHandler`)
//! `qualia_core_db::sparql_library::sparql_mm::SparqlMmHandler` already recognises
//! the `hasBoundingBox` / `hasTrackId` predicate hashes this layer emits, and a
//! `&[VisionQuin]` could be reinterpreted as `&[NQuin]` (both are 6×u64 `#[repr(C)]`).
//! Its TEMPORAL dimension, however, keys off `ma-ont#mediaTimeMs` / `hasStartTime`
//! payloads in the *object* field — whereas vision observations carry the frame
//! index in *metadata*. Routing the temporal half through `SparqlMmHandler` would
//! therefore silently match nothing (or require inventing media-time edges we do
//! not have). The vision-native primitives model the frame semantics exactly, so
//! they are the route that composes correctly AND compiles cleanly. Documented per
//! the task's "prefer the one that compiles cleanly" instruction.
//!
//! ## Template-leak safety
//! `VisionQuin` holds OBSERVATIONS, never biometric templates. This query returns
//! only instance/subject IDENTITY hashes (`u64` node identifiers) and a count — it
//! never emits an `object` payload (which is where a packed box, or in other quin
//! families a template digest, would live). Callers cannot recover template bytes
//! through this surface.

use crate::semantic::{query_by_frame_range, query_instances_in_region, VisionQuin};

/// Result of a spatial+temporal observation query: a count plus the instance
/// (subject) hashes that fall inside the window. Never contains object payloads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObsQueryResult {
    /// Number of distinct in-window observation instances.
    pub count: usize,
    /// Distinct instance/subject identity hashes inside the zone+frame window.
    pub instance_hashes: Vec<u64>,
}

/// Count + identify observation instances whose bounding box intersects the zone
/// `[x0,y0,x1,y1]` AND whose `VisualObservation` frame is in `[frame_start,
/// frame_end]`.
///
/// Returns only instance hashes and a count — never template/object-payload bytes.
pub fn faces_in_zone_time(
    obs: &[VisionQuin],
    x0: u16,
    y0: u16,
    x1: u16,
    y1: u16,
    frame_start: u32,
    frame_end: u32,
) -> ObsQueryResult {
    if obs.is_empty() {
        return ObsQueryResult::default();
    }

    // Temporal half: observation quins whose frame is in range. Each carries
    // object = instance_hash (media --VisualObservation--> instance).
    let mut frame_hits = vec![VisionQuin::with_parity(0, 0, 0, 0, 0); obs.len()];
    let n_frame = query_by_frame_range(obs, frame_start, frame_end, &mut frame_hits);

    // Spatial half: instance hashes whose bounding box intersects the zone
    // (bbox quin subject = instance_hash).
    let mut region_inst = vec![0u64; obs.len()];
    let n_region = query_instances_in_region(obs, x0, y0, x1, y1, &mut region_inst);

    // Join on instance hash: keep instances present in BOTH halves. We only ever
    // move `subject`/`object` identity hashes here — no object payload escapes.
    let mut instance_hashes: Vec<u64> = Vec::new();
    for &inst in &region_inst[..n_region] {
        let in_window = frame_hits[..n_frame].iter().any(|q| q.object == inst);
        if in_window && !instance_hashes.contains(&inst) {
            instance_hashes.push(inst);
        }
    }

    ObsQueryResult {
        count: instance_hashes.len(),
        instance_hashes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{bbox_quin, observation_quin, pack_bbox_u64, MediaDigest};
    use crate::types::Detection;

    fn det(instance: u64, x0: u16, y0: u16, x1: u16, y1: u16, frame: u32) -> Detection {
        Detection {
            class_hash: 0xC1A55,
            instance_hash: instance,
            score_u16: 40_000,
            x_min_u16: x0,
            y_min_u16: y0,
            x_max_u16: x1,
            y_max_u16: y1,
            frame_index: frame,
            track_id: 0,
            flags: 0,
        }
    }

    /// Build the two quins the query joins on (observation + bbox) for one det.
    fn quins_for(media: MediaDigest, d: &Detection, model: u64) -> [VisionQuin; 2] {
        [observation_quin(media, d, model), bbox_quin(d, model)]
    }

    #[test]
    fn counts_only_in_zone_and_in_frame() {
        let media = MediaDigest {
            hash: 0xF00D,
            byte_len: 10,
        };
        let model = 0x42;

        // In zone (top-left quadrant), in frame window.
        let a = det(0xAAAA, 100, 100, 4000, 4000, 5);
        // In zone, but OUT of frame window.
        let b = det(0xBBBB, 200, 200, 4000, 4000, 999);
        // Out of zone (far bottom-right), in frame window.
        let c = det(0xCCCC, 60000, 60000, 65000, 65000, 6);
        // In zone, in frame window — second valid hit.
        let e = det(0xEEEE, 500, 500, 3000, 3000, 7);

        let mut all: Vec<VisionQuin> = Vec::new();
        for d in [&a, &b, &c, &e] {
            all.extend_from_slice(&quins_for(media, d, model));
        }

        // Zone = top-left region; frame window [0,100].
        let r = faces_in_zone_time(&all, 0, 0, 10000, 10000, 0, 100);

        assert_eq!(r.count, 2, "only A and E are in zone AND in frame window");
        assert!(r.instance_hashes.contains(&0xAAAA));
        assert!(r.instance_hashes.contains(&0xEEEE));
        assert!(!r.instance_hashes.contains(&0xBBBB), "B excluded by frame");
        assert!(!r.instance_hashes.contains(&0xCCCC), "C excluded by zone");
    }

    #[test]
    fn returns_no_template_or_object_payload_bytes() {
        let media = MediaDigest {
            hash: 0x1234,
            byte_len: 5,
        };
        let model = 7;
        let a = det(0xAAAA, 100, 100, 4000, 4000, 3);

        let mut all: Vec<VisionQuin> = Vec::new();
        all.extend_from_slice(&quins_for(media, &a, model));

        let r = faces_in_zone_time(&all, 0, 0, 10000, 10000, 0, 100);
        assert_eq!(r.count, 1);

        // The ONLY value returned is the instance identity hash — never the packed
        // bbox object payload (the field where template-like bytes would live).
        let packed_box = pack_bbox_u64(&a);
        assert!(
            !r.instance_hashes.contains(&packed_box),
            "packed object payload must never appear in results"
        );
        for &h in &r.instance_hashes {
            assert_eq!(h, 0xAAAA, "results are subject/instance hashes only");
        }
    }

    #[test]
    fn empty_input_is_empty_result() {
        let r = faces_in_zone_time(&[], 0, 0, 100, 100, 0, 10);
        assert_eq!(r.count, 0);
        assert!(r.instance_hashes.is_empty());
    }
}
