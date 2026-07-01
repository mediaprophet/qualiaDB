//! Asset-import bridge — `OBJ` / `STL` mesh → `NQuin` stream (Phase 1.3,
//! `RENDERER_IMPLEMENTATION_PLAN.md`).
//!
//! Pure `&[u8]` → (geometry, semantic NQuins). **No `std::fs`** — wasm-safe (migration review
//! §2.1): the shell / CLI reads the file (or runs the OS picker) and hands the bytes down. This is
//! the ingest path (not a hot path), so `Vec` / `HashMap` are fine — the same convention as
//! `kml_bridge`.
//!
//! Two layers, per STELLAR §E ("artefacts carry their geometry, and are *semantically known*"):
//!   * [`Mesh`] — raw geometry (vertex positions + triangle indices + bounding box): the data the
//!     GPU vertex/index buffers (Phase 1.2) will consume.
//!   * [`mesh_to_nquins`] — the **semantic** layer: the asset is *known* (type, counts, bounding
//!     box, centroid, source format) as NQuins in the one identity space — not just points/pixels.
//!
//! Hot-path rendering (depth-stencil, mesh buffers, projection) is the GPU half of Phase 1 and is
//! verified on hardware; this module is the CPU half and is unit-tested here.

use std::collections::HashMap;

use crate::frame_layout::pack_float_object;
use crate::{q_hash, NQuin};
use serde_json::Value;

// ── Named-graph context + predicate / class hashes (one identity space; `q_hash`) ──────────────
pub const GEOMETRY_CONTEXT: u64 = q_hash("urn:qualia:context:geometry");

const P_RDF_TYPE: u64 = q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
const C_MESH: u64 = q_hash("urn:qualia:geometry:Mesh");
const P_VERTEX_COUNT: u64 = q_hash("urn:qualia:geometry:vertexCount");
const P_TRIANGLE_COUNT: u64 = q_hash("urn:qualia:geometry:triangleCount");
const P_SOURCE_FORMAT: u64 = q_hash("urn:qualia:geometry:sourceFormat");
const P_BBOX_MIN_X: u64 = q_hash("urn:qualia:geometry:bboxMinX");
const P_BBOX_MIN_Y: u64 = q_hash("urn:qualia:geometry:bboxMinY");
const P_BBOX_MIN_Z: u64 = q_hash("urn:qualia:geometry:bboxMinZ");
const P_BBOX_MAX_X: u64 = q_hash("urn:qualia:geometry:bboxMaxX");
const P_BBOX_MAX_Y: u64 = q_hash("urn:qualia:geometry:bboxMaxY");
const P_BBOX_MAX_Z: u64 = q_hash("urn:qualia:geometry:bboxMaxZ");
const P_CENTROID_X: u64 = q_hash("urn:qualia:geometry:centroidX");
const P_CENTROID_Y: u64 = q_hash("urn:qualia:geometry:centroidY");
const P_CENTROID_Z: u64 = q_hash("urn:qualia:geometry:centroidZ");

/// Error type for asset import.
#[derive(Debug, PartialEq, Eq)]
pub enum AssetError {
    /// The bytes parsed but produced no geometry.
    Empty,
    /// The format could not be recognised from the bytes (and no usable hint was given).
    UnknownFormat,
    /// A structural parse failure, with a human-readable reason.
    Parse(String),
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetError::Empty => write!(f, "asset import: no geometry produced"),
            AssetError::UnknownFormat => write!(f, "asset import: unrecognised format"),
            AssetError::Parse(s) => write!(f, "asset import: parse error: {s}"),
        }
    }
}

impl std::error::Error for AssetError {}

/// Raw triangle-mesh geometry. The hot-path GPU buffers (Phase 1.2) consume `positions` +
/// `triangles`; the bounding box is precomputed for the projection / culling layer (§E §4).
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    /// Vertex positions in model space.
    pub positions: Vec<[f32; 3]>,
    /// Triangle vertex indices into `positions` (CCW winding as authored).
    pub triangles: Vec<[u32; 3]>,
    /// Axis-aligned bounding-box minimum corner.
    pub min: [f32; 3],
    /// Axis-aligned bounding-box maximum corner.
    pub max: [f32; 3],
}

impl Mesh {
    /// Number of vertices.
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Number of triangles.
    #[inline]
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Bounding-box centre (the simple centroid used by the projection/culling layer).
    #[inline]
    pub fn centroid(&self) -> [f32; 3] {
        [
            0.5 * (self.min[0] + self.max[0]),
            0.5 * (self.min[1] + self.max[1]),
            0.5 * (self.min[2] + self.max[2]),
        ]
    }

    /// Build a mesh from positions + triangles, computing the bounding box. Validates that every
    /// index is in range.
    fn build(positions: Vec<[f32; 3]>, triangles: Vec<[u32; 3]>) -> Result<Mesh, AssetError> {
        if positions.is_empty() || triangles.is_empty() {
            return Err(AssetError::Empty);
        }
        let n = positions.len() as u32;
        for t in &triangles {
            if t[0] >= n || t[1] >= n || t[2] >= n {
                return Err(AssetError::Parse(format!(
                    "triangle index out of range (verts={n}, tri={t:?})"
                )));
            }
        }
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for p in &positions {
            for k in 0..3 {
                min[k] = min[k].min(p[k]);
                max[k] = max[k].max(p[k]);
            }
        }
        Ok(Mesh {
            positions,
            triangles,
            min,
            max,
        })
    }
}

// ── Format detection + dispatch ───────────────────────────────────────────────────────────────

/// Import a mesh, sniffing the format from the bytes (or trusting an explicit lowercase
/// extension hint like `"obj"` / `"stl"`).
pub fn import_asset(bytes: &[u8], hint: Option<&str>) -> Result<Mesh, AssetError> {
    match hint {
        Some(h) if h.eq_ignore_ascii_case("obj") => return import_obj(bytes),
        Some(h) if h.eq_ignore_ascii_case("stl") => return import_stl(bytes),
        Some(h) if h.eq_ignore_ascii_case("glb") || h.eq_ignore_ascii_case("gltf") => {
            return import_glb(bytes)
        }
        _ => {}
    }
    if looks_like_glb(bytes) {
        import_glb(bytes) // unambiguous "glTF" magic — check first
    } else if looks_like_binary_stl(bytes) || looks_like_ascii_stl(bytes) {
        import_stl(bytes)
    } else if looks_like_obj(bytes) {
        import_obj(bytes)
    } else {
        Err(AssetError::UnknownFormat)
    }
}

fn looks_like_obj(bytes: &[u8]) -> bool {
    // An OBJ has `v ` (vertex) lines and usually `f ` (face) lines; comments start with `#`.
    let text = match core::str::from_utf8(bytes) {
        Ok(t) => t,
        Err(_) => return false,
    };
    text.lines().any(|l| {
        let l = l.trim_start();
        l.starts_with("v ") || l.starts_with("f ") || l.starts_with("vn ") || l.starts_with("vt ")
    })
}

fn looks_like_ascii_stl(bytes: &[u8]) -> bool {
    let prefix = &bytes[..bytes.len().min(512)];
    match core::str::from_utf8(prefix) {
        Ok(t) => {
            let t = t.trim_start();
            t.starts_with("solid") && t.contains("facet")
        }
        Err(_) => false,
    }
}

/// Binary STL has no magic; it is detected structurally: an 80-byte header, a `u32` triangle
/// count, then exactly `50 * count` bytes. (Some exporters start a *binary* file with "solid",
/// which is why the size check — not the prefix — is authoritative.)
fn looks_like_binary_stl(bytes: &[u8]) -> bool {
    if bytes.len() < 84 {
        return false;
    }
    let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
    bytes.len() == 84 + count * 50
}

// ── Wavefront OBJ ─────────────────────────────────────────────────────────────────────────────

/// Parse a Wavefront `.obj`: `v x y z` vertices and `f` faces (polygons fan-triangulated).
/// Face tokens may be `v`, `v/vt`, `v//vn`, or `v/vt/vn`; indices are 1-based and may be negative
/// (relative to the current vertex count). `vt` / `vn` / `vp` / groups are ignored for geometry.
pub fn import_obj(bytes: &[u8]) -> Result<Mesh, AssetError> {
    let text = core::str::from_utf8(bytes)
        .map_err(|_| AssetError::Parse("OBJ is not valid UTF-8".into()))?;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();

    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut tok = line.split_whitespace();
        match tok.next() {
            Some("v") => {
                let coords: Vec<f32> = tok.filter_map(|s| s.parse::<f32>().ok()).collect();
                if coords.len() < 3 {
                    return Err(AssetError::Parse(format!(
                        "OBJ line {}: vertex needs 3 coords",
                        lineno + 1
                    )));
                }
                positions.push([coords[0], coords[1], coords[2]]);
            }
            Some("f") => {
                // Resolve each face vertex token to a 0-based index, then fan-triangulate.
                let mut face: Vec<u32> = Vec::new();
                for t in tok {
                    let first = t.split('/').next().unwrap_or("");
                    let idx: i64 = match first.parse() {
                        Ok(i) => i,
                        Err(_) => {
                            return Err(AssetError::Parse(format!(
                                "OBJ line {}: bad face index {t:?}",
                                lineno + 1
                            )))
                        }
                    };
                    let zero_based = if idx > 0 {
                        (idx - 1) as i64
                    } else if idx < 0 {
                        positions.len() as i64 + idx
                    } else {
                        return Err(AssetError::Parse(format!(
                            "OBJ line {}: face index 0 is invalid",
                            lineno + 1
                        )));
                    };
                    if zero_based < 0 || zero_based as usize >= positions.len() {
                        return Err(AssetError::Parse(format!(
                            "OBJ line {}: face index {idx} out of range",
                            lineno + 1
                        )));
                    }
                    face.push(zero_based as u32);
                }
                for i in 1..face.len().saturating_sub(1) {
                    triangles.push([face[0], face[i], face[i + 1]]);
                }
            }
            _ => {} // vt / vn / vp / g / o / s / mtllib / usemtl — not needed for geometry
        }
    }

    Mesh::build(positions, triangles)
}

// ── STL (binary + ASCII) ──────────────────────────────────────────────────────────────────────

/// Parse a `.stl` (auto-detects binary vs ASCII). Each STL triangle contributes 3 fresh vertices
/// (no welding) — a faithful, lossless first cut; vertex de-duplication is a later optimisation.
pub fn import_stl(bytes: &[u8]) -> Result<Mesh, AssetError> {
    if looks_like_binary_stl(bytes) {
        import_stl_binary(bytes)
    } else {
        import_stl_ascii(bytes)
    }
}

fn import_stl_binary(bytes: &[u8]) -> Result<Mesh, AssetError> {
    if bytes.len() < 84 {
        return Err(AssetError::Parse("binary STL shorter than header".into()));
    }
    let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
    let expected = 84 + count * 50;
    if bytes.len() != expected {
        return Err(AssetError::Parse(format!(
            "binary STL size {} != expected {expected} for {count} triangles",
            bytes.len()
        )));
    }
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(count * 3);
    let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(count);
    let read_f32 = |o: usize| -> f32 {
        f32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]])
    };
    for t in 0..count {
        // 50 bytes/triangle: 12 (normal) + 3×12 (verts) + 2 (attr). Skip the normal.
        let base = 84 + t * 50 + 12;
        let v0 = positions.len() as u32;
        for v in 0..3 {
            let o = base + v * 12;
            positions.push([read_f32(o), read_f32(o + 4), read_f32(o + 8)]);
        }
        triangles.push([v0, v0 + 1, v0 + 2]);
    }
    Mesh::build(positions, triangles)
}

fn import_stl_ascii(bytes: &[u8]) -> Result<Mesh, AssetError> {
    let text = core::str::from_utf8(bytes)
        .map_err(|_| AssetError::Parse("ASCII STL is not valid UTF-8".into()))?;
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    let mut pending: Vec<[f32; 3]> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("vertex ") {
            let c: Vec<f32> = rest
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if c.len() < 3 {
                return Err(AssetError::Parse("ASCII STL: vertex needs 3 coords".into()));
            }
            pending.push([c[0], c[1], c[2]]);
            if pending.len() == 3 {
                let v0 = positions.len() as u32;
                positions.extend_from_slice(&pending);
                triangles.push([v0, v0 + 1, v0 + 2]);
                pending.clear();
            }
        }
    }
    Mesh::build(positions, triangles)
}

// ── glTF Binary (GLB) ─────────────────────────────────────────────────────────────────────────

const GLB_MAGIC: u32 = 0x4654_6C67; // "glTF" little-endian
const CHUNK_JSON: u32 = 0x4E4F_534A; // "JSON"
const CHUNK_BIN: u32 = 0x004E_4942; // "BIN\0"

fn looks_like_glb(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) == GLB_MAGIC
}

/// Parse a binary glTF (`.glb`): the 12-byte header, JSON chunk, and BIN chunk, then walk
/// `meshes[].primitives[]`, reading each `POSITION` accessor (FLOAT VEC3) and the optional index
/// accessor (u8/u16/u32 SCALAR) out of the BIN buffer. Triangle primitives only (mode 4); other
/// modes are skipped (a faithful first cut). Embedded/external-URI buffers are not handled here —
/// self-contained GLB binary only.
pub fn import_glb(bytes: &[u8]) -> Result<Mesh, AssetError> {
    if bytes.len() < 12 {
        return Err(AssetError::Parse("GLB shorter than 12-byte header".into()));
    }
    if u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) != GLB_MAGIC {
        return Err(AssetError::UnknownFormat);
    }
    // bytes[4..8] = version, bytes[8..12] = total length (not re-validated).

    let mut json: Option<&[u8]> = None;
    let mut bin: Option<&[u8]> = None;
    let mut off = 12usize;
    while off + 8 <= bytes.len() {
        let clen = u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
            as usize;
        let ctype = u32::from_le_bytes([
            bytes[off + 4],
            bytes[off + 5],
            bytes[off + 6],
            bytes[off + 7],
        ]);
        let dstart = off + 8;
        let dend = dstart
            .checked_add(clen)
            .ok_or_else(|| AssetError::Parse("GLB chunk length overflow".into()))?;
        if dend > bytes.len() {
            return Err(AssetError::Parse("GLB chunk exceeds file".into()));
        }
        match ctype {
            CHUNK_JSON => json = Some(&bytes[dstart..dend]),
            CHUNK_BIN => bin = Some(&bytes[dstart..dend]),
            _ => {}
        }
        off = dend;
    }

    let json = json.ok_or_else(|| AssetError::Parse("GLB has no JSON chunk".into()))?;
    let gltf: Value =
        serde_json::from_slice(json).map_err(|e| AssetError::Parse(format!("glTF JSON: {e}")))?;
    let bin = bin.unwrap_or(&[]);

    let empty: Vec<Value> = Vec::new();
    let accessors = gltf["accessors"].as_array().unwrap_or(&empty);
    let buffer_views = gltf["bufferViews"].as_array().unwrap_or(&empty);
    let meshes = gltf["meshes"].as_array().unwrap_or(&empty);

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();

    for mesh in meshes {
        for prim in mesh["primitives"].as_array().into_iter().flatten() {
            if prim["mode"].as_u64().unwrap_or(4) != 4 {
                continue; // not TRIANGLES
            }
            let pos_idx = prim["attributes"]["POSITION"]
                .as_u64()
                .ok_or_else(|| AssetError::Parse("primitive has no POSITION".into()))?
                as usize;
            let pos_acc = accessors
                .get(pos_idx)
                .ok_or_else(|| AssetError::Parse("POSITION accessor index out of range".into()))?;
            let base = positions.len() as u32;
            let prim_pos = read_positions(pos_acc, buffer_views, bin)?;
            let vcount = prim_pos.len() as u32;
            positions.extend_from_slice(&prim_pos);

            match prim["indices"].as_u64() {
                Some(idx_i) => {
                    let idx_acc = accessors
                        .get(idx_i as usize)
                        .ok_or_else(|| AssetError::Parse("index accessor out of range".into()))?;
                    let idx = read_indices(idx_acc, buffer_views, bin)?;
                    for c in idx.chunks_exact(3) {
                        triangles.push([base + c[0], base + c[1], base + c[2]]);
                    }
                }
                None => {
                    let mut i = 0u32;
                    while i + 3 <= vcount {
                        triangles.push([base + i, base + i + 1, base + i + 2]);
                        i += 3;
                    }
                }
            }
        }
    }

    Mesh::build(positions, triangles)
}

/// Read a FLOAT VEC3 accessor (e.g. `POSITION`) out of the GLB binary buffer.
fn read_positions(
    accessor: &Value,
    bvs: &[Value],
    bin: &[u8],
) -> Result<Vec<[f32; 3]>, AssetError> {
    if accessor["componentType"].as_u64() != Some(5126) {
        return Err(AssetError::Parse(
            "POSITION componentType must be FLOAT (5126)".into(),
        ));
    }
    if accessor["type"].as_str() != Some("VEC3") {
        return Err(AssetError::Parse(
            "POSITION accessor type must be VEC3".into(),
        ));
    }
    let count = accessor["count"]
        .as_u64()
        .ok_or_else(|| AssetError::Parse("accessor.count missing".into()))?
        as usize;
    let acc_off = accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let bv_idx = accessor["bufferView"]
        .as_u64()
        .ok_or_else(|| AssetError::Parse("accessor.bufferView missing".into()))?
        as usize;
    let bv = bvs
        .get(bv_idx)
        .ok_or_else(|| AssetError::Parse("bufferView index out of range".into()))?;
    let bv_off = bv["byteOffset"].as_u64().unwrap_or(0) as usize;
    let stride = match bv["byteStride"].as_u64().unwrap_or(0) {
        0 => 12, // tightly packed VEC3 f32
        s => s as usize,
    };
    let start = bv_off + acc_off;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = start + i * stride;
        if o + 12 > bin.len() {
            return Err(AssetError::Parse("POSITION read past end of BIN".into()));
        }
        let rd = |k: usize| {
            f32::from_le_bytes([bin[o + k], bin[o + k + 1], bin[o + k + 2], bin[o + k + 3]])
        };
        out.push([rd(0), rd(4), rd(8)]);
    }
    Ok(out)
}

/// Read a SCALAR index accessor (u8/u16/u32) out of the GLB binary buffer, widened to `u32`.
fn read_indices(accessor: &Value, bvs: &[Value], bin: &[u8]) -> Result<Vec<u32>, AssetError> {
    if accessor["type"].as_str() != Some("SCALAR") {
        return Err(AssetError::Parse(
            "index accessor type must be SCALAR".into(),
        ));
    }
    let comp = match accessor["componentType"].as_u64() {
        Some(5121) => 1usize,
        Some(5123) => 2,
        Some(5125) => 4,
        _ => {
            return Err(AssetError::Parse(
                "index componentType must be u8/u16/u32".into(),
            ))
        }
    };
    let count = accessor["count"]
        .as_u64()
        .ok_or_else(|| AssetError::Parse("accessor.count missing".into()))?
        as usize;
    let acc_off = accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let bv_idx = accessor["bufferView"]
        .as_u64()
        .ok_or_else(|| AssetError::Parse("accessor.bufferView missing".into()))?
        as usize;
    let bv = bvs
        .get(bv_idx)
        .ok_or_else(|| AssetError::Parse("bufferView index out of range".into()))?;
    let bv_off = bv["byteOffset"].as_u64().unwrap_or(0) as usize;
    let start = bv_off + acc_off;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = start + i * comp;
        if o + comp > bin.len() {
            return Err(AssetError::Parse("index read past end of BIN".into()));
        }
        let v = match comp {
            1 => bin[o] as u32,
            2 => u16::from_le_bytes([bin[o], bin[o + 1]]) as u32,
            _ => u32::from_le_bytes([bin[o], bin[o + 1], bin[o + 2], bin[o + 3]]),
        };
        out.push(v);
    }
    Ok(out)
}

// ── Semantic layer: Mesh → NQuins ─────────────────────────────────────────────────────────────

/// Emit the **semantic** quins for a mesh asset (the asset is *known*, not just drawn): its type,
/// vertex/triangle counts, bounding box, centroid, and source format — all in `GEOMETRY_CONTEXT`,
/// in the one identity space. Floats use the inline-float object tag (ADR 0008); counts are raw
/// integers. The returned lexicon maps the asset-URI / format hashes back to their strings.
pub fn mesh_to_nquins(
    mesh: &Mesh,
    asset_uri: &str,
    source_format: &str,
) -> (Vec<NQuin>, HashMap<u64, String>) {
    let subject = fnv_hash(asset_uri.as_bytes());
    let mut quins: Vec<NQuin> = Vec::with_capacity(13);
    let mut lexicon: HashMap<u64, String> = HashMap::new();
    lexicon.insert(subject, asset_uri.to_owned());

    quins.push(make_quin(subject, P_RDF_TYPE, C_MESH));
    quins.push(make_quin(
        subject,
        P_VERTEX_COUNT,
        mesh.vertex_count() as u64,
    ));
    quins.push(make_quin(
        subject,
        P_TRIANGLE_COUNT,
        mesh.triangle_count() as u64,
    ));

    let fmt_hash = fnv_hash(source_format.as_bytes());
    lexicon.insert(fmt_hash, source_format.to_owned());
    quins.push(make_quin(subject, P_SOURCE_FORMAT, fmt_hash));

    quins.push(make_quin(
        subject,
        P_BBOX_MIN_X,
        pack_float_object(mesh.min[0]),
    ));
    quins.push(make_quin(
        subject,
        P_BBOX_MIN_Y,
        pack_float_object(mesh.min[1]),
    ));
    quins.push(make_quin(
        subject,
        P_BBOX_MIN_Z,
        pack_float_object(mesh.min[2]),
    ));
    quins.push(make_quin(
        subject,
        P_BBOX_MAX_X,
        pack_float_object(mesh.max[0]),
    ));
    quins.push(make_quin(
        subject,
        P_BBOX_MAX_Y,
        pack_float_object(mesh.max[1]),
    ));
    quins.push(make_quin(
        subject,
        P_BBOX_MAX_Z,
        pack_float_object(mesh.max[2]),
    ));

    let c = mesh.centroid();
    quins.push(make_quin(subject, P_CENTROID_X, pack_float_object(c[0])));
    quins.push(make_quin(subject, P_CENTROID_Y, pack_float_object(c[1])));
    quins.push(make_quin(subject, P_CENTROID_Z, pack_float_object(c[2])));

    (quins, lexicon)
}

#[inline]
fn make_quin(subject: u64, predicate: u64, object: u64) -> NQuin {
    NQuin {
        subject,
        predicate,
        object,
        context: GEOMETRY_CONTEXT,
        metadata: 0,
        parity: 0,
    }
}

/// FNV-1a (60-bit), matching `crate::q_hash` for runtime strings so asset IRIs hashed here share
/// the one identity space (same convention as `kml_bridge`).
#[inline]
fn fnv_hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h & 0x0FFF_FFFF_FFFF_FFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRI_OBJ: &str = "# a single triangle\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";

    // A unit quad (two triangles via fan) + face tokens with v/vt/vn slashes.
    const QUAD_OBJ: &str = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1/1/1 2/2/1 3/3/1 4/4/1\n";

    #[test]
    fn obj_triangle() {
        let m = import_obj(TRI_OBJ.as_bytes()).unwrap();
        assert_eq!(m.vertex_count(), 3);
        assert_eq!(m.triangle_count(), 1);
        assert_eq!(m.min, [0.0, 0.0, 0.0]);
        assert_eq!(m.max, [1.0, 1.0, 0.0]);
        assert_eq!(m.triangles[0], [0, 1, 2]);
    }

    #[test]
    fn obj_quad_fan_triangulates_and_ignores_vt_vn() {
        let m = import_obj(QUAD_OBJ.as_bytes()).unwrap();
        assert_eq!(m.vertex_count(), 4);
        assert_eq!(m.triangle_count(), 2); // quad → 2 triangles
        assert_eq!(m.triangles[0], [0, 1, 2]);
        assert_eq!(m.triangles[1], [0, 2, 3]);
    }

    #[test]
    fn obj_negative_indices() {
        // -1/-2/-3 reference the three most-recent vertices.
        let obj = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\n";
        let m = import_obj(obj.as_bytes()).unwrap();
        assert_eq!(m.triangles[0], [0, 1, 2]);
    }

    #[test]
    fn obj_out_of_range_face_is_error() {
        let obj = "v 0 0 0\nv 1 0 0\nf 1 2 9\n";
        assert!(matches!(
            import_obj(obj.as_bytes()),
            Err(AssetError::Parse(_))
        ));
    }

    #[test]
    fn stl_ascii_triangle() {
        let stl = "solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid t\n";
        let m = import_stl(stl.as_bytes()).unwrap();
        assert_eq!(m.vertex_count(), 3);
        assert_eq!(m.triangle_count(), 1);
        assert_eq!(m.max, [1.0, 1.0, 0.0]);
    }

    #[test]
    fn stl_binary_triangle() {
        // 80-byte header + u32(1) + one 50-byte triangle record.
        let mut b = vec![0u8; 80];
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&[0u8; 12]); // normal
        for v in [[0f32, 0., 0.], [2., 0., 0.], [0., 3., 0.]] {
            for c in v {
                b.extend_from_slice(&c.to_le_bytes());
            }
        }
        b.extend_from_slice(&[0u8; 2]); // attribute byte count
        assert!(looks_like_binary_stl(&b));
        let m = import_stl(&b).unwrap();
        assert_eq!(m.vertex_count(), 3);
        assert_eq!(m.triangle_count(), 1);
        assert_eq!(m.max, [2.0, 3.0, 0.0]);
    }

    #[test]
    fn dispatch_sniffs_format() {
        assert_eq!(
            import_asset(TRI_OBJ.as_bytes(), None)
                .unwrap()
                .triangle_count(),
            1
        );
        assert_eq!(
            import_asset(TRI_OBJ.as_bytes(), Some("obj"))
                .unwrap()
                .vertex_count(),
            3
        );
    }

    #[test]
    fn mesh_to_nquins_emits_known_geometry() {
        let m = import_obj(TRI_OBJ.as_bytes()).unwrap();
        let (quins, lex) = mesh_to_nquins(&m, "urn:asset:tri", "obj");
        // type + 2 counts + format + 6 bbox + 3 centroid = 13.
        assert_eq!(quins.len(), 13);
        let subject = fnv_hash(b"urn:asset:tri");
        assert_eq!(lex.get(&subject).unwrap(), "urn:asset:tri");
        // The type quin is present.
        assert!(quins
            .iter()
            .any(|q| q.predicate == P_RDF_TYPE && q.object == C_MESH));
        // bboxMaxX round-trips through the inline-float tag.
        let max_x = quins.iter().find(|q| q.predicate == P_BBOX_MAX_X).unwrap();
        assert_eq!(unpack_float_object(max_x.object), 1.0);
    }

    fn build_test_glb() -> Vec<u8> {
        // BIN: 3 positions (VEC3 f32, 36 B) then 3 indices (u16, 6 B), padded to 4 -> 44 B.
        let mut bin = Vec::new();
        for v in [[0f32, 0., 0.], [2., 0., 0.], [0., 4., 0.]] {
            for c in v {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        for i in [0u16, 1, 2] {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let json = r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":44}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":6}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"},{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}],"meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1}]}]}"#;
        let mut jb = json.as_bytes().to_vec();
        while jb.len() % 4 != 0 {
            jb.push(b' ');
        }
        let total = 12 + 8 + jb.len() + 8 + bin.len();
        let mut glb = Vec::new();
        glb.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total as u32).to_le_bytes());
        glb.extend_from_slice(&(jb.len() as u32).to_le_bytes());
        glb.extend_from_slice(&CHUNK_JSON.to_le_bytes());
        glb.extend_from_slice(&jb);
        glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        glb.extend_from_slice(&CHUNK_BIN.to_le_bytes());
        glb.extend_from_slice(&bin);
        glb
    }

    #[test]
    fn glb_single_triangle() {
        let glb = build_test_glb();
        assert!(looks_like_glb(&glb));
        let m = import_glb(&glb).unwrap();
        assert_eq!(m.vertex_count(), 3);
        assert_eq!(m.triangle_count(), 1);
        assert_eq!(m.max, [2.0, 4.0, 0.0]);
        assert_eq!(m.triangles[0], [0, 1, 2]);
        // dispatch via the "glTF" magic
        assert_eq!(import_asset(&glb, None).unwrap().triangle_count(), 1);
        assert_eq!(import_asset(&glb, Some("glb")).unwrap().vertex_count(), 3);
    }

    #[test]
    fn empty_obj_is_error() {
        assert_eq!(import_obj(b"# nothing here\n"), Err(AssetError::Empty));
    }
}
