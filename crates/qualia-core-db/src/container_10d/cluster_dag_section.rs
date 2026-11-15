//! Section 23 (`CDAG`): Nanite-Style Hierarchical Meshlet Cluster DAG Section.
//!
//! Encodes a continuous level-of-detail (CLOD) directed acyclic graph of 64-byte
//! cache-line-aligned meshlet clusters (`MeshletClusterNode`) preceded by a 32-byte
//! header (`ClusterDagMiniHeader`).
//!
//! Each cluster stores its own bounding sphere and quadric error (`lod_error_self`)
//! alongside its parent group's bounding sphere and error (`lod_error_parent`),
//! satisfying the monotonic DAG cut invariant `lod_error_parent >= lod_error_self`.
//! At runtime, GPU compute or CPU zero-heap traversal selects the cut where:
//! `projected_error(self) <= threshold_px < projected_error(parent)`.

use bytemuck::{bytes_of, pod_read_unaligned, Pod, Zeroable};

/// 4-byte ASCII magic for `.10d` Cluster DAG sections (`"CDAG"`).
pub const MAGIC_CDAG: [u8; 4] = *b"CDAG";

/// Current format version of the `CDAG` section.
pub const CLUSTER_DAG_SECTION_VERSION: u16 = 1;

/// Fixed byte size of [`ClusterDagMiniHeader`].
pub const CLUSTER_DAG_HEADER_SIZE: usize = 32;

/// Fixed byte size of [`MeshletClusterNode`].
pub const MESHLET_CLUSTER_NODE_SIZE: usize = 64;

/// Maximum vertices allowed in a single hardware meshlet cluster.
pub const MAX_MESHLET_VERTICES: u8 = 64;

/// Maximum triangles allowed in a single hardware meshlet cluster.
pub const MAX_MESHLET_TRIANGLES: u8 = 124;

/// Maximum cluster nodes admitted in a single `CDAG` section.
pub const MAX_CLUSTER_DAG_NODES: usize = 262_144;

/// Cluster border vertices were seam-locked during hierarchical QEM simplification.
pub const FLAG_CDAG_SEAM_LOCKED: u32 = 1 << 0;

/// Normal cone axis and cutoff are populated for backface cluster culling.
pub const FLAG_CDAG_CONE_CULLED: u32 = 1 << 1;

/// 32-byte POD mini-header for `SectionType::ClusterDag` (`CDAG`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct ClusterDagMiniHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub max_lod_depth: u16,
    pub cluster_count: u32,
    pub group_count: u32,
    pub total_meshlet_vertices: u32,
    pub total_meshlet_triangles: u32,
    pub root_cluster_offset: u32,
    pub flags: u32,
}

const _: () = assert!(core::mem::size_of::<ClusterDagMiniHeader>() == CLUSTER_DAG_HEADER_SIZE);

impl Default for ClusterDagMiniHeader {
    fn default() -> Self {
        Self {
            magic: MAGIC_CDAG,
            version: CLUSTER_DAG_SECTION_VERSION,
            max_lod_depth: 0,
            cluster_count: 0,
            group_count: 0,
            total_meshlet_vertices: 0,
            total_meshlet_triangles: 0,
            root_cluster_offset: 0,
            flags: FLAG_CDAG_SEAM_LOCKED | FLAG_CDAG_CONE_CULLED,
        }
    }
}

/// 64-byte cache-line-aligned POD node representing one meshlet in the Cluster DAG.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct MeshletClusterNode {
    /// Bounding sphere `[cx, cy, cz, radius]` in local meters (16B).
    pub bounding_sphere: [f32; 4],
    /// Parent cluster-group bounding sphere `[px, py, pz, parent_radius]` (16B).
    pub parent_error_sphere: [f32; 4],
    /// World-space QEM geometric error of this cluster in meters (4B).
    pub lod_error_self: f32,
    /// Monotonic parent group error (`>= lod_error_self`; `f32::INFINITY` for roots) (4B).
    pub lod_error_parent: f32,
    /// Octahedral normal cone axis in `snorm16` (4B).
    pub cone_axis_oct16: [i16; 2],
    /// Quantized cone cutoff `cos(half_angle + pi/2)` in `snorm16` (2B).
    pub cone_cutoff_snorm16: i16,
    /// Hierarchy level (`0` = finest leaf meshlet) (1B).
    pub lod_level: u8,
    /// Vertex count in this meshlet (`<= 64`) (1B).
    pub vertex_count: u8,
    /// Triangle count in this meshlet (`<= 124`) (1B).
    pub triangle_count: u8,
    /// Material binding slot index (1B).
    pub material_slot: u8,
    /// First child cluster group index (`0xFFFF` for leaf clusters) (2B).
    pub child_group_first: u16,
    /// Offset into the meshlet vertex index stream (4B).
    pub vertex_data_offset: u32,
    /// Offset into the meshlet local triangle micro-index stream (4B).
    pub triangle_data_offset: u32,
    /// Hashed `did:q42:` anatomical/mechanical subpart identifier (4B).
    pub semantic_subpart_id: u32,
}

const _: () = assert!(core::mem::size_of::<MeshletClusterNode>() == 64);
const _: () = assert!(core::mem::align_of::<MeshletClusterNode>() == 64);

/// Errors emitted by the `CDAG` section codec and zero-heap LOD cut selector.
#[derive(Debug, Clone, PartialEq)]
pub enum ClusterDagSectionError {
    PayloadTooShort {
        need: usize,
        have: usize,
    },
    BadMagic([u8; 4]),
    UnsupportedVersion(u16),
    TooManyClusters {
        count: usize,
        max: usize,
    },
    PayloadLengthMismatch {
        declared_clusters: u32,
        payload_bytes: usize,
    },
    BufferTooSmall {
        need: usize,
        have: usize,
    },
    NonMonotonicError {
        cluster_index: usize,
        self_error: f32,
        parent_error: f32,
    },
    InvalidBoundingSphere {
        cluster_index: usize,
    },
    MeshletLimitExceeded {
        cluster_index: usize,
        vertices: u8,
        triangles: u8,
    },
    InvalidCameraParameters,
}

impl core::fmt::Display for ClusterDagSectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PayloadTooShort { need, have } => {
                write!(f, "CDAG payload too short: have {have}, need {need}")
            }
            Self::BadMagic(m) => write!(f, "CDAG bad magic: {m:?}"),
            Self::UnsupportedVersion(v) => write!(f, "CDAG unsupported version: {v}"),
            Self::TooManyClusters { count, max } => {
                write!(f, "CDAG cluster count {count} exceeds max {max}")
            }
            Self::PayloadLengthMismatch {
                declared_clusters,
                payload_bytes,
            } => write!(
                f,
                "CDAG payload length {payload_bytes} does not match {declared_clusters} clusters"
            ),
            Self::BufferTooSmall { need, have } => {
                write!(f, "CDAG buffer too small: have {have}, need {need}")
            }
            Self::NonMonotonicError {
                cluster_index,
                self_error,
                parent_error,
            } => write!(
                f,
                "CDAG cluster[{cluster_index}] violates monotonic error: self={self_error} > parent={parent_error}"
            ),
            Self::InvalidBoundingSphere { cluster_index } => {
                write!(f, "CDAG cluster[{cluster_index}] has invalid bounding sphere")
            }
            Self::MeshletLimitExceeded {
                cluster_index,
                vertices,
                triangles,
            } => write!(
                f,
                "CDAG cluster[{cluster_index}] exceeds meshlet limits: {vertices} verts, {triangles} tris"
            ),
            Self::InvalidCameraParameters => write!(f, "CDAG invalid viewport or FOV parameters"),
        }
    }
}

impl std::error::Error for ClusterDagSectionError {}

/// Compute exact byte length for a `CDAG` section with `cluster_count` nodes.
#[inline]
pub const fn cluster_dag_encoded_len(cluster_count: usize) -> Option<usize> {
    if cluster_count > MAX_CLUSTER_DAG_NODES {
        return None;
    }
    match cluster_count.checked_mul(MESHLET_CLUSTER_NODE_SIZE) {
        Some(body) => CLUSTER_DAG_HEADER_SIZE.checked_add(body),
        None => None,
    }
}

/// Encode a unit 3D direction into `[i16; 2]` octahedral `snorm16` coordinates.
#[inline]
pub fn encode_oct16_normal(dir: [f32; 3]) -> [i16; 2] {
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    let (mut x, mut y, z) = if len > 1e-6 && len.is_finite() {
        (dir[0] / len, dir[1] / len, dir[2] / len)
    } else {
        (0.0, 0.0, 1.0)
    };
    let l1 = (x.abs() + y.abs() + z.abs()).max(1e-6);
    x /= l1;
    y /= l1;
    if z < 0.0 {
        let ox = (1.0 - y.abs()) * if x >= 0.0 { 1.0 } else { -1.0 };
        let oy = (1.0 - x.abs()) * if y >= 0.0 { 1.0 } else { -1.0 };
        x = ox;
        y = oy;
    }
    [
        (x.clamp(-1.0, 1.0) * 32767.0).round() as i16,
        (y.clamp(-1.0, 1.0) * 32767.0).round() as i16,
    ]
}

/// Decode `[i16; 2]` octahedral `snorm16` coordinates back into a unit 3D vector.
#[inline]
pub fn decode_oct16_normal(oct: [i16; 2]) -> [f32; 3] {
    let mut x = (oct[0] as f32 / 32767.0).clamp(-1.0, 1.0);
    let mut y = (oct[1] as f32 / 32767.0).clamp(-1.0, 1.0);
    let z = 1.0 - x.abs() - y.abs();
    if z < 0.0 {
        let old_x = x;
        x = (1.0 - y.abs()) * if old_x >= 0.0 { 1.0 } else { -1.0 };
        y = (1.0 - old_x.abs()) * if y >= 0.0 { 1.0 } else { -1.0 };
    }
    let len = (x * x + y * y + z * z).sqrt();
    if len > 1e-6 {
        [x / len, y / len, z / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn validate_cluster_node(
    index: usize,
    node: &MeshletClusterNode,
) -> Result<(), ClusterDagSectionError> {
    let r_self = node.bounding_sphere[3];
    let r_parent = node.parent_error_sphere[3];
    if !node.bounding_sphere[0].is_finite()
        || !node.bounding_sphere[1].is_finite()
        || !node.bounding_sphere[2].is_finite()
        || !r_self.is_finite()
        || r_self <= 0.0
        || !node.parent_error_sphere[0].is_finite()
        || !node.parent_error_sphere[1].is_finite()
        || !node.parent_error_sphere[2].is_finite()
        || !r_parent.is_finite()
        || r_parent < r_self
    {
        return Err(ClusterDagSectionError::InvalidBoundingSphere {
            cluster_index: index,
        });
    }
    if !node.lod_error_self.is_finite()
        || node.lod_error_self < 0.0
        || node.lod_error_parent.is_nan()
        || node.lod_error_parent < node.lod_error_self
    {
        return Err(ClusterDagSectionError::NonMonotonicError {
            cluster_index: index,
            self_error: node.lod_error_self,
            parent_error: node.lod_error_parent,
        });
    }
    if node.vertex_count == 0
        || node.vertex_count > MAX_MESHLET_VERTICES
        || node.triangle_count == 0
        || node.triangle_count > MAX_MESHLET_TRIANGLES
    {
        return Err(ClusterDagSectionError::MeshletLimitExceeded {
            cluster_index: index,
            vertices: node.vertex_count,
            triangles: node.triangle_count,
        });
    }
    Ok(())
}

/// Encode a `CDAG` section into a caller-provided byte slice without heap allocation.
pub fn encode_cluster_dag_section(
    header: &ClusterDagMiniHeader,
    nodes: &[MeshletClusterNode],
    out: &mut [u8],
) -> Result<usize, ClusterDagSectionError> {
    if header.magic != MAGIC_CDAG {
        return Err(ClusterDagSectionError::BadMagic(header.magic));
    }
    if header.version != CLUSTER_DAG_SECTION_VERSION {
        return Err(ClusterDagSectionError::UnsupportedVersion(header.version));
    }
    let needed = cluster_dag_encoded_len(nodes.len()).ok_or(
        ClusterDagSectionError::TooManyClusters {
            count: nodes.len(),
            max: MAX_CLUSTER_DAG_NODES,
        },
    )?;
    if out.len() < needed {
        return Err(ClusterDagSectionError::BufferTooSmall {
            need: needed,
            have: out.len(),
        });
    }
    for (idx, node) in nodes.iter().enumerate() {
        validate_cluster_node(idx, node)?;
    }

    let mut canonical_header = *header;
    canonical_header.cluster_count = nodes.len() as u32;
    out[..CLUSTER_DAG_HEADER_SIZE].copy_from_slice(bytes_of(&canonical_header));

    let mut offset = CLUSTER_DAG_HEADER_SIZE;
    for node in nodes {
        out[offset..offset + MESHLET_CLUSTER_NODE_SIZE].copy_from_slice(bytes_of(node));
        offset += MESHLET_CLUSTER_NODE_SIZE;
    }
    Ok(needed)
}

/// Decode and validate a `CDAG` section into caller-provided `MeshletClusterNode` storage (Zero-Heap).
pub fn decode_cluster_dag_section(
    payload: &[u8],
    out_nodes: &mut [MeshletClusterNode],
) -> Result<(ClusterDagMiniHeader, usize), ClusterDagSectionError> {
    if payload.len() < CLUSTER_DAG_HEADER_SIZE {
        return Err(ClusterDagSectionError::PayloadTooShort {
            need: CLUSTER_DAG_HEADER_SIZE,
            have: payload.len(),
        });
    }
    let header: ClusterDagMiniHeader = pod_read_unaligned(&payload[..CLUSTER_DAG_HEADER_SIZE]);
    if header.magic != MAGIC_CDAG {
        return Err(ClusterDagSectionError::BadMagic(header.magic));
    }
    if header.version != CLUSTER_DAG_SECTION_VERSION {
        return Err(ClusterDagSectionError::UnsupportedVersion(header.version));
    }
    let count = header.cluster_count as usize;
    let needed_bytes = cluster_dag_encoded_len(count).ok_or(
        ClusterDagSectionError::TooManyClusters {
            count,
            max: MAX_CLUSTER_DAG_NODES,
        },
    )?;
    if payload.len() != needed_bytes {
        return Err(ClusterDagSectionError::PayloadLengthMismatch {
            declared_clusters: header.cluster_count,
            payload_bytes: payload.len(),
        });
    }
    if out_nodes.len() < count {
        return Err(ClusterDagSectionError::BufferTooSmall {
            need: count,
            have: out_nodes.len(),
        });
    }

    let mut offset = CLUSTER_DAG_HEADER_SIZE;
    for (idx, slot) in out_nodes[..count].iter_mut().enumerate() {
        let node: MeshletClusterNode =
            pod_read_unaligned(&payload[offset..offset + MESHLET_CLUSTER_NODE_SIZE]);
        validate_cluster_node(idx, &node)?;
        *slot = node;
        offset += MESHLET_CLUSTER_NODE_SIZE;
    }
    Ok((header, count))
}

/// Project a world-space geometric error `world_error` at `sphere` into screen-space pixels.
#[inline]
pub fn project_sphere_error_px(
    sphere: [f32; 4],
    world_error: f32,
    camera_pos: [f32; 3],
    proj_scale: f32,
) -> f32 {
    if world_error.is_infinite() {
        return f32::INFINITY;
    }
    let dx = sphere[0] - camera_pos[0];
    let dy = sphere[1] - camera_pos[1];
    let dz = sphere[2] - camera_pos[2];
    let center_dist = (dx * dx + dy * dy + dz * dz).sqrt();
    let surface_dist = (center_dist - sphere[3]).max(1e-4);
    (world_error * proj_scale) / surface_dist
}

/// Select the active continuous-LOD cut into `out_cluster_indices` without heap allocation.
///
/// A cluster is selected iff `projected_error(self) <= error_threshold_px < projected_error(parent)`.
pub fn select_cut_clusters_zero_heap(
    nodes: &[MeshletClusterNode],
    camera_pos: [f32; 3],
    viewport_height_px: f32,
    fov_y_rad: f32,
    error_threshold_px: f32,
    out_cluster_indices: &mut [u32],
) -> Result<usize, ClusterDagSectionError> {
    if !viewport_height_px.is_finite()
        || viewport_height_px <= 0.0
        || !fov_y_rad.is_finite()
        || fov_y_rad <= 0.0
        || fov_y_rad >= core::f32::consts::PI
        || !error_threshold_px.is_finite()
        || error_threshold_px < 0.0
    {
        return Err(ClusterDagSectionError::InvalidCameraParameters);
    }
    let half_tan = (0.5 * fov_y_rad).tan();
    if !(half_tan > 1e-6) || !half_tan.is_finite() {
        return Err(ClusterDagSectionError::InvalidCameraParameters);
    }
    let proj_scale = viewport_height_px / (2.0 * half_tan);

    let mut selected = 0usize;
    for (idx, node) in nodes.iter().enumerate() {
        let err_self = project_sphere_error_px(
            node.bounding_sphere,
            node.lod_error_self,
            camera_pos,
            proj_scale,
        );
        let err_parent = project_sphere_error_px(
            node.parent_error_sphere,
            node.lod_error_parent,
            camera_pos,
            proj_scale,
        );
        if err_self <= error_threshold_px && err_parent > error_threshold_px {
            if selected >= out_cluster_indices.len() {
                return Err(ClusterDagSectionError::BufferTooSmall {
                    need: selected + 1,
                    have: out_cluster_indices.len(),
                });
            }
            out_cluster_indices[selected] = idx as u32;
            selected += 1;
        }
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dag_nodes() -> [MeshletClusterNode; 3] {
        let axis = encode_oct16_normal([0.0, 0.0, 1.0]);
        [
            // Leaf 0 (LOD 0): self error 0.0m, parent error 0.02m
            MeshletClusterNode {
                bounding_sphere: [-0.5, 0.0, 0.0, 1.0],
                parent_error_sphere: [0.0, 0.0, 0.0, 2.0],
                lod_error_self: 0.0,
                lod_error_parent: 0.02,
                cone_axis_oct16: axis,
                cone_cutoff_snorm16: 0,
                lod_level: 0,
                vertex_count: 64,
                triangle_count: 120,
                material_slot: 0,
                child_group_first: 0xFFFF,
                vertex_data_offset: 0,
                triangle_data_offset: 0,
                semantic_subpart_id: 0x1001,
            },
            // Leaf 1 (LOD 0): self error 0.0m, parent error 0.02m
            MeshletClusterNode {
                bounding_sphere: [0.5, 0.0, 0.0, 1.0],
                parent_error_sphere: [0.0, 0.0, 0.0, 2.0],
                lod_error_self: 0.0,
                lod_error_parent: 0.02,
                cone_axis_oct16: axis,
                cone_cutoff_snorm16: 0,
                lod_level: 0,
                vertex_count: 64,
                triangle_count: 120,
                material_slot: 0,
                child_group_first: 0xFFFF,
                vertex_data_offset: 64,
                triangle_data_offset: 360,
                semantic_subpart_id: 0x1002,
            },
            // Root (LOD 1): simplified parent group, self error 0.02m, parent error +inf
            MeshletClusterNode {
                bounding_sphere: [0.0, 0.0, 0.0, 2.0],
                parent_error_sphere: [0.0, 0.0, 0.0, 2.0],
                lod_error_self: 0.02,
                lod_error_parent: f32::INFINITY,
                cone_axis_oct16: axis,
                cone_cutoff_snorm16: 0,
                lod_level: 1,
                vertex_count: 64,
                triangle_count: 120,
                material_slot: 0,
                child_group_first: 0,
                vertex_data_offset: 128,
                triangle_data_offset: 720,
                semantic_subpart_id: 0x1000,
            },
        ]
    }

    #[test]
    fn cdag_layout_and_roundtrip() {
        assert_eq!(core::mem::size_of::<ClusterDagMiniHeader>(), 32);
        assert_eq!(core::mem::size_of::<MeshletClusterNode>(), 64);

        let nodes = sample_dag_nodes();
        let header = ClusterDagMiniHeader {
            max_lod_depth: 1,
            cluster_count: 3,
            group_count: 1,
            total_meshlet_vertices: 192,
            total_meshlet_triangles: 360,
            root_cluster_offset: 2,
            ..ClusterDagMiniHeader::default()
        };
        let mut buf = [0u8; 32 + 3 * 64];
        let written = encode_cluster_dag_section(&header, &nodes, &mut buf).unwrap();
        assert_eq!(written, buf.len());

        let mut decoded = [MeshletClusterNode::zeroed(); 3];
        let (dec_hdr, count) = decode_cluster_dag_section(&buf, &mut decoded).unwrap();
        assert_eq!(count, 3);
        assert_eq!(dec_hdr, header);
        assert_eq!(decoded, nodes);
    }

    #[test]
    fn cdag_cut_selection_transitions_between_leaf_and_root() {
        let nodes = sample_dag_nodes();
        let mut cut = [0u32; 4];

        // Camera close (z = 4.0m): parent error (0.02m) projects > 1.0px -> selects leaves [0, 1]
        let near_count = select_cut_clusters_zero_heap(
            &nodes,
            [0.0, 0.0, 4.0],
            1080.0,
            core::f32::consts::FRAC_PI_3,
            1.0,
            &mut cut,
        )
        .unwrap();
        assert_eq!(&cut[..near_count], &[0, 1]);

        // Camera far (z = 50.0m): parent error (0.02m) projects < 1.0px -> selects root [2]
        let far_count = select_cut_clusters_zero_heap(
            &nodes,
            [0.0, 0.0, 50.0],
            1080.0,
            core::f32::consts::FRAC_PI_3,
            1.0,
            &mut cut,
        )
        .unwrap();
        assert_eq!(&cut[..far_count], &[2]);
    }
}
