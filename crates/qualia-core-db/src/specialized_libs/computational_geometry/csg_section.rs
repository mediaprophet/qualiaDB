//! CSG/arrangement `.10d` sections and repair operations (P12.9).
//!
//! Serializes CSG expression trees, exact-point pools, region labels, and
//! output meshes into canonical `.10d` sections with per-section CRC-32C.
//! Repair operations extract shells, remove duplicate sheets, and report
//! unresolved non-manifold input.
//!
//! ## Format
//!
//! ```text
//! [4 bytes: magic "CSG1"]
//! [1 byte: version]
//! [1 byte: section type]
//! [2 bytes: flags]
//! [4 bytes: vertex count]
//! [4 bytes: triangle count]
//! [4 bytes: region label count]
//! [4 bytes: expression tree byte length]
//! [vertex_count * 24 bytes: vertices (f64 x,y,z)]
//! [triangle_count * 12 bytes: triangles (u32 x3)]
//! [region_label_count * 4 bytes: region labels (u32 each)]
//! [expr_tree_len bytes: serialized expression tree]
//! [4 bytes: CRC-32C of all preceding bytes]
//! ```
//!
//! ## Acceptance gate (P12.9)
//!
//! Expression tree, exact-point pool, region labels and output mesh round-trip
//! canonically; repair can extract shells, remove duplicate sheets and report
//! unresolved non-manifold input.
//!
//! Tier-2 cold construction.

use super::arrangement_3d::EdgeKey;
use super::nary_boolean::BoolExpr;
use super::primitives::Point3;
use super::recon_section::crc32c;

// ───────────────────────────────────────────────────────────────────────────
//  Constants
// ───────────────────────────────────────────────────────────────────────────

/// Magic bytes for CSG sections: "CSG1".
pub const CSG_MAGIC: [u8; 4] = *b"CSG1";

/// Current version.
pub const CSG_VERSION: u8 = 1;

/// Section types.
pub const CSG_TYPE_EXPRESSION: u8 = 0;
pub const CSG_TYPE_ARRANGEMENT: u8 = 1;
pub const CSG_TYPE_REPAIR_REPORT: u8 = 2;

/// Header size (fixed part before variable data).
pub const CSG_HEADER_SIZE: usize = 4 + 1 + 1 + 2 + 4 + 4 + 4 + 4;

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsgSectionError {
    PayloadTooShort { got: usize, need: usize },
    MagicMismatch { got: [u8; 4] },
    UnsupportedVersion { got: u8 },
    UnknownType { got: u8 },
    SizeMismatch { expected: usize, got: usize },
    CrcMismatch { expected: u32, got: u32 },
    InvalidExprTree { reason: String },
}

impl core::fmt::Display for CsgSectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PayloadTooShort { got, need } => {
                write!(f, "csg_section: payload too short, {got} < {need}")
            }
            Self::MagicMismatch { got } => {
                write!(f, "csg_section: magic mismatch, got {:?}", got)
            }
            Self::UnsupportedVersion { got } => {
                write!(f, "csg_section: unsupported version {got}")
            }
            Self::UnknownType { got } => {
                write!(f, "csg_section: unknown type {got}")
            }
            Self::SizeMismatch { expected, got } => {
                write!(
                    f,
                    "csg_section: size mismatch, expected {expected}, got {got}"
                )
            }
            Self::CrcMismatch { expected, got } => {
                write!(
                    f,
                    "csg_section: CRC mismatch, expected {expected:#x}, got {got:#x}"
                )
            }
            Self::InvalidExprTree { reason } => {
                write!(f, "csg_section: invalid expression tree — {reason}")
            }
        }
    }
}

impl std::error::Error for CsgSectionError {}

// ───────────────────────────────────────────────────────────────────────────
//  Expression tree serialization
// ───────────────────────────────────────────────────────────────────────────

/// Serialize a `BoolExpr` to bytes.
///
/// Format:
/// - 1 byte: node type tag
/// - For Operand: 4 bytes (u32 operand index)
/// - For Union/Intersection: 4 bytes (u32 child count) + children
/// - For Difference: first child + 4 bytes (u32 rest count) + rest children
/// - For Xor: two children
/// - For Complement: one child
///
/// Node type tags:
/// - 0: Operand
/// - 1: Union
/// - 2: Intersection
/// - 3: Difference
/// - 4: Xor
/// - 5: Complement
pub fn serialize_expr(expr: &BoolExpr) -> Vec<u8> {
    let mut buf = Vec::new();
    serialize_expr_recursive(expr, &mut buf);
    buf
}

fn serialize_expr_recursive(expr: &BoolExpr, buf: &mut Vec<u8>) {
    match expr {
        BoolExpr::Operand(i) => {
            buf.push(0u8);
            buf.extend_from_slice(&(*i as u32).to_le_bytes());
        }
        BoolExpr::Union(children) => {
            buf.push(1u8);
            buf.extend_from_slice(&(children.len() as u32).to_le_bytes());
            for c in children {
                serialize_expr_recursive(c, buf);
            }
        }
        BoolExpr::Intersection(children) => {
            buf.push(2u8);
            buf.extend_from_slice(&(children.len() as u32).to_le_bytes());
            for c in children {
                serialize_expr_recursive(c, buf);
            }
        }
        BoolExpr::Difference(first, rest) => {
            buf.push(3u8);
            serialize_expr_recursive(first, buf);
            buf.extend_from_slice(&(rest.len() as u32).to_le_bytes());
            for c in rest {
                serialize_expr_recursive(c, buf);
            }
        }
        BoolExpr::Xor(a, b) => {
            buf.push(4u8);
            serialize_expr_recursive(a, buf);
            serialize_expr_recursive(b, buf);
        }
        BoolExpr::Complement(a) => {
            buf.push(5u8);
            serialize_expr_recursive(a, buf);
        }
    }
}

/// Deserialize a `BoolExpr` from bytes.
pub fn deserialize_expr(bytes: &[u8]) -> Result<BoolExpr, CsgSectionError> {
    let mut pos = 0usize;
    let expr = deserialize_expr_recursive(bytes, &mut pos)?;
    if pos != bytes.len() {
        return Err(CsgSectionError::InvalidExprTree {
            reason: format!("trailing bytes: {} unread", bytes.len() - pos),
        });
    }
    Ok(expr)
}

fn deserialize_expr_recursive(bytes: &[u8], pos: &mut usize) -> Result<BoolExpr, CsgSectionError> {
    if *pos >= bytes.len() {
        return Err(CsgSectionError::InvalidExprTree {
            reason: "unexpected end of data".to_string(),
        });
    }
    let tag = bytes[*pos];
    *pos += 1;

    match tag {
        0 => {
            // Operand
            if *pos + 4 > bytes.len() {
                return Err(CsgSectionError::InvalidExprTree {
                    reason: "Operand: not enough bytes for index".to_string(),
                });
            }
            let i = u32::from_le_bytes([
                bytes[*pos],
                bytes[*pos + 1],
                bytes[*pos + 2],
                bytes[*pos + 3],
            ]) as usize;
            *pos += 4;
            Ok(BoolExpr::Operand(i))
        }
        1 => {
            // Union
            let children = deserialize_children(bytes, pos)?;
            Ok(BoolExpr::Union(children))
        }
        2 => {
            // Intersection
            let children = deserialize_children(bytes, pos)?;
            Ok(BoolExpr::Intersection(children))
        }
        3 => {
            // Difference
            let first = deserialize_expr_recursive(bytes, pos)?;
            let rest = deserialize_children(bytes, pos)?;
            Ok(BoolExpr::Difference(Box::new(first), rest))
        }
        4 => {
            // Xor
            let a = deserialize_expr_recursive(bytes, pos)?;
            let b = deserialize_expr_recursive(bytes, pos)?;
            Ok(BoolExpr::Xor(Box::new(a), Box::new(b)))
        }
        5 => {
            // Complement
            let a = deserialize_expr_recursive(bytes, pos)?;
            Ok(BoolExpr::Complement(Box::new(a)))
        }
        _ => Err(CsgSectionError::InvalidExprTree {
            reason: format!("unknown node type tag: {tag}"),
        }),
    }
}

fn deserialize_children(bytes: &[u8], pos: &mut usize) -> Result<Vec<BoolExpr>, CsgSectionError> {
    if *pos + 4 > bytes.len() {
        return Err(CsgSectionError::InvalidExprTree {
            reason: "not enough bytes for child count".to_string(),
        });
    }
    let count = u32::from_le_bytes([
        bytes[*pos],
        bytes[*pos + 1],
        bytes[*pos + 2],
        bytes[*pos + 3],
    ]) as usize;
    *pos += 4;

    let mut children = Vec::with_capacity(count);
    for _ in 0..count {
        children.push(deserialize_expr_recursive(bytes, pos)?);
    }
    Ok(children)
}

// ───────────────────────────────────────────────────────────────────────────
//  CSG section encode/decode
// ───────────────────────────────────────────────────────────────────────────

/// A CSG section payload: mesh + region labels + expression tree.
#[derive(Debug, Clone)]
pub struct CsgSection {
    pub section_type: u8,
    pub flags: u16,
    pub vertices: Vec<Point3>,
    pub triangles: Vec<[u32; 3]>,
    pub region_labels: Vec<u32>,
    pub expr_tree: Option<BoolExpr>,
}

/// Encode a CSG section to bytes.
///
/// The output includes a CRC-32C trailer.
pub fn encode_csg_section(section: &CsgSection) -> Vec<u8> {
    let expr_bytes = match &section.expr_tree {
        Some(expr) => serialize_expr(expr),
        None => Vec::new(),
    };

    let needed = CSG_HEADER_SIZE
        + section.vertices.len() * 24
        + section.triangles.len() * 12
        + section.region_labels.len() * 4
        + expr_bytes.len()
        + 4;

    let mut buf = Vec::with_capacity(needed);

    // Header.
    buf.extend_from_slice(&CSG_MAGIC);
    buf.push(CSG_VERSION);
    buf.push(section.section_type);
    buf.extend_from_slice(&section.flags.to_le_bytes());
    buf.extend_from_slice(&(section.vertices.len() as u32).to_le_bytes());
    buf.extend_from_slice(&(section.triangles.len() as u32).to_le_bytes());
    buf.extend_from_slice(&(section.region_labels.len() as u32).to_le_bytes());
    buf.extend_from_slice(&(expr_bytes.len() as u32).to_le_bytes());

    // Vertices.
    for v in &section.vertices {
        buf.extend_from_slice(&v.x.to_le_bytes());
        buf.extend_from_slice(&v.y.to_le_bytes());
        buf.extend_from_slice(&v.z.to_le_bytes());
    }

    // Triangles.
    for tri in &section.triangles {
        buf.extend_from_slice(&tri[0].to_le_bytes());
        buf.extend_from_slice(&tri[1].to_le_bytes());
        buf.extend_from_slice(&tri[2].to_le_bytes());
    }

    // Region labels.
    for label in &section.region_labels {
        buf.extend_from_slice(&label.to_le_bytes());
    }

    // Expression tree.
    buf.extend_from_slice(&expr_bytes);

    // CRC-32C.
    let crc = crc32c(&buf);
    buf.extend_from_slice(&crc.to_le_bytes());

    buf
}

/// Decoded CSG section.
#[derive(Debug, Clone)]
pub struct DecodedCsgSection {
    pub section_type: u8,
    pub flags: u16,
    pub vertices: Vec<Point3>,
    pub triangles: Vec<[u32; 3]>,
    pub region_labels: Vec<u32>,
    pub expr_tree: Option<BoolExpr>,
}

/// Decode a CSG section from bytes.
///
/// Verifies magic, version, and CRC-32C.
pub fn decode_csg_section(bytes: &[u8]) -> Result<DecodedCsgSection, CsgSectionError> {
    if bytes.len() < CSG_HEADER_SIZE + 4 {
        return Err(CsgSectionError::PayloadTooShort {
            got: bytes.len(),
            need: CSG_HEADER_SIZE + 4,
        });
    }

    // Magic (check before CRC for correct error type on corrupted headers).
    let magic = [bytes[0], bytes[1], bytes[2], bytes[3]];
    if magic != CSG_MAGIC {
        return Err(CsgSectionError::MagicMismatch { got: magic });
    }

    // Verify CRC.
    let data_len = bytes.len() - 4;
    let stored_crc = u32::from_le_bytes([
        bytes[data_len],
        bytes[data_len + 1],
        bytes[data_len + 2],
        bytes[data_len + 3],
    ]);
    let computed_crc = crc32c(&bytes[..data_len]);
    if stored_crc != computed_crc {
        return Err(CsgSectionError::CrcMismatch {
            expected: stored_crc,
            got: computed_crc,
        });
    }

    // Version.
    let version = bytes[4];
    if version != CSG_VERSION {
        return Err(CsgSectionError::UnsupportedVersion { got: version });
    }

    let section_type = bytes[5];
    let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
    let v_count = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let t_count = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
    let r_count = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;
    let e_len = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]) as usize;

    let needed = CSG_HEADER_SIZE + v_count * 24 + t_count * 12 + r_count * 4 + e_len + 4;
    if bytes.len() < needed {
        return Err(CsgSectionError::SizeMismatch {
            expected: needed,
            got: bytes.len(),
        });
    }

    let mut pos = CSG_HEADER_SIZE;

    // Vertices.
    let mut vertices = Vec::with_capacity(v_count);
    for _ in 0..v_count {
        let x = f64::from_le_bytes([
            bytes[pos],
            bytes[pos + 1],
            bytes[pos + 2],
            bytes[pos + 3],
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]);
        let y = f64::from_le_bytes([
            bytes[pos + 8],
            bytes[pos + 9],
            bytes[pos + 10],
            bytes[pos + 11],
            bytes[pos + 12],
            bytes[pos + 13],
            bytes[pos + 14],
            bytes[pos + 15],
        ]);
        let z = f64::from_le_bytes([
            bytes[pos + 16],
            bytes[pos + 17],
            bytes[pos + 18],
            bytes[pos + 19],
            bytes[pos + 20],
            bytes[pos + 21],
            bytes[pos + 22],
            bytes[pos + 23],
        ]);
        vertices.push(Point3::new(x, y, z));
        pos += 24;
    }

    // Triangles.
    let mut triangles = Vec::with_capacity(t_count);
    for _ in 0..t_count {
        let a = u32::from_le_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]);
        let b = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]);
        let c = u32::from_le_bytes([
            bytes[pos + 8],
            bytes[pos + 9],
            bytes[pos + 10],
            bytes[pos + 11],
        ]);
        triangles.push([a, b, c]);
        pos += 12;
    }

    // Region labels.
    let mut region_labels = Vec::with_capacity(r_count);
    for _ in 0..r_count {
        let label =
            u32::from_le_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]);
        region_labels.push(label);
        pos += 4;
    }

    // Expression tree.
    let expr_tree = if e_len > 0 {
        let expr_bytes = &bytes[pos..pos + e_len];
        Some(deserialize_expr(expr_bytes)?)
    } else {
        None
    };

    Ok(DecodedCsgSection {
        section_type,
        flags,
        vertices,
        triangles,
        region_labels,
        expr_tree,
    })
}

// ───────────────────────────────────────────────────────────────────────────
//  Repair operations
// ───────────────────────────────────────────────────────────────────────────

/// A repair report: issues found and actions taken.
#[derive(Debug, Clone, PartialEq)]
pub struct RepairReport {
    /// Number of duplicate sheets removed.
    pub duplicate_sheets_removed: usize,
    /// Number of shells extracted.
    pub shells_extracted: usize,
    /// Unresolved non-manifold edges (edge → facet count).
    pub non_manifold_edges: Vec<(EdgeKey, usize)>,
    /// Whether the mesh is manifold after repair.
    pub is_manifold: bool,
    /// Number of degenerate triangles removed.
    pub degenerate_triangles_removed: usize,
}

/// Repair a triangle mesh: extract shells, remove duplicate sheets,
/// and report unresolved non-manifold input.
///
/// # Algorithm
///
/// 1. Remove degenerate triangles (repeated vertices).
/// 2. Find duplicate sheets (identical triangles with opposite winding).
/// 3. Identify non-manifold edges (edges with >2 incident triangles).
/// 4. Extract shells (connected components).
/// 5. Report findings.
pub fn repair_mesh(_vertices: &[Point3], triangles: &[[u32; 3]]) -> (Vec<[u32; 3]>, RepairReport) {
    let mut report = RepairReport {
        duplicate_sheets_removed: 0,
        shells_extracted: 0,
        non_manifold_edges: Vec::new(),
        is_manifold: true,
        degenerate_triangles_removed: 0,
    };

    // 1. Remove degenerate triangles.
    let mut clean_triangles: Vec<[u32; 3]> = triangles
        .iter()
        .filter(|tri| {
            let degenerate = tri[0] == tri[1] || tri[1] == tri[2] || tri[2] == tri[0];
            if degenerate {
                report.degenerate_triangles_removed += 1;
            }
            !degenerate
        })
        .copied()
        .collect();

    // 2. Find and remove duplicate sheets (identical triangles with opposite winding).
    // Build a set of sorted vertex triples for quick lookup.
    let mut seen: std::collections::BTreeSet<(u32, u32, u32)> = std::collections::BTreeSet::new();
    let mut deduped: Vec<[u32; 3]> = Vec::with_capacity(clean_triangles.len());

    for tri in &clean_triangles {
        // Sorted key for the undirected triangle.
        let key = {
            let mut v = [tri[0], tri[1], tri[2]];
            v.sort_unstable();
            (v[0], v[1], v[2])
        };

        if seen.contains(&key) {
            // Duplicate sheet — skip it.
            report.duplicate_sheets_removed += 1;
        } else {
            seen.insert(key);
            deduped.push(*tri);
        }
    }
    clean_triangles = deduped;

    // 3. Identify non-manifold edges.
    let mut edge_count: std::collections::BTreeMap<EdgeKey, usize> =
        std::collections::BTreeMap::new();
    for tri in &clean_triangles {
        for local in 0..3 {
            let v0 = tri[local];
            let v1 = tri[(local + 1) % 3];
            let key = EdgeKey::new(v0, v1);
            *edge_count.entry(key).or_default() += 1;
        }
    }

    for (key, count) in &edge_count {
        if *count > 2 {
            report.non_manifold_edges.push((*key, *count));
        }
    }

    report.is_manifold = report.non_manifold_edges.is_empty();

    // 4. Extract shells (connected components).
    let n = clean_triangles.len();
    if n > 0 {
        let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut edge_tris: std::collections::BTreeMap<EdgeKey, Vec<usize>> =
            std::collections::BTreeMap::new();
        for (i, tri) in clean_triangles.iter().enumerate() {
            for local in 0..3 {
                let v0 = tri[local];
                let v1 = tri[(local + 1) % 3];
                let key = EdgeKey::new(v0, v1);
                edge_tris.entry(key).or_default().push(i);
            }
        }
        for incident in edge_tris.values() {
            for i in 0..incident.len() {
                for j in (i + 1)..incident.len() {
                    adjacency[incident[i]].push(incident[j]);
                    adjacency[incident[j]].push(incident[i]);
                }
            }
        }

        let mut visited = vec![false; n];
        for start in 0..n {
            if visited[start] {
                continue;
            }
            let mut stack = vec![start];
            visited[start] = true;
            while let Some(tri) = stack.pop() {
                for &neighbor in &adjacency[tri] {
                    if !visited[neighbor] {
                        visited[neighbor] = true;
                        stack.push(neighbor);
                    }
                }
            }
            report.shells_extracted += 1;
        }
    }

    (clean_triangles, report)
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, z: f64) -> Point3 {
        Point3::new(x, y, z)
    }

    #[test]
    fn expr_serialize_deserialize_operand() {
        let expr = BoolExpr::Operand(3);
        let bytes = serialize_expr(&expr);
        let decoded = deserialize_expr(&bytes).unwrap();
        assert_eq!(expr, decoded);
    }

    #[test]
    fn expr_serialize_deserialize_union() {
        let expr = BoolExpr::Union(vec![
            BoolExpr::Operand(0),
            BoolExpr::Operand(1),
            BoolExpr::Operand(2),
        ]);
        let bytes = serialize_expr(&expr);
        let decoded = deserialize_expr(&bytes).unwrap();
        assert_eq!(expr, decoded);
    }

    #[test]
    fn expr_serialize_deserialize_difference() {
        let expr = BoolExpr::Difference(
            Box::new(BoolExpr::Operand(0)),
            vec![BoolExpr::Operand(1), BoolExpr::Operand(2)],
        );
        let bytes = serialize_expr(&expr);
        let decoded = deserialize_expr(&bytes).unwrap();
        assert_eq!(expr, decoded);
    }

    #[test]
    fn expr_serialize_deserialize_xor() {
        let expr = BoolExpr::xor(BoolExpr::Operand(0), BoolExpr::Operand(1));
        let bytes = serialize_expr(&expr);
        let decoded = deserialize_expr(&bytes).unwrap();
        assert_eq!(expr, decoded);
    }

    #[test]
    fn expr_serialize_deserialize_complement() {
        let expr = BoolExpr::complement(BoolExpr::Operand(0));
        let bytes = serialize_expr(&expr);
        let decoded = deserialize_expr(&bytes).unwrap();
        assert_eq!(expr, decoded);
    }

    #[test]
    fn expr_serialize_deserialize_nested() {
        let expr = BoolExpr::union(
            BoolExpr::intersection(BoolExpr::Operand(0), BoolExpr::Operand(1)),
            BoolExpr::difference(BoolExpr::Operand(2), BoolExpr::Operand(0)),
        );
        let bytes = serialize_expr(&expr);
        let decoded = deserialize_expr(&bytes).unwrap();
        assert_eq!(expr, decoded);
    }

    #[test]
    fn csg_section_round_trip() {
        let section = CsgSection {
            section_type: CSG_TYPE_EXPRESSION,
            flags: 0,
            vertices: vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0)],
            triangles: vec![[0, 1, 2]],
            region_labels: vec![0u32],
            expr_tree: Some(BoolExpr::union(BoolExpr::Operand(0), BoolExpr::Operand(1))),
        };

        let encoded = encode_csg_section(&section);
        let decoded = decode_csg_section(&encoded).unwrap();

        assert_eq!(decoded.section_type, CSG_TYPE_EXPRESSION);
        assert_eq!(decoded.flags, 0);
        assert_eq!(decoded.vertices, section.vertices);
        assert_eq!(decoded.triangles, section.triangles);
        assert_eq!(decoded.region_labels, section.region_labels);
        assert_eq!(decoded.expr_tree, section.expr_tree);
    }

    #[test]
    fn csg_section_round_trip_no_expr() {
        let section = CsgSection {
            section_type: CSG_TYPE_ARRANGEMENT,
            flags: 42,
            vertices: vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0)],
            triangles: vec![[0, 1, 2]],
            region_labels: vec![],
            expr_tree: None,
        };

        let encoded = encode_csg_section(&section);
        let decoded = decode_csg_section(&encoded).unwrap();

        assert_eq!(decoded.section_type, CSG_TYPE_ARRANGEMENT);
        assert_eq!(decoded.flags, 42);
        assert_eq!(decoded.vertices, section.vertices);
        assert_eq!(decoded.triangles, section.triangles);
        assert_eq!(decoded.region_labels, Vec::<u32>::new());
        assert_eq!(decoded.expr_tree, None);
    }

    #[test]
    fn csg_section_crc_mismatch_detected() {
        let section = CsgSection {
            section_type: CSG_TYPE_EXPRESSION,
            flags: 0,
            vertices: vec![p(0.0, 0.0, 0.0)],
            triangles: vec![],
            region_labels: vec![],
            expr_tree: None,
        };

        let mut encoded = encode_csg_section(&section);
        // Corrupt a byte in the payload.
        encoded[10] ^= 0xFF;
        let result = decode_csg_section(&encoded);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CsgSectionError::CrcMismatch { .. }
        ));
    }

    #[test]
    fn csg_section_magic_mismatch() {
        let mut encoded = encode_csg_section(&CsgSection {
            section_type: 0,
            flags: 0,
            vertices: vec![],
            triangles: vec![],
            region_labels: vec![],
            expr_tree: None,
        });
        encoded[0] = b'X';
        let result = decode_csg_section(&encoded);
        assert!(matches!(
            result.unwrap_err(),
            CsgSectionError::MagicMismatch { .. }
        ));
    }

    #[test]
    fn repair_removes_degenerate_triangles() {
        let vertices = vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0)];
        let triangles = vec![[0, 1, 2], [0, 0, 1]]; // second is degenerate

        let (clean, report) = repair_mesh(&vertices, &triangles);
        assert_eq!(clean.len(), 1);
        assert_eq!(report.degenerate_triangles_removed, 1);
    }

    #[test]
    fn repair_removes_duplicate_sheets() {
        let vertices = vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0)];
        // Two identical triangles with opposite winding = duplicate sheet.
        let triangles = vec![[0, 1, 2], [2, 1, 0]];

        let (clean, report) = repair_mesh(&vertices, &triangles);
        assert_eq!(clean.len(), 1);
        assert_eq!(report.duplicate_sheets_removed, 1);
    }

    #[test]
    fn repair_reports_non_manifold_edges() {
        let vertices = vec![
            p(0.0, 0.0, 0.0),
            p(0.0, 0.0, 1.0),
            p(1.0, 0.0, 0.5),
            p(-1.0, 0.0, 0.5),
            p(0.0, 1.0, 0.5),
        ];
        // Four triangles sharing edge (0,1) — non-manifold.
        let triangles = vec![[0, 2, 1], [0, 1, 3], [0, 1, 4], [1, 4, 3]];

        let (_clean, report) = repair_mesh(&vertices, &triangles);
        assert!(!report.is_manifold);
        assert!(!report.non_manifold_edges.is_empty());
    }

    #[test]
    fn repair_manifold_mesh() {
        let vertices = vec![
            p(0.0, 0.0, 0.0),
            p(1.0, 0.0, 0.0),
            p(0.0, 1.0, 0.0),
            p(0.0, 0.0, 1.0),
        ];
        let triangles = vec![[0, 1, 2], [0, 3, 1], [1, 3, 2], [2, 3, 0]];

        let (clean, report) = repair_mesh(&vertices, &triangles);
        assert_eq!(clean.len(), 4);
        assert!(report.is_manifold);
        assert_eq!(report.shells_extracted, 1);
    }

    #[test]
    fn repair_disjoint_shells() {
        let vertices = vec![
            p(0.0, 0.0, 0.0),
            p(1.0, 0.0, 0.0),
            p(0.0, 1.0, 0.0),
            p(10.0, 0.0, 0.0),
            p(11.0, 0.0, 0.0),
            p(10.0, 1.0, 0.0),
        ];
        let triangles = vec![[0, 1, 2], [3, 4, 5]];

        let (_clean, report) = repair_mesh(&vertices, &triangles);
        assert_eq!(report.shells_extracted, 2);
    }

    #[test]
    fn csg_section_determinism() {
        let section = CsgSection {
            section_type: CSG_TYPE_EXPRESSION,
            flags: 0,
            vertices: vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0)],
            triangles: vec![[0, 1, 2]],
            region_labels: vec![0u32, 0u32],
            expr_tree: Some(BoolExpr::union(BoolExpr::Operand(0), BoolExpr::Operand(1))),
        };

        let e1 = encode_csg_section(&section);
        let e2 = encode_csg_section(&section);
        assert_eq!(e1, e2);
    }
}
