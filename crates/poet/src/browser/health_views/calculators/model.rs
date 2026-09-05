//! Fail-closed clinical calculator drafts. No fabricated patient values.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalculatorKind {
    Framingham,
    Cha2ds2Vasc,
    Score2,
}

impl CalculatorKind {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "framingham" => Some(Self::Framingham),
            "cha2ds2_vasc" | "cha2ds2" => Some(Self::Cha2ds2Vasc),
            "score2" => Some(Self::Score2),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Framingham => "framingham",
            Self::Cha2ds2Vasc => "cha2ds2_vasc",
            Self::Score2 => "score2",
        }
    }

    pub fn capability(self) -> &'static str {
        match self {
            Self::Framingham => "ClinicalRisk.framingham",
            Self::Cha2ds2Vasc => "ClinicalRisk.cha2ds2_vasc",
            Self::Score2 => "ClinicalRisk.score2",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Framingham => "Framingham 10-year CVD risk",
            Self::Cha2ds2Vasc => "CHA₂DS₂-VASc stroke risk",
            Self::Score2 => "SCORE2 10-year CVD risk",
        }
    }

    pub fn version(self) -> &'static str {
        match self {
            Self::Framingham => "wilson-1998-atp3",
            Self::Cha2ds2Vasc => "lip-2010-esc-2020",
            Self::Score2 => "score2-2021",
        }
    }

    pub fn applicability(self) -> &'static str {
        match self {
            Self::Framingham => "Adults 30–74 years. Lipids in mmol/L. Blood pressure in mmHg.",
            Self::Cha2ds2Vasc => {
                "Non-valvular atrial fibrillation in adults 18–120 years. Age bands come from age."
            }
            Self::Score2 => {
                "Adults 40–69 years. European risk region is required and is never defaulted."
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CalculatorDraft {
    pub kind: Option<CalculatorKind>,
    pub age: Option<u8>,
    pub sex_male: Option<bool>,
    pub total_cholesterol_mmol: Option<f64>,
    pub hdl_cholesterol_mmol: Option<f64>,
    pub systolic_bp: Option<f64>,
    pub bp_treated: Option<bool>,
    pub current_smoker: Option<bool>,
    pub diabetic: Option<bool>,
    pub congestive_heart_failure: Option<bool>,
    pub hypertension: Option<bool>,
    pub stroke_tia_history: Option<bool>,
    pub vascular_disease: Option<bool>,
    pub atrial_fibrillation: Option<bool>,
    pub risk_region: Option<String>,
}

impl CalculatorDraft {
    pub fn parse_number(raw: &str) -> Option<f64> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        trimmed
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
    }

    pub fn parse_bool(raw: &str) -> Option<bool> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" | "male" => Some(true),
            "false" | "no" | "0" | "female" => Some(false),
            _ => None,
        }
    }

    pub fn incomplete_reason(&self) -> Option<&'static str> {
        let Some(kind) = self.kind else {
            return Some(
                "Choose an algorithm. No estimate is produced until every required input is entered.",
            );
        };
        if self.age.is_none() {
            return Some("Age in years is required. Incomplete input cannot calculate.");
        }
        match kind {
            CalculatorKind::Framingham => {
                if !(30..=74).contains(&self.age.unwrap_or(0)) {
                    return Some("Framingham applies to ages 30–74 years.");
                }
                if self.sex_male.is_none()
                    || self.total_cholesterol_mmol.is_none()
                    || self.hdl_cholesterol_mmol.is_none()
                    || self.systolic_bp.is_none()
                    || self.bp_treated.is_none()
                    || self.current_smoker.is_none()
                    || self.diabetic.is_none()
                {
                    return Some(
                        "Framingham needs sex, lipids (mmol/L), systolic BP (mmHg), treated BP, smoking, and diabetes — each answered, not assumed.",
                    );
                }
                if self.hdl_cholesterol_mmol >= self.total_cholesterol_mmol {
                    return Some(
                        "HDL cholesterol (mmol/L) must be lower than total cholesterol. Inapplicable input cannot calculate.",
                    );
                }
            }
            CalculatorKind::Cha2ds2Vasc => {
                if !(18..=120).contains(&self.age.unwrap_or(0)) {
                    return Some("CHA₂DS₂-VASc applies to adults 18–120 years.");
                }
                if self.atrial_fibrillation != Some(true) {
                    return Some(
                        "CHA₂DS₂-VASc applies only when atrial fibrillation is present. Inapplicable input cannot calculate.",
                    );
                }
                if self.sex_male.is_none()
                    || self.congestive_heart_failure.is_none()
                    || self.hypertension.is_none()
                    || self.diabetic.is_none()
                    || self.stroke_tia_history.is_none()
                    || self.vascular_disease.is_none()
                {
                    return Some(
                        "CHA₂DS₂-VASc needs sex and each clinical factor as yes or no. Missing is not no.",
                    );
                }
            }
            CalculatorKind::Score2 => {
                if !(40..=69).contains(&self.age.unwrap_or(0)) {
                    return Some("SCORE2 applies to ages 40–69 years.");
                }
                if self.sex_male.is_none()
                    || self.systolic_bp.is_none()
                    || self.total_cholesterol_mmol.is_none()
                    || self.hdl_cholesterol_mmol.is_none()
                    || self.current_smoker.is_none()
                    || self.risk_region.as_deref().unwrap_or("").is_empty()
                {
                    return Some(
                        "SCORE2 needs sex, systolic BP (mmHg), lipids (mmol/L), smoking, and a named European risk region.",
                    );
                }
                if self.hdl_cholesterol_mmol >= self.total_cholesterol_mmol {
                    return Some(
                        "HDL cholesterol (mmol/L) must be lower than total cholesterol. Inapplicable input cannot calculate.",
                    );
                }
            }
        }
        None
    }

    pub fn invoke_args(&self) -> Result<(CalculatorKind, serde_json::Value), &'static str> {
        if let Some(reason) = self.incomplete_reason() {
            return Err(reason);
        }
        let kind = self.kind.expect("completeness already checked");
        let args = match kind {
            CalculatorKind::Framingham => serde_json::json!({
                "age": self.age,
                "sex_male": self.sex_male,
                "total_cholesterol_mmol": self.total_cholesterol_mmol,
                "hdl_cholesterol_mmol": self.hdl_cholesterol_mmol,
                "systolic_bp": self.systolic_bp,
                "bp_treated": self.bp_treated,
                "current_smoker": self.current_smoker,
                "diabetic": self.diabetic
            }),
            CalculatorKind::Cha2ds2Vasc => serde_json::json!({
                "age": self.age,
                "atrial_fibrillation": true,
                "sex_female": self.sex_male == Some(false),
                "congestive_heart_failure": self.congestive_heart_failure,
                "hypertension": self.hypertension,
                "diabetes": self.diabetic,
                "stroke_tia_history": self.stroke_tia_history,
                "vascular_disease": self.vascular_disease
            }),
            CalculatorKind::Score2 => serde_json::json!({
                "age": self.age,
                "sex_male": self.sex_male,
                "systolic_bp": self.systolic_bp,
                "total_cholesterol_mmol": self.total_cholesterol_mmol,
                "hdl_cholesterol_mmol": self.hdl_cholesterol_mmol,
                "current_smoker": self.current_smoker,
                "risk_region": self.risk_region
            }),
        };
        Ok((kind, args))
    }
}

pub const NOT_DIAGNOSIS: &str =
    "This number is not a diagnosis, treatment recommendation, or clinical advice.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_draft_cannot_calculate() {
        let draft = CalculatorDraft::default();
        assert!(draft.incomplete_reason().is_some());
        assert!(draft.invoke_args().is_err());
    }

    #[test]
    fn missing_bool_is_not_false() {
        let draft = CalculatorDraft {
            kind: Some(CalculatorKind::Framingham),
            age: Some(55),
            sex_male: Some(true),
            total_cholesterol_mmol: Some(5.2),
            hdl_cholesterol_mmol: Some(1.3),
            systolic_bp: Some(130.0),
            bp_treated: Some(false),
            current_smoker: Some(false),
            diabetic: None,
            ..CalculatorDraft::default()
        };
        assert!(draft.invoke_args().is_err());
    }

    #[test]
    fn cha2ds2_without_af_cannot_calculate() {
        let draft = CalculatorDraft {
            kind: Some(CalculatorKind::Cha2ds2Vasc),
            age: Some(80),
            sex_male: Some(false),
            atrial_fibrillation: Some(false),
            congestive_heart_failure: Some(true),
            hypertension: Some(true),
            diabetic: Some(true),
            stroke_tia_history: Some(true),
            vascular_disease: Some(true),
            ..CalculatorDraft::default()
        };
        assert!(draft.invoke_args().is_err());
    }

    #[test]
    fn score2_without_region_cannot_calculate() {
        let draft = CalculatorDraft {
            kind: Some(CalculatorKind::Score2),
            age: Some(55),
            sex_male: Some(true),
            systolic_bp: Some(140.0),
            total_cholesterol_mmol: Some(5.5),
            hdl_cholesterol_mmol: Some(1.1),
            current_smoker: Some(false),
            risk_region: None,
            ..CalculatorDraft::default()
        };
        assert!(draft.invoke_args().is_err());
    }

    #[test]
    fn hdl_not_below_total_cannot_calculate() {
        let draft = CalculatorDraft {
            kind: Some(CalculatorKind::Framingham),
            age: Some(55),
            sex_male: Some(true),
            total_cholesterol_mmol: Some(1.0),
            hdl_cholesterol_mmol: Some(1.5),
            systolic_bp: Some(120.0),
            bp_treated: Some(false),
            current_smoker: Some(false),
            diabetic: Some(false),
            ..CalculatorDraft::default()
        };
        assert!(draft.invoke_args().is_err());
    }

    #[test]
    fn complete_framingham_names_live_capability() {
        let draft = CalculatorDraft {
            kind: Some(CalculatorKind::Framingham),
            age: Some(60),
            sex_male: Some(true),
            total_cholesterol_mmol: Some(6.5),
            hdl_cholesterol_mmol: Some(0.9),
            systolic_bp: Some(162.0),
            bp_treated: Some(false),
            current_smoker: Some(true),
            diabetic: Some(true),
            ..CalculatorDraft::default()
        };
        let (kind, args) = draft.invoke_args().expect("complete");
        assert_eq!(kind.capability(), "ClinicalRisk.framingham");
        assert_eq!(args["age"], 60);
        assert_eq!(args["diabetic"], true);
    }
}
