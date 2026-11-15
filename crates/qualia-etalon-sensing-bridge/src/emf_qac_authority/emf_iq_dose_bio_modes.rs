//! EMF authority-gated observation bridges for Mode `0x0106` (`ionizing_radiation_v1`)
//! and Mode `0x0107` (`bioelectric_dipole_v1`), along with component-targeted uncertainty
//! descriptors and epistemic class verification.

use super::*;

/// Uncertainty semantics for component-targeted EMF records (C02-C05).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BridgeUncertaintyKind {
    StandardAbsolute,
    ExpandedAbsolute {
        coverage_factor_k: f64,
        confidence_probability: f64,
    },
    Relative,
}

/// Explicit component-targeted uncertainty descriptor (C02-C05), rejecting unlabeled scalar
/// uncertainty on mixed-unit records (Modes `0x0106` and `0x0107`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComponentTargetedUncertainty {
    pub target_quantity: SemanticId,
    pub target_unit: SemanticId,
    pub kind: BridgeUncertaintyKind,
    pub value: f64,
}

impl ComponentTargetedUncertainty {
    pub fn validate(&self) -> Result<(), EmfAuthorityBridgeError> {
        if self.target_quantity.value().is_none() || self.target_unit.value().is_none() {
            return Err(EmfAuthorityBridgeError::MissingUncertaintyTargetQuantityOrUnit);
        }
        if !self.value.is_finite() || self.value < 0.0 {
            return Err(EmfAuthorityBridgeError::InvalidUncertaintyValue);
        }
        if let BridgeUncertaintyKind::ExpandedAbsolute {
            coverage_factor_k,
            confidence_probability,
        } = self.kind
        {
            if !coverage_factor_k.is_finite()
                || coverage_factor_k <= 0.0
                || !confidence_probability.is_finite()
                || confidence_probability <= 0.0
                || confidence_probability >= 1.0
            {
                return Err(EmfAuthorityBridgeError::InvalidUncertaintyValue);
            }
        }
        Ok(())
    }
}

/// Epistemic class guard for empirical EMF observation bridges (C02-C07).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeEpistemicClass {
    Observation,
    Interpolated,
    Predicted,
    Hypothesis,
    Counterfactual,
    Fictional,
}

impl BridgeEpistemicClass {
    pub fn verify_empirical_admissible(self) -> Result<(), EmfAuthorityBridgeError> {
        match self {
            Self::Observation | Self::Interpolated | Self::Predicted => Ok(()),
            Self::Hypothesis | Self::Counterfactual | Self::Fictional => {
                Err(EmfAuthorityBridgeError::NonEmpiricalEpistemicClassRejected)
            }
        }
    }
}

/// Component tags for mixed-unit ionizing radiation records (Mode `0x0106`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IonizingComponentTag {
    PhotonEnergyKev,
    MassAttenuationCm2PerG,
    AbsorbedDoseGy,
    EquivalentDoseSv,
}

/// Separation between monoenergetic photon attenuation and measured absorbed/equivalent dose (Mode `0x0106`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IonizingObservationKind {
    MonoenergeticPhotonAttenuation {
        photon_energy_kev: f64,
        mass_attenuation_cm2_per_g: f64,
        energy_uncertainty: ComponentTargetedUncertainty,
        attenuation_uncertainty: ComponentTargetedUncertainty,
    },
    MeasuredDose {
        absorbed_dose_gy: f64,
        absorbed_dose_uncertainty: ComponentTargetedUncertainty,
        equivalent_dose_sv: Option<f64>,
        equivalent_dose_uncertainty: Option<ComponentTargetedUncertainty>,
        weighting_model_id: Option<SemanticId>,
        infer_dose_from_attenuation_attempted: bool,
    },
}

/// Input contract for EMF Mode `0x0106` (`ionizing_radiation_v1`).
#[derive(Clone, Copy, Debug)]
pub struct EmfMode0106IonizingInput {
    pub record_id: u64,
    pub qualia_source_id: u64,
    pub observation: ObservationEnvelope,
    pub binding: ResolvedFieldPayloadBinding,
    pub payload_digest: [u8; 32],
    pub calibration_artifact_id: [u8; 32],
    pub calibration_window: BridgeCalibrationWindow,
    pub epistemic_class: BridgeEpistemicClass,
    pub unlabeled_scalar_uncertainty_present: bool,
    pub kind: IonizingObservationKind,
    pub receipt: SignedEmfBridgeReceipt,
    pub expected_receipt_digest: [u8; 32],
}

/// Verified and bridged EMF Mode `0x0106` ionizing radiation observation.
pub struct BridgedEmfMode0106Ionizing {
    pub bridged: BridgedObservation,
    pub observation_instant: BridgeRationalInstant,
    pub kind: IonizingObservationKind,
    pub authority_receipt_digest: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub fn bridge_emf_mode0106_ionizing_with_consent<A: ConsentBridgeAuthority, const CAP: usize>(
    authority: &A,
    permit: SensingPermit,
    grant: &ConsentGrant,
    identity: &impl IdentityAuthority,
    sensor_scope: &impl SensingScopeAuthority,
    revocations: &impl RevocationAuthority,
    clock: &mut impl TrustedUnixClock,
    replay_window: &mut EmfBridgeReceiptReplayWindow<CAP>,
    input: EmfMode0106IonizingInput,
) -> Result<BridgedEmfMode0106Ionizing, EmfAuthorityBridgeError> {
    input.epistemic_class.verify_empirical_admissible()?;
    if input.unlabeled_scalar_uncertainty_present {
        return Err(EmfAuthorityBridgeError::UnlabeledMixedUnitUncertaintyRejected);
    }
    match input.kind {
        IonizingObservationKind::MonoenergeticPhotonAttenuation {
            photon_energy_kev,
            mass_attenuation_cm2_per_g,
            energy_uncertainty,
            attenuation_uncertainty,
        } => {
            if !photon_energy_kev.is_finite()
                || photon_energy_kev <= 0.0
                || !mass_attenuation_cm2_per_g.is_finite()
                || mass_attenuation_cm2_per_g <= 0.0
            {
                return Err(EmfAuthorityBridgeError::InvalidIonizingQuantity);
            }
            energy_uncertainty.validate()?;
            attenuation_uncertainty.validate()?;
            if energy_uncertainty.target_unit == attenuation_uncertainty.target_unit {
                return Err(EmfAuthorityBridgeError::MixedUnitComponentsShareIdenticalUnit);
            }
        }
        IonizingObservationKind::MeasuredDose {
            absorbed_dose_gy,
            absorbed_dose_uncertainty,
            equivalent_dose_sv,
            equivalent_dose_uncertainty,
            weighting_model_id,
            infer_dose_from_attenuation_attempted,
        } => {
            if infer_dose_from_attenuation_attempted {
                return Err(EmfAuthorityBridgeError::DoseInferredFromAttenuationRejected);
            }
            if !absorbed_dose_gy.is_finite() || absorbed_dose_gy < 0.0 {
                return Err(EmfAuthorityBridgeError::InvalidIonizingQuantity);
            }
            absorbed_dose_uncertainty.validate()?;
            if let Some(sv) = equivalent_dose_sv {
                if !sv.is_finite() || sv < 0.0 {
                    return Err(EmfAuthorityBridgeError::InvalidIonizingQuantity);
                }
                let model = weighting_model_id
                    .ok_or(EmfAuthorityBridgeError::MissingDoseWeightingModel)?;
                if model.value().is_none() {
                    return Err(EmfAuthorityBridgeError::MissingDoseWeightingModel);
                }
                let sv_unc = equivalent_dose_uncertainty
                    .ok_or(EmfAuthorityBridgeError::MissingUncertaintyTargetQuantityOrUnit)?;
                sv_unc.validate()?;
            }
        }
    }

    let now_unix = clock
        .now_unix()
        .map_err(|_| EmfAuthorityBridgeError::ClockUnavailable)?;
    input.receipt.verify_against(
        EMF_MODE_0106_IONIZING_RADIATION,
        input.expected_receipt_digest,
        &permit,
        input.payload_digest,
        input.calibration_artifact_id,
        now_unix,
    )?;

    let observation_instant = input
        .calibration_window
        .verify_observation_calibration(&input.observation, &input.calibration_artifact_id)
        .map_err(EmfAuthorityBridgeError::CalibrationClock)?;

    let bridged = bridge_observation_with_consent(
        authority,
        permit,
        grant,
        identity,
        sensor_scope,
        revocations,
        clock,
        input.record_id,
        input.qualia_source_id,
        input.observation,
        input.binding,
    )
    .map_err(EmfAuthorityBridgeError::Bridge)?;

    replay_window.admit_once(input.expected_receipt_digest)?;

    Ok(BridgedEmfMode0106Ionizing {
        bridged,
        observation_instant,
        kind: input.kind,
        authority_receipt_digest: input.expected_receipt_digest,
    })
}

/// Input contract for EMF Mode `0x0107` (`bioelectric_dipole_v1`, Schema 137).
#[derive(Clone, Copy, Debug)]
pub struct EmfMode0107BioDipoleInput {
    pub record_id: u64,
    pub qualia_source_id: u64,
    pub observation: ObservationEnvelope,
    pub binding: ResolvedFieldPayloadBinding,
    pub payload_digest: [u8; 32],
    pub calibration_artifact_id: [u8; 32],
    pub calibration_window: BridgeCalibrationWindow,
    pub anatomy_ontology_id: SemanticId,
    pub anatomy_identifier: SemanticId,
    pub consent_receipt_digest: [u8; 32],
    pub source_dipole_position_m: [f64; 3],
    pub observation_electrode_position_m: Option<[f64; 3]>,
    pub reference_potential_id: Option<SemanticId>,
    pub dipole_moment_amp_m: [f64; 3],
    pub extracellular_potential_v: Option<f64>,
    pub dipole_moment_uncertainty: ComponentTargetedUncertainty,
    pub potential_uncertainty: Option<ComponentTargetedUncertainty>,
    pub receipt: SignedEmfBridgeReceipt,
    pub expected_receipt_digest: [u8; 32],
}

/// Verified and bridged EMF Mode `0x0107` bioelectric dipole observation.
pub struct BridgedEmfMode0107BioDipole {
    pub bridged: BridgedObservation,
    pub observation_instant: BridgeRationalInstant,
    pub anatomy_ontology_id: SemanticId,
    pub anatomy_identifier: SemanticId,
    pub consent_receipt_digest: [u8; 32],
    pub source_to_electrode_distance_m: Option<f64>,
    pub dipole_moment_amp_m: [f64; 3],
    pub extracellular_potential_v: Option<f64>,
}

#[allow(clippy::too_many_arguments)]
pub fn bridge_emf_mode0107_bio_dipole_with_consent<A: ConsentBridgeAuthority, const CAP: usize>(
    authority: &A,
    permit: SensingPermit,
    grant: &ConsentGrant,
    identity: &impl IdentityAuthority,
    sensor_scope: &impl SensingScopeAuthority,
    revocations: &impl RevocationAuthority,
    clock: &mut impl TrustedUnixClock,
    replay_window: &mut EmfBridgeReceiptReplayWindow<CAP>,
    input: EmfMode0107BioDipoleInput,
) -> Result<BridgedEmfMode0107BioDipole, EmfAuthorityBridgeError> {
    if input.anatomy_ontology_id.value().is_none() || input.anatomy_identifier.value().is_none() {
        return Err(EmfAuthorityBridgeError::MissingAnatomyOntologyBinding);
    }
    if input.consent_receipt_digest == [0u8; 32]
        || input.consent_receipt_digest != input.expected_receipt_digest
    {
        return Err(EmfAuthorityBridgeError::ReceiptDigestMismatch);
    }
    if !all_finite_3(input.source_dipole_position_m) || !all_finite_3(input.dipole_moment_amp_m) {
        return Err(EmfAuthorityBridgeError::InvalidBioDipoleGeometry);
    }
    input.dipole_moment_uncertainty.validate()?;

    let source_to_electrode_distance_m = match input.observation_electrode_position_m {
        Some(obs_pos) => {
            if !all_finite_3(obs_pos) {
                return Err(EmfAuthorityBridgeError::InvalidBioDipoleGeometry);
            }
            let dx = obs_pos[0] - input.source_dipole_position_m[0];
            let dy = obs_pos[1] - input.source_dipole_position_m[1];
            let dz = obs_pos[2] - input.source_dipole_position_m[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if !dist_sq.is_finite() || dist_sq < 1e-18 {
                return Err(EmfAuthorityBridgeError::SingularDipoleSourceEvaluationRejected);
            }
            Some(dist_sq.sqrt())
        }
        None => None,
    };

    if let Some(phi_v) = input.extracellular_potential_v {
        if !phi_v.is_finite() {
            return Err(EmfAuthorityBridgeError::InvalidBioDipoleGeometry);
        }
        if source_to_electrode_distance_m.is_none() {
            return Err(EmfAuthorityBridgeError::MissingElectrodeObservationPosition);
        }
        let ref_id = input
            .reference_potential_id
            .ok_or(EmfAuthorityBridgeError::MissingReferencePotential)?;
        if ref_id.value().is_none() {
            return Err(EmfAuthorityBridgeError::MissingReferencePotential);
        }
        let pot_unc = input
            .potential_uncertainty
            .ok_or(EmfAuthorityBridgeError::MissingUncertaintyTargetQuantityOrUnit)?;
        pot_unc.validate()?;
    }

    let now_unix = clock
        .now_unix()
        .map_err(|_| EmfAuthorityBridgeError::ClockUnavailable)?;
    input.receipt.verify_against(
        EMF_MODE_0107_BIOELECTRIC_DIPOLE,
        input.expected_receipt_digest,
        &permit,
        input.payload_digest,
        input.calibration_artifact_id,
        now_unix,
    )?;

    let observation_instant = input
        .calibration_window
        .verify_observation_calibration(&input.observation, &input.calibration_artifact_id)
        .map_err(EmfAuthorityBridgeError::CalibrationClock)?;

    let bridged = bridge_observation_with_consent(
        authority,
        permit,
        grant,
        identity,
        sensor_scope,
        revocations,
        clock,
        input.record_id,
        input.qualia_source_id,
        input.observation,
        input.binding,
    )
    .map_err(EmfAuthorityBridgeError::Bridge)?;

    replay_window.admit_once(input.expected_receipt_digest)?;

    Ok(BridgedEmfMode0107BioDipole {
        bridged,
        observation_instant,
        anatomy_ontology_id: input.anatomy_ontology_id,
        anatomy_identifier: input.anatomy_identifier,
        consent_receipt_digest: input.consent_receipt_digest,
        source_to_electrode_distance_m,
        dipole_moment_amp_m: input.dipole_moment_amp_m,
        extracellular_potential_v: input.extracellular_potential_v,
    })
}

fn all_finite_3(v: [f64; 3]) -> bool {
    v[0].is_finite() && v[1].is_finite() && v[2].is_finite()
}
