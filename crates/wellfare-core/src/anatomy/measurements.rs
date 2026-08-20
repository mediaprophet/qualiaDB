//! Comprehensive **body measurements** — one integer record for anatomy fit, clothing,
//! footwear, helmets, gloves, rings, and eyewear.
//!
//! Vocabulary follows ISO 8559-1 / ISO 7250-1 (garment anthropometry) plus Brannock-style
//! foot measures and standard headform arcs. This is **not** a certification claim; the
//! names are so a clothing or helmet module can consume the same numbers the person typed.
//!
//! Every field is optional. Absence is not an assumption. Anatomy [`super::constitution::BodyFit`]
//! currently uses only stature, sitting height, inseam, arm span, shoulder, chest, waist,
//! and hip. The rest are stored and listed by use — they do not invent a morph.

use serde::{Deserialize, Serialize};

/// Soft clamps so a typo cannot explode a consumer. Values outside are rejected on save.
pub const STATURE_MM_RANGE: (u16, u16) = (300, 2500);
pub const WEIGHT_G_RANGE: (u32, u32) = (500, 400_000);
/// Legacy torso-girth window (kept for callers). Per-field ranges live on the catalog.
pub const CIRC_MM_RANGE: (u16, u16) = (80, 2500);

const FIT: u16 = 1;
const CLOTH: u16 = 2;
const FOOTW: u16 = 4;
const HEADW: u16 = 8;
const GLOVE: u16 = 16;
const EYE: u16 = 32;
const RING: u16 = 64;

/// Who consumes this measure. A field can serve several.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementUse {
    AnatomyFit,
    Clothing,
    Footwear,
    Headwear,
    Gloves,
    Eyewear,
    Rings,
}

/// Form / catalog grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementGroup {
    WholeBody,
    Torso,
    Arms,
    Legs,
    Head,
    Hands,
    Feet,
}

impl MeasurementGroup {
    pub fn label(self) -> &'static str {
        match self {
            Self::WholeBody => "Whole body",
            Self::Torso => "Torso and clothing",
            Self::Arms => "Arms and sleeves",
            Self::Legs => "Legs and trousers",
            Self::Head => "Head, helmets, and hats",
            Self::Hands => "Hands, gloves, and rings",
            Self::Feet => "Feet and footwear",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::WholeBody => {
                "Height and mass for the body, plus the long measures used to proportion it."
            }
            Self::Torso => {
                "Shirt, jacket, dress, and bra measures (ISO 8559-1 names). Underbust is stored; cup size is a garment convention, not a body measure."
            }
            Self::Arms => {
                "Sleeve length and arm girths for shirts, coats, and compression sleeves."
            }
            Self::Legs => "Inseam, outseam, and limb girths for trousers, shorts, and hosiery.",
            Self::Head => {
                "Helmets, hats, and some eyewear. Circumference plus length/breadth/arcs — not a hat-size letter."
            }
            Self::Hands => "Gloves and rings. Left/right ring girth when the two hands differ.",
            Self::Feet => {
                "Footwear lasts (Brannock-style). Prefer left and right; a single foot length is the fallback."
            }
        }
    }
}

/// How the person types the number. Stored unit is always mm or g.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementInputUnit {
    Cm,
    Kg,
    Mm,
}

/// One curated measurement the person may declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasurementSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub group: MeasurementGroup,
    pub lo: u32,
    pub hi: u32,
    pub input: MeasurementInputUnit,
    pub uses: u16,
    pub hint: &'static str,
}

impl MeasurementSpec {
    pub fn uses_list(self) -> Vec<MeasurementUse> {
        decode_uses(self.uses)
    }

    pub fn serves(self, use_: MeasurementUse) -> bool {
        self.uses & bit(use_) != 0
    }

    pub fn store_unit(self) -> &'static str {
        match self.input {
            MeasurementInputUnit::Kg => "g",
            _ => "mm",
        }
    }
}

/// Lengths and girths the person may declare. Units are millimetres / grams so the
/// stored numbers are integers (no float health arithmetic in the record).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyMeasurements {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stature_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight_g: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sitting_height_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cervical_height_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crotch_height_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knee_height_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ankle_height_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waist_height_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arm_span_mm: Option<u16>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub neck_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub neck_base_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shoulder_width_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub across_back_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub across_chest_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chest_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underbust_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waist_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abdomen_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hip_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nape_to_waist_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub front_waist_length_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rise_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outseam_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shoulder_girth_mm: Option<u16>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sleeve_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper_arm_length_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underarm_length_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elbow_to_wrist_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub biceps_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elbow_girth_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forearm_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrist_mm: Option<u16>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inseam_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thigh_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mid_thigh_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knee_girth_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calf_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ankle_mm: Option<u16>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_length_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_breadth_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_height_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_length_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bizygomatic_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sagittal_arc_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitragion_arc_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpupillary_mm: Option<u16>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand_length_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palm_length_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand_breadth_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand_circ_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub middle_finger_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumb_circ_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ring_finger_circ_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ring_finger_left_circ_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ring_finger_right_circ_mm: Option<u16>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foot_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foot_left_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foot_right_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foot_width_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foot_width_left_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foot_width_right_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heel_width_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ball_girth_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instep_girth_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arch_length_mm: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toe_height_mm: Option<u16>,
}

impl BodyMeasurements {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    pub fn get(&self, id: &str) -> Option<u32> {
        let v = serde_json::to_value(self).ok()?;
        v.get(id)?.as_u64().map(|n| n as u32)
    }

    pub fn declared(&self) -> Vec<(&'static MeasurementSpec, u32)> {
        MEASUREMENT_CATALOG
            .iter()
            .filter_map(|spec| self.get(spec.id).map(|n| (spec, n)))
            .collect()
    }

    pub fn values_for(&self, use_: MeasurementUse) -> Vec<(&'static MeasurementSpec, u32)> {
        self.declared()
            .into_iter()
            .filter(|(spec, _)| spec.serves(use_))
            .collect()
    }

    pub fn has_beyond_anatomy_fit(&self) -> bool {
        self.declared()
            .into_iter()
            .any(|(spec, _)| !spec.serves(MeasurementUse::AnatomyFit))
    }

    /// Single foot length for a consumer that does not handle laterality.
    pub fn foot_length_mm(&self) -> Option<u16> {
        self.foot_mm
            .or_else(|| max_opt(self.foot_left_mm, self.foot_right_mm))
    }

    pub fn foot_width_for_last(&self) -> Option<u16> {
        self.foot_width_mm
            .or_else(|| max_opt(self.foot_width_left_mm, self.foot_width_right_mm))
    }

    pub fn validate(&self) -> Result<(), String> {
        for spec in MEASUREMENT_CATALOG {
            if let Some(n) = self.get(spec.id) {
                if n < spec.lo || n > spec.hi {
                    return Err(format!(
                        "{} {n} is outside {}–{} {}",
                        spec.id,
                        spec.lo,
                        spec.hi,
                        spec.store_unit()
                    ));
                }
            }
        }
        if let (Some(stature), Some(sit)) = (self.stature_mm, self.sitting_height_mm) {
            if sit >= stature {
                return Err("sitting height must be shorter than standing height".into());
            }
        }
        Ok(())
    }
}

/// Authoritative field list. New garment/helmet measures get a row here — not a vendor struct.
pub const MEASUREMENT_CATALOG: &[MeasurementSpec] = &[
    // Whole body
    spec(
        "stature_mm",
        "Standing height",
        MeasurementGroup::WholeBody,
        300,
        2500,
        MeasurementInputUnit::Cm,
        FIT | CLOTH,
        "Crown to floor.",
    ),
    spec(
        "weight_g",
        "Body mass",
        MeasurementGroup::WholeBody,
        500,
        400_000,
        MeasurementInputUnit::Kg,
        FIT | CLOTH,
        "Stored in grams.",
    ),
    spec(
        "sitting_height_mm",
        "Sitting height",
        MeasurementGroup::WholeBody,
        200,
        1600,
        MeasurementInputUnit::Cm,
        FIT | CLOTH,
        "Crown to seat.",
    ),
    spec(
        "cervical_height_mm",
        "Cervical height",
        MeasurementGroup::WholeBody,
        200,
        1800,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Nape to floor, standing.",
    ),
    spec(
        "crotch_height_mm",
        "Crotch height",
        MeasurementGroup::WholeBody,
        200,
        1200,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Crotch to floor.",
    ),
    spec(
        "knee_height_mm",
        "Knee height",
        MeasurementGroup::WholeBody,
        150,
        800,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Knee crease to floor.",
    ),
    spec(
        "ankle_height_mm",
        "Ankle height",
        MeasurementGroup::WholeBody,
        40,
        200,
        MeasurementInputUnit::Cm,
        CLOTH | FOOTW,
        "Lateral malleolus to floor.",
    ),
    spec(
        "waist_height_mm",
        "Waist height",
        MeasurementGroup::WholeBody,
        300,
        1500,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Natural waist to floor.",
    ),
    spec(
        "arm_span_mm",
        "Arm span",
        MeasurementGroup::WholeBody,
        300,
        2800,
        MeasurementInputUnit::Cm,
        FIT | CLOTH,
        "Fingertip to fingertip.",
    ),
    // Torso / clothing
    spec(
        "neck_mm",
        "Neck circumference",
        MeasurementGroup::Torso,
        150,
        700,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Shirt collar.",
    ),
    spec(
        "neck_base_mm",
        "Neck-base circumference",
        MeasurementGroup::Torso,
        200,
        800,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Around the neck-shoulder join.",
    ),
    spec(
        "shoulder_width_mm",
        "Shoulder breadth",
        MeasurementGroup::Torso,
        200,
        700,
        MeasurementInputUnit::Cm,
        FIT | CLOTH,
        "Biacromial, bone to bone.",
    ),
    spec(
        "across_back_mm",
        "Across back",
        MeasurementGroup::Torso,
        200,
        700,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Armscye to armscye, back.",
    ),
    spec(
        "across_chest_mm",
        "Across chest",
        MeasurementGroup::Torso,
        200,
        700,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Armscye to armscye, front.",
    ),
    spec(
        "chest_mm",
        "Chest / bust circumference",
        MeasurementGroup::Torso,
        300,
        2000,
        MeasurementInputUnit::Cm,
        FIT | CLOTH,
        "Fullest bust / chest.",
    ),
    spec(
        "underbust_mm",
        "Underbust circumference",
        MeasurementGroup::Torso,
        300,
        1800,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Band measurement. Not a cup size.",
    ),
    spec(
        "waist_mm",
        "Waist circumference",
        MeasurementGroup::Torso,
        300,
        2000,
        MeasurementInputUnit::Cm,
        FIT | CLOTH,
        "Natural waist.",
    ),
    spec(
        "abdomen_mm",
        "Abdomen / high-hip circumference",
        MeasurementGroup::Torso,
        300,
        2200,
        MeasurementInputUnit::Cm,
        CLOTH,
        "About 8 cm below the waist.",
    ),
    spec(
        "hip_mm",
        "Hip / seat circumference",
        MeasurementGroup::Torso,
        300,
        2200,
        MeasurementInputUnit::Cm,
        FIT | CLOTH,
        "Fullest seat.",
    ),
    spec(
        "nape_to_waist_mm",
        "Nape to waist",
        MeasurementGroup::Torso,
        200,
        700,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Cervicale to back waist.",
    ),
    spec(
        "front_waist_length_mm",
        "Front waist length",
        MeasurementGroup::Torso,
        200,
        700,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Side-neck to front waist.",
    ),
    spec(
        "rise_mm",
        "Rise / crotch depth",
        MeasurementGroup::Torso,
        150,
        500,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Seated waist to chair, or crotch depth.",
    ),
    spec(
        "outseam_mm",
        "Outseam",
        MeasurementGroup::Torso,
        400,
        1400,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Waist to ankle, outside leg.",
    ),
    spec(
        "shoulder_girth_mm",
        "Shoulder / overarm girth",
        MeasurementGroup::Torso,
        600,
        2200,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Around both shoulders and chest.",
    ),
    // Arms
    spec(
        "sleeve_mm",
        "Sleeve length",
        MeasurementGroup::Arms,
        300,
        900,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Shoulder point to wrist.",
    ),
    spec(
        "upper_arm_length_mm",
        "Upper-arm length",
        MeasurementGroup::Arms,
        150,
        500,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Shoulder to elbow.",
    ),
    spec(
        "underarm_length_mm",
        "Underarm length",
        MeasurementGroup::Arms,
        250,
        800,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Armpit to wrist.",
    ),
    spec(
        "elbow_to_wrist_mm",
        "Elbow to wrist",
        MeasurementGroup::Arms,
        120,
        400,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Forearm length.",
    ),
    spec(
        "biceps_mm",
        "Upper-arm / biceps circumference",
        MeasurementGroup::Arms,
        120,
        700,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Fullest upper arm.",
    ),
    spec(
        "elbow_girth_mm",
        "Elbow circumference",
        MeasurementGroup::Arms,
        120,
        500,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Around the bent elbow.",
    ),
    spec(
        "forearm_mm",
        "Forearm circumference",
        MeasurementGroup::Arms,
        100,
        500,
        MeasurementInputUnit::Cm,
        CLOTH | GLOVE,
        "Fullest forearm.",
    ),
    spec(
        "wrist_mm",
        "Wrist circumference",
        MeasurementGroup::Arms,
        80,
        300,
        MeasurementInputUnit::Cm,
        CLOTH | GLOVE,
        "Just distal to the styloid.",
    ),
    // Legs
    spec(
        "inseam_mm",
        "Inside leg / inseam",
        MeasurementGroup::Legs,
        300,
        1200,
        MeasurementInputUnit::Cm,
        FIT | CLOTH,
        "Crotch to floor, or trouser inseam.",
    ),
    spec(
        "thigh_mm",
        "Thigh circumference",
        MeasurementGroup::Legs,
        250,
        1200,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Fullest thigh.",
    ),
    spec(
        "mid_thigh_mm",
        "Mid-thigh circumference",
        MeasurementGroup::Legs,
        200,
        1100,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Halfway hip to knee.",
    ),
    spec(
        "knee_girth_mm",
        "Knee circumference",
        MeasurementGroup::Legs,
        200,
        800,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Around the knee.",
    ),
    spec(
        "calf_mm",
        "Calf circumference",
        MeasurementGroup::Legs,
        180,
        700,
        MeasurementInputUnit::Cm,
        CLOTH,
        "Fullest calf.",
    ),
    spec(
        "ankle_mm",
        "Ankle circumference",
        MeasurementGroup::Legs,
        120,
        400,
        MeasurementInputUnit::Cm,
        CLOTH | FOOTW,
        "Minimum ankle girth.",
    ),
    // Head / helmet / eyewear
    spec(
        "head_mm",
        "Head circumference",
        MeasurementGroup::Head,
        280,
        750,
        MeasurementInputUnit::Cm,
        HEADW,
        "Above the brow, helmet tape.",
    ),
    spec(
        "head_length_mm",
        "Head length",
        MeasurementGroup::Head,
        120,
        280,
        MeasurementInputUnit::Cm,
        HEADW,
        "Glabella to occiput.",
    ),
    spec(
        "head_breadth_mm",
        "Head breadth",
        MeasurementGroup::Head,
        100,
        220,
        MeasurementInputUnit::Cm,
        HEADW,
        "Maximum cranial breadth.",
    ),
    spec(
        "head_height_mm",
        "Head height",
        MeasurementGroup::Head,
        150,
        300,
        MeasurementInputUnit::Cm,
        HEADW,
        "Vertex to menton, or tragion to vertex.",
    ),
    spec(
        "face_length_mm",
        "Face length",
        MeasurementGroup::Head,
        80,
        180,
        MeasurementInputUnit::Cm,
        HEADW | EYE,
        "Nasion to menton.",
    ),
    spec(
        "bizygomatic_mm",
        "Face width",
        MeasurementGroup::Head,
        90,
        180,
        MeasurementInputUnit::Cm,
        HEADW | EYE,
        "Bizygomatic breadth.",
    ),
    spec(
        "sagittal_arc_mm",
        "Sagittal arc",
        MeasurementGroup::Head,
        250,
        450,
        MeasurementInputUnit::Cm,
        HEADW,
        "Glabella over vertex to inion.",
    ),
    spec(
        "bitragion_arc_mm",
        "Bitragion coronal arc",
        MeasurementGroup::Head,
        250,
        450,
        MeasurementInputUnit::Cm,
        HEADW,
        "Ear to ear over the crown.",
    ),
    spec(
        "interpupillary_mm",
        "Interpupillary distance",
        MeasurementGroup::Head,
        40,
        90,
        MeasurementInputUnit::Mm,
        EYE,
        "IPD in millimetres.",
    ),
    // Hands
    spec(
        "hand_length_mm",
        "Hand length",
        MeasurementGroup::Hands,
        100,
        280,
        MeasurementInputUnit::Cm,
        GLOVE,
        "Wrist crease to middle fingertip.",
    ),
    spec(
        "palm_length_mm",
        "Palm length",
        MeasurementGroup::Hands,
        60,
        160,
        MeasurementInputUnit::Cm,
        GLOVE,
        "Wrist crease to palmar digital crease.",
    ),
    spec(
        "hand_breadth_mm",
        "Hand breadth",
        MeasurementGroup::Hands,
        50,
        140,
        MeasurementInputUnit::Cm,
        GLOVE,
        "Across the metacarpals.",
    ),
    spec(
        "hand_circ_mm",
        "Hand circumference",
        MeasurementGroup::Hands,
        120,
        320,
        MeasurementInputUnit::Cm,
        GLOVE,
        "Around the knuckles, excluding thumb.",
    ),
    spec(
        "middle_finger_mm",
        "Middle-finger length",
        MeasurementGroup::Hands,
        40,
        120,
        MeasurementInputUnit::Cm,
        GLOVE,
        "Proximal crease to tip.",
    ),
    spec(
        "thumb_circ_mm",
        "Thumb circumference",
        MeasurementGroup::Hands,
        40,
        120,
        MeasurementInputUnit::Cm,
        GLOVE,
        "Mid-thumb.",
    ),
    spec(
        "ring_finger_circ_mm",
        "Ring-finger circumference",
        MeasurementGroup::Hands,
        35,
        100,
        MeasurementInputUnit::Mm,
        RING,
        "Preferred ring finger, millimetres.",
    ),
    spec(
        "ring_finger_left_circ_mm",
        "Left ring-finger circumference",
        MeasurementGroup::Hands,
        35,
        100,
        MeasurementInputUnit::Mm,
        RING,
        "When the hands differ.",
    ),
    spec(
        "ring_finger_right_circ_mm",
        "Right ring-finger circumference",
        MeasurementGroup::Hands,
        35,
        100,
        MeasurementInputUnit::Mm,
        RING,
        "When the hands differ.",
    ),
    // Feet
    spec(
        "foot_mm",
        "Foot length",
        MeasurementGroup::Feet,
        80,
        400,
        MeasurementInputUnit::Cm,
        FOOTW,
        "Heel to longest toe. Fallback if left/right omitted.",
    ),
    spec(
        "foot_left_mm",
        "Left foot length",
        MeasurementGroup::Feet,
        80,
        400,
        MeasurementInputUnit::Cm,
        FOOTW,
        "Prefer this plus right.",
    ),
    spec(
        "foot_right_mm",
        "Right foot length",
        MeasurementGroup::Feet,
        80,
        400,
        MeasurementInputUnit::Cm,
        FOOTW,
        "Prefer this plus left.",
    ),
    spec(
        "foot_width_mm",
        "Foot width (ball)",
        MeasurementGroup::Feet,
        50,
        160,
        MeasurementInputUnit::Cm,
        FOOTW,
        "Ball width. Fallback if left/right omitted.",
    ),
    spec(
        "foot_width_left_mm",
        "Left foot width",
        MeasurementGroup::Feet,
        50,
        160,
        MeasurementInputUnit::Cm,
        FOOTW,
        "",
    ),
    spec(
        "foot_width_right_mm",
        "Right foot width",
        MeasurementGroup::Feet,
        50,
        160,
        MeasurementInputUnit::Cm,
        FOOTW,
        "",
    ),
    spec(
        "heel_width_mm",
        "Heel width",
        MeasurementGroup::Feet,
        40,
        120,
        MeasurementInputUnit::Cm,
        FOOTW,
        "",
    ),
    spec(
        "ball_girth_mm",
        "Ball girth",
        MeasurementGroup::Feet,
        120,
        320,
        MeasurementInputUnit::Cm,
        FOOTW,
        "Around the metatarsal heads.",
    ),
    spec(
        "instep_girth_mm",
        "Instep girth",
        MeasurementGroup::Feet,
        120,
        320,
        MeasurementInputUnit::Cm,
        FOOTW,
        "Around the arch / instep.",
    ),
    spec(
        "arch_length_mm",
        "Arch length",
        MeasurementGroup::Feet,
        80,
        280,
        MeasurementInputUnit::Cm,
        FOOTW,
        "Heel to ball (Brannock arch).",
    ),
    spec(
        "toe_height_mm",
        "Toe-box height",
        MeasurementGroup::Feet,
        15,
        60,
        MeasurementInputUnit::Mm,
        FOOTW,
        "Vertical clearance at the toes, millimetres.",
    ),
];

const fn spec(
    id: &'static str,
    label: &'static str,
    group: MeasurementGroup,
    lo: u32,
    hi: u32,
    input: MeasurementInputUnit,
    uses: u16,
    hint: &'static str,
) -> MeasurementSpec {
    MeasurementSpec {
        id,
        label,
        group,
        lo,
        hi,
        input,
        uses,
        hint,
    }
}

const fn bit(use_: MeasurementUse) -> u16 {
    match use_ {
        MeasurementUse::AnatomyFit => FIT,
        MeasurementUse::Clothing => CLOTH,
        MeasurementUse::Footwear => FOOTW,
        MeasurementUse::Headwear => HEADW,
        MeasurementUse::Gloves => GLOVE,
        MeasurementUse::Eyewear => EYE,
        MeasurementUse::Rings => RING,
    }
}

fn decode_uses(bits: u16) -> Vec<MeasurementUse> {
    [
        MeasurementUse::AnatomyFit,
        MeasurementUse::Clothing,
        MeasurementUse::Footwear,
        MeasurementUse::Headwear,
        MeasurementUse::Gloves,
        MeasurementUse::Eyewear,
        MeasurementUse::Rings,
    ]
    .into_iter()
    .filter(|u| bits & bit(*u) != 0)
    .collect()
}

fn max_opt(a: Option<u16>, b: Option<u16>) -> Option<u16> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

/// Grouped catalog for the Care form and for clothing/helmet consumers.
pub fn measurement_catalog_json() -> serde_json::Value {
    let mut groups = Vec::new();
    for group in [
        MeasurementGroup::WholeBody,
        MeasurementGroup::Torso,
        MeasurementGroup::Arms,
        MeasurementGroup::Legs,
        MeasurementGroup::Head,
        MeasurementGroup::Hands,
        MeasurementGroup::Feet,
    ] {
        let fields: Vec<serde_json::Value> = MEASUREMENT_CATALOG
            .iter()
            .filter(|s| s.group == group)
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "label": s.label,
                    "hint": s.hint,
                    "lo": s.lo,
                    "hi": s.hi,
                    "input_unit": s.input,
                    "store_unit": s.store_unit(),
                    "uses": s.uses_list(),
                })
            })
            .collect();
        groups.push(serde_json::json!({
            "id": group,
            "label": group.label(),
            "hint": group.hint(),
            "fields": fields,
        }));
    }
    serde_json::json!({
        "version": 1,
        "honesty": "Fill in what you have. Clothing, footwear, helmets, gloves, rings, and glasses in Qualia read this same record. Anatomy currently reshapes the reference body from height, sitting height, inseam, arm span, shoulders, chest, waist, and hip only.",
        "groups": groups,
        "field_count": MEASUREMENT_CATALOG.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_ids_are_unique_and_cover_the_struct() {
        let mut ids = std::collections::BTreeSet::new();
        for spec in MEASUREMENT_CATALOG {
            assert!(ids.insert(spec.id), "duplicate catalog id {}", spec.id);
            assert!(spec.lo < spec.hi);
        }
        let populated = fully_populated();
        let json = serde_json::to_value(&populated).unwrap();
        let struct_keys: std::collections::BTreeSet<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        let catalog_keys: std::collections::BTreeSet<&str> =
            MEASUREMENT_CATALOG.iter().map(|s| s.id).collect();
        assert_eq!(
            struct_keys, catalog_keys,
            "every BodyMeasurements field must be in MEASUREMENT_CATALOG and vice versa"
        );
    }

    #[test]
    fn clothing_footwear_and_helmet_slices_are_useful() {
        let mut m = BodyMeasurements::default();
        m.chest_mm = Some(980);
        m.sleeve_mm = Some(620);
        m.neck_mm = Some(390);
        m.foot_left_mm = Some(265);
        m.foot_right_mm = Some(268);
        m.head_mm = Some(570);
        m.head_length_mm = Some(196);
        assert!(
            m.values_for(MeasurementUse::Clothing)
                .iter()
                .any(|(s, _)| s.id == "sleeve_mm")
        );
        assert!(
            m.values_for(MeasurementUse::Footwear)
                .iter()
                .any(|(s, _)| s.id == "foot_right_mm")
        );
        assert_eq!(m.foot_length_mm(), Some(268));
        assert!(
            m.values_for(MeasurementUse::Headwear)
                .iter()
                .any(|(s, _)| s.id == "head_mm")
        );
        assert!(
            !m.values_for(MeasurementUse::AnatomyFit)
                .iter()
                .any(|(s, _)| s.id == "sleeve_mm")
        );
    }

    #[test]
    fn validate_uses_per_field_ranges() {
        let mut m = BodyMeasurements::default();
        m.ring_finger_circ_mm = Some(2000);
        assert!(m.validate().is_err());
        m.ring_finger_circ_mm = Some(55);
        assert!(m.validate().is_ok());
        m.stature_mm = Some(1600);
        m.sitting_height_mm = Some(1600);
        assert!(m.validate().is_err());
    }

    #[test]
    fn legacy_json_still_loads() {
        let v: BodyMeasurements =
            serde_json::from_str(r#"{"stature_mm":1720,"chest_mm":940,"foot_mm":255}"#).unwrap();
        assert_eq!(v.stature_mm, Some(1720));
        assert_eq!(v.chest_mm, Some(940));
        assert_eq!(v.foot_mm, Some(255));
        assert!(v.sleeve_mm.is_none());
    }

    #[test]
    fn catalog_json_lists_every_group() {
        let j = measurement_catalog_json();
        assert_eq!(j["field_count"], MEASUREMENT_CATALOG.len());
        assert_eq!(j["groups"].as_array().unwrap().len(), 7);
        assert!(MEASUREMENT_CATALOG.len() >= 60);
    }

    fn fully_populated() -> BodyMeasurements {
        let mut obj = serde_json::Map::new();
        for spec in MEASUREMENT_CATALOG {
            obj.insert(spec.id.to_string(), serde_json::json!(spec.lo + 1));
        }
        serde_json::from_value(serde_json::Value::Object(obj)).unwrap()
    }
}
