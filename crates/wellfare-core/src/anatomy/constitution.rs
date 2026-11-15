//! The person's **body constitution** — measurements, characteristics, and attributes they declare
//! about their own body, and the illustrative [`BodyFit`] that maps those onto a Visible Human /
//! CCF reference mesh.
//!
//! This is forum-internum / Sanctuary-class selfhood content. Every field is optional: the person
//! declares what they know. Absence is not an assumption. The fit is a **hypothesis overlay** on a
//! public reference body, never a scan of them and never a diagnosis.
//!
//! Geometry stays on the reference atlas. Constitution is a view transform (scale, regional
//! stretch, hide declared-absent parts, coarse pregnancy bulge). It does not rewrite `.10d` files.

use serde::{Deserialize, Serialize};

use super::knowledge_context::SubjectKnowledgeContext;
use super::model::{AnatomyModel, Karyotype, normalize_organ_key};
use super::physiology::Trimester;

/// Visible Human Male standing height used by the CCF / HRA male reference set.
/// NLM Visible Human Project (male donor). Seed reference, not a population mean.
pub const VH_MALE_STATURE_MM: u16 = 1800;
/// Visible Human Female standing height used by the CCF / HRA female reference set.
pub const VH_FEMALE_STATURE_MM: u16 = 1676;

/// Typical sitting-height / stature ratio on the adult reference (seed).
pub const REF_SITTING_RATIO: f32 = 0.52;
/// Typical inseam / stature ratio on the adult reference (seed).
pub const REF_INSEAM_RATIO: f32 = 0.45;

pub const AGE_MONTHS_RANGE: (u16, u16) = (0, 1560);

pub use super::measurements::BodyMeasurements;

/// Side the person names as dominant, if they name one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DominantSide {
    Left,
    Right,
    Ambidextrous,
}

/// Relatively stable facts the person declares about their body (not a measurement).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyCharacteristics {
    /// Chromosomal basis they declare — selects the CCF male/female reference set.
    /// Biological-substrate attribute, not a gender or identity claim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub karyotype: Option<Karyotype>,
    /// Age in whole months (a newborn is 0; 18 years is 216).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age_months: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dominant_side: Option<DominantSide>,
}

impl BodyCharacteristics {
    ///  when empty.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The reference mesh set this declaration selects, if they named a karyotype.
    pub fn anatomy_model(&self) -> Option<AnatomyModel> {
        self.karyotype.map(Karyotype::anatomy_model)
    }
}

/// Why a named structure is not present on *this* person's body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbsenceReason {
    Surgical,
    Congenital,
    Amputation,
    PersonDeclared,
}

/// A structure the person says is not on their body (so the reference mesh is hidden).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsentPart {
    /// Organ / part key (`uterus`, `prostate`, `left-leg`, or a CCF token).
    pub key: String,
    pub reason: AbsenceReason,
}

/// Person-authored attributes that change which parts are shown, or apply a named
/// coarse shape (pregnancy abdomen). Not a diagnosis.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyAttributes {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub absent: Vec<AbsentPart>,
    /// Optional pregnancy abdomen hint. Prefer the physiological-state declaration
    /// when both exist — this is a geometry-only override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pregnancy: Option<Trimester>,
    /// The person's own words. Stored, never parsed into geometry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Declared appearance. Independent of ethnicity. Painted only when those meshes exist (W16).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eye_colour: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hair_colour: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skin_tone: Option<String>,
}

impl BodyAttributes {
    ///  when empty.
    pub fn is_empty(&self) -> bool {
        self.absent.is_empty()
            && self.pregnancy.is_none()
            && self.notes.is_none()
            && self.eye_colour.is_none()
            && self.hair_colour.is_none()
            && self.skin_tone.is_none()
    }
}

/// The full constitution the person may author. Empty = identity fit (reference body as published).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyConstitution {
    #[serde(default)]
    pub measurements: BodyMeasurements,
    #[serde(default)]
    pub characteristics: BodyCharacteristics,
    #[serde(default)]
    pub attributes: BodyAttributes,
    /// Ethnicity / imported ancestry. Knowledge context only — never a fit input.
    #[serde(default, skip_serializing_if = "SubjectKnowledgeContext::is_empty")]
    pub knowledge: SubjectKnowledgeContext,
}

impl BodyConstitution {
    ///  when empty.
    pub fn is_empty(&self) -> bool {
        self.measurements.is_empty()
            && self.characteristics.is_empty()
            && self.attributes.is_empty()
            && self.knowledge.is_empty()
    }

    /// Reject values outside the soft clamps. The person can still clear a field.
    pub fn validate(&self) -> Result<(), String> {
        self.measurements.validate()?;
        if let Some(a) = self.characteristics.age_months {
            if a > AGE_MONTHS_RANGE.1 {
                return Err(format!(
                    "age_months {a} is outside 0–{}",
                    AGE_MONTHS_RANGE.1
                ));
            }
        }
        Ok(())
    }

    /// The reference stature for the mesh set this constitution will be fitted onto.
    pub fn reference_stature_mm(&self) -> u16 {
        match self.characteristics.anatomy_model() {
            Some(AnatomyModel::Female) => VH_FEMALE_STATURE_MM,
            _ => VH_MALE_STATURE_MM,
        }
    }

    /// Illustrative seed girths (mm) for the chosen reference. Not measured on the GLB.
    pub fn reference_girths_mm(&self) -> ReferenceGirths {
        match self.characteristics.anatomy_model() {
            Some(AnatomyModel::Female) => ReferenceGirths {
                chest_mm: 940,
                waist_mm: 780,
                hip_mm: 1000,
                shoulder_width_mm: 360,
            },
            _ => ReferenceGirths {
                chest_mm: 1000,
                waist_mm: 900,
                hip_mm: 980,
                shoulder_width_mm: 410,
            },
        }
    }

    /// Compute the view transform. Does not invent a height from age.
    pub fn fit(&self) -> BodyFit {
        self.fit_with_pregnancy(self.attributes.pregnancy)
    }

    /// Same as [`Self::fit`], but a caller can pass the declared physiological
    /// trimester so pregnancy geometry and the continuum stay one source of truth.
    pub fn fit_with_pregnancy(&self, pregnancy: Option<Trimester>) -> BodyFit {
        let ref_h = self.reference_stature_mm() as f32;
        let girth = self.reference_girths_mm();
        let mut used: Vec<String> = Vec::new();
        let mut notes: Vec<String> = Vec::new();

        let stature_scale = if let Some(h) = self.measurements.stature_mm {
            used.push("stature_mm".into());
            (h as f32 / ref_h).clamp(0.25, 2.4)
        } else {
            notes.push(
                "No standing height declared — the body stays at Visible Human scale.".into(),
            );
            1.0
        };

        let mut torso_scale_y = 1.0;
        let mut leg_scale_y = 1.0;
        if let (Some(stature), Some(sit)) = (
            self.measurements.stature_mm,
            self.measurements.sitting_height_mm,
        ) {
            used.push("sitting_height_mm".into());
            let person_ratio = sit as f32 / stature as f32;
            torso_scale_y = (person_ratio / REF_SITTING_RATIO).clamp(0.7, 1.35);
            // Keep overall stature: legs absorb the complementary stretch.
            leg_scale_y = ((1.0 - person_ratio) / (1.0 - REF_SITTING_RATIO)).clamp(0.7, 1.45);
        } else if let (Some(stature), Some(inseam)) =
            (self.measurements.stature_mm, self.measurements.inseam_mm)
        {
            used.push("inseam_mm".into());
            let person_ratio = inseam as f32 / stature as f32;
            leg_scale_y = (person_ratio / REF_INSEAM_RATIO).clamp(0.7, 1.45);
            torso_scale_y = ((1.0 - person_ratio) / (1.0 - REF_INSEAM_RATIO)).clamp(0.7, 1.35);
        }

        let arm_span_scale_x = if let (Some(stature), Some(span)) =
            (self.measurements.stature_mm, self.measurements.arm_span_mm)
        {
            used.push("arm_span_mm".into());
            (span as f32 / stature as f32).clamp(0.75, 1.25)
        } else {
            1.0
        };

        let chest_radial = ratio_or_one(
            self.measurements.chest_mm,
            girth.chest_mm,
            &mut used,
            "chest_mm",
        );
        let waist_radial = ratio_or_one(
            self.measurements.waist_mm,
            girth.waist_mm,
            &mut used,
            "waist_mm",
        );
        let hip_radial = ratio_or_one(self.measurements.hip_mm, girth.hip_mm, &mut used, "hip_mm");
        let shoulder_scale_x = ratio_or_one(
            self.measurements.shoulder_width_mm,
            girth.shoulder_width_mm,
            &mut used,
            "shoulder_width_mm",
        );

        if self.measurements.weight_g.is_some() {
            used.push("weight_g".into());
            notes.push(
                "Weight is stored and shown; it does not yet drive a fat/muscle morph.".into(),
            );
        }
        if self.characteristics.age_months.is_some() && self.measurements.stature_mm.is_none() {
            notes.push(
                "Age is stored; height is not guessed from age. Declare standing height to scale the body.".into(),
            );
        }

        let pregnancy_abdomen = match pregnancy.or(self.attributes.pregnancy) {
            Some(Trimester::First) => 0.12,
            Some(Trimester::Second) => 0.28,
            Some(Trimester::Third) => 0.45,
            None => 0.0,
        };
        if pregnancy_abdomen > 0.0 {
            used.push("pregnancy".into());
            notes.push(
                "Pregnancy abdomen is a coarse forward bulge on the midriff, not a model of your uterus.".into(),
            );
        }
        if !self.knowledge.is_empty() {
            notes.push(
                "Ethnicity / ancestry is knowledge context (prevalence, pharmacology, screening hypotheses). It does not change the mesh, skin, hair, or chromosomal reference.".into(),
            );
        }
        if self.attributes.eye_colour.is_some()
            || self.attributes.hair_colour.is_some()
            || self.attributes.skin_tone.is_some()
        {
            notes.push(
                "Eye / hair / skin are declared appearance. They are independent of ethnicity and are not yet painted on the mesh.".into(),
            );
        }
        if self.measurements.has_beyond_anatomy_fit() {
            notes.push(
                "Garment, footwear, helmet, glove, and eyewear measures are stored for those Qualia surfaces. Anatomy fit currently uses height, sitting height, inseam, arm span, shoulders, chest, waist, and hip only.".into(),
            );
        }

        let hidden_keys: Vec<String> = self
            .attributes
            .absent
            .iter()
            .map(|p| normalize_organ_key(&p.key))
            .collect();
        if !hidden_keys.is_empty() {
            used.push("absent_parts".into());
        }

        notes.insert(
            0,
            "Illustrative fit onto a Visible Human / CCF reference body — not a scan of you."
                .into(),
        );

        BodyFit {
            stature_scale,
            torso_scale_y,
            leg_scale_y,
            arm_span_scale_x,
            shoulder_scale_x,
            chest_radial,
            waist_radial,
            hip_radial,
            pregnancy_abdomen,
            pelvis_y_norm: 0.42,
            waist_y_norm: 0.52,
            chest_y_norm: 0.68,
            shoulder_y_norm: 0.78,
            hidden_keys,
            used_fields: used,
            honesty_notes: notes,
            identity: stature_scale == 1.0
                && torso_scale_y == 1.0
                && leg_scale_y == 1.0
                && arm_span_scale_x == 1.0
                && shoulder_scale_x == 1.0
                && chest_radial == 1.0
                && waist_radial == 1.0
                && hip_radial == 1.0
                && pregnancy_abdomen == 0.0,
        }
    }
}

fn ratio_or_one(declared: Option<u16>, reference: u16, used: &mut Vec<String>, field: &str) -> f32 {
    match declared {
        Some(v) if reference > 0 => {
            used.push(field.into());
            (v as f32 / reference as f32).clamp(0.55, 1.8)
        }
        _ => 1.0,
    }
}

/// Seed girths for the chosen Visible Human reference (not measured on the mesh).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceGirths {
    pub chest_mm: u16,
    pub waist_mm: u16,
    pub hip_mm: u16,
    pub shoulder_width_mm: u16,
}

/// Numeric view transform applied to decoded organ vertices in CCF space,
/// *before* the portal's global orbit-frame normalise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyFit {
    pub stature_scale: f32,
    pub torso_scale_y: f32,
    pub leg_scale_y: f32,
    pub arm_span_scale_x: f32,
    pub shoulder_scale_x: f32,
    pub chest_radial: f32,
    pub waist_radial: f32,
    pub hip_radial: f32,
    /// 0 = none; ~0.45 = third-trimester coarse bulge.
    pub pregnancy_abdomen: f32,
    pub pelvis_y_norm: f32,
    pub waist_y_norm: f32,
    pub chest_y_norm: f32,
    pub shoulder_y_norm: f32,
    pub hidden_keys: Vec<String>,
    pub used_fields: Vec<String>,
    pub honesty_notes: Vec<String>,
    /// True when the fit is the identity (no declared geometry).
    pub identity: bool,
}

impl BodyFit {
    /// Identity transform.
    pub fn identity() -> Self {
        Self {
            stature_scale: 1.0,
            torso_scale_y: 1.0,
            leg_scale_y: 1.0,
            arm_span_scale_x: 1.0,
            shoulder_scale_x: 1.0,
            chest_radial: 1.0,
            waist_radial: 1.0,
            hip_radial: 1.0,
            pregnancy_abdomen: 0.0,
            pelvis_y_norm: 0.42,
            waist_y_norm: 0.52,
            chest_y_norm: 0.68,
            shoulder_y_norm: 0.78,
            hidden_keys: Vec::new(),
            used_fields: Vec::new(),
            honesty_notes: vec![
                "No constitution declared — showing the public reference body.".into(),
            ],
            identity: true,
        }
    }

    /// Whether `organ_key` should be omitted from the assembled body.
    pub fn hides(&self, organ_key: &str) -> bool {
        let norm = normalize_organ_key(organ_key);
        self.hidden_keys
            .iter()
            .any(|k| k == &norm || norm.contains(k.as_str()) || k.contains(norm.as_str()))
    }

    /// Apply the fit to one vertex in the body's axis-aligned bounds.
    /// `y` is the CCF up axis (feet → head).
    pub fn transform_point(&self, p: [f32; 3], gmin: [f32; 3], gmax: [f32; 3]) -> [f32; 3] {
        let span_y = (gmax[1] - gmin[1]).max(1e-6);
        let mid_x = (gmin[0] + gmax[0]) * 0.5;
        let mid_z = (gmin[2] + gmax[2]) * 0.5;
        let y_norm = ((p[1] - gmin[1]) / span_y).clamp(0.0, 1.0);

        let mut x = p[0] - mid_x;
        let mut y = p[1];
        let mut z = p[2] - mid_z;

        let y_seg = if y_norm < self.pelvis_y_norm {
            self.leg_scale_y
        } else {
            self.torso_scale_y
        };
        y = gmin[1] + (y - gmin[1]) * y_seg;

        let radial = if y_norm < self.pelvis_y_norm {
            lerp(
                1.0,
                self.hip_radial,
                smoothstep(0.20, self.pelvis_y_norm, y_norm),
            )
        } else if y_norm < self.waist_y_norm {
            lerp(
                self.hip_radial,
                self.waist_radial,
                smoothstep(self.pelvis_y_norm, self.waist_y_norm, y_norm),
            )
        } else if y_norm < self.chest_y_norm {
            lerp(
                self.waist_radial,
                self.chest_radial,
                smoothstep(self.waist_y_norm, self.chest_y_norm, y_norm),
            )
        } else {
            lerp(
                self.chest_radial,
                self.shoulder_scale_x,
                smoothstep(self.chest_y_norm, self.shoulder_y_norm, y_norm),
            )
        };
        x *= radial * self.arm_span_scale_x;
        z *= radial;

        if self.pregnancy_abdomen > 0.0 {
            let band = bump(y_norm, 0.48, 0.10);
            z += self.pregnancy_abdomen * band * span_y * 0.18;
            x *= 1.0 + self.pregnancy_abdomen * band * 0.12;
        }

        [
            mid_x + x * self.stature_scale,
            (gmin[1] + (y - gmin[1]) * self.stature_scale),
            mid_z + z * self.stature_scale,
        ]
    }

    /// Applies in place.
    pub fn apply_in_place(&self, positions: &mut [[f32; 3]], gmin: [f32; 3], gmax: [f32; 3]) {
        if self.identity {
            return;
        }
        for p in positions.iter_mut() {
            *p = self.transform_point(*p, gmin, gmax);
        }
    }
}

// ===========================================================================
// 3-Stage Calibration Pipeline (BIO-WP-08):
//   Stage 1: SSDM Subspace Projection & Non-Uniform Bone Scaling Tensors S_b
//   Stage 2: Locally Affine Stiffness-Penalized Registration
//   Stage 3: Harmonic / Biharmonic TPS Soft-Tissue & Organ Cavity Co-Calibration
// ===========================================================================

/// Number of standardized anthropometric / ontogenetic / endocrine features in [`SsdmBasis`].
pub const SSDM_FEATURE_COUNT: usize = 14;

const REF_WEIGHT_MALE_G: f64 = 78_000.0;
const REF_WEIGHT_FEMALE_G: f64 = 64_000.0;
const REF_THIGH_MALE_MM: f64 = 560.0;
const REF_THIGH_FEMALE_MM: f64 = 550.0;
const REF_CALF_MALE_MM: f64 = 375.0;
const REF_CALF_FEMALE_MM: f64 = 355.0;
const REF_BICEPS_MALE_MM: f64 = 325.0;
const REF_BICEPS_FEMALE_MM: f64 = 285.0;

/// Per-bone sensitivity of log-scale `(sx, sy, sz)` to a single [`SsdmShapeMode`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoneScaleLoading {
    pub bone_id: String,
    pub axis_loadings: [f64; 3],
}

/// One principal geometric mode in the Statistical Shape & Density Model (`SsdmBasis`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SsdmShapeMode {
    pub id: String,
    /// Prior variance (`sigma_i^2 > 0`) for ridge regularization.
    pub eigenvalue: f64,
    /// Loadings across the [`SSDM_FEATURE_COUNT`] standardized anthropometric features.
    pub feature_loadings: [f64; SSDM_FEATURE_COUNT],
    /// Resulting per-bone log-scale sensitivities `[dx, dy, dz]`.
    pub bone_loadings: Vec<BoneScaleLoading>,
}

/// One principal bone-mineral-density (BMD) mode in [`SsdmBasis`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SsdmDensityMode {
    pub id: String,
    pub eigenvalue: f64,
    pub age_loading: f64,
    pub endocrine_loading: f64,
    pub mass_loading: f64,
    /// Regional apparent density shift `delta rho_app` (g/cm^3 per unit mode weight).
    pub regional_bmd_loadings: Vec<(String, f64)>,
}

/// Low-rank Statistical Shape & Density Mode basis for Stage 1 projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SsdmBasis {
    pub reference_model: AnatomyModel,
    pub mean_stature_mm: f64,
    pub mean_mass_g: f64,
    pub regularization_lambda: f64,
    pub shape_modes: Vec<SsdmShapeMode>,
    pub density_modes: Vec<SsdmDensityMode>,
}

impl SsdmBasis {
    /// Construct the canonical human low-rank SSDM basis for `VH_Male` or `VH_Female`.
    pub fn canonical_human(reference_model: AnatomyModel) -> Self {
        let (mean_stature_mm, mean_mass_g) = match reference_model {
            AnatomyModel::Male => (VH_MALE_STATURE_MM as f64, REF_WEIGHT_MALE_G),
            AnatomyModel::Female => (VH_FEMALE_STATURE_MM as f64, REF_WEIGHT_FEMALE_G),
        };

        let loading = |bone_id: &str, axis_loadings: [f64; 3]| BoneScaleLoading {
            bone_id: bone_id.to_string(),
            axis_loadings,
        };

        let shape_modes = vec![
            // Mode 1: Overall stature & longitudinal skeletal allometry
            SsdmShapeMode {
                id: "overall_stature_allometry".to_string(),
                eigenvalue: 1.0,
                feature_loadings: [
                    0.78, 0.04, 0.06, 0.56, 0.16, 0.10, 0.06, 0.10, 0.00, 0.04, 0.04, 0.04,
                    -0.06, 0.00,
                ],
                bone_loadings: vec![
                    loading("cranium", [0.22, 0.24, 0.22]),
                    loading("thoracic_ribcage", [0.38, 0.72, 0.35]),
                    loading("lumbar_spine", [0.30, 0.80, 0.30]),
                    loading("pelvis", [0.42, 0.65, 0.40]),
                    loading("femur_l", [0.32, 0.92, 0.32]),
                    loading("femur_r", [0.32, 0.92, 0.32]),
                    loading("tibia_l", [0.30, 0.94, 0.30]),
                    loading("tibia_r", [0.30, 0.94, 0.30]),
                    loading("humerus_l", [0.28, 0.88, 0.28]),
                    loading("humerus_r", [0.28, 0.88, 0.28]),
                    loading("radius_ulna_l", [0.26, 0.90, 0.26]),
                    loading("radius_ulna_r", [0.26, 0.90, 0.26]),
                ],
            },
            // Mode 2: Cormic index / trunk-to-leg proportion
            SsdmShapeMode {
                id: "trunk_to_limb_proportion".to_string(),
                eigenvalue: 0.45,
                feature_loadings: [
                    0.00, 0.72, -0.68, -0.10, 0.04, 0.04, 0.02, 0.00, 0.00, -0.02, -0.02, 0.00,
                    -0.08, 0.00,
                ],
                bone_loadings: vec![
                    loading("thoracic_ribcage", [0.08, 0.58, 0.08]),
                    loading("lumbar_spine", [0.06, 0.68, 0.06]),
                    loading("pelvis", [0.05, 0.25, 0.05]),
                    loading("femur_l", [0.02, -0.62, 0.02]),
                    loading("femur_r", [0.02, -0.62, 0.02]),
                    loading("tibia_l", [0.02, -0.65, 0.02]),
                    loading("tibia_r", [0.02, -0.65, 0.02]),
                ],
            },
            // Mode 3: Shoulder & thoracic frame breadth
            SsdmShapeMode {
                id: "shoulder_thoracic_frame".to_string(),
                eigenvalue: 0.40,
                feature_loadings: [
                    0.05, 0.02, -0.02, 0.26, 0.78, 0.52, 0.14, 0.06, 0.08, 0.04, 0.02, 0.14,
                    0.00, 0.00,
                ],
                bone_loadings: vec![
                    loading("thoracic_ribcage", [0.74, 0.08, 0.52]),
                    loading("humerus_l", [0.22, 0.14, 0.22]),
                    loading("humerus_r", [0.22, 0.14, 0.22]),
                    loading("radius_ulna_l", [0.16, 0.10, 0.16]),
                    loading("radius_ulna_r", [0.16, 0.10, 0.16]),
                ],
            },
            // Mode 4: Pelvic & hip width / gestational pelvic flare
            SsdmShapeMode {
                id: "pelvic_hip_dimorphism".to_string(),
                eigenvalue: 0.35,
                feature_loadings: [
                    0.00, 0.02, -0.02, 0.00, -0.10, 0.08, 0.34, 0.80, 0.22, 0.32, 0.10, 0.04,
                    0.04, 0.28,
                ],
                bone_loadings: vec![
                    loading("pelvis", [0.76, 0.08, 0.58]),
                    loading("lumbar_spine", [0.14, -0.04, 0.24]),
                    loading("femur_l", [0.24, 0.00, 0.20]),
                    loading("femur_r", [0.24, 0.00, 0.20]),
                ],
            },
            // Mode 5: Soft-tissue / visceral-adipose & gestational abdominal expansion
            SsdmShapeMode {
                id: "adiposity_soft_tissue_mass".to_string(),
                eigenvalue: 0.50,
                feature_loadings: [
                    -0.02, 0.00, 0.00, 0.00, 0.08, 0.34, 0.72, 0.38, 0.38, 0.20, 0.10, 0.12,
                    0.06, 0.18,
                ],
                bone_loadings: vec![
                    loading("thoracic_ribcage", [0.16, 0.00, 0.24]),
                    loading("lumbar_spine", [0.06, 0.00, 0.12]),
                    loading("pelvis", [0.12, 0.00, 0.18]),
                ],
            },
            // Mode 6: Appendicular muscularity & diaphyseal robusticity
            SsdmShapeMode {
                id: "appendicular_muscularity".to_string(),
                eigenvalue: 0.30,
                feature_loadings: [
                    0.02, 0.00, 0.00, 0.12, 0.20, 0.22, -0.10, 0.08, 0.32, 0.54, 0.48, 0.52,
                    -0.04, 0.00,
                ],
                bone_loadings: vec![
                    loading("femur_l", [0.28, 0.02, 0.28]),
                    loading("femur_r", [0.28, 0.02, 0.28]),
                    loading("tibia_l", [0.24, 0.02, 0.24]),
                    loading("tibia_r", [0.24, 0.02, 0.24]),
                    loading("humerus_l", [0.26, 0.02, 0.26]),
                    loading("humerus_r", [0.26, 0.02, 0.26]),
                    loading("radius_ulna_l", [0.22, 0.02, 0.22]),
                    loading("radius_ulna_r", [0.22, 0.02, 0.22]),
                ],
            },
        ];

        let density_modes = vec![
            SsdmDensityMode {
                id: "age_senescent_bmd_trajectory".to_string(),
                eigenvalue: 1.0,
                age_loading: -0.85,
                endocrine_loading: -0.35,
                mass_loading: 0.10,
                regional_bmd_loadings: vec![
                    ("lumbar_spine".to_string(), 0.22),
                    ("pelvis".to_string(), 0.20),
                    ("femur_l".to_string(), 0.24),
                    ("femur_r".to_string(), 0.24),
                    ("tibia_l".to_string(), 0.18),
                    ("tibia_r".to_string(), 0.18),
                    ("thoracic_ribcage".to_string(), 0.16),
                    ("humerus_l".to_string(), 0.15),
                    ("humerus_r".to_string(), 0.15),
                    ("radius_ulna_l".to_string(), 0.14),
                    ("radius_ulna_r".to_string(), 0.14),
                    ("cranium".to_string(), 0.08),
                ],
            },
            SsdmDensityMode {
                id: "mechanostat_loading_hypertrophy".to_string(),
                eigenvalue: 0.5,
                age_loading: -0.10,
                endocrine_loading: 0.05,
                mass_loading: 0.75,
                regional_bmd_loadings: vec![
                    ("femur_l".to_string(), 0.16),
                    ("femur_r".to_string(), 0.16),
                    ("tibia_l".to_string(), 0.15),
                    ("tibia_r".to_string(), 0.15),
                    ("lumbar_spine".to_string(), 0.12),
                    ("pelvis".to_string(), 0.12),
                ],
            },
            SsdmDensityMode {
                id: "endocrine_calcium_mobilization".to_string(),
                eigenvalue: 0.4,
                age_loading: -0.15,
                endocrine_loading: -0.80,
                mass_loading: 0.05,
                regional_bmd_loadings: vec![
                    ("lumbar_spine".to_string(), 0.18),
                    ("pelvis".to_string(), 0.16),
                    ("femur_l".to_string(), 0.12),
                    ("femur_r".to_string(), 0.12),
                ],
            },
        ];

        Self {
            reference_model,
            mean_stature_mm,
            mean_mass_g,
            regularization_lambda: 0.08,
            shape_modes,
            density_modes,
        }
    }
}

/// Non-uniform affine bone scaling tensor `S_b = diag(s_{b,x}, s_{b,y}, s_{b,z})`
/// and compartmental bone mineral stiffness from Stage 1 SSDM projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoneScaleTensor {
    pub bone_id: String,
    /// Principal diagonal scale factors `[s_x, s_y, s_z]` (strictly positive).
    pub diag_scale: [f64; 3],
    /// Apparent wet bone density `rho_app` in g/cm^3.
    pub apparent_density_g_cm3: f64,
    /// Carter-Hayes / Morgan-Keaveny elastic modulus `E(rho_app)` in GPa.
    pub elastic_modulus_gpa: f64,
    /// Estimated cortical shell thickness in mm.
    pub cortical_thickness_mm: f64,
}

impl BoneScaleTensor {
    /// Materializes `S_b = diag(s_{b,x}, s_{b,y}, s_{b,z})` as a row-major `3x3` matrix.
    pub fn matrix_3x3(&self) -> [[f64; 3]; 3] {
        [
            [self.diag_scale[0], 0.0, 0.0],
            [0.0, self.diag_scale[1], 0.0],
            [0.0, 0.0, self.diag_scale[2]],
        ]
    }
}

/// Weighted anthropometric or anatomical landmark observation for Stage 2 registration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationObservation {
    /// Name of the measurement or landmark (e.g. `"waist_mm"`, `"femur_l_trochanter"`).
    pub feature_id: String,
    /// Optional target bone identifier for spatial Stage 2 registration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region_bone_id: Option<String>,
    /// Optional 3D target displacement vector (meters) in CCF coordinates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_displacement_m: Option<[f64; 3]>,
    /// Instrument/epistemic confidence `c_0` in `[0.0, 1.0]`.
    pub base_confidence: f64,
    /// Elapsed days since phenomenon time (`>= 0.0`).
    pub elapsed_days: f64,
    /// Biological temporal half-life `t_{1/2}` in days (`> 0.0`).
    pub half_life_days: f64,
}

impl CalibrationObservation {
    /// Exponential temporal decay weight `w_k = c_obs(t) = c_0 * 2^(-dt / half_life)`.
    pub fn decayed_confidence(&self) -> f64 {
        let c0 = self.base_confidence.clamp(0.0, 1.0);
        if !c0.is_finite() || !self.elapsed_days.is_finite() {
            return 0.0;
        }
        let dt = self.elapsed_days.max(0.0);
        let hl = if self.half_life_days.is_finite() && self.half_life_days > 0.0 {
            self.half_life_days
        } else {
            365.0
        };
        (c0 * (-(dt / hl) * std::f64::consts::LN_2).exp()).clamp(0.0, 1.0)
    }
}

/// Stage 2 per-bone locally affine stiffness-penalized correction `(A_b, t_b)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoneAffineCorrection {
    pub bone_id: String,
    /// Orientation-preserving locally affine matrix `A_b` (`det(A_b) > 0`).
    pub linear_3x3: [[f64; 3]; 3],
    /// Local translational correction `t_b` in meters.
    pub translation_m: [f64; 3],
    /// Determinant `det(A_b) > 0`.
    pub jacobian_det: f64,
    /// Bone mineral stiffness `E(rho_app)` in GPa governing stiffness regularization.
    pub bone_stiffness_gpa: f64,
    /// Aggregated observation confidence `w_k = c_obs(t)` driving this bone's correction.
    pub observation_confidence: f64,
}

/// Stage 3 deformed internal organ / soft-tissue cavity point with Jacobian volume check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganCavityTransfer {
    pub organ_id: String,
    pub reference_point_m: [f64; 3],
    pub calibrated_point_m: [f64; 3],
    /// Local volume ratio `det(J) > 0` guaranteeing no self-intersection / foldover.
    pub jacobian_det: f64,
}

/// Input bundle for [`derive_anatomical_calibration`].
pub struct AnatomicalCalibrationInput<'a> {
    pub constitution: &'a BodyConstitution,
    pub physiological_state: Option<super::physiology::PhysiologicalState>,
    pub observations: &'a [CalibrationObservation],
    /// Optional internal organ/cavity query coordinates `(organ_id, xyz_m)`.
    /// When empty, canonical thoracic/abdominal/pelvic cavity landmarks are evaluated.
    pub organ_query_points: &'a [(&'a str, [f64; 3])],
    /// Optional caller-supplied SSDM basis; defaults to [`SsdmBasis::canonical_human`].
    pub basis: Option<&'a SsdmBasis>,
}

/// Full 3-stage calibration output preserving backward compatibility via `body_fit`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnatomicalCalibrationResult {
    pub reference_model: AnatomyModel,
    pub shape_mode_weights: Vec<f64>,
    pub density_mode_weights: Vec<f64>,
    pub bone_scalings: Vec<BoneScaleTensor>,
    pub bone_corrections: Vec<BoneAffineCorrection>,
    pub control_cage: Vec<[f64; 3]>,
    pub displaced_cage: Vec<[f64; 3]>,
    pub organ_transfers: Vec<OrganCavityTransfer>,
    pub body_fit: BodyFit,
    pub is_identity: bool,
}

/// Canonical bone reference anchors `[x, y, z]` in meters (CCF up = `+y`, anterior = `+z`).
const CANONICAL_BONES: &[(&str, [f64; 3], f64, f64)] = &[
    // (bone_id, canonical_center_m, baseline_rho_app_g_cm3, baseline_cortical_mm)
    ("cranium", [0.0, 1.68, 0.01], 1.82, 4.2),
    ("thoracic_ribcage", [0.0, 1.28, 0.0], 1.48, 2.4),
    ("lumbar_spine", [0.0, 1.02, -0.04], 1.18, 1.8),
    ("pelvis", [0.0, 0.86, 0.0], 1.42, 2.8),
    ("femur_l", [-0.10, 0.64, 0.0], 1.85, 5.2),
    ("femur_r", [0.10, 0.64, 0.0], 1.85, 5.2),
    ("tibia_l", [-0.09, 0.25, 0.0], 1.84, 4.6),
    ("tibia_r", [0.09, 0.25, 0.0], 1.84, 4.6),
    ("humerus_l", [-0.21, 1.24, 0.0], 1.78, 3.8),
    ("humerus_r", [0.21, 1.24, 0.0], 1.78, 3.8),
    ("radius_ulna_l", [-0.24, 0.96, 0.02], 1.76, 3.2),
    ("radius_ulna_r", [0.24, 0.96, 0.02], 1.76, 3.2),
];

/// Canonical thoracic / abdominal / pelvic internal organ cavity query landmarks (meters).
const CANONICAL_ORGAN_CAVITY_POINTS: &[(&str, [f64; 3])] = &[
    ("heart", [0.03, 1.27, 0.04]),
    ("lung_l", [-0.08, 1.30, 0.01]),
    ("lung_r", [0.08, 1.30, 0.01]),
    ("diaphragm", [0.0, 1.18, 0.02]),
    ("liver", [0.07, 1.12, 0.03]),
    ("stomach", [-0.05, 1.10, 0.04]),
    ("pelvic_cavity", [0.0, 0.84, 0.02]),
];

fn bone_elastic_modulus_gpa(rho_app: f64) -> f64 {
    // Carter & Hayes (1977) / Morgan-Keaveny power-law scaling normalized to cortical 1.85 g/cm^3 -> 16.5 GPa
    let r = (rho_app / 1.85).clamp(0.15, 1.35);
    16.5 * r.powf(2.2)
}

fn mat3_det_f64(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// Solve dense `n x n` linear system `A x = b` via Gaussian elimination with partial pivoting.
fn solve_linear_system(n: usize, a_in: &[f64], b_in: &[f64]) -> Option<Vec<f64>> {
    if n == 0 || a_in.len() != n * n || b_in.len() != n {
        return None;
    }
    let mut a = a_in.to_vec();
    let mut b = b_in.to_vec();
    for col in 0..n {
        let mut pivot = col;
        let mut max_abs = a[col * n + col].abs();
        for row in (col + 1)..n {
            let v = a[row * n + col].abs();
            if v > max_abs {
                max_abs = v;
                pivot = row;
            }
        }
        if !max_abs.is_finite() || max_abs < 1e-13 {
            return None;
        }
        if pivot != col {
            for k in 0..n {
                a.swap(col * n + k, pivot * n + k);
            }
            b.swap(col, pivot);
        }
        let diag = a[col * n + col];
        for row in (col + 1)..n {
            let factor = a[row * n + col] / diag;
            a[row * n + col] = 0.0;
            for k in (col + 1)..n {
                a[row * n + k] -= factor * a[col * n + k];
            }
            b[row] -= factor * b[col];
        }
    }
    let mut x = vec![0.0_f64; n];
    for row in (0..n).rev() {
        let mut sum = b[row];
        for col in (row + 1)..n {
            sum -= a[row * n + col] * x[col];
        }
        let diag = a[row * n + row];
        if !diag.is_finite() || diag.abs() < 1e-13 {
            return None;
        }
        x[row] = sum / diag;
    }
    Some(x)
}

/// Precomputed 3D biharmonic Thin-Plate Spline (TPS) / Harmonic volume transfer field.
#[derive(Debug, Clone)]
struct HarmonicTpsField {
    centroid: [f64; 3],
    scale: f64,
    eps: f64,
    norm_cage: Vec<[f64; 3]>,
    weights: Vec<[f64; 3]>,
    affine_linear: [[f64; 3]; 3],
    affine_translation: [f64; 3],
}

impl HarmonicTpsField {
    fn fit(control_cage: &[[f64; 3]], displaced_cage: &[[f64; 3]]) -> Option<Self> {
        let n = control_cage.len();
        if n == 0 || n != displaced_cage.len() {
            return None;
        }
        if control_cage
            .iter()
            .chain(displaced_cage.iter())
            .flatten()
            .any(|v| !v.is_finite())
        {
            return None;
        }

        let mut centroid = [0.0_f64; 3];
        for p in control_cage {
            centroid[0] += p[0];
            centroid[1] += p[1];
            centroid[2] += p[2];
        }
        centroid[0] /= n as f64;
        centroid[1] /= n as f64;
        centroid[2] /= n as f64;

        let mut max_r2 = 0.0_f64;
        for p in control_cage {
            let dx = p[0] - centroid[0];
            let dy = p[1] - centroid[1];
            let dz = p[2] - centroid[2];
            max_r2 = max_r2.max(dx * dx + dy * dy + dz * dz);
        }
        let scale = max_r2.sqrt().max(1e-4);
        let eps = 1e-3;
        let norm_cage: Vec<[f64; 3]> = control_cage
            .iter()
            .map(|p| [
                (p[0] - centroid[0]) / scale,
                (p[1] - centroid[1]) / scale,
                (p[2] - centroid[2]) / scale,
            ])
            .collect();
        let norm_disp: Vec<[f64; 3]> = displaced_cage
            .iter()
            .zip(control_cage.iter())
            .map(|(d, c)| [
                (d[0] - c[0]) / scale,
                (d[1] - c[1]) / scale,
                (d[2] - c[2]) / scale,
            ])
            .collect();

        // Build orthonormal polynomial columns Q1 (N x r, r <= 4) from [1, x, y, z] via modified Gram-Schmidt
        let mut p_cols: Vec<Vec<f64>> = vec![vec![0.0; n]; 4];
        for (i, c) in norm_cage.iter().enumerate() {
            p_cols[0][i] = 1.0;
            p_cols[1][i] = c[0];
            p_cols[2][i] = c[1];
            p_cols[3][i] = c[2];
        }

        let mut q_cols: Vec<Vec<f64>> = Vec::with_capacity(4);
        let mut q_orig_idx: Vec<usize> = Vec::with_capacity(4);
        let mut r_mat = vec![vec![0.0_f64; 4]; 4];
        for col_idx in 0..4 {
            let mut v = p_cols[col_idx].clone();
            for (q_idx, q) in q_cols.iter().enumerate() {
                let dot: f64 = v.iter().zip(q.iter()).map(|(a, b)| a * b).sum();
                r_mat[q_idx][col_idx] = dot;
                for (vi, qi) in v.iter_mut().zip(q.iter()) {
                    *vi -= dot * qi;
                }
            }
            let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm > 1e-8 {
                let inv = 1.0 / norm;
                for vi in &mut v {
                    *vi *= inv;
                }
                r_mat[q_cols.len()][col_idx] = norm;
                q_orig_idx.push(col_idx);
                q_cols.push(v);
            }
        }

        let r_rank = q_cols.len();
        let dim = n + r_rank;
        let mut sys = vec![0.0_f64; dim * dim];
        let lambda_k = 1e-6;
        for i in 0..n {
            for j in 0..n {
                let dx = norm_cage[i][0] - norm_cage[j][0];
                let dy = norm_cage[i][1] - norm_cage[j][1];
                let dz = norm_cage[i][2] - norm_cage[j][2];
                let r = (dx * dx + dy * dy + dz * dz + eps * eps).sqrt();
                // 3D biharmonic kernel U(r) = -r is conditionally positive-definite on ker(P^T)
                sys[i * dim + j] = -r + if i == j { lambda_k } else { 0.0 };
            }
            for k in 0..r_rank {
                let qik = q_cols[k][i];
                sys[i * dim + (n + k)] = qik;
                sys[(n + k) * dim + i] = qik;
            }
        }

        let mut weights = vec![[0.0_f64; 3]; n];
        let mut affine_linear = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let mut affine_translation = [0.0_f64; 3];

        for axis in 0..3 {
            let mut rhs = vec![0.0_f64; dim];
            for i in 0..n {
                rhs[i] = norm_disp[i][axis];
            }
            let sol = solve_linear_system(dim, &sys, &rhs)?;
            for i in 0..n {
                weights[i][axis] = sol[i];
            }
            // Recover affine coefficients in [1, x, y, z] from Q1 * sol[n..]
            let mut poly = [0.0_f64; 4];
            for k in (0..r_rank).rev() {
                let orig = q_orig_idx[k];
                let mut sum = sol[n + k];
                for j in (k + 1)..r_rank {
                    let orig_j = q_orig_idx[j];
                    sum -= r_mat[k][orig_j] * poly[orig_j];
                }
                poly[orig] = sum / r_mat[k][orig];
            }
            affine_translation[axis] = poly[0] * scale;
            affine_linear[axis][0] += poly[1];
            affine_linear[axis][1] += poly[2];
            affine_linear[axis][2] += poly[3];
        }

        Some(Self {
            centroid,
            scale,
            eps,
            norm_cage,
            weights,
            affine_linear,
            affine_translation,
        })
    }

    fn eval_raw(&self, query_point: [f64; 3]) -> ([f64; 3], [[f64; 3]; 3]) {
        let qn = [
            (query_point[0] - self.centroid[0]) / self.scale,
            (query_point[1] - self.centroid[1]) / self.scale,
            (query_point[2] - self.centroid[2]) / self.scale,
        ];
        let mut out = [0.0_f64; 3];
        let mut jac = self.affine_linear;
        for axis in 0..3 {
            out[axis] = self.centroid[axis]
                + self.affine_translation[axis]
                + self.scale
                    * (self.affine_linear[axis][0] * qn[0]
                        + self.affine_linear[axis][1] * qn[1]
                        + self.affine_linear[axis][2] * qn[2]);
        }

        for (c, w) in self.norm_cage.iter().zip(self.weights.iter()) {
            let dx = qn[0] - c[0];
            let dy = qn[1] - c[1];
            let dz = qn[2] - c[2];
            let r = (dx * dx + dy * dy + dz * dz + self.eps * self.eps).sqrt();
            let u = -r;
            let inv_r = 1.0 / r;
            let grad = [-dx * inv_r, -dy * inv_r, -dz * inv_r];
            for axis in 0..3 {
                out[axis] += self.scale * w[axis] * u;
                for col in 0..3 {
                    jac[axis][col] += w[axis] * grad[col];
                }
            }
        }
        (out, jac)
    }

    /// Evaluate deformed point and Jacobian determinant, enforcing `det(J) > 0`
    /// via homotopy damping toward an orientation-preserving baseline when needed.
    fn eval_volume_preserving(&self, query_point: [f64; 3]) -> ([f64; 3], f64) {
        let (pt, jac) = self.eval_raw(query_point);
        let det = mat3_det_f64(&jac);
        const MIN_DET: f64 = 1e-3;
        if det.is_finite() && det > MIN_DET {
            return (pt, det);
        }

        // Choose an orientation-preserving baseline: affine base if det(A) > MIN_DET, else identity.
        let det_aff = mat3_det_f64(&self.affine_linear);
        let (base_pt, base_jac) = if det_aff.is_finite() && det_aff > MIN_DET {
            let qn = [
                (query_point[0] - self.centroid[0]) / self.scale,
                (query_point[1] - self.centroid[1]) / self.scale,
                (query_point[2] - self.centroid[2]) / self.scale,
            ];
            let mut bp = [0.0; 3];
            for axis in 0..3 {
                bp[axis] = self.centroid[axis]
                    + self.affine_translation[axis]
                    + self.scale
                        * (self.affine_linear[axis][0] * qn[0]
                            + self.affine_linear[axis][1] * qn[1]
                            + self.affine_linear[axis][2] * qn[2]);
            }
            (bp, self.affine_linear)
        } else {
            (
                query_point,
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            )
        };

        let mut lo = 0.0_f64;
        let mut hi = 1.0_f64;
        for _ in 0..24 {
            let mid = 0.5 * (lo + hi);
            let mut j_mid = [[0.0_f64; 3]; 3];
            for r in 0..3 {
                for c in 0..3 {
                    j_mid[r][c] = (1.0 - mid) * base_jac[r][c] + mid * jac[r][c];
                }
            }
            if mat3_det_f64(&j_mid) > MIN_DET {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        let mut j_safe = [[0.0_f64; 3]; 3];
        for r in 0..3 {
            for c in 0..3 {
                j_safe[r][c] = (1.0 - lo) * base_jac[r][c] + lo * jac[r][c];
            }
        }
        let safe_pt = [
            (1.0 - lo) * base_pt[0] + lo * pt[0],
            (1.0 - lo) * base_pt[1] + lo * pt[1],
            (1.0 - lo) * base_pt[2] + lo * pt[2],
        ];
        (safe_pt, mat3_det_f64(&j_safe).max(MIN_DET))
    }
}

/// Stage 3: Harmonic / 3D biharmonic Thin-Plate Spline soft-tissue & organ-cavity transfer.
///
/// Maps `query_point` smoothly from `control_cage` to `displaced_cage` and returns
/// `(deformed_point, det_jacobian)` with guaranteed `det(J) > 0` so thoracic/abdominal
/// organ cavities never invert or self-intersect.
pub fn harmonic_tps_organ_transfer(
    control_cage: &[[f64; 3]],
    displaced_cage: &[[f64; 3]],
    query_point: [f64; 3],
) -> ([f64; 3], f64) {
    if query_point.iter().any(|v| !v.is_finite()) {
        return (query_point, 1.0);
    }
    match HarmonicTpsField::fit(control_cage, displaced_cage) {
        Some(field) => field.eval_volume_preserving(query_point),
        None => (query_point, 1.0),
    }
}

/// Extract observed standardized anthropometric/ontogenetic/endocrine features and their
/// observation confidence weights `(feature_index, standardized_delta, weight)`.
fn extract_ssdm_observations(
    constitution: &BodyConstitution,
    physiological_state: Option<super::physiology::PhysiologicalState>,
    observations: &[CalibrationObservation],
    basis: &SsdmBasis,
) -> (Vec<(usize, f64, f64)>, f64, f64, f64) {
    let m = &constitution.measurements;
    let girth = constitution.reference_girths_mm();
    let model = constitution
        .characteristics
        .anatomy_model()
        .unwrap_or(basis.reference_model);
    let (ref_thigh, ref_calf, ref_biceps) = match model {
        AnatomyModel::Male => (REF_THIGH_MALE_MM, REF_CALF_MALE_MM, REF_BICEPS_MALE_MM),
        AnatomyModel::Female => (
            REF_THIGH_FEMALE_MM,
            REF_CALF_FEMALE_MM,
            REF_BICEPS_FEMALE_MM,
        ),
    };

    let obs_weight = |key: &str| -> f64 {
        let mut best = 0.0_f64;
        for o in observations {
            if o.feature_id == key {
                best = best.max(o.decayed_confidence());
            }
        }
        if best > 0.0 { best } else { 1.0 }
    };

    let mut feats: Vec<(usize, f64, f64)> = Vec::new();
    let stature_mm = m.stature_mm.map(|v| v as f64);
    if let Some(h) = stature_mm {
        feats.push((0, (h / basis.mean_stature_mm) - 1.0, obs_weight("stature_mm")));
    }
    let eff_stature = stature_mm.unwrap_or(basis.mean_stature_mm);
    if let Some(sit) = m.sitting_height_mm {
        let delta = (sit as f64 / eff_stature) - REF_SITTING_RATIO as f64;
        feats.push((1, delta / REF_SITTING_RATIO as f64, obs_weight("sitting_height_mm")));
    }
    if let Some(ins) = m.inseam_mm {
        let delta = (ins as f64 / eff_stature) - REF_INSEAM_RATIO as f64;
        feats.push((2, delta / REF_INSEAM_RATIO as f64, obs_weight("inseam_mm")));
    }
    if let Some(span) = m.arm_span_mm {
        feats.push((3, (span as f64 / eff_stature) - 1.0, obs_weight("arm_span_mm")));
    }
    if let Some(sh) = m.shoulder_width_mm {
        feats.push((
            4,
            (sh as f64 / girth.shoulder_width_mm as f64) - 1.0,
            obs_weight("shoulder_width_mm"),
        ));
    }
    if let Some(ch) = m.chest_mm {
        feats.push((
            5,
            (ch as f64 / girth.chest_mm as f64) - 1.0,
            obs_weight("chest_mm"),
        ));
    }
    if let Some(wa) = m.waist_mm.or(m.abdomen_mm) {
        feats.push((
            6,
            (wa as f64 / girth.waist_mm as f64) - 1.0,
            obs_weight("waist_mm"),
        ));
    }
    if let Some(hp) = m.hip_mm {
        feats.push((
            7,
            (hp as f64 / girth.hip_mm as f64) - 1.0,
            obs_weight("hip_mm"),
        ));
    }
    let mut mass_z = 0.0_f64;
    if let Some(wt) = m.weight_g {
        let h_ratio = eff_stature / basis.mean_stature_mm;
        let expected_mass = basis.mean_mass_g * h_ratio * h_ratio;
        mass_z = ((wt as f64 / expected_mass).max(0.25)).ln();
        feats.push((8, mass_z, obs_weight("weight_g")));
    }
    if let Some(th) = m.thigh_mm.or(m.mid_thigh_mm) {
        feats.push((9, (th as f64 / ref_thigh) - 1.0, obs_weight("thigh_mm")));
    }
    if let Some(cf) = m.calf_mm {
        feats.push((10, (cf as f64 / ref_calf) - 1.0, obs_weight("calf_mm")));
    }
    if let Some(bc) = m.biceps_mm {
        feats.push((11, (bc as f64 / ref_biceps) - 1.0, obs_weight("biceps_mm")));
    }

    // Age maturation & senescent remodeling index (centered around young-adult peak 30y = 360m)
    let age_z = if let Some(months) = constitution.characteristics.age_months {
        let years = months as f64 / 12.0;
        let z = if years < 20.0 {
            -(20.0 - years) / 20.0
        } else if years <= 35.0 {
            0.0
        } else {
            ((years - 35.0) / 50.0).min(1.2)
        };
        feats.push((12, z, 0.6));
        z
    } else {
        0.0
    };

    // Sex/endocrine continuum state
    use super::physiology::{PhysiologicalState, ReproductiveState};
    let endocrine_z = match physiological_state {
        Some(PhysiologicalState::Reproductive(ReproductiveState::Pregnant(t))) => match t {
            Trimester::First => 0.25,
            Trimester::Second => 0.60,
            Trimester::Third => 1.00,
        },
        Some(PhysiologicalState::Reproductive(ReproductiveState::Lactating)) => 0.35,
        Some(PhysiologicalState::Reproductive(ReproductiveState::Perimenopause)) => 0.45,
        Some(PhysiologicalState::Reproductive(ReproductiveState::PostMenopause)) => 0.85,
        _ => match constitution.attributes.pregnancy {
            Some(Trimester::First) => 0.25,
            Some(Trimester::Second) => 0.60,
            Some(Trimester::Third) => 1.00,
            None => 0.0,
        },
    };
    if endocrine_z > 0.0 {
        feats.push((13, endocrine_z, 0.85));
    }

    (feats, age_z, endocrine_z, mass_z)
}

/// Derive the full 3-stage anatomical, densitometric, and organ-cavity calibration.
///
/// Preserves full backward compatibility via `result.body_fit` while adding:
/// - **Stage 1:** Weighted ridge-regularized SSDM subspace projection -> `(b, c)` and
///   non-uniform bone scaling tensors `S_b = diag(s_{b,x}, s_{b,y}, s_{b,z})`.
/// - **Stage 2:** Locally affine stiffness-penalized per-bone registration weighted by
///   observation confidence `w_k = c_obs(t)` and bone mineral stiffness `E(rho_app)`.
/// - **Stage 3:** 3D biharmonic TPS / Harmonic soft-tissue & organ-cavity transfer with
///   guaranteed `det(J) > 0`.
pub fn derive_anatomical_calibration(
    input: &AnatomicalCalibrationInput<'_>,
) -> AnatomicalCalibrationResult {
    let model = input
        .constitution
        .characteristics
        .anatomy_model()
        .unwrap_or(AnatomyModel::Male);
    let owned_basis;
    let basis = match input.basis {
        Some(b) => b,
        None => {
            owned_basis = SsdmBasis::canonical_human(model);
            &owned_basis
        }
    };

    let pregnancy_override = match input.physiological_state {
        Some(super::physiology::PhysiologicalState::Reproductive(
            super::physiology::ReproductiveState::Pregnant(t),
        )) => Some(t),
        _ => input.constitution.attributes.pregnancy,
    };
    let body_fit = input.constitution.fit_with_pregnancy(pregnancy_override);

    let (feats, age_z, endocrine_z, mass_z) = extract_ssdm_observations(
        input.constitution,
        input.physiological_state,
        input.observations,
        basis,
    );

    // Stage 1A: Solve regularized weighted least squares (Phi^T W Phi + Lambda) b = Phi^T W f
    let m_modes = basis.shape_modes.len();
    let mut shape_weights = vec![0.0_f64; m_modes];
    if !feats.is_empty() && m_modes > 0 {
        let mut h = vec![0.0_f64; m_modes * m_modes];
        let mut rhs = vec![0.0_f64; m_modes];
        for i in 0..m_modes {
            let eig = basis.shape_modes[i].eigenvalue.max(1e-4);
            h[i * m_modes + i] = basis.regularization_lambda / eig;
        }
        for &(feat_idx, delta, w) in &feats {
            for i in 0..m_modes {
                let phi_i = basis.shape_modes[i].feature_loadings[feat_idx];
                rhs[i] += w * phi_i * delta;
                for j in 0..m_modes {
                    let phi_j = basis.shape_modes[j].feature_loadings[feat_idx];
                    h[i * m_modes + j] += w * phi_i * phi_j;
                }
            }
        }
        if let Some(sol) = solve_linear_system(m_modes, &h, &rhs) {
            shape_weights = sol;
        }
    }

    // Stage 1B: Density mode projection (c)
    let k_modes = basis.density_modes.len();
    let mut density_weights = vec![0.0_f64; k_modes];
    for (k, dm) in basis.density_modes.iter().enumerate() {
        let raw = dm.age_loading * age_z.max(0.0)
            + dm.endocrine_loading * endocrine_z
            + dm.mass_loading * mass_z;
        density_weights[k] = (raw / (1.0 + basis.regularization_lambda / dm.eigenvalue.max(1e-4)))
            .clamp(-2.5, 2.5);
    }

    // Stage 1C: Compute non-uniform bone scaling tensors S_b = diag(s_x, s_y, s_z) and E(rho_app)
    let mut bone_scalings = Vec::with_capacity(CANONICAL_BONES.len());
    for &(bone_id, _center, base_rho, base_cort_mm) in CANONICAL_BONES {
        let mut log_s = [0.0_f64; 3];
        for (i, sm) in basis.shape_modes.iter().enumerate() {
            let b_i = shape_weights[i];
            if let Some(bl) = sm.bone_loadings.iter().find(|bl| bl.bone_id == bone_id) {
                log_s[0] += b_i * bl.axis_loadings[0];
                log_s[1] += b_i * bl.axis_loadings[1];
                log_s[2] += b_i * bl.axis_loadings[2];
            }
        }
        let diag_scale = [
            log_s[0].exp().clamp(0.35, 2.5),
            log_s[1].exp().clamp(0.35, 2.5),
            log_s[2].exp().clamp(0.35, 2.5),
        ];

        let mut delta_rho = 0.0_f64;
        for (k, dm) in basis.density_modes.iter().enumerate() {
            if let Some((_, loading)) = dm.regional_bmd_loadings.iter().find(|(id, _)| id == bone_id) {
                delta_rho += density_weights[k] * *loading;
            }
        }

        // Dominant side upper-limb cortical hypertrophy (+8% cortical thickness, +4% BMD)
        let dom_mult = match input.constitution.characteristics.dominant_side {
            Some(DominantSide::Left)
                if bone_id.ends_with("_l") && (bone_id.starts_with("humerus") || bone_id.starts_with("radius")) =>
            {
                1.04
            }
            Some(DominantSide::Right)
                if bone_id.ends_with("_r") && (bone_id.starts_with("humerus") || bone_id.starts_with("radius")) =>
            {
                1.04
            }
            _ => 1.0,
        };

        let rho_app = ((base_rho + delta_rho) * dom_mult).clamp(0.30, 2.25);
        let cort_mm = (base_cort_mm * (rho_app / base_rho) * dom_mult).clamp(0.4, 12.0);
        let e_gpa = bone_elastic_modulus_gpa(rho_app);

        bone_scalings.push(BoneScaleTensor {
            bone_id: bone_id.to_string(),
            diag_scale,
            apparent_density_g_cm3: rho_app,
            elastic_modulus_gpa: e_gpa,
            cortical_thickness_mm: cort_mm,
        });
    }

    // Stage 2: Locally affine stiffness-penalized per-bone registration
    // Relaxin modifier during pregnancy increases pelvic/lumbar compliance slightly.
    let pelvic_relaxin_factor = if pregnancy_override.is_some() { 0.72 } else { 1.0 };

    let mut bone_corrections = Vec::with_capacity(CANONICAL_BONES.len());
    let mut control_cage = Vec::with_capacity(CANONICAL_BONES.len());
    let mut displaced_cage = Vec::with_capacity(CANONICAL_BONES.len());

    // Global vertical anchor (feet at y=0): accumulate scaled vertical centers cleanly
    let global_y_scale = bone_scalings
        .iter()
        .find(|b| b.bone_id == "femur_l")
        .map(|b| b.diag_scale[1])
        .unwrap_or(1.0);
    let trunk_x_scale = bone_scalings
        .iter()
        .find(|b| b.bone_id == "thoracic_ribcage")
        .map(|b| b.diag_scale[0])
        .unwrap_or(1.0);

    for (idx, &(bone_id, ref_center, _base_rho, _)) in CANONICAL_BONES.iter().enumerate() {
        let sb = &bone_scalings[idx];
        let mut linear = sb.matrix_3x3();
        let mut trans = [
            ref_center[0] * (trunk_x_scale - 1.0) * 0.45,
            ref_center[1] * (0.5 * (global_y_scale + sb.diag_scale[1]) - 1.0),
            0.0,
        ];

        // Pregnancy abdominal/pelvic anterior shift
        if body_fit.pregnancy_abdomen > 0.0 && (bone_id == "pelvis" || bone_id == "lumbar_spine") {
            trans[2] += body_fit.pregnancy_abdomen as f64 * 0.045;
        }

        let eff_stiffness = sb.elastic_modulus_gpa
            * if bone_id == "pelvis" || bone_id == "lumbar_spine" {
                pelvic_relaxin_factor
            } else {
                1.0
            };

        let mut total_conf = 0.0_f64;
        let mut weighted_disp = [0.0_f64; 3];
        for obs in input.observations {
            let matches_bone = obs
                .region_bone_id
                .as_deref()
                .is_some_and(|b| b == bone_id)
                || obs.feature_id == bone_id;
            if matches_bone {
                let w = obs.decayed_confidence();
                if w > 0.0 {
                    total_conf += w;
                    if let Some(d) = obs.target_displacement_m {
                        weighted_disp[0] += w * d[0];
                        weighted_disp[1] += w * d[1];
                        weighted_disp[2] += w * d[2];
                    }
                }
            }
        }

        if total_conf > 0.0 {
            // Stiffness-penalized update: d_applied = (w / (w + alpha * E / E_ref)) * d_obs
            // Higher cortical BMD E(rho_app) resists non-physiological local distortion.
            let alpha_stiff = 0.25 * (eff_stiffness / 15.0);
            let gain = total_conf / (total_conf + alpha_stiff);
            let avg_d = [
                weighted_disp[0] / total_conf,
                weighted_disp[1] / total_conf,
                weighted_disp[2] / total_conf,
            ];
            trans[0] += gain * avg_d[0];
            trans[1] += gain * avg_d[1];
            trans[2] += gain * avg_d[2];

            // Gentle stiffness-penalized anisotropic shear/scale accommodation
            let local_strain = (gain * 0.15).min(0.25);
            for axis in 0..3 {
                let s_adj = (1.0 + local_strain * avg_d[axis].clamp(-1.0, 1.0)).clamp(0.75, 1.35);
                linear[axis][axis] *= s_adj;
            }
        }

        let det = mat3_det_f64(&linear).max(1e-6);
        let displaced_center = [
            ref_center[0] * linear[0][0]
                + ref_center[1] * linear[0][1]
                + ref_center[2] * linear[0][2]
                + trans[0],
            ref_center[0] * linear[1][0]
                + ref_center[1] * linear[1][1]
                + ref_center[2] * linear[1][2]
                + trans[1],
            ref_center[0] * linear[2][0]
                + ref_center[1] * linear[2][1]
                + ref_center[2] * linear[2][2]
                + trans[2],
        ];

        control_cage.push(ref_center);
        displaced_cage.push(displaced_center);
        bone_corrections.push(BoneAffineCorrection {
            bone_id: bone_id.to_string(),
            linear_3x3: linear,
            translation_m: trans,
            jacobian_det: det,
            bone_stiffness_gpa: eff_stiffness,
            observation_confidence: total_conf,
        });
    }

    // Stage 3: Harmonic / TPS Soft-Tissue & Organ Cavity Co-Calibration
    let tps_field = HarmonicTpsField::fit(&control_cage, &displaced_cage);
    let queries: Vec<(&str, [f64; 3])> = if input.organ_query_points.is_empty() {
        CANONICAL_ORGAN_CAVITY_POINTS.to_vec()
    } else {
        input.organ_query_points.to_vec()
    };

    let mut organ_transfers = Vec::with_capacity(queries.len());
    for &(organ_id, qpt) in &queries {
        let (cal_pt, j_det) = match &tps_field {
            Some(f) if !qpt.iter().any(|v| !v.is_finite()) => f.eval_volume_preserving(qpt),
            _ => (qpt, 1.0),
        };
        organ_transfers.push(OrganCavityTransfer {
            organ_id: organ_id.to_string(),
            reference_point_m: qpt,
            calibrated_point_m: cal_pt,
            jacobian_det: j_det,
        });
    }

    let is_identity = body_fit.identity
        && input.observations.is_empty()
        && shape_weights.iter().all(|w| w.abs() < 1e-9);

    AnatomicalCalibrationResult {
        reference_model: model,
        shape_mode_weights: shape_weights,
        density_mode_weights: density_weights,
        bone_scalings,
        bone_corrections,
        control_cage,
        displaced_cage,
        organ_transfers,
        body_fit,
        is_identity,
    }
}

impl BodyConstitution {
    /// Run the 3-stage SSDM + Stiffness-Penalized Registration + Harmonic/TPS calibration pipeline.
    pub fn derive_anatomical_calibration(
        &self,
        physiological_state: Option<super::physiology::PhysiologicalState>,
        observations: &[CalibrationObservation],
    ) -> AnatomicalCalibrationResult {
        derive_anatomical_calibration(&AnatomicalCalibrationInput {
            constitution: self,
            physiological_state,
            observations,
            organ_query_points: &[],
            basis: None,
        })
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn bump(x: f32, center: f32, width: f32) -> f32 {
    let t = ((x - center) / width).abs();
    if t >= 1.0 { 0.0 } else { 1.0 - t * t }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_constitution_is_identity_fit() {
        let fit = BodyConstitution::default().fit();
        assert!(fit.identity);
        assert!((fit.stature_scale - 1.0).abs() < f32::EPSILON);
        assert!(fit.hidden_keys.is_empty());
    }

    #[test]
    fn taller_person_scales_up() {
        let mut c = BodyConstitution::default();
        c.measurements.stature_mm = Some(2000);
        let fit = c.fit();
        assert!(!fit.identity);
        assert!((fit.stature_scale - 2000.0 / 1800.0).abs() < 1e-4);
        let p = fit.transform_point([0.0, 1.8, 0.0], [0.0, 0.0, 0.0], [0.4, 1.8, 0.3]);
        assert!(p[1] > 1.8, "crown should rise with stature, got {}", p[1]);
    }

    #[test]
    fn xx_uses_female_reference_stature() {
        let mut c = BodyConstitution::default();
        c.characteristics.karyotype = Some(Karyotype::Xx);
        c.measurements.stature_mm = Some(VH_FEMALE_STATURE_MM);
        let fit = c.fit();
        assert!(
            (fit.stature_scale - 1.0).abs() < 1e-4,
            "matching the female reference is identity scale"
        );
    }

    #[test]
    fn sitting_height_stretches_torso_not_legs() {
        let mut c = BodyConstitution::default();
        c.measurements.stature_mm = Some(1800);
        c.measurements.sitting_height_mm = Some(1100); // longer torso than 0.52*1800=936
        let fit = c.fit();
        assert!(fit.torso_scale_y > 1.05);
        assert!(fit.leg_scale_y < 1.0);
    }

    #[test]
    fn absent_uterus_hides_uterus_key() {
        let mut c = BodyConstitution::default();
        c.attributes.absent.push(AbsentPart {
            key: "3d-vh-f-uterus.glb".into(),
            reason: AbsenceReason::Surgical,
        });
        let fit = c.fit();
        assert!(fit.hides("uterus"));
        assert!(fit.hides("3d-vh-f-uterus.glb"));
        assert!(!fit.hides("heart"));
    }

    #[test]
    fn third_trimester_adds_abdomen_bulge() {
        let mut c = BodyConstitution::default();
        c.attributes.pregnancy = Some(Trimester::Third);
        let fit = c.fit();
        assert!(fit.pregnancy_abdomen > 0.3);
        let mid = fit.transform_point([0.0, 0.95, 0.0], [0.0, 0.0, 0.0], [0.4, 1.8, 0.3]);
        let crown = fit.transform_point([0.0, 1.8, 0.0], [0.0, 0.0, 0.0], [0.4, 1.8, 0.3]);
        assert!(
            mid[2] > crown[2],
            "abdomen should come forward more than the crown"
        );
    }

    #[test]
    fn validate_rejects_impossible_sitting_height() {
        let mut c = BodyConstitution::default();
        c.measurements.stature_mm = Some(1600);
        c.measurements.sitting_height_mm = Some(1600);
        assert!(c.validate().is_err());
    }

    #[test]
    fn validate_rejects_out_of_range_stature() {
        let mut c = BodyConstitution::default();
        c.measurements.stature_mm = Some(80);
        assert!(c.validate().is_err());
    }

    #[test]
    fn age_without_height_does_not_invent_stature() {
        let mut c = BodyConstitution::default();
        c.characteristics.age_months = Some(48);
        let fit = c.fit();
        assert!((fit.stature_scale - 1.0).abs() < f32::EPSILON);
        assert!(
            fit.honesty_notes
                .iter()
                .any(|n| n.contains("not guessed from age"))
        );
    }

    #[test]
    fn round_trip_json() {
        let mut c = BodyConstitution::default();
        c.measurements.stature_mm = Some(1720);
        c.measurements.waist_mm = Some(820);
        c.characteristics.karyotype = Some(Karyotype::Xx);
        c.attributes.absent.push(AbsentPart {
            key: "uterus".into(),
            reason: AbsenceReason::Surgical,
        });
        let s = serde_json::to_string(&c).unwrap();
        let back: BodyConstitution = serde_json::from_str(&s).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn garment_and_helmet_measures_do_not_change_fit() {
        let mut clothed = BodyConstitution::default();
        clothed.measurements.sleeve_mm = Some(620);
        clothed.measurements.neck_mm = Some(390);
        clothed.measurements.head_mm = Some(570);
        clothed.measurements.foot_left_mm = Some(265);
        clothed.measurements.foot_right_mm = Some(268);
        clothed.measurements.ring_finger_circ_mm = Some(55);
        let empty = BodyConstitution::default().fit();
        let fit = clothed.fit();
        assert!(fit.identity);
        assert_eq!(fit.stature_scale, empty.stature_scale);
        assert_eq!(fit.chest_radial, empty.chest_radial);
        assert!(
            fit.honesty_notes
                .iter()
                .any(|n| n.contains("Garment, footwear, helmet"))
        );
    }

    #[test]
    fn stage1_ssdm_projection_produces_non_uniform_bone_scalings_and_bmd() {
        use super::super::physiology::{PhysiologicalState, ReproductiveState};

        let mut young = BodyConstitution::default();
        young.characteristics.karyotype = Some(Karyotype::Xx);
        young.characteristics.age_months = Some(30 * 12);
        young.characteristics.dominant_side = Some(DominantSide::Right);
        young.measurements.stature_mm = Some(1760);
        young.measurements.sitting_height_mm = Some(930);
        young.measurements.waist_mm = Some(740);
        young.measurements.hip_mm = Some(1020);
        young.measurements.weight_g = Some(68_000);

        let cal_young = young.derive_anatomical_calibration(None, &[]);
        assert!(!cal_young.is_identity);
        assert_eq!(cal_young.reference_model, AnatomyModel::Female);
        assert!(cal_young.body_fit.stature_scale > 1.0);

        let femur_r = cal_young
            .bone_scalings
            .iter()
            .find(|b| b.bone_id == "femur_r")
            .unwrap();
        // Non-uniform scaling: longitudinal y differs from transverse x/z
        assert!(
            (femur_r.diag_scale[1] - femur_r.diag_scale[0]).abs() > 1e-3,
            "expected non-uniform S_b: {:?}",
            femur_r.diag_scale
        );
        assert!(femur_r.diag_scale.iter().all(|&s| s > 0.35 && s < 2.5));

        // Dominant right humerus has higher BMD & cortical thickness than left
        let hum_r = cal_young
            .bone_scalings
            .iter()
            .find(|b| b.bone_id == "humerus_r")
            .unwrap();
        let hum_l = cal_young
            .bone_scalings
            .iter()
            .find(|b| b.bone_id == "humerus_l")
            .unwrap();
        assert!(hum_r.apparent_density_g_cm3 > hum_l.apparent_density_g_cm3);
        assert!(hum_r.cortical_thickness_mm > hum_l.cortical_thickness_mm);

        // Older + postmenopausal state lowers lumbar BMD & elastic modulus E(rho_app)
        let mut older = young.clone();
        older.characteristics.age_months = Some(75 * 12);
        let cal_older = older.derive_anatomical_calibration(
            Some(PhysiologicalState::Reproductive(
                ReproductiveState::PostMenopause,
            )),
            &[],
        );
        let lumbar_young = cal_young
            .bone_scalings
            .iter()
            .find(|b| b.bone_id == "lumbar_spine")
            .unwrap();
        let lumbar_older = cal_older
            .bone_scalings
            .iter()
            .find(|b| b.bone_id == "lumbar_spine")
            .unwrap();
        assert!(lumbar_older.apparent_density_g_cm3 < lumbar_young.apparent_density_g_cm3);
        assert!(lumbar_older.elastic_modulus_gpa < lumbar_young.elastic_modulus_gpa);
    }

    #[test]
    fn stage2_stiffness_penalized_registration_weights_confidence_and_bmd() {
        let mut young = BodyConstitution::default();
        young.characteristics.age_months = Some(28 * 12);
        young.measurements.stature_mm = Some(1800);

        let mut older = young.clone();
        older.characteristics.age_months = Some(80 * 12);

        let obs_fresh = [CalibrationObservation {
            feature_id: "lumbar_spine".into(),
            region_bone_id: Some("lumbar_spine".into()),
            target_displacement_m: Some([0.0, 0.0, 0.08]),
            base_confidence: 0.95,
            elapsed_days: 1.0,
            half_life_days: 180.0,
        }];
        let obs_stale = [CalibrationObservation {
            feature_id: "lumbar_spine".into(),
            region_bone_id: Some("lumbar_spine".into()),
            target_displacement_m: Some([0.0, 0.0, 0.08]),
            base_confidence: 0.95,
            elapsed_days: 3650.0,
            half_life_days: 180.0,
        }];

        let res_young_fresh = young.derive_anatomical_calibration(None, &obs_fresh);
        let res_older_fresh = older.derive_anatomical_calibration(None, &obs_fresh);
        let res_young_stale = young.derive_anatomical_calibration(None, &obs_stale);

        let c_yf = res_young_fresh
            .bone_corrections
            .iter()
            .find(|b| b.bone_id == "lumbar_spine")
            .unwrap();
        let c_of = res_older_fresh
            .bone_corrections
            .iter()
            .find(|b| b.bone_id == "lumbar_spine")
            .unwrap();
        let c_ys = res_young_stale
            .bone_corrections
            .iter()
            .find(|b| b.bone_id == "lumbar_spine")
            .unwrap();

        assert!(c_yf.observation_confidence > c_ys.observation_confidence);
        assert!(c_yf.translation_m[2] > c_ys.translation_m[2]);
        // Lower BMD (lower E) yields more to the same observation displacement than stiffer bone
        assert!(c_of.bone_stiffness_gpa < c_yf.bone_stiffness_gpa);
        assert!(c_of.translation_m[2] > c_yf.translation_m[2]);
        assert!(c_yf.jacobian_det > 0.0 && c_of.jacobian_det > 0.0);
    }

    #[test]
    fn stage3_harmonic_tps_preserves_affine_and_positive_jacobian_det() {
        let cage = [
            [-0.2, 0.8, -0.15],
            [0.2, 0.8, -0.15],
            [-0.2, 1.4, -0.15],
            [0.2, 1.4, -0.15],
            [-0.2, 0.8, 0.15],
            [0.2, 0.8, 0.15],
            [-0.2, 1.4, 0.15],
            [0.2, 1.4, 0.15],
        ];
        // Pure anisotropic affine scaling (1.15, 1.10, 1.25) -> det = 1.58125
        let displaced_affine: Vec<[f64; 3]> = cage
            .iter()
            .map(|p| [p[0] * 1.15, p[1] * 1.10, p[2] * 1.25 + 0.02])
            .collect();
        let (pt, det) = harmonic_tps_organ_transfer(&cage, &displaced_affine, [0.04, 1.15, 0.03]);
        assert!((pt[0] - 0.04 * 1.15).abs() < 1e-6);
        assert!((pt[1] - 1.15 * 1.10).abs() < 1e-6);
        assert!((pt[2] - (0.03 * 1.25 + 0.02)).abs() < 1e-6);
        assert!((det - (1.15 * 1.10 * 1.25)).abs() < 1e-5, "det={det}");

        // Extreme inverted cage must still preserve det(J) > 0 (no organ cavity foldover)
        let displaced_inverted: Vec<[f64; 3]> = cage
            .iter()
            .map(|p| [-p[0] * 2.0, p[1], -p[2] * 2.0])
            .collect();
        let (safe_pt, safe_det) =
            harmonic_tps_organ_transfer(&cage, &displaced_inverted, [0.02, 1.10, 0.02]);
        assert!(safe_pt.iter().all(|v| v.is_finite()));
        assert!(safe_det > 0.0, "Jacobian det must stay > 0, got {safe_det}");

        // Full 3-stage pipeline preserves det(J) > 0 across all internal organ cavities
        let mut c = BodyConstitution::default();
        c.characteristics.karyotype = Some(Karyotype::Xx);
        c.measurements.stature_mm = Some(1700);
        c.measurements.chest_mm = Some(980);
        c.measurements.waist_mm = Some(860);
        c.measurements.hip_mm = Some(1080);
        c.attributes.pregnancy = Some(Trimester::Third);
        let full = c.derive_anatomical_calibration(None, &[]);
        assert!(!full.organ_transfers.is_empty());
        for t in &full.organ_transfers {
            assert!(
                t.jacobian_det > 0.0,
                "organ {} had non-positive det(J)={}",
                t.organ_id,
                t.jacobian_det
            );
            assert!(t.calibrated_point_m.iter().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn ethnicity_does_not_alter_3stage_calibration_geometry() {
        let mut with_eth = BodyConstitution::default();
        with_eth.measurements.stature_mm = Some(1740);
        with_eth.measurements.waist_mm = Some(810);
        with_eth
            .knowledge
            .ethnicities
            .push(super::super::knowledge_context::EthnicityAffiliation::declared("Greek").unwrap());

        let mut without_eth = BodyConstitution::default();
        without_eth.measurements.stature_mm = Some(1740);
        without_eth.measurements.waist_mm = Some(810);

        let a = with_eth.derive_anatomical_calibration(None, &[]);
        let b = without_eth.derive_anatomical_calibration(None, &[]);
        assert_eq!(a.bone_scalings, b.bone_scalings);
        assert_eq!(a.organ_transfers, b.organ_transfers);
    }
}
