//! Per-leaf `.emf` companion registration, digest-verified optical binding,
//! and physical-metre 3D spectral/polarimetric surface sampling.
//!
//! Resolves `urn:qualia:anatomy:atlas:appearance:*` companion registrations from
//! HMC bundles (embedded Q42 semantic models, companion `.q42` volumes, and `.ttl`
//! graphs) or explicit caller registrations, validates digest and coordinate-frame
//! contracts fail-closed, and evaluates linear RGBA optical vertex colours in the
//! leaf's authored metre frame before viewer normalization or assembly placement.

use crate::bundle::{
    semantic_model::{open_semantic_model, valid_iri, SEMANTIC_MODEL_KEY},
    BundleReader,
};
use crate::q42::{
    emf_profiles::det_3x3,
    field_spectrum::VACUUM_SPEED_OF_LIGHT_M_PER_S,
};
use crate::q42_volume::{
    Q42RangeQueryCursor, Q42RangeQueryPattern, Q42RangeQueryPlan, WasmSliceQ42Reader,
    MAX_COMPRESSED_SUPERBLOCK_SIZE, SUPERBLOCK_SIZE,
};
use crate::render::portal_emf_state::{
    carrier_hz_to_spectral_sigma, DisplayEmfCell, DisplayOpticalCell, DisplaySelection, LoadedEmf,
};
use crate::{q_hash, NQuin};
use rio_api::{
    model::{Subject, Term},
    parser::TriplesParser,
};
use rio_turtle::TurtleParser;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Cursor;

pub const APPEARANCE_NS: &str = "urn:qualia:anatomy:atlas:appearance:";
pub const PRED_HAS_COMPANION: &str =
    "urn:qualia:anatomy:atlas:appearance:hasIllustrativeEmfCompanion";
pub const CLASS_ILLUSTRATIVE_COMPANION: &str =
    "urn:qualia:anatomy:atlas:appearance:IllustrativeFieldCompanion";
pub const PRED_GEOMETRY_MEMBER: &str = "urn:qualia:anatomy:atlas:appearance:geometryMember";
pub const PRED_FIELD_MEMBER: &str = "urn:qualia:anatomy:atlas:appearance:fieldMember";
pub const PRED_GEOMETRY_SHA256: &str = "urn:qualia:anatomy:atlas:appearance:geometrySha256";
pub const PRED_FIELD_SHA256: &str = "urn:qualia:anatomy:atlas:appearance:fieldSha256";
pub const PRED_SOURCE_BUNDLE_SHA256: &str =
    "urn:qualia:anatomy:atlas:appearance:sourceBundleSha256";
pub const PRED_FRAME: &str = "urn:qualia:anatomy:atlas:appearance:frame";
pub const PRED_UNITS: &str = "urn:qualia:anatomy:atlas:appearance:units";
pub const PRED_EMITTER_X_M: &str = "urn:qualia:anatomy:atlas:appearance:emitterXM";
pub const PRED_EMITTER_Y_M: &str = "urn:qualia:anatomy:atlas:appearance:emitterYM";
pub const PRED_EMITTER_Z_M: &str = "urn:qualia:anatomy:atlas:appearance:emitterZM";
pub const PRED_WAVELENGTH_NM: &str = "urn:qualia:anatomy:atlas:appearance:authoredWavelengthNm";
pub const PRED_SCOPE: &str = "urn:qualia:anatomy:atlas:appearance:scope";
pub const PRED_FIELD_FROM_LEAF_MATRIX: &str =
    "urn:qualia:anatomy:atlas:appearance:fieldFromLeafMatrix";

/// Identity 4x4 column-major transform in metres.
pub const IDENTITY_TRANSFORM_4X4: [[f64; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// Declared registration linking one `.10d` geometry leaf to one `.emf` optical companion.
#[derive(Clone, Debug, PartialEq)]
pub struct EmfCompanionRegistration {
    pub companion_iri: String,
    pub geometry_member: String,
    pub field_member: String,
    pub geometry_sha256: Option<[u8; 32]>,
    pub field_sha256: Option<[u8; 32]>,
    pub units: String,
    pub frame_description: String,
    pub authored_wavelength_nm: Option<f64>,
    pub emitter_position_m: Option<[f64; 3]>,
    /// Column-major 4x4 affine matrix mapping leaf-authored metre coordinates into
    /// the EMF companion's physical metre frame.
    pub field_from_leaf: [[f64; 4]; 4],
    pub scope: String,
}

impl EmfCompanionRegistration {
    /// Construct a default metre-frame identity registration between `geometry_member` and `field_member`.
    pub fn identity_metres(geometry_member: impl Into<String>, field_member: impl Into<String>) -> Self {
        Self {
            companion_iri: String::from("urn:qualia:anatomy:atlas:appearance:default-companion"),
            geometry_member: geometry_member.into(),
            field_member: field_member.into(),
            geometry_sha256: None,
            field_sha256: None,
            units: String::from("metres"),
            frame_description: String::from("Authored leaf physical metre frame"),
            authored_wavelength_nm: None,
            emitter_position_m: None,
            field_from_leaf: IDENTITY_TRANSFORM_4X4,
            scope: String::from("Registered optical field companion"),
        }
    }

    /// Validate units, wavelengths, and invertibility of `field_from_leaf` fail-closed.
    pub fn validate(&self) -> Result<(), String> {
        if self.geometry_member.trim().is_empty() {
            return Err("EMF companion registration is missing geometryMember".into());
        }
        if self.field_member.trim().is_empty() {
            return Err("EMF companion registration is missing fieldMember".into());
        }
        let u = self.units.trim();
        if !u.is_empty() && u != "metres" && u != "meters" && u != "m" {
            return Err(format!(
                "EMF companion registration requires metre units ('metres' or 'm'), got {u:?}"
            ));
        }
        if let Some(w) = self.authored_wavelength_nm {
            if !w.is_finite() || w <= 0.0 {
                return Err(format!(
                    "EMF companion registration has invalid authoredWavelengthNm: {w}"
                ));
            }
        }
        if let Some(pos) = self.emitter_position_m {
            if pos.iter().any(|v| !v.is_finite()) {
                return Err("EMF companion registration has non-finite emitter coordinates".into());
            }
        }
        validate_affine_transform_4x4(&self.field_from_leaf)
    }
}

/// Validate that a column-major 4x4 affine matrix is finite, has bottom row `[0, 0, 0, 1]`,
/// and has an invertible 3x3 linear part.
pub fn validate_affine_transform_4x4(m: &[[f64; 4]; 4]) -> Result<(), String> {
    if m.iter().flatten().any(|v| !v.is_finite()) {
        return Err("EMF companion registration transform contains non-finite entries".into());
    }
    if m[0][3].abs() > 1.0e-12
        || m[1][3].abs() > 1.0e-12
        || m[2][3].abs() > 1.0e-12
        || (m[3][3] - 1.0).abs() > 1.0e-12
    {
        return Err(
            "EMF companion registration transform must be a valid 4x4 affine matrix".into(),
        );
    }
    let linear = [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ];
    let det = det_3x3(linear);
    if !det.is_finite() || det.abs() <= 1.0e-15 {
        return Err("EMF companion registration transform is singular (non-invertible)".into());
    }
    Ok(())
}

/// Transform a 3D point in leaf-authored metres by a column-major 4x4 affine matrix.
#[inline]
pub fn transform_leaf_point_m(m: &[[f64; 4]; 4], p_leaf: [f32; 3]) -> [f64; 3] {
    let [x, y, z] = [f64::from(p_leaf[0]), f64::from(p_leaf[1]), f64::from(p_leaf[2])];
    [
        m[0][0] * x + m[1][0] * y + m[2][0] * z + m[3][0],
        m[0][1] * x + m[1][1] * y + m[2][1] * z + m[3][1],
        m[0][2] * x + m[1][2] * y + m[2][2] * z + m[3][2],
    ]
}

/// A verified per-leaf binding between a `.10d` geometry leaf and an admitted `LoadedEmf` field.
pub struct BoundLeafEmfCompanion<'a> {
    pub registration: EmfCompanionRegistration,
    pub field: &'a LoadedEmf,
    pub time_slice: u32,
}

impl<'a> BoundLeafEmfCompanion<'a> {
    /// Construct and validate a per-leaf optical binding against the leaf's `.10d` SHA-256 digest.
    pub fn bind(
        registration: EmfCompanionRegistration,
        leaf_sha256: [u8; 32],
        field: &'a LoadedEmf,
        time_slice: u32,
    ) -> Result<Self, String> {
        registration.validate()?;
        if let Some(expected_geom) = registration.geometry_sha256 {
            if expected_geom != leaf_sha256 {
                return Err(format!(
                    "EMF companion geometrySha256 mismatch for {:?}",
                    registration.geometry_member
                ));
            }
        }
        let field_sha: [u8; 32] = Sha256::digest(field.source_bytes()).into();
        if let Some(expected_field) = registration.field_sha256 {
            if expected_field != field_sha {
                return Err(format!(
                    "EMF companion fieldSha256 mismatch for {:?}",
                    registration.field_member
                ));
            }
        }
        let nt = field.dimensions()[3].max(1);
        if time_slice >= nt {
            return Err(format!(
                "EMF companion time_slice {time_slice} out of bounds for nt={nt}"
            ));
        }
        Ok(Self {
            registration,
            field,
            time_slice,
        })
    }

    /// Deterministic 32-byte digest capturing the companion field digest, selection, time slice,
    /// wavelength interpretation, and leaf-to-field transform for residency cache invalidation.
    pub fn binding_digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"qualia:portal:leaf-emf-binding:v1");
        hasher.update(self.registration.geometry_member.as_bytes());
        hasher.update([0]);
        hasher.update(self.registration.field_member.as_bytes());
        hasher.update([0]);
        let field_sha: [u8; 32] = Sha256::digest(self.field.source_bytes()).into();
        hasher.update(field_sha);
        let sel: DisplaySelection = self.field.selection();
        for id in [sel.descriptor_id, sel.frame_id, sel.clock_id, sel.stream_id, self.time_slice] {
            hasher.update(id.to_le_bytes());
        }
        for col in &self.registration.field_from_leaf {
            for val in col {
                hasher.update(val.to_bits().to_le_bytes());
            }
        }
        let wl_bits = self
            .registration
            .authored_wavelength_nm
            .unwrap_or(0.0)
            .to_bits();
        hasher.update(wl_bits.to_le_bytes());
        hasher.finalize().into()
    }
}

/// Combine a base geometry or assembly residency digest with one or more bound leaf companion digests.
pub fn combine_residency_digest_with_emf(
    base_digest: [u8; 32],
    leaf_bindings: &[(usize, [u8; 32])],
) -> [u8; 32] {
    if leaf_bindings.is_empty() {
        return base_digest;
    }
    let mut hasher = Sha256::new();
    hasher.update(b"qualia:portal:mesh-emf-residency:v1");
    hasher.update(base_digest);
    for (slot, digest) in leaf_bindings {
        hasher.update((*slot as u64).to_le_bytes());
        hasher.update(digest);
    }
    hasher.finalize().into()
}

/// Parse a 64-character hex SHA-256 string into `[u8; 32]`.
pub fn parse_sha256_hex(hex_str: &str) -> Result<[u8; 32], String> {
    let trimmed = hex_str.trim();
    if trimmed.len() != 64 {
        return Err(format!("Expected 64-char SHA-256 hex digest, got {:?}", trimmed));
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&trimmed[i * 2..i * 2 + 2], 16)
            .map_err(|_| format!("Invalid hex in SHA-256 digest: {:?}", trimmed))?;
    }
    Ok(out)
}

/// Parse a whitespace- or comma-separated 16-float column-major 4x4 matrix literal.
pub fn parse_matrix_4x4(text: &str) -> Result<[[f64; 4]; 4], String> {
    let cleaned = text.trim().trim_matches(|c| c == '[' || c == ']');
    let nums: Result<Vec<f64>, _> = cleaned
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<f64>())
        .collect();
    let vals = nums.map_err(|_| format!("Invalid 4x4 transform matrix literal: {text:?}"))?;
    if vals.len() != 16 {
        return Err(format!(
            "Expected 16 numbers in fieldFromLeafMatrix, got {}",
            vals.len()
        ));
    }
    let mut m = [[0.0f64; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            m[col][row] = vals[col * 4 + row];
        }
    }
    validate_affine_transform_4x4(&m)?;
    Ok(m)
}

#[derive(Default)]
struct RawCompanionProps {
    props: BTreeMap<String, String>,
}

impl RawCompanionProps {
    fn set(&mut self, pred: &str, value: String) {
        self.props.insert(pred.to_owned(), value);
    }

    fn into_registration(self, subject_iri: String) -> Result<Option<EmfCompanionRegistration>, String> {
        let Some(geometry_member) = self.props.get(PRED_GEOMETRY_MEMBER).cloned() else {
            return Ok(None);
        };
        let Some(field_member) = self.props.get(PRED_FIELD_MEMBER).cloned() else {
            return Err(format!(
                "EMF companion {subject_iri} declares geometryMember without fieldMember"
            ));
        };
        let geometry_sha256 = match self.props.get(PRED_GEOMETRY_SHA256) {
            Some(s) => Some(parse_sha256_hex(s)?),
            None => None,
        };
        let field_sha256 = match self.props.get(PRED_FIELD_SHA256) {
            Some(s) => Some(parse_sha256_hex(s)?),
            None => None,
        };
        let units = self
            .props
            .get(PRED_UNITS)
            .cloned()
            .unwrap_or_else(|| String::from("metres"));
        let frame_description = self.props.get(PRED_FRAME).cloned().unwrap_or_default();
        let authored_wavelength_nm = match self.props.get(PRED_WAVELENGTH_NM) {
            Some(s) => Some(
                s.trim()
                    .parse::<f64>()
                    .map_err(|_| format!("Invalid authoredWavelengthNm literal: {s:?}"))?,
            ),
            None => None,
        };
        let emitter_position_m = match (
            self.props.get(PRED_EMITTER_X_M),
            self.props.get(PRED_EMITTER_Y_M),
            self.props.get(PRED_EMITTER_Z_M),
        ) {
            (Some(x), Some(y), Some(z)) => {
                let px = x
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| format!("Invalid emitterXM: {x:?}"))?;
                let py = y
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| format!("Invalid emitterYM: {y:?}"))?;
                let pz = z
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| format!("Invalid emitterZM: {z:?}"))?;
                Some([px, py, pz])
            }
            _ => None,
        };
        let field_from_leaf = match self.props.get(PRED_FIELD_FROM_LEAF_MATRIX) {
            Some(s) => parse_matrix_4x4(s)?,
            None => IDENTITY_TRANSFORM_4X4,
        };
        let scope = self.props.get(PRED_SCOPE).cloned().unwrap_or_default();
        let reg = EmfCompanionRegistration {
            companion_iri: subject_iri,
            geometry_member,
            field_member,
            geometry_sha256,
            field_sha256,
            units,
            frame_description,
            authored_wavelength_nm,
            emitter_position_m,
            field_from_leaf,
            scope,
        };
        reg.validate()?;
        Ok(Some(reg))
    }
}

/// Strip RDF literal quotes and datatype/language suffix (e.g. `"620.0"^^<http://...>` -> `620.0`).
fn unquote_rdf_term(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix('"') {
        if let Some(end_quote) = rest.rfind('"') {
            return rest[..end_quote].replace("\\\"", "\"").replace("\\\\", "\\");
        }
    }
    trimmed.to_owned()
}

/// Parse Turtle text for `urn:qualia:anatomy:atlas:appearance:*` companion registrations.
pub fn parse_turtle_emf_registrations(
    turtle_str: &str,
) -> Result<Vec<EmfCompanionRegistration>, String> {
    let base = oxiri::Iri::parse(String::from("urn:qualia:hmc:registration")).ok();
    let mut parser = TurtleParser::new(Cursor::new(turtle_str.as_bytes()), base);
    let mut subjects: BTreeMap<String, RawCompanionProps> = BTreeMap::new();
    let mut parse_err: Option<String> = None;

    let res = parser.parse_all(&mut |triple| -> Result<(), Box<dyn std::error::Error>> {
        let pred = triple.predicate.iri;
        if !pred.starts_with(APPEARANCE_NS) {
            return Ok(());
        }
        let subj = match triple.subject {
            Subject::NamedNode(n) => n.iri.to_owned(),
            Subject::BlankNode(b) => format!("_:{}", b.id),
            Subject::Triple(_) => return Ok(()),
        };
        let obj_val = match triple.object {
            Term::NamedNode(n) => n.iri.to_owned(),
            Term::BlankNode(b) => format!("_:{}", b.id),
            Term::Literal(lit) => unquote_rdf_term(&lit.to_string()),
            Term::Triple(_) => return Ok(()),
        };
        subjects.entry(subj).or_default().set(pred, obj_val);
        Ok(())
    });
    if let Err(e) = res {
        parse_err = Some(e.to_string());
    }
    if let Some(err) = parse_err {
        return Err(format!("Invalid EMF companion registration Turtle: {err}"));
    }

    let mut out = Vec::new();
    for (subj, raw) in subjects {
        if let Some(reg) = raw.into_registration(subj)? {
            out.push(reg);
        }
    }
    Ok(out)
}

/// Query a `WasmSliceQ42Reader` for `urn:qualia:anatomy:atlas:appearance:*` companion registrations.
pub fn extract_q42_emf_registrations(
    reader: &WasmSliceQ42Reader<'_>,
) -> Result<Vec<EmfCompanionRegistration>, String> {
    const KNOWN_PREDS: [&str; 12] = [
        PRED_GEOMETRY_MEMBER,
        PRED_FIELD_MEMBER,
        PRED_GEOMETRY_SHA256,
        PRED_FIELD_SHA256,
        PRED_SOURCE_BUNDLE_SHA256,
        PRED_FRAME,
        PRED_UNITS,
        PRED_EMITTER_X_M,
        PRED_EMITTER_Y_M,
        PRED_EMITTER_Z_M,
        PRED_WAVELENGTH_NM,
        PRED_FIELD_FROM_LEAF_MATRIX,
    ];
    let mut compressed = vec![0u8; MAX_COMPRESSED_SUPERBLOCK_SIZE];
    let mut decoded = vec![0u8; SUPERBLOCK_SIZE];
    let mut rows = [NQuin::default(); 64];
    let mut lex_page = vec![0u8; 1024 * 1024];
    let mut lex_buf = [0u8; 4096];

    let mut subjects: BTreeMap<u64, (String, RawCompanionProps)> = BTreeMap::new();

    for pred_iri in KNOWN_PREDS {
        let pred_hash = q_hash(pred_iri);
        let plan = Q42RangeQueryPlan::for_pattern(Q42RangeQueryPattern {
            predicate: Some(pred_hash),
            ..Q42RangeQueryPattern::default()
        });
        let mut cursor = Q42RangeQueryCursor::default();
        loop {
            let page = reader
                .execute_query_page_into(
                    plan,
                    cursor,
                    &mut compressed,
                    &mut decoded,
                    &mut rows,
                )
                .map_err(|e| format!("Q42 companion query error: {e}"))?;
            for row in &rows[..page.returned] {
                let Some(obj_len) = reader
                    .lookup_lexicon_hash_into(row.object, &mut lex_page, &mut lex_buf)
                    .map_err(|e| e.to_string())?
                else {
                    continue;
                };
                let Ok(obj_raw) = std::str::from_utf8(&lex_buf[..obj_len]) else {
                    continue;
                };
                let obj_val = unquote_rdf_term(obj_raw);
                let subj_iri = if let Some((existing_iri, _)) = subjects.get(&row.subject) {
                    existing_iri.clone()
                } else {
                    let s_len = reader
                        .lookup_lexicon_hash_into(row.subject, &mut lex_page, &mut lex_buf)
                        .map_err(|e| e.to_string())?
                        .unwrap_or(0);
                    std::str::from_utf8(&lex_buf[..s_len])
                        .ok()
                        .filter(|s| valid_iri(s))
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("urn:qualia:q42:subject:{:016x}", row.subject))
                };
                subjects
                    .entry(row.subject)
                    .or_insert_with(|| (subj_iri, RawCompanionProps::default()))
                    .1
                    .set(pred_iri, obj_val);
            }
            let Some(next) = page.next_cursor else {
                break;
            };
            cursor = next;
        }
    }

    let mut out = Vec::new();
    for (_, (subj_iri, raw)) in subjects {
        if let Some(reg) = raw.into_registration(subj_iri)? {
            out.push(reg);
        }
    }
    Ok(out)
}

/// Discover and validate all `.emf` companion registrations in an HMC bundle.
///
/// Inspects `.ttl` entries, embedded `semantic_model.q42` / `.q42` entries, and verifies
/// that each referenced `geometryMember` and `fieldMember` in the bundle has the expected
/// kind and SHA-256 digest.
pub fn resolve_bundle_emf_registrations(
    bundle: &BundleReader<'_>,
) -> Result<BTreeMap<String, EmfCompanionRegistration>, String> {
    let mut by_geometry: BTreeMap<String, EmfCompanionRegistration> = BTreeMap::new();

    // 1. Inspect any Turtle registration entries in the bundle.
    for entry in bundle.entries() {
        if entry.kind == "ttl" || entry.key.ends_with(".ttl") {
            if !bundle.verify_entry(&entry.key) {
                return Err(format!(
                    "HMC Turtle entry {:?} failed SHA-256 verification",
                    entry.key
                ));
            }
            if let Some(bytes) = bundle.get(&entry.key) {
                if bytes.iter().any(|&b| b == b'@')
                    && std::str::from_utf8(bytes)
                        .is_ok_and(|s| s.contains(APPEARANCE_NS))
                {
                    let text = std::str::from_utf8(bytes)
                        .map_err(|e| format!("Invalid UTF-8 in {:?}: {e}", entry.key))?;
                    for reg in parse_turtle_emf_registrations(text)? {
                        insert_or_verify_registration(&mut by_geometry, reg)?;
                    }
                }
            }
        }
    }

    // 2. Inspect semantic_model.q42 and any standalone .q42 entries in the bundle.
    if let Ok(model) = open_semantic_model(bundle) {
        for reg in extract_q42_emf_registrations(&model)? {
            insert_or_verify_registration(&mut by_geometry, reg)?;
        }
    }
    for entry in bundle.entries() {
        if entry.kind == "q42" && entry.key != SEMANTIC_MODEL_KEY {
            if !bundle.verify_entry(&entry.key) {
                return Err(format!(
                    "HMC Q42 entry {:?} failed SHA-256 verification",
                    entry.key
                ));
            }
            if let Some(bytes) = bundle.get(&entry.key) {
                if let Ok(reader) = WasmSliceQ42Reader::open(bytes) {
                    for reg in extract_q42_emf_registrations(&reader)? {
                        insert_or_verify_registration(&mut by_geometry, reg)?;
                    }
                }
            }
        }
    }

    // 3. Validate each discovered registration against the bundle's manifest entries.
    for reg in by_geometry.values_mut() {
        let geom_entry = bundle.entry(&reg.geometry_member).ok_or_else(|| {
            format!(
                "EMF companion registration references missing geometryMember {:?}",
                reg.geometry_member
            )
        })?;
        if geom_entry.kind != "10d" || !bundle.verify_entry(&reg.geometry_member) {
            return Err(format!(
                "EMF companion geometryMember {:?} failed 10d kind or SHA-256 check",
                reg.geometry_member
            ));
        }
        let geom_sha: [u8; 32] = geom_entry
            .sha256
            .as_slice()
            .try_into()
            .map_err(|_| format!("Invalid SHA-256 length on {:?}", reg.geometry_member))?;
        if let Some(expected) = reg.geometry_sha256 {
            if expected != geom_sha {
                return Err(format!(
                    "EMF companion registration geometrySha256 mismatch for {:?}",
                    reg.geometry_member
                ));
            }
        } else {
            reg.geometry_sha256 = Some(geom_sha);
        }

        let field_entry = bundle.entry(&reg.field_member).ok_or_else(|| {
            format!(
                "EMF companion registration references missing fieldMember {:?}",
                reg.field_member
            )
        })?;
        if field_entry.kind != "emf" || !bundle.verify_entry(&reg.field_member) {
            return Err(format!(
                "EMF companion fieldMember {:?} failed emf kind or SHA-256 check",
                reg.field_member
            ));
        }
        let field_sha: [u8; 32] = field_entry
            .sha256
            .as_slice()
            .try_into()
            .map_err(|_| format!("Invalid SHA-256 length on {:?}", reg.field_member))?;
        if let Some(expected) = reg.field_sha256 {
            if expected != field_sha {
                return Err(format!(
                    "EMF companion registration fieldSha256 mismatch for {:?}",
                    reg.field_member
                ));
            }
        } else {
            reg.field_sha256 = Some(field_sha);
        }
    }

    Ok(by_geometry)
}

fn insert_or_verify_registration(
    map: &mut BTreeMap<String, EmfCompanionRegistration>,
    reg: EmfCompanionRegistration,
) -> Result<(), String> {
    if let Some(existing) = map.get(&reg.geometry_member) {
        if existing.field_member != reg.field_member
            || (existing.geometry_sha256.is_some()
                && reg.geometry_sha256.is_some()
                && existing.geometry_sha256 != reg.geometry_sha256)
            || (existing.field_sha256.is_some()
                && reg.field_sha256.is_some()
                && existing.field_sha256 != reg.field_sha256)
            || existing.field_from_leaf != reg.field_from_leaf
        {
            return Err(format!(
                "Conflicting EMF companion registrations for geometryMember {:?}",
                reg.geometry_member
            ));
        }
        return Ok(());
    }
    map.insert(reg.geometry_member.clone(), reg);
    Ok(())
}

/// Sample a `LoadedEmf` optical field at physical metre positions `positions_m` in the leaf's
/// authored metre frame, returning linear RGBA vertex colours `[r, g, b, 1.0]`.
///
/// The sampler transforms each leaf vertex into the field's physical metre frame via
/// `bound.registration.field_from_leaf`, converts from the field's `FramePose` (`origin_m`
/// and `rotation_xyzw`) into local grid coordinates, trilinearly interpolates valid 3D cells
/// at `bound.time_slice`, and evaluates the tissue optical response combining base tissue
/// reflectance with the local spectral excitation hue and degree of polarization.
pub fn sample_optical_surface_colors(
    positions_m: &[[f32; 3]],
    base_tissue_rgb: [f32; 3],
    bound: &BoundLeafEmfCompanion<'_>,
) -> Vec<[f32; 4]> {
    let field = bound.field;
    let [nx, ny, nz, nt] = field.dimensions();
    let ti = (bound.time_slice.min(nt.saturating_sub(1))) as usize;
    let spatial = (nx as usize) * (ny as usize) * (nz as usize);
    let slice_offset = ti * spatial;
    let cells = &field.display_cells()[slice_offset..slice_offset + spatial];
    let opt_cells = &field.optical_cells()[slice_offset..slice_offset + spatial];

    // Determine the physical peak magnitude across valid cells in this time slice so both
    // micro-excitation fields (e.g. 0.18 V/m in bladder.emf) and macro-fields normalize
    // cleanly without clipping spatial gradients.
    let mut peak_mag = 0.0_f64;
    let mut min_valid_mag = f64::INFINITY;
    for c in cells {
        if c.valid && c.magnitude_v_per_m.is_finite() && c.magnitude_v_per_m > 0.0 {
            peak_mag = peak_mag.max(c.magnitude_v_per_m);
            min_valid_mag = min_valid_mag.min(c.magnitude_v_per_m);
        }
    }
    if !min_valid_mag.is_finite() {
        min_valid_mag = 0.0;
    }

    let fallback_sigma = bound
        .registration
        .authored_wavelength_nm
        .filter(|&w| w.is_finite() && w > 0.0)
        .map(|w_nm| {
            let hz = (VACUUM_SPEED_OF_LIGHT_M_PER_S * 1.0e9) / w_nm;
            carrier_hz_to_spectral_sigma(hz)
        })
        .unwrap_or(0.0);

    let mut out = Vec::with_capacity(positions_m.len());
    for &p_leaf in positions_m {
        let p_field = transform_leaf_point_m(&bound.registration.field_from_leaf, p_leaf);
        let sampled = sample_trilinear_cell(field, cells, opt_cells, p_field);
        let rgba = evaluate_tissue_optical_response(
            sampled,
            base_tissue_rgb,
            peak_mag,
            min_valid_mag,
            fallback_sigma,
        );
        out.push(rgba);
    }
    out
}

#[derive(Clone, Copy, Default)]
struct SampledOpticalPoint {
    magnitude_v_per_m: f64,
    stokes_i: f64,
    stokes_q: f64,
    stokes_u: f64,
    stokes_v: f64,
    spectral_sigma: f64,
    has_sigma: bool,
    coherence: f64,
    weight: f64,
}

fn sample_trilinear_cell(
    field: &LoadedEmf,
    cells: &[DisplayEmfCell],
    opt_cells: &[DisplayOpticalCell],
    p_field_m: [f64; 3],
) -> SampledOpticalPoint {
    let [nx, ny, nz, _] = field.dimensions();
    if nx == 0 || ny == 0 || nz == 0 {
        return SampledOpticalPoint::default();
    }
    let pose = field.frame_pose();
    let bounds = field.local_bounds();

    // Map physical metre coordinates p_field_m into the frame's local cell-footprint coordinates.
    // Note: in EMF v4 mode-0 grids, `pose.origin_m` is the physical origin of cell (0,0,0) (or
    // lower corner of the domain) while `bounds` is `[-0.5*s, (dim - 0.5)*s]`.
    let rel = [
        p_field_m[0] - pose.origin_m[0],
        p_field_m[1] - pose.origin_m[1],
        p_field_m[2] - pose.origin_m[2],
    ];
    let local = pose.inverse_transform_vector(rel);

    let map_axis = |coord: f64, lo: f32, hi: f32, dim: u32| -> f64 {
        if dim <= 1 {
            return 0.0;
        }
        let lo = f64::from(lo);
        let hi = f64::from(hi);
        let span = (hi - lo).max(1.0e-12);
        // Continuous index in [0, dim - 1]
        let u = ((coord - lo) / span).clamp(0.0, 1.0);
        u * f64::from(dim - 1)
    };

    let fx = map_axis(local[0], bounds[0], bounds[1], nx);
    let fy = map_axis(local[1], bounds[2], bounds[3], ny);
    let fz = map_axis(local[2], bounds[4], bounds[5], nz);

    let ix0 = (fx.floor() as usize).min(nx as usize - 1);
    let iy0 = (fy.floor() as usize).min(ny as usize - 1);
    let iz0 = (fz.floor() as usize).min(nz as usize - 1);
    let ix1 = (ix0 + 1).min(nx as usize - 1);
    let iy1 = (iy0 + 1).min(ny as usize - 1);
    let iz1 = (iz0 + 1).min(nz as usize - 1);

    let tx = (fx - ix0 as f64).clamp(0.0, 1.0);
    let ty = (fy - iy0 as f64).clamp(0.0, 1.0);
    let tz = (fz - iz0 as f64).clamp(0.0, 1.0);

    let mut acc = SampledOpticalPoint::default();
    let mut sigma_weight = 0.0_f64;

    for (dz, wz) in [(iz0, 1.0 - tz), (iz1, tz)] {
        for (dy, wy) in [(iy0, 1.0 - ty), (iy1, ty)] {
            for (dx, wx) in [(ix0, 1.0 - tx), (ix1, tx)] {
                let w = wx * wy * wz;
                if w <= 0.0 {
                    continue;
                }
                let idx = (dz * (ny as usize) + dy) * (nx as usize) + dx;
                let c = cells[idx];
                let o = opt_cells[idx];
                if !c.valid || !o.valid {
                    continue;
                }
                acc.weight += w;
                acc.magnitude_v_per_m += w * c.magnitude_v_per_m;
                acc.stokes_i += w * o.stokes_i;
                acc.stokes_q += w * o.stokes_q;
                acc.stokes_u += w * o.stokes_u;
                acc.stokes_v += w * o.stokes_v;
                acc.coherence += w * o.coherence;
                if o.spectral_sigma > 0.0 || o.carrier_hz > 0.0 {
                    let s = if o.spectral_sigma > 0.0 {
                        o.spectral_sigma
                    } else {
                        carrier_hz_to_spectral_sigma(o.carrier_hz)
                    };
                    let sw = w * c.magnitude_v_per_m.max(1.0e-9);
                    acc.spectral_sigma += sw * s;
                    sigma_weight += sw;
                    acc.has_sigma = true;
                }
            }
        }
    }

    if acc.weight > 0.0 {
        let inv = 1.0 / acc.weight;
        acc.magnitude_v_per_m *= inv;
        acc.stokes_i *= inv;
        acc.stokes_q *= inv;
        acc.stokes_u *= inv;
        acc.stokes_v *= inv;
        acc.coherence *= inv;
        if sigma_weight > 0.0 {
            acc.spectral_sigma /= sigma_weight;
        }
    }
    acc
}

fn evaluate_tissue_optical_response(
    sampled: SampledOpticalPoint,
    base_tissue_rgb: [f32; 3],
    peak_mag: f64,
    min_valid_mag: f64,
    fallback_sigma: f64,
) -> [f32; 4] {
    if sampled.weight <= 0.0 || peak_mag <= 0.0 {
        return [base_tissue_rgb[0], base_tissue_rgb[1], base_tissue_rgb[2], 1.0];
    }

    // Relative field strength in [0, 1] across the active dynamic range, combined with
    // absolute physical response so spatial gradients across an organ (such as a lateral
    // dipole emitter) produce a clear spatial pattern while scaling monotonically with |E|.
    let abs_norm = (sampled.magnitude_v_per_m / peak_mag).clamp(0.0, 1.0);
    let span = peak_mag - min_valid_mag;
    let rel_norm = if span > peak_mag * 1.0e-3 {
        ((sampled.magnitude_v_per_m - min_valid_mag) / span).clamp(0.0, 1.0)
    } else {
        abs_norm
    };
    let excitation = (0.35 * abs_norm + 0.65 * rel_norm).clamp(0.0, 1.0) as f32;

    let sigma = if sampled.has_sigma {
        sampled.spectral_sigma
    } else {
        fallback_sigma
    };

    let spectral_rgb = if sigma > 0.0 {
        let raw = if sigma < 1.0 {
            crate::render::spectral::sigma_to_linear_rgb(sigma as f32)
        } else {
            etalon_render::emf::sigma_to_linear_rgb_cpu(sigma as f32)
        };
        let max_ch = raw[0].max(raw[1]).max(raw[2]).max(1.0e-5);
        [raw[0] / max_ch, raw[1] / max_ch, raw[2] / max_ch]
    } else {
        [1.0, 1.0, 1.0]
    };

    let pol_mag = sampled
        .stokes_q
        .hypot(sampled.stokes_u)
        .hypot(sampled.stokes_v);
    let dop = if sampled.stokes_i > 1.0e-18 {
        (pol_mag / sampled.stokes_i).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };
    let pol_boost = 1.0 + 0.20 * dop;

    // Combine base tissue diffuse albedo with the coupled spectral optical response.
    // Under scene lighting in mesh.wgsl (`base = material.base_color.rgb * vertex_rgb`),
    // unexcited regions retain natural tissue albedo while excited regions shift smoothly
    // to the physical spectral response.
    let blend = (0.04 + 0.96 * excitation.powf(0.65)).clamp(0.0, 1.0);
    let intensity = (0.55 + 0.65 * excitation) * pol_boost;
    let r = ((1.0 - blend) * base_tissue_rgb[0] + blend * spectral_rgb[0] * intensity).clamp(0.02, 2.0);
    let g = ((1.0 - blend) * base_tissue_rgb[1] + blend * spectral_rgb[1] * intensity).clamp(0.02, 2.0);
    let b = ((1.0 - blend) * base_tissue_rgb[2] + blend * spectral_rgb[2] * intensity).clamp(0.02, 2.0);

    [r, g, b, 1.0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{
        semantic_model::{pack_semantic_hmc, HmcPackAsset, HmcPackLimits, HmcSalience},
        BundleReadLimits, BundleReader, BundleWriter,
    };
    use crate::container_10d::{
        encode_archetype_ref_record, encode_container, encode_instance_batch,
        encode_instance_overrides, encode_manifold_identity_v2, encode_multiscale_strata_section,
        encode_symbolic_rdf_cbor_ld, instance_batch_encoded_len, instance_override_encoded_len,
        manifold_identity_v2_encoded_len, multiscale_strata_encoded_len, parse_section_table,
        sdf_brick_section::{
            encode_sdfb_into, sdfb_encoded_len, seal_10d_with_sdfb, SdfBrickGrid, SdfBrickKey,
            SdfBrickRecord, SdfSampleEncoding, SDFB_OCCUPANCY_BYTES, SDFB_SAMPLES,
        },
        seal_whole_file_crc32c, AlignmentTier, ArchetypeDigestAlg, ArchetypeRefRecord,
        ArchetypeTargetKind, Container10dHeader, InstanceBatchRecord, ManifoldIdentityV2,
        MultilingualLexicalTerm, NestedStratumNodeRecord, SectionInput, SectionType,
        StableEntityId, SymbolicRdfCborLdPayload, TypedFieldRecord, TypedPayloadResolver,
        ARCHETYPE_REF_RECORD_SIZE, FIELD_KIND_ARCHETYPE_REF, FIELD_KIND_INSTANCE_BATCH,
        FIELD_KIND_INSTANCE_OVERRIDES, FIELD_KIND_MULTISCALE_STRATA,
        FIELD_KIND_SYMBOLIC_RDF_CBOR_LD, MANIFOLD_IDENTITY_V2_MAX_FIELDS,
    };
    use crate::q42::{
        emf_profiles::{
            encode_field_samples, field_entry_slot, field_metadata_len, prepare_field_document,
            FieldChunkInput, FieldClock, FieldDocument, FieldFrame, FieldProvenance, FieldSample,
            FieldWriteLimits, GridIndexing, GridSpec, Limits as EmfLimits,
        },
        field_semantics as sem,
        field_spectrum::VACUUM_SPEED_OF_LIGHT_M_PER_S,
    };
    use crate::render::{
        assets::Mesh,
        compile_10d::{compile_mesh_to_10d, upgrade_legacy_10d_to_holographic_v2},
        portal_load_10d::{
            assembly_residency_digest, decode_hmc_leaf_assembly_with_emf,
            decode_hmc_leaf_cutaway_with_emf, decode_load_10d_with_emf, decode_sdfb_cutaway,
            decode_sdfb_cutaway_with_emf, PickSurfaceRegion, SectionPlane,
        },
        portal_load_emf::decode_emf_scientific,
        skeletal::Motor,
    };
    use crate::topology::header::{CellAnchor, TopologyInvariants};
    use std::sync::atomic::AtomicBool;

    const TEST_PACK_LIMITS: HmcPackLimits = HmcPackLimits {
        bundle: BundleReadLimits {
            file_bytes: 16 * 1024 * 1024,
            index_bytes: 256 * 1024,
            entries: 32,
            key_bytes: 1024,
            kind_bytes: 64,
            meta_bytes: 4096,
            decoded_index_bytes: 256 * 1024,
        },
        resource_bytes: 8 * 1024 * 1024,
        total_resource_bytes: 16 * 1024 * 1024,
        semantic_bytes: 4 * 1024 * 1024,
    };

    fn attach_strata_and_cbld(
        base_10d: &[u8],
        root_iri: &str,
        class_iri: &str,
        extra_iris: &[&str],
    ) -> Vec<u8> {
        const CONTEXT: &str = "https://qualiadb.org/contexts/10d-test-v2";
        const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
        let header = Container10dHeader::parse(base_10d).unwrap();
        let mut scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
        let mut records = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
        let source =
            TypedPayloadResolver::new(base_10d, &[], &mut scratch, &mut records).unwrap();
        let identity = source.identity().unwrap();

        let root_hash = q_hash(root_iri);
        let class_hash = q_hash(class_iri);
        let rdf_type_hash = q_hash(RDF_TYPE);
        let ctx_hash = q_hash(CONTEXT);

        let mut node = NestedStratumNodeRecord {
            host_anchor: CellAnchor {
                entity_hash: root_hash,
                cell_index: 0,
                anchor_domain: 0xFF,
                orientation_sign: 1,
                material_or_stratum: 0,
            },
            parent_node_index: u16::MAX,
            stratum_level: 0,
            constituent_kind: 0,
            interaction_regime: 0,
            embodiment_state: 3,
            quantum_basis_and_xc: 0,
            characteristic_scale_log10_m_q12: -12,
            taxon_or_ontology_iri_hash: class_hash,
            target_archetype_or_qpu_nquin_hash: 0,
            density_or_abundance_per_m3: 1.0,
            metabolic_flux_or_hartree_energy: 0.0,
            coupled_metric_a_f16: 0,
            coupled_metric_b_f16: 0,
            logic_invariants: TopologyInvariants::EMPTY,
            belnap_and_spin_u16: 1,
            orbital_or_coupling_flags: 0,
            record_crc32c: 0,
        };
        node.seal_crc32c();
        let mut mstr_bytes = vec![0; multiscale_strata_encoded_len(1).unwrap()];
        encode_multiscale_strata_section(&[node], 0, &mut mstr_bytes).unwrap();

        let mut terms = vec![root_iri, class_iri, RDF_TYPE, CONTEXT];
        terms.extend_from_slice(extra_iris);
        terms.sort_unstable();
        terms.dedup();
        let lexicon = terms
            .into_iter()
            .map(|iri| MultilingualLexicalTerm {
                term_hash: q_hash(iri),
                iri_utf8: iri.to_owned(),
                lang_bcp47: None,
                label_utf8: String::new(),
            })
            .collect();
        let cbld_bytes = encode_symbolic_rdf_cbor_ld(&SymbolicRdfCborLdPayload {
            context_iri: CONTEXT.to_owned(),
            lexicon,
            nquins: vec![[
                root_hash,
                rdf_type_hash,
                class_hash,
                ctx_hash,
                0,
                NQuin::calculate_parity(root_hash, rdf_type_hash, class_hash, ctx_hash, 0),
            ]],
            ..Default::default()
        })
        .unwrap();

        let mut sidecar = source
            .section_payload(SectionType::FieldSidecar)
            .unwrap_or_default()
            .to_vec();
        let mstr_off = sidecar.len() as u64;
        sidecar.extend_from_slice(&mstr_bytes);
        let cbld_off = sidecar.len() as u64;
        sidecar.extend_from_slice(&cbld_bytes);

        let mut fields = source.fields().to_vec();
        let next_id = fields.last().map_or(1, |f| f.field_id + 1);
        fields.push(TypedFieldRecord {
            field_id: next_id,
            kind: FIELD_KIND_MULTISCALE_STRATA,
            payload_section_id: SectionType::FieldSidecar as u32,
            payload_offset: mstr_off,
            payload_length: mstr_bytes.len() as u64,
            ..TypedFieldRecord::default()
        });
        fields.push(TypedFieldRecord {
            field_id: next_id + 1,
            kind: FIELD_KIND_SYMBOLIC_RDF_CBOR_LD,
            payload_section_id: SectionType::FieldSidecar as u32,
            payload_offset: cbld_off,
            payload_length: cbld_bytes.len() as u64,
            ..TypedFieldRecord::default()
        });

        let mut mid2 = vec![0; manifold_identity_v2_encoded_len(fields.len()).unwrap()];
        encode_manifold_identity_v2(identity, &fields, &mut mid2).unwrap();

        let mut sections = Vec::new();
        for desc in parse_section_table(base_10d, &header).unwrap() {
            let kind = desc.typ().unwrap();
            if matches!(kind, SectionType::ManifoldIdentityV2 | SectionType::FieldSidecar) {
                continue;
            }
            sections.push(SectionInput {
                section_type: kind,
                alignment_tier: AlignmentTier::from_u8(desc.alignment_tier).unwrap(),
                stride: desc.stride,
                element_count: desc.element_count,
                payload: source.section_payload(kind).unwrap(),
            });
        }
        sections.push(SectionInput {
            section_type: SectionType::FieldSidecar,
            alignment_tier: AlignmentTier::Page,
            stride: 0,
            element_count: 0,
            payload: &sidecar,
        });
        sections.push(SectionInput {
            section_type: SectionType::ManifoldIdentityV2,
            alignment_tier: AlignmentTier::Page,
            stride: 0,
            element_count: 0,
            payload: &mid2,
        });
        let mut out = vec![0; base_10d.len() + sidecar.len() + mid2.len() + 32768];
        let written = encode_container(&header, &sections, &mut out).unwrap();
        out.truncate(written);
        seal_whole_file_crc32c(&mut out);
        out
    }

    /// Build a sealed `.10d` SDFB hollow tube with an open internal cavity lumen.
    fn build_sdfb_hollow_organ_10d(root_iri: &str, wall_iri: &str, lumen_iri: &str) -> Vec<u8> {
        let proxy_mesh = Mesh {
            positions: vec![
                [-3.5, -3.5, -3.5],
                [3.5, -3.5, -3.5],
                [3.5, 3.5, -3.5],
                [-3.5, 3.5, -3.5],
                [-3.5, -3.5, 3.5],
                [3.5, -3.5, 3.5],
                [3.5, 3.5, 3.5],
                [-3.5, 3.5, 3.5],
            ],
            triangles: vec![
                [0, 2, 1],
                [0, 3, 2],
                [4, 5, 6],
                [4, 6, 7],
                [0, 1, 5],
                [0, 5, 4],
                [2, 3, 7],
                [2, 7, 6],
                [0, 4, 7],
                [0, 7, 3],
                [1, 2, 6],
                [1, 6, 5],
            ],
            min: [-3.5; 3],
            max: [3.5; 3],
        };
        let raw_10d = compile_mesh_to_10d(&proxy_mesh).unwrap();
        let holo_10d = upgrade_legacy_10d_to_holographic_v2(
            &raw_10d,
            root_iri,
            1,
            [0.0, 0.0, 0.0, 1.0, 0.0, 0.55, 1.0, 0.5, 0.98, 1.0],
        )
        .unwrap();
        let base_10d = attach_strata_and_cbld(
            &holo_10d,
            root_iri,
            "urn:test:class:HollowOrgan",
            &[wall_iri, lumen_iri],
        );

        let wall_id = q_hash(wall_iri);
        let lumen_id = q_hash(lumen_iri);
        let grid = SdfBrickGrid {
            origin_m: [-3.5; 3],
            voxel_spacing_m: [1.0; 3],
            base_bricks: [1; 3],
            encoding: SdfSampleEncoding::ScaledI16,
            meters_per_i16: 0.01,
        };
        let mut brick = SdfBrickRecord {
            key: SdfBrickKey {
                refinement_level: 0,
                coordinate: [0; 3],
            },
            wall_semantic_id: wall_id,
            lumen_semantic_id: lumen_id,
            samples: [0; SDFB_SAMPLES],
            occupancy: [0; SDFB_OCCUPANCY_BYTES],
        };
        for (index, bits) in brick.samples.iter_mut().enumerate() {
            let [ix, iy, iz] = [index % 8, (index / 8) % 8, index / 64];
            let [x, y, z] = [ix as f32 - 3.5, iy as f32 - 3.5, iz as f32 - 3.5];
            let r = x.hypot(y);
            let d = (r - 2.5).max(1.2 - r).max(z.abs() - 2.8);
            *bits = (d * 100.0).round() as i16 as u16;
            if d < 0.0 {
                brick.occupancy[index / 8] |= 1 << (index % 8);
            }
        }
        let mut sdfb_bytes = vec![0; sdfb_encoded_len(1).unwrap()];
        encode_sdfb_into(grid, &[brick], &mut sdfb_bytes).unwrap();
        seal_10d_with_sdfb(&base_10d, &sdfb_bytes, wall_iri, lumen_iri).unwrap()
    }

    /// Build a mode-0 EMF v4 companion over `[-3.5..+3.5]^3` in metres with an equal peak
    /// electric field magnitude (`1.0 V/m`) concentrated around `emitter_pos_m` at `wavelength_nm`.
    fn build_equal_magnitude_emf_v4(
        origin_m: [f64; 3],
        emitter_pos_m: [f64; 3],
        wavelength_nm: f64,
    ) -> Vec<u8> {
        let dims = [8u32, 8, 8];
        let spacing = [1.0, 1.0, 1.0];
        let spec = GridSpec {
            dimensions: dims,
            brick_dimensions: dims,
            spacing_m: spacing,
            indexing: GridIndexing::Linear,
        };
        let layout = spec.validate().unwrap();
        let brick = layout.brick(0).unwrap();
        let carrier_hz = (VACUUM_SPEED_OF_LIGHT_M_PER_S * 1.0e9) / wavelength_nm;

        let mut samples = Vec::with_capacity(brick.cell_count() as usize);
        for local in 0..brick.cell_count() {
            let local_pos = brick.local_position_m(local).unwrap();
            let world_pos = [
                origin_m[0] + local_pos[0],
                origin_m[1] + local_pos[1],
                origin_m[2] + local_pos[2],
            ];
            let dx = world_pos[0] - emitter_pos_m[0];
            let dy = world_pos[1] - emitter_pos_m[1];
            let dz = world_pos[2] - emitter_pos_m[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            // Peak |E| is exactly 1.0 V/m at the emitter; decays smoothly with distance.
            let amp = (-dist_sq / 6.0).exp().max(0.02);
            samples.push(FieldSample {
                electric_v_per_m: [amp, 0.0, 0.0],
                magnetic_tesla: [0.0, amp / VACUUM_SPEED_OF_LIGHT_M_PER_S, 0.0],
                position_m: local_pos,
                tick: 0,
                carrier_hz: Some(carrier_hz),
                stored_poynting_w_per_m2: None,
                confidence: Some(1.0),
            });
        }
        let mut payload = vec![0; samples.len() * 128];
        encode_field_samples(&samples, &mut payload, EmfLimits::default()).unwrap();
        let chunks = [FieldChunkInput {
            stream_id: 0,
            start_tick: 0,
            time_count: 0,
            spatial_key: Some(0),
            payload: &payload,
        }];
        let doc = FieldDocument {
            clock: FieldClock {
                scale: 0,
                epoch_seconds: 1_700_000_000,
                epoch_nanoseconds: 0,
                ticks_per_second_num: 1,
                ticks_per_second_den: 1,
            },
            frame: FieldFrame {
                origin_m,
                rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
                grid: Some(spec),
            },
            provenance: FieldProvenance {
                role: 1,
                algorithm: Some("qualia-emf-acceptance-fixture"),
                algorithm_version: Some("1"),
                lineage: None,
            },
            temporal_step_ticks: None,
            chunks: &chunks,
            align_payloads_to_4096: false,
        };
        let limits = FieldWriteLimits::default();
        let mut metadata = vec![0; field_metadata_len(&doc, limits).unwrap()];
        let mut entries = vec![field_entry_slot(); 1];
        let prepared = prepare_field_document(&doc, &mut metadata, &mut entries, limits).unwrap();
        let n = prepared.encoded_len().unwrap();
        let mut stage = vec![0; n];
        let mut out = vec![0; n];
        let mut m = [sem::NodeState::default(); 16];
        let mut e = [sem::NodeState::default(); 1];
        prepared
            .encode_into(
                &mut stage,
                &mut out,
                &mut sem::Scratch {
                    metadata: &mut m,
                    entries: &mut e,
                },
            )
            .unwrap();
        out
    }

    fn build_two_leaf_assembly_10d(
        assembly_root_iri: &str,
        leaf_a_key: &str,
        leaf_a_bytes: &[u8],
        leaf_a_class_iri: &str,
        leaf_a_inst_iri: &str,
        leaf_a_tx: f64,
        leaf_b_key: &str,
        leaf_b_bytes: &[u8],
        leaf_b_class_iri: &str,
        leaf_b_inst_iri: &str,
        leaf_b_tx: f64,
    ) -> Vec<u8> {
        let proxy_mesh = Mesh {
            positions: vec![[-1.0, -1.0, -1.0], [1.0, -1.0, -1.0], [0.0, 1.0, -1.0]],
            triangles: vec![[0, 1, 2]],
            min: [-1.0; 3],
            max: [1.0; 3],
        };
        let raw_10d = compile_mesh_to_10d(&proxy_mesh).unwrap();
        let holo_10d = upgrade_legacy_10d_to_holographic_v2(
            &raw_10d,
            assembly_root_iri,
            1,
            [0.0, 0.0, 0.0, 1.0, 0.0, 0.55, 1.0, 0.5, 0.98, 1.0],
        )
        .unwrap();
        let leaf_a_key_iri = crate::bundle::semantic_model::hmc_entry_key_iri(leaf_a_key);
        let leaf_b_key_iri = crate::bundle::semantic_model::hmc_entry_key_iri(leaf_b_key);
        let base_10d = attach_strata_and_cbld(
            &holo_10d,
            assembly_root_iri,
            "urn:test:class:OrganAssembly",
            &[
                leaf_a_class_iri,
                leaf_a_inst_iri,
                &leaf_a_key_iri,
                leaf_b_class_iri,
                leaf_b_inst_iri,
                &leaf_b_key_iri,
            ],
        );

        let header = Container10dHeader::parse(&base_10d).unwrap();
        let mut scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
        let mut records = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
        let source =
            TypedPayloadResolver::new(&base_10d, &[], &mut scratch, &mut records).unwrap();
        let mut sidecar = source
            .section_payload(SectionType::FieldSidecar)
            .unwrap()
            .to_vec();
        let mut fields = source.fields().to_vec();
        let mut next_id = fields.last().map_or(1, |f| f.field_id + 1);

        for (key_iri, bytes, class_iri, tx) in [
            (&leaf_a_key_iri, leaf_a_bytes, leaf_a_class_iri, leaf_a_tx),
            (&leaf_b_key_iri, leaf_b_bytes, leaf_b_class_iri, leaf_b_tx),
        ] {
            let mut l_scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
            let mut l_fields = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
            let leaf_view =
                TypedPayloadResolver::new(bytes, &[], &mut l_scratch, &mut l_fields).unwrap();
            let leaf_entity_id = leaf_view.identity().unwrap().entity_id.0;
            let sha: [u8; 32] = Sha256::digest(bytes).into();
            let aref = ArchetypeRefRecord::new(
                ArchetypeDigestAlg::Sha256,
                ArchetypeTargetKind::HmcBundleEntry,
                1,
                sha,
                leaf_entity_id,
                q_hash(class_iri),
                q_hash(key_iri),
            );
            let mut aref_bytes = [0u8; ARCHETYPE_REF_RECORD_SIZE];
            encode_archetype_ref_record(&aref, &mut aref_bytes).unwrap();
            let aref_off = sidecar.len() as u64;
            sidecar.extend_from_slice(&aref_bytes);
            fields.push(TypedFieldRecord {
                field_id: next_id,
                kind: FIELD_KIND_ARCHETYPE_REF,
                payload_section_id: SectionType::FieldSidecar as u32,
                payload_offset: aref_off,
                payload_length: aref_bytes.len() as u64,
                ..TypedFieldRecord::default()
            });
            next_id += 1;

            let motor = Motor::from_translation(tx, 0.0, 0.0);
            let pga = [
                motor.r_w as f32,
                motor.r_x as f32,
                motor.r_y as f32,
                motor.r_z as f32,
                -motor.d_x as f32,
                -motor.d_y as f32,
                -motor.d_z as f32,
                motor.d_w as f32,
            ];
            let mut iovr_bytes = vec![0u8; instance_override_encoded_len(0).unwrap()];
            encode_instance_overrides(1.0, 1.0, pga, &[], &mut iovr_bytes).unwrap();
            let iovr_off = sidecar.len() as u64;
            sidecar.extend_from_slice(&iovr_bytes);
            fields.push(TypedFieldRecord {
                field_id: next_id,
                kind: FIELD_KIND_INSTANCE_OVERRIDES,
                payload_section_id: SectionType::FieldSidecar as u32,
                payload_offset: iovr_off,
                payload_length: iovr_bytes.len() as u64,
                ..TypedFieldRecord::default()
            });
            next_id += 1;
        }

        let mut rec_a = InstanceBatchRecord::default();
        rec_a.entity_id_hash = q_hash(leaf_a_inst_iri);
        rec_a.proc_seed = 1;
        rec_a.seal_crc32c();
        let mut rec_b = InstanceBatchRecord::default();
        rec_b.entity_id_hash = q_hash(leaf_b_inst_iri);
        rec_b.proc_seed = 2;
        rec_b.seal_crc32c();
        let inst_records = [rec_a, rec_b];
        let mut inst_bytes = vec![0u8; instance_batch_encoded_len(2).unwrap()];
        encode_instance_batch(&inst_records, &mut inst_bytes).unwrap();
        let inst_off = sidecar.len() as u64;
        sidecar.extend_from_slice(&inst_bytes);
        fields.push(TypedFieldRecord {
            field_id: next_id,
            kind: FIELD_KIND_INSTANCE_BATCH,
            payload_section_id: SectionType::FieldSidecar as u32,
            payload_offset: inst_off,
            payload_length: inst_bytes.len() as u64,
            ..TypedFieldRecord::default()
        });

        let identity = ManifoldIdentityV2 {
            entity_id: StableEntityId(*b"test-assembly-01"),
            profile_id: 2,
            domain_id: 1,
            coordinate_convention_id: 1,
            coordinates: [0.0; 10],
            external_digest: None,
        };
        let mut mid2 = vec![0; manifold_identity_v2_encoded_len(fields.len()).unwrap()];
        encode_manifold_identity_v2(&identity, &fields, &mut mid2).unwrap();

        let mut sections = Vec::new();
        for desc in parse_section_table(&base_10d, &header).unwrap() {
            let kind = desc.typ().unwrap();
            if matches!(kind, SectionType::ManifoldIdentityV2 | SectionType::FieldSidecar) {
                continue;
            }
            sections.push(SectionInput {
                section_type: kind,
                alignment_tier: AlignmentTier::from_u8(desc.alignment_tier).unwrap(),
                stride: desc.stride,
                element_count: desc.element_count,
                payload: source.section_payload(kind).unwrap(),
            });
        }
        sections.push(SectionInput {
            section_type: SectionType::FieldSidecar,
            alignment_tier: AlignmentTier::Page,
            stride: 0,
            element_count: 0,
            payload: &sidecar,
        });
        sections.push(SectionInput {
            section_type: SectionType::ManifoldIdentityV2,
            alignment_tier: AlignmentTier::Page,
            stride: 0,
            element_count: 0,
            payload: &mid2,
        });
        let mut out = vec![0; base_10d.len() + sidecar.len() + mid2.len() + 32768];
        let written = encode_container(&header, &sections, &mut out).unwrap();
        out.truncate(written);
        seal_whole_file_crc32c(&mut out);
        out
    }

    #[test]
    #[serial_test::serial(gpu)]
    fn acceptance_swapping_equal_magnitude_emf_companions_updates_exterior_and_cutaway_preserving_geometry_lumen_and_picks() {
        let organ_10d = build_sdfb_hollow_organ_10d(
            "urn:test:organ:bladder",
            "urn:test:organ:bladder-wall",
            "urn:test:organ:bladder-lumen",
        );
        let organ_sha: [u8; 32] = Sha256::digest(&organ_10d).into();

        // Companion A: 620 nm (red-orange) concentrated at [+2.5, 0, 0] m (right wall).
        // Companion B: 465 nm (blue) concentrated at [-2.5, 0, 0] m (left wall), with equal peak |E| = 1.0 V/m.
        let emf_a_bytes = build_equal_magnitude_emf_v4([-3.5, -3.5, -3.5], [2.5, 0.0, 0.0], 620.0);
        let emf_b_bytes = build_equal_magnitude_emf_v4([-3.5, -3.5, -3.5], [-2.5, 0.0, 0.0], 465.0);
        let emf_a = decode_emf_scientific(&emf_a_bytes).unwrap();
        let emf_b = decode_emf_scientific(&emf_b_bytes).unwrap();
        assert_eq!(emf_a.frame_pose().origin_m, [-3.5, -3.5, -3.5]);
        assert_eq!(emf_b.frame_pose().origin_m, [-3.5, -3.5, -3.5]);

        let mut reg_a = EmfCompanionRegistration::identity_metres(
            "outputs/bladder.10d",
            "outputs/bladder.emf",
        );
        reg_a.geometry_sha256 = Some(organ_sha);
        reg_a.field_sha256 = Some(Sha256::digest(emf_a.source_bytes()).into());
        reg_a.authored_wavelength_nm = Some(620.0);

        let mut reg_b = reg_a.clone();
        reg_b.field_sha256 = Some(Sha256::digest(emf_b.source_bytes()).into());
        reg_b.authored_wavelength_nm = Some(465.0);

        let bound_a = BoundLeafEmfCompanion::bind(reg_a, organ_sha, &emf_a, 0).unwrap();
        let bound_b = BoundLeafEmfCompanion::bind(reg_b, organ_sha, &emf_b, 0).unwrap();

        // 1. Residency cache invalidation when swapping companions with identical .10d geometry digest.
        assert_ne!(bound_a.binding_digest(), bound_b.binding_digest());
        let res_uncoloured = combine_residency_digest_with_emf(organ_sha, &[]);
        let res_a = combine_residency_digest_with_emf(organ_sha, &[(0, bound_a.binding_digest())]);
        let res_b = combine_residency_digest_with_emf(organ_sha, &[(0, bound_b.binding_digest())]);
        assert_ne!(res_uncoloured, res_a);
        assert_ne!(res_a, res_b);

        // 2. Standalone SDFB exterior: colours change in expected regions while geometry and pick ranges stay identical.
        let ext_base = decode_load_10d_with_emf(&organ_10d, true, None).unwrap();
        let ext_a = decode_load_10d_with_emf(&organ_10d, true, Some(&bound_a)).unwrap();
        let ext_b = decode_load_10d_with_emf(&organ_10d, true, Some(&bound_b)).unwrap();

        assert!(ext_base.colors.is_empty());
        assert_eq!(ext_a.positions, ext_base.positions);
        assert_eq!(ext_b.positions, ext_base.positions);
        assert_eq!(ext_a.indices, ext_base.indices);
        assert_eq!(ext_b.indices, ext_base.indices);
        assert_eq!(ext_a.ranges, ext_base.ranges);
        assert_eq!(ext_b.ranges, ext_base.ranges);
        assert_eq!(ext_a.colors.len(), ext_a.positions.len());
        assert_eq!(ext_b.colors.len(), ext_b.positions.len());
        assert!(ext_a.materials.iter().all(|m| m.multiply_vertex_color));
        assert!(ext_b.materials.iter().all(|m| m.multiply_vertex_color));

        // Check right-side (+x) vs left-side (-x) exterior vertices:
        let mut right_red_dominates_in_a = 0usize;
        let mut left_blue_dominates_in_b = 0usize;
        for (i, pos) in ext_a.positions.iter().enumerate() {
            let ca = ext_a.colors[i];
            let cb = ext_b.colors[i];
            if pos[0] > 0.35 && pos[1].abs() < 0.35 && pos[2].abs() < 0.35 {
                assert!(
                    ca[0] > cb[0] && ca[0] > ca[2],
                    "Right wall (+x) must be redder under Companion A (620nm at +x) than B: ca={ca:?}, cb={cb:?}"
                );
                right_red_dominates_in_a += 1;
            } else if pos[0] < -0.35 && pos[1].abs() < 0.35 && pos[2].abs() < 0.35 {
                assert!(
                    cb[2] > ca[2] && cb[2] > cb[0],
                    "Left wall (-x) must be bluer under Companion B (465nm at -x) than A: ca={ca:?}, cb={cb:?}"
                );
                left_blue_dominates_in_b += 1;
            }
        }
        assert!(right_red_dominates_in_a > 0);
        assert!(left_blue_dominates_in_b > 0);

        // 3. Standalone SDFB cutaway: exterior and cut tissue wall reflect the optical field while
        // signed-distance geometry, lumen triangle count, lumen material, and pick IDs stay unchanged.
        let plane = SectionPlane::new([0.0, 0.0, 1.0, 0.0], 0.0).unwrap();
        let cut_base = decode_sdfb_cutaway(&organ_10d, plane).unwrap();
        let cut_a = decode_sdfb_cutaway_with_emf(&organ_10d, plane, Some(&bound_a)).unwrap();
        let cut_b = decode_sdfb_cutaway_with_emf(&organ_10d, plane, Some(&bound_b)).unwrap();

        assert_eq!(cut_a.mesh.positions, cut_base.mesh.positions);
        assert_eq!(cut_b.mesh.positions, cut_base.mesh.positions);
        assert_eq!(cut_a.mesh.triangles, cut_base.mesh.triangles);
        assert_eq!(cut_b.mesh.triangles, cut_base.mesh.triangles);
        assert_eq!(cut_a.ranges, cut_base.ranges);
        assert_eq!(cut_b.ranges, cut_base.ranges);
        assert_eq!(cut_a.regions, cut_base.regions);
        assert_eq!(cut_b.regions, cut_base.regions);
        assert_eq!(cut_a.semantic_iris, cut_base.semantic_iris);

        let mut cut_wall_changed = 0usize;
        let mut uncut_changed = 0usize;
        let mut lumen_count = 0usize;
        for (range, &region) in cut_a.ranges.iter().zip(&cut_a.regions) {
            let tri_start = (range.first_index / 3) as usize;
            let tri_end = tri_start + (range.index_count / 3) as usize;
            for tri in &cut_a.mesh.triangles[tri_start..tri_end] {
                for &v in tri {
                    let va = cut_a.colors[v as usize];
                    let vb = cut_b.colors[v as usize];
                    match region {
                        PickSurfaceRegion::UncutSurface => {
                            if (va[0] - vb[0]).abs() > 0.05 || (va[2] - vb[2]).abs() > 0.05 {
                                uncut_changed += 1;
                            }
                        }
                        PickSurfaceRegion::CutTissueWall => {
                            if (va[0] - vb[0]).abs() > 0.05 || (va[2] - vb[2]).abs() > 0.05 {
                                cut_wall_changed += 1;
                            }
                        }
                        PickSurfaceRegion::InternalCavityLumen => {
                            assert_eq!(va, [1.0, 1.0, 1.0, 1.0]);
                            assert_eq!(vb, [1.0, 1.0, 1.0, 1.0]);
                            lumen_count += 1;
                        }
                    }
                }
            }
        }
        assert!(uncut_changed > 20, "Exterior cutaway vertices must change colour");
        assert!(cut_wall_changed > 10, "Newly exposed cut tissue wall vertices must change colour");
        assert!(lumen_count > 6, "Positive lumen aperture must be present and distinct from cut tissue");

        // 4. Transformed two-leaf HMC assembly (both exterior and cutaway):
        let leaf_right_10d = build_sdfb_hollow_organ_10d(
            "urn:test:organ:leaf-right",
            "urn:test:organ:leaf-right-wall",
            "urn:test:organ:leaf-right-lumen",
        );
        let assembly_10d = build_two_leaf_assembly_10d(
            "urn:test:assembly:pelvis",
            "outputs/bladder.10d",
            &organ_10d,
            "urn:test:class:Bladder",
            "urn:test:instance:bladder-0",
            -4.5,
            "outputs/leaf_right.10d",
            &leaf_right_10d,
            "urn:test:class:RightOrgan",
            "urn:test:instance:right-1",
            4.5,
        );

        let make_hmc_with_companion = |emf_bytes: &[u8], wl_nm: f64| -> Vec<u8> {
            let geom_hex: String = organ_sha.iter().map(|b| format!("{b:02x}")).collect();
            let field_sha: [u8; 32] = Sha256::digest(emf_bytes).into();
            let field_hex: String = field_sha.iter().map(|b| format!("{b:02x}")).collect();
            let ttl = format!(
                "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
                 @prefix appearance: <urn:qualia:anatomy:atlas:appearance:> .\n\
                 <urn:test:organ:bladder#companion>\n\
                     a appearance:IllustrativeFieldCompanion ;\n\
                     appearance:geometryMember \"outputs/bladder.10d\" ;\n\
                     appearance:fieldMember \"outputs/bladder.emf\" ;\n\
                     appearance:geometrySha256 \"{geom_hex}\" ;\n\
                     appearance:fieldSha256 \"{field_hex}\" ;\n\
                     appearance:units \"metres\" ;\n\
                     appearance:authoredWavelengthNm \"{wl_nm}\"^^xsd:decimal .\n"
            );
            let cancel = AtomicBool::new(false);
            let packed = pack_semantic_hmc(
                &[
                    HmcPackAsset {
                        key: "outputs/assembly.10d",
                        kind: "10d",
                        bytes: &assembly_10d,
                        metadata: None,
                        salience: Some(HmcSalience::A),
                    },
                    HmcPackAsset {
                        key: "outputs/bladder.10d",
                        kind: "10d",
                        bytes: &organ_10d,
                        metadata: None,
                        salience: Some(HmcSalience::B),
                    },
                    HmcPackAsset {
                        key: "outputs/leaf_right.10d",
                        kind: "10d",
                        bytes: &leaf_right_10d,
                        metadata: None,
                        salience: Some(HmcSalience::B),
                    },
                    HmcPackAsset {
                        key: "outputs/bladder.emf",
                        kind: "emf",
                        bytes: emf_bytes,
                        metadata: None,
                        salience: Some(HmcSalience::B),
                    },
                    HmcPackAsset {
                        key: "outputs/semantics/spectral-registration.ttl",
                        kind: "ttl",
                        bytes: ttl.as_bytes(),
                        metadata: None,
                        salience: Some(HmcSalience::C),
                    },
                ],
                None,
                TEST_PACK_LIMITS,
                &cancel,
            )
            .unwrap();
            packed.hmc
        };

        let hmc_a = make_hmc_with_companion(&emf_a_bytes, 620.0);
        let hmc_b = make_hmc_with_companion(&emf_b_bytes, 465.0);
        let bundle_a = BundleReader::parse(&hmc_a).unwrap();
        let bundle_b = BundleReader::parse(&hmc_b).unwrap();

        let digest_a =
            assembly_residency_digest(&bundle_a, &assembly_10d, Sha256::digest(&assembly_10d).into())
                .unwrap()
                .unwrap();
        let digest_b =
            assembly_residency_digest(&bundle_b, &assembly_10d, Sha256::digest(&assembly_10d).into())
                .unwrap()
                .unwrap();
        assert_ne!(
            digest_a, digest_b,
            "Assembly residency digest must invalidate when registered companion changes"
        );

        let asm_ext_a =
            decode_hmc_leaf_assembly_with_emf(&bundle_a, "outputs/assembly.10d", None).unwrap();
        let asm_ext_b =
            decode_hmc_leaf_assembly_with_emf(&bundle_b, "outputs/assembly.10d", None).unwrap();
        assert_eq!(asm_ext_a.positions, asm_ext_b.positions);
        assert_eq!(asm_ext_a.indices, asm_ext_b.indices);
        assert_eq!(asm_ext_a.instances.len(), 2);
        assert_eq!(
            asm_ext_a.instances[0].semantic_id(),
            asm_ext_b.instances[0].semantic_id()
        );
        assert_eq!(
            asm_ext_a.instances[1].semantic_id(),
            asm_ext_b.instances[1].semantic_id()
        );
        assert_ne!(asm_ext_a.colors, asm_ext_b.colors);

        let asm_cut_a = decode_hmc_leaf_cutaway_with_emf(
            &bundle_a,
            "outputs/assembly.10d",
            plane,
            &[],
            None,
        )
        .unwrap();
        let asm_cut_b = decode_hmc_leaf_cutaway_with_emf(
            &bundle_b,
            "outputs/assembly.10d",
            plane,
            &[],
            None,
        )
        .unwrap();
        assert_eq!(asm_cut_a.draw.positions, asm_cut_b.draw.positions);
        assert_eq!(asm_cut_a.draw.indices, asm_cut_b.draw.indices);
        assert_eq!(asm_cut_a.cut_wall_triangles, asm_cut_b.cut_wall_triangles);
        assert_eq!(asm_cut_a.lumen_triangles, asm_cut_b.lumen_triangles);
        assert_eq!(asm_cut_a.pick_metadata, asm_cut_b.pick_metadata);
        assert_ne!(
            asm_cut_a.draw.colors, asm_cut_b.draw.colors,
            "Assembly cutaway vertex colours must update when companion changes"
        );

        // 5. Offscreen GPU framebuffer pixel verification when a hardware/driver adapter is available:
        #[cfg(feature = "gpu-runtime")]
        if crate::wgsl_forge::test_gpu_available() {
            if let Ok(mut gpu) = crate::render::gpu::PortalGpu::new_offscreen(64, 64, 0) {
                gpu.set_clear_color(0.0, 0.0, 0.0, 1.0);
                gpu.set_lighting(0.0, 0.0, 1.0, 0.25, 1.0);
                gpu.set_screen_space_ao_enabled(false);
                let capture = |gpu: &mut crate::render::gpu::PortalGpu| -> Vec<u8> {
                    gpu.render(0.0, &crate::render::telemetry::SystemTelemetry::default())
                        .unwrap();
                    let mut rgba = vec![0u8; gpu.required_rgba8_bytes()];
                    gpu.read_rgba8_into(&mut rgba).unwrap();
                    rgba
                };

                crate::render::portal_load_10d::upload_load_10d_to_gpu(&mut gpu, &ext_a).unwrap();
                let fb_ext_a = capture(&mut gpu);
                crate::render::portal_load_10d::upload_load_10d_to_gpu(&mut gpu, &ext_b).unwrap();
                let fb_ext_b = capture(&mut gpu);
                assert_ne!(
                    fb_ext_a, fb_ext_b,
                    "Rendered offscreen exterior framebuffer pixels must change when swapping equal-magnitude companions"
                );

                let counts_cut_a = gpu
                    .set_section_plane_with_emf(&organ_10d, plane, Some(&bound_a))
                    .unwrap();
                let fb_cut_a = capture(&mut gpu);
                let counts_cut_b = gpu
                    .set_section_plane_with_emf(&organ_10d, plane, Some(&bound_b))
                    .unwrap();
                let fb_cut_b = capture(&mut gpu);
                assert_eq!(counts_cut_a, counts_cut_b);
                assert_ne!(
                    fb_cut_a, fb_cut_b,
                    "Rendered offscreen cutaway framebuffer pixels must change when swapping equal-magnitude companions"
                );

                let (asm_counts_a, occ_a) = gpu
                    .set_hmc_section_plane_with_emf(&bundle_a, "outputs/assembly.10d", plane, &[], None)
                    .unwrap();
                let fb_asm_a = capture(&mut gpu);
                let (asm_counts_b, occ_b) = gpu
                    .set_hmc_section_plane_with_emf(&bundle_b, "outputs/assembly.10d", plane, &[], None)
                    .unwrap();
                let fb_asm_b = capture(&mut gpu);
                assert_eq!(asm_counts_a, asm_counts_b);
                assert_eq!(occ_a, occ_b);
                assert_ne!(
                    fb_asm_a, fb_asm_b,
                    "Rendered offscreen assembly cutaway framebuffer pixels must change when swapping equal-magnitude companions"
                );
            }
        }
    }

    #[test]
    fn fail_closed_on_missing_or_corrupt_companions_and_invalid_transforms() {
        let organ_10d = build_sdfb_hollow_organ_10d(
            "urn:test:organ:bladder",
            "urn:test:organ:bladder-wall",
            "urn:test:organ:bladder-lumen",
        );
        let organ_sha: [u8; 32] = Sha256::digest(&organ_10d).into();
        let emf_bytes = build_equal_magnitude_emf_v4([-3.5, -3.5, -3.5], [2.5, 0.0, 0.0], 620.0);
        let emf = decode_emf_scientific(&emf_bytes).unwrap();

        // 1. Singular transform rejected
        let mut singular_reg = EmfCompanionRegistration::identity_metres(
            "outputs/bladder.10d",
            "outputs/bladder.emf",
        );
        singular_reg.field_from_leaf = [[0.0; 4]; 4];
        assert!(BoundLeafEmfCompanion::bind(singular_reg, organ_sha, &emf, 0).is_err());

        // 2. Non-finite transform rejected
        let mut nan_reg = EmfCompanionRegistration::identity_metres(
            "outputs/bladder.10d",
            "outputs/bladder.emf",
        );
        nan_reg.field_from_leaf[3][0] = f64::NAN;
        assert!(BoundLeafEmfCompanion::bind(nan_reg, organ_sha, &emf, 0).is_err());

        // 3. Non-metre units rejected
        let mut bad_units = EmfCompanionRegistration::identity_metres(
            "outputs/bladder.10d",
            "outputs/bladder.emf",
        );
        bad_units.units = String::from("inches");
        assert!(BoundLeafEmfCompanion::bind(bad_units, organ_sha, &emf, 0).is_err());

        // 4. Digest mismatch rejected
        let mut wrong_sha = EmfCompanionRegistration::identity_metres(
            "outputs/bladder.10d",
            "outputs/bladder.emf",
        );
        wrong_sha.field_sha256 = Some([0xAA; 32]);
        assert!(BoundLeafEmfCompanion::bind(wrong_sha, organ_sha, &emf, 0).is_err());

        // 5. Missing companion member in HMC bundle rejected
        let ttl_missing = "@prefix appearance: <urn:qualia:anatomy:atlas:appearance:> .\n\
            <urn:test:c> a appearance:IllustrativeFieldCompanion ;\n\
            appearance:geometryMember \"outputs/bladder.10d\" ;\n\
            appearance:fieldMember \"outputs/missing.emf\" .\n";
        let mut writer = BundleWriter::new();
        writer
            .add_file("outputs/bladder.10d", "10d", organ_10d.clone(), None)
            .unwrap();
        writer
            .add_file("outputs/reg.ttl", "ttl", ttl_missing.as_bytes().to_vec(), None)
            .unwrap();
        let hmc_missing = writer.build().unwrap();
        let bundle_missing = BundleReader::parse(&hmc_missing).unwrap();
        assert!(resolve_bundle_emf_registrations(&bundle_missing).is_err());

        // 6. Corrupt EMF companion bytes rejected
        let mut corrupt_emf = emf_bytes.clone();
        let last = corrupt_emf.len() - 1;
        corrupt_emf[last] ^= 0xFF;
        assert!(decode_emf_scientific(&corrupt_emf).is_err());
    }
}

