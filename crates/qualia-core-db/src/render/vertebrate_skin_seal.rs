//! Seal the anatomy accumulator's existing four-weight binding in SKL1.
use crate::container_10d::skeletal_container::attach_skeletal_section_into;
use crate::container_10d::skeletal_section::{encode_skeletal_section_into, ClipRecord, Counts, SkeletalData};
use crate::render::skeletal::{Key, SkinWeight, Track};
use super::authored_skin::AuthoredSkinBinding;
use crate::render::skeletal::fauna_rig::{FaunaSkeletalRig10d, CLIP_CRUISE_TROT_GLIDE, CLIP_SPRINT_HOP_DIVE};

pub(super) fn seal_skin(bytes: &[u8], rig: &FaunaSkeletalRig10d, authored: Option<&AuthoredSkinBinding>, indices: &[[u16; 4]], weights: &[[f32; 4]]) -> Result<Vec<u8>, String> {
    if indices.len() != weights.len() { return Err("skin stream length mismatch".into()); }
    let skin: Vec<_> = indices.iter().zip(weights).map(|(&joints, &weights)| SkinWeight { joints, weights }).collect();
    let joints = authored.map_or(rig.joints.as_slice(), |b| b.joints.as_slice());
    let count = joints.len();
    if authored.is_some_and(|b| b.cruise.len() != count || b.sprint.len() != count) {
        return Err("authored bind pose count mismatch".into());
    }
    let ids: Vec<_> = (0..count).map(|i| i as u64 + 1).collect();
    let tracks: Vec<_> = (0..count).map(|i| Track { joint: i as u16, first_key: (i * 4) as u32, key_count: 4 }).collect();
    // Existing archetype poses: rest, cruise, sprint/hop, return to rest.
    let keys: Vec<_> = (0..count).flat_map(|i| [
        Key { time: 0.0, local: joints[i].rest_local },
        Key { time: 1.0 / 3.0, local: authored.map_or_else(|| rig.clip_poses[CLIP_CRUISE_TROT_GLIDE][i], |b| b.cruise[i]) },
        Key { time: 2.0 / 3.0, local: authored.map_or_else(|| rig.clip_poses[CLIP_SPRINT_HOP_DIVE][i], |b| b.sprint[i]) },
        Key { time: 1.0, local: joints[i].rest_local },
    ]).collect();
    let clips = [ClipRecord { id: 1, duration: 1.0, first_track: 0, track_count: count as u32, looping: true }];
    let counts = Counts { joints: ids.len(), vertices: skin.len(), clips: 1, tracks: tracks.len(), keys: keys.len() };
    let mut payload = vec![0; counts.encoded_len().ok_or("skin exceeds SKL1 limits")?];
    encode_skeletal_section_into(&SkeletalData { joints, joint_ids: &ids, skin: &skin, clips: &clips, tracks: &tracks, keys: &keys }, &mut payload).map_err(|e| format!("invalid anatomy skin: {e:?}"))?;
    let capacity = bytes.len().checked_add(payload.len()).and_then(|n| n.checked_add(4096)).ok_or("container size overflow")?;
    if capacity > 42 * 1024 * 1024 { return Err("skeletal container exceeds budget".into()); }
    let mut out = vec![0; capacity];
    let len = attach_skeletal_section_into(bytes, &payload, &mut out).map_err(|e| format!("attach anatomy skin: {e:?}"))?;
    out.truncate(len);
    Ok(out)
}
