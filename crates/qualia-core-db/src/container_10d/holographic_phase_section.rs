//! Holographic Phase-Space Manifold Codec & Zero-Heap Evaluators (`HOLO`, `0x0306`).
//!
//! Restores the foundational architecture defined in:
//! - *Holographic 3D Engine for Semantic Graphs* (12-triangle proxy bounding lenses,
//!   $<32$-step SDF raymarching, and N3Logic wave interference)
//! - *Holographic 10D Engine Architecture Design* (Hamiltonian Stationary-Action
//!   Boundary-Value Splines $\delta S = 0$, Poincaré-ball hyperbolic $v=2$ Möbius
//!   morphogenesis, golden-angle Bessel phyllotaxis, scale-space harmonic LODs $k=0..K$,
//!   Eulerian Spectral Streamlines $(\Phi, \Psi)$, Drucker-Prager / Herschel-Bulkley
//!   continuum rheology, Exner sediment conservation, and the 16-KB `Q42ConstituentLutEntry`
//!   GPU L1 cache bridge).
//!
//! All structs are 64-byte cache-line aligned (`#[repr(C, align(64))]`, `Pod + Zeroable`)
//! and map 1:1 to WGSL `array<vec4<u32>, 4>` storage buffers with zero heap allocations.

use bytemuck::{bytes_of, pod_read_unaligned, Pod, Zeroable};
use half::f16;

use crate::container_10d::crc32c::crc32c;
use crate::q42::m64_matter::{
    M64HoloRecord, M64PhaseJunctureRecord, M64StrataColumnRecord, Q42ConstituentLutEntry,
};
use crate::render::assets::Mesh;

/// 32-bit binary format discriminant for `HOLO` phase-space payloads (`0x4F4C_4F48` LE).
pub const HOLOGRAPHIC_PHASE_TAG: [u8; 4] = *b"HOLO";
pub const HOLOGRAPHIC_PHASE_VERSION: u16 = 1;
pub const HOLOGRAPHIC_PHASE_HEADER_SIZE: usize = 64;
pub const EULERIAN_WAVEFIELD_RECORD_SIZE: usize = 64;
pub const MAX_HOLO_SEGMENTS: usize = 4096;
pub const MAX_EULERIAN_WAVEFIELDS: usize = 1024;
/// Strict TDR-safe raymarching step cap mandated by Doc 1 & Doc 3 (`step < 32u`).
pub const HOLOGRAPHIC_MAX_RAYMARCH_STEPS: u32 = 32;
/// Golden angle $\omega_\phi = 2\pi(1 - 1/\varphi) \approx 2.3999632\text{ rad}$ ($137.50776^\circ$) for botanical phyllotaxis.
pub const GOLDEN_ANGLE_RAD: f32 = 2.399_963_2;

/// 64-byte cache-line-aligned header for `FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE` (`0x0306`, `HOLO`).
///
/// Defines the 12-triangle proxy bounding lens (`proxy_aabb_min`, `proxy_aabb_max`),
/// segment counts, active harmonic cutoff $K \le 12$, raymarch step bound ($\le 32$),
/// and `Q42ConstituentLutEntry` binding index.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct HolographicPhaseHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    /// Number of 64-byte [`M64HoloRecord`] segments following this header.
    pub holo_segment_count: u16,
    /// Number of 64-byte [`EulerianWavefieldRecord`] fluid/wave streamline records following the segments.
    pub wavefield_count: u16,
    /// Maximum spherical/Bessel harmonic order (`1..=12`).
    pub max_harmonic_order: u8,
    /// Strict GPU TDR-safe raymarch step cap (`<= 32`).
    pub max_raymarch_steps: u8,
    /// Primary 1-byte index into the 16-KB `BankedConstituentLut` (`256 * 64B` `Q42ConstituentLutEntry`).
    pub primary_constituent_lut_idx: u8,
    /// Primitive lens kind (`0=RoundedBox, 1=Sphere, 2=Capsule, 3=MetaballCluster, 4=HyperbolicCanopy`).
    pub proxy_lens_kind: u8,
    /// Proxy bounding lens minimum corner `[min_x, min_y, min_z]` in SI meters (`12B`).
    pub proxy_aabb_min: [f32; 3],
    /// Proxy bounding lens maximum corner `[max_x, max_y, max_z]` in SI meters (`12B`).
    pub proxy_aabb_max: [f32; 3],
    /// Smooth-minimum ($C^1$ SDF fillet) blending radius in meters (`4B`).
    pub sdf_smooth_union_radius: f32,
    /// Reference 10D spectral signature $\sigma \in [0, 1]$ for CIE 1931 emission (`4B`).
    pub reference_sigma: f32,
    /// 64-bit `q_hash` of the governing `.q42` NQuin subject (`8B`).
    pub nquin_subject_hash: u64,
    /// Castagnoli CRC-32C over the payload bytes following this header (`4B`).
    pub payload_crc32c: u32,
    /// Castagnoli CRC-32C over bytes `0..60` of this header (`4B`).
    pub header_crc32c: u32,
}

const _: () = assert!(core::mem::size_of::<HolographicPhaseHeader>() == HOLOGRAPHIC_PHASE_HEADER_SIZE);
const _: () = assert!(core::mem::align_of::<HolographicPhaseHeader>() == 64);

impl HolographicPhaseHeader {
    #[inline]
    pub fn compute_header_crc32c(&self) -> u32 {
        let bytes: &[u8; HOLOGRAPHIC_PHASE_HEADER_SIZE] = bytemuck::cast_ref(self);
        crc32c(&bytes[..60])
    }

    #[inline]
    pub fn seal_header_crc32c(&mut self) {
        self.header_crc32c = self.compute_header_crc32c();
    }
}

/// 64-byte cache-line-aligned Eulerian Spectral Streamline & Gerstner/Airy Wave Record.
///
/// Evaluates zero-particle holographic rivers and atmospheric wavefields in ~25 GPU instructions:
/// $\mathbf{u}_{\text{river}}(x,y,z,t) = \nabla \times \Psi(x,y) + \sum_{m=1}^4 A_m(q)\cosh(k_m(z - z_{\text{bed}}))\cos(\mathbf{k}_m\cdot\mathbf{x}_{xy} - \omega_m t + \phi_m)$.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct EulerianWavefieldRecord {
    /// Mean Eulerian stream velocity `[u_x, u_y, u_z]` in m/s (`12B`).
    pub stream_velocity_m_s: [f32; 3],
    /// Streamfunction vorticity curl $\nabla \times \Psi$ (`topological_spin`, rad/s) (`4B`).
    pub vorticity_curl_rad_s: f32,
    /// Bed elevation $z_{\text{bed}}$ and mean water depth $h_0$ in meters (`8B`).
    pub bed_elevation_m: f32,
    pub water_depth_m: f32,
    /// Fluid density $\rho_{\text{fluid}}$ in kg/m$^3$ (e.g. `1000.0` freshwater, `1.225` air) and drag coefficient $C_d$ (`8B`).
    pub fluid_density_kg_m3: f32,
    pub drag_coefficient_cd: f32,
    /// 4 Gerstner/Airy spectral wave modes: amplitudes $A_m$ in `f16` meters (`8B`).
    pub mode_amplitudes_f16: [u16; 4],
    /// 4 Gerstner/Airy wave numbers $k_m$ in `f16` rad/m (`8B`).
    pub mode_wavenumbers_f16: [u16; 4],
    /// 4 Gerstner/Airy angular frequencies $\omega_m$ in `f16` rad/s (`8B`).
    pub mode_frequencies_f16: [u16; 4],
    /// Wave direction angles in quantized `i8` ($[-\pi, \pi]$) (`4B`).
    pub mode_directions_q8: [i8; 4],
    /// Castagnoli CRC-32C over bytes `0..60` (`4B`).
    pub record_crc32c: u32,
}

const _: () =
    assert!(core::mem::size_of::<EulerianWavefieldRecord>() == EULERIAN_WAVEFIELD_RECORD_SIZE);
const _: () = assert!(core::mem::align_of::<EulerianWavefieldRecord>() == 64);

impl EulerianWavefieldRecord {
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes: &[u8; EULERIAN_WAVEFIELD_RECORD_SIZE] = bytemuck::cast_ref(self);
        crc32c(&bytes[..60])
    }

    #[inline]
    pub fn seal_crc32c(&mut self) {
        self.record_crc32c = self.compute_crc32c();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolographicPhaseError {
    PayloadTooShort { need: usize, have: usize },
    OutputTooSmall { need: usize, have: usize },
    BadMagic([u8; 4]),
    UnsupportedVersion(u16),
    CountLimitExceeded,
    RaymarchStepsExceeded { got: u8, max: u8 },
    InvalidBounds,
    LengthMismatch { expected: usize, actual: usize },
    ChecksumMismatch { expected: u32, actual: u32 },
}

impl core::fmt::Display for HolographicPhaseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for HolographicPhaseError {}

/// Compute exact byte length for a `HOLO` section slice.
#[inline]
pub const fn holographic_phase_encoded_len(
    segment_count: usize,
    wavefield_count: usize,
) -> Option<usize> {
    if segment_count > MAX_HOLO_SEGMENTS || wavefield_count > MAX_EULERIAN_WAVEFIELDS {
        return None;
    }
    Some(
        HOLOGRAPHIC_PHASE_HEADER_SIZE
            + segment_count * core::mem::size_of::<M64HoloRecord>()
            + wavefield_count * EULERIAN_WAVEFIELD_RECORD_SIZE,
    )
}

/// Encode a `HOLO` (`FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE = 0x0306`) section slice into caller storage. Zero heap.
pub fn encode_holographic_phase_section(
    mut header: HolographicPhaseHeader,
    segments: &[M64HoloRecord],
    wavefields: &[EulerianWavefieldRecord],
    out: &mut [u8],
) -> Result<usize, HolographicPhaseError> {
    let needed = holographic_phase_encoded_len(segments.len(), wavefields.len())
        .ok_or(HolographicPhaseError::CountLimitExceeded)?;
    if out.len() < needed {
        return Err(HolographicPhaseError::OutputTooSmall {
            need: needed,
            have: out.len(),
        });
    }
    if header.max_raymarch_steps == 0
        || (header.max_raymarch_steps as u32) > HOLOGRAPHIC_MAX_RAYMARCH_STEPS
    {
        return Err(HolographicPhaseError::RaymarchStepsExceeded {
            got: header.max_raymarch_steps,
            max: HOLOGRAPHIC_MAX_RAYMARCH_STEPS as u8,
        });
    }
    for axis in 0..3 {
        if !header.proxy_aabb_min[axis].is_finite()
            || !header.proxy_aabb_max[axis].is_finite()
            || header.proxy_aabb_min[axis] > header.proxy_aabb_max[axis]
        {
            return Err(HolographicPhaseError::InvalidBounds);
        }
    }

    let mut cursor = HOLOGRAPHIC_PHASE_HEADER_SIZE;
    for seg in segments {
        out[cursor..cursor + 64].copy_from_slice(bytes_of(seg));
        cursor += 64;
    }
    for wf in wavefields {
        let mut sealed_wf = *wf;
        sealed_wf.seal_crc32c();
        out[cursor..cursor + 64].copy_from_slice(bytes_of(&sealed_wf));
        cursor += 64;
    }

    header.magic = HOLOGRAPHIC_PHASE_TAG;
    header.version = HOLOGRAPHIC_PHASE_VERSION;
    header.holo_segment_count = segments.len() as u16;
    header.wavefield_count = wavefields.len() as u16;
    header.payload_crc32c = crc32c(&out[HOLOGRAPHIC_PHASE_HEADER_SIZE..needed]);
    header.seal_header_crc32c();
    out[..HOLOGRAPHIC_PHASE_HEADER_SIZE].copy_from_slice(bytes_of(&header));
    Ok(needed)
}

/// Decode and verify a `HOLO` (`FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE = 0x0306`) section slice into caller buffers. Zero heap.
pub fn decode_holographic_phase_section_into(
    payload: &[u8],
    out_segments: &mut [M64HoloRecord],
    out_wavefields: &mut [EulerianWavefieldRecord],
) -> Result<(HolographicPhaseHeader, usize, usize), HolographicPhaseError> {
    if payload.len() < HOLOGRAPHIC_PHASE_HEADER_SIZE {
        return Err(HolographicPhaseError::PayloadTooShort {
            need: HOLOGRAPHIC_PHASE_HEADER_SIZE,
            have: payload.len(),
        });
    }
    let header: HolographicPhaseHeader =
        pod_read_unaligned(&payload[..HOLOGRAPHIC_PHASE_HEADER_SIZE]);
    if header.magic != HOLOGRAPHIC_PHASE_TAG {
        return Err(HolographicPhaseError::BadMagic(header.magic));
    }
    if header.version != HOLOGRAPHIC_PHASE_VERSION {
        return Err(HolographicPhaseError::UnsupportedVersion(header.version));
    }
    let hdr_crc = header.compute_header_crc32c();
    if header.header_crc32c != hdr_crc {
        return Err(HolographicPhaseError::ChecksumMismatch {
            expected: header.header_crc32c,
            actual: hdr_crc,
        });
    }
    if header.max_raymarch_steps == 0
        || (header.max_raymarch_steps as u32) > HOLOGRAPHIC_MAX_RAYMARCH_STEPS
    {
        return Err(HolographicPhaseError::RaymarchStepsExceeded {
            got: header.max_raymarch_steps,
            max: HOLOGRAPHIC_MAX_RAYMARCH_STEPS as u8,
        });
    }
    let seg_count = header.holo_segment_count as usize;
    let wf_count = header.wavefield_count as usize;
    let expected = holographic_phase_encoded_len(seg_count, wf_count)
        .ok_or(HolographicPhaseError::CountLimitExceeded)?;
    if payload.len() != expected {
        return Err(HolographicPhaseError::LengthMismatch {
            expected,
            actual: payload.len(),
        });
    }
    if out_segments.len() < seg_count || out_wavefields.len() < wf_count {
        return Err(HolographicPhaseError::OutputTooSmall {
            need: seg_count + wf_count,
            have: out_segments.len() + out_wavefields.len(),
        });
    }
    let body_crc = crc32c(&payload[HOLOGRAPHIC_PHASE_HEADER_SIZE..expected]);
    if header.payload_crc32c != body_crc {
        return Err(HolographicPhaseError::ChecksumMismatch {
            expected: header.payload_crc32c,
            actual: body_crc,
        });
    }

    let mut cursor = HOLOGRAPHIC_PHASE_HEADER_SIZE;
    for i in 0..seg_count {
        out_segments[i] = pod_read_unaligned(&payload[cursor..cursor + 64]);
        cursor += 64;
    }
    for i in 0..wf_count {
        let wf: EulerianWavefieldRecord = pod_read_unaligned(&payload[cursor..cursor + 64]);
        let wf_crc = wf.compute_crc32c();
        if wf.record_crc32c != wf_crc {
            return Err(HolographicPhaseError::ChecksumMismatch {
                expected: wf.record_crc32c,
                actual: wf_crc,
            });
        }
        out_wavefields[i] = wf;
        cursor += 64;
    }

    Ok((header, seg_count, wf_count))
}

// ── Zero-Heap Analytical Holographic & Continuum Evaluators (Doc 1 & Doc 3) ──

/// Select active harmonic order $K_{\text{active}} \in [1, 12]$ from camera distance $d$ (Scale-Space Infinite LOD).
///
/// At $d \ge 1000\text{ m}$ (Tier 0), evaluates only $k = 1..2$ low-frequency capsule harmonics;
/// at $d \le 1\text{ m}$ (Tier 3), evaluates all 12 spherical/Bessel harmonics.
#[inline]
pub fn select_scale_space_harmonic_cutoff_zero_heap(
    distance_m: f32,
    max_order: u8,
) -> usize {
    let cap = (max_order.clamp(1, 12)) as usize;
    if distance_m >= 1000.0 {
        2.min(cap)
    } else if distance_m >= 150.0 {
        4.min(cap)
    } else if distance_m >= 25.0 {
        8.min(cap)
    } else {
        cap
    }
}

/// Evaluate the Quantum Path-Integral / Stationary-Action ($\delta S = 0$) Cubic Hermite
/// Hamiltonian Boundary-Value Spline + Damped Coupled Oscillator + Fluid-Structure Drag shift:
///
/// $$\mathbf{s}(t, q) = H_0(u)\mathbf{s}_0 + H_1(u)\mathbf{s}_1 + \frac{H_2(u)}{\alpha}\mathbf{p}_0 + \frac{H_3(u)}{\alpha}\mathbf{p}_1 + \frac{\mathbf{F}_{\text{drag}}}{\alpha \omega^2} + \sum_{k=0}^2 A_k(q)e^{-\gamma t}\cos(\omega_k t + \phi_k)$$
#[inline]
pub fn evaluate_hamiltonian_stationary_action_spline_zero_heap(
    s0: [f32; 3],
    p0: [f32; 3],
    juncture: &M64PhaseJunctureRecord,
    holo: &M64HoloRecord,
    fluid_velocity_m_s: [f32; 3],
    fluid_density_kg_m3: f32,
    time_sec: f32,
    duration_sec: f32,
) -> [f32; 3] {
    let u = if duration_sec > 1e-5 {
        (time_sec / duration_sec).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let u2 = u * u;
    let u3 = u2 * u;

    // Cubic Hermite basis functions H0, H1, H2, H3
    let h0 = 2.0 * u3 - 3.0 * u2 + 1.0;
    let h1 = -2.0 * u3 + 3.0 * u2;
    let h2 = u3 - 2.0 * u2 + u;
    let h3 = u3 - u2;

    let s1 = [
        (juncture.anchor_pos_mm[0] as f32) * 0.001,
        (juncture.anchor_pos_mm[1] as f32) * 0.001,
        (juncture.anchor_pos_mm[2] as f32) * 0.001,
    ];
    let p1 = [
        juncture.stress_energy_tensor[0],
        juncture.stress_energy_tensor[1],
        juncture.stress_energy_tensor[2],
    ];

    let alpha_mass = f16::from_bits(holo.mass_amplitude).to_f32().max(0.05);
    let gamma = f16::from_bits(holo.damping_decay).to_f32().max(0.01);
    let decay = (-gamma * time_sec.max(0.0)).exp();
    let q_phase = (juncture.quantum_branch_id as f32) * 0.785_398_2;

    let mut out = [0.0f32; 3];
    for axis in 0..3 {
        let omega = f16::from_bits(holo.natural_freq_omega[axis])
            .to_f32()
            .max(0.5);
        // Analytical fluid drag equilibrium shift: F_drag / (alpha * omega^2)
        let v_rel = fluid_velocity_m_s[axis];
        let f_drag = 0.5 * fluid_density_kg_m3 * 0.8 * v_rel * v_rel.abs();
        let steady_drag_bend = (f_drag / (alpha_mass * omega * omega)).clamp(-2.5, 2.5);

        let osc_amp = 0.05 * (1.0 + ((holo.phase_attractor_mask & 0x0F) as f32) * 0.1);
        let osc = osc_amp * decay * (omega * time_sec + q_phase).cos();

        out[axis] = h0 * s0[axis]
            + h1 * s1[axis]
            + (h2 * p0[axis] + h3 * p1[axis]) / alpha_mass
            + steady_drag_bend
            + osc;
    }
    out
}

/// Evaluate Continuous Hyperbolic $v = 2$ (Poincaré Ball Möbius Isometry) Morphogenesis
/// and Golden-Angle Bessel/Spherical-Harmonic Surface Displacement:
///
/// $$T_{\mathbf{b}}(\mathbf{z}) = \frac{(1 + 2\langle \mathbf{b}, \mathbf{z}\rangle + \|\mathbf{z}\|^2)\mathbf{b} + (1 - \|\mathbf{b}\|^2)\mathbf{z}}{1 + 2\langle \mathbf{b}, \mathbf{z}\rangle + \|\mathbf{b}\|^2\|\mathbf{z}\|^2}$$
#[inline]
pub fn evaluate_hyperbolic_mobius_morphogenesis_zero_heap(
    local_pos: [f32; 3],
    holo: &M64HoloRecord,
    growth_driver_b: [f32; 3],
    time_sec: f32,
    camera_distance_m: f32,
    max_order: u8,
) -> [f32; 3] {
    // Clamp growth driver b inside the open Poincaré unit ball (||b|| < 0.95)
    let b_norm_sq = growth_driver_b[0] * growth_driver_b[0]
        + growth_driver_b[1] * growth_driver_b[1]
        + growth_driver_b[2] * growth_driver_b[2];
    let b_scale = if b_norm_sq > 0.9025 {
        0.95 / b_norm_sq.sqrt()
    } else {
        1.0
    };
    let b = [
        growth_driver_b[0] * b_scale,
        growth_driver_b[1] * b_scale,
        growth_driver_b[2] * b_scale,
    ];
    let b_sq = b[0] * b[0] + b[1] * b[1] + b[2] * b[2];
    let z_sq = local_pos[0] * local_pos[0]
        + local_pos[1] * local_pos[1]
        + local_pos[2] * local_pos[2];
    let b_dot_z = b[0] * local_pos[0] + b[1] * local_pos[1] + b[2] * local_pos[2];

    let denom = (1.0 + 2.0 * b_dot_z + b_sq * z_sq).max(1e-5);
    let c1 = 1.0 + 2.0 * b_dot_z + z_sq;
    let c2 = 1.0 - b_sq;

    let mut morphed = [
        (c1 * b[0] + c2 * local_pos[0]) / denom,
        (c1 * b[1] + c2 * local_pos[1]) / denom,
        (c1 * b[2] + c2 * local_pos[2]) / denom,
    ];

    // Golden-angle Bessel / spherical harmonic surface displacement filtered by scale-space LOD cutoff
    let k_active = select_scale_space_harmonic_cutoff_zero_heap(camera_distance_m, max_order);
    let r_xy = (morphed[0] * morphed[0] + morphed[2] * morphed[2]).sqrt().max(1e-5);
    let theta = morphed[2].atan2(morphed[0]);
    let h = morphed[1];

    let mut radial_disp = 0.0f32;
    for m in 0..k_active {
        let coeff = (holo.sdf_harmonics[m] as f32) / 127.0;
        let m_f = (m + 1) as f32;
        // Analytical J_m Bessel-like radial envelope + golden-angle phyllotaxis phase
        let bessel_env = (m_f * r_xy).sin() / (1.0 + m_f * r_xy);
        let phase = m_f * theta - GOLDEN_ANGLE_RAD * h + 0.1 * time_sec;
        radial_disp += coeff * 0.08 * bessel_env * phase.cos();
    }

    let inv_r = 1.0 / r_xy;
    morphed[0] += morphed[0] * inv_r * radial_disp;
    morphed[2] += morphed[2] * inv_r * radial_disp;
    morphed
}

/// Evaluate Eulerian Spectral Streamline + Gerstner Wave velocity $\mathbf{u}_{\text{river}}(x,y,z,t)$
/// and Drucker-Prager / Herschel-Bulkley shear stability against a `Q42ConstituentLutEntry`.
#[inline]
pub fn evaluate_eulerian_streamline_and_drucker_prager_zero_heap(
    pos_xyz: [f32; 3],
    time_sec: f32,
    wavefield: &EulerianWavefieldRecord,
    strata: &M64StrataColumnRecord,
    lut_entry: &Q42ConstituentLutEntry,
) -> ([f32; 3], f32, bool) {
    let mut vel = wavefield.stream_velocity_m_s;
    // Add rotational curl contribution from streamfunctionPsi
    vel[0] -= 0.5 * wavefield.vorticity_curl_rad_s * pos_xyz[1];
    vel[1] += 0.5 * wavefield.vorticity_curl_rad_s * pos_xyz[0];

    let z_rel = (pos_xyz[2] - wavefield.bed_elevation_m).max(0.0);
    for m in 0..4 {
        let amp = f16::from_bits(wavefield.mode_amplitudes_f16[m]).to_f32();
        let k = f16::from_bits(wavefield.mode_wavenumbers_f16[m]).to_f32();
        let omega = f16::from_bits(wavefield.mode_frequencies_f16[m]).to_f32();
        let dir_angle = (wavefield.mode_directions_q8[m] as f32) * (core::f32::consts::PI / 127.0);
        let kx = k * dir_angle.cos();
        let ky = k * dir_angle.sin();
        let phase = kx * pos_xyz[0] + ky * pos_xyz[1] - omega * time_sec;
        let depth_atten = (k * z_rel).min(3.0).cosh();
        let wave_v = amp * omega * depth_atten * phase.cos();
        vel[0] += wave_v * dir_angle.cos();
        vel[1] += wave_v * dir_angle.sin();
    }

    // Mohr-Coulomb / Drucker-Prager yield stress tau_y = c + (sigma_n - P_pore) * tan(phi)
    let cohesion_kpa = lut_entry.cohesion_kpa as f32;
    let friction_rad = (lut_entry.friction_angle_deg as f32).to_radians();
    let water_depth_m = (strata.surface_fluid_depth_mm as f32) * 0.001;
    let pore_pressure_kpa = 9.81 * water_depth_m;
    let overburden_kpa =
        ((lut_entry.bulk_density_kg_m3 as f32) * 9.81 * 0.5) * 0.001;
    let effective_normal_kpa = (overburden_kpa - pore_pressure_kpa).max(0.0);
    let yield_stress_kpa = cohesion_kpa + effective_normal_kpa * friction_rad.tan();

    let flow_speed_sq = vel[0] * vel[0] + vel[1] * vel[1];
    let hydrodynamic_shear_kpa = 0.5 * wavefield.fluid_density_kg_m3 * flow_speed_sq * 0.001;
    let slumping_or_eroding = hydrodynamic_shear_kpa > yield_stress_kpa;

    (vel, yield_stress_kpa, slumping_or_eroding)
}

/// Deterministically fit a 64-byte [`M64HoloRecord`] and [`HolographicPhaseHeader`] proxy lens
/// from an existing triangle [`Mesh`] and optional vertex color reading. Zero heap.
///
/// This powers automated conversion of legacy B-Rep `.10d` assets into dual-capable
/// Holographic Phase-Space `.10d` v2 containers.
pub fn fit_m64_holo_record_from_mesh_zero_heap(
    mesh: &Mesh,
    surface_reading: Option<&[[f32; 4]]>,
    nquin_subject_hash: u64,
    provenance_did_hash: u64,
    metric_variety: u8,
) -> (HolographicPhaseHeader, M64HoloRecord) {
    let min = mesh.min;
    let max = mesh.max;
    let center = [
        0.5 * (min[0] + max[0]),
        0.5 * (min[1] + max[1]),
        0.5 * (min[2] + max[2]),
    ];
    let extent = [
        (max[0] - min[0]).max(1e-4),
        (max[1] - min[1]).max(1e-4),
        (max[2] - min[2]).max(1e-4),
    ];
    let mean_radius = 0.5 * (extent[0] + extent[2]);

    // Project vertex radial deviations onto 12 angular/vertical Bessel harmonics
    let mut accum = [0.0f32; 12];
    let mut weight_sum = [0.0f32; 12];
    for p in &mesh.positions {
        let dx = p[0] - center[0];
        let dy = (p[1] - min[1]) / extent[1];
        let dz = p[2] - center[2];
        let r = (dx * dx + dz * dz).sqrt();
        let dev = ((r - 0.5 * mean_radius) / mean_radius.max(1e-4)).clamp(-1.0, 1.0);
        let theta = dz.atan2(dx);
        for m in 0..12 {
            let m_f = (m + 1) as f32;
            let basis = (m_f * theta - GOLDEN_ANGLE_RAD * dy).cos();
            accum[m] += dev * basis;
            weight_sum[m] += basis * basis;
        }
    }

    let mut sdf_harmonics = [0i8; 12];
    for m in 0..12 {
        if weight_sum[m] > 1e-5 {
            let norm = (accum[m] / weight_sum[m]).clamp(-1.0, 1.0);
            sdf_harmonics[m] = (norm * 127.0).round() as i8;
        }
    }

    // Extract 6-band spectral reflectance estimate from surface_reading if present
    let mut spectral_spd_base = [200u8, 185, 165, 220, 235, 210];
    if let Some(colors) = surface_reading {
        if !colors.is_empty() {
            let mut rgb = [0.0f32; 3];
            for c in colors {
                rgb[0] += c[0].clamp(0.0, 1.0);
                rgb[1] += c[1].clamp(0.0, 1.0);
                rgb[2] += c[2].clamp(0.0, 1.0);
            }
            let inv = 1.0 / (colors.len() as f32);
            let r8 = (rgb[0] * inv * 255.0).round() as u8;
            let g8 = (rgb[1] * inv * 255.0).round() as u8;
            let b8 = (rgb[2] * inv * 255.0).round() as u8;
            spectral_spd_base = [
                b8,
                ((b8 as u16 + g8 as u16) / 2) as u8,
                g8,
                ((g8 as u16 + r8 as u16) / 2) as u8,
                r8,
                r8.saturating_add(15),
            ];
        }
    }

    // Natural eigen-frequencies scale inversely with physical extent
    let omega_x = f16::from_f32((3.5 / extent[0].sqrt()).clamp(0.4, 25.0)).to_bits();
    let omega_y = f16::from_f32((4.5 / extent[1].sqrt()).clamp(0.4, 25.0)).to_bits();
    let omega_z = f16::from_f32((3.5 / extent[2].sqrt()).clamp(0.4, 25.0)).to_bits();

    let holo = M64HoloRecord {
        local_origin: [
            f16::from_f32(center[0]).to_bits(),
            f16::from_f32(center[1]).to_bits(),
            f16::from_f32(center[2]).to_bits(),
        ],
        metric_variety,
        semantic_node_idx: 0,
        parent_joint_offset: 0,
        scale_bounds: [
            f16::from_f32(0.25).to_bits(),
            f16::from_f32(2.50).to_bits(),
        ],
        topological_spin: 1375,
        natural_freq_omega: [omega_x, omega_y, omega_z],
        damping_decay: f16::from_f32(0.35).to_bits(),
        mass_amplitude: f16::from_f32((extent[0] * extent[1] * extent[2]).clamp(0.1, 500.0))
            .to_bits(),
        phase_attractor_mask: 0x000F,
        hamiltonian_potential: 0x3F80_0000,
        sdf_harmonics,
        tropism_sensitivity: [64, -64, 32, 24],
        spectral_spd_base,
        subsurface_roughness: [36, 82],
        provenance_did_hash,
    };

    let mut header = HolographicPhaseHeader {
        magic: HOLOGRAPHIC_PHASE_TAG,
        version: HOLOGRAPHIC_PHASE_VERSION,
        flags: 0,
        holo_segment_count: 1,
        wavefield_count: 0,
        max_harmonic_order: 12,
        max_raymarch_steps: HOLOGRAPHIC_MAX_RAYMARCH_STEPS as u8,
        primary_constituent_lut_idx: 1,
        proxy_lens_kind: 0,
        proxy_aabb_min: min,
        proxy_aabb_max: max,
        sdf_smooth_union_radius: (0.05 * mean_radius).clamp(0.005, 0.5),
        reference_sigma: 0.55,
        nquin_subject_hash,
        payload_crc32c: 0,
        header_crc32c: 0,
    };
    header.seal_header_crc32c();

    (header, holo)
}

/// Maximum number of spatial sub-segments fitted by [`fit_m64_holo_segments_from_mesh_zero_heap`].
pub const MAX_FITTED_HOLO_SEGMENTS: usize = 64;

/// 2D point-in-triangle test in the XZ plane with barycentric tolerance.
#[inline]
fn point_in_triangle_xz(px: f32, pz: f32, a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> bool {
    let v0x = c[0] - a[0];
    let v0z = c[2] - a[2];
    let v1x = b[0] - a[0];
    let v1z = b[2] - a[2];
    let v2x = px - a[0];
    let v2z = pz - a[2];

    let dot00 = v0x * v0x + v0z * v0z;
    let dot01 = v0x * v1x + v0z * v1z;
    let dot02 = v0x * v2x + v0z * v2z;
    let dot11 = v1x * v1x + v1z * v1z;
    let dot12 = v1x * v2x + v1z * v2z;

    let denom = dot00 * dot11 - dot01 * dot01;
    if denom.abs() < 1e-10 {
        return false;
    }
    let inv = 1.0 / denom;
    let u = (dot11 * dot02 - dot01 * dot12) * inv;
    let v = (dot00 * dot12 - dot01 * dot02) * inv;
    let eps = -1e-4;
    (u >= eps) && (v >= eps) && (u + v <= 1.0 - eps)
}

/// Partition a triangle [`Mesh`] into up to [`MAX_FITTED_HOLO_SEGMENTS`] connected row-run
/// [`M64HoloRecord`] segments over a high-resolution $24 \times 24$ XZ occupancy grid (`local_origin`,
/// per-run XZ half-widths in `scale_bounds`, Y half-height in `mass_amplitude`, 12 local Bessel/spherical
/// harmonics `sdf_harmonics`, and spectral reflectance).
///
/// Unlike [`fit_m64_holo_record_from_mesh_zero_heap`] (which collapses an entire perforated structure
/// into a single segment) or isolated cell pillars (which leave exterior gaps between columns),
/// merging contiguous occupied XZ cells along each Z row with an overlapping Z half-depth
/// (`0.58 * cell_dz`) guarantees connected bone manifolds while keeping all distinct anatomical
/// canals and foramina enclosed and open across harmonic cutoffs $K \in \{2, 4, 8, 12\}$. Zero heap.
pub fn fit_m64_holo_segments_from_mesh_zero_heap(
    mesh: &Mesh,
    surface_reading: Option<&[[f32; 4]]>,
    nquin_subject_hash: u64,
    provenance_did_hash: u64,
    metric_variety: u8,
    out_segments: &mut [M64HoloRecord; MAX_FITTED_HOLO_SEGMENTS],
) -> (HolographicPhaseHeader, usize) {
    let (mut base_hdr, single_rec) = fit_m64_holo_record_from_mesh_zero_heap(
        mesh,
        surface_reading,
        nquin_subject_hash,
        provenance_did_hash,
        metric_variety,
    );
    if mesh.positions.len() < 12 || mesh.triangles.is_empty() {
        out_segments[0] = single_rec;
        return (base_hdr, 1);
    }

    let min = mesh.min;
    let max = mesh.max;
    let ext = [
        (max[0] - min[0]).max(1e-4),
        (max[1] - min[1]).max(1e-4),
        (max[2] - min[2]).max(1e-4),
    ];

    // 24x24 XZ fine sampling grid (576 cells), merged along X into <= 64 connected row-run segments.
    // Resolves narrow lateral canals (e.g. C2/C6/C7 transverse foramina) while guaranteeing
    // overlap between adjacent runs so open canals are enclosed rather than leaking to the exterior.
    const G: usize = 24;
    const CELLS: usize = G * G;
    let cell_dx = ext[0] / (G as f32);
    let cell_dz = ext[2] / (G as f32);

    let mut cell_tri_counts = [0u32; CELLS];
    let mut cell_ymin = [f32::INFINITY; CELLS];
    let mut cell_ymax = [f32::NEG_INFINITY; CELLS];
    let mut cell_rgb = [[0.0f32; 3]; CELLS];

    for tri in &mesh.triangles {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        if i0 >= mesh.positions.len() || i1 >= mesh.positions.len() || i2 >= mesh.positions.len() {
            continue;
        }
        let a = mesh.positions[i0];
        let b = mesh.positions[i1];
        let c = mesh.positions[i2];

        let tmin_x = a[0].min(b[0]).min(c[0]);
        let tmax_x = a[0].max(b[0]).max(c[0]);
        let tmin_z = a[2].min(b[2]).min(c[2]);
        let tmax_z = a[2].max(b[2]).max(c[2]);
        let tmin_y = a[1].min(b[1]).min(c[1]);
        let tmax_y = a[1].max(b[1]).max(c[1]);

        let ix0 = (((tmin_x - min[0]) / cell_dx).floor().max(0.0) as usize).min(G - 1);
        let ix1 = (((tmax_x - min[0]) / cell_dx).floor().max(0.0) as usize).min(G - 1);
        let iz0 = (((tmin_z - min[2]) / cell_dz).floor().max(0.0) as usize).min(G - 1);
        let iz1 = (((tmax_z - min[2]) / cell_dz).floor().max(0.0) as usize).min(G - 1);

        let mut tri_col = [0.0f32; 3];
        if let Some(cols) = surface_reading {
            for &vi in &[i0, i1, i2] {
                if let Some(col) = cols.get(vi) {
                    tri_col[0] += col[0].clamp(0.0, 1.0) / 3.0;
                    tri_col[1] += col[1].clamp(0.0, 1.0) / 3.0;
                    tri_col[2] += col[2].clamp(0.0, 1.0) / 3.0;
                }
            }
        }

        for iz in iz0..=iz1 {
            let z_lo = min[2] + (iz as f32) * cell_dz;
            let z_hi = z_lo + cell_dz;
            let cz = 0.5 * (z_lo + z_hi);
            for ix in ix0..=ix1 {
                let x_lo = min[0] + (ix as f32) * cell_dx;
                let x_hi = x_lo + cell_dx;
                let cx = 0.5 * (x_lo + x_hi);
                let cid = iz * G + ix;
                // Record the full vertical extent [tmin_y, tmax_y] of any triangle overlapping
                // this cell's horizontal footprint so sloping superior/inferior laminae do not
                // underestimate run_ymax/run_ymin at off-center Z positions within the cell.
                if tmax_x >= x_lo - 1e-5
                    && tmin_x <= x_hi + 1e-5
                    && tmax_z >= z_lo - 1e-5
                    && tmin_z <= z_hi + 1e-5
                {
                    cell_ymin[cid] = cell_ymin[cid].min(tmin_y);
                    cell_ymax[cid] = cell_ymax[cid].max(tmax_y);
                }
                if cz >= tmin_z - 1e-5
                    && cz <= tmax_z + 1e-5
                    && cx >= tmin_x - 1e-5
                    && cx <= tmax_x + 1e-5
                    && point_in_triangle_xz(cx, cz, a, b, c)
                {
                    cell_tri_counts[cid] += 1;
                    if surface_reading.is_some() {
                        cell_rgb[cid][0] += tri_col[0];
                        cell_rgb[cid][1] += tri_col[1];
                        cell_rgb[cid][2] += tri_col[2];
                    }
                }
            }
        }
    }

    let mut seg_count = 0usize;
    for iz in 0..G {
        let z0 = min[2] + (iz as f32) * cell_dz;
        let z1 = z0 + cell_dz;
        let mut ix = 0usize;
        while ix < G {
            let cid0 = iz * G + ix;
            if cell_tri_counts[cid0] == 0 {
                ix += 1;
                continue;
            }
            if seg_count >= MAX_FITTED_HOLO_SEGMENTS {
                break;
            }
            let start_ix = ix;
            let mut run_tris = 0u32;
            let mut run_ymin = f32::INFINITY;
            let mut run_ymax = f32::NEG_INFINITY;
            let mut run_rgb = [0.0f32; 3];
            while ix < G && cell_tri_counts[iz * G + ix] > 0 {
                let cid = iz * G + ix;
                run_tris += cell_tri_counts[cid];
                run_ymin = run_ymin.min(cell_ymin[cid]);
                run_ymax = run_ymax.max(cell_ymax[cid]);
                run_rgb[0] += cell_rgb[cid][0];
                run_rgb[1] += cell_rgb[cid][1];
                run_rgb[2] += cell_rgb[cid][2];
                ix += 1;
            }
            let end_ix = ix;
            let x0 = min[0] + (start_ix as f32) * cell_dx;
            let x1 = min[0] + (end_ix as f32) * cell_dx;
            let ymin = run_ymin.max(min[1]);
            let ymax = run_ymax.min(max[1]).max(ymin + 1e-3);

            let ccenter = [0.5 * (x0 + x1), 0.5 * (ymin + ymax), 0.5 * (z0 + z1)];
            // Add a small overlap margin (+0.08 * cell_dx in X and 0.58 * cell_dz in Z) so
            // adjacent row runs remain solidly connected across Z rows even when harmonic
            // modulation scales radius_z by (1 - 0.10) = 0.90 (0.58 * 0.90 = 0.522 > 0.50).
            let half_wx = (0.5 * (x1 - x0) + 0.08 * cell_dx).max(0.001);
            let half_dz = (0.58 * cell_dz).max(0.001);
            let half_hy = (0.5 * (ymax - ymin)).max(0.02 * ext[1]).max(0.001);
            let cell_radius = 0.5 * (half_wx + half_dz);

            // Fit 12 local harmonics for vertices in this row run
            let mut accum = [0.0f32; 12];
            let mut weight_sum = [0.0f32; 12];
            for p in &mesh.positions {
                if p[0] < x0 - 0.5 * cell_dx
                    || p[0] > x1 + 0.5 * cell_dx
                    || p[2] < z0 - 0.5 * cell_dz
                    || p[2] > z1 + 0.5 * cell_dz
                {
                    continue;
                }
                let dx = p[0] - ccenter[0];
                let dy = ((p[1] - ymin) / (2.0 * half_hy)).clamp(0.0, 1.0);
                let dz = p[2] - ccenter[2];
                let r = (dx * dx + dz * dz).sqrt();
                let dev = ((r - cell_radius) / cell_radius.max(1e-4)).clamp(-1.0, 1.0);
                let theta = dz.atan2(dx);
                for m in 0..12 {
                    let m_f = (m + 1) as f32;
                    let basis = (m_f * theta - GOLDEN_ANGLE_RAD * dy).cos();
                    accum[m] += dev * basis;
                    weight_sum[m] += basis * basis;
                }
            }
            let mut sdf_harmonics = single_rec.sdf_harmonics;
            for m in 0..12 {
                if weight_sum[m] > 1e-5 {
                    let norm = (accum[m] / weight_sum[m]).clamp(-1.0, 1.0);
                    // Damp local harmonic amplitude so scale-space modulation refines surface
                    // contour without shifting segment walls across narrow canals.
                    sdf_harmonics[m] = (norm * 42.0).round() as i8;
                }
            }

            let mut spectral_spd_base = single_rec.spectral_spd_base;
            if surface_reading.is_some() && run_tris > 0 {
                let inv = 1.0 / (run_tris as f32);
                let r8 = (run_rgb[0] * inv * 255.0).round() as u8;
                let g8 = (run_rgb[1] * inv * 255.0).round() as u8;
                let b8 = (run_rgb[2] * inv * 255.0).round() as u8;
                spectral_spd_base = [
                    b8,
                    ((b8 as u16 + g8 as u16) / 2) as u8,
                    g8,
                    ((g8 as u16 + r8 as u16) / 2) as u8,
                    r8,
                    r8.saturating_add(15),
                ];
            }

            let mut rec = single_rec;
            rec.local_origin = [
                f16::from_f32(ccenter[0]).to_bits(),
                f16::from_f32(ccenter[1]).to_bits(),
                f16::from_f32(ccenter[2]).to_bits(),
            ];
            rec.semantic_node_idx = (seg_count as u8).min(255);
            rec.parent_joint_offset = if seg_count == 0 { 0 } else { -(seg_count as i16) };
            rec.scale_bounds = [
                f16::from_f32(half_wx).to_bits(),
                f16::from_f32(half_dz).to_bits(),
            ];
            rec.mass_amplitude = f16::from_f32(half_hy).to_bits();
            rec.sdf_harmonics = sdf_harmonics;
            rec.spectral_spd_base = spectral_spd_base;
            out_segments[seg_count] = rec;
            seg_count += 1;
        }
    }

    if seg_count == 0 {
        out_segments[0] = single_rec;
        seg_count = 1;
    }

    base_hdr.holo_segment_count = seg_count as u16;
    base_hdr.proxy_lens_kind = if seg_count > 1 { 3 } else { 0 }; // 3 = MetaballCluster
    base_hdr.seal_header_crc32c();
    (base_hdr, seg_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holographic_phase_section_and_analytical_evaluators_roundtrip_zero_heap() {
        let mesh = Mesh {
            positions: vec![
                [-0.5, 0.0, -0.5],
                [0.5, 0.0, -0.5],
                [0.5, 2.0, 0.5],
                [-0.5, 2.0, 0.5],
            ],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
            min: [-0.5, 0.0, -0.5],
            max: [0.5, 2.0, 0.5],
        };
        let (hdr, holo) = fit_m64_holo_record_from_mesh_zero_heap(
            &mesh,
            Some(&[[0.2, 0.6, 0.25, 1.0]; 4]),
            0xAAAA_BBBB,
            0xCCCC_DDDD,
            2,
        );
        assert_eq!(hdr.max_raymarch_steps, 32);

        let wf = EulerianWavefieldRecord {
            stream_velocity_m_s: [1.4, 0.2, 0.0],
            vorticity_curl_rad_s: 0.35,
            bed_elevation_m: -1.2,
            water_depth_m: 1.2,
            fluid_density_kg_m3: 1000.0,
            drag_coefficient_cd: 0.9,
            mode_amplitudes_f16: [f16::from_f32(0.08).to_bits(); 4],
            mode_wavenumbers_f16: [f16::from_f32(1.5).to_bits(); 4],
            mode_frequencies_f16: [f16::from_f32(2.2).to_bits(); 4],
            mode_directions_q8: [12, -8, 24, 0],
            record_crc32c: 0,
        };

        let mut buf = [0u8; 256];
        let written = encode_holographic_phase_section(hdr, &[holo], &[wf], &mut buf).unwrap();
        assert_eq!(written, 192);

        let mut out_segs = [M64HoloRecord::default(); 2];
        let mut out_wfs = [EulerianWavefieldRecord::zeroed(); 2];
        let (dec_hdr, sc, wc) =
            decode_holographic_phase_section_into(&buf[..written], &mut out_segs, &mut out_wfs)
                .unwrap();
        assert_eq!(sc, 1);
        assert_eq!(wc, 1);
        assert_eq!(dec_hdr.nquin_subject_hash, 0xAAAA_BBBB);
        assert_eq!(out_segs[0], holo);

        // Evaluate Hamiltonian Stationary-Action Spline + Fluid Drag
        let juncture = M64PhaseJunctureRecord {
            provenance_did_hash: 0xCCCC_DDDD,
            lamport_clock: 10,
            anchor_pos_mm: [1200, 500, -300],
            quantum_branch_id: 1,
            boundary_flags: 0,
            action_integral_s: 0.42,
            stress_energy_tensor: [0.1, 0.2, 0.0, 0.05],
            excavation_radius_mm: 0,
            excavated_lut_material: 1,
            re_inflation_delay_ticks: 0,
            record_crc32c: 0,
        };
        let pos_t = evaluate_hamiltonian_stationary_action_spline_zero_heap(
            [0.0, 0.0, 0.0],
            [0.2, 0.0, 0.0],
            &juncture,
            &holo,
            [1.4, 0.2, 0.0],
            1000.0,
            0.5,
            1.0,
        );
        assert!(pos_t.iter().all(|v| v.is_finite()));
        assert!(pos_t[0] > 0.5); // Shifted toward juncture + downstream fluid drag

        // Evaluate Hyperbolic v=2 Mobius Morphogenesis + Scale-Space Harmonic LOD
        let morphed_near = evaluate_hyperbolic_mobius_morphogenesis_zero_heap(
            [0.3, 1.0, 0.2],
            &holo,
            [0.05, 0.15, 0.0],
            1.0,
            2.0,
            12,
        );
        let morphed_far = evaluate_hyperbolic_mobius_morphogenesis_zero_heap(
            [0.3, 1.0, 0.2],
            &holo,
            [0.05, 0.15, 0.0],
            1.0,
            1500.0,
            12,
        );
        assert!(morphed_near.iter().all(|v| v.is_finite()));
        assert!(morphed_far.iter().all(|v| v.is_finite()));
        assert_eq!(select_scale_space_harmonic_cutoff_zero_heap(1500.0, 12), 2);
        assert_eq!(select_scale_space_harmonic_cutoff_zero_heap(0.5, 12), 12);

        // Evaluate Eulerian Streamline + Drucker-Prager stability
        let strata = M64StrataColumnRecord::default();
        let lut = Q42ConstituentLutEntry::default();
        let (u_vel, yield_kpa, _eroding) = evaluate_eulerian_streamline_and_drucker_prager_zero_heap(
            [2.0, 1.0, -0.5],
            1.5,
            &out_wfs[0],
            &strata,
            &lut,
        );
        assert!(u_vel.iter().all(|v| v.is_finite()));
        assert!(yield_kpa > 0.0);
    }
}
