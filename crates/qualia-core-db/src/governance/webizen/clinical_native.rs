//! `SlgOpcode::NativeClinicalRisk` — real engines, fail closed on missing inputs.
//!
//! A `VmFrame` only carries four registers. Framingham, CHA₂DS₂-VASc, and SCORE2
//! need a complete patient record. Those facts live as Quins on the patient node
//! (`frame.subject_reg`). Missing predicates are held; they are never invented.

use crate::clinical_engine::{
    cha2ds2_vasc_score, framingham_10yr_risk, score2_risk, Cha2ds2VascInput, FraminghamInput,
    Score2Input, Score2Region,
};
use crate::frame_layout::{
    object_tag, unpack_float_object, INLINE_TAG_BOOLEAN, INLINE_TAG_DECIMAL, INLINE_TAG_FLOAT,
    INLINE_TAG_INTEGER, INLINE_VALUE_MASK,
};
use crate::q_hash;
use crate::NQuin;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NativeClinicalRiskOutcome {
    HeldIncomplete,
    UnknownModel,
    /// Engine ran. `value` is the model-native score (Framingham fraction,
    /// CHA₂DS₂-VASc points, or SCORE2 percent).
    Calculated { value: f64 },
}

/// `model_id`: 0 Framingham, 1 CHA₂DS₂-VASc, 2 SCORE2.
pub fn evaluate(model_id: u8) -> NativeClinicalRiskOutcome {
    evaluate_patient(model_id, 0, &[])
}

/// Evaluate against Quins already in the SLG arena for `patient`.
pub fn evaluate_patient(
    model_id: u8,
    patient: u64,
    quins: &[NQuin],
) -> NativeClinicalRiskOutcome {
    match model_id {
        0 => match framingham_from_quins(patient, quins) {
            Some(input) => NativeClinicalRiskOutcome::Calculated {
                value: framingham_10yr_risk(&input).risk_10yr,
            },
            None => NativeClinicalRiskOutcome::HeldIncomplete,
        },
        1 => match cha2ds2_from_quins(patient, quins) {
            Some(input) => NativeClinicalRiskOutcome::Calculated {
                value: f64::from(cha2ds2_vasc_score(&input).score),
            },
            None => NativeClinicalRiskOutcome::HeldIncomplete,
        },
        2 => match score2_from_quins(patient, quins) {
            Some(input) => NativeClinicalRiskOutcome::Calculated {
                value: score2_risk(&input).risk_10yr_pct,
            },
            None => NativeClinicalRiskOutcome::HeldIncomplete,
        },
        _ => NativeClinicalRiskOutcome::UnknownModel,
    }
}

fn pred(name: &str) -> u64 {
    q_hash(name)
}

fn find_object(patient: u64, predicate: u64, quins: &[NQuin]) -> Option<u64> {
    if patient == 0 {
        return None;
    }
    quins
        .iter()
        .find(|q| q.subject == patient && q.predicate == predicate)
        .map(|q| q.object)
}

fn as_bool(object: u64) -> Option<bool> {
    if object_tag(object) != INLINE_TAG_BOOLEAN {
        return None;
    }
    Some((object & 1) != 0)
}

fn as_u8(object: u64) -> Option<u8> {
    if object_tag(object) != INLINE_TAG_INTEGER {
        return None;
    }
    u8::try_from(object & INLINE_VALUE_MASK).ok()
}

fn as_f64(object: u64) -> Option<f64> {
    let tag = object_tag(object);
    if tag == INLINE_TAG_FLOAT {
        let v = unpack_float_object(object);
        return v.is_finite().then_some(f64::from(v));
    }
    if tag == INLINE_TAG_DECIMAL {
        let raw = (object & INLINE_VALUE_MASK) as i64;
        // Sign-extend the 60-bit payload.
        let shifted = (raw << 4) >> 4;
        return Some((shifted as f64) / 1_000_000.0);
    }
    if tag == INLINE_TAG_INTEGER {
        return Some((object & INLINE_VALUE_MASK) as f64);
    }
    None
}

fn need_bool(patient: u64, name: &str, quins: &[NQuin]) -> Option<bool> {
    as_bool(find_object(patient, pred(name), quins)?)
}

fn need_u8(patient: u64, name: &str, quins: &[NQuin]) -> Option<u8> {
    as_u8(find_object(patient, pred(name), quins)?)
}

fn need_f64(patient: u64, name: &str, quins: &[NQuin]) -> Option<f64> {
    let v = as_f64(find_object(patient, pred(name), quins)?)?;
    v.is_finite().then_some(v)
}

fn framingham_from_quins(patient: u64, quins: &[NQuin]) -> Option<FraminghamInput> {
    let age = need_u8(patient, "q42:age", quins)?;
    if !(30..=74).contains(&age) {
        return None;
    }
    let total = need_f64(patient, "q42:totalCholesterolMmol", quins)?;
    let hdl = need_f64(patient, "q42:hdlCholesterolMmol", quins)?;
    if !(total > 0.0 && hdl > 0.0 && hdl < total && total <= 20.0 && hdl <= 5.0) {
        return None;
    }
    let sbp = need_f64(patient, "q42:systolicBp", quins)?;
    if !(sbp > 0.0 && sbp <= 260.0) {
        return None;
    }
    Some(FraminghamInput {
        age,
        sex_male: need_bool(patient, "q42:sexMale", quins)?,
        total_cholesterol_mmol: total,
        hdl_cholesterol_mmol: hdl,
        systolic_bp: sbp,
        bp_treated: need_bool(patient, "q42:bpTreated", quins)?,
        current_smoker: need_bool(patient, "q42:currentSmoker", quins)?,
        diabetic: need_bool(patient, "q42:diabetic", quins)?,
    })
}

fn cha2ds2_from_quins(patient: u64, quins: &[NQuin]) -> Option<Cha2ds2VascInput> {
    if need_bool(patient, "q42:atrialFibrillation", quins) != Some(true) {
        return None;
    }
    let age = need_u8(patient, "q42:age", quins)?;
    if !(18..=120).contains(&age) {
        return None;
    }
    Some(Cha2ds2VascInput {
        congestive_heart_failure: need_bool(patient, "q42:congestiveHeartFailure", quins)?,
        hypertension: need_bool(patient, "q42:hypertension", quins)?,
        age_75_or_older: age >= 75,
        diabetes: need_bool(patient, "q42:diabetes", quins)?,
        stroke_tia_history: need_bool(patient, "q42:strokeTiaHistory", quins)?,
        vascular_disease: need_bool(patient, "q42:vascularDisease", quins)?,
        age_65_to_74: (65..75).contains(&age),
        sex_female: need_bool(patient, "q42:sexFemale", quins)?,
    })
}

fn score2_from_quins(patient: u64, quins: &[NQuin]) -> Option<Score2Input> {
    let age = need_u8(patient, "q42:age", quins)?;
    if !(40..=69).contains(&age) {
        return None;
    }
    let region = match need_u8(patient, "q42:score2Region", quins)? {
        0 => Score2Region::Low,
        1 => Score2Region::Moderate,
        2 => Score2Region::High,
        3 => Score2Region::VeryHigh,
        _ => return None,
    };
    let total = need_f64(patient, "q42:totalCholesterolMmol", quins)?;
    let hdl = need_f64(patient, "q42:hdlCholesterolMmol", quins)?;
    if !(total > 0.0 && hdl > 0.0 && hdl < total && total <= 20.0 && hdl <= 5.0) {
        return None;
    }
    let sbp = need_f64(patient, "q42:systolicBp", quins)?;
    if !(sbp > 0.0 && sbp <= 260.0) {
        return None;
    }
    Some(Score2Input {
        age,
        sex_male: need_bool(patient, "q42:sexMale", quins)?,
        systolic_bp: sbp,
        total_cholesterol_mmol: total,
        hdl_cholesterol_mmol: hdl,
        current_smoker: need_bool(patient, "q42:currentSmoker", quins)?,
        risk_region: region,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame_layout::{pack_float_object, INLINE_TAG_BOOLEAN, INLINE_TAG_INTEGER};

    fn fact(patient: u64, pred: &str, object: u64) -> NQuin {
        NQuin {
            subject: patient,
            predicate: q_hash(pred),
            object,
            context: 0,
            metadata: 0,
            parity: 0,
        }
    }

    fn int_obj(v: u64) -> u64 {
        INLINE_TAG_INTEGER | (v & INLINE_VALUE_MASK)
    }

    fn bool_obj(v: bool) -> u64 {
        INLINE_TAG_BOOLEAN | u64::from(v)
    }

    fn float_obj(v: f32) -> u64 {
        pack_float_object(v)
    }

    fn framingham_quins(patient: u64) -> Vec<NQuin> {
        vec![
            fact(patient, "q42:age", int_obj(60)),
            fact(patient, "q42:sexMale", bool_obj(true)),
            fact(patient, "q42:totalCholesterolMmol", float_obj(6.5)),
            fact(patient, "q42:hdlCholesterolMmol", float_obj(0.9)),
            fact(patient, "q42:systolicBp", float_obj(162.0)),
            fact(patient, "q42:bpTreated", bool_obj(false)),
            fact(patient, "q42:currentSmoker", bool_obj(true)),
            fact(patient, "q42:diabetic", bool_obj(true)),
        ]
    }

    #[test]
    fn known_models_do_not_calculate_from_registers() {
        for model in [0_u8, 1, 2] {
            assert_eq!(evaluate(model), NativeClinicalRiskOutcome::HeldIncomplete);
        }
    }

    #[test]
    fn unknown_model_is_not_a_default_calculator() {
        assert_eq!(evaluate(99), NativeClinicalRiskOutcome::UnknownModel);
    }

    #[test]
    fn incomplete_framingham_patient_is_held() {
        let patient = 0xA11CE;
        let quins = [fact(patient, "q42:age", int_obj(60))];
        assert_eq!(
            evaluate_patient(0, patient, &quins),
            NativeClinicalRiskOutcome::HeldIncomplete
        );
    }

    #[test]
    fn complete_framingham_runs_the_engine() {
        let patient = 0xA11CE;
        let quins = framingham_quins(patient);
        let expected = framingham_10yr_risk(&FraminghamInput {
            age: 60,
            sex_male: true,
            total_cholesterol_mmol: 6.5,
            hdl_cholesterol_mmol: 0.9,
            systolic_bp: 162.0,
            bp_treated: false,
            current_smoker: true,
            diabetic: true,
        });
        match evaluate_patient(0, patient, &quins) {
            NativeClinicalRiskOutcome::Calculated { value } => {
                assert!((value - expected.risk_10yr).abs() < 1e-5);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn cha2ds2_without_af_is_held() {
        let patient = 0xAF00;
        let quins = [
            fact(patient, "q42:age", int_obj(80)),
            fact(patient, "q42:atrialFibrillation", bool_obj(false)),
            fact(patient, "q42:congestiveHeartFailure", bool_obj(true)),
            fact(patient, "q42:hypertension", bool_obj(true)),
            fact(patient, "q42:diabetes", bool_obj(true)),
            fact(patient, "q42:strokeTiaHistory", bool_obj(true)),
            fact(patient, "q42:vascularDisease", bool_obj(true)),
            fact(patient, "q42:sexFemale", bool_obj(false)),
        ];
        assert_eq!(
            evaluate_patient(1, patient, &quins),
            NativeClinicalRiskOutcome::HeldIncomplete
        );
    }
}
