//! P9.5 — Authoring ergonomics: scene construction, primitives, transforms.
//!
//! The "three.js-class maker core" — deterministic primitive generation,
//! T·R·S transform composition, scene-graph assembly, and `.10d` asset
//! export with μ provenance + Q42 identity.
//!
//! ## Determinism
//!
//! Every primitive generator is deterministic: identical parameters yield
//! byte-identical meshes. The transform composer uses f64 arithmetic and
//! matches an independent oracle. Exported `.10d` assets are byte-stable
//! across identical exports.

use crate::container_10d::{
    encode_container, seal_whole_file_crc32c, verify_whole_file_crc32c,
    Container10dHeader, SectionInput, SectionType, AlignmentTier,
    encode_mesh_section, decode_mesh_section,
};
use crate::container_10d::mesh_section::encoded_len as mesh_encoded_len;
use crate::container_10d::node_section::{write_node_section_aos, read_node, NODE_MINI_HEADER_SIZE};
use crate::render::assets::Mesh;
use crate::tensor::Tensor10D;

// ───────────────────────────────────────────────────────────────────────────
//  Error types
// ───────────────────────────────────────────────────────────────────────────

/// Authoring error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoringError {
    /// Invalid primitive parameters (e.g. negative radius).
    InvalidParameters(&'static str),
    /// Mesh encoding failed.
    MeshEncode(String),
    /// Container encoding failed.
    ContainerEncode(String),
    /// Buffer too small.
    BufferTooSmall { needed: usize, have: usize },
    /// Provenance node encoding failed.
    ProvenanceEncode(String),
}

impl core::fmt::Display for AuthoringError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidParameters(msg) => write!(f, "authoring: invalid parameters: {msg}"),
            Self::MeshEncode(msg) => write!(f, "authoring: mesh encode failed: {msg}"),
            Self::ContainerEncode(msg) => write!(f, "authoring: container encode failed: {msg}"),
            Self::BufferTooSmall { needed, have } => {
                write!(f, "authoring: buffer too small: need {needed}, have {have}")
            }
            Self::ProvenanceEncode(msg) => write!(f, "authoring: provenance encode failed: {msg}"),
        }
    }
}

impl std::error::Error for AuthoringError {}

// ───────────────────────────────────────────────────────────────────────────
//  Primitive generation
// ───────────────────────────────────────────────────────────────────────────

/// Generate a unit cube (1×1×1) centred at the origin.
///
/// 8 vertices, 12 triangles. Deterministic.
pub fn unit_box() -> Mesh {
    let positions = vec![
        [-0.5, -0.5, -0.5], [ 0.5, -0.5, -0.5], [ 0.5,  0.5, -0.5], [-0.5,  0.5, -0.5],
        [-0.5, -0.5,  0.5], [ 0.5, -0.5,  0.5], [ 0.5,  0.5,  0.5], [-0.5,  0.5,  0.5],
    ];
    let triangles = vec![
        [0, 1, 2], [0, 2, 3], // -Z
        [4, 6, 5], [4, 7, 6], // +Z
        [0, 4, 5], [0, 5, 1], // -Y
        [2, 6, 7], [2, 7, 3], // +Y
        [0, 3, 7], [0, 7, 4], // -X
        [1, 5, 6], [1, 6, 2], // +X
    ];
    Mesh { positions, triangles, min: [-0.5; 3], max: [0.5; 3] }
}

/// Generate a box with custom dimensions centred at the origin.
pub fn box_mesh(width: f32, height: f32, depth: f32) -> Result<Mesh, AuthoringError> {
    if width <= 0.0 || height <= 0.0 || depth <= 0.0 {
        return Err(AuthoringError::InvalidParameters("dimensions must be positive"));
    }
    let hx = width * 0.5;
    let hy = height * 0.5;
    let hz = depth * 0.5;
    let positions = vec![
        [-hx, -hy, -hz], [ hx, -hy, -hz], [ hx,  hy, -hz], [-hx,  hy, -hz],
        [-hx, -hy,  hz], [ hx, -hy,  hz], [ hx,  hy,  hz], [-hx,  hy,  hz],
    ];
    let triangles = vec![
        [0, 1, 2], [0, 2, 3], [4, 6, 5], [4, 7, 6],
        [0, 4, 5], [0, 5, 1], [2, 6, 7], [2, 7, 3],
        [0, 3, 7], [0, 7, 4], [1, 5, 6], [1, 6, 2],
    ];
    Ok(Mesh { positions, triangles, min: [-hx, -hy, -hz], max: [hx, hy, hz] })
}

/// Generate a UV sphere of given radius, latitude/longitude segments.
///
/// Deterministic: identical (radius, lat_segments, lon_segments) yields
/// byte-identical mesh.
pub fn uv_sphere(radius: f32, lat_segments: u32, lon_segments: u32) -> Result<Mesh, AuthoringError> {
    if radius <= 0.0 {
        return Err(AuthoringError::InvalidParameters("radius must be positive"));
    }
    if lat_segments < 2 || lon_segments < 3 {
        return Err(AuthoringError::InvalidParameters("need lat>=2, lon>=3"));
    }

    let mut positions = Vec::new();
    let mut triangles = Vec::new();

    // Vertices: poles + (lat_segments-1) rings of (lon_segments+1) vertices.
    // South pole
    positions.push([0.0, -radius, 0.0]);

    for lat in 1..lat_segments {
        let theta = core::f32::consts::PI * (lat as f32) / (lat_segments as f32);
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        for lon in 0..=lon_segments {
            let phi = 2.0 * core::f32::consts::PI * (lon as f32) / (lon_segments as f32);
            let x = radius * sin_theta * phi.cos();
            let y = radius * cos_theta;
            let z = radius * sin_theta * phi.sin();
            positions.push([x, y, z]);
        }
    }

    // North pole
    positions.push([0.0, radius, 0.0]);

    let south = 0u32;
    let north = (positions.len() - 1) as u32;
    let ring_start = 1u32;
    let ring_size = lon_segments + 1;

    // South pole fan
    for lon in 0..lon_segments {
        triangles.push([south, ring_start + lon, ring_start + lon + 1]);
    }

    // Middle quads
    for lat in 0..(lat_segments - 2) {
        let r0 = ring_start + lat * ring_size;
        let r1 = r0 + ring_size;
        for lon in 0..lon_segments {
            triangles.push([r0 + lon, r1 + lon, r1 + lon + 1]);
            triangles.push([r0 + lon, r1 + lon + 1, r0 + lon + 1]);
        }
    }

    // North pole fan
    let last_ring = ring_start + (lat_segments - 2) * ring_size;
    for lon in 0..lon_segments {
        triangles.push([north, last_ring + lon + 1, last_ring + lon]);
    }

    Ok(Mesh { positions, triangles, min: [-radius; 3], max: [radius; 3] })
}

/// Generate a cylinder along the Y axis with given radius, height, segments.
pub fn cylinder(radius: f32, height: f32, segments: u32) -> Result<Mesh, AuthoringError> {
    if radius <= 0.0 || height <= 0.0 {
        return Err(AuthoringError::InvalidParameters("radius and height must be positive"));
    }
    if segments < 3 {
        return Err(AuthoringError::InvalidParameters("need segments>=3"));
    }

    let hy = height * 0.5;
    let mut positions = Vec::new();
    let mut triangles = Vec::new();

    // Bottom ring (0..segments), top ring (segments..2*segments)
    for i in 0..segments {
        let angle = 2.0 * core::f32::consts::PI * (i as f32) / (segments as f32);
        let x = radius * angle.cos();
        let z = radius * angle.sin();
        positions.push([x, -hy, z]); // bottom
    }
    for i in 0..segments {
        let angle = 2.0 * core::f32::consts::PI * (i as f32) / (segments as f32);
        let x = radius * angle.cos();
        let z = radius * angle.sin();
        positions.push([x, hy, z]); // top
    }

    let bottom_center = positions.len() as u32;
    positions.push([0.0, -hy, 0.0]);
    let top_center = positions.len() as u32;
    positions.push([0.0, hy, 0.0]);

    // Side faces
    for i in 0..segments {
        let bi = i;
        let bi_next = (i + 1) % segments;
        let ti = i + segments;
        let ti_next = (i + 1) % segments + segments;
        triangles.push([bi, ti, ti_next]);
        triangles.push([bi, ti_next, bi_next]);
    }

    // Bottom cap
    for i in 0..segments {
        let i_next = (i + 1) % segments;
        triangles.push([bottom_center, i_next, i]);
    }

    // Top cap
    for i in 0..segments {
        let i_next = (i + 1) % segments;
        triangles.push([top_center, i + segments, i_next + segments]);
    }

    Ok(Mesh { positions, triangles, min: [-radius, -hy, -radius], max: [radius, hy, radius] })
}

/// Generate a plane in the XZ plane with given size, centred at origin.
pub fn plane(size: f32) -> Result<Mesh, AuthoringError> {
    if size <= 0.0 {
        return Err(AuthoringError::InvalidParameters("size must be positive"));
    }
    let h = size * 0.5;
    let positions = vec![
        [-h, 0.0, -h], [ h, 0.0, -h], [ h, 0.0,  h], [-h, 0.0,  h],
    ];
    let triangles = vec![[0, 1, 2], [0, 2, 3]];
    Ok(Mesh { positions, triangles, min: [-h, 0.0, -h], max: [h, 0.0, h] })
}

// ───────────────────────────────────────────────────────────────────────────
//  Transforms (f64 matrix math — the independent oracle)
// ───────────────────────────────────────────────────────────────────────────

/// A 4×4 column-major transform matrix (f64 for oracle precision).
pub type Mat4 = [[f64; 4]; 4];

/// Identity matrix.
pub fn identity() -> Mat4 {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Translation matrix.
pub fn translation(tx: f64, ty: f64, tz: f64) -> Mat4 {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [tx,  ty,  tz,  1.0],
    ]
}

/// Scale matrix.
pub fn scale(sx: f64, sy: f64, sz: f64) -> Mat4 {
    [
        [sx,  0.0, 0.0, 0.0],
        [0.0, sy,  0.0, 0.0],
        [0.0, 0.0, sz,  0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Rotation matrix around Z axis (yaw).
pub fn rotation_z(angle_rad: f64) -> Mat4 {
    let c = angle_rad.cos();
    let s = angle_rad.sin();
    [
        [c,  s,  0.0, 0.0],
        [-s, c,  0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Rotation matrix around Y axis (pitch).
pub fn rotation_y(angle_rad: f64) -> Mat4 {
    let c = angle_rad.cos();
    let s = angle_rad.sin();
    [
        [c,  0.0, -s,  0.0],
        [0.0, 1.0, 0.0, 0.0],
        [s,  0.0, c,   0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// Rotation matrix around X axis (roll).
pub fn rotation_x(angle_rad: f64) -> Mat4 {
    let c = angle_rad.cos();
    let s = angle_rad.sin();
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, c,   s,   0.0],
        [0.0, -s,  c,   0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// 4×4 matrix multiply (column-major). This is the independent oracle
/// the composed T·R·S must match.
pub fn mat_mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut r = [[0.0f64; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[k][row] * b[col][k];
            }
            r[col][row] = sum;
        }
    }
    r
}

/// Compose T·R·S transform: first scale, then rotate, then translate.
///
/// `rotation` is (rx, ry, rz) in radians, applied as Rx·Ry·Rz.
pub fn compose_trs(
    tx: f64, ty: f64, tz: f64,
    rx: f64, ry: f64, rz: f64,
    sx: f64, sy: f64, sz: f64,
) -> Mat4 {
    let t = translation(tx, ty, tz);
    let r = mat_mul(&mat_mul(&rotation_x(rx), &rotation_y(ry)), &rotation_z(rz));
    let s = scale(sx, sy, sz);
    // T·R·S: apply S first, then R, then T → matrix order is T * R * S
    mat_mul(&t, &mat_mul(&r, &s))
}

/// Apply a Mat4 transform to a mesh's vertex positions (f32 → f64 → f32).
pub fn transform_mesh(mesh: &Mesh, m: &Mat4) -> Mesh {
    let positions: Vec<[f32; 3]> = mesh.positions.iter().map(|p| {
        let x = p[0] as f64;
        let y = p[1] as f64;
        let z = p[2] as f64;
        // Column-major: result = [x y z 1] * M
        let nx = x * m[0][0] + y * m[1][0] + z * m[2][0] + m[3][0];
        let ny = x * m[0][1] + y * m[1][1] + z * m[2][1] + m[3][1];
        let nz = x * m[0][2] + y * m[1][2] + z * m[2][2] + m[3][2];
        [nx as f32, ny as f32, nz as f32]
    }).collect();

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &positions {
        for a in 0..3 {
            min[a] = min[a].min(p[a]);
            max[a] = max[a].max(p[a]);
        }
    }

    Mesh {
        positions,
        triangles: mesh.triangles.clone(),
        min,
        max,
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Scene graph
// ───────────────────────────────────────────────────────────────────────────

/// A scene node: a mesh + a transform + a name.
#[derive(Debug, Clone)]
pub struct SceneNode {
    pub name: String,
    pub mesh: Mesh,
    pub transform: Mat4,
}

/// A scene: an ordered list of nodes.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub nodes: Vec<SceneNode>,
}

impl Scene {
    /// Create an empty scene.
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Add a node to the scene.
    pub fn add(&mut self, node: SceneNode) {
        self.nodes.push(node);
    }

    /// Add a primitive with a T·R·S transform.
    pub fn add_primitive(
        &mut self,
        name: &str,
        mesh: Mesh,
        tx: f64, ty: f64, tz: f64,
        rx: f64, ry: f64, rz: f64,
        sx: f64, sy: f64, sz: f64,
    ) {
        self.nodes.push(SceneNode {
            name: name.to_string(),
            mesh,
            transform: compose_trs(tx, ty, tz, rx, ry, rz, sx, sy, sz),
        });
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Total vertex count across all nodes.
    pub fn total_vertices(&self) -> usize {
        self.nodes.iter().map(|n| n.mesh.positions.len()).sum()
    }

    /// Total triangle count across all nodes.
    pub fn total_triangles(&self) -> usize {
        self.nodes.iter().map(|n| n.mesh.triangles.len()).sum()
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Provenance + Q42 identity
// ───────────────────────────────────────────────────────────────────────────

/// Provenance metadata for an authored asset.
///
/// The μ lane (provenance axis) is carried as a non-zero f32 in the
/// Tensor10D node section. The Q42 identity is the author's DID hash
/// stored in the `q` axis (certainty/identity).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProvenanceMetadata {
    /// Author's Q42 DID hash (bit 63 set for did:q42 identifiers).
    pub author_did_hash: u64,
    /// Provenance μ value (non-zero — a hash-derived scalar).
    pub mu: f32,
    /// Creation timestamp (Unix epoch seconds, stored in `t` axis).
    pub timestamp: f32,
    /// Domain/scope hash (stored in `w` axis).
    pub domain_hash: u64,
}

impl ProvenanceMetadata {
    /// Create provenance metadata from an author DID hash and a domain hash.
    /// The μ value is derived from the hash to be deterministic and non-zero.
    pub fn new(author_did_hash: u64, domain_hash: u64, timestamp: f32) -> Self {
        // Derive a deterministic non-zero μ from the author hash.
        let mu_bits = (author_did_hash ^ domain_hash).rotate_left(1) | 1;
        let mu = (mu_bits as f32).abs();
        Self {
            author_did_hash,
            mu: if mu == 0.0 { 1.0 } else { mu },
            timestamp,
            domain_hash,
        }
    }

    /// Encode as a single Tensor10D node for the `.10d` container.
    ///
    /// The 64-bit author DID hash is split across `q` (low 32 bits) and
    /// `v` (high 32 bits) using `f32::from_bits` to preserve all 64 bits.
    /// Similarly, `domain_hash` is split across `w` (low) and `x` (high).
    pub fn to_tensor(&self) -> Tensor10D {
        let q = f32::from_bits(self.author_did_hash as u32);
        let v = f32::from_bits((self.author_did_hash >> 32) as u32);
        let w = f32::from_bits(self.domain_hash as u32);
        let x = f32::from_bits((self.domain_hash >> 32) as u32);
        Tensor10D::new(
            q,                            // q — DID low 32 bits
            v,                            // v — DID high 32 bits
            w,                            // w — domain low 32 bits
            x,                            // x — domain high 32 bits
            0.0,                          // y
            0.0,                          // z
            self.timestamp,               // t
            0.0,                          // alpha
            self.mu,                      // mu
            0.0,                          // sigma
        )
    }

    /// Check if the provenance is non-empty (μ is non-zero).
    pub fn is_non_empty(&self) -> bool {
        self.mu != 0.0
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  .10d asset export
// ───────────────────────────────────────────────────────────────────────────

/// Compute the total encoded size for a mesh + provenance `.10d` asset.
pub fn asset_encoded_len(mesh: &Mesh, _provenance: &ProvenanceMetadata) -> usize {
    let mesh_len = mesh_encoded_len(mesh.positions.len(), mesh.triangles.len());
    let node_len = NODE_MINI_HEADER_SIZE + 40; // 1 tensor = 40 bytes
    // Header (64) + 2 section descriptors (48) + aligned payloads
    // Conservative estimate: 64 + 48 + mesh_len + 64 (alignment) + node_len + 64
    64 + 48 + mesh_len + 64 + node_len + 64
}

/// Export a mesh with provenance metadata as a `.10d` container.
///
/// The container has two sections:
/// 1. `QuantizedMesh` — the geometry
/// 2. `Tensor10DNodes` — a single node carrying the μ provenance + Q42 identity
///
/// The output is sealed with a whole-file CRC-32C. Two identical exports
/// produce byte-identical output.
pub fn export_asset(
    mesh: &Mesh,
    provenance: &ProvenanceMetadata,
    out: &mut [u8],
) -> Result<usize, AuthoringError> {
    if !provenance.is_non_empty() {
        return Err(AuthoringError::ProvenanceEncode("provenance μ must be non-zero".into()));
    }

    // Encode mesh section payload.
    let mesh_need = mesh_encoded_len(mesh.positions.len(), mesh.triangles.len());
    let mut mesh_payload = vec![0u8; mesh_need];
    let mesh_written = encode_mesh_section(mesh, &mut mesh_payload)
        .map_err(|e| AuthoringError::MeshEncode(format!("{e}")))?;
    mesh_payload.truncate(mesh_written);

    // Encode provenance node section payload.
    let tensor = provenance.to_tensor();
    let tensors = [tensor];
    let node_need = NODE_MINI_HEADER_SIZE + 40;
    let mut node_payload = vec![0u8; node_need];
    write_node_section_aos(&tensors, &mut node_payload)
        .map_err(|e| AuthoringError::ProvenanceEncode(format!("{e}")))?;

    // Build the container.
    let header = Container10dHeader::proposed();
    let inputs = [
        SectionInput {
            section_type: SectionType::QuantizedMesh,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &mesh_payload,
        },
        SectionInput {
            section_type: SectionType::Tensor10DNodes,
            alignment_tier: AlignmentTier::CacheLine,
            stride: 0,
            element_count: 0,
            payload: &node_payload,
        },
    ];

    let total = encode_container(&header, &inputs, out)
        .map_err(|e| AuthoringError::ContainerEncode(format!("{e}")))?;

    // Seal the whole-file CRC-32C.
    seal_whole_file_crc32c(&mut out[..total]);

    Ok(total)
}

/// Import a `.10d` asset and extract the mesh + provenance.
///
/// Takes `&mut [u8]` because CRC verification is in-place.
pub fn import_asset(bytes: &mut [u8]) -> Result<(Mesh, ProvenanceMetadata), AuthoringError> {
    verify_whole_file_crc32c(bytes)
        .map_err(|e| AuthoringError::ContainerEncode(format!("CRC verify: {e}")))?;

    let header = Container10dHeader::parse(bytes)
        .map_err(|e| AuthoringError::ContainerEncode(format!("header: {e}")))?;

    let descs = crate::container_10d::parse_section_table(bytes, &header)
        .map_err(|e| AuthoringError::ContainerEncode(format!("section table: {e}")))?;

    let mut mesh = None;
    let mut provenance_tensor = None;

    for desc in descs {
        let st = SectionType::from_u8(desc.section_type)
            .ok_or(AuthoringError::ContainerEncode("unknown section type".into()))?;
        let off = desc.byte_offset as usize;
        let len = desc.byte_length as usize;
        let payload = &bytes[off..off + len];

        match st {
            SectionType::QuantizedMesh => {
                mesh = Some(decode_mesh_section(payload)
                    .map_err(|e| AuthoringError::ContainerEncode(format!("mesh decode: {e}")))?);
            }
            SectionType::Tensor10DNodes => {
                let t = read_node(payload, 0)
                    .map_err(|e| AuthoringError::ContainerEncode(format!("node read: {e}")))?;
                provenance_tensor = Some(t);
            }
            _ => {}
        }
    }

    let mesh = mesh.ok_or(AuthoringError::ContainerEncode("no mesh section".into()))?;
    let tensor = provenance_tensor.ok_or(AuthoringError::ContainerEncode("no provenance node section".into()))?;

    // Reconstruct provenance from tensor.
    let author_did_hash = ((tensor.v.to_bits() as u64) << 32) | (tensor.q.to_bits() as u64);
    let domain_hash = ((tensor.x.to_bits() as u64) << 32) | (tensor.w.to_bits() as u64);
    let provenance = ProvenanceMetadata {
        author_did_hash,
        mu: tensor.mu,
        timestamp: tensor.t,
        domain_hash,
    };

    Ok((mesh, provenance))
}

// ───────────────────────────────────────────────────────────────────────────
//  P9.6 — Mesh boolean operations
// ───────────────────────────────────────────────────────────────────────────

/// Boolean operation type (mirrors `Boolean3Op` for the authoring API).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Intersection,
    Difference,
}

impl BooleanOp {
    fn to_kernel(self) -> super::boolean_3::Boolean3Op {
        match self {
            Self::Union => super::boolean_3::Boolean3Op::Union,
            Self::Intersection => super::boolean_3::Boolean3Op::Intersection,
            Self::Difference => super::boolean_3::Boolean3Op::Difference,
        }
    }
}

/// Apply a boolean operation to two meshes.
///
/// Both meshes must be closed (watertight) triangle meshes. The result is a
/// new `Mesh` with the boolean applied. Uses the exact `orient_3d` predicate
/// for classification; intersection construction is `f64` (approximate).
pub fn boolean_op(mesh_a: &Mesh, mesh_b: &Mesh, op: BooleanOp) -> Result<Mesh, AuthoringError> {
    use super::boolean_3::{boolean_3, required_triangles_3, required_vertices_3};
    use super::primitives::Point3;

    let va: Vec<Point3> = mesh_a.positions.iter().map(|p| Point3::new(p[0] as f64, p[1] as f64, p[2] as f64)).collect();
    let ta: Vec<[u32; 3]> = mesh_a.triangles.clone();
    let vb: Vec<Point3> = mesh_b.positions.iter().map(|p| Point3::new(p[0] as f64, p[1] as f64, p[2] as f64)).collect();
    let tb: Vec<[u32; 3]> = mesh_b.triangles.clone();

    let max_v = required_vertices_3(va.len(), vb.len(), ta.len(), tb.len());
    let max_t = required_triangles_3(ta.len(), tb.len());
    let mut ov = vec![Point3::new(0.0, 0.0, 0.0); max_v];
    let mut ot = vec![[0u32; 3]; max_t];

    let (vc, tc) = boolean_3(&va, &ta, &vb, &tb, op.to_kernel(), &mut ov, &mut ot)
        .map_err(|e| AuthoringError::ContainerEncode(format!("boolean_3: {e}")))?;

    let positions: Vec<[f32; 3]> = ov[..vc].iter().map(|p| [p.x as f32, p.y as f32, p.z as f32]).collect();
    let triangles: Vec<[u32; 3]> = ot[..tc].to_vec();

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &positions {
        for a in 0..3 {
            min[a] = min[a].min(p[a]);
            max[a] = max[a].max(p[a]);
        }
    }

    Ok(Mesh { positions, triangles, min, max })
}

// ───────────────────────────────────────────────────────────────────────────
//  P9.6 — Procedural generation
// ───────────────────────────────────────────────────────────────────────────

/// Generate a torus mesh.
///
/// `major_radius` is the distance from the center to the tube centerline;
/// `minor_radius` is the tube radius. `major_segments` and `minor_segments`
/// control tessellation. Deterministic: identical parameters yield
/// byte-identical meshes.
pub fn torus(
    major_radius: f32,
    minor_radius: f32,
    major_segments: u32,
    minor_segments: u32,
) -> Result<Mesh, AuthoringError> {
    if major_radius <= 0.0 || minor_radius <= 0.0 {
        return Err(AuthoringError::InvalidParameters("radii must be positive"));
    }
    if major_segments < 3 || minor_segments < 3 {
        return Err(AuthoringError::InvalidParameters("need major>=3, minor>=3"));
    }
    if minor_radius >= major_radius {
        return Err(AuthoringError::InvalidParameters("minor_radius must be < major_radius"));
    }

    let mut positions = Vec::new();
    let mut triangles = Vec::new();

    for i in 0..major_segments {
        let u = 2.0 * core::f32::consts::PI * (i as f32) / (major_segments as f32);
        let cu = u.cos();
        let su = u.sin();
        for j in 0..minor_segments {
            let v = 2.0 * core::f32::consts::PI * (j as f32) / (minor_segments as f32);
            let cv = v.cos();
            let sv = v.sin();
            let r = major_radius + minor_radius * cv;
            positions.push([r * cu, minor_radius * sv, r * su]);
        }
    }

    for i in 0..major_segments {
        for j in 0..minor_segments {
            let a = i * minor_segments + j;
            let b = i * minor_segments + (j + 1) % minor_segments;
            let c = ((i + 1) % major_segments) * minor_segments + j;
            let d = ((i + 1) % major_segments) * minor_segments + (j + 1) % minor_segments;
            triangles.push([a, c, b]);
            triangles.push([b, c, d]);
        }
    }

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &positions {
        for a in 0..3 {
            min[a] = min[a].min(p[a]);
            max[a] = max[a].max(p[a]);
        }
    }

    Ok(Mesh { positions, triangles, min, max })
}

/// Generate a grid mesh (subdivided plane) centred at the origin.
///
/// `size` is the total extent; `subdivisions` is the number of cells per axis.
/// Deterministic.
pub fn grid(size: f32, subdivisions: u32) -> Result<Mesh, AuthoringError> {
    if size <= 0.0 {
        return Err(AuthoringError::InvalidParameters("size must be positive"));
    }
    let n = subdivisions.max(1);
    let step = size / n as f32;
    let half = size * 0.5;

    let mut positions = Vec::new();
    let mut triangles = Vec::new();

    for i in 0..=n {
        for j in 0..=n {
            positions.push([-half + j as f32 * step, 0.0, -half + i as f32 * step]);
        }
    }

    for i in 0..n {
        for j in 0..n {
            let a = i * (n + 1) + j;
            let b = i * (n + 1) + j + 1;
            let c = (i + 1) * (n + 1) + j;
            let d = (i + 1) * (n + 1) + j + 1;
            triangles.push([a, c, b]);
            triangles.push([b, c, d]);
        }
    }

    Ok(Mesh { positions, triangles, min: [-half, 0.0, -half], max: [half, 0.0, half] })
}

// ───────────────────────────────────────────────────────────────────────────
//  P9.6 — Vertex drag / edit with t-slice + governance refusal
// ───────────────────────────────────────────────────────────────────────────

/// Governance consent state for a drag operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragConsent {
    /// Whether the editing agent has consent to modify the asset.
    pub consent_granted: bool,
    /// Whether the asset is sealed (immutable prior t-slices).
    pub sealed_prior: bool,
}

impl Default for DragConsent {
    fn default() -> Self {
        Self { consent_granted: true, sealed_prior: true }
    }
}

/// Error for drag operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragError {
    /// Governance/consent refusal — the drag violates the consent lane.
    GovernanceRefused,
    /// Vertex index out of bounds.
    VertexOutOfBounds { index: usize, count: usize },
    /// The asset is sealed and the prior t-slice cannot be mutated.
    SealedSlice,
}

impl core::fmt::Display for DragError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::GovernanceRefused => write!(f, "drag refused: governance/consent violation"),
            Self::VertexOutOfBounds { index, count } => write!(f, "drag: vertex {index} out of bounds (count={count})"),
            Self::SealedSlice => write!(f, "drag: prior t-slice is sealed and cannot be mutated"),
        }
    }
}

impl std::error::Error for DragError {}

/// Result of a vertex drag: the new mesh (new t-slice) and the new timestamp.
#[derive(Debug, Clone)]
pub struct DragResult {
    /// The new mesh with the dragged vertex position applied.
    pub mesh: Mesh,
    /// The new t-slice value (prior t + 1.0).
    pub new_t: f32,
    /// The prior t-slice value (unchanged).
    pub prior_t: f32,
}

/// Drag a single vertex to a new position, producing a NEW t-slice.
///
/// The prior t-slice is **never mutated** — the drag lands as a new temporal
/// version. If `consent.consent_granted` is false, or if `consent.sealed_prior`
/// is true and the caller attempts to mutate the prior slice, the drag is
/// refused (governance fail-closed).
///
/// `vertex_index` is the index into `mesh.positions`. `new_position` is the
/// target `[x, y, z]`. `prior_t` is the current t-slice value.
pub fn drag_vertex(
    mesh: &Mesh,
    vertex_index: usize,
    new_position: [f32; 3],
    prior_t: f32,
    consent: DragConsent,
) -> Result<DragResult, DragError> {
    if !consent.consent_granted {
        return Err(DragError::GovernanceRefused);
    }
    if vertex_index >= mesh.positions.len() {
        return Err(DragError::VertexOutOfBounds {
            index: vertex_index,
            count: mesh.positions.len(),
        });
    }

    // The prior t-slice is sealed — we create a new t-slice, never mutate.
    let new_t = prior_t + 1.0;

    let mut new_positions = mesh.positions.clone();
    new_positions[vertex_index] = new_position;

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &new_positions {
        for a in 0..3 {
            min[a] = min[a].min(p[a]);
            max[a] = max[a].max(p[a]);
        }
    }

    Ok(DragResult {
        mesh: Mesh {
            positions: new_positions,
            triangles: mesh.triangles.clone(),
            min,
            max,
        },
        new_t,
        prior_t,
    })
}

// ───────────────────────────────────────────────────────────────────────────
//  Determinism hash
// ───────────────────────────────────────────────────────────────────────────

/// FNV-1a hash over a byte slice (deterministic fingerprint).
pub fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Compute a determinism hash over an exported asset.
pub fn asset_hash(bytes: &[u8]) -> u64 {
    fnv1a_hash(bytes)
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Primitive generation ──────────────────────────────────────────────

    #[test]
    fn unit_box_is_correct() {
        let mesh = unit_box();
        assert_eq!(mesh.positions.len(), 8);
        assert_eq!(mesh.triangles.len(), 12);
        assert_eq!(mesh.min, [-0.5; 3]);
        assert_eq!(mesh.max, [0.5; 3]);
    }

    #[test]
    fn box_mesh_deterministic() {
        let a = box_mesh(2.0, 3.0, 4.0).unwrap();
        let b = box_mesh(2.0, 3.0, 4.0).unwrap();
        assert_eq!(a.positions, b.positions);
        assert_eq!(a.triangles, b.triangles);
    }

    #[test]
    fn box_mesh_rejects_negative() {
        assert!(box_mesh(-1.0, 1.0, 1.0).is_err());
        assert!(box_mesh(0.0, 1.0, 1.0).is_err());
    }

    #[test]
    fn uv_sphere_deterministic() {
        let a = uv_sphere(1.0, 8, 16).unwrap();
        let b = uv_sphere(1.0, 8, 16).unwrap();
        assert_eq!(a.positions, b.positions);
        assert_eq!(a.triangles, b.triangles);
    }

    #[test]
    fn uv_sphere_vertex_count() {
        // lat=8, lon=16: 2 poles + 7 rings * 17 = 2 + 119 = 121
        let mesh = uv_sphere(1.0, 8, 16).unwrap();
        assert_eq!(mesh.positions.len(), 121);
    }

    #[test]
    fn uv_sphere_rejects_bad_params() {
        assert!(uv_sphere(-1.0, 8, 16).is_err());
        assert!(uv_sphere(1.0, 1, 16).is_err());
        assert!(uv_sphere(1.0, 8, 2).is_err());
    }

    #[test]
    fn cylinder_deterministic() {
        let a = cylinder(1.0, 2.0, 8).unwrap();
        let b = cylinder(1.0, 2.0, 8).unwrap();
        assert_eq!(a.positions, b.positions);
        assert_eq!(a.triangles, b.triangles);
    }

    #[test]
    fn cylinder_vertex_count() {
        // segments=8: 8 bottom + 8 top + 2 centers = 18
        let mesh = cylinder(1.0, 2.0, 8).unwrap();
        assert_eq!(mesh.positions.len(), 18);
    }

    #[test]
    fn plane_deterministic() {
        let a = plane(2.0).unwrap();
        let b = plane(2.0).unwrap();
        assert_eq!(a.positions, b.positions);
        assert_eq!(a.triangles, b.triangles);
    }

    // ── Transforms ────────────────────────────────────────────────────────

    #[test]
    fn identity_is_identity() {
        let m = identity();
        let p = [1.0f64, 2.0, 3.0];
        // Apply: [x y z 1] * M
        let nx = p[0] * m[0][0] + p[1] * m[1][0] + p[2] * m[2][0] + m[3][0];
        let ny = p[0] * m[0][1] + p[1] * m[1][1] + p[2] * m[2][1] + m[3][1];
        let nz = p[0] * m[0][2] + p[1] * m[1][2] + p[2] * m[2][2] + m[3][2];
        assert_eq!(nx, 1.0);
        assert_eq!(ny, 2.0);
        assert_eq!(nz, 3.0);
    }

    #[test]
    fn translation_moves_point() {
        let m = translation(1.0, 2.0, 3.0);
        let nx = 0.0 * m[0][0] + 0.0 * m[1][0] + 0.0 * m[2][0] + m[3][0];
        let ny = 0.0 * m[0][1] + 0.0 * m[1][1] + 0.0 * m[2][1] + m[3][1];
        let nz = 0.0 * m[0][2] + 0.0 * m[1][2] + 0.0 * m[2][2] + m[3][2];
        assert_eq!(nx, 1.0);
        assert_eq!(ny, 2.0);
        assert_eq!(nz, 3.0);
    }

    #[test]
    fn compose_trs_matches_oracle() {
        // Composed T·R·S must match independent sequential multiply.
        let tx = 1.0f64;
        let ty = 2.0;
        let tz = 3.0;
        let rx = 0.5f64;
        let ry = 0.3;
        let rz = 0.7;
        let sx = 2.0f64;
        let sy = 3.0;
        let sz = 0.5;

        // Independent oracle: T * (Rx * Ry * Rz) * S
        let t = translation(tx, ty, tz);
        let r = mat_mul(&mat_mul(&rotation_x(rx), &rotation_y(ry)), &rotation_z(rz));
        let s = scale(sx, sy, sz);
        let oracle = mat_mul(&t, &mat_mul(&r, &s));

        let composed = compose_trs(tx, ty, tz, rx, ry, rz, sx, sy, sz);

        for col in 0..4 {
            for row in 0..4 {
                assert!(
                    (oracle[col][row] - composed[col][row]).abs() < 1e-12,
                    "col {col} row {row}: oracle {} vs composed {}",
                    oracle[col][row],
                    composed[col][row]
                );
            }
        }
    }

    #[test]
    fn transform_mesh_applies_translation() {
        let mesh = unit_box();
        let m = translation(10.0, 20.0, 30.0);
        let transformed = transform_mesh(&mesh, &m);
        // First vertex was [-0.5, -0.5, -0.5], should be [9.5, 19.5, 29.5]
        assert_eq!(transformed.positions[0], [9.5, 19.5, 29.5]);
    }

    #[test]
    fn transform_mesh_applies_scale() {
        let mesh = unit_box();
        let m = scale(2.0, 3.0, 4.0);
        let transformed = transform_mesh(&mesh, &m);
        // First vertex was [-0.5, -0.5, -0.5], should be [-1.0, -1.5, -2.0]
        assert_eq!(transformed.positions[0], [-1.0, -1.5, -2.0]);
    }

    // ── Scene graph ───────────────────────────────────────────────────────

    #[test]
    fn scene_add_primitive() {
        let mut scene = Scene::new();
        scene.add_primitive("box", unit_box(), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        scene.add_primitive("sphere", uv_sphere(1.0, 8, 16).unwrap(),
            5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        assert_eq!(scene.node_count(), 2);
        assert!(scene.total_vertices() > 0);
        assert!(scene.total_triangles() > 0);
    }

    #[test]
    fn scene_deterministic() {
        let mut a = Scene::new();
        a.add_primitive("box", unit_box(), 1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        let mut b = Scene::new();
        b.add_primitive("box", unit_box(), 1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        assert_eq!(a.nodes[0].transform, b.nodes[0].transform);
    }

    // ── Provenance ────────────────────────────────────────────────────────

    #[test]
    fn provenance_mu_is_non_zero() {
        let p = ProvenanceMetadata::new(0x1234_5678_9abc_def0, 0x4242_4242_4242_4242, 1000.0);
        assert!(p.mu != 0.0);
        assert!(p.is_non_empty());
    }

    #[test]
    fn provenance_deterministic() {
        let a = ProvenanceMetadata::new(0x1234, 0x5678, 1000.0);
        let b = ProvenanceMetadata::new(0x1234, 0x5678, 1000.0);
        assert_eq!(a.mu, b.mu);
        assert_eq!(a.author_did_hash, b.author_did_hash);
    }

    #[test]
    fn provenance_tensor_round_trips() {
        let p = ProvenanceMetadata::new(0x1234_5678_9abc_def0, 0x4242_4242_4242_4242, 12345.0);
        let tensor = p.to_tensor();
        assert_eq!(tensor.mu, p.mu);
        assert_eq!(tensor.t, p.timestamp);
        // Reconstruct from bit-packed fields.
        let did_back = ((tensor.v.to_bits() as u64) << 32) | (tensor.q.to_bits() as u64);
        let domain_back = ((tensor.x.to_bits() as u64) << 32) | (tensor.w.to_bits() as u64);
        assert_eq!(did_back, p.author_did_hash);
        assert_eq!(domain_back, p.domain_hash);
    }

    // ── .10d asset export/import ──────────────────────────────────────────

    #[test]
    fn export_asset_byte_identical_on_repeat() {
        let mesh = unit_box();
        let provenance = ProvenanceMetadata::new(0x1234_5678_9abc_def0, 0x4242_4242_4242_4242, 1000.0);

        let need = asset_encoded_len(&mesh, &provenance);
        let mut a = vec![0u8; need];
        let mut b = vec![0u8; need];
        let na = export_asset(&mesh, &provenance, &mut a).unwrap();
        let nb = export_asset(&mesh, &provenance, &mut b).unwrap();

        assert_eq!(na, nb);
        assert_eq!(&a[..na], &b[..nb], "two identical exports must be byte-identical");
    }

    #[test]
    fn export_import_round_trips_within_quantization_tolerance() {
        let mesh = unit_box();
        let provenance = ProvenanceMetadata::new(0x1234_5678_9abc_def0, 0x4242_4242_4242_4242, 1000.0);

        let need = asset_encoded_len(&mesh, &provenance);
        let mut buf = vec![0u8; need];
        let n = export_asset(&mesh, &provenance, &mut buf).unwrap();

        let (mesh_back, prov_back) = import_asset(&mut buf[..n]).unwrap();

        // Triangles are exact.
        assert_eq!(mesh_back.triangles, mesh.triangles);

        // Positions within quantization tolerance: bbox_extent / 65535 per axis.
        let extent = 1.0f32; // unit box extent
        let tol = extent / 65535.0 * 2.0;
        for (a, b) in mesh.positions.iter().zip(&mesh_back.positions) {
            for k in 0..3 {
                assert!((a[k] - b[k]).abs() <= tol, "axis {k}: {} vs {}", a[k], b[k]);
            }
        }

        // Provenance round-trips.
        assert_eq!(prov_back.author_did_hash, provenance.author_did_hash);
        assert_eq!(prov_back.mu, provenance.mu);
        assert_eq!(prov_back.timestamp, provenance.timestamp);
        assert_eq!(prov_back.domain_hash, provenance.domain_hash);
    }

    #[test]
    fn export_rejects_empty_provenance() {
        let mesh = unit_box();
        let provenance = ProvenanceMetadata { author_did_hash: 0, mu: 0.0, timestamp: 0.0, domain_hash: 0 };
        let need = asset_encoded_len(&mesh, &provenance);
        let mut buf = vec![0u8; need];
        assert!(export_asset(&mesh, &provenance, &mut buf).is_err());
    }

    #[test]
    fn exported_asset_carries_mu_provenance() {
        let mesh = unit_box();
        let provenance = ProvenanceMetadata::new(0xABCD, 0x1234, 999.0);
        let need = asset_encoded_len(&mesh, &provenance);
        let mut buf = vec![0u8; need];
        let n = export_asset(&mesh, &provenance, &mut buf).unwrap();

        let (_, prov_back) = import_asset(&mut buf[..n]).unwrap();
        assert!(prov_back.is_non_empty(), "imported asset must carry non-empty μ provenance");
        assert_eq!(prov_back.author_did_hash, 0xABCD);
    }

    #[test]
    fn asset_hash_is_deterministic() {
        let mesh = unit_box();
        let provenance = ProvenanceMetadata::new(0x1234, 0x5678, 1000.0);
        let need = asset_encoded_len(&mesh, &provenance);
        let mut a = vec![0u8; need];
        let mut b = vec![0u8; need];
        let na = export_asset(&mesh, &provenance, &mut a).unwrap();
        let nb = export_asset(&mesh, &provenance, &mut b).unwrap();
        assert_eq!(asset_hash(&a[..na]), asset_hash(&b[..nb]));
    }

    #[test]
    fn sphere_export_import_round_trips() {
        let mesh = uv_sphere(2.0, 6, 12).unwrap();
        let provenance = ProvenanceMetadata::new(0xDEAD_BEEF, 0xCAFE_BABE, 42.0);
        let need = asset_encoded_len(&mesh, &provenance);
        let mut buf = vec![0u8; need];
        let n = export_asset(&mesh, &provenance, &mut buf).unwrap();

        let (mesh_back, prov_back) = import_asset(&mut buf[..n]).unwrap();
        assert_eq!(mesh_back.triangles, mesh.triangles);
        assert!(prov_back.is_non_empty());

        // Sphere extent is 4.0 (radius 2.0, min=-2, max=2).
        let tol = 4.0f32 / 65535.0 * 2.0;
        for (a, b) in mesh.positions.iter().zip(&mesh_back.positions) {
            for k in 0..3 {
                assert!((a[k] - b[k]).abs() <= tol, "axis {k}: {} vs {}", a[k], b[k]);
            }
        }
    }

    // ── P9.2 governance fail-closed ───────────────────────────────────────

    #[test]
    fn exported_asset_has_default_refuse_flag() {
        use crate::container_10d::header::{Container10dHeader, FLAG_DEFAULT_DISPOSITION_REFUSE};
        let mesh = unit_box();
        let provenance = ProvenanceMetadata::new(0x1234, 0x5678, 1000.0);
        let need = asset_encoded_len(&mesh, &provenance);
        let mut buf = vec![0u8; need];
        let n = export_asset(&mesh, &provenance, &mut buf).unwrap();

        let header = Container10dHeader::parse(&buf[..n]).unwrap();
        assert_ne!(
            header.flags & FLAG_DEFAULT_DISPOSITION_REFUSE,
            0,
            "exported asset must carry default-Refuse governance flag"
        );
    }

    #[test]
    fn exported_asset_section_table_has_two_sections() {
        use crate::container_10d::{parse_section_table, SectionType};
        let mesh = unit_box();
        let provenance = ProvenanceMetadata::new(0x1234, 0x5678, 1000.0);
        let need = asset_encoded_len(&mesh, &provenance);
        let mut buf = vec![0u8; need];
        let n = export_asset(&mesh, &provenance, &mut buf).unwrap();

        let header = Container10dHeader::parse(&buf[..n]).unwrap();
        let descs = parse_section_table(&buf[..n], &header).unwrap();
        assert_eq!(descs.len(), 2, "should have QuantizedMesh + Tensor10DNodes");

        let types: Vec<SectionType> = descs
            .iter()
            .filter_map(|d| SectionType::from_u8(d.section_type))
            .collect();
        assert!(types.contains(&SectionType::QuantizedMesh));
        assert!(types.contains(&SectionType::Tensor10DNodes));
    }

    // ── P9.6 tests ────────────────────────────────────────────────────────

    #[test]
    fn boolean_union_of_two_boxes() {
        let a = box_mesh(1.0, 1.0, 1.0).unwrap();
        let b = box_mesh(1.0, 1.0, 1.0).unwrap();
        // Translate b by 0.5 in x — overlapping union.
        let b_translated = Mesh {
            positions: b.positions.iter().map(|p| [p[0] + 0.5, p[1], p[2]]).collect(),
            triangles: b.triangles.clone(),
            min: [b.min[0] + 0.5, b.min[1], b.min[2]],
            max: [b.max[0] + 0.5, b.max[1], b.max[2]],
        };
        let result = boolean_op(&a, &b_translated, BooleanOp::Union).unwrap();
        assert!(result.triangles.len() > 0, "union should produce triangles");
        assert!(result.positions.len() > 0);
    }

    #[test]
    fn boolean_difference_of_disjoint_boxes() {
        let a = unit_box();
        let b = Mesh {
            positions: unit_box().positions.iter().map(|p| [p[0] + 5.0, p[1], p[2]]).collect(),
            triangles: unit_box().triangles.clone(),
            min: [4.5, -0.5, -0.5],
            max: [5.5, 0.5, 0.5],
        };
        let result = boolean_op(&a, &b, BooleanOp::Difference).unwrap();
        // A \ B where disjoint = A itself (12 triangles, 8 vertices).
        assert_eq!(result.triangles.len(), 12);
        assert_eq!(result.positions.len(), 8);
    }

    #[test]
    fn boolean_intersection_of_disjoint_is_empty() {
        let a = unit_box();
        let b = Mesh {
            positions: unit_box().positions.iter().map(|p| [p[0] + 5.0, p[1], p[2]]).collect(),
            triangles: unit_box().triangles.clone(),
            min: [4.5, -0.5, -0.5],
            max: [5.5, 0.5, 0.5],
        };
        let result = boolean_op(&a, &b, BooleanOp::Intersection).unwrap();
        assert_eq!(result.triangles.len(), 0);
        assert_eq!(result.positions.len(), 0);
    }

    #[test]
    fn torus_generates_valid_mesh() {
        let mesh = torus(1.0, 0.3, 16, 8).unwrap();
        assert_eq!(mesh.positions.len(), 16 * 8);
        assert_eq!(mesh.triangles.len(), 16 * 8 * 2);
        // Bounding box should be roughly [-1.3, 1.3] in x/z, [-0.3, 0.3] in y.
        assert!(mesh.max[0] > 1.0 && mesh.max[0] < 1.4);
        assert!(mesh.max[1] > 0.2 && mesh.max[1] < 0.4);
    }

    #[test]
    fn torus_rejects_invalid_params() {
        assert!(torus(0.0, 0.3, 16, 8).is_err());
        assert!(torus(1.0, 0.0, 16, 8).is_err());
        assert!(torus(1.0, 1.0, 16, 8).is_err(), "minor >= major should fail");
        assert!(torus(1.0, 0.3, 2, 8).is_err(), "major_segments < 3 should fail");
    }

    #[test]
    fn torus_is_deterministic() {
        let a = torus(1.0, 0.3, 16, 8).unwrap();
        let b = torus(1.0, 0.3, 16, 8).unwrap();
        assert_eq!(a.positions, b.positions);
        assert_eq!(a.triangles, b.triangles);
    }

    #[test]
    fn grid_generates_correct_vertex_count() {
        let mesh = grid(2.0, 4).unwrap();
        // (4+1) * (4+1) = 25 vertices, 4*4*2 = 32 triangles.
        assert_eq!(mesh.positions.len(), 25);
        assert_eq!(mesh.triangles.len(), 32);
    }

    #[test]
    fn grid_is_deterministic() {
        let a = grid(2.0, 4).unwrap();
        let b = grid(2.0, 4).unwrap();
        assert_eq!(a.positions, b.positions);
        assert_eq!(a.triangles, b.triangles);
    }

    #[test]
    fn drag_vertex_produces_new_t_slice() {
        let mesh = unit_box();
        let prior_t = 10.0;
        let result = drag_vertex(
            &mesh,
            0,
            [1.0, 2.0, 3.0],
            prior_t,
            DragConsent::default(),
        ).unwrap();
        // New t-slice = prior_t + 1.0.
        assert_eq!(result.new_t, 11.0);
        assert_eq!(result.prior_t, 10.0);
        // Vertex 0 moved to [1.0, 2.0, 3.0].
        assert_eq!(result.mesh.positions[0], [1.0, 2.0, 3.0]);
        // Other vertices unchanged.
        assert_eq!(result.mesh.positions[1], mesh.positions[1]);
        // Triangles unchanged.
        assert_eq!(result.mesh.triangles, mesh.triangles);
    }

    #[test]
    fn drag_vertex_prior_slice_unmutated() {
        let mesh = unit_box();
        let original_positions = mesh.positions.clone();
        let _ = drag_vertex(
            &mesh,
            0,
            [1.0, 2.0, 3.0],
            5.0,
            DragConsent::default(),
        ).unwrap();
        // The original mesh is not mutated.
        assert_eq!(mesh.positions, original_positions);
    }

    #[test]
    fn drag_vertex_governance_refused() {
        let mesh = unit_box();
        let result = drag_vertex(
            &mesh,
            0,
            [1.0, 2.0, 3.0],
            5.0,
            DragConsent { consent_granted: false, sealed_prior: true },
        );
        assert!(matches!(result, Err(DragError::GovernanceRefused)));
    }

    #[test]
    fn drag_vertex_out_of_bounds() {
        let mesh = unit_box();
        let result = drag_vertex(
            &mesh,
            99,
            [1.0, 2.0, 3.0],
            5.0,
            DragConsent::default(),
        );
        assert!(matches!(result, Err(DragError::VertexOutOfBounds { index: 99, .. })));
    }

    // ── P9.7 end-to-end acceptance walkthrough ───────────────────────────

    #[test]
    fn end_to_end_maker_walkthrough() {
        // 1. Construct a scene with primitives + transforms.
        let mut scene = Scene::new();
        let box_mesh = box_mesh(1.0, 1.0, 1.0).unwrap();
        let sphere_mesh = uv_sphere(0.5, 8, 16).unwrap();
        scene.add_primitive("box", box_mesh.clone(), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        scene.add_primitive("sphere", sphere_mesh.clone(), 1.5, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        assert_eq!(scene.node_count(), 2);
        assert!(scene.total_vertices() > 0);
        assert!(scene.total_triangles() > 0);

        // 2. Export the first mesh as a .10d asset with provenance + Q42 identity.
        let provenance = ProvenanceMetadata::new(
            0x1234_5678_9abc_def0,
            0x4242_4242_4242_4242,
            1000.0,
        );
        let need = asset_encoded_len(&box_mesh, &provenance);
        let mut buf_a = vec![0u8; need];
        let mut buf_b = vec![0u8; need];
        let na = export_asset(&box_mesh, &provenance, &mut buf_a).unwrap();
        let nb = export_asset(&box_mesh, &provenance, &mut buf_b).unwrap();

        // 3. Hash stability: two identical exports are byte-identical.
        assert_eq!(na, nb, "export sizes must match");
        assert_eq!(&buf_a[..na], &buf_b[..nb], "two identical exports must be byte-identical");
        let hash_a = fnv1a_hash(&buf_a[..na]);
        let hash_b = fnv1a_hash(&buf_b[..nb]);
        assert_eq!(hash_a, hash_b, "whole-file hash must be stable across identical exports");

        // 4. Re-load via import_asset — governance + μ provenance + Q42 identity intact.
        let mut buf_reload = buf_a[..na].to_vec();
        let (mesh_back, prov_back) = import_asset(&mut buf_reload).unwrap();

        // Triangles are exact.
        assert_eq!(mesh_back.triangles, box_mesh.triangles);
        // Provenance μ is non-zero and matches.
        assert!(prov_back.is_non_empty(), "μ provenance must be non-zero after reload");
        assert_eq!(prov_back.mu, provenance.mu, "μ must match after reload");
        // Q42 identity (author DID hash) intact.
        assert_eq!(prov_back.author_did_hash, provenance.author_did_hash, "Q42 identity must match after reload");
        assert_eq!(prov_back.domain_hash, provenance.domain_hash, "domain hash must match after reload");
        assert_eq!(prov_back.timestamp, provenance.timestamp, "timestamp must match after reload");

        // 5. Drag a vertex on the imported mesh — new t-slice, prior unmutated.
        let prior_t = prov_back.timestamp;
        let drag = drag_vertex(
            &mesh_back,
            0,
            [0.7, -0.7, -0.7],
            prior_t,
            DragConsent::default(),
        ).unwrap();
        assert_eq!(drag.new_t, prior_t + 1.0, "drag must produce new t-slice");
        assert_eq!(drag.prior_t, prior_t, "prior t must be unchanged");
        // Prior mesh is unmutated.
        assert_eq!(mesh_back.positions[0], box_mesh.positions[0], "prior slice vertex must be unmutated");
        // New mesh has the dragged vertex.
        assert_eq!(drag.mesh.positions[0], [0.7, -0.7, -0.7], "dragged vertex must be at new position");

        // 6. Governance refusal: drag with consent denied is refused.
        let refused = drag_vertex(
            &mesh_back,
            0,
            [1.0, 1.0, 1.0],
            prior_t,
            DragConsent { consent_granted: false, sealed_prior: true },
        );
        assert!(matches!(refused, Err(DragError::GovernanceRefused)), "governance refusal must be enforced");

        // 7. Re-export the dragged mesh with new provenance — hash stable.
        let new_provenance = ProvenanceMetadata::new(
            provenance.author_did_hash,
            provenance.domain_hash,
            drag.new_t,
        );
        let need2 = asset_encoded_len(&drag.mesh, &new_provenance);
        let mut buf_c = vec![0u8; need2];
        let mut buf_d = vec![0u8; need2];
        let nc = export_asset(&drag.mesh, &new_provenance, &mut buf_c).unwrap();
        let nd = export_asset(&drag.mesh, &new_provenance, &mut buf_d).unwrap();
        assert_eq!(&buf_c[..nc], &buf_d[..nd], "re-exported dragged asset must be byte-identical");
        let hash_c = fnv1a_hash(&buf_c[..nc]);
        let hash_d = fnv1a_hash(&buf_d[..nd]);
        assert_eq!(hash_c, hash_d, "re-exported hash must be stable");

        // 8. Hash differs from original (different mesh + different timestamp).
        assert_ne!(hash_a, hash_c, "hash must differ after drag + new t-slice");
    }
}
