//! Semantic procedural header adapter and universe admission for canonical slab residency.
use crate::container_10d::procedural_synthesis::{ProceduralSynthesisHeader, MAGIC_PROC};
use crate::gpu_context::{global_vram_ledger, VramReservation, VramResourceClass};
use bytemuck::{Pod, Zeroable};
pub use etalon_render::mesh_slab::{
    AssetRepresentationKind, MeshSlabArenaDescriptor, MeshSlabVertex, ResidentAssetEntry,
    SkinAttrVertex, SlabRange, SlabRangeAllocator, SubmeshSlice, INVALID_HEADER_SLOT,
    MAX_SLAB_PROC_HEADERS, MAX_SLAB_RESIDENT_ASSETS, REP_KIND_HOLOGRAPHIC_SDF_PROXY,
    REP_KIND_MESH_BREP, REP_KIND_OCTAHEDRAL_IMPOSTOR, REP_KIND_SGGX_CANOPY, REP_KIND_SURFEL_CLOUD,
};
/// 64-byte GPU storage representation of `.10d` `MAGIC_PROC` `ProceduralSynthesisHeader`.
///
/// Uses 32-bit words for header fields so WGSL shaders can read it directly without `u64`/`u16`
/// extension requirements while remaining byte-for-byte compatible with `ProceduralSynthesisHeader`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct ProceduralSynthesisGpu {
    pub magic_u32: u32,
    /// Low 16 bits = `version`, high 16 bits = `flags` (`FLAG_CELLULAR_VORONOI`, `FLAG_PGA_MOTOR_DYNAMICS`, etc.).
    pub version_and_flags: u32,
    pub seed_lo: u32,
    pub seed_hi: u32,
    pub voronoi_frequency: f32,
    pub voronoi_jitter: f32,
    pub fillet_blend_radius: f32,
    pub fillet_threshold: f32,
    pub allometry_exponent_k: f32,
    pub allometry_base_b: f32,
    pub spiral_growth_rate_k: f32,
    pub spiral_initial_radius_a: f32,
    pub motor_frequency_hz: f32,
    pub motor_damping_ratio: f32,
    pub critical_stress_threshold: f32,
    pub reserved_f32: f32,
}

const _: [(); 64] = [(); std::mem::size_of::<ProceduralSynthesisGpu>()];

impl Default for ProceduralSynthesisGpu {
    fn default() -> Self {
        bytemuck::Zeroable::zeroed()
    }
}

impl ProceduralSynthesisGpu {
    /// Converts a `.10d` `ProceduralSynthesisHeader` into its 64-byte GPU storage representation.
    #[inline]
    pub const fn from_header(header: &ProceduralSynthesisHeader) -> Self {
        let magic_u32 = u32::from_le_bytes(header.magic);
        let version_and_flags = (header.version as u32) | ((header.flags as u32) << 16);
        let seed_lo = header.seed as u32;
        let seed_hi = (header.seed >> 32) as u32;
        Self {
            magic_u32,
            version_and_flags,
            seed_lo,
            seed_hi,
            voronoi_frequency: header.voronoi_frequency,
            voronoi_jitter: header.voronoi_jitter,
            fillet_blend_radius: header.fillet_blend_radius,
            fillet_threshold: header.fillet_threshold,
            allometry_exponent_k: header.allometry_exponent_k,
            allometry_base_b: header.allometry_base_b,
            spiral_growth_rate_k: header.spiral_growth_rate_k,
            spiral_initial_radius_a: header.spiral_initial_radius_a,
            motor_frequency_hz: header.motor_frequency_hz,
            motor_damping_ratio: header.motor_damping_ratio,
            critical_stress_threshold: header.critical_stress_threshold,
            reserved_f32: header.reserved_f32,
        }
    }

    /// Reconstructs the CPU `ProceduralSynthesisHeader`.
    #[inline]
    pub const fn to_header(self) -> ProceduralSynthesisHeader {
        ProceduralSynthesisHeader {
            magic: self.magic_u32.to_le_bytes(),
            version: (self.version_and_flags & 0xFFFF) as u16,
            flags: (self.version_and_flags >> 16) as u16,
            seed: (self.seed_lo as u64) | ((self.seed_hi as u64) << 32),
            voronoi_frequency: self.voronoi_frequency,
            voronoi_jitter: self.voronoi_jitter,
            fillet_blend_radius: self.fillet_blend_radius,
            fillet_threshold: self.fillet_threshold,
            allometry_exponent_k: self.allometry_exponent_k,
            allometry_base_b: self.allometry_base_b,
            spiral_growth_rate_k: self.spiral_growth_rate_k,
            spiral_initial_radius_a: self.spiral_initial_radius_a,
            motor_frequency_hz: self.motor_frequency_hz,
            motor_damping_ratio: self.motor_damping_ratio,
            critical_stress_threshold: self.critical_stress_threshold,
            reserved_f32: self.reserved_f32,
        }
    }

    /// Returns the 16-bit `PROC1` synthesis recipe flags.
    #[inline]
    pub const fn flags(&self) -> u16 {
        (self.version_and_flags >> 16) as u16
    }

    /// Returns `true` when `magic_u32` matches `MAGIC_PROC`.
    #[inline]
    pub const fn is_valid(&self) -> bool {
        let m = self.magic_u32.to_le_bytes();
        m[0] == MAGIC_PROC[0]
            && m[1] == MAGIC_PROC[1]
            && m[2] == MAGIC_PROC[2]
            && m[3] == MAGIC_PROC[3]
    }
}

pub struct MeshSlabArenaGpu(
    etalon_render::mesh_slab::MeshSlabArenaGpu<VramReservation<'static>, ProceduralSynthesisGpu>,
);
impl std::ops::Deref for MeshSlabArenaGpu {
    type Target = etalon_render::mesh_slab::MeshSlabArenaGpu<
        VramReservation<'static>,
        ProceduralSynthesisGpu,
    >;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for MeshSlabArenaGpu {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl MeshSlabArenaGpu {
    pub fn required_vram_bytes(desc: MeshSlabArenaDescriptor) -> Option<u64> {
        etalon_render::mesh_slab::MeshSlabArenaGpu::<VramReservation<'static>,ProceduralSynthesisGpu>::required_vram_bytes(desc)
    }
    pub fn try_new(device: &etalon::Device, desc: MeshSlabArenaDescriptor) -> Option<Self> {
        etalon_render::mesh_slab::MeshSlabArenaGpu::try_new(device, desc, |bytes| {
            global_vram_ledger()
                .try_reserve_graphics(VramResourceClass::Geometry, bytes)
                .ok()
        })
        .map(Self)
    }
    pub fn register_proc_header(
        &mut self,
        queue: &etalon::Queue,
        header: &ProceduralSynthesisHeader,
    ) -> Option<u16> {
        self.0
            .register_proc_header(queue, &ProceduralSynthesisGpu::from_header(header))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::procedural_synthesis::{
        FLAG_CELLULAR_VORONOI, FLAG_PGA_MOTOR_DYNAMICS,
    };

    #[test]
    fn slab_range_allocator_allocates_aligns_and_coalesces_zero_heap() {
        let mut alloc = SlabRangeAllocator::<16>::new(1024);
        assert_eq!(alloc.capacity(), 1024);
        assert_eq!(alloc.free_remaining(), 1024);
        assert_eq!(alloc.free_range_count(), 1);

        let a = alloc.allocate(100, 1).expect("alloc a");
        let b = alloc.allocate(200, 16).expect("alloc b aligned 16");
        let c = alloc.allocate(50, 1).expect("alloc c");
        assert_eq!(a, 0);
        assert_eq!(b % 16, 0);
        assert!(b >= 100);
        assert_eq!(c, b + 200);

        // Free out of order and verify bidirectional coalescing restores a single contiguous span
        assert!(alloc.free(b, 200));
        assert!(alloc.free(a, 100));
        assert!(alloc.free(c, 50));
        assert_eq!(alloc.used(), 0);
        assert_eq!(alloc.free_remaining(), 1024);
        assert_eq!(alloc.free_range_count(), 1);
    }

    #[test]
    fn procedural_synthesis_gpu_mirrors_header_in_64_bytes() {
        let header = ProceduralSynthesisHeader {
            magic: MAGIC_PROC,
            version: 1,
            flags: (FLAG_CELLULAR_VORONOI | FLAG_PGA_MOTOR_DYNAMICS) as u16,
            seed: 0xDEAD_BEEF_CAFE_BABEu64,
            voronoi_frequency: 24.0,
            voronoi_jitter: 0.9,
            fillet_blend_radius: 0.25,
            fillet_threshold: 0.05,
            allometry_exponent_k: 1.5,
            allometry_base_b: 0.8,
            spiral_growth_rate_k: 0.22,
            spiral_initial_radius_a: 0.15,
            motor_frequency_hz: 1.8,
            motor_damping_ratio: 0.05,
            critical_stress_threshold: 60.0,
            reserved_f32: 0.0,
        };
        let gpu = ProceduralSynthesisGpu::from_header(&header);
        assert_eq!(std::mem::size_of::<ProceduralSynthesisGpu>(), 64);
        assert!(gpu.is_valid());
        assert_eq!(
            gpu.flags(),
            (FLAG_CELLULAR_VORONOI | FLAG_PGA_MOTOR_DYNAMICS) as u16
        );
        assert_eq!(gpu.to_header(), header);
    }
}
