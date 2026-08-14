//! Presentation catalog for the constitution form.
//!
//! The engine catalog in `wellfare-core::anatomy::MEASUREMENT_CATALOG` is authoritative
//! for validation and clothing/footwear/helmet consumers. This table is the Care surface
//! (studio cannot depend on wellfare-core on wasm32). Keep ids aligned.

#[derive(Clone, Copy)]
pub struct MeasureField {
    pub id: &'static str,
    pub label: &'static str,
    pub group: MeasureGroup,
    pub input: MeasureInput,
    pub hint: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasureGroup {
    WholeBody,
    Torso,
    Arms,
    Legs,
    Head,
    Hands,
    Feet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasureInput {
    Cm,
    Kg,
    Mm,
}

impl MeasureGroup {
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
                "Height and mass, plus the long measures used to proportion the body."
            }
            Self::Torso => {
                "Shirt, jacket, dress, and bra measures. Underbust is stored; cup size is a garment convention, not a body measure."
            }
            Self::Arms => "Sleeve length and arm girths for shirts, coats, and sleeves.",
            Self::Legs => "Inseam, outseam, and limb girths for trousers and hosiery.",
            Self::Head => "Helmets, hats, and glasses. Circumference plus length, breadth, and arcs.",
            Self::Hands => "Gloves and rings. Left/right ring girth when the two hands differ.",
            Self::Feet => {
                "Footwear lasts. Prefer left and right; a single foot length is the fallback."
            }
        }
    }

    pub fn all() -> [MeasureGroup; 7] {
        [
            Self::WholeBody,
            Self::Torso,
            Self::Arms,
            Self::Legs,
            Self::Head,
            Self::Hands,
            Self::Feet,
        ]
    }
}

impl MeasureInput {
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Cm => "cm",
            Self::Kg => "kg",
            Self::Mm => "mm",
        }
    }
}

pub const MEASURE_FIELDS: &[MeasureField] = &[
    f("stature_mm", "Standing height", MeasureGroup::WholeBody, MeasureInput::Cm, "Crown to floor."),
    f("weight_g", "Body mass", MeasureGroup::WholeBody, MeasureInput::Kg, "Stored as grams."),
    f("sitting_height_mm", "Sitting height", MeasureGroup::WholeBody, MeasureInput::Cm, "Crown to seat."),
    f("cervical_height_mm", "Cervical height", MeasureGroup::WholeBody, MeasureInput::Cm, "Nape to floor."),
    f("crotch_height_mm", "Crotch height", MeasureGroup::WholeBody, MeasureInput::Cm, "Crotch to floor."),
    f("knee_height_mm", "Knee height", MeasureGroup::WholeBody, MeasureInput::Cm, "Knee to floor."),
    f("ankle_height_mm", "Ankle height", MeasureGroup::WholeBody, MeasureInput::Cm, "Malleolus to floor."),
    f("waist_height_mm", "Waist height", MeasureGroup::WholeBody, MeasureInput::Cm, "Natural waist to floor."),
    f("arm_span_mm", "Arm span", MeasureGroup::WholeBody, MeasureInput::Cm, "Fingertip to fingertip."),
    f("neck_mm", "Neck circumference", MeasureGroup::Torso, MeasureInput::Cm, "Shirt collar."),
    f("neck_base_mm", "Neck-base circumference", MeasureGroup::Torso, MeasureInput::Cm, "Neck–shoulder join."),
    f("shoulder_width_mm", "Shoulder breadth", MeasureGroup::Torso, MeasureInput::Cm, "Biacromial."),
    f("across_back_mm", "Across back", MeasureGroup::Torso, MeasureInput::Cm, "Armscye to armscye, back."),
    f("across_chest_mm", "Across chest", MeasureGroup::Torso, MeasureInput::Cm, "Armscye to armscye, front."),
    f("chest_mm", "Chest / bust", MeasureGroup::Torso, MeasureInput::Cm, "Fullest chest or bust."),
    f("underbust_mm", "Underbust", MeasureGroup::Torso, MeasureInput::Cm, "Band. Not a cup size."),
    f("waist_mm", "Waist circumference", MeasureGroup::Torso, MeasureInput::Cm, "Natural waist."),
    f("abdomen_mm", "Abdomen / high hip", MeasureGroup::Torso, MeasureInput::Cm, "About 8 cm below the waist."),
    f("hip_mm", "Hip / seat", MeasureGroup::Torso, MeasureInput::Cm, "Fullest seat."),
    f("nape_to_waist_mm", "Nape to waist", MeasureGroup::Torso, MeasureInput::Cm, "Back waist length."),
    f("front_waist_length_mm", "Front waist length", MeasureGroup::Torso, MeasureInput::Cm, "Side-neck to front waist."),
    f("rise_mm", "Rise / crotch depth", MeasureGroup::Torso, MeasureInput::Cm, "Trouser rise."),
    f("outseam_mm", "Outseam", MeasureGroup::Torso, MeasureInput::Cm, "Waist to ankle, outside."),
    f("shoulder_girth_mm", "Shoulder / overarm girth", MeasureGroup::Torso, MeasureInput::Cm, "Around both shoulders."),
    f("sleeve_mm", "Sleeve length", MeasureGroup::Arms, MeasureInput::Cm, "Shoulder to wrist."),
    f("upper_arm_length_mm", "Upper-arm length", MeasureGroup::Arms, MeasureInput::Cm, "Shoulder to elbow."),
    f("underarm_length_mm", "Underarm length", MeasureGroup::Arms, MeasureInput::Cm, "Armpit to wrist."),
    f("elbow_to_wrist_mm", "Elbow to wrist", MeasureGroup::Arms, MeasureInput::Cm, "Forearm length."),
    f("biceps_mm", "Biceps circumference", MeasureGroup::Arms, MeasureInput::Cm, "Fullest upper arm."),
    f("elbow_girth_mm", "Elbow circumference", MeasureGroup::Arms, MeasureInput::Cm, "Bent elbow."),
    f("forearm_mm", "Forearm circumference", MeasureGroup::Arms, MeasureInput::Cm, "Fullest forearm."),
    f("wrist_mm", "Wrist circumference", MeasureGroup::Arms, MeasureInput::Cm, "Just below the styloid."),
    f("inseam_mm", "Inside leg / inseam", MeasureGroup::Legs, MeasureInput::Cm, "Crotch to floor."),
    f("thigh_mm", "Thigh circumference", MeasureGroup::Legs, MeasureInput::Cm, "Fullest thigh."),
    f("mid_thigh_mm", "Mid-thigh circumference", MeasureGroup::Legs, MeasureInput::Cm, "Halfway hip to knee."),
    f("knee_girth_mm", "Knee circumference", MeasureGroup::Legs, MeasureInput::Cm, "Around the knee."),
    f("calf_mm", "Calf circumference", MeasureGroup::Legs, MeasureInput::Cm, "Fullest calf."),
    f("ankle_mm", "Ankle circumference", MeasureGroup::Legs, MeasureInput::Cm, "Minimum ankle girth."),
    f("head_mm", "Head circumference", MeasureGroup::Head, MeasureInput::Cm, "Helmet tape, above the brow."),
    f("head_length_mm", "Head length", MeasureGroup::Head, MeasureInput::Cm, "Glabella to occiput."),
    f("head_breadth_mm", "Head breadth", MeasureGroup::Head, MeasureInput::Cm, "Maximum cranial breadth."),
    f("head_height_mm", "Head height", MeasureGroup::Head, MeasureInput::Cm, "Vertex to menton."),
    f("face_length_mm", "Face length", MeasureGroup::Head, MeasureInput::Cm, "Nasion to menton."),
    f("bizygomatic_mm", "Face width", MeasureGroup::Head, MeasureInput::Cm, "Bizygomatic breadth."),
    f("sagittal_arc_mm", "Sagittal arc", MeasureGroup::Head, MeasureInput::Cm, "Glabella over vertex to inion."),
    f("bitragion_arc_mm", "Bitragion coronal arc", MeasureGroup::Head, MeasureInput::Cm, "Ear to ear over the crown."),
    f("interpupillary_mm", "Interpupillary distance", MeasureGroup::Head, MeasureInput::Mm, "IPD in millimetres."),
    f("hand_length_mm", "Hand length", MeasureGroup::Hands, MeasureInput::Cm, "Wrist to middle fingertip."),
    f("palm_length_mm", "Palm length", MeasureGroup::Hands, MeasureInput::Cm, "Wrist to palmar crease."),
    f("hand_breadth_mm", "Hand breadth", MeasureGroup::Hands, MeasureInput::Cm, "Across the knuckles."),
    f("hand_circ_mm", "Hand circumference", MeasureGroup::Hands, MeasureInput::Cm, "Knuckles, excluding thumb."),
    f("middle_finger_mm", "Middle-finger length", MeasureGroup::Hands, MeasureInput::Cm, "Crease to tip."),
    f("thumb_circ_mm", "Thumb circumference", MeasureGroup::Hands, MeasureInput::Cm, "Mid-thumb."),
    f("ring_finger_circ_mm", "Ring-finger circumference", MeasureGroup::Hands, MeasureInput::Mm, "Preferred ring finger, mm."),
    f("ring_finger_left_circ_mm", "Left ring-finger circumference", MeasureGroup::Hands, MeasureInput::Mm, "When the hands differ."),
    f("ring_finger_right_circ_mm", "Right ring-finger circumference", MeasureGroup::Hands, MeasureInput::Mm, "When the hands differ."),
    f("foot_mm", "Foot length", MeasureGroup::Feet, MeasureInput::Cm, "Fallback if left/right omitted."),
    f("foot_left_mm", "Left foot length", MeasureGroup::Feet, MeasureInput::Cm, "Prefer left and right."),
    f("foot_right_mm", "Right foot length", MeasureGroup::Feet, MeasureInput::Cm, "Prefer left and right."),
    f("foot_width_mm", "Foot width (ball)", MeasureGroup::Feet, MeasureInput::Cm, "Fallback if left/right omitted."),
    f("foot_width_left_mm", "Left foot width", MeasureGroup::Feet, MeasureInput::Cm, ""),
    f("foot_width_right_mm", "Right foot width", MeasureGroup::Feet, MeasureInput::Cm, ""),
    f("heel_width_mm", "Heel width", MeasureGroup::Feet, MeasureInput::Cm, ""),
    f("ball_girth_mm", "Ball girth", MeasureGroup::Feet, MeasureInput::Cm, "Around the metatarsal heads."),
    f("instep_girth_mm", "Instep girth", MeasureGroup::Feet, MeasureInput::Cm, "Around the arch."),
    f("arch_length_mm", "Arch length", MeasureGroup::Feet, MeasureInput::Cm, "Heel to ball."),
    f("toe_height_mm", "Toe-box height", MeasureGroup::Feet, MeasureInput::Mm, "Vertical clearance, mm."),
];

const fn f(
    id: &'static str,
    label: &'static str,
    group: MeasureGroup,
    input: MeasureInput,
    hint: &'static str,
) -> MeasureField {
    MeasureField {
        id,
        label,
        group,
        input,
        hint,
    }
}

pub fn fields_in(group: MeasureGroup) -> impl Iterator<Item = &'static MeasureField> {
    MEASURE_FIELDS.iter().filter(move |f| f.group == group)
}
