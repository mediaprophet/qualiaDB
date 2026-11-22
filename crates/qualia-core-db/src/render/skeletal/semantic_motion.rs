//! Resolve one semantic rotation against an admitted articulation envelope.

use super::Motor;
use crate::{q42_volume::WasmSliceQ42Reader, q_hash};

mod blend;
mod dca;
mod femc;
mod graph;
mod hydraulic;
mod instance;
mod morph_targets;
mod rule;
mod vehicle;
pub use instance::compose_leaf_delta;
pub use blend::PreparedClipBlend;
pub use femc::{prepare_femc_asset_motion, FemcFrame, FemcState, PreparedFemcMotion, MAX_FEMC_STEP_S};
pub use morph_targets::{PreparedMorphTargets, MORPH_TARGET_GROUP_IRI, MORPH_TARGET_SLOT_IRI};
pub use dca::{DcaChainState, DcaChainStep, PreparedDcaChain, prepare_dca_motor_chain};
pub use hydraulic::{HydraulicEdgeStep, HydraulicStep, PreparedHydraulicMotion, prepare_hydraulic_motion, MAX_HYDRAULIC_EDGES};
pub use vehicle::{MarineVehicleControl, MarineVehicleState, MarineVehicleStep, PreparedMarineVehicle, prepare_marine_vehicle};

pub(crate) fn rest_poses_agree(expected: Motor, authored: Motor) -> bool {
    if !super::valid_motor(&expected) || !super::valid_motor(&authored) { return false; }
    let (expected_rotation, expected_translation) = expected.to_rotation_translation();
    let (rotation, translation) = authored.to_rotation_translation();
    let alignment = expected_rotation.iter().zip(rotation).map(|(a, b)| a * b).sum::<f64>().abs();
    (1.0 - alignment).abs() <= 1e-6 && expected_translation.iter().zip(translation).all(|(a, b)| (a - b).abs() <= 1e-6)
}
pub use graph::{
    prepare_aeroelastic_rotation, prepare_feedback_rotation, prepare_feedback_rotation_for_resource,
    prepare_q42_rule_macro, prepare_q42_semantic_macro, sample_prepared_macro,
    stage_feedback_rotation, stage_semantic_macro, ControllerFeedback, FeedbackRotation,
    MacroChannel, MacroKeyframe, MacroValue, MotionGraph, PreparedAeroelasticRotation,
    PreparedFeedbackRotation, SemanticAction,
};

/// Fitted local axis, pivot, and continuous allowed interval from an articulation receipt.
#[derive(Clone, Copy, Debug)]
pub struct FittedRotation {
    pub semantic_id: u64,
    pub local_axis: [f64; 3],
    pub local_pivot_m: [f64; 3],
    pub rom_radians: [f64; 2],
    pub contact_safe_radians: [f64; 2],
}

#[derive(Clone, Copy)]
pub struct SemanticRotation {
    pub semantic_id: u64,
    pub angle_radians: f64,
    pub motor: Motor,
}

#[derive(Debug)]
pub enum SemanticMotionError {
    Model(std::io::Error),
    UnknownIdentity,
    MissingFit,
    AmbiguousFit,
    InvalidFit,
    InvalidCommand,
    ConflictingFact,
    MissingFact,
    InvalidFact,
    InvalidCapability,
    BudgetExceeded,
    OutputTooSmall,
    InvalidMaterial,
    UnboundTarget,
    AmbiguousTarget,
    UnreachableTarget,
    FemcRestResidual { metres: f64 },
    FemcEnthesisResidual { metres: f64 },
    FemcVolumeResidual { tet: usize, determinant: f64 },
    FemcSurface(String),
}

/// Stage one rotation after a Q42LEX round-trip and clamp to both the ROM and
/// fitted contact-safe interval. Scratch buffers belong to the caller.
pub fn stage_semantic_rotation(
    model: &WasmSliceQ42Reader<'_>,
    fits: &[FittedRotation],
    iri: &str,
    radians: f64,
    page_scratch: &mut [u8],
    lexical_scratch: &mut [u8],
) -> Result<SemanticRotation, SemanticMotionError> {
    if !radians.is_finite() {
        return Err(SemanticMotionError::InvalidCommand);
    }
    let semantic_id = q_hash(iri);
    if semantic_id == 0 || !model.is_unified_volume() {
        return Err(SemanticMotionError::UnknownIdentity);
    }
    let length = model.lookup_lexicon_hash_into(semantic_id, page_scratch, lexical_scratch)
        .map_err(SemanticMotionError::Model)?
        .ok_or(SemanticMotionError::UnknownIdentity)?;
    if lexical_scratch.get(..length) != Some(iri.as_bytes()) {
        return Err(SemanticMotionError::UnknownIdentity);
    }
    let mut matching = fits.iter().filter(|fit| fit.semantic_id == semantic_id);
    let fit = matching.next().ok_or(SemanticMotionError::MissingFit)?;
    if matching.next().is_some() {
        return Err(SemanticMotionError::AmbiguousFit);
    }
    fitted_rotation(fit, radians)
}

fn fitted_rotation(fit: &FittedRotation, radians: f64) -> Result<SemanticRotation, SemanticMotionError> {
    let [rom_min, rom_max] = fit.rom_radians;
    let [contact_min, contact_max] = fit.contact_safe_radians;
    if ![rom_min, rom_max, contact_min, contact_max].iter().all(|v| v.is_finite())
        || !fit.local_pivot_m.iter().all(|v| v.is_finite())
    {
        return Err(SemanticMotionError::InvalidFit);
    }
    let min = rom_min.max(contact_min);
    let max = rom_max.min(contact_max);
    let norm_squared = fit.local_axis.iter().map(|v| v * v).sum::<f64>();
    if min > max || !norm_squared.is_finite() || norm_squared <= f64::MIN_POSITIVE {
        return Err(SemanticMotionError::InvalidFit);
    }
    let norm = norm_squared.sqrt();
    let [x, y, z] = fit.local_axis.map(|v| v / norm);
    let [px, py, pz] = fit.local_pivot_m;
    let angle = radians.clamp(min, max);
    let (sin, cos) = angle.sin_cos();
    let dot = x * px + y * py + z * pz;
    let cross = [y * pz - z * py, z * px - x * pz, x * py - y * px];
    let rotated = std::array::from_fn::<_, 3, _>(|lane| {
        fit.local_pivot_m[lane] * cos + cross[lane] * sin + [x, y, z][lane] * dot * (1.0 - cos)
    });
    let translation = std::array::from_fn(|lane| fit.local_pivot_m[lane] - rotated[lane]);
    if !translation.iter().all(|v| v.is_finite()) {
        return Err(SemanticMotionError::InvalidFit);
    }
    let (s, c) = (angle * 0.5).sin_cos();
    let motor = Motor::from_rotation_translation([c, x * s, y * s, z * s], translation);
    Ok(SemanticRotation {
        semantic_id: fit.semantic_id,
        angle_radians: angle,
        motor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersects_the_rom_and_contact_intervals() {
        let mut fit = FittedRotation {
            semantic_id: q_hash("urn:test:fitted:joint"),
            local_axis: [0.0, 3.0, 0.0],
            local_pivot_m: [0.0; 3],
            rom_radians: [-0.2, 0.3],
            contact_safe_radians: [-0.1, 0.4],
        };
        let positive = fitted_rotation(&fit, 1.0).unwrap();
        assert_eq!(positive.angle_radians, 0.3);
        assert_eq!(positive.semantic_id, fit.semantic_id);
        assert_eq!(fitted_rotation(&fit, -1.0).unwrap().angle_radians, -0.1);
        fit.contact_safe_radians = [0.5, 0.7];
        assert!(matches!(fitted_rotation(&fit, 0.0), Err(SemanticMotionError::InvalidFit)));
    }
}
