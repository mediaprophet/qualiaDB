//! `SlgOpcode::NativeClinicalRisk` — fail closed.
//!
//! A `VmFrame` only carries four registers. That is not a complete Framingham,
//! CHA₂DS₂-VASc, or SCORE2 input. Inventing lipids, blood pressure, region, or
//! omitted clinical booleans would fabricate a patient. Incomplete frames are
//! held; they never calculate.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeClinicalRiskOutcome {
    HeldIncomplete,
    UnknownModel,
}

/// `model_id`: 0 Framingham, 1 CHA₂DS₂-VASc, 2 SCORE2.
pub fn evaluate(model_id: u8) -> NativeClinicalRiskOutcome {
    match model_id {
        0 | 1 | 2 => NativeClinicalRiskOutcome::HeldIncomplete,
        _ => NativeClinicalRiskOutcome::UnknownModel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
