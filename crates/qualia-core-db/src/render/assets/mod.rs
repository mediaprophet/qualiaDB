//! Asset-import bridge — `OBJ` / `STL` mesh → `NQuin` stream (Phase 1.3,
//! `RENDERER_IMPLEMENTATION_PLAN.md`).
//!
//! Pure `&[u8]` → (geometry, semantic NQuins). **No `std::fs`** — wasm-safe (migration review
//! §2.1): the shell / CLI reads the file (or runs the OS picker) and hands the bytes down. This is
//! the ingest path (not a hot path), so `Vec` / `HashMap` are fine — the same convention as
//! `kml_bridge`.
//!
//! Two layers, per STELLAR §E ("artefacts carry their geometry, and are *semantically known*"):
//!   * `Mesh` — raw geometry (vertex positions + triangle indices + bounding box): the data the
//!     GPU vertex/index buffers (Phase 1.2) will consume.
//!   * `mesh_to_nquins` — the **semantic** layer: the asset is *known* (type, counts, bounding
//!     box, centroid, source format) as NQuins in the one identity space — not just points/pixels.
//!
//! Hot-path rendering (depth-stencil, mesh buffers, projection) is the GPU half of Phase 1 and is
//! verified on hardware; this module is the CPU half and is unit-tested here.

use std::collections::HashMap;

use crate::container_10d::{MaterialRecord, SubmeshRange};
use crate::frame_layout::pack_float_object;
use crate::{q_hash, NQuin};

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
const P_SOURCE_DIGEST: u64 = q_hash("urn:qualia:geometry:sourceDigest");
const P_COMPILED_DIGEST: u64 = q_hash("urn:qualia:geometry:compiledDigest");
const P_BODY_SYSTEM: u64 = q_hash("urn:qualia:geometry:bodySystem");
const P_ANATOMY_MODEL: u64 = q_hash("urn:qualia:geometry:anatomyModel");
const P_GESTATIONAL_AGE_DAYS: u64 = q_hash("urn:qualia:geometry:gestationalAgeDays");
const P_CARNEGIE_STAGE: u64 = q_hash("urn:qualia:geometry:carnegieStage");

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

/// Mesh plus optional normal and tangent streams. Tangents may be authored or generated from
/// TEXCOORD_0; importers without the required data leave a stream empty for renderer fallback.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedMesh {
    pub mesh: Mesh,
    pub normals: Option<Vec<[f32; 3]>>,
    /// Authored glTF tangent direction and handedness, or MikkTSpace-generated frames when UV0 is
    /// available. A missing UV0 leaves the stream absent for the renderer's capability fallback.
    pub tangents: Option<Vec<[f32; 4]>>,
    /// Vertex-aligned glTF TEXCOORD_0 pairs, including any vertices split to preserve tangent seams.
    pub texture_coordinates_0: Option<Vec<[f32; 2]>>,
    /// Imported glTF materials. Their IDs are source-local (`index + 1`) until the asset
    /// compiler scopes them to a stable asset identity.
    pub materials: Vec<MaterialRecord>,
    /// Content-addressed image payloads referenced by imported material records. Payload bytes
    /// remain intact for HMC/resource packaging; they are not copied into the 10D semantic arena.
    pub texture_dependencies: Vec<TextureDependency>,
    /// Contiguous index ranges in the same order as the flattened primitive index stream.
    pub submeshes: Vec<SubmeshRange>,
}

/// Immutable image asset discovered while importing an embedded glTF image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureDependency {
    /// SHA-256 of `bytes`, matching the corresponding MAT2/MAT1 resource references.
    pub digest: [u8; 32],
    /// Source MIME type (for example `image/png`, `image/jpeg`, or `image/ktx2`).
    pub mime_type: String,
    /// Original encoded image bytes, retained for external HMC/Webizen processing.
    pub bytes: Vec<u8>,
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
    import_asset_with_normals(bytes, hint).map(|imported| imported.mesh)
}

/// Import a mesh and preserve authored normals when the source format provides them. Missing
/// normals in individual primitives are generated from the primitive's triangle geometry.
pub fn import_asset_with_normals(
    bytes: &[u8],
    hint: Option<&str>,
) -> Result<ImportedMesh, AssetError> {
    match hint {
        Some(h) if h.eq_ignore_ascii_case("obj") => {
            return import_obj(bytes).map(|mesh| ImportedMesh {
                mesh,
                normals: None,
                tangents: None,
                texture_coordinates_0: None,
                materials: Vec::new(),
                texture_dependencies: Vec::new(),
                submeshes: Vec::new(),
            });
        }
        Some(h) if h.eq_ignore_ascii_case("stl") => {
            return import_stl(bytes).map(|mesh| ImportedMesh {
                mesh,
                normals: None,
                tangents: None,
                texture_coordinates_0: None,
                materials: Vec::new(),
                texture_dependencies: Vec::new(),
                submeshes: Vec::new(),
            });
        }
        Some(h) if h.eq_ignore_ascii_case("glb") || h.eq_ignore_ascii_case("gltf") => {
            return import_glb_with_normals(bytes);
        }
        _ => {}
    }
    if looks_like_glb(bytes) {
        import_glb_with_normals(bytes) // unambiguous "glTF" magic — check first
    } else if looks_like_binary_stl(bytes) || looks_like_ascii_stl(bytes) {
        import_stl(bytes).map(|mesh| ImportedMesh {
            mesh,
            normals: None,
            tangents: None,
            texture_coordinates_0: None,
            materials: Vec::new(),
            texture_dependencies: Vec::new(),
            submeshes: Vec::new(),
        })
    } else if looks_like_obj(bytes) {
        import_obj(bytes).map(|mesh| ImportedMesh {
            mesh,
            normals: None,
            tangents: None,
            texture_coordinates_0: None,
            materials: Vec::new(),
            texture_dependencies: Vec::new(),
            submeshes: Vec::new(),
        })
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
                            )));
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

const GLB_MAGIC: u32 = 0x4654_6C67; // "glTF" little-endian
const CHUNK_JSON: u32 = 0x4E4F_534A; // "JSON"
const CHUNK_BIN: u32 = 0x004E_4942; // "BIN\\0"

#[path = "glb.rs"]
mod glb;
#[path = "glb_materials.rs"]
mod glb_materials;
#[path = "glb_texture_transform.rs"]
mod glb_texture_transform;
#[path = "tangents.rs"]
mod tangents;
use glb::looks_like_glb;
pub use glb::{import_glb, import_glb_with_normals};
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

/// Like `mesh_to_nquins` but also asserts the immutable `sourceDigest` and the `.10d`
/// `compiledDigest` — the manifest→container join (geometry-asset-ontology §4). Both are CRC-32C
/// `u32` content hashes stored as `u64` objects: `sourceDigest` over the immutable source asset
/// bytes, `compiledDigest` the whole-file CRC of the `.10d` container this manifest describes.
pub fn mesh_to_nquins_with_digests(
    mesh: &Mesh,
    asset_uri: &str,
    source_format: &str,
    source_digest: u32,
    compiled_digest: u32,
) -> (Vec<NQuin>, HashMap<u64, String>) {
    let (mut quins, lexicon) = mesh_to_nquins(mesh, asset_uri, source_format);
    let subject = fnv_hash(asset_uri.as_bytes());
    quins.push(make_quin(subject, P_SOURCE_DIGEST, source_digest as u64));
    quins.push(make_quin(
        subject,
        P_COMPILED_DIGEST,
        compiled_digest as u64,
    ));
    (quins, lexicon)
}

/// Like [`mesh_to_nquins_with_digests`] but also asserts the anatomy binding for a 3D-body organ:
/// `bodySystem` (which of the 17 body systems this organ belongs to — which system's burden colours
/// it) and `anatomyModel` (`"male"` / `"female"` — the reference model whose set it is part of, chosen
/// from the user's declared XY/XX chromosomal basis). Both are string facts carried in the lexicon,
/// like `sourceFormat`; a `None` field is simply not asserted.
#[allow(clippy::too_many_arguments)]
pub fn mesh_to_nquins_with_meta(
    mesh: &Mesh,
    asset_uri: &str,
    source_format: &str,
    source_digest: u32,
    compiled_digest: u32,
    body_system: Option<&str>,
    anatomy_model: Option<&str>,
) -> (Vec<NQuin>, HashMap<u64, String>) {
    let (mut quins, mut lexicon) = mesh_to_nquins_with_digests(
        mesh,
        asset_uri,
        source_format,
        source_digest,
        compiled_digest,
    );
    let subject = fnv_hash(asset_uri.as_bytes());
    if let Some(sys) = body_system {
        let h = fnv_hash(sys.as_bytes());
        lexicon.insert(h, sys.to_owned());
        quins.push(make_quin(subject, P_BODY_SYSTEM, h));
    }
    if let Some(model) = anatomy_model {
        let h = fnv_hash(model.as_bytes());
        lexicon.insert(h, model.to_owned());
        quins.push(make_quin(subject, P_ANATOMY_MODEL, h));
    }
    (quins, lexicon)
}

/// Like [`mesh_to_nquins_with_digests`] but for a **developmental** body: also asserts the gestational
/// `t`-axis coordinate — `gestationalAgeDays` (postfertilization) and `carnegieStage`. This is what makes
/// a fetal `.10d` a *slice* of a 4-D developmental body: consecutive stages, ordered by gestational age,
/// are the same body along the `t`-axis (reproductive-continuum plan §2). Both are plain `u64` objects.
pub fn mesh_to_nquins_with_dev(
    mesh: &Mesh,
    asset_uri: &str,
    source_format: &str,
    source_digest: u32,
    compiled_digest: u32,
    gestational_age_days: u16,
    carnegie_stage: u8,
) -> (Vec<NQuin>, HashMap<u64, String>) {
    let (mut quins, lexicon) = mesh_to_nquins_with_digests(
        mesh,
        asset_uri,
        source_format,
        source_digest,
        compiled_digest,
    );
    let subject = fnv_hash(asset_uri.as_bytes());
    quins.push(make_quin(
        subject,
        P_GESTATIONAL_AGE_DAYS,
        gestational_age_days as u64,
    ));
    quins.push(make_quin(subject, P_CARNEGIE_STAGE, carnegie_stage as u64));
    (quins, lexicon)
}

#[inline]
fn make_quin(subject: u64, predicate: u64, object: u64) -> NQuin {
    let context = GEOMETRY_CONTEXT;
    let metadata = 0;
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        // Was hardcoded `0` — emitted invalid-parity geometry NQuins. Use the canonical parity so
        // the "every runtime NQuin has valid field parity" invariant holds (visual-plan §3.1 fix).
        parity: NQuin::calculate_parity(subject, predicate, object, context, metadata),
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
    use super::{glb::read_texcoords_0, tangents::generate_mikktspace_tangents};
    use crate::frame_layout::unpack_float_object;

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
        // Every emitted geometry NQuin has valid (non-zero, canonical) parity — regression guard for
        // the former hardcoded `parity: 0`.
        for q in &quins {
            assert_eq!(
                q.parity,
                NQuin::calculate_parity(q.subject, q.predicate, q.object, q.context, q.metadata),
                "geometry NQuin must carry canonical parity"
            );
        }
    }

    fn build_test_glb() -> Vec<u8> {
        // BIN: positions (36 B), normals (36 B), tangents (48 B), indices (6 B), padded -> 128 B.
        let mut bin = Vec::new();
        for v in [[0f32, 0., 0.], [2., 0., 0.], [0., 4., 0.]] {
            for c in v {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        for _ in 0..3 {
            for c in [0f32, 0., 2.] {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        for _ in 0..3 {
            for c in [2f32, 0., 0.5, 1.] {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        for i in [0u16, 1, 2] {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let json = r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":128}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":36},{"buffer":0,"byteOffset":72,"byteLength":48},{"buffer":0,"byteOffset":120,"byteLength":6}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"},{"bufferView":1,"componentType":5126,"count":3,"type":"VEC3"},{"bufferView":2,"componentType":5126,"count":3,"type":"VEC4"},{"bufferView":3,"componentType":5123,"count":3,"type":"SCALAR"}],"meshes":[{"primitives":[{"attributes":{"POSITION":0,"NORMAL":1,"TANGENT":2},"indices":3}]}]}"#;
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

    fn build_test_glb_without_tangents_with_uv0() -> Vec<u8> {
        let mut bin = Vec::new();
        for position in [[0.0f32, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 4.0, 0.0]] {
            for component in position {
                bin.extend_from_slice(&component.to_le_bytes());
            }
        }
        for _ in 0..3 {
            for component in [0.0f32, 0.0, 1.0] {
                bin.extend_from_slice(&component.to_le_bytes());
            }
        }
        for uv in [[0.0f32, 0.0], [1.0, 0.0], [0.0, 1.0]] {
            for component in uv {
                bin.extend_from_slice(&component.to_le_bytes());
            }
        }
        for index in [0u16, 1, 2] {
            bin.extend_from_slice(&index.to_le_bytes());
        }
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let json = r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":104}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":36},{"buffer":0,"byteOffset":72,"byteLength":24},{"buffer":0,"byteOffset":96,"byteLength":6}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"},{"bufferView":1,"componentType":5126,"count":3,"type":"VEC3"},{"bufferView":2,"componentType":5126,"count":3,"type":"VEC2"},{"bufferView":3,"componentType":5123,"count":3,"type":"SCALAR"}],"meshes":[{"primitives":[{"attributes":{"POSITION":0,"NORMAL":1,"TEXCOORD_0":2},"indices":3}]}]}"#;
        let mut json_bytes = json.as_bytes().to_vec();
        while json_bytes.len() % 4 != 0 {
            json_bytes.push(b' ');
        }
        let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
        let mut glb = Vec::new();
        glb.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total as u32).to_le_bytes());
        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(&CHUNK_JSON.to_le_bytes());
        glb.extend_from_slice(&json_bytes);
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
        let imported = import_glb_with_normals(&glb).unwrap();
        assert_eq!(imported.normals.unwrap(), vec![[0.0, 0.0, 1.0]; 3]);
        let imported = import_glb_with_normals(&glb).unwrap();
        assert_eq!(imported.tangents.unwrap(), vec![[1.0, 0.0, 0.0, 1.0]; 3]);
        // dispatch via the "glTF" magic
        assert_eq!(import_asset(&glb, None).unwrap().triangle_count(), 1);
        assert_eq!(import_asset(&glb, Some("glb")).unwrap().vertex_count(), 3);
        let compiled = crate::render::compile_10d::compile_asset(
            &glb,
            Some("glb"),
            "urn:test:authored-normal-triangle",
            "glb",
        )
        .unwrap();
        let compiled_normals = crate::render::compile_10d::decode_10d_normals(
            &compiled.container_10d,
            compiled.mesh.vertex_count(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(compiled_normals, vec![[0.0, 0.0, 1.0]; 3]);
        let compiled_tangents = crate::render::compile_10d::decode_10d_tangents(
            &compiled.container_10d,
            compiled.mesh.vertex_count(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(compiled_tangents, vec![[1.0, 0.0, 0.0, 1.0]; 3]);
    }

    #[test]
    fn glb_rejects_zero_length_authored_normal() {
        let mut glb = build_test_glb();
        // The second view is the authored NORMAL stream, beginning at byte 36 in BIN.
        let bin_chunk = glb
            .windows(4)
            .position(|w| w == CHUNK_BIN.to_le_bytes())
            .unwrap()
            + 4;
        for byte in &mut glb[bin_chunk + 36..bin_chunk + 72] {
            *byte = 0;
        }
        assert!(matches!(
            import_glb_with_normals(&glb),
            Err(AssetError::Parse(_))
        ));
    }

    #[test]
    fn glb_rejects_invalid_tangent_handedness() {
        let mut glb = build_test_glb();
        // The third view is TANGENT VEC4 at byte 72; w is the fourth f32.
        let bin_chunk = glb
            .windows(4)
            .position(|w| w == CHUNK_BIN.to_le_bytes())
            .unwrap()
            + 4;
        glb[bin_chunk + 84..bin_chunk + 88].copy_from_slice(&0.0f32.to_le_bytes());
        assert!(matches!(
            import_glb_with_normals(&glb),
            Err(AssetError::Parse(_))
        ));
    }

    #[test]
    fn mikktspace_generation_tracks_uv_orientation_and_corner_splits() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = [[0.0, 0.0, 1.0]; 3];
        let indices = [0, 1, 2];
        let standard = generate_mikktspace_tangents(
            &positions,
            &normals,
            &[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            &indices,
        )
        .unwrap();
        assert_eq!(standard.source_vertices, vec![0, 1, 2]);
        assert_eq!(standard.indices, vec![0, 1, 2]);
        for tangent in standard.tangents {
            assert!((tangent[0] - 1.0).abs() < 1e-5);
            assert!(tangent[1].abs() < 1e-5 && tangent[2].abs() < 1e-5);
            assert_eq!(tangent[3], 1.0);
        }

        let mirrored = generate_mikktspace_tangents(
            &positions,
            &normals,
            &[[0.0, 0.0], [1.0, 0.0], [0.0, -1.0]],
            &indices,
        )
        .unwrap();
        assert!(mirrored.tangents.iter().all(|tangent| tangent[3] == -1.0));

        let quad_positions = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let quad_normals = [[0.0, 0.0, 1.0]; 4];
        let quad_uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 0.0]];
        let split = generate_mikktspace_tangents(
            &quad_positions,
            &quad_normals,
            &quad_uv,
            &[0, 1, 2, 0, 2, 3],
        )
        .unwrap();
        assert_eq!(split.indices.len(), 6);
        assert!(split.source_vertices.len() > quad_positions.len());
        let signs_for_shared_vertex: std::collections::HashSet<_> = split
            .source_vertices
            .iter()
            .enumerate()
            .filter(|(_, source)| **source == 0)
            .map(|(output, _)| split.tangents[output][3].is_sign_negative())
            .collect();
        assert_eq!(signs_for_shared_vertex.len(), 2);
    }

    #[test]
    fn mikktspace_degenerate_uvs_produce_finite_fallback_frames() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = [[0.0, 0.0, 1.0]; 3];
        let generated =
            generate_mikktspace_tangents(&positions, &normals, &[[0.0, 0.0]; 3], &[0, 1, 2])
                .unwrap();

        assert_eq!(generated.indices, vec![0, 1, 2]);
        assert!(generated.tangents.iter().all(|tangent| {
            tangent.iter().all(|component| component.is_finite())
                && (tangent[3].abs() - 1.0).abs() < f32::EPSILON
        }));
    }

    #[test]
    fn glb_uv_tangents_survive_asset_compilation_and_10d_decode() {
        let glb = build_test_glb_without_tangents_with_uv0();
        let imported = import_glb_with_normals(&glb).unwrap();
        assert_eq!(imported.tangents.unwrap(), vec![[1.0, 0.0, 0.0, 1.0]; 3]);

        let compiled = crate::render::compile_10d::compile_asset(
            &glb,
            Some("glb"),
            "urn:test:uv-tangent-triangle",
            "glb",
        )
        .unwrap();
        let decoded = crate::render::compile_10d::decode_10d_tangents(
            &compiled.container_10d,
            compiled.mesh.vertex_count(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(decoded, vec![[1.0, 0.0, 0.0, 1.0]; 3]);
    }

    #[test]
    fn texcoord_reader_decodes_normalized_unsigned_bytes_and_checks_view_bounds() {
        let accessor = serde_json::json!({
            "componentType": 5121,
            "normalized": true,
            "count": 2,
            "type": "VEC2",
            "bufferView": 0
        });
        let view = serde_json::json!({ "byteOffset": 0, "byteLength": 4 });
        assert_eq!(
            read_texcoords_0(&accessor, &[view.clone()], &[0, 255, 127, 64]).unwrap(),
            vec![[0.0, 1.0], [127.0 / 255.0, 64.0 / 255.0]]
        );

        let short_view = serde_json::json!({ "byteOffset": 0, "byteLength": 3 });
        assert!(matches!(
            read_texcoords_0(&accessor, &[short_view], &[0, 255, 127, 64]),
            Err(AssetError::Parse(_))
        ));
    }

    #[test]
    fn empty_obj_is_error() {
        assert_eq!(import_obj(b"# nothing here\n"), Err(AssetError::Empty));
    }
}
