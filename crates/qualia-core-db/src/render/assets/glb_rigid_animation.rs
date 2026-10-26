//! Exact SKL1-compatible glTF channels. Unsupported interpolation is refused.
use super::rigid_skin::RigidSkin;
use super::{
    AssetError, ClipRecord, GlbInterpolation, GlbMorphWeightTrack, GlbNodeRecord, Key, Motor, Track,
};
use serde_json::Value;
use std::collections::BTreeMap;

struct Channel<const N: usize> {
    times: Vec<f32>,
    values: Vec<[f64; N]>,
}
#[derive(Default)]
struct Channels {
    translation: Option<Channel<3>>,
    rotation: Option<Channel<4>>,
}
fn bad(message: &str) -> AssetError {
    AssetError::Parse(message.into())
}
fn varying<const N: usize>(values: &[[f64; N]]) -> bool {
    values
        .windows(2)
        .any(|pair| pair[0].iter().zip(pair[1]).any(|(a, b)| *a != b))
}
fn channel_times(accessor: &Value, views: &[Value], bin: &[u8]) -> Result<Vec<f32>, AssetError> {
    if accessor["componentType"].as_u64() != Some(5126)
        || accessor["type"].as_str() != Some("SCALAR")
    {
        return Err(bad("animation times require FLOAT SCALAR"));
    }
    let times = super::read_f32_scalars(accessor, views, bin)?;
    if times.is_empty()
        || times.len() > crate::render::skeletal::MAX_KEYS
        || times
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0 || *v > 86400.0)
        || times.windows(2).any(|w| w[0] >= w[1])
    {
        return Err(bad("animation times must be finite, nonnegative and strictly increasing within native bounds"));
    }
    Ok(times)
}

pub(super) fn parse_animations(
    gltf: &Value,
    nodes: &[GlbNodeRecord],
    skin: &RigidSkin,
    accessors: &[Value],
    views: &[Value],
    bin: &[u8],
) -> Result<
    (
        Vec<ClipRecord>,
        Vec<Track>,
        Vec<Key>,
        Vec<GlbMorphWeightTrack>,
    ),
    AssetError,
> {
    let animations = gltf["animations"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if animations.len() > crate::container_10d::skeletal_section::MAX_CLIPS {
        return Err(bad("glTF animations exceed native clip budget"));
    }
    let (mut clips, mut tracks, mut keys, mut morphs) = (vec![], vec![], vec![], vec![]);
    for (index, animation) in animations.iter().enumerate() {
        let samplers = animation["samplers"]
            .as_array()
            .ok_or_else(|| bad("animation samplers missing"))?;
        let channels = animation["channels"]
            .as_array()
            .ok_or_else(|| bad("animation channels missing"))?;
        let mut mapped = BTreeMap::<u16, Channels>::new();
        let mut duration = 0.0f64;
        for channel in channels {
            let node = channel["target"]["node"]
                .as_u64()
                .and_then(|v| usize::try_from(v).ok())
                .filter(|v| *v < nodes.len())
                .ok_or_else(|| bad("animation node missing"))?;
            let sampler = samplers
                .get(
                    channel["sampler"]
                        .as_u64()
                        .ok_or_else(|| bad("animation sampler missing"))?
                        as usize,
                )
                .ok_or_else(|| bad("animation sampler missing"))?;
            let input = accessors
                .get(
                    sampler["input"]
                        .as_u64()
                        .ok_or_else(|| bad("animation input missing"))?
                        as usize,
                )
                .ok_or_else(|| bad("animation input missing"))?;
            let output = accessors
                .get(
                    sampler["output"]
                        .as_u64()
                        .ok_or_else(|| bad("animation output missing"))?
                        as usize,
                )
                .ok_or_else(|| bad("animation output missing"))?;
            let times = channel_times(input, views, bin)?;
            duration = duration.max(*times.last().unwrap() as f64);
            let interpolation = match sampler["interpolation"].as_str().unwrap_or("LINEAR") {
                "LINEAR" => GlbInterpolation::Linear,
                "STEP" => GlbInterpolation::Step,
                "CUBICSPLINE" => GlbInterpolation::CubicSpline,
                _ => return Err(bad("unknown glTF interpolation")),
            };
            let path = channel["target"]["path"]
                .as_str()
                .ok_or_else(|| bad("animation target path missing"))?;
            if path == "weights" {
                let target_count = gltf["nodes"][node]["mesh"]
                    .as_u64()
                    .and_then(|mesh| gltf["meshes"][mesh as usize]["primitives"].as_array())
                    .and_then(|primitives| primitives.first())
                    .and_then(|p| p["targets"].as_array())
                    .map_or(0, Vec::len);
                let values = super::read_f32_scalars(output, views, bin)?;
                let samples = if interpolation == GlbInterpolation::CubicSpline {
                    3
                } else {
                    1
                };
                if target_count == 0
                    || target_count > 16
                    || values.len() != times.len() * target_count * samples
                    || values.iter().any(|v| !v.is_finite())
                {
                    return Err(bad("morph animation dimensions or values invalid"));
                }
                morphs.push(GlbMorphWeightTrack {
                    node_index: node as u16,
                    interpolation,
                    times,
                    values,
                });
                continue;
            }
            if interpolation != GlbInterpolation::Linear {
                return Err(bad("STEP/CUBICSPLINE joint channels require interpolation-preserving native tracks"));
            }
            let joint = *skin.node_to_joint.get(&node).ok_or_else(|| {
                bad("animated non-joint nodes require node-animation preserving native tracks")
            })?;
            let mapped = mapped.entry(joint).or_default();
            match path {
                "translation" => {
                    if mapped.translation.is_some() {
                        return Err(bad("duplicate glTF joint translation channel"));
                    }
                    super::rigid_skin::checked_accessor(
                        output,
                        views,
                        bin,
                        12,
                        crate::render::skeletal::MAX_KEYS,
                    )?;
                    let values = super::read_vec3(output, views, bin, "translation")?
                        .into_iter()
                        .map(|v| v.map(f64::from))
                        .collect::<Vec<_>>();
                    if values.len() != times.len()
                        || values.iter().flatten().any(|v| !v.is_finite())
                    {
                        return Err(bad(
                            "translation values differ from time count or are non-finite",
                        ));
                    }
                    mapped.translation = Some(Channel { times, values });
                }
                "rotation" => {
                    if mapped.rotation.is_some() {
                        return Err(bad("duplicate glTF joint rotation channel"));
                    }
                    super::rigid_skin::checked_accessor(
                        output,
                        views,
                        bin,
                        16,
                        crate::render::skeletal::MAX_KEYS,
                    )?;
                    let raw = super::read_vec4(output, views, bin, "rotation")?;
                    if raw.len() != times.len() {
                        return Err(bad("rotation values differ from time count"));
                    }
                    let mut values = Vec::with_capacity(raw.len());
                    for q in raw {
                        let q = [q[3] as f64, q[0] as f64, q[1] as f64, q[2] as f64];
                        let length = q.iter().map(|v| v * v).sum::<f64>().sqrt();
                        if !length.is_finite() || (length - 1.0).abs() > 1e-5 {
                            return Err(bad("animation rotation is not a finite unit quaternion"));
                        }
                        let mut q = q.map(|v| v / length);
                        if values.last().is_some_and(|previous: &[f64; 4]| {
                            previous.iter().zip(q).map(|(a, b)| a * b).sum::<f64>() < 0.0
                        }) {
                            q = q.map(|v| -v);
                        }
                        values.push(q);
                    }
                    mapped.rotation = Some(Channel { times, values });
                }
                "scale" => {
                    return Err(bad(
                        "joint scale animation requires a scale-preserving native skeletal profile",
                    ))
                }
                _ => return Err(bad("unknown glTF animation target path")),
            }
        }
        if skin.joints.is_empty() {
            continue;
        }
        append_clip(
            skin,
            &mapped,
            crate::q_hash(&format!(
                "urn:qualia:clip:{index}:{}",
                animation["name"].as_str().unwrap_or("clip")
            )),
            duration.max(1e-3),
            &mut clips,
            &mut tracks,
            &mut keys,
        )?;
    }
    if clips.is_empty() && !skin.joints.is_empty() {
        append_clip(
            skin,
            &BTreeMap::new(),
            crate::q_hash("urn:qualia:clip:default-pose"),
            1.0,
            &mut clips,
            &mut tracks,
            &mut keys,
        )?;
    }
    Ok((clips, tracks, keys, morphs))
}

fn append_clip(
    skin: &RigidSkin,
    channels: &BTreeMap<u16, Channels>,
    id: u64,
    duration: f64,
    clips: &mut Vec<ClipRecord>,
    tracks: &mut Vec<Track>,
    keys: &mut Vec<Key>,
) -> Result<(), AssetError> {
    let first_track = tracks.len() as u32;
    for joint in 0..skin.joints.len() {
        let (default_rotation, default_translation) =
            skin.node_local[joint].to_rotation_translation();
        let channels = channels.get(&(joint as u16));
        let translation = channels.and_then(|c| c.translation.as_ref());
        let rotation = channels.and_then(|c| c.rotation.as_ref());
        let translation_varies = translation.is_some_and(|c| varying(&c.values));
        let rotation_varies = rotation.is_some_and(|c| varying(&c.values));
        if translation_varies && rotation_varies {
            return Err(bad("combined varying translation/rotation requires glTF interpolation preserving native tracks, not implicit motor Sclerp"));
        }
        // A fixed non-joint ancestor prefix is composed after direct-node
        // interpolation, preserving that authored transform for every key.
        let times = if translation_varies {
            &translation.unwrap().times
        } else if rotation_varies {
            &rotation.unwrap().times
        } else {
            static ZERO: [f32; 1] = [0.0];
            &ZERO[..]
        };
        if keys
            .len()
            .checked_add(times.len())
            .is_none_or(|n| n > crate::render::skeletal::MAX_KEYS)
        {
            return Err(bad("glTF motor keys exceed native budget"));
        }
        let first_key = keys.len() as u32;
        for (index, &time) in times.iter().enumerate() {
            let translation = translation.map_or(default_translation, |c| {
                c.values[if translation_varies { index } else { 0 }]
            });
            let rotation = rotation.map_or(default_rotation, |c| {
                c.values[if rotation_varies { index } else { 0 }]
            });
            keys.push(Key {
                time: time as f64,
                local: skin.channel_prefix[joint]
                    .mul(&Motor::from_rotation_translation(rotation, translation))
                    .normalize(),
            });
        }
        tracks.push(Track {
            joint: joint as u16,
            first_key,
            key_count: times.len() as u16,
        });
    }
    clips.push(ClipRecord {
        id,
        duration,
        first_track,
        track_count: skin.joints.len() as u32,
        looping: false,
    });
    Ok(())
}
