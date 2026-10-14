//! Principled Procedural Geometry & Surface Synthesis for `.10d` Living Containers.
//!
//! Replaces massive static geometry and texture assets with compact mathematical
//! recipes evaluated deterministically on the GPU / SIMD hot paths in zero heap:
//!
//! 1. **Cellular Voronoi Micro-Surfaces**: Organic scales, tree bark fissures, leaf venation,
//!    cracked mud, and corrosion pitting synthesized directly from a 64-bit seed.
//! 2. **Implicit SDF Smooth-Fillets**: Smooth-minimum blending $r$ for seamless branch-to-trunk
//!    unions and structural welded joints without hand-modeled seam meshes.
//! 3. **Allometric Growth Curves ($y = b \cdot x^k$) & Logarithmic Spirals ($r = a e^{k \theta}$)**:
//!    Horns, tusks, antlers, shells, and ontogenetic proportion changes governed by ODEs.
//! 4. **PGA Motor Oscillations**: Projective Geometric Algebra motors for secondary wind/motion
//!    foliage rustle, tail wag, and vehicle chassis sway.
//! 5. **CSG Stress-Tensor Cleavage**: Dynamic fracture along physical cleavage planes when
//!    tensile stress exceeds $\sigma_{\text{crit}}$, avoiding duplicate pre-fractured mesh assets.

use bytemuck::{Pod, Zeroable};

pub const PROCEDURAL_SYNTHESIS_SECTION_VERSION: u16 = 1;
pub const PROCEDURAL_SYNTHESIS_HEADER_SIZE: usize = 64;
pub const MAGIC_PROC: [u8; 4] = *b"PROC";

pub const FLAG_CELLULAR_VORONOI: u32 = 1 << 0;
pub const FLAG_IMPLICIT_FILLET: u32 = 1 << 1;
pub const FLAG_ALLOMETRIC_GROWTH: u32 = 1 << 2;
pub const FLAG_LOG_SPIRAL_ODE: u32 = 1 << 3;
pub const FLAG_PGA_MOTOR_DYNAMICS: u32 = 1 << 4;
pub const FLAG_STRESS_FRACTURE: u32 = 1 << 5;

/// 64-byte POD header encoding procedural synthesis recipes in SectionType::ProceduralSynthesis (18).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct ProceduralSynthesisHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub seed: u64,
    // Cellular Voronoi params
    pub voronoi_frequency: f32,
    pub voronoi_jitter: f32,
    // Implicit SDF fillet params
    pub fillet_blend_radius: f32,
    pub fillet_threshold: f32,
    // Allometric & Spiral params
    pub allometry_exponent_k: f32,
    pub allometry_base_b: f32,
    pub spiral_growth_rate_k: f32,
    pub spiral_initial_radius_a: f32,
    // PGA dynamics params
    pub motor_frequency_hz: f32,
    pub motor_damping_ratio: f32,
    // Fracture params
    pub critical_stress_threshold: f32,
    pub reserved_f32: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProceduralSynthesisError {
    PayloadTooShort { need: usize, have: usize },
    BadMagic([u8; 4]),
    UnsupportedVersion(u16),
    BufferTooSmall { need: usize, have: usize },
}

impl std::fmt::Display for ProceduralSynthesisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PayloadTooShort { need, have } => {
                write!(f, "procedural synthesis payload too short: have {have}, need {need}")
            }
            Self::BadMagic(m) => write!(f, "bad procedural synthesis magic: {m:?}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported procedural synthesis version {v}"),
            Self::BufferTooSmall { need, have } => {
                write!(f, "buffer too small: have {have}, need {need}")
            }
        }
    }
}

impl std::error::Error for ProceduralSynthesisError {}

/// Encode procedural synthesis header into a caller-supplied byte slice.
pub fn encode_procedural_synthesis_section(
    header: &ProceduralSynthesisHeader,
    out: &mut [u8],
) -> Result<usize, ProceduralSynthesisError> {
    if out.len() < PROCEDURAL_SYNTHESIS_HEADER_SIZE {
        return Err(ProceduralSynthesisError::BufferTooSmall {
            need: PROCEDURAL_SYNTHESIS_HEADER_SIZE,
            have: out.len(),
        });
    }
    let bytes = bytemuck::bytes_of(header);
    out[..PROCEDURAL_SYNTHESIS_HEADER_SIZE].copy_from_slice(bytes);
    Ok(PROCEDURAL_SYNTHESIS_HEADER_SIZE)
}

/// Decode procedural synthesis header from a byte slice. Zero heap.
pub fn decode_procedural_synthesis_section(
    payload: &[u8],
) -> Result<ProceduralSynthesisHeader, ProceduralSynthesisError> {
    if payload.len() < PROCEDURAL_SYNTHESIS_HEADER_SIZE {
        return Err(ProceduralSynthesisError::PayloadTooShort {
            need: PROCEDURAL_SYNTHESIS_HEADER_SIZE,
            have: payload.len(),
        });
    }
    let header: ProceduralSynthesisHeader =
        *bytemuck::from_bytes(&payload[..PROCEDURAL_SYNTHESIS_HEADER_SIZE]);

    if header.magic != MAGIC_PROC {
        return Err(ProceduralSynthesisError::BadMagic(header.magic));
    }
    if header.version != PROCEDURAL_SYNTHESIS_SECTION_VERSION {
        return Err(ProceduralSynthesisError::UnsupportedVersion(header.version));
    }

    Ok(header)
}

// ── Principled Zero-Heap Evaluators ─────────────────────────────────────────

/// Evaluate allometric morphological dimension according to Huxley's power equation:
/// $y = b \cdot x^k$, where $x$ is body size / developmental progress.
#[inline]
pub fn evaluate_allometric_dimension(body_scale: f32, base_b: f32, exponent_k: f32) -> f32 {
    if body_scale <= 0.0 {
        return 0.0;
    }
    base_b * body_scale.powf(exponent_k)
}

/// Evaluate logarithmic spiral ODE position $(x, y) = (r \cos\theta, r \sin\theta)$
/// where $r = a \cdot e^{k \cdot \theta}$. Governs horns, tusks, and ammonite shells.
#[inline]
pub fn evaluate_log_spiral_2d(theta: f32, initial_a: f32, growth_k: f32) -> (f32, f32) {
    let r = initial_a * (growth_k * theta).exp();
    let x = r * theta.cos();
    let y = r * theta.sin();
    (x, y)
}

/// Polynomial smooth-minimum blending (SDF implicit fillet) between two distances $d_1$ and $d_2$:
/// $smin(d_1, d_2, r) = \min(d_1, d_2) - \frac{h^2}{4r}$ where $h = \max(r - |d_1 - d_2|, 0)$.
#[inline]
pub fn evaluate_smooth_minimum_sdf(d1: f32, d2: f32, blend_radius: f32) -> f32 {
    if blend_radius <= 0.0 {
        return d1.min(d2);
    }
    let h = (blend_radius - (d1 - d2).abs()).max(0.0);
    d1.min(d2) - (h * h) / (4.0 * blend_radius)
}

/// Zero-heap 2D Voronoi cellular distance perturbation for procedural micro-surfaces.
/// Generates cell distance $F_1$ and cell hash from a coordinate and seed.
#[inline]
pub fn evaluate_voronoi_cellular_f1(u: f32, v: f32, seed: u64, frequency: f32) -> f32 {
    let su = u * frequency;
    let sv = v * frequency;
    let iu = su.floor() as i32;
    let iv = sv.floor() as i32;
    let fu = su.fract();
    let fv = sv.fract();

    let mut min_dist_sq = 100.0f32;

    // Search 3x3 local neighbor cells
    let mut di = -1;
    while di <= 1 {
        let mut dj = -1;
        while dj <= 1 {
            let cell_x = iu + di;
            let cell_y = iv + dj;

            // Deterministic integer FNV-style pseudo-random offset
            let h = ((cell_x as u64)
                .wrapping_mul(0x517cc1b727220a95)
                .wrapping_add((cell_y as u64).wrapping_mul(0x6c8e9cf570932b35))
                .wrapping_add(seed))
                ^ 0xbf58476d1ce4e5b9;

            let rx = ((h & 0xFFFF) as f32) / 65535.0;
            let ry = (((h >> 16) & 0xFFFF) as f32) / 65535.0;

            let dx = (di as f32) + rx - fu;
            let dy = (dj as f32) + ry - fv;
            let d_sq = dx * dx + dy * dy;

            if d_sq < min_dist_sq {
                min_dist_sq = d_sq;
            }
            dj += 1;
        }
        di += 1;
    }

    min_dist_sq.sqrt().min(1.0)
}

/// Projective Geometric Algebra (PGA) rotor oscillation displacement for secondary dynamics.
/// Evaluates dynamic sway vector from time, natural frequency, damping, and an external drive force.
#[inline]
pub fn evaluate_pga_motor_oscillation(
    rest_offset: [f32; 3],
    time_sec: f32,
    freq_hz: f32,
    damping: f32,
    drive_direction: [f32; 3],
) -> [f32; 3] {
    let omega = 2.0 * std::f32::consts::PI * freq_hz;
    let decay = (-damping * time_sec.max(0.0)).exp();
    let phase = (omega * time_sec).sin() * decay;

    [
        rest_offset[0] + drive_direction[0] * phase,
        rest_offset[1] + drive_direction[1] * phase,
        rest_offset[2] + drive_direction[2] * phase,
    ]
}

/// Evaluate structural fracture criterion under principal tensile stress $\sigma_1$.
/// Returns true if $\sigma_1 \ge \sigma_{\text{crit}}$ and determines the cleavage partition sign.
#[inline]
pub fn evaluate_stress_cleavage_partition(
    vertex_pos: [f32; 3],
    cleavage_plane_normal: [f32; 3],
    cleavage_plane_dist: f32,
    tensile_stress: f32,
    crit_threshold: f32,
) -> Option<bool> {
    if tensile_stress < crit_threshold {
        return None;
    }
    let dot = vertex_pos[0] * cleavage_plane_normal[0]
        + vertex_pos[1] * cleavage_plane_normal[1]
        + vertex_pos[2] * cleavage_plane_normal[2];
    Some(dot >= cleavage_plane_dist)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn procedural_synthesis_header_roundtrips() {
        let header = ProceduralSynthesisHeader {
            magic: MAGIC_PROC,
            version: 1,
            flags: (FLAG_CELLULAR_VORONOI | FLAG_ALLOMETRIC_GROWTH | FLAG_LOG_SPIRAL_ODE) as u16,
            seed: 0x1234_5678_9ABC_DEF0,
            voronoi_frequency: 16.0,
            voronoi_jitter: 0.85,
            fillet_blend_radius: 0.15,
            fillet_threshold: 0.05,
            allometry_exponent_k: 1.35,
            allometry_base_b: 0.42,
            spiral_growth_rate_k: 0.18,
            spiral_initial_radius_a: 0.10,
            motor_frequency_hz: 2.5,
            motor_damping_ratio: 0.12,
            critical_stress_threshold: 45.0,
            reserved_f32: 0.0,
        };

        let mut buffer = [0u8; 128];
        let bytes_written = encode_procedural_synthesis_section(&header, &mut buffer).unwrap();
        assert_eq!(bytes_written, PROCEDURAL_SYNTHESIS_HEADER_SIZE);

        let decoded = decode_procedural_synthesis_section(&buffer[..bytes_written]).unwrap();
        assert_eq!(decoded.magic, MAGIC_PROC);
        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.seed, 0x1234_5678_9ABC_DEF0);
        assert!((decoded.voronoi_frequency - 16.0).abs() < 1e-6);
        assert!((decoded.allometry_exponent_k - 1.35).abs() < 1e-6);
    }

    #[test]
    fn allometric_and_spiral_evaluations_are_deterministic() {
        let dim = evaluate_allometric_dimension(2.0, 0.5, 2.0);
        assert!((dim - 2.0).abs() < 1e-5); // 0.5 * 2^2 = 2.0

        let (x, y) = evaluate_log_spiral_2d(0.0, 1.0, 0.2);
        assert!((x - 1.0).abs() < 1e-5);
        assert!(y.abs() < 1e-5);

        let smin = evaluate_smooth_minimum_sdf(1.0, 2.0, 0.5);
        assert!(smin <= 1.0);

        let vor = evaluate_voronoi_cellular_f1(0.5, 0.5, 42, 10.0);
        assert!(vor >= 0.0 && vor <= 1.0);

        let pga = evaluate_pga_motor_oscillation([0.0, 1.0, 0.0], 0.25, 1.0, 0.0, [1.0, 0.0, 0.0]);
        assert_ne!(pga[0], 0.0);
        assert_eq!(pga[1], 1.0);
    }
}
