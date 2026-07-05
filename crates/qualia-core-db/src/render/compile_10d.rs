//! Compile an imported triangle [`Mesh`] into a sealed `.10d` container — the
//! **dense compiled-geometry** half of a geometry asset (see
//! `docs/manuals/standards/geometry-asset-ontology.md` §3). This is the
//! "mesh → `.10d`" step of the 3-D-anatomy asset pipeline: the renderer and the
//! anatomy layer read the `.10d` back with [`decode_10d_mesh`] instead of
//! reparsing the source GLB, and the q42 semantic manifest cites the container's
//! [`compiled_digest`].
//!
//! **v0 scope (honest):** emits a single `QuantizedMesh` section (u16-quantized
//! vertices in the bbox + u16/u32 indices — 2× smaller than raw f32, visually
//! lossless at organ scale). Topology + spatial-index sections (for scan-free
//! picking) and the LOD chain (from `decimate_3`, P5.7) are named as the next
//! slices — deliberately not stubbed with placeholders here.

use crate::container_10d::header::Container10dHeader;
use crate::container_10d::integrity::{compute_whole_file_crc32c, seal_whole_file_crc32c};
use crate::container_10d::mesh_section::{
    decode_mesh_section, encode_mesh_section, encoded_len, MeshSectionError,
};
use crate::container_10d::crc32c::crc32c;
use crate::container_10d::section::{
    encode_container, parse_section_table, AlignmentTier, SectionInput, SectionTableError,
    SectionType,
};
use crate::render::assets::{import_asset, mesh_to_nquins_with_meta, AssetError, Mesh};
use crate::NQuin;
use std::collections::HashMap;

/// Failure modes for `.10d` compilation and read-back.
///
/// Not `Clone` — it wraps [`AssetError`], which is not `Clone`. Errors are consumed
/// on the failure path, not duplicated.
#[derive(Debug, PartialEq, Eq)]
pub enum Compile10dError {
    /// Decoding the source asset bytes (OBJ/STL/GLB) failed.
    Import(AssetError),
    /// The QuantizedMesh section encode/decode failed.
    Mesh(MeshSectionError),
    /// The container section table encode/decode failed.
    Section(SectionTableError),
    /// The container parsed but held no `QuantizedMesh` section.
    NoMeshSection,
    /// The 64-byte container header failed to parse on read-back.
    BadHeader,
    /// A section descriptor's byte range fell outside the container bytes.
    SectionOutOfBounds,
}

impl std::fmt::Display for Compile10dError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Import(e) => write!(f, ".10d compile: source import: {e}"),
            Self::Mesh(e) => write!(f, ".10d compile: mesh section: {e}"),
            Self::Section(e) => write!(f, ".10d compile: section table: {e:?}"),
            Self::NoMeshSection => write!(f, ".10d: no QuantizedMesh section in container"),
            Self::BadHeader => write!(f, ".10d: container header failed to parse"),
            Self::SectionOutOfBounds => write!(f, ".10d: section byte range outside container"),
        }
    }
}

impl std::error::Error for Compile10dError {}

/// Compile a mesh into a **sealed** `.10d` container holding one `QuantizedMesh`
/// section. The whole-file CRC-32C is written (the container is self-verifying).
///
/// Cold path (one-shot asset compilation) — allocates the output `Vec`; the hot
/// render/query paths operate zero-heap on the decoded section, not here.
/// Deterministic: identical input → byte-identical container (attestable).
pub fn compile_mesh_to_10d(mesh: &Mesh) -> Result<Vec<u8>, Compile10dError> {
    // 1. Encode the QuantizedMesh section payload.
    let mut payload = vec![0u8; encoded_len(mesh.vertex_count(), mesh.triangle_count())];
    let written = encode_mesh_section(mesh, &mut payload).map_err(Compile10dError::Mesh)?;
    payload.truncate(written);

    // 2. Assemble the container. Page-aligned so the mesh payload is GPU-stageable.
    let header = Container10dHeader::proposed();
    let inputs = [SectionInput {
        section_type: SectionType::QuantizedMesh,
        alignment_tier: AlignmentTier::Page,
        stride: 0,
        element_count: 0,
        payload: &payload,
    }];
    // Dry-run against an empty buffer to size the output exactly.
    let needed = match encode_container(&header, &inputs, &mut []) {
        Err(SectionTableError::OutputBufferTooSmall { needed, .. }) => needed,
        Ok(n) => n,
        Err(e) => return Err(Compile10dError::Section(e)),
    };
    let mut out = vec![0u8; needed];
    let total = encode_container(&header, &inputs, &mut out).map_err(Compile10dError::Section)?;
    out.truncate(total);

    // 3. Seal the whole-file CRC-32C (the `compiledDigest` source).
    seal_whole_file_crc32c(&mut out);
    Ok(out)
}

/// The `compiledDigest` a q42 asset manifest cites: the container's whole-file
/// CRC-32C (computed with the header's own CRC field zeroed, so it equals the
/// sealed header value). Deterministic; changes on any geometry-byte change.
#[inline]
pub fn compiled_digest(container_10d: &[u8]) -> u32 {
    compute_whole_file_crc32c(container_10d)
}

/// Read a `.10d` container back into a dequantized [`Mesh`] — the renderer /
/// anatomy path that avoids reparsing the source GLB. Extracts the first
/// `QuantizedMesh` section.
pub fn decode_10d_mesh(container_10d: &[u8]) -> Result<Mesh, Compile10dError> {
    let header = Container10dHeader::parse(container_10d).map_err(|_| Compile10dError::BadHeader)?;
    let descs = parse_section_table(container_10d, &header).map_err(Compile10dError::Section)?;
    for d in descs {
        if d.typ() == Some(SectionType::QuantizedMesh) {
            let start = d.byte_offset as usize;
            let end = start
                .checked_add(d.byte_length as usize)
                .ok_or(Compile10dError::SectionOutOfBounds)?;
            let payload = container_10d
                .get(start..end)
                .ok_or(Compile10dError::SectionOutOfBounds)?;
            return decode_mesh_section(payload).map_err(Compile10dError::Mesh);
        }
    }
    Err(Compile10dError::NoMeshSection)
}

/// A fully compiled geometry asset: the source-imported [`Mesh`], the sealed `.10d`
/// container, its two content digests, and the q42 semantic manifest that cites the
/// container by `compiledDigest` (geometry-asset-ontology §1 two-layer model).
///
/// This is the whole "GLB → `.10d` + manifest" pipeline output. The `container_10d`
/// bytes are the on-disk sidecar; `quins`/`lexicon` are the portable q42 facts that
/// travel in the graph and point at the container by hash.
#[derive(Debug, Clone)]
pub struct CompiledAsset {
    /// The imported triangle mesh (positions + indices + bbox).
    pub mesh: Mesh,
    /// The sealed `.10d` container (dense compiled geometry).
    pub container_10d: Vec<u8>,
    /// Whole-file CRC-32C of `container_10d` — the manifest's `compiledDigest`.
    pub compiled_digest: u32,
    /// CRC-32C of the immutable source bytes — the manifest's `sourceDigest`.
    pub source_digest: u32,
    /// The q42 manifest facts, including both digests (via [`mesh_to_nquins_with_digests`]).
    pub quins: Vec<NQuin>,
    /// Object-lexicon for the string-valued facts in `quins`.
    pub lexicon: HashMap<u64, String>,
}

/// The end-to-end asset-compile step: source asset bytes → [`CompiledAsset`].
///
/// Runs the full pipeline — import the source ([`import_asset`]), compile the dense
/// `.10d` ([`compile_mesh_to_10d`]), hash both layers, then emit the q42 manifest that
/// binds them ([`mesh_to_nquins_with_digests`]). Deterministic: identical
/// `(source_bytes, asset_uri, source_format)` → byte-identical container and identical
/// digests, so the manifest→container citation is attestable.
///
/// `hint` is the source-format hint forwarded to `import_asset` (e.g. `Some("glb")`);
/// `source_format` is the value recorded in the manifest and should agree with the real
/// format of `source_bytes`.
pub fn compile_asset(
    source_bytes: &[u8],
    hint: Option<&str>,
    asset_uri: &str,
    source_format: &str,
) -> Result<CompiledAsset, Compile10dError> {
    compile_organ_asset(source_bytes, hint, asset_uri, source_format, None, None)
}

/// Like [`compile_asset`] but also binds the compiled asset to a 3D-body organ: its `body_system`
/// (which of the 17 systems colours it by burden) and `anatomy_model` (`"male"` / `"female"`, from the
/// user's declared XY/XX basis). This is the compile step for an anatomy organ mesh — the manifest it
/// emits carries `geo:bodySystem` + `geo:anatomyModel` so the renderer can look the organ up in the
/// per-system percept table (S5.1) and both colour and sonify it.
#[allow(clippy::too_many_arguments)]
pub fn compile_organ_asset(
    source_bytes: &[u8],
    hint: Option<&str>,
    asset_uri: &str,
    source_format: &str,
    body_system: Option<&str>,
    anatomy_model: Option<&str>,
) -> Result<CompiledAsset, Compile10dError> {
    let mesh = import_asset(source_bytes, hint).map_err(Compile10dError::Import)?;
    let container_10d = compile_mesh_to_10d(&mesh)?;
    let compiled = compiled_digest(&container_10d);
    let source = crc32c(source_bytes);
    let (quins, lexicon) = mesh_to_nquins_with_meta(
        &mesh,
        asset_uri,
        source_format,
        source,
        compiled,
        body_system,
        anatomy_model,
    );
    Ok(CompiledAsset {
        mesh,
        container_10d,
        compiled_digest: compiled,
        source_digest: source,
        quins,
        lexicon,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::integrity::verify_whole_file_crc32c;

    /// Unit cube, 8 vertices / 12 triangles — a valid `render::Mesh`.
    fn cube() -> Mesh {
        Mesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
                [0.0, 1.0, 1.0],
            ],
            triangles: vec![
                [0, 3, 2], [0, 2, 1],
                [4, 5, 6], [4, 6, 7],
                [0, 1, 5], [0, 5, 4],
                [3, 7, 6], [3, 6, 2],
                [0, 4, 7], [0, 7, 3],
                [1, 2, 6], [1, 6, 5],
            ],
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
        }
    }

    #[test]
    fn container_is_well_formed_and_sealed() {
        let mut bytes = compile_mesh_to_10d(&cube()).unwrap();
        assert!(bytes.len() > 64, "container must be larger than the 64-byte header");
        // The seal verifies (whole-file CRC matches the header) — proves it's well-formed.
        verify_whole_file_crc32c(&mut bytes).expect(".10d whole-file CRC must verify");
    }

    #[test]
    fn round_trips_the_mesh_within_quantization_tolerance() {
        let m = cube();
        let bytes = compile_mesh_to_10d(&m).unwrap();
        let back = decode_10d_mesh(&bytes).unwrap();
        assert_eq!(back.vertex_count(), m.vertex_count());
        assert_eq!(back.triangle_count(), m.triangle_count());
        assert_eq!(back.triangles, m.triangles, "indices are exact");
        // Positions survive within the u16-in-bbox quantization bound (extent/65535).
        let tol = 1.0 / 65535.0 * 1.001;
        for (a, b) in m.positions.iter().zip(back.positions.iter()) {
            for k in 0..3 {
                assert!((a[k] - b[k]).abs() <= tol, "vertex {a:?} vs {b:?} axis {k}");
            }
        }
    }

    #[test]
    fn compilation_is_deterministic() {
        let m = cube();
        let a = compile_mesh_to_10d(&m).unwrap();
        let b = compile_mesh_to_10d(&m).unwrap();
        assert_eq!(a, b, "identical mesh → byte-identical .10d");
        assert_eq!(compiled_digest(&a), compiled_digest(&b));
    }

    #[test]
    fn digest_changes_when_geometry_changes() {
        let a = compile_mesh_to_10d(&cube()).unwrap();
        let mut m2 = cube();
        m2.positions[6] = [1.0, 1.0, 0.5]; // move one vertex
        let b = compile_mesh_to_10d(&m2).unwrap();
        assert_ne!(compiled_digest(&a), compiled_digest(&b));
    }

    #[test]
    fn decode_rejects_garbage() {
        assert_eq!(decode_10d_mesh(&[0u8; 8]), Err(Compile10dError::BadHeader));
    }

    /// A single OBJ triangle — the smallest valid source asset.
    const TRI_OBJ: &[u8] = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";

    #[test]
    fn compile_asset_binds_manifest_to_its_container() {
        let a = compile_asset(TRI_OBJ, Some("obj"), "urn:asset:tri", "obj").unwrap();

        // The container round-trips back to the mesh the manifest describes.
        let back = decode_10d_mesh(&a.container_10d).unwrap();
        assert_eq!(back.triangle_count(), 1);
        assert_eq!(back.vertex_count(), 3);

        // compiled_digest field == the container's real whole-file CRC (the citation is honest).
        assert_eq!(a.compiled_digest, compiled_digest(&a.container_10d));
        // source_digest == CRC of the source bytes.
        assert_eq!(a.source_digest, crc32c(TRI_OBJ));

        // Both digests appear as manifest facts (object-side), binding the two layers.
        let objs: Vec<u64> = a.quins.iter().map(|q| q.object).collect();
        assert!(objs.contains(&(a.compiled_digest as u64)), "manifest cites compiledDigest");
        assert!(objs.contains(&(a.source_digest as u64)), "manifest cites sourceDigest");
    }

    #[test]
    fn compile_asset_is_deterministic() {
        let a = compile_asset(TRI_OBJ, Some("obj"), "urn:asset:tri", "obj").unwrap();
        let b = compile_asset(TRI_OBJ, Some("obj"), "urn:asset:tri", "obj").unwrap();
        assert_eq!(a.container_10d, b.container_10d, "byte-identical container");
        assert_eq!(a.compiled_digest, b.compiled_digest);
        assert_eq!(a.source_digest, b.source_digest);
    }

    #[test]
    fn compile_asset_surfaces_import_errors() {
        // Not a recognisable asset in any supported format.
        let err = compile_asset(b"\x00\x01\x02\x03", None, "urn:asset:junk", "obj");
        assert!(matches!(err, Err(Compile10dError::Import(_))));
    }

    #[test]
    fn compile_organ_asset_binds_body_system_and_model() {
        let plain = compile_asset(TRI_OBJ, Some("obj"), "urn:asset:organ", "obj").unwrap();
        let organ = compile_organ_asset(
            TRI_OBJ,
            Some("obj"),
            "urn:asset:organ",
            "obj",
            Some("respiratory"),
            Some("male"),
        )
        .unwrap();
        // Same geometry → identical container + digests; only the manifest gains the anatomy facts.
        assert_eq!(organ.container_10d, plain.container_10d);
        assert_eq!(organ.compiled_digest, plain.compiled_digest);
        assert_eq!(organ.quins.len(), plain.quins.len() + 2, "two anatomy facts added");
        // bodySystem + anatomyModel strings are carried in the lexicon.
        let vals: Vec<&str> = organ.lexicon.values().map(String::as_str).collect();
        assert!(vals.contains(&"respiratory"), "bodySystem fact present");
        assert!(vals.contains(&"male"), "anatomyModel fact present");
        // None/None path is exactly compile_asset — no phantom facts.
        let none =
            compile_organ_asset(TRI_OBJ, Some("obj"), "urn:asset:organ", "obj", None, None).unwrap();
        assert_eq!(none.quins.len(), plain.quins.len());
    }
}
