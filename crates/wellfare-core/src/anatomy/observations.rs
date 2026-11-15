//! Extensible **body observations** — measurements and attributes from the person
//! or from instruments (scale, tape, watch, clinical device, camera).
//!
//! New instruments add a **code** (LOINC / house IRI), not a new struct field.
//! Drawing on the anatomy model requires a curated [`RepresentationBind`]. An
//! unknown code is stored and listed; it is not painted.

use qualia_cooperative_core::contextual_assertion::{
    AssertionStatus, ContextualAssertion, StatementType, VerificationStatus,
};
use qualia_cooperative_core::contextual_evidence::{
    BiologicalScope, ClaimBasis, EpistemicTier, EvidenceDetail, EvidenceValue, EvidencedAssertion,
    ObservationMethod, ObserverRole, TemporalObservationAxis, TemporalStabilityClass, Uncertainty,
};
use serde::{Deserialize, Serialize};

use crate::clinical::ClaimStatus;
use crate::record::{
    EpistemicStatus, EvidenceType, InstantBridge, RecordEnvelope, SensitivityClass,
};

/// Where a value came from. Extensible via [`InstrumentKind::Other`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind {
    /// The person typed or declared it.
    Declared,
    Scale,
    Tape,
    Watch,
    ClinicalDevice,
    Camera,
    Other(String),
}

/// How (if at all) this observation may affect the rendered body.
/// Closed on purpose: a new watch metric does not get a new shader until
/// a bind is curated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationBind {
    /// Stored only — the default for unknown codes.
    None,
    /// Drives [`super::constitution::BodyFit`] stature.
    FitStature,
    /// Reserved for a future weight morph (not implemented).
    FitWeight,
    /// Tints eye meshes if present.
    AppearanceEye,
    /// Tints hair meshes if present.
    AppearanceHair,
    /// Tints skin envelope if present (declared; not a race code).
    AppearanceSkin,
    /// Pulse / rate overlay on the circulatory system.
    CirculatoryRate,
    /// Informs graph considerations (prevalence, pharmacology, screening).
    /// **Never** the mesh shape, karyotype, or a skin/hair tint.
    KnowledgeContext,
    /// Generic overlay on a named body-system id.
    OverlayOnSystem { system_id: String },
}

/// One coded observation about the subject. Integer value + UCUM unit
/// (no float health arithmetic in the record).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyObservation {
    /// LOINC, UCUM-backed house IRI, or `q42:` token.
    pub code: String,
    pub value_milli: i64,
    /// UCUM or a short token (`mm`, `g`, `bpm`, `{named}`).
    pub unit: String,
    pub instrument: InstrumentKind,
    pub at_unix: u32,
    /// Optional human caption (“left wrist watch”).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Named value for `{named}` codes (ethnicity, eye/hair/skin). Integer `value_milli` stays 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named: Option<String>,
}

/// A seed code the environment already knows how to bind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownObservationCode {
    pub code: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub bind: RepresentationBindSeed,
}

/// Copy-friendly bind for the seed table (`OverlayOnSystem` is not in the seed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentationBindSeed {
    None,
    FitStature,
    FitWeight,
    AppearanceEye,
    AppearanceHair,
    AppearanceSkin,
    CirculatoryRate,
    KnowledgeContext,
}

impl RepresentationBindSeed {
    /// Converts into a bind.
    pub fn into_bind(self) -> RepresentationBind {
        match self {
            Self::None => RepresentationBind::None,
            Self::FitStature => RepresentationBind::FitStature,
            Self::FitWeight => RepresentationBind::FitWeight,
            Self::AppearanceEye => RepresentationBind::AppearanceEye,
            Self::AppearanceHair => RepresentationBind::AppearanceHair,
            Self::AppearanceSkin => RepresentationBind::AppearanceSkin,
            Self::CirculatoryRate => RepresentationBind::CirculatoryRate,
            Self::KnowledgeContext => RepresentationBind::KnowledgeContext,
        }
    }
}

/// Seed binds. New instruments: add a row here or register via the graph (W16).
pub const KNOWN_OBSERVATION_CODES: &[KnownObservationCode] = &[
    KnownObservationCode {
        code: "8302-2",
        label: "body height",
        unit: "mm",
        bind: RepresentationBindSeed::FitStature,
    },
    KnownObservationCode {
        code: "29463-7",
        label: "body weight",
        unit: "g",
        bind: RepresentationBindSeed::FitWeight,
    },
    KnownObservationCode {
        code: "8867-4",
        label: "heart rate",
        unit: "/min",
        bind: RepresentationBindSeed::CirculatoryRate,
    },
    KnownObservationCode {
        code: "q42:eye-colour",
        label: "eye colour (declared)",
        unit: "{named}",
        bind: RepresentationBindSeed::AppearanceEye,
    },
    KnownObservationCode {
        code: "q42:hair-colour",
        label: "hair colour (declared)",
        unit: "{named}",
        bind: RepresentationBindSeed::AppearanceHair,
    },
    KnownObservationCode {
        code: "q42:skin-tone",
        label: "skin tone (declared; not inferred from ethnicity)",
        unit: "{named}",
        bind: RepresentationBindSeed::AppearanceSkin,
    },
    KnownObservationCode {
        code: "q42:ethnicity",
        label: "self-identified ethnicity (declared, repeatable)",
        unit: "{named}",
        bind: RepresentationBindSeed::KnowledgeContext,
    },
    KnownObservationCode {
        code: "q42:genetic-ancestry",
        label: "genetic ancestry context (only if the person imported a record)",
        unit: "{named}",
        bind: RepresentationBindSeed::KnowledgeContext,
    },
    KnownObservationCode {
        code: "38265-5",
        label: "DEXA areal bone mineral density",
        unit: "g/cm2",
        bind: RepresentationBindSeed::None,
    },
    KnownObservationCode {
        code: "q42:qct-vbmd",
        label: "QCT volumetric bone mineral density",
        unit: "mg/cm3",
        bind: RepresentationBindSeed::None,
    },
    KnownObservationCode {
        code: "q42:muscle-hu",
        label: "CT skeletal muscle radiodensity",
        unit: "[hnsf'U]",
        bind: RepresentationBindSeed::None,
    },
];

/// Bind for a code. Unknown → [`RepresentationBind::None`] (extensible store).
pub fn bind_for_code(code: &str) -> RepresentationBind {
    KNOWN_OBSERVATION_CODES
        .iter()
        .find(|k| k.code == code.trim())
        .map(|k| k.bind.into_bind())
        .unwrap_or(RepresentationBind::None)
}

///  when known-code.
pub fn is_known_code(code: &str) -> bool {
    KNOWN_OBSERVATION_CODES
        .iter()
        .any(|k| k.code == code.trim())
}

// ===========================================================================
// BIO-WP-05: Evidenced Anatomical Observation Adapter & Sovereign Sealing
// ===========================================================================

/// Enforce sovereign biomedical privacy: personal/clinical anatomical records are
/// never sealed as `SensitivityClass::Public` (upgraded to `Restricted`, while
/// preserving `Classified`).
fn enforce_private_sensitivity(sensitivity: SensitivityClass) -> SensitivityClass {
    match sensitivity {
        SensitivityClass::Public => SensitivityClass::Restricted,
        SensitivityClass::Restricted => SensitivityClass::Restricted,
        SensitivityClass::Classified => SensitivityClass::Classified,
    }
}

/// Subject-scoped anatomical or densitometric observation wrapping
/// [`EvidenceDetail`] (7-tier [`EpistemicTier`], `q42m:*` [`ObservationMethod`],
/// [`ObserverRole`], and 4-axis [`TemporalObservationAxis`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnatomicalObservationRecord {
    pub id: String,
    pub subject_uri: String,
    pub root_source_id: String,
    pub scope_domain: String,
    #[serde(default)]
    pub derived_from_assertion_ids: Vec<String>,
    pub sensitivity: SensitivityClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_status: Option<ClaimStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_label: Option<String>,
    pub evidence: EvidenceDetail,
}

impl AnatomicalObservationRecord {
    /// Physical anthropometry observation via flexible anthropometric tape measure.
    #[allow(clippy::too_many_arguments)]
    pub fn from_tape_measure(
        id: impl Into<String>,
        subject_uri: impl Into<String>,
        property_iri: impl Into<String>,
        value_mm: f64,
        observer_role: ObserverRole,
        phenomenon_time_ms: i64,
        uncertainty_mm: Option<f64>,
        part_iri: Option<String>,
    ) -> Result<Self, String> {
        let id = id.into();
        let subject_uri = subject_uri.into();
        let property_iri = property_iri.into();
        if id.trim().is_empty() || subject_uri.trim().is_empty() {
            return Err("id and subject_uri are required".into());
        }
        if !value_mm.is_finite() || value_mm <= 0.0 {
            return Err("tape measure value_mm must be finite and positive".into());
        }
        let uncertainty = match uncertainty_mm {
            Some(sd) => {
                if !sd.is_finite() || sd < 0.0 {
                    return Err("uncertainty_mm must be finite and non-negative".into());
                }
                Some(Uncertainty {
                    standard_deviation: Some(sd),
                    interval_confidence: Some(0.95),
                    ..Default::default()
                })
            }
            None => None,
        };
        let axis = TemporalObservationAxis::with_stability_class(
            phenomenon_time_ms,
            TemporalStabilityClass::SubacuteMetabolicGravidAndHealing,
        );
        let detail = EvidenceDetail {
            property_iri,
            basis: ClaimBasis::Measured,
            value: Some(EvidenceValue::Quantity {
                value: value_mm,
                unit_iri: "http://qudt.org/vocab/unit/MilliM".into(),
            }),
            uncertainty,
            method_iri: Some(ObservationMethod::FlexibleTapeCircumference.as_iri()),
            scope: BiologicalScope {
                part_iri,
                individual_iri: Some(subject_uri.clone()),
                ..Default::default()
            },
            epistemic_tier: Some(EpistemicTier::DirectlyObserved),
            observation_method: Some(ObservationMethod::FlexibleTapeCircumference),
            observer_role: Some(observer_role),
            temporal_axis: Some(axis),
            stability_class: Some(TemporalStabilityClass::SubacuteMetabolicGravidAndHealing),
            ..Default::default()
        };
        detail.validate()?;
        Ok(Self {
            id: id.clone(),
            subject_uri,
            root_source_id: format!("src:tape:{id}"),
            scope_domain: "welfare:anatomy:anthropometry".into(),
            derived_from_assertion_ids: Vec::new(),
            sensitivity: SensitivityClass::Restricted,
            claim_status: None,
            summary_label: Some("tape-measure anthropometry".into()),
            evidence: detail,
        })
    }

    /// Clinical DEXA areal bone mineral density (`g/cm^2`) observation.
    #[allow(clippy::too_many_arguments)]
    pub fn from_dexa_scan(
        id: impl Into<String>,
        subject_uri: impl Into<String>,
        property_iri: impl Into<String>,
        areal_bmd_g_cm2: f64,
        phenomenon_time_ms: i64,
        acquisition_time_ms: Option<i64>,
        part_iri: Option<String>,
        root_source_id: impl Into<String>,
    ) -> Result<Self, String> {
        let id = id.into();
        let subject_uri = subject_uri.into();
        let property_iri = property_iri.into();
        let root_source_id = root_source_id.into();
        if id.trim().is_empty()
            || subject_uri.trim().is_empty()
            || root_source_id.trim().is_empty()
        {
            return Err("id, subject_uri, and root_source_id are required".into());
        }
        if !areal_bmd_g_cm2.is_finite() || areal_bmd_g_cm2 < 0.0 {
            return Err("areal_bmd_g_cm2 must be finite and non-negative".into());
        }
        let mut axis = TemporalObservationAxis::with_stability_class(
            phenomenon_time_ms,
            TemporalStabilityClass::OntogeneticAndSkeletalRemodeling,
        );
        axis.acquisition_time_ms = acquisition_time_ms;
        let detail = EvidenceDetail {
            property_iri,
            basis: ClaimBasis::Measured,
            value: Some(EvidenceValue::Quantity {
                value: areal_bmd_g_cm2,
                unit_iri: "http://qudt.org/vocab/unit/GM-PER-CentiM2".into(),
            }),
            method_iri: Some(ObservationMethod::DexaArealDensitometry.as_iri()),
            scope: BiologicalScope {
                part_iri,
                individual_iri: Some(subject_uri.clone()),
                ..Default::default()
            },
            epistemic_tier: Some(EpistemicTier::DirectlyObserved),
            observation_method: Some(ObservationMethod::DexaArealDensitometry),
            observer_role: Some(ObserverRole::ClinicianMeasured),
            temporal_axis: Some(axis),
            stability_class: Some(TemporalStabilityClass::OntogeneticAndSkeletalRemodeling),
            ..Default::default()
        };
        detail.validate()?;
        Ok(Self {
            id,
            subject_uri,
            root_source_id,
            scope_domain: "welfare:anatomy:densitometry:dexa".into(),
            derived_from_assertion_ids: Vec::new(),
            sensitivity: SensitivityClass::Restricted,
            claim_status: Some(ClaimStatus::ClinicianConfirmed),
            summary_label: Some("DEXA areal BMD".into()),
            evidence: detail,
        })
    }

    /// Quantitative CT volumetric bone mineral density (`mg HA/cm^3`) voxel observation.
    #[allow(clippy::too_many_arguments)]
    pub fn from_qct_voxel(
        id: impl Into<String>,
        subject_uri: impl Into<String>,
        property_iri: impl Into<String>,
        volumetric_bmd_mg_ha_cm3: f64,
        phenomenon_time_ms: i64,
        acquisition_time_ms: Option<i64>,
        part_iri: Option<String>,
        root_source_id: impl Into<String>,
    ) -> Result<Self, String> {
        let id = id.into();
        let subject_uri = subject_uri.into();
        let property_iri = property_iri.into();
        let root_source_id = root_source_id.into();
        if id.trim().is_empty()
            || subject_uri.trim().is_empty()
            || root_source_id.trim().is_empty()
        {
            return Err("id, subject_uri, and root_source_id are required".into());
        }
        if !volumetric_bmd_mg_ha_cm3.is_finite() || volumetric_bmd_mg_ha_cm3 < 0.0 {
            return Err("volumetric_bmd_mg_ha_cm3 must be finite and non-negative".into());
        }
        let mut axis = TemporalObservationAxis::with_stability_class(
            phenomenon_time_ms,
            TemporalStabilityClass::OntogeneticAndSkeletalRemodeling,
        );
        axis.acquisition_time_ms = acquisition_time_ms;
        let detail = EvidenceDetail {
            property_iri,
            basis: ClaimBasis::Measured,
            value: Some(EvidenceValue::Quantity {
                value: volumetric_bmd_mg_ha_cm3,
                unit_iri: "http://qudt.org/vocab/unit/MilliGM-PER-CentiM3".into(),
            }),
            method_iri: Some(ObservationMethod::ClinicalCtQuantitativeQct.as_iri()),
            scope: BiologicalScope {
                part_iri,
                individual_iri: Some(subject_uri.clone()),
                ..Default::default()
            },
            epistemic_tier: Some(EpistemicTier::DirectlyObserved),
            observation_method: Some(ObservationMethod::ClinicalCtQuantitativeQct),
            observer_role: Some(ObserverRole::ClinicianMeasured),
            temporal_axis: Some(axis),
            stability_class: Some(TemporalStabilityClass::OntogeneticAndSkeletalRemodeling),
            ..Default::default()
        };
        detail.validate()?;
        Ok(Self {
            id,
            subject_uri,
            root_source_id,
            scope_domain: "welfare:anatomy:densitometry:qct".into(),
            derived_from_assertion_ids: Vec::new(),
            sensitivity: SensitivityClass::Restricted,
            claim_status: Some(ClaimStatus::ClinicianConfirmed),
            summary_label: Some("QCT volumetric BMD".into()),
            evidence: detail,
        })
    }

    /// Clinical FHIR / EHR / narrative record extraction preserving documentary provenance
    /// and [`ClaimStatus`] honesty (unconfirmed records never launder into clinician evidence).
    #[allow(clippy::too_many_arguments)]
    pub fn from_clinical_record(
        id: impl Into<String>,
        subject_uri: impl Into<String>,
        property_iri: impl Into<String>,
        value: EvidenceValue,
        claim_status: ClaimStatus,
        reported_method: Option<ObservationMethod>,
        phenomenon_time_ms: i64,
        extraction_time_ms: i64,
        source_document_id: impl Into<String>,
    ) -> Result<Self, String> {
        let id = id.into();
        let subject_uri = subject_uri.into();
        let property_iri = property_iri.into();
        let source_document_id = source_document_id.into();
        if id.trim().is_empty()
            || subject_uri.trim().is_empty()
            || source_document_id.trim().is_empty()
        {
            return Err("id, subject_uri, and source_document_id are required".into());
        }
        let method = reported_method.unwrap_or(ObservationMethod::ElectronicHealthRecordFhir);
        let mut axis = TemporalObservationAxis::with_stability_class(
            phenomenon_time_ms,
            TemporalStabilityClass::OntogeneticAndSkeletalRemodeling,
        );
        axis.extraction_time_ms = Some(extraction_time_ms);
        let detail = EvidenceDetail {
            property_iri,
            basis: ClaimBasis::Described,
            value: Some(value),
            method_iri: Some(method.as_iri()),
            scope: BiologicalScope {
                individual_iri: Some(subject_uri.clone()),
                ..Default::default()
            },
            source_record_id: Some(source_document_id.clone()),
            epistemic_tier: Some(EpistemicTier::ExtractedFromRecord),
            observation_method: Some(method),
            observer_role: Some(ObserverRole::CuratorExtracted),
            temporal_axis: Some(axis),
            stability_class: Some(TemporalStabilityClass::OntogeneticAndSkeletalRemodeling),
            ..Default::default()
        };
        detail.validate()?;
        Ok(Self {
            id,
            subject_uri,
            root_source_id: source_document_id,
            scope_domain: "welfare:anatomy:clinical_record".into(),
            derived_from_assertion_ids: Vec::new(),
            sensitivity: SensitivityClass::Restricted,
            claim_status: Some(claim_status),
            summary_label: Some("FHIR/EHR clinical record extraction".into()),
            evidence: detail,
        })
    }

    /// Compile this record into a [`EvidencedAssertion`] enforcing non-public sensitivity.
    pub fn to_evidenced_assertion(
        &self,
        author_did: &str,
        asserted_at_unix: u32,
    ) -> Result<EvidencedAssertion, String> {
        self.evidence.validate()?;
        let statement_type = match self.evidence.epistemic_tier {
            Some(EpistemicTier::InferredFromObservation)
            | Some(EpistemicTier::LongitudinallyProjected)
            | Some(EpistemicTier::StatisticallyCalibrated)
            | Some(EpistemicTier::PhylogeneticallyBracketed) => StatementType::Inference,
            Some(EpistemicTier::AuthorialIntent) => StatementType::Attestation,
            _ => StatementType::Observation,
        };
        let verification_status = match self.claim_status {
            Some(ClaimStatus::ClinicianConfirmed) => VerificationStatus::Verified,
            Some(ClaimStatus::Disputed) => VerificationStatus::Disputed,
            _ => VerificationStatus::Proposed,
        };
        let assertion_status = match self.claim_status {
            Some(ClaimStatus::Disputed) => AssertionStatus::Challenged,
            Some(ClaimStatus::Superseded) => AssertionStatus::Superseded,
            _ => AssertionStatus::Active,
        };
        let ea = EvidencedAssertion {
            assertion: ContextualAssertion {
                id: self.id.clone(),
                subject_uri: self.subject_uri.clone(),
                statement_type,
                assertion_status,
                verification_status,
                claim_summary: self
                    .summary_label
                    .clone()
                    .unwrap_or_else(|| self.evidence.property_iri.clone()),
                author_did: author_did.to_string(),
                root_source_id: self.root_source_id.clone(),
                derived_from_assertion_ids: self.derived_from_assertion_ids.clone(),
                scope_domain: self.scope_domain.clone(),
                sensitivity: enforce_private_sensitivity(self.sensitivity),
                asserted_at_unix,
                expires_at_unix: None,
                superseded_by_id: None,
                adjudicated_by_human_did: None,
            },
            evidence: self.evidence.clone(),
        };
        ea.validate()?;
        Ok(ea)
    }

    /// Seal this observation into a sovereign [`RecordEnvelope`], enforcing at minimum
    /// [`SensitivityClass::Restricted`] (upgrading `Public` to `Restricted`) and
    /// preserving honest epistemic/evidence provenance.
    pub fn seal_into_envelope(
        &self,
        owner_did: &str,
        author_did: &str,
        asserted_unix: u32,
        blob_hash: Option<String>,
    ) -> RecordEnvelope {
        let (epistemic_status, evidence_type) = match self.claim_status {
            Some(ClaimStatus::Disputed) => (EpistemicStatus::Disputed, EvidenceType::SelfReported),
            Some(ClaimStatus::Draft | ClaimStatus::Submitted) => {
                (EpistemicStatus::Hypothesis, EvidenceType::SelfReported)
            }
            Some(ClaimStatus::ClinicianConfirmed) => {
                (EpistemicStatus::Asserted, EvidenceType::ClinicianObserved)
            }
            Some(ClaimStatus::Superseded) => (EpistemicStatus::Asserted, EvidenceType::SelfReported),
            None => match self.evidence.epistemic_tier {
                Some(EpistemicTier::DirectlyObserved) => match self.evidence.observer_role {
                    Some(ObserverRole::ClinicianMeasured | ObserverRole::IsakCertified) => {
                        (EpistemicStatus::Asserted, EvidenceType::ClinicianObserved)
                    }
                    Some(ObserverRole::AutomatedSensor) => {
                        (EpistemicStatus::Asserted, EvidenceType::DeviceMeasured)
                    }
                    _ => match self.evidence.observation_method {
                        Some(ObservationMethod::SubjectSelfReportQuestionnaire) => {
                            (EpistemicStatus::Asserted, EvidenceType::SelfReported)
                        }
                        Some(_) => (EpistemicStatus::Asserted, EvidenceType::DeviceMeasured),
                        None => (EpistemicStatus::Asserted, EvidenceType::SelfReported),
                    },
                },
                Some(EpistemicTier::InferredFromObservation)
                | Some(EpistemicTier::LongitudinallyProjected)
                | Some(EpistemicTier::StatisticallyCalibrated)
                | Some(EpistemicTier::PhylogeneticallyBracketed) => {
                    (EpistemicStatus::Hypothesis, EvidenceType::Inferred)
                }
                Some(EpistemicTier::AuthorialIntent) => {
                    (EpistemicStatus::Hypothesis, EvidenceType::SelfReported)
                }
                Some(EpistemicTier::ExtractedFromRecord) | None => {
                    (EpistemicStatus::Asserted, EvidenceType::SelfReported)
                }
            },
        };

        let valid_start_instant = self.evidence.temporal_axis.map(|axis| {
            let secs = axis.phenomenon_time_ms.div_euclid(1000);
            let rem_ms = axis.phenomenon_time_ms.rem_euclid(1000) as u32;
            InstantBridge::unix(secs, rem_ms * 1_000_000)
        });
        let valid_start_unix = valid_start_instant.map(|i| i.secs.max(0) as u32);

        RecordEnvelope {
            id: format!("urn:wellfair:anatomical_observation:{}", self.id),
            owner_did: owner_did.to_string(),
            author_did: author_did.to_string(),
            proxy_did: None,
            epistemic_status,
            evidence_type,
            sensitivity: enforce_private_sensitivity(self.sensitivity),
            asserted_time_unix: asserted_unix,
            asserted_instant: Some(InstantBridge::from_coarse(asserted_unix)),
            valid_time_start_unix: valid_start_unix,
            valid_time_start_instant: valid_start_instant,
            valid_time_end_unix: None,
            valid_time_end_instant: None,
            predecessor_id: None,
            blob_hash,
            tombstone: false,
        }
    }
}

impl BodyObservation {
    /// Convert a legacy [`BodyObservation`] into an [`AnatomicalObservationRecord`]
    /// with explicit [`EpistemicTier`], [`ObservationMethod`], [`ObserverRole`], and
    /// [`TemporalObservationAxis`].
    pub fn to_anatomical_record(
        &self,
        id: impl Into<String>,
        subject_uri: impl Into<String>,
        scope_domain: impl Into<String>,
    ) -> Result<AnatomicalObservationRecord, String> {
        let id = id.into();
        let subject_uri = subject_uri.into();
        let scope_domain = scope_domain.into();
        if id.trim().is_empty() || subject_uri.trim().is_empty() {
            return Err("id and subject_uri are required".into());
        }
        let (method, role, tier, basis, stability) = match &self.instrument {
            InstrumentKind::Tape => (
                ObservationMethod::FlexibleTapeCircumference,
                ObserverRole::SelfMeasured,
                EpistemicTier::DirectlyObserved,
                ClaimBasis::Measured,
                TemporalStabilityClass::SubacuteMetabolicGravidAndHealing,
            ),
            InstrumentKind::Scale => (
                ObservationMethod::GravimetricScale,
                ObserverRole::SelfMeasured,
                EpistemicTier::DirectlyObserved,
                ClaimBasis::Measured,
                TemporalStabilityClass::SubacuteMetabolicGravidAndHealing,
            ),
            InstrumentKind::Watch => (
                ObservationMethod::WearablePpgSmartwatch,
                ObserverRole::AutomatedSensor,
                EpistemicTier::DirectlyObserved,
                ClaimBasis::Measured,
                TemporalStabilityClass::AcutePhysiological,
            ),
            InstrumentKind::ClinicalDevice => {
                let m = if self.code.contains("qct") {
                    ObservationMethod::ClinicalCtQuantitativeQct
                } else {
                    ObservationMethod::DexaArealDensitometry
                };
                (
                    m,
                    ObserverRole::ClinicianMeasured,
                    EpistemicTier::DirectlyObserved,
                    ClaimBasis::Measured,
                    TemporalStabilityClass::OntogeneticAndSkeletalRemodeling,
                )
            }
            InstrumentKind::Camera => (
                ObservationMethod::MultiViewPhotogrammetry,
                ObserverRole::AutomatedSensor,
                EpistemicTier::DirectlyObserved,
                ClaimBasis::Measured,
                TemporalStabilityClass::SubacuteMetabolicGravidAndHealing,
            ),
            InstrumentKind::Declared => (
                ObservationMethod::SubjectSelfReportQuestionnaire,
                ObserverRole::SelfMeasured,
                EpistemicTier::AuthorialIntent,
                ClaimBasis::Described,
                TemporalStabilityClass::SubacuteMetabolicGravidAndHealing,
            ),
            InstrumentKind::Other(_) => (
                ObservationMethod::SubjectSelfReportQuestionnaire,
                ObserverRole::SelfMeasured,
                EpistemicTier::ExtractedFromRecord,
                ClaimBasis::Described,
                TemporalStabilityClass::SubacuteMetabolicGravidAndHealing,
            ),
        };

        let value = if let Some(named) = self.named.as_ref().filter(|s| !s.trim().is_empty()) {
            EvidenceValue::Text {
                text: named.clone(),
                language: None,
            }
        } else {
            EvidenceValue::Quantity {
                value: self.value_milli as f64 / 1000.0,
                unit_iri: if self.unit.trim().is_empty() {
                    "urn:ucum:1".into()
                } else {
                    format!("urn:ucum:{}", self.unit.trim())
                },
            }
        };

        let phenomenon_time_ms = (self.at_unix as i64) * 1000;
        let axis = TemporalObservationAxis::with_stability_class(phenomenon_time_ms, stability);
        let detail = EvidenceDetail {
            property_iri: format!("urn:code:{}", self.code.trim()),
            basis,
            value: Some(value),
            method_iri: Some(method.as_iri()),
            scope: BiologicalScope {
                individual_iri: Some(subject_uri.clone()),
                ..Default::default()
            },
            epistemic_tier: Some(tier),
            observation_method: Some(method),
            observer_role: Some(role),
            temporal_axis: Some(axis),
            stability_class: Some(stability),
            ..Default::default()
        };
        detail.validate()?;

        Ok(AnatomicalObservationRecord {
            id: id.clone(),
            subject_uri,
            root_source_id: format!("src:body_obs:{id}"),
            scope_domain,
            derived_from_assertion_ids: Vec::new(),
            sensitivity: SensitivityClass::Restricted,
            claim_status: None,
            summary_label: self.note.clone().or_else(|| Some(self.code.clone())),
            evidence: detail,
        })
    }

    /// Bridge directly into a validated [`EvidencedAssertion`].
    pub fn to_evidenced_assertion(
        &self,
        id: impl Into<String>,
        subject_uri: impl Into<String>,
        scope_domain: impl Into<String>,
        author_did: &str,
        asserted_at_unix: u32,
    ) -> Result<EvidencedAssertion, String> {
        self.to_anatomical_record(id, subject_uri, scope_domain)?
            .to_evidenced_assertion(author_did, asserted_at_unix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_instrument_code_is_stored_not_drawn() {
        let o = BodyObservation {
            code: "99999-9".into(),
            value_milli: 72_000,
            unit: "/min".into(),
            instrument: InstrumentKind::Watch,
            at_unix: 1,
            note: Some("new watch metric".into()),
            named: None,
        };
        assert!(!is_known_code(&o.code));
        assert_eq!(bind_for_code(&o.code), RepresentationBind::None);
    }

    #[test]
    fn pulse_binds_to_circulatory_rate_not_skeleton() {
        assert_eq!(bind_for_code("8867-4"), RepresentationBind::CirculatoryRate);
        assert_ne!(
            bind_for_code("8867-4"),
            RepresentationBind::OverlayOnSystem {
                system_id: "skeletal".into()
            }
        );
    }

    #[test]
    fn ethnicity_is_knowledge_context_never_appearance_or_fit() {
        let b = bind_for_code("q42:ethnicity");
        assert_eq!(b, RepresentationBind::KnowledgeContext);
        assert_ne!(b, RepresentationBind::AppearanceSkin);
        assert_ne!(b, RepresentationBind::AppearanceHair);
        assert_ne!(b, RepresentationBind::AppearanceEye);
        assert_ne!(b, RepresentationBind::FitStature);
        assert_eq!(
            bind_for_code("q42:genetic-ancestry"),
            RepresentationBind::KnowledgeContext
        );
    }

    #[test]
    fn stature_and_weight_and_appearance_are_seeded() {
        assert_eq!(bind_for_code("8302-2"), RepresentationBind::FitStature);
        assert_eq!(bind_for_code("29463-7"), RepresentationBind::FitWeight);
        assert_eq!(
            bind_for_code("q42:eye-colour"),
            RepresentationBind::AppearanceEye
        );
        assert_eq!(
            bind_for_code("q42:hair-colour"),
            RepresentationBind::AppearanceHair
        );
        assert_eq!(
            bind_for_code("q42:skin-tone"),
            RepresentationBind::AppearanceSkin
        );
    }

    #[test]
    fn constructors_and_envelope_sealing_preserve_epistemic_honesty() {
        use qualia_cooperative_core::contextual_evidence::EvidenceValue;
        use qualia_cooperative_core::record::{
            EpistemicStatus, EvidenceType, NQuin, SensitivityClass,
        };

        let tape = AnatomicalObservationRecord::from_tape_measure(
            "obs:tape-1",
            "did:qualia:person:ada",
            "q42:fit/thigh-circumference",
            520.0,
            ObserverRole::SelfMeasured,
            1_700_000_000_000,
            Some(3.0),
            Some("q42:anat/thigh-left".to_string()),
        )
        .expect("tape measure observation should succeed");
        assert_eq!(
            tape.evidence.epistemic_tier,
            Some(EpistemicTier::DirectlyObserved)
        );
        assert_eq!(
            tape.evidence.observation_method,
            Some(ObservationMethod::FlexibleTapeCircumference)
        );
        assert_eq!(tape.evidence.observer_role, Some(ObserverRole::SelfMeasured));
        assert_eq!(tape.sensitivity, SensitivityClass::Restricted);

        let dexa = AnatomicalObservationRecord::from_dexa_scan(
            "obs:dexa-1",
            "did:qualia:person:ada",
            "q42:densitometry/areal-bmd",
            0.94,
            1_700_000_010_000,
            Some(1_700_000_012_000),
            Some("q42:anat/femoral-neck-left".to_string()),
            "dexa-report-001",
        )
        .expect("dexa scan observation should succeed");
        assert_eq!(
            dexa.evidence.epistemic_tier,
            Some(EpistemicTier::DirectlyObserved)
        );
        assert_eq!(
            dexa.evidence.observation_method,
            Some(ObservationMethod::DexaArealDensitometry)
        );
        assert_eq!(
            dexa.evidence.observer_role,
            Some(ObserverRole::ClinicianMeasured)
        );

        let qct = AnatomicalObservationRecord::from_qct_voxel(
            "obs:qct-1",
            "did:qualia:person:ada",
            "q42:densitometry/volumetric-bmd",
            145.0,
            1_700_000_020_000,
            None,
            None,
            "qct-series-002",
        )
        .expect("qct voxel observation should succeed");
        assert_eq!(
            qct.evidence.epistemic_tier,
            Some(EpistemicTier::DirectlyObserved)
        );
        assert_eq!(
            qct.evidence.observation_method,
            Some(ObservationMethod::ClinicalCtQuantitativeQct)
        );

        // Unconfirmed clinical claim must downgrade from ClinicianObserved to SelfReported.
        let unconfirmed = AnatomicalObservationRecord::from_clinical_record(
            "obs:ehr-draft",
            "did:qualia:person:ada",
            "q42:densitometry/t-score",
            EvidenceValue::Quantity {
                value: -2.6,
                unit_iri: "urn:ucum:1".into(),
            },
            ClaimStatus::Draft,
            Some(ObservationMethod::ElectronicHealthRecordFhir),
            1_700_000_030_000,
            1_700_000_040_000,
            "fhir-bundle-1",
        )
        .expect("draft clinical record should build");
        let unconfirmed_env = unconfirmed.seal_into_envelope(
            "did:qualia:person:ada",
            "did:qualia:agent:ingest",
            1_700_000_040,
            None,
        );
        assert_eq!(unconfirmed_env.evidence_type, EvidenceType::SelfReported);
        assert_eq!(unconfirmed_env.sensitivity, SensitivityClass::Restricted);
        assert_eq!(unconfirmed_env.epistemic_status, EpistemicStatus::Hypothesis);

        // Disputed clinical claim must stay SelfReported and EpistemicStatus::Disputed.
        let disputed = AnatomicalObservationRecord::from_clinical_record(
            "obs:ehr-disputed",
            "did:qualia:person:ada",
            "q42:densitometry/t-score",
            EvidenceValue::Quantity {
                value: -2.6,
                unit_iri: "urn:ucum:1".into(),
            },
            ClaimStatus::Disputed,
            None,
            1_700_000_030_000,
            1_700_000_040_000,
            "fhir-bundle-2",
        )
        .expect("disputed clinical record should build");
        let disputed_env = disputed.seal_into_envelope(
            "did:qualia:person:ada",
            "did:qualia:agent:ingest",
            1_700_000_040,
            None,
        );
        assert_eq!(disputed_env.evidence_type, EvidenceType::SelfReported);
        assert_eq!(disputed_env.epistemic_status, EpistemicStatus::Disputed);

        // Confirmed clinical claim preserves ClinicianObserved and Asserted.
        let confirmed = AnatomicalObservationRecord::from_clinical_record(
            "obs:ehr-confirmed",
            "did:qualia:person:ada",
            "q42:densitometry/t-score",
            EvidenceValue::Quantity {
                value: -2.6,
                unit_iri: "urn:ucum:1".into(),
            },
            ClaimStatus::ClinicianConfirmed,
            None,
            1_700_000_030_000,
            1_700_000_040_000,
            "fhir-bundle-3",
        )
        .expect("confirmed clinical record should build");
        let confirmed_assertion = confirmed
            .to_evidenced_assertion("did:qualia:clinician:1", 1_700_000_040)
            .expect("evidenced assertion should build");
        let mut quins = [NQuin::default(); 64];
        let quin_count = confirmed_assertion.compile_to_quins(&mut quins);
        assert!(quin_count > 0);

        let confirmed_env = confirmed.seal_into_envelope(
            "did:qualia:person:ada",
            "did:qualia:clinician:1",
            1_700_000_040,
            None,
        );
        assert_eq!(confirmed_env.evidence_type, EvidenceType::ClinicianObserved);
        assert_eq!(confirmed_env.epistemic_status, EpistemicStatus::Asserted);
        assert_eq!(confirmed_env.sensitivity, SensitivityClass::Restricted);

        // Even if sensitivity is overridden to Public, seal_into_envelope upgrades it to Restricted.
        let mut forced_public = dexa.clone();
        forced_public.sensitivity = SensitivityClass::Public;
        let forced_env = forced_public.seal_into_envelope(
            "did:qualia:person:ada",
            "did:qualia:device:dexa",
            1_700_000_012,
            Some("blake3:deadbeef".to_string()),
        );
        assert_eq!(forced_env.sensitivity, SensitivityClass::Restricted);

        // BodyObservation bridges cleanly into AnatomicalObservationRecord + EvidencedAssertion.
        let legacy = BodyObservation {
            code: "8302-2".to_string(),
            instrument: InstrumentKind::Tape,
            value_milli: 1_680_000,
            unit: "mm".to_string(),
            at_unix: 1_700_000_050,
            note: Some("clinic-visit-1".into()),
            named: None,
        };
        let bridged = legacy
            .to_anatomical_record("obs:legacy-1", "did:qualia:person:ada", "anthropometry")
            .expect("BodyObservation should convert");
        assert_eq!(
            bridged.evidence.observation_method,
            Some(ObservationMethod::FlexibleTapeCircumference)
        );
        let bridged_assertion = legacy
            .to_evidenced_assertion(
                "obs:legacy-1",
                "did:qualia:person:ada",
                "anthropometry",
                "did:qualia:clinician:1",
                1_700_000_055,
            )
            .expect("BodyObservation evidenced assertion should compile");
        let mut bridged_quins = [NQuin::default(); 64];
        let bridged_quin_count = bridged_assertion.compile_to_quins(&mut bridged_quins);
        assert!(bridged_quin_count > 0);
    }
}
