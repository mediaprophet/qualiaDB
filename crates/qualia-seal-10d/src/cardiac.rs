//! Reduced-order cardiac-cycle SKL1 (Arts/Bovendeerd kinematics, vibe `heartbeat_pulse` envelope).

use qualia_core_db::container_10d::skeletal_section::{
    encode_skeletal_section_into, ClipRecord, Counts, SkeletalData,
};
use qualia_core_db::q_hash;
use qualia_core_db::render::skeletal::{Joint, Key, Motor, SkinWeight, Track};

use super::SealError;

const BASE: &str = "urn:etalon:reconstruct:joint:base";
const APEX: &str = "urn:etalon:reconstruct:joint:apex";
const LV: &str = "urn:etalon:reconstruct:joint:lv";
const RV: &str = "urn:etalon:reconstruct:joint:rv";
const LA: &str = "urn:etalon:reconstruct:joint:la";
const RA: &str = "urn:etalon:reconstruct:joint:ra";
const BEAT: &str = "urn:etalon:reconstruct:clip:beat";

pub const BEAT_DURATION: f64 = 0.85;
const KEY_TIMES: [f64; 7] = [0.0, 0.06, 0.12, 0.22, 0.28, 0.34, 0.85];
const LONGITUDINAL: f64 = 0.15;
const RADIAL: f64 = 0.22;
const APEX_TORSION: f64 = 15.0 * std::f64::consts::PI / 180.0;
const BASE_TORSION: f64 = -8.0 * std::f64::consts::PI / 180.0;

pub fn encode_cardiac_skl1(
    positions: &[[f32; 3]],
    min: [f32; 3],
    max: [f32; 3],
) -> Result<Vec<u8>, SealError> {
    let vertex_count = positions.len();
    if vertex_count == 0 {
        return Err(SealError::EmptyMesh);
    }
    let counts = Counts {
        joints: 6,
        vertices: vertex_count,
        clips: 1,
        tracks: 6,
        keys: 6 * KEY_TIMES.len(),
    };
    let len = counts
        .encoded_len()
        .ok_or_else(|| SealError::Skeletal("vertex/joint/clip counts out of range".into()))?;

    let identity = Motor::identity();
    let joints = [
        Joint {
            parent: None,
            rest_local: identity,
            inverse_bind: identity,
        },
        Joint {
            parent: Some(0),
            rest_local: identity,
            inverse_bind: identity,
        },
        Joint {
            parent: Some(0),
            rest_local: identity,
            inverse_bind: identity,
        },
        Joint {
            parent: Some(0),
            rest_local: identity,
            inverse_bind: identity,
        },
        Joint {
            parent: Some(0),
            rest_local: identity,
            inverse_bind: identity,
        },
        Joint {
            parent: Some(0),
            rest_local: identity,
            inverse_bind: identity,
        },
    ];
    let joint_ids = [
        q_hash(BASE),
        q_hash(APEX),
        q_hash(LV),
        q_hash(RV),
        q_hash(LA),
        q_hash(RA),
    ];
    let size = [
        (max[0] - min[0]).max(1.0e-3) as f64,
        (max[1] - min[1]).max(1.0e-3) as f64,
        (max[2] - min[2]).max(1.0e-3) as f64,
    ];
    let skin: Vec<SkinWeight> = positions
        .iter()
        .map(|p| skin_vertex(*p, min, max))
        .collect();
    let clips = [ClipRecord {
        id: q_hash(BEAT),
        duration: BEAT_DURATION,
        first_track: 0,
        track_count: 6,
        looping: true,
    }];
    let tracks: [Track; 6] = std::array::from_fn(|j| Track {
        joint: j as u16,
        first_key: (j * KEY_TIMES.len()) as u32,
        key_count: KEY_TIMES.len() as u16,
    });
    let mut keys = Vec::with_capacity(6 * KEY_TIMES.len());
    for joint in 0..6 {
        for &t in &KEY_TIMES {
            let env = heartbeat_envelope(t);
            keys.push(Key {
                time: t,
                local: motor_for_joint(joint, env, size),
            });
        }
    }
    let data = SkeletalData {
        joints: &joints,
        joint_ids: &joint_ids,
        skin: &skin,
        clips: &clips,
        tracks: &tracks,
        keys: &keys,
    };
    let mut payload = vec![0u8; len];
    let written = encode_skeletal_section_into(&data, &mut payload)
        .map_err(|e| SealError::Skeletal(format!("encode: {e:?}")))?;
    payload.truncate(written);
    Ok(payload)
}

pub fn heartbeat_envelope(t: f64) -> f64 {
    let cycle = t.rem_euclid(BEAT_DURATION);
    let s1 = if cycle < 0.12 {
        (cycle / 0.12 * std::f64::consts::PI).sin()
    } else {
        0.0
    };
    let s2 = if cycle > 0.22 && cycle < 0.34 {
        ((cycle - 0.22) / 0.12 * std::f64::consts::PI).sin() * 0.6
    } else {
        0.0
    };
    s1 + s2
}

fn motor_for_joint(joint: usize, env: f64, size: [f64; 3]) -> Motor {
    let long = LONGITUDINAL * size[1] * env;
    let rad_x = RADIAL * size[0] * 0.5 * env;
    let rad_z = RADIAL * size[2] * 0.5 * env;
    match joint {
        0 => rotation_y(BASE_TORSION * env),
        1 => Motor::from_rotation_translation(
            quat_y(APEX_TORSION * env),
            [0.0, long, 0.0],
        ),
        2 => Motor::from_translation(-rad_x, long * 0.35, 0.0),
        3 => Motor::from_translation(rad_x * 0.8, long * 0.25, 0.0),
        4 => Motor::from_translation(-rad_x * 0.25, long * 0.1, rad_z * 0.2),
        _ => Motor::from_translation(rad_x * 0.2, long * 0.08, rad_z * 0.15),
    }
}

fn rotation_y(angle: f64) -> Motor {
    Motor::from_rotation_translation(quat_y(angle), [0.0, 0.0, 0.0])
}

fn quat_y(angle: f64) -> [f64; 4] {
    let h = angle * 0.5;
    [h.cos(), 0.0, h.sin(), 0.0]
}

fn skin_vertex(p: [f32; 3], min: [f32; 3], max: [f32; 3]) -> SkinWeight {
    let mut t = [0.0f32; 3];
    for i in 0..3 {
        let span = (max[i] - min[i]).max(1.0e-6);
        t[i] = ((p[i] - min[i]) / span).clamp(0.0, 1.0);
    }
    let mut w = [
        t[1].powf(1.15),
        (1.0 - t[1]).powf(1.35),
        t[0].powf(1.1),
        (1.0 - t[0]).powf(1.1),
    ];
    let sum = w[0] + w[1] + w[2] + w[3];
    if sum > 1.0e-8 {
        for wi in &mut w {
            *wi /= sum;
        }
    } else {
        w = [1.0, 0.0, 0.0, 0.0];
    }
    SkinWeight {
        joints: [0, 1, 2, 3],
        weights: w,
    }
}
