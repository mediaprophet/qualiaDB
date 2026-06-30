//! Ambient particle instances derived from the resident 10D tensor (CPU side of the field).
use super::*;
pub(super) fn particles_from_tensor(
    bytes: &[u8],
    cap: usize,
) -> Result<Vec<ParticleInstance>, String> {
    let count = tensor_node_count(bytes).map_err(|e| e.to_string())?;
    if count == 0 {
        return Ok(Vec::new());
    }
    let step = (count / cap).max(1);
    let mut out = Vec::with_capacity(cap.min(count));
    for i in (0..count).step_by(step) {
        let t = read_tensor_at(bytes, i).map_err(|e| e.to_string())?;
        out.push(ParticleInstance {
            position: [t.x, t.y, t.z],
            epistemic_q: t.q,
        });
        if out.len() >= cap {
            break;
        }
    }
    Ok(out)
}

pub(super) fn generate_particles(count: usize) -> Vec<ParticleInstance> {
    let mut out = Vec::with_capacity(count);
    let mut seed: u32 = 0xC0FFEE_u32;
    for _ in 0..count {
        seed = lcg(seed);
        let x = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
        seed = lcg(seed);
        let y = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
        seed = lcg(seed);
        let z = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
        out.push(ParticleInstance {
            position: [x, y, z],
            epistemic_q: 0.0,
        });
    }
    out
}

#[inline]
pub(super) fn lcg(seed: u32) -> u32 {
    seed.wrapping_mul(1_103_515_245).wrapping_add(12_345)
}

#[inline]
pub fn particle_cap_for_mode(mode: OperationalMode, tier: u8) -> usize {
    if tier < 2 {
        return 0;
    }
    // Buffer is always allocated at Full capacity; ledger throttles draw instances.
    let _ = mode;
    MAX_AMBIENT_INSTANCES
}
