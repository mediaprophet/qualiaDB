//! Section 22 (`VOX0`): 3D Sparse Voxel Octree / DAG Lattice Section.
//!
//! Encodes volumetric underground strata, cavities, root networks, and CSG excavations
//! using a 16-byte cache-friendly Directed Acyclic Graph (`Vox0DagNode`).
//!
//! # 16-Byte POD Binary Layout (`Vox0DagNode`)
//!
//! | Offset | Type | Field | Description |
//! |---|---|---|---|
//! | `0x00` | `u8` | `child_mask` | Valid child octant presence bitmask (8 bits) |
//! | `0x01` | `u8` | `leaf_mask` | Octants terminating as leaves (8 bits) |
//! | `0x02` | `u8` | `primary_material_lut` | Majority constituent LUT index (0..255) |
//! | `0x03` | `u8` | `secondary_material_lut` | Secondary constituent LUT index (0..255) |
//! | `0x04` | `u32` | `child_ptr_or_density` | SVO index to child cluster, or packed density |
//! | `0x08` | `[i8; 2]` | `normal_oct_snorm8` | Octahedral encoded surface normal vector |
//! | `0x0A` | `u8` | `moisture_u8` | Volumetric water saturation Sw (0..255) |
//! | `0x0B` | `u8` | `compaction_u8` | Overburden compaction / hardness metric |
//! | `0x0C` | `u16` | `node_crc16` | Integrity checksum over bytes 0..11 |
//! | `0x0E` | `[u8; 2]` | `_reserved` | Explicit zero padding to 16 bytes |

use bytemuck::{bytes_of, cast_slice, Pod, Zeroable};

pub const VOXEL_SECTION_VERSION: u16 = 1;
pub const VOXEL_MINI_HEADER_SIZE: usize = 64;
pub const VOX0_NODE_SIZE: usize = 16;
pub const MAGIC_VOX0: [u8; 4] = *b"VOX0";

pub const FLAG_VOXEL_COMPACT_DAG: u16 = 1 << 0;
pub const FLAG_VOXEL_HAS_EXCAVATIONS: u16 = 1 << 1;

/// 64-Byte POD Mini Header for SectionType::VoxelLattice (22).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct VoxelSectionMiniHeader {
    /// 4B: ASCII magic bytes "VOX0".
    pub magic: [u8; 4],
    /// 2B: Section version (1).
    pub version: u16,
    /// 2B: Feature flags (e.g. compact DAG vs octree).
    pub flags: u16,
    /// 4B: Total count of 16-byte Vox0DagNode records.
    pub node_count: u32,
    /// 4B: Stride of each node record in bytes (16).
    pub stride: u32,
    /// 4B: Array index of the root node.
    pub root_node_index: u32,
    /// 2B: Maximum tree depth.
    pub max_depth: u16,
    /// 2B: Explicit padding.
    pub _pad: u16,
    /// 12B: Minimum spatial bounding box in local millimeters.
    pub aabb_min: [f32; 3],
    /// 12B: Maximum spatial bounding box in local millimeters.
    pub aabb_max: [f32; 3],
    /// 4B: Leaf voxel spatial resolution in millimeters.
    pub voxel_resolution_mm: f32,
    /// 12B: Reserved padding to reach 64 bytes.
    pub _reserved: [u8; 12],
}

impl Default for VoxelSectionMiniHeader {
    fn default() -> Self {
        Self {
            magic: MAGIC_VOX0,
            version: VOXEL_SECTION_VERSION,
            flags: 0,
            node_count: 0,
            stride: VOX0_NODE_SIZE as u32,
            root_node_index: 0,
            max_depth: 8,
            _pad: 0,
            aabb_min: [-32000.0, -32000.0, -10000.0],
            aabb_max: [32000.0, 32000.0, 10000.0],
            voxel_resolution_mm: 250.0, // 25cm default leaf voxel
            _reserved: [0; 12],
        }
    }
}

/// 16-Byte POD Node Record for Sparse Voxel Octrees and DAGs.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Vox0DagNode {
    /// 1B: Valid octant child bitmask (bits 0..7).
    pub child_mask: u8,
    /// 1B: Bitmask of octants that terminate as solid leaves (bits 0..7).
    pub leaf_mask: u8,
    /// 1B: Primary constituent LUT index (0..255).
    pub primary_material_lut: u8,
    /// 1B: Secondary constituent LUT index (0..255).
    pub secondary_material_lut: u8,
    /// 4B: Child pointer into node array (when non-leaf) or packed density.
    pub child_ptr_or_density: u32,
    /// 2B: Octahedrally-encoded surface normal vector.
    pub normal_oct_snorm8: [i8; 2],
    /// 1B: Volumetric moisture saturation Sw (0..255 maps to 0..100%).
    pub moisture_u8: u8,
    /// 1B: Soil compaction / hardness metric (0..255).
    pub compaction_u8: u8,
    /// 2B: CRC-16 checksum of bytes 0..11.
    pub node_crc16: u16,
    /// 2B: Explicit zero padding to 16 bytes.
    pub _reserved: [u8; 2],
}

impl Default for Vox0DagNode {
    fn default() -> Self {
        Self {
            child_mask: 0,
            leaf_mask: 0xFF, // Solid leaf by default
            primary_material_lut: 1, // Topsoil default
            secondary_material_lut: 1,
            child_ptr_or_density: 0,
            normal_oct_snorm8: [0, 127], // Up vector
            moisture_u8: 128,
            compaction_u8: 100,
            node_crc16: 0,
            _reserved: [0; 2],
        }
    }
}

impl Vox0DagNode {
    /// Returns true if the child octant (0..7) exists in the lattice.
    #[inline]
    pub fn has_child(&self, octant: usize) -> bool {
        (self.child_mask & (1 << (octant & 7))) != 0
    }

    /// Returns true if the child octant (0..7) terminates as a leaf.
    #[inline]
    pub fn is_leaf(&self, octant: usize) -> bool {
        (self.leaf_mask & (1 << (octant & 7))) != 0
    }

    /// Compute simple CRC-16 (IBM polynomial 0xA001) over bytes 0..11.
    #[inline]
    pub fn compute_crc16(&self) -> u16 {
        let bytes: &[u8; VOX0_NODE_SIZE] = bytemuck::cast_ref(self);
        let mut crc = 0xFFFFu16;
        for &b in &bytes[..12] {
            crc ^= b as u16;
            for _ in 0..8 {
                if (crc & 1) != 0 {
                    crc = (crc >> 1) ^ 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }
        crc
    }

    /// Update `node_crc16` with the computed checksum.
    #[inline]
    pub fn update_crc(&mut self) {
        self.node_crc16 = self.compute_crc16();
    }

    /// Validate the CRC-16 of this node.
    #[inline]
    pub fn validate(&self) -> bool {
        self.node_crc16 == self.compute_crc16()
    }
}

/// Errors during voxel section encode/decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoxelSectionError {
    BufferTooSmall { required: usize, provided: usize },
    InvalidMagic,
    UnsupportedVersion(u16),
    InvalidStride(u32),
    NodeCountOverflow,
    MisalignedBuffer,
}

/// Encode a voxel section into a caller-supplied fixed slice without heap allocation.
pub fn encode_voxel_section(
    header: &VoxelSectionMiniHeader,
    nodes: &[Vox0DagNode],
    out: &mut [u8],
) -> Result<usize, VoxelSectionError> {
    let required_len = VOXEL_MINI_HEADER_SIZE + nodes.len() * VOX0_NODE_SIZE;
    if out.len() < required_len {
        return Err(VoxelSectionError::BufferTooSmall {
            required: required_len,
            provided: out.len(),
        });
    }

    let mut header_copy = *header;
    header_copy.magic = MAGIC_VOX0;
    header_copy.version = VOXEL_SECTION_VERSION;
    header_copy.node_count = nodes.len() as u32;
    header_copy.stride = VOX0_NODE_SIZE as u32;

    out[..VOXEL_MINI_HEADER_SIZE].copy_from_slice(bytes_of(&header_copy));
    let nodes_bytes: &[u8] = cast_slice(nodes);
    out[VOXEL_MINI_HEADER_SIZE..required_len].copy_from_slice(nodes_bytes);

    Ok(required_len)
}

/// Zero-copy parse a voxel section from a byte slice.
pub fn parse_voxel_section(
    bytes: &[u8],
) -> Result<(&VoxelSectionMiniHeader, &[Vox0DagNode]), VoxelSectionError> {
    if bytes.len() < VOXEL_MINI_HEADER_SIZE {
        return Err(VoxelSectionError::BufferTooSmall {
            required: VOXEL_MINI_HEADER_SIZE,
            provided: bytes.len(),
        });
    }

    let (header_bytes, payload) = bytes.split_at(VOXEL_MINI_HEADER_SIZE);
    let header: &VoxelSectionMiniHeader = bytemuck::try_from_bytes(header_bytes)
        .map_err(|_| VoxelSectionError::MisalignedBuffer)?;

    if header.magic != MAGIC_VOX0 {
        return Err(VoxelSectionError::InvalidMagic);
    }
    if header.version != VOXEL_SECTION_VERSION {
        return Err(VoxelSectionError::UnsupportedVersion(header.version));
    }
    if header.stride != VOX0_NODE_SIZE as u32 {
        return Err(VoxelSectionError::InvalidStride(header.stride));
    }

    let expected_payload_len = header.node_count as usize * VOX0_NODE_SIZE;
    if payload.len() < expected_payload_len {
        return Err(VoxelSectionError::BufferTooSmall {
            required: expected_payload_len,
            provided: payload.len(),
        });
    }

    let nodes: &[Vox0DagNode] = bytemuck::try_cast_slice(&payload[..expected_payload_len])
        .map_err(|_| VoxelSectionError::MisalignedBuffer)?;

    Ok((header, nodes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vox0_dag_node_size_and_alignment() {
        assert_eq!(
            std::mem::size_of::<Vox0DagNode>(),
            16,
            "Vox0DagNode must be exactly 16 bytes"
        );
        assert_eq!(
            std::mem::align_of::<Vox0DagNode>(),
            16,
            "Vox0DagNode must have 16-byte alignment"
        );
    }

    #[test]
    fn test_voxel_section_mini_header_size() {
        assert_eq!(
            std::mem::size_of::<VoxelSectionMiniHeader>(),
            64,
            "VoxelSectionMiniHeader must be exactly 64 bytes"
        );
    }

    #[test]
    fn test_voxel_section_encode_decode_roundtrip() {
        let mut header = VoxelSectionMiniHeader::default();
        header.voxel_resolution_mm = 125.0;

        let mut node1 = Vox0DagNode::default();
        node1.child_mask = 0b0000_1111;
        node1.primary_material_lut = 2; // Clay
        node1.update_crc();
        assert!(node1.validate());

        let mut node2 = Vox0DagNode::default();
        node2.child_mask = 0b1111_0000;
        node2.primary_material_lut = 5; // Basalt
        node2.update_crc();
        assert!(node2.validate());

        let nodes = [node1, node2];
        #[repr(align(64))]
        struct AlignedBuffer([u8; 256]);
        let mut buffer = AlignedBuffer([0u8; 256]);

        let written = encode_voxel_section(&header, &nodes, &mut buffer.0).expect("encode succeeds");
        assert_eq!(written, 64 + 32);

        let (parsed_header, parsed_nodes) =
            parse_voxel_section(&buffer.0[..written]).expect("parse succeeds");

        assert_eq!(parsed_header.magic, MAGIC_VOX0);
        assert_eq!(parsed_header.node_count, 2);
        assert_eq!(parsed_header.voxel_resolution_mm, 125.0);
        assert_eq!(parsed_nodes.len(), 2);
        assert_eq!(parsed_nodes[0].primary_material_lut, 2);
        assert_eq!(parsed_nodes[1].primary_material_lut, 5);
        assert!(parsed_nodes[0].validate());
        assert!(parsed_nodes[1].validate());
    }

    #[test]
    fn test_voxel_section_error_handling() {
        let header = VoxelSectionMiniHeader::default();
        let nodes = [Vox0DagNode::default()];
        let mut small_buf = [0u8; 32]; // Less than 64B header

        // Buffer too small for encode
        assert!(matches!(
            encode_voxel_section(&header, &nodes, &mut small_buf),
            Err(VoxelSectionError::BufferTooSmall { .. })
        ));

        // Buffer too small for parse
        assert!(matches!(
            parse_voxel_section(&small_buf),
            Err(VoxelSectionError::BufferTooSmall { .. })
        ));

        // Corrupted magic with aligned buffer
        #[repr(align(64))]
        struct AlignedBuffer([u8; 128]);
        let mut valid_buf = AlignedBuffer([0u8; 128]);
        let written = encode_voxel_section(&header, &nodes, &mut valid_buf.0).unwrap();
        valid_buf.0[0] = b'X'; // Corrupt magic "XOX0"
        assert!(matches!(
            parse_voxel_section(&valid_buf.0[..written]),
            Err(VoxelSectionError::InvalidMagic)
        ));
    }
}
