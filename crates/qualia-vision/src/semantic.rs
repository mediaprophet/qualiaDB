//! Observation → fixed NQuin-compatible records (epistemic, not ground truth).
//!
//! Layout matches Qualia `NQuin` (6 × u64). We keep a local definition so
//! Phase-1 does not force a full `qualia-core-db` link for every UI binary;
//! field meanings are identical and can be cast/transmuted at the core boundary.
//!
//! V6: human reject/correct, bbox/track predicates, full compile (up to 4 quins/det).

use crate::types::Detection;

/// Predicate / context IRIs (hashed with FNV-1a 64, same family as Qualia `q_hash`).
pub const P_VISUAL_OBSERVATION: &str = "https://ns.webizen.org/q42/VisualObservation";
pub const P_PROPOSES_CLASS: &str = "https://ns.webizen.org/q42/proposesClass";
pub const P_HAS_BBOX: &str = "https://ns.webizen.org/q42/hasBoundingBox";
pub const P_HAS_TRACK: &str = "https://ns.webizen.org/q42/hasTrackId";
pub const P_HUMAN_REJECTS: &str = "https://ns.webizen.org/q42/humanRejects";
pub const P_HUMAN_CORRECTS: &str = "https://ns.webizen.org/q42/humanCorrectsClass";
pub const P_MODEL_DIGEST: &str = "https://ns.webizen.org/q42/modelDigest";
pub const CTX_VISION: &str = "https://ns.webizen.org/q42/vision-observation";
pub const CTX_HUMAN_ATTESTATION: &str = "https://ns.webizen.org/q42/human-attestation";

/// Max observation quins emitted per batch (up to 4 per detection in full compile).
pub const MAX_OBS_QUINS: usize = 256;

/// 48-byte semantic quin (parity = XOR fold of other fields).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisionQuin {
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    pub context: u64,
    pub metadata: u64,
    pub parity: u64,
}

impl VisionQuin {
    #[inline]
    pub fn with_parity(
        subject: u64,
        predicate: u64,
        object: u64,
        context: u64,
        metadata: u64,
    ) -> Self {
        let parity = subject ^ predicate ^ object ^ context ^ metadata;
        Self {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        }
    }
}

/// Content digest of media bytes (FNV-1a over up to 64 KiB prefix + length).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaDigest {
    pub hash: u64,
    pub byte_len: u64,
}

/// FNV-1a 64 — matches Qualia URI hashing family.
#[inline]
pub fn q_hash(s: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x100_0000_01b3;
    let mut h = FNV_OFFSET;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

#[inline]
pub fn q_hash_bytes(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x100_0000_01b3;
    let mut h = FNV_OFFSET;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

pub fn media_digest(bytes: &[u8]) -> MediaDigest {
    let take = bytes.len().min(65_536);
    let mut h = q_hash_bytes(&bytes[..take]);
    // Mix length so different sizes of same prefix differ.
    h ^= (bytes.len() as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    MediaDigest {
        hash: h,
        byte_len: bytes.len() as u64,
    }
}

/// One observation quin: media --VisualObservation--> instance, score in metadata low 16.
pub fn observation_quin(media: MediaDigest, det: &Detection, model_hash: u64) -> VisionQuin {
    let subject = media.hash;
    let predicate = q_hash(P_VISUAL_OBSERVATION);
    let object = det.instance_hash;
    let context = q_hash(CTX_VISION) ^ model_hash;
    // metadata: score | frame in high bits (simple packing).
    let metadata = (det.score_u16 as u64)
        | ((det.frame_index as u64) << 16)
        | ((det.flags as u64) << 48);
    VisionQuin::with_parity(subject, predicate, object, context, metadata)
}

/// Class proposal quin: instance --proposesClass--> class_hash.
pub fn class_proposal_quin(det: &Detection, model_hash: u64) -> VisionQuin {
    let subject = det.instance_hash;
    let predicate = q_hash(P_PROPOSES_CLASS);
    let object = det.class_hash;
    let context = q_hash(CTX_VISION) ^ model_hash;
    let metadata = det.score_u16 as u64;
    VisionQuin::with_parity(subject, predicate, object, context, metadata)
}

/// Pack normalized u16 bbox into a single u64 (x0|y0|x1|y1 as 16-bit lanes).
#[inline]
pub fn pack_bbox_u64(det: &Detection) -> u64 {
    (det.x_min_u16 as u64)
        | ((det.y_min_u16 as u64) << 16)
        | ((det.x_max_u16 as u64) << 32)
        | ((det.y_max_u16 as u64) << 48)
}

#[inline]
pub fn unpack_bbox_u64(v: u64) -> (u16, u16, u16, u16) {
    (
        (v & 0xFFFF) as u16,
        ((v >> 16) & 0xFFFF) as u16,
        ((v >> 32) & 0xFFFF) as u16,
        ((v >> 48) & 0xFFFF) as u16,
    )
}

/// instance --hasBoundingBox--> packed box (metadata: score).
pub fn bbox_quin(det: &Detection, model_hash: u64) -> VisionQuin {
    let subject = det.instance_hash;
    let predicate = q_hash(P_HAS_BBOX);
    let object = pack_bbox_u64(det);
    let context = q_hash(CTX_VISION) ^ model_hash;
    let metadata = det.score_u16 as u64 | ((det.frame_index as u64) << 16);
    VisionQuin::with_parity(subject, predicate, object, context, metadata)
}

/// instance --hasTrackId--> track_id (0 if untracked).
pub fn track_quin(det: &Detection, model_hash: u64) -> VisionQuin {
    let subject = det.instance_hash;
    let predicate = q_hash(P_HAS_TRACK);
    let object = det.track_id as u64;
    let context = q_hash(CTX_VISION) ^ model_hash;
    let metadata = det.frame_index as u64;
    VisionQuin::with_parity(subject, predicate, object, context, metadata)
}

/// Detailed human-readable semantic description of an object detection.
pub fn format_detection_text_description(det: &Detection, class_name: &str) -> String {
    let score_pct = (det.score_u16 as f32 / 65535.0) * 100.0;
    let x_min = (det.x_min_u16 as f32 / 65535.0) * 100.0;
    let y_min = (det.y_min_u16 as f32 / 65535.0) * 100.0;
    let w = ((det.x_max_u16.saturating_sub(det.x_min_u16)) as f32 / 65535.0) * 100.0;
    let h = ((det.y_max_u16.saturating_sub(det.y_min_u16)) as f32 / 65535.0) * 100.0;

    let track_info = if det.track_id > 0 {
        format!(" (Track #{})", det.track_id)
    } else {
        String::new()
    };

    format!(
        "Detected '{class_name}' (confidence {score_pct:.1}%){track_info} at bbox [x: {x_min:.1}%, y: {y_min:.1}%, w: {w:.1}%, h: {h:.1}%] in frame {}",
        det.frame_index
    )
}

/// RDF Turtle representation of a detected object instance.
pub fn format_detection_turtle_rdf(det: &Detection, class_name: &str, media_hash: u64) -> String {
    let score = det.score_u16 as f32 / 65535.0;
    let x_min = det.x_min_u16 as f32 / 65535.0;
    let y_min = det.y_min_u16 as f32 / 65535.0;
    let x_max = det.x_max_u16 as f32 / 65535.0;
    let y_max = det.y_max_u16 as f32 / 65535.0;

    format!(
        r#"@prefix q42: <https://ns.webizen.org/q42/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix schema: <http://schema.org/> .
@prefix prov: <http://www.w3.org/ns/prov#> .

<urn:q42:instance:{inst_hash:016x}> a q42:DetectedObject ;
    rdfs:label "{class_name}" ;
    schema:confidence {score:.4} ;
    q42:classHash "{class_hash:016x}" ;
    q42:frameIndex {frame_index} ;
    q42:trackId {track_id} ;
    q42:boundingBox [
        q42:xMin {x_min:.4} ;
        q42:yMin {y_min:.4} ;
        q42:xMax {x_max:.4} ;
        q42:yMax {y_max:.4}
    ] ;
    prov:wasDerivedFrom <urn:q42:media:{media_hash:016x}> .
"#,
        inst_hash = det.instance_hash,
        class_name = class_name,
        score = score,
        class_hash = det.class_hash,
        frame_index = det.frame_index,
        track_id = det.track_id,
        x_min = x_min,
        y_min = y_min,
        x_max = x_max,
        y_max = y_max,
        media_hash = media_hash
    )
}

/// Human rejects a machine observation instance (does not erase the machine claim).
/// subject = human_did_hash, object = instance_hash that is rejected.
pub fn human_reject_quin(human_did_hash: u64, instance_hash: u64, reason_hash: u64) -> VisionQuin {
    VisionQuin::with_parity(
        human_did_hash,
        q_hash(P_HUMAN_REJECTS),
        instance_hash,
        q_hash(CTX_HUMAN_ATTESTATION),
        reason_hash,
    )
}

/// Human corrects class: subject = human, object = new_class_hash;
/// metadata low 64 bits of original instance for linkage (instance in context xor).
pub fn human_correct_quin(
    human_did_hash: u64,
    instance_hash: u64,
    new_class_hash: u64,
) -> VisionQuin {
    VisionQuin::with_parity(
        human_did_hash,
        q_hash(P_HUMAN_CORRECTS),
        new_class_hash,
        q_hash(CTX_HUMAN_ATTESTATION) ^ instance_hash,
        instance_hash,
    )
}

/// media --modelDigest--> model_hash (provenance of which model produced claims).
pub fn model_digest_quin(media: MediaDigest, model_hash: u64) -> VisionQuin {
    VisionQuin::with_parity(
        media.hash,
        q_hash(P_MODEL_DIGEST),
        model_hash,
        q_hash(CTX_VISION),
        media.byte_len,
    )
}

/// Compile detections into observation + class quins (epistemic layer).
///
/// Returns number of quins written. Each detection contributes up to 2 quins.
pub fn compile_observation_quins(
    media: MediaDigest,
    detections: &[Detection],
    model_hash: u64,
    out: &mut [VisionQuin],
) -> usize {
    let mut w = 0usize;
    for det in detections {
        if det.class_hash == 0 && det.score_u16 == 0 {
            continue;
        }
        if w + 2 > out.len() {
            break;
        }
        out[w] = observation_quin(media, det, model_hash);
        w += 1;
        out[w] = class_proposal_quin(det, model_hash);
        w += 1;
    }
    w
}

/// Full epistemic compile: model digest + per-det (observation, class, bbox, track).
/// Machine claims are never erased by later human reject/correct (separate context).
pub fn compile_observation_quins_full(
    media: MediaDigest,
    detections: &[Detection],
    model_hash: u64,
    out: &mut [VisionQuin],
) -> usize {
    let mut w = 0usize;
    if out.is_empty() {
        return 0;
    }
    out[w] = model_digest_quin(media, model_hash);
    w += 1;
    for det in detections {
        if det.class_hash == 0 && det.score_u16 == 0 {
            continue;
        }
        if w + 4 > out.len() {
            break;
        }
        out[w] = observation_quin(media, det, model_hash);
        w += 1;
        out[w] = class_proposal_quin(det, model_hash);
        w += 1;
        out[w] = bbox_quin(det, model_hash);
        w += 1;
        out[w] = track_quin(det, model_hash);
        w += 1;
    }
    w
}

/// Region query over compiled observation quins: keep instances whose bbox
/// intersects the query box (normalized u16). Writes matching instance hashes.
pub fn query_instances_in_region(
    quins: &[VisionQuin],
    x0: u16,
    y0: u16,
    x1: u16,
    y1: u16,
    out_instances: &mut [u64],
) -> usize {
    let mut w = 0usize;
    let pred = q_hash(P_HAS_BBOX);
    for q in quins {
        if q.predicate != pred {
            continue;
        }
        let (bx0, by0, bx1, by1) = unpack_bbox_u64(q.object);
        if boxes_intersect(x0, y0, x1, y1, bx0, by0, bx1, by1) {
            if w >= out_instances.len() {
                break;
            }
            // Dedupe instance
            let inst = q.subject;
            if out_instances[..w].contains(&inst) {
                continue;
            }
            out_instances[w] = inst;
            w += 1;
        }
    }
    w
}

/// Filter observation quins by media time / frame range (frame in observation metadata bits 16..48).
pub fn query_by_frame_range(
    quins: &[VisionQuin],
    frame_min: u32,
    frame_max: u32,
    out: &mut [VisionQuin],
) -> usize {
    let pred_obs = q_hash(P_VISUAL_OBSERVATION);
    let mut w = 0usize;
    for q in quins {
        if q.predicate != pred_obs {
            continue;
        }
        let frame = ((q.metadata >> 16) & 0xFFFF_FFFF) as u32;
        if frame >= frame_min && frame <= frame_max {
            if w >= out.len() {
                break;
            }
            out[w] = *q;
            w += 1;
        }
    }
    w
}

/// Filter by model hash mixed into context (context = CTX_VISION ^ model_hash).
pub fn query_by_model(
    quins: &[VisionQuin],
    model_hash: u64,
    out: &mut [VisionQuin],
) -> usize {
    let ctx = q_hash(CTX_VISION) ^ model_hash;
    let mut w = 0usize;
    for q in quins {
        if q.context == ctx || (q.predicate == q_hash(P_MODEL_DIGEST) && q.object == model_hash)
        {
            if w >= out.len() {
                break;
            }
            out[w] = *q;
            w += 1;
        }
    }
    w
}

#[inline]
fn boxes_intersect(
    ax0: u16,
    ay0: u16,
    ax1: u16,
    ay1: u16,
    bx0: u16,
    by0: u16,
    bx1: u16,
    by1: u16,
) -> bool {
    ax0 < bx1 && ax1 > bx0 && ay0 < by1 && ay1 > by0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Detection;

    #[test]
    fn parity_valid() {
        let d = Detection {
            class_hash: 0xABC,
            instance_hash: 0xDEF,
            score_u16: 40_000,
            x_min_u16: 0,
            y_min_u16: 0,
            x_max_u16: 65535,
            y_max_u16: 65535,
            frame_index: 0,
            track_id: 0,
            flags: Detection::FLAG_REFERENCE_BACKEND,
        };
        let m = MediaDigest {
            hash: 0x1111,
            byte_len: 100,
        };
        let q = observation_quin(m, &d, 0x42);
        assert_eq!(
            q.parity,
            q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata
        );
    }

    #[test]
    fn compile_two_per_detection() {
        let mut out = [VisionQuin::with_parity(0, 0, 0, 0, 0); 8];
        let d = Detection {
            class_hash: 1,
            instance_hash: 2,
            score_u16: 1000,
            ..Detection::empty()
        };
        let n = compile_observation_quins(
            MediaDigest {
                hash: 9,
                byte_len: 1,
            },
            &[d],
            7,
            &mut out,
        );
        assert_eq!(n, 2);
    }

    #[test]
    fn full_compile_includes_bbox_and_track() {
        let d = Detection {
            class_hash: 1,
            instance_hash: 2,
            score_u16: 1000,
            x_min_u16: 100,
            y_min_u16: 200,
            x_max_u16: 3000,
            y_max_u16: 4000,
            frame_index: 3,
            track_id: 7,
            flags: 0,
        };
        let media = MediaDigest {
            hash: 9,
            byte_len: 1,
        };
        let mut out = [VisionQuin::with_parity(0, 0, 0, 0, 0); 16];
        let n = compile_observation_quins_full(media, &[d], 7, &mut out);
        assert_eq!(n, 5); // digest + 4 per det
        assert_eq!(out[0].predicate, q_hash(P_MODEL_DIGEST));

        let mut inst = [0u64; 4];
        let qn = query_instances_in_region(&out[..n], 0, 0, 5000, 5000, &mut inst);
        assert_eq!(qn, 1);
        assert_eq!(inst[0], 2);

        let rej = human_reject_quin(0xD1D_u64, d.instance_hash, 0);
        assert_eq!(rej.predicate, q_hash(P_HUMAN_REJECTS));
        let corr = human_correct_quin(0xD1D_u64, d.instance_hash, 99);
        assert_eq!(corr.object, 99);
        // Machine claim still present after human acts.
        assert!(out[..n]
            .iter()
            .any(|q| q.predicate == q_hash(P_PROPOSES_CLASS)));
        let _ = corr;
    }

    #[test]
    fn pack_unpack_bbox_roundtrip() {
        let d = Detection {
            x_min_u16: 1,
            y_min_u16: 2,
            x_max_u16: 3,
            y_max_u16: 4,
            ..Detection::empty()
        };
        let p = pack_bbox_u64(&d);
        assert_eq!(unpack_bbox_u64(p), (1, 2, 3, 4));
    }

    #[test]
    fn test_detection_semantic_formatting() {
        let d = Detection {
            class_hash: 0x1234,
            instance_hash: 0x5678,
            score_u16: 60000,
            x_min_u16: 6553,
            y_min_u16: 13107,
            x_max_u16: 39321,
            y_max_u16: 52428,
            frame_index: 12,
            track_id: 3,
            flags: 0,
        };

        let desc = format_detection_text_description(&d, "person");
        assert!(desc.contains("Detected 'person'"));
        assert!(desc.contains("Track #3"));
        assert!(desc.contains("frame 12"));

        let ttl = format_detection_turtle_rdf(&d, "person", 0x9999);
        assert!(ttl.contains("a q42:DetectedObject"));
        assert!(ttl.contains("rdfs:label \"person\""));
        assert!(ttl.contains("prov:wasDerivedFrom <urn:q42:media:0000000000009999>"));
    }
}
