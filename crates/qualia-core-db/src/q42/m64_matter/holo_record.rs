//! 64-Byte Aligned Holographic Phase-Space Entity Record (.m64 / .e64 Mode 0x01)
//!
//! Represents continuous botanical growth, allometric scaling, Hamiltonian motion graphs,
//! and spectral optics for flora, fauna, and geological objects without baking discrete keyframes.

use bytemuck::{Pod, Zeroable};

/// 64-Byte Aligned Holographic Entity Record (.m64 Mode 0x01)
/// Fits in a single CPU L1 cache line & maps 1:1 to a WGSL `array<vec4<u32>, 4>`.
#[repr(C, align(64))]
#[derive(Debug, Copy, Clone, PartialEq, Pod, Zeroable)]
pub struct M64HoloRecord {
    // --- 16 Bytes: Spatial & Topological Anchor ---
    /// Local parent-relative anchor coordinates (x, y, z) as IEEE 754 half-precision bits
    pub local_origin: [u16; 3],
    /// Metric variety v-axis (0=Euclidean, 1=Cyclic, 2=Hyperbolic Taxonomic Tree)
    pub metric_variety: u8,
    /// Ontological node index in the associated .q42 semantic volume
    pub semantic_node_idx: u8,
    /// Relative offset to parent record in hierarchical tree (0 = root)
    pub parent_joint_offset: i16,
    /// Allometric growth limits [min_scale, max_scale] as half-precision bits
    pub scale_bounds: [u16; 2],
    /// Quantized phyllotaxis curl / chirality spin [-32768..32767]
    pub topological_spin: i16,

    // --- 16 Bytes: Hamiltonian Kinematics & Branching (q, t) ---
    /// 3-axis natural eigen-frequencies (ω_x, ω_y, ω_z) as half-precision bits
    pub natural_freq_omega: [u16; 3],
    /// Damping and energy dissipation coefficient (γ) as half-precision bits
    pub damping_decay: u16,
    /// Invariant mass / structural rigidity (α-axis) as half-precision bits
    pub mass_amplitude: u16,
    /// Bitmask of permissible quantum simulation branches q
    pub phase_attractor_mask: u16,
    /// Packed 4x8-bit Hamiltonian potential well parameters
    pub hamiltonian_potential: u32,

    // --- 16 Bytes: Holographic Shape & Morphogenesis Harmonics ---
    /// Quantized spherical harmonic / Bessel surface deformation weights
    pub sdf_harmonics: [i8; 12],
    /// Tropism response sensitivities: [Phototropism, Gravitropism, Hydrotropism, Wind]
    pub tropism_sensitivity: [i8; 4],

    // --- 16 Bytes: Spectral Optics (σ) & Provenance (μ) ---
    /// 6-band EMF reflectance spectrum (visible through near-IR)
    pub spectral_spd_base: [u8; 6],
    /// Physical optical traits: [Subsurface scattering factor, Microfacet roughness]
    pub subsurface_roughness: [u8; 2],
    /// FNV-1a hash of Q42 provenance DID or genomic seed authority
    pub provenance_did_hash: u64,
}

/// Backward-compatible type alias for E64HoloRecord.
pub type E64HoloRecord = M64HoloRecord;

impl Default for M64HoloRecord {
    fn default() -> Self {
        Self {
            local_origin: [0, 0, 0],
            metric_variety: 2, // Default: Hyperbolic tree (botanical)
            semantic_node_idx: 0,
            parent_joint_offset: 0,
            scale_bounds: [0x3800, 0x4400], // Half-precision ~ [0.5, 4.0]
            topological_spin: 0,
            natural_freq_omega: [0x3c00, 0x3c00, 0x3c00], // ~ [1.0, 1.0, 1.0]
            damping_decay: 0x3800, // ~ 0.5
            mass_amplitude: 0x3c00, // ~ 1.0
            phase_attractor_mask: 0x0001,
            hamiltonian_potential: 0,
            sdf_harmonics: [0; 12],
            tropism_sensitivity: [64, -64, 32, 16],
            spectral_spd_base: [200, 180, 160, 220, 240, 210],
            subsurface_roughness: [30, 80],
            provenance_did_hash: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_m64_holo_record_size_and_alignment() {
        assert_eq!(std::mem::size_of::<M64HoloRecord>(), 64);
        assert_eq!(std::mem::align_of::<M64HoloRecord>(), 64);
    }
}
