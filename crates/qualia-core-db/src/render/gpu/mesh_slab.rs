//! Unified Multi-Modal GPU Mesh Slab & `.10d` Living Container Residency (`MeshSlabArenaGpu`).
//!
//! Implements Epic 2 of the Superior Game Engine Master Plan:
//! - Zero-heap `SlabRangeAllocator<MAX_RANGES>` sub-allocator for GPU vertex/index buffers.
//! - Multi-modal `ResidentAssetEntry` tracking 4 LOD tiers across B-Rep meshes, Holographic
//!   Proxy-Lens SDFs, `.10d` `MAGIC_SURF` Gaussian surfels, `SGGX` microflake canopies, and
//!   `.10d` `MAGIC_IMPO` hemi-octahedral impostors.
//! - Direct GPU upload of `.10d` `MAGIC_PROC` `ProceduralSynthesisHeader` (`ProceduralSynthesisGpu`,
//!   64 bytes) for GPU-evaluated Voronoi micro-relief, smooth-minimum SDFs, Huxley allometry,
//!   logarithmic spirals, and PGA motor harmonic oscillations.

use crate::container_10d::procedural_synthesis::{MAGIC_PROC, ProceduralSynthesisHeader};
use crate::gpu_context::{global_vram_ledger, VramReservation, VramResourceClass};
use bytemuck::{Pod, Zeroable};

/// Representation kind for a `SubmeshSlice` LOD slot (`0..=4`).
pub const REP_KIND_MESH_BREP: u32 = 0;
pub const REP_KIND_HOLOGRAPHIC_SDF_PROXY: u32 = 1;
pub const REP_KIND_SURFEL_CLOUD: u32 = 2;
pub const REP_KIND_SGGX_CANOPY: u32 = 3;
pub const REP_KIND_OCTAHEDRAL_IMPOSTOR: u32 = 4;

/// Sentinel index for an absent `.10d` `PROC1` or `EONT1` header slot.
pub const INVALID_HEADER_SLOT: u16 = 0xFFFF;

/// Maximum resident assets tracked by the zero-heap slab table in portable mode.
pub const MAX_SLAB_RESIDENT_ASSETS: usize = 512;
/// Maximum resident `.10d` `PROC1` synthesis headers in the GPU storage buffer.
pub const MAX_SLAB_PROC_HEADERS: usize = 256;

/// Typed representation archetype for a LOD tier in `ResidentAssetEntry`.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetRepresentationKind {
    MeshBRep = REP_KIND_MESH_BREP,
    HolographicSdfProxy = REP_KIND_HOLOGRAPHIC_SDF_PROXY,
    SurfelCloud = REP_KIND_SURFEL_CLOUD,
    SggxCanopy = REP_KIND_SGGX_CANOPY,
    OctahedralImpostor = REP_KIND_OCTAHEDRAL_IMPOSTOR,
}

/// Contiguous free span `[offset .. offset + length)` inside a GPU buffer slab.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct SlabRange {
    pub offset: u32,
    pub length: u32,
}

/// Zero-heap first-fit range sub-allocator with immediate bidirectional coalescing.
#[derive(Clone, Debug)]
pub struct SlabRangeAllocator<const MAX_RANGES: usize = 512> {
    free_ranges: [SlabRange; MAX_RANGES],
    free_count: usize,
    capacity: u32,
    used: u32,
}

impl<const MAX_RANGES: usize> SlabRangeAllocator<MAX_RANGES> {
    /// Creates a new zero-heap range allocator managing `[0 .. capacity)`.
    #[inline]
    pub const fn new(capacity: u32) -> Self {
        let mut free_ranges = [SlabRange {
            offset: 0,
            length: 0,
        }; MAX_RANGES];
        let free_count = if capacity > 0 && MAX_RANGES > 0 {
            free_ranges[0] = SlabRange {
                offset: 0,
                length: capacity,
            };
            1
        } else {
            0
        };
        Self {
            free_ranges,
            free_count,
            capacity,
            used: 0,
        }
    }

    /// Total element capacity of the slab.
    #[inline]
    pub const fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Currently allocated element count (including alignment padding).
    #[inline]
    pub const fn used(&self) -> u32 {
        self.used
    }

    /// Remaining free element capacity.
    #[inline]
    pub const fn free_remaining(&self) -> u32 {
        self.capacity.saturating_sub(self.used)
    }

    /// Number of coalesced free spans currently tracked.
    #[inline]
    pub const fn free_range_count(&self) -> usize {
        self.free_count
    }

    /// Allocates `count` contiguous elements aligned to `alignment` (power of two or `>= 1`).
    /// Performs zero heap allocations.
    pub fn allocate(&mut self, count: u32, alignment: u32) -> Option<u32> {
        if count == 0 {
            return Some(0);
        }
        let align = alignment.max(1);
        for idx in 0..self.free_count {
            let range = self.free_ranges[idx];
            let rem = range.offset % align;
            let pad = if rem == 0 { 0 } else { align - rem };
            let total_needed = count.checked_add(pad)?;
            if range.length >= total_needed {
                let aligned_offset = range.offset + pad;
                let tail_start = aligned_offset + count;
                let range_end = range.offset + range.length;
                let tail_len = range_end - tail_start;

                if pad == 0 && tail_len == 0 {
                    // Exact match: remove free slot by shifting left
                    for k in idx..self.free_count - 1 {
                        self.free_ranges[k] = self.free_ranges[k + 1];
                    }
                    self.free_count -= 1;
                } else if pad == 0 {
                    // Trim front of current range
                    self.free_ranges[idx] = SlabRange {
                        offset: tail_start,
                        length: tail_len,
                    };
                } else if tail_len == 0 {
                    // Keep only front alignment padding
                    self.free_ranges[idx] = SlabRange {
                        offset: range.offset,
                        length: pad,
                    };
                } else {
                    // Split into front pad and tail remainder if capacity permits
                    if self.free_count >= MAX_RANGES {
                        continue;
                    }
                    for k in (idx + 1..=self.free_count).rev() {
                        self.free_ranges[k] = self.free_ranges[k - 1];
                    }
                    self.free_ranges[idx] = SlabRange {
                        offset: range.offset,
                        length: pad,
                    };
                    self.free_ranges[idx + 1] = SlabRange {
                        offset: tail_start,
                        length: tail_len,
                    };
                    self.free_count += 1;
                }
                self.used = self.used.saturating_add(count);
                return Some(aligned_offset);
            }
        }
        None
    }

    /// Frees `[offset .. offset + count)` and coalesces with adjacent free spans in-place.
    pub fn free(&mut self, offset: u32, count: u32) -> bool {
        if count == 0 {
            return true;
        }
        let Some(end) = offset.checked_add(count) else {
            return false;
        };
        if end > self.capacity {
            return false;
        }

        // Find sorted insertion point by offset
        let mut insert_idx = 0;
        while insert_idx < self.free_count && self.free_ranges[insert_idx].offset < offset {
            insert_idx += 1;
        }

        // Reject overlapping frees
        if insert_idx > 0 {
            let prev = self.free_ranges[insert_idx - 1];
            if prev.offset + prev.length > offset {
                return false;
            }
        }
        if insert_idx < self.free_count {
            let next = self.free_ranges[insert_idx];
            if end > next.offset {
                return false;
            }
        }

        let merge_prev = insert_idx > 0 && {
            let prev = self.free_ranges[insert_idx - 1];
            prev.offset + prev.length == offset
        };
        let merge_next = insert_idx < self.free_count && {
            let next = self.free_ranges[insert_idx];
            end == next.offset
        };

        match (merge_prev, merge_next) {
            (true, true) => {
                let next_len = self.free_ranges[insert_idx].length;
                self.free_ranges[insert_idx - 1].length += count + next_len;
                for k in insert_idx..self.free_count - 1 {
                    self.free_ranges[k] = self.free_ranges[k + 1];
                }
                self.free_count -= 1;
            }
            (true, false) => {
                self.free_ranges[insert_idx - 1].length += count;
            }
            (false, true) => {
                self.free_ranges[insert_idx].offset = offset;
                self.free_ranges[insert_idx].length += count;
            }
            (false, false) => {
                if self.free_count >= MAX_RANGES {
                    return false;
                }
                for k in (insert_idx + 1..=self.free_count).rev() {
                    self.free_ranges[k] = self.free_ranges[k - 1];
                }
                self.free_ranges[insert_idx] = SlabRange {
                    offset,
                    length: count,
                };
                self.free_count += 1;
            }
        }
        self.used = self.used.saturating_sub(count);
        true
    }
}

/// 16-byte LOD slice descriptor within `MeshSlabArenaGpu`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct SubmeshSlice {
    pub first_index: u32,
    pub index_count: u32,
    pub base_vertex: i32,
    /// `0 = MeshBRep`, `1 = HolographicSdfProxy`, `2 = SurfelCloud`, `3 = SggxCanopy`, `4 = OctahedralImpostor`.
    pub rep_kind: u32,
}

const _: [(); 16] = [(); std::mem::size_of::<SubmeshSlice>()];

/// 112-byte multi-modal resident asset entry tracking 4 LOD tiers and `.10d` `PROC`/`EONT`/`SURF`/`IMPO` bindings.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct ResidentAssetEntry {
    pub asset_id: u32,
    /// `.10d` `MAGIC_PROC` `ProceduralSynthesisHeader` slot (`0xFFFF` if none).
    pub proc_header_index: u16,
    /// `.10d` `MAGIC_EONT` `EntityOntologyMiniHeader` slot (`0xFFFF` if none).
    pub eont_header_index: u16,
    /// Offset into the `.10d` `MAGIC_SURF` surfel storage buffer.
    pub surfel_slice_offset: u32,
    /// Number of Gaussian covariance surfels for this asset.
    pub surfel_count: u32,
    /// `.10d` `MAGIC_IMPO` hemi-octahedral impostor atlas layer.
    pub impostor_atlas_layer: u16,
    /// Bit flags (bit 0 = active, bit 1 = casts shadow, bit 2 = skinned).
    pub flags: u16,
    /// Vertex count allocated in `vertex_buffer` for cleanup on `free_asset`.
    pub total_vertex_offset: u32,
    pub total_vertex_count: u32,
    pub reserved_u32: u32,
    /// Local bounding sphere `[center_x, center_y, center_z, radius]`.
    pub bounding_sphere: [f32; 4],
    /// 4-tier LOD slices (`LOD0` near .. `LOD3` far).
    pub lod_slices: [SubmeshSlice; 4],
}

const _: [(); 112] = [(); std::mem::size_of::<ResidentAssetEntry>()];

impl Default for ResidentAssetEntry {
    fn default() -> Self {
        Self {
            asset_id: 0,
            proc_header_index: INVALID_HEADER_SLOT,
            eont_header_index: INVALID_HEADER_SLOT,
            surfel_slice_offset: 0,
            surfel_count: 0,
            impostor_atlas_layer: 0,
            flags: 0,
            total_vertex_offset: 0,
            total_vertex_count: 0,
            reserved_u32: 0,
            bounding_sphere: [0.0, 0.0, 0.0, 1.0],
            lod_slices: [SubmeshSlice::default(); 4],
        }
    }
}

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

/// 64-byte interleaved vertex record inside `MeshSlabArenaGpu::vertex_buffer`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct MeshSlabVertex {
    pub position: [f32; 3],
    pub uv0_x: f32,
    pub normal: [f32; 3],
    pub uv0_y: f32,
    pub tangent: [f32; 4],
    pub color: [f32; 4],
}

const _: [(); 64] = [(); std::mem::size_of::<MeshSlabVertex>()];

/// 24-byte skeletal skinning vertex attribute record (`JOINTS_0` + `WEIGHTS_0`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct SkinAttrVertex {
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}

const _: [(); 24] = [(); std::mem::size_of::<SkinAttrVertex>()];

/// Capacity configuration for `MeshSlabArenaGpu`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshSlabArenaDescriptor {
    pub max_vertices: u32,
    pub max_indices: u32,
    pub max_assets: u32,
    pub max_proc_headers: u32,
    pub max_indirect_commands: u32,
}

impl Default for MeshSlabArenaDescriptor {
    fn default() -> Self {
        Self {
            max_vertices: 65_536,
            max_indices: 196_608,
            max_assets: MAX_SLAB_RESIDENT_ASSETS as u32,
            max_proc_headers: MAX_SLAB_PROC_HEADERS as u32,
            max_indirect_commands: 2_048,
        }
    }
}

/// Unified GPU mesh & multi-modal `.10d` asset residency arena.
pub struct MeshSlabArenaGpu {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub skin_attr_buffer: wgpu::Buffer,
    pub proc_headers_buffer: wgpu::Buffer,
    pub asset_table_buffer: wgpu::Buffer,
    pub indirect_args_buffer: wgpu::Buffer,
    pub vertex_allocator: SlabRangeAllocator<512>,
    pub index_allocator: SlabRangeAllocator<512>,
    pub assets: [ResidentAssetEntry; MAX_SLAB_RESIDENT_ASSETS],
    pub proc_headers: [ProceduralSynthesisGpu; MAX_SLAB_PROC_HEADERS],
    pub asset_count: u32,
    pub proc_header_count: u32,
    pub descriptor: MeshSlabArenaDescriptor,
    _vram_reservation: VramReservation<'static>,
}

impl MeshSlabArenaGpu {
    /// Computes the total VRAM bytes required for a `MeshSlabArenaDescriptor`.
    pub fn required_vram_bytes(desc: MeshSlabArenaDescriptor) -> Option<u64> {
        let v_bytes = u64::from(desc.max_vertices.max(1))
            .checked_mul(std::mem::size_of::<MeshSlabVertex>() as u64)?;
        let i_bytes =
            u64::from(desc.max_indices.max(1)).checked_mul(std::mem::size_of::<u32>() as u64)?;
        let s_bytes = u64::from(desc.max_vertices.max(1))
            .checked_mul(std::mem::size_of::<SkinAttrVertex>() as u64)?;
        let p_bytes = u64::from(desc.max_proc_headers.max(1))
            .checked_mul(std::mem::size_of::<ProceduralSynthesisGpu>() as u64)?;
        let a_bytes = u64::from(desc.max_assets.max(1))
            .checked_mul(std::mem::size_of::<ResidentAssetEntry>() as u64)?;
        let cmd_bytes = u64::from(desc.max_indirect_commands.max(1)).checked_mul(20)?;
        v_bytes
            .checked_add(i_bytes)?
            .checked_add(s_bytes)?
            .checked_add(p_bytes)?
            .checked_add(a_bytes)?
            .checked_add(cmd_bytes)
    }

    /// Allocates the unified GPU slab buffers under `VramLedger` governance.
    pub fn try_new(device: &wgpu::Device, desc: MeshSlabArenaDescriptor) -> Option<Self> {
        let total_bytes = Self::required_vram_bytes(desc)?;
        let reservation = global_vram_ledger()
            .try_reserve_graphics(VramResourceClass::Geometry, total_bytes)
            .ok()?;

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia-mesh-slab-vertices"),
            size: u64::from(desc.max_vertices.max(1)) * std::mem::size_of::<MeshSlabVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia-mesh-slab-indices"),
            size: u64::from(desc.max_indices.max(1)) * std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let skin_attr_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia-mesh-slab-skin-attrs"),
            size: u64::from(desc.max_vertices.max(1)) * std::mem::size_of::<SkinAttrVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let proc_headers_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia-mesh-slab-proc-headers"),
            size: u64::from(desc.max_proc_headers.max(1))
                * std::mem::size_of::<ProceduralSynthesisGpu>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let asset_table_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia-mesh-slab-asset-table"),
            size: u64::from(desc.max_assets.max(1))
                * std::mem::size_of::<ResidentAssetEntry>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let indirect_args_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia-mesh-slab-indirect-args"),
            size: u64::from(desc.max_indirect_commands.max(1)) * 20,
            usage: wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Some(Self {
            vertex_buffer,
            index_buffer,
            skin_attr_buffer,
            proc_headers_buffer,
            asset_table_buffer,
            indirect_args_buffer,
            vertex_allocator: SlabRangeAllocator::new(desc.max_vertices),
            index_allocator: SlabRangeAllocator::new(desc.max_indices),
            assets: [ResidentAssetEntry::default(); MAX_SLAB_RESIDENT_ASSETS],
            proc_headers: [ProceduralSynthesisGpu::default(); MAX_SLAB_PROC_HEADERS],
            asset_count: 0,
            proc_header_count: 0,
            descriptor: desc,
            _vram_reservation: reservation,
        })
    }

    /// Uploads a `.10d` `PROC1` header into `proc_headers_buffer` and returns its slot index.
    pub fn register_proc_header(
        &mut self,
        queue: &wgpu::Queue,
        header: &ProceduralSynthesisHeader,
    ) -> Option<u16> {
        let slot = self.proc_header_count as usize;
        if slot >= MAX_SLAB_PROC_HEADERS || slot >= self.descriptor.max_proc_headers as usize {
            return None;
        }
        let gpu_header = ProceduralSynthesisGpu::from_header(header);
        self.proc_headers[slot] = gpu_header;
        self.proc_header_count += 1;
        let byte_offset = (slot * std::mem::size_of::<ProceduralSynthesisGpu>()) as u64;
        queue.write_buffer(
            &self.proc_headers_buffer,
            byte_offset,
            bytemuck::bytes_of(&gpu_header),
        );
        Some(slot as u16)
    }

    /// Allocates vertex/index slab space and uploads a multi-LOD asset entry into the unified arena.
    pub fn register_mesh_lod_asset(
        &mut self,
        queue: &wgpu::Queue,
        asset_id: u32,
        vertices: &[MeshSlabVertex],
        skin_attrs: Option<&[SkinAttrVertex]>,
        lod_indices: [&[u32]; 4],
        lod_rep_kinds: [u32; 4],
        bounding_sphere: [f32; 4],
        proc_header_index: u16,
        eont_header_index: u16,
    ) -> Option<u32> {
        let slot = self.asset_count as usize;
        if slot >= MAX_SLAB_RESIDENT_ASSETS || slot >= self.descriptor.max_assets as usize {
            return None;
        }
        let v_count = u32::try_from(vertices.len()).ok()?;
        let base_vertex = self.vertex_allocator.allocate(v_count, 1)?;

        let total_indices: u32 = lod_indices
            .iter()
            .try_fold(0u32, |acc, slice| acc.checked_add(u32::try_from(slice.len()).ok()?))?;
        let Some(first_index_base) = self.index_allocator.allocate(total_indices, 1) else {
            self.vertex_allocator.free(base_vertex, v_count);
            return None;
        };

        if !vertices.is_empty() {
            let v_byte_offset =
                u64::from(base_vertex) * std::mem::size_of::<MeshSlabVertex>() as u64;
            queue.write_buffer(
                &self.vertex_buffer,
                v_byte_offset,
                bytemuck::cast_slice(vertices),
            );
            if let Some(skin) = skin_attrs {
                let len = skin.len().min(vertices.len());
                let s_byte_offset =
                    u64::from(base_vertex) * std::mem::size_of::<SkinAttrVertex>() as u64;
                queue.write_buffer(
                    &self.skin_attr_buffer,
                    s_byte_offset,
                    bytemuck::cast_slice(&skin[..len]),
                );
            }
        }

        let mut lod_slices = [SubmeshSlice::default(); 4];
        let mut cursor = first_index_base;
        for lod in 0..4 {
            let idx_slice = lod_indices[lod];
            let count = idx_slice.len() as u32;
            if count > 0 {
                let i_byte_offset = u64::from(cursor) * std::mem::size_of::<u32>() as u64;
                queue.write_buffer(
                    &self.index_buffer,
                    i_byte_offset,
                    bytemuck::cast_slice(idx_slice),
                );
            }
            lod_slices[lod] = SubmeshSlice {
                first_index: cursor,
                index_count: count,
                base_vertex: base_vertex as i32,
                rep_kind: lod_rep_kinds[lod],
            };
            cursor += count;
        }

        let entry = ResidentAssetEntry {
            asset_id,
            proc_header_index,
            eont_header_index,
            surfel_slice_offset: 0,
            surfel_count: 0,
            impostor_atlas_layer: 0,
            flags: 1 | if skin_attrs.is_some() { 4 } else { 0 },
            total_vertex_offset: base_vertex,
            total_vertex_count: v_count,
            reserved_u32: 0,
            bounding_sphere,
            lod_slices,
        };
        self.assets[slot] = entry;
        self.asset_count += 1;
        let entry_offset = (slot * std::mem::size_of::<ResidentAssetEntry>()) as u64;
        queue.write_buffer(
            &self.asset_table_buffer,
            entry_offset,
            bytemuck::bytes_of(&entry),
        );
        Some(slot as u32)
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
