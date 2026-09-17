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

    pub fn apply_in_place(&self, positions: &mut [[f32; 3]], gmin: [f32; 3], gmax: [f32; 3]) {
        if self.identity {
            return;
        }
        for p in positions.iter_mut() {
            *p = self.transform_point(*p, gmin, gmax);
        }
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
}
