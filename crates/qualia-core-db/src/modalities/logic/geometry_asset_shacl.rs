//! Geometry-asset SHACL — the runtime half of `shapes/geometry-asset.shacl.ttl` (the normative
//! declarative source; see `docs/manuals/standards/geometry-asset-ontology.md` §5).
//!
//! Two halves, honestly separated:
//! 1. **Per-property bounds** → [`GeometryAssetConfiguration::to_opcodes`], SLG-VM opcodes matching the
//!    `.ttl` `geo:MeshShape` (counts ≤ 2²², `sourceFormat`/`unit` ∈ an allowed set). This is the same
//!    `Configuration → to_opcodes` pattern as `specialized_libs_shacl.rs`.
//! 2. **Cross-property (relational) constraints** → [`validate_geometry_manifest`]. Plain per-property
//!    SHACL cannot express "bbox not inverted", "every index < vertexCount", or "compiledDigest ==
//!    the real `.10d` CRC" — they need computation over several facts at once. The `.ttl` explicitly
//!    defers these to this shim; here they are real checks, not comments.

use crate::governance::webizen::SlgOpcode;
use crate::q_hash;

/// The `.10d` container's `MAX_VERTEX_COUNT` / `MAX_TRIANGLE_COUNT` (2²²) — the malicious-size guard.
pub const MAX_GEOMETRY_COUNT: u32 = 1 << 22;

/// `q42:GeometryAssetConfiguration` — the per-property bounds for a compiled geometry-asset manifest.
#[derive(Debug, Clone)]
pub struct GeometryAssetConfiguration {
    pub max_vertex_count: u32,
    pub max_triangle_count: u32,
    pub allowed_source_formats: Vec<String>,
    pub allowed_units: Vec<String>,
    pub allowed_licences: Vec<String>,
    /// Sensitivity classes, least→most restrictive (index = rank). Absent/unknown ⇒ most restrictive.
    pub sensitivity_ladder: Vec<String>,
}

impl Default for GeometryAssetConfiguration {
    fn default() -> Self {
        let owned = |xs: &[&str]| xs.iter().map(|s| s.to_string()).collect();
        Self {
            max_vertex_count: MAX_GEOMETRY_COUNT,
            max_triangle_count: MAX_GEOMETRY_COUNT,
            allowed_source_formats: owned(&["obj", "stl", "glb", "gltf"]),
            allowed_units: owned(&["metre", "millimetre", "centimetre", "inch", "dimensionless"]),
            allowed_licences: owned(&["CC0", "CC-BY", "CC-BY-SA", "ODC-PDDL", "MIT", "Apache-2.0"]),
            sensitivity_ladder: owned(&["Public", "Restricted", "Classified", "Sanctuary"]),
        }
    }
}

impl GeometryAssetConfiguration {
    /// The per-property SHACL constraints as SLG-VM opcodes: `vertexCount`/`triangleCount` in
    /// `[1, max]`, and each allowed `sourceFormat`/`unit` value as a `CheckHasValue` (the `sh:in` set).
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        let mut ops = vec![
            SlgOpcode::CheckMinInclusive(1.0),
            SlgOpcode::CheckMaxInclusive(self.max_vertex_count as f64),
            SlgOpcode::CheckMaxInclusive(self.max_triangle_count as f64),
        ];
        for fmt in &self.allowed_source_formats {
            ops.push(SlgOpcode::CheckHasValue(q_hash(fmt)));
        }
        for unit in &self.allowed_units {
            ops.push(SlgOpcode::CheckHasValue(q_hash(unit)));
        }
        for licence in &self.allowed_licences {
            ops.push(SlgOpcode::CheckHasValue(q_hash(licence)));
        }
        ops
    }

    /// Rank of a sensitivity class on the ladder (higher = more restrictive). An unknown/absent class
    /// is treated **fail-closed** as the most restrictive rank — you cannot down-classify by mislabelling.
    fn sensitivity_rank(&self, class: &str) -> usize {
        self.sensitivity_ladder
            .iter()
            .position(|c| c == class)
            .unwrap_or(self.sensitivity_ladder.len().saturating_sub(1))
    }
}

/// The relational facts of a compiled geometry asset that plain per-property SHACL cannot see.
#[derive(Debug, Clone)]
pub struct GeometryManifestFacts<'a> {
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub source_format: &'a str,
    pub unit: &'a str,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    /// The largest triangle vertex index used, if indices are being checked.
    pub max_triangle_index: Option<u32>,
    /// The `compiledDigest` asserted by the manifest.
    pub claimed_compiled_digest: u32,
    /// The actual whole-file CRC-32C of the `.10d` container the manifest describes
    /// (recompute with `render::compile_10d::compiled_digest`).
    pub actual_container_crc32c: u32,
    /// Sensitivity classes of the inputs this asset was derived from (for the high-water-mark).
    pub input_sensitivities: &'a [&'a str],
    /// The sensitivity class declared on this asset (if any).
    pub declared_sensitivity: Option<&'a str>,
    pub licence: &'a str,
    pub creator: Option<&'a str>,
    pub valid_from: Option<u64>,
    pub valid_until: Option<u64>,
}

/// A geometry-asset constraint violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeometryConstraintViolation {
    VertexCountOutOfRange {
        count: u32,
        max: u32,
    },
    TriangleCountOutOfRange {
        count: u32,
        max: u32,
    },
    UnknownSourceFormat(String),
    UnknownUnit(String),
    UnknownLicence(String),
    /// A bbox coordinate is NaN/∞ — no unit-bearing geometry may carry a non-finite box.
    NonFiniteBbox,
    /// `bboxMin[axis] > bboxMax[axis]` — an inverted (impossible) box.
    InvertedBbox {
        axis: u8,
    },
    /// A triangle references a vertex index `>= vertexCount`.
    IndexOutOfBounds {
        max_index: u32,
        vertex_count: u32,
    },
    /// The manifest's `compiledDigest` does not equal the real `.10d` whole-file CRC — the manifest
    /// does not describe the container it claims to.
    CompiledDigestMismatch {
        claimed: u32,
        actual: u32,
    },
    /// A derived asset declared a **less** restrictive sensitivity than one of its inputs — the
    /// high-water-mark forbids down-classifying derived geometry.
    SensitivityDowngraded {
        declared: String,
        required: String,
    },
    /// validFrom > validUntil
    TemporalValidityInverted {
        from: u64,
        until: u64,
    },
}

/// Validate the relational constraints of a compiled geometry-asset manifest. Empty result = valid.
/// This is the load-bearing check the declarative `.ttl` cannot perform.
pub fn validate_geometry_manifest(
    facts: &GeometryManifestFacts,
    cfg: &GeometryAssetConfiguration,
) -> Vec<GeometryConstraintViolation> {
    use GeometryConstraintViolation::*;
    let mut v = Vec::new();

    // Counts in [1, max].
    if facts.vertex_count < 1 || facts.vertex_count > cfg.max_vertex_count {
        v.push(VertexCountOutOfRange {
            count: facts.vertex_count,
            max: cfg.max_vertex_count,
        });
    }
    if facts.triangle_count < 1 || facts.triangle_count > cfg.max_triangle_count {
        v.push(TriangleCountOutOfRange {
            count: facts.triangle_count,
            max: cfg.max_triangle_count,
        });
    }

    // Format / unit / licence membership (the sh:in sets).
    if !cfg
        .allowed_source_formats
        .iter()
        .any(|s| s == facts.source_format)
    {
        v.push(UnknownSourceFormat(facts.source_format.to_string()));
    }
    if !cfg.allowed_units.iter().any(|s| s == facts.unit) {
        v.push(UnknownUnit(facts.unit.to_string()));
    }
    if !cfg.allowed_licences.iter().any(|s| s == facts.licence) {
        v.push(UnknownLicence(facts.licence.to_string()));
    }

    // Bbox finite and non-inverted.
    let finite = facts
        .bbox_min
        .iter()
        .chain(facts.bbox_max.iter())
        .all(|x| x.is_finite());
    if !finite {
        v.push(NonFiniteBbox);
    } else {
        for axis in 0..3 {
            if facts.bbox_min[axis] > facts.bbox_max[axis] {
                v.push(InvertedBbox { axis: axis as u8 });
            }
        }
    }

    // Every triangle index < vertexCount.
    if let Some(max_index) = facts.max_triangle_index {
        if max_index >= facts.vertex_count {
            v.push(IndexOutOfBounds {
                max_index,
                vertex_count: facts.vertex_count,
            });
        }
    }

    // The manifest cites its real container.
    if facts.claimed_compiled_digest != facts.actual_container_crc32c {
        v.push(CompiledDigestMismatch {
            claimed: facts.claimed_compiled_digest,
            actual: facts.actual_container_crc32c,
        });
    }

    // Sensitivity high-water-mark: declared ≥ the most-restrictive input.
    if let Some(declared) = facts.declared_sensitivity {
        if let Some(required) = facts
            .input_sensitivities
            .iter()
            .max_by_key(|c| cfg.sensitivity_rank(c))
        {
            if cfg.sensitivity_rank(declared) < cfg.sensitivity_rank(required) {
                v.push(SensitivityDowngraded {
                    declared: declared.to_string(),
                    required: required.to_string(),
                });
            }
        }
    }

    // Temporal validity checks
    if let (Some(from), Some(until)) = (facts.valid_from, facts.valid_until) {
        if from > until {
            v.push(TemporalValidityInverted { from, until });
        }
    }

    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_facts() -> GeometryManifestFacts<'static> {
        GeometryManifestFacts {
            vertex_count: 3,
            triangle_count: 1,
            source_format: "glb",
            unit: "metre",
            bbox_min: [0.0, 0.0, 0.0],
            bbox_max: [1.0, 1.0, 1.0],
            max_triangle_index: Some(2),
            claimed_compiled_digest: 0xDEAD_BEEF,
            actual_container_crc32c: 0xDEAD_BEEF,
            input_sensitivities: &["Restricted"],
            declared_sensitivity: Some("Classified"),
            licence: "CC-BY",
            creator: Some("did:qualia:test_creator"),
            valid_from: Some(100),
            valid_until: Some(200),
        }
    }

    #[test]
    fn a_well_formed_manifest_passes() {
        assert!(
            validate_geometry_manifest(&valid_facts(), &GeometryAssetConfiguration::default())
                .is_empty()
        );
    }

    #[test]
    fn to_opcodes_emits_bounds_and_membership() {
        let ops = GeometryAssetConfiguration::default().to_opcodes();
        // min(1) + 2×max + 4 formats + 5 units + 6 licences = 18 opcodes.
        assert_eq!(ops.len(), 18);
        assert!(ops.iter().any(
            |o| matches!(o, SlgOpcode::CheckMaxInclusive(m) if *m == MAX_GEOMETRY_COUNT as f64)
        ));
        assert!(ops.contains(&SlgOpcode::CheckHasValue(q_hash("glb"))));
        assert!(ops.contains(&SlgOpcode::CheckHasValue(q_hash("metre"))));
        assert!(ops.contains(&SlgOpcode::CheckHasValue(q_hash("CC0"))));
    }

    #[test]
    fn counts_out_of_range_are_caught() {
        let cfg = GeometryAssetConfiguration::default();
        let mut f = valid_facts();
        f.vertex_count = 0;
        f.triangle_count = MAX_GEOMETRY_COUNT + 1;
        let v = validate_geometry_manifest(&f, &cfg);
        assert!(
            v.contains(&GeometryConstraintViolation::VertexCountOutOfRange {
                count: 0,
                max: MAX_GEOMETRY_COUNT
            })
        );
        assert!(v.iter().any(|x| matches!(
            x,
            GeometryConstraintViolation::TriangleCountOutOfRange { .. }
        )));
    }

    #[test]
    fn unknown_format_and_unit_and_licence_are_caught() {
        let cfg = GeometryAssetConfiguration::default();
        let mut f = valid_facts();
        f.source_format = "fbx";
        f.unit = "cubits";
        f.licence = "proprietary";
        let v = validate_geometry_manifest(&f, &cfg);
        assert!(
            v.contains(&GeometryConstraintViolation::UnknownSourceFormat(
                "fbx".to_string()
            ))
        );
        assert!(v.contains(&GeometryConstraintViolation::UnknownUnit(
            "cubits".to_string()
        )));
        assert!(v.contains(&GeometryConstraintViolation::UnknownLicence(
            "proprietary".to_string()
        )));
    }

    #[test]
    fn inverted_and_nonfinite_bbox_are_caught() {
        let cfg = GeometryAssetConfiguration::default();
        let mut f = valid_facts();
        f.bbox_min = [0.0, 5.0, 0.0]; // y min > y max
        f.bbox_max = [1.0, 1.0, 1.0];
        assert!(validate_geometry_manifest(&f, &cfg)
            .contains(&GeometryConstraintViolation::InvertedBbox { axis: 1 }));

        let mut g = valid_facts();
        g.bbox_max = [f32::NAN, 1.0, 1.0];
        assert!(validate_geometry_manifest(&g, &cfg)
            .contains(&GeometryConstraintViolation::NonFiniteBbox));
    }

    #[test]
    fn out_of_bounds_index_is_caught() {
        let cfg = GeometryAssetConfiguration::default();
        let mut f = valid_facts();
        f.max_triangle_index = Some(3); // == vertex_count (3) → OOB
        assert!(validate_geometry_manifest(&f, &cfg).contains(
            &GeometryConstraintViolation::IndexOutOfBounds {
                max_index: 3,
                vertex_count: 3
            }
        ));
    }

    #[test]
    fn a_manifest_that_lies_about_its_container_is_caught() {
        let cfg = GeometryAssetConfiguration::default();
        let mut f = valid_facts();
        f.claimed_compiled_digest = 0x0000_0001; // manifest cites a different container
        assert!(validate_geometry_manifest(&f, &cfg).contains(
            &GeometryConstraintViolation::CompiledDigestMismatch {
                claimed: 1,
                actual: 0xDEAD_BEEF
            }
        ));
    }

    #[test]
    fn sensitivity_cannot_be_downgraded_below_an_input() {
        let cfg = GeometryAssetConfiguration::default();
        let mut f = valid_facts();
        f.input_sensitivities = &["Public", "Classified"]; // most restrictive input = Classified
        f.declared_sensitivity = Some("Restricted"); // below Classified → downgrade
        assert!(validate_geometry_manifest(&f, &cfg).contains(
            &GeometryConstraintViolation::SensitivityDowngraded {
                declared: "Restricted".to_string(),
                required: "Classified".to_string(),
            }
        ));
        // Declaring at-or-above the high-water-mark is fine.
        f.declared_sensitivity = Some("Sanctuary");
        assert!(!validate_geometry_manifest(&f, &cfg)
            .iter()
            .any(|x| matches!(x, GeometryConstraintViolation::SensitivityDowngraded { .. })));
    }

    #[test]
    fn inverted_temporal_validity_is_caught() {
        let cfg = GeometryAssetConfiguration::default();
        let mut f = valid_facts();
        f.valid_from = Some(200);
        f.valid_until = Some(100);
        assert!(validate_geometry_manifest(&f, &cfg).contains(
            &GeometryConstraintViolation::TemporalValidityInverted {
                from: 200,
                until: 100
            }
        ));
    }
}
