//! Evidence detail for contextual assertions about subjects such as taxa, specimens and parts.
//!
//! A [`ContextualAssertion`](crate::contextual_assertion::ContextualAssertion) records who asserted
//! what, from which root source, with what lifecycle and verification status. [`EvidenceDetail`]
//! holds the content of a measured or described fact: property, value and unit, uncertainty,
//! method, observation/validity time, biological scope, the record or media region it came from,
//! and the basis on which it is held (measured, described, derived, model assumption, artistic
//! choice or unknown). [`EvidencedAssertion`] pairs the two without changing the existing
//! assertion type. Competing values for one property coexist as separate assertions.

use serde::{Deserialize, Serialize};

use crate::record::{q_hash_str, NQuin};

/// Basis on which an assertion's value is held. Keeps model assumptions and artistic choices
/// distinguishable from source evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimBasis {
    /// Measured by a stated method.
    Measured,
    /// Descriptive statement from a source (treatment, flora, description).
    Described,
    /// Derived by inference or calculation from other assertions.
    Derived,
    /// Model or simulation assumption, not evidence about the subject.
    ModelAssumption,
    /// Authored artistic or illustrative choice.
    ArtisticChoice,
    /// The value is unknown; the assertion records the gap.
    Unknown,
}

impl ClaimBasis {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::Described => "described",
            Self::Derived => "derived",
            Self::ModelAssumption => "model_assumption",
            Self::ArtisticChoice => "artistic_choice",
            Self::Unknown => "unknown",
        }
    }
}

impl Default for ClaimBasis {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Seven-tier epistemic provenance lattice (`epistemic:hasEpistemicStatus`).
///
/// Distinguishes direct physical readouts from subject-specific inferences, documentary
/// extractions, longitudinal projections, population priors, phylogenetic brackets, and
/// authorial intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicTier {
    /// 1:1 physical readout of the asserted quantity (CT/MRI/DEXA/XROMM/tape/caliper).
    DirectlyObserved,
    /// Computed deterministically from `DirectlyObserved` or `ExtractedFromRecord` inputs of
    /// the same subject via a named transfer formula.
    InferredFromObservation,
    /// Parsed or transcribed from a clinical record, report, or literature table.
    ExtractedFromRecord,
    /// Projected forward or backward in time from an earlier observation of the same subject.
    LongitudinallyProjected,
    /// Completed or regularised using population SSM/SDM or allometric scaling priors.
    StatisticallyCalibrated,
    /// Reconstructed across evolutionary homologs via Witmer's Extant Phylogenetic Bracket.
    PhylogeneticallyBracketed,
    /// Deliberately posed, sculpted, or hypothetically modified (e.g. clinical test or creative).
    AuthorialIntent,
}

impl EpistemicTier {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::DirectlyObserved => "directly_observed",
            Self::InferredFromObservation => "inferred_from_observation",
            Self::ExtractedFromRecord => "extracted_from_record",
            Self::LongitudinallyProjected => "longitudinally_projected",
            Self::StatisticallyCalibrated => "statistically_calibrated",
            Self::PhylogeneticallyBracketed => "phylogenetically_bracketed",
            Self::AuthorialIntent => "authorial_intent",
        }
    }

    pub const fn as_iri(&self) -> &'static str {
        match self {
            Self::DirectlyObserved => "https://ns.webizen.org/epistemic#DirectlyObserved",
            Self::InferredFromObservation => "https://ns.webizen.org/epistemic#InferredFromObservation",
            Self::ExtractedFromRecord => "https://ns.webizen.org/epistemic#ExtractedFromRecord",
            Self::LongitudinallyProjected => "https://ns.webizen.org/epistemic#LongitudinallyProjected",
            Self::StatisticallyCalibrated => "https://ns.webizen.org/epistemic#StatisticallyCalibrated",
            Self::PhylogeneticallyBracketed => "https://ns.webizen.org/epistemic#PhylogeneticallyBracketed",
            Self::AuthorialIntent => "https://ns.webizen.org/epistemic#AuthorialIntent",
        }
    }

    /// Ordinal rank in the 7-tier lattice (0 = most direct empirical observation).
    pub const fn rank(&self) -> u8 {
        match self {
            Self::DirectlyObserved => 0,
            Self::InferredFromObservation => 1,
            Self::ExtractedFromRecord => 2,
            Self::LongitudinallyProjected => 3,
            Self::StatisticallyCalibrated => 4,
            Self::PhylogeneticallyBracketed => 5,
            Self::AuthorialIntent => 6,
        }
    }
}

/// Normative `q42m:*` observation method taxonomy covering physical anthropometry,
/// volumetric/planar/densitometric imaging, biomechanical telemetry, and documentary records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationMethod {
    // 1. Direct Physical Anthropometry
    FlexibleTapeCircumference,
    LinearAnthropometryStadiometer,
    InfantLengthBoard,
    OsteometricBoard,
    SlidingCaliper,
    SpreadingCaliper,
    SkinfoldCaliper,
    GravimetricScale,
    BioelectricalImpedanceBia,
    ClinicalGoniometer,
    DigitalInclinometer,
    // 2. Volumetric, Planar & Densitometric Imaging
    DexaArealDensitometry,
    ClinicalCtQuantitativeQct,
    ClinicalCtStandard,
    HrPqctMicroarchitecture,
    MicroCtSynchrotron,
    ClinicalMriStructural,
    ClinicalMriDixonPdff,
    ClinicalMriDti,
    DiagnosticUltrasound2D,
    DiagnosticUltrasound3D,
    ShearWaveElastography,
    QuantitativeUltrasoundCalcaneal,
    PlanarRadiographXRay,
    EosSlotScanningBiplanar,
    BiplanarFluoroscopyXromm,
    Optical3DSurfaceScan,
    MultiViewPhotogrammetry,
    TerrestrialLidarTls,
    // 3. Kinematic, Kinetic & Wearable Telemetry
    OpticalMarkerMoCap,
    MarkerlessMultiViewPose,
    InertialImuArray,
    ForcePlateGrf,
    InShoePressureInsole,
    GaitMatPressureWalkway,
    IsokineticDynamometry,
    HandgripDynamometry,
    SurfaceEmg,
    FineWireEmg,
    Spirometry,
    WearablePpgSmartwatch,
    ElectrocardiographyEcg,
    // 4. Clinical Records, Lab Assays & Documentary Sources
    ElectronicHealthRecordFhir,
    Hl7v2ClinicalFeed,
    DicomStructuredReport,
    ClinicalLaboratoryAssay,
    IcdCodedDiagnosis,
    PathologyReport,
    ClinicalNarrativeDocument,
    PublishedScientificLiterature,
    MuseumSpecimenRegister,
    SubjectSelfReportQuestionnaire,
}

impl ObservationMethod {
    pub const fn as_curie(&self) -> &'static str {
        match self {
            Self::FlexibleTapeCircumference => "q42m:FlexibleTapeCircumference",
            Self::LinearAnthropometryStadiometer => "q42m:LinearAnthropometry_Stadiometer",
            Self::InfantLengthBoard => "q42m:InfantLengthBoard",
            Self::OsteometricBoard => "q42m:OsteometricBoard",
            Self::SlidingCaliper => "q42m:SlidingCaliper",
            Self::SpreadingCaliper => "q42m:SpreadingCaliper",
            Self::SkinfoldCaliper => "q42m:SkinfoldCaliper",
            Self::GravimetricScale => "q42m:GravimetricScale",
            Self::BioelectricalImpedanceBia => "q42m:BioelectricalImpedance_BIA",
            Self::ClinicalGoniometer => "q42m:ClinicalGoniometer",
            Self::DigitalInclinometer => "q42m:DigitalInclinometer",
            Self::DexaArealDensitometry => "q42m:DEXA_ArealDensitometry",
            Self::ClinicalCtQuantitativeQct => "q42m:ClinicalCT_Quantitative_QCT",
            Self::ClinicalCtStandard => "q42m:ClinicalCT_Standard",
            Self::HrPqctMicroarchitecture => "q42m:HR_pQCT_Microarchitecture",
            Self::MicroCtSynchrotron => "q42m:MicroCT_Synchrotron",
            Self::ClinicalMriStructural => "q42m:ClinicalMRI_Structural",
            Self::ClinicalMriDixonPdff => "q42m:ClinicalMRI_DixonPDFF",
            Self::ClinicalMriDti => "q42m:ClinicalMRI_DTI",
            Self::DiagnosticUltrasound2D => "q42m:DiagnosticUltrasound_2D",
            Self::DiagnosticUltrasound3D => "q42m:DiagnosticUltrasound_3D",
            Self::ShearWaveElastography => "q42m:ShearWaveElastography",
            Self::QuantitativeUltrasoundCalcaneal => "q42m:QuantitativeUltrasound_Calcaneal",
            Self::PlanarRadiographXRay => "q42m:PlanarRadiograph_XRay",
            Self::EosSlotScanningBiplanar => "q42m:EOS_SlotScanningBiplanar",
            Self::BiplanarFluoroscopyXromm => "q42m:BiplanarFluoroscopy_XROMM",
            Self::Optical3DSurfaceScan => "q42m:Optical3DSurfaceScan",
            Self::MultiViewPhotogrammetry => "q42m:MultiViewPhotogrammetry",
            Self::TerrestrialLidarTls => "q42m:TerrestrialLiDAR_TLS",
            Self::OpticalMarkerMoCap => "q42m:OpticalMarkerMoCap",
            Self::MarkerlessMultiViewPose => "q42m:MarkerlessMultiViewPose",
            Self::InertialImuArray => "q42m:InertialIMU_Array",
            Self::ForcePlateGrf => "q42m:ForcePlate_GRF",
            Self::InShoePressureInsole => "q42m:InShoePressureInsole",
            Self::GaitMatPressureWalkway => "q42m:GaitMat_PressureWalkway",
            Self::IsokineticDynamometry => "q42m:IsokineticDynamometry",
            Self::HandgripDynamometry => "q42m:HandgripDynamometry",
            Self::SurfaceEmg => "q42m:SurfaceEMG_sEMG",
            Self::FineWireEmg => "q42m:FineWireEMG",
            Self::Spirometry => "q42m:Spirometry",
            Self::WearablePpgSmartwatch => "q42m:WearablePPG_Smartwatch",
            Self::ElectrocardiographyEcg => "q42m:Electrocardiography_ECG",
            Self::ElectronicHealthRecordFhir => "q42m:ElectronicHealthRecord_FHIR",
            Self::Hl7v2ClinicalFeed => "q42m:HL7v2_ClinicalFeed",
            Self::DicomStructuredReport => "q42m:DICOM_StructuredReport",
            Self::ClinicalLaboratoryAssay => "q42m:ClinicalLaboratoryAssay",
            Self::IcdCodedDiagnosis => "q42m:ICD_CodedDiagnosis",
            Self::PathologyReport => "q42m:PathologyReport",
            Self::ClinicalNarrativeDocument => "q42m:ClinicalNarrativeDocument",
            Self::PublishedScientificLiterature => "q42m:PublishedScientificLiterature",
            Self::MuseumSpecimenRegister => "q42m:MuseumSpecimenRegister",
            Self::SubjectSelfReportQuestionnaire => "q42m:SubjectSelfReport_Questionnaire",
        }
    }

    pub fn as_iri(&self) -> String {
        let local = self.as_curie().strip_prefix("q42m:").unwrap_or(self.as_curie());
        format!("https://ns.webizen.org/epistemic/method#{local}")
    }

    /// 1 = PhysicalAnthropometry, 2 = VolumetricAndPlanarImaging, 3 = BiomechanicalTelemetry,
    /// 4 = DocumentaryAndRecordExtraction.
    pub const fn category_code(&self) -> u8 {
        match self {
            Self::FlexibleTapeCircumference
            | Self::LinearAnthropometryStadiometer
            | Self::InfantLengthBoard
            | Self::OsteometricBoard
            | Self::SlidingCaliper
            | Self::SpreadingCaliper
            | Self::SkinfoldCaliper
            | Self::GravimetricScale
            | Self::BioelectricalImpedanceBia
            | Self::ClinicalGoniometer
            | Self::DigitalInclinometer => 1,
            Self::DexaArealDensitometry
            | Self::ClinicalCtQuantitativeQct
            | Self::ClinicalCtStandard
            | Self::HrPqctMicroarchitecture
            | Self::MicroCtSynchrotron
            | Self::ClinicalMriStructural
            | Self::ClinicalMriDixonPdff
            | Self::ClinicalMriDti
            | Self::DiagnosticUltrasound2D
            | Self::DiagnosticUltrasound3D
            | Self::ShearWaveElastography
            | Self::QuantitativeUltrasoundCalcaneal
            | Self::PlanarRadiographXRay
            | Self::EosSlotScanningBiplanar
            | Self::BiplanarFluoroscopyXromm
            | Self::Optical3DSurfaceScan
            | Self::MultiViewPhotogrammetry
            | Self::TerrestrialLidarTls => 2,
            Self::OpticalMarkerMoCap
            | Self::MarkerlessMultiViewPose
            | Self::InertialImuArray
            | Self::ForcePlateGrf
            | Self::InShoePressureInsole
            | Self::GaitMatPressureWalkway
            | Self::IsokineticDynamometry
            | Self::HandgripDynamometry
            | Self::SurfaceEmg
            | Self::FineWireEmg
            | Self::Spirometry
            | Self::WearablePpgSmartwatch
            | Self::ElectrocardiographyEcg => 3,
            Self::ElectronicHealthRecordFhir
            | Self::Hl7v2ClinicalFeed
            | Self::DicomStructuredReport
            | Self::ClinicalLaboratoryAssay
            | Self::IcdCodedDiagnosis
            | Self::PathologyReport
            | Self::ClinicalNarrativeDocument
            | Self::PublishedScientificLiterature
            | Self::MuseumSpecimenRegister
            | Self::SubjectSelfReportQuestionnaire => 4,
        }
    }
}

/// Role of the observer who acquired or extracted the evidence (`q42:ObserverRole`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObserverRole {
    SelfMeasured,
    PeerMeasured,
    ClinicianMeasured,
    IsakCertified,
    AutomatedSensor,
    CuratorExtracted,
}

impl ObserverRole {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SelfMeasured => "self_measured",
            Self::PeerMeasured => "peer_measured",
            Self::ClinicianMeasured => "clinician_measured",
            Self::IsakCertified => "isak_certified",
            Self::AutomatedSensor => "automated_sensor",
            Self::CuratorExtracted => "curator_extracted",
        }
    }

    pub const fn as_iri(&self) -> &'static str {
        match self {
            Self::SelfMeasured => "https://ns.webizen.org/epistemic#SelfMeasured",
            Self::PeerMeasured => "https://ns.webizen.org/epistemic#PeerMeasured",
            Self::ClinicianMeasured => "https://ns.webizen.org/epistemic#ClinicianMeasured",
            Self::IsakCertified => "https://ns.webizen.org/epistemic#ISAKCertified",
            Self::AutomatedSensor => "https://ns.webizen.org/epistemic#AutomatedSensor",
            Self::CuratorExtracted => "https://ns.webizen.org/epistemic#CuratorExtracted",
        }
    }
}

/// Biological freshness stability class governing half-life decay ($t_{1/2}$).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemporalStabilityClass {
    /// $t_{1/2} \sim 10^1\text{–}10^3\text{ s}$ (heart rate, EMG, acute posture).
    AcutePhysiological,
    /// $t_{1/2} \sim 10^5\text{–}10^6\text{ s}$ (girths, body weight, gestation, fracture callus).
    SubacuteMetabolicGravidAndHealing,
    /// $t_{1/2} \sim 10^7\text{–}10^8\text{ s}$ (aBMD/vBMD, muscle PCSA/HU, pediatric growth).
    OntogeneticAndSkeletalRemodeling,
    /// $t_{1/2} > 10^9\text{ s}$ (adult diaphyseal length, fossil geometry).
    SkeletallyMatureInvariant,
}

impl TemporalStabilityClass {
    /// Default biological half-life in days (`None` = invariant / non-decaying).
    pub const fn default_half_life_days(&self) -> Option<f64> {
        match self {
            Self::AcutePhysiological => Some(0.005), // ~432 s
            Self::SubacuteMetabolicGravidAndHealing => Some(7.0),
            Self::OntogeneticAndSkeletalRemodeling => Some(365.0),
            Self::SkeletallyMatureInvariant => None,
        }
    }
}

/// Four-axis temporal provenance & exponential freshness decay model.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct TemporalObservationAxis {
    /// When the physical state existed in the organism (Unix epoch ms).
    pub phenomenon_time_ms: i64,
    /// When the sensor, imaging study, or clinician captured the measurement (Unix epoch ms).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquisition_time_ms: Option<i64>,
    /// When the datum was parsed or transcribed from a document/record (Unix epoch ms).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extraction_time_ms: Option<i64>,
    /// When the assertion was compiled into the `.q42`/`.10d`/`.m64` model (Unix epoch ms).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration_time_ms: Option<i64>,
    /// Biological half-life in days (`None` = invariant / no temporal decay).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub half_life_days: Option<f64>,
}

impl TemporalObservationAxis {
    pub const fn with_stability_class(
        phenomenon_time_ms: i64,
        class: TemporalStabilityClass,
    ) -> Self {
        Self {
            phenomenon_time_ms,
            acquisition_time_ms: None,
            extraction_time_ms: None,
            calibration_time_ms: None,
            half_life_days: class.default_half_life_days(),
        }
    }

    /// Exponential freshness decay: $2^{-\max(0, t_{\text{eval}} - t_{\text{phenomenon}}) / t_{1/2}}$.
    pub fn compute_decayed_confidence(&self, eval_time_ms: i64) -> f64 {
        let dt_ms = eval_time_ms.saturating_sub(self.phenomenon_time_ms).max(0) as f64;
        if dt_ms == 0.0 {
            return 1.0;
        }
        match self.half_life_days {
            None => 1.0,
            Some(h) if !h.is_finite() || h <= 0.0 => 0.0,
            Some(h) => 2.0_f64.powf(-dt_ms / (h * 86_400_000.0)),
        }
    }

    /// Scales `base_confidence` (clamped to `0.0..=1.0`) by the exponential decay factor.
    pub fn compute_decayed_confidence_from_base(
        &self,
        base_confidence: f64,
        eval_time_ms: i64,
    ) -> f64 {
        base_confidence.clamp(0.0, 1.0) * self.compute_decayed_confidence(eval_time_ms)
    }
}

/// Asserted value. Numeric values carry a unit IRI (for example a QUDT unit).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EvidenceValue {
    Quantity { value: f64, unit_iri: String },
    Range { min: f64, max: f64, unit_iri: String },
    Text { text: String, language: Option<String> },
    Iri { iri: String },
}

/// Statistical uncertainty or sample description for a value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Uncertainty {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard_deviation: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<(f64, f64)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_size: Option<u32>,
}

/// What the assertion is about biologically. Type-level facts leave `individual_iri` empty;
/// observations of particular organisms set it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BiologicalScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taxon_concept_iri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_iri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub life_stage_iri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sex_iri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub population_iri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub individual_iri: Option<String>,
}

/// Content of a measured or described fact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EvidenceDetail {
    pub property_iri: String,
    pub basis: ClaimBasis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<EvidenceValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty: Option<Uncertainty>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method_iri: Option<String>,
    /// ISO 8601 instant or interval of observation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<String>,
    /// ISO 8601 interval during which the assertion is held valid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_during: Option<String>,
    /// ISO 8601 start of validity interval (`q42:validFrom`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<String>,
    /// ISO 8601 end of validity interval (`q42:validUntil`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<String>,
    #[serde(default)]
    pub scope: BiologicalScope,
    /// Identifier of the record within the root source (for example a trait observation id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_record_id: Option<String>,
    /// Region of a media item supporting the assertion (for example an Audiovisual Core ROI).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_region: Option<String>,
    /// Seven-tier epistemic provenance state (`q42:epistemicTier`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epistemic_tier: Option<EpistemicTier>,
    /// Normative `q42m:*` observation method (`q42:observationMethod`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation_method: Option<ObservationMethod>,
    /// Role of the observer (`q42:observerRole`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observer_role: Option<ObserverRole>,
    /// Four-axis temporal provenance and half-life decay parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporal_axis: Option<TemporalObservationAxis>,
    /// Biological freshness stability class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stability_class: Option<TemporalStabilityClass>,
    /// Canonical IRI/hash of the transfer formula for `InferredFromObservation`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference_formula_iri: Option<String>,
    /// Upstream observation assertion IDs for `InferredFromObservation`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derived_from_observation_ids: Vec<String>,
}

const INLINE_TAG_DECIMAL: u64 = 0b010 << 60;
const PAYLOAD_60: u64 = 0x0FFF_FFFF_FFFF_FFFF;

pub const P_PROPERTY: &str = "https://ns.webizen.org/evidence#property";
pub const P_BASIS: &str = "https://ns.webizen.org/evidence#basis";
pub const P_VALUE: &str = "https://ns.webizen.org/evidence#value";
pub const P_VALUE_MIN: &str = "https://ns.webizen.org/evidence#valueMin";
pub const P_VALUE_MAX: &str = "https://ns.webizen.org/evidence#valueMax";
pub const P_UNIT: &str = "https://ns.webizen.org/evidence#unit";
pub const P_METHOD: &str = "https://ns.webizen.org/evidence#method";
pub const P_TAXON: &str = "https://ns.webizen.org/evidence#taxonConcept";
pub const P_PART: &str = "https://ns.webizen.org/evidence#part";
pub const P_LIFE_STAGE: &str = "https://ns.webizen.org/evidence#lifeStage";
pub const P_SEX: &str = "https://ns.webizen.org/evidence#sex";
pub const P_POPULATION: &str = "https://ns.webizen.org/evidence#population";
pub const P_INDIVIDUAL: &str = "https://ns.webizen.org/evidence#individual";
pub const P_SOURCE_RECORD: &str = "https://ns.webizen.org/evidence#sourceRecord";

pub const P_OBSERVED_AT: &str = "q42:observedAt";
pub const P_VALID_FROM: &str = "q42:validFrom";
pub const P_VALID_UNTIL: &str = "q42:validUntil";
pub const P_VALID_DURING: &str = "q42:validDuring";
pub const P_OBSERVATION_METHOD: &str = "q42:observationMethod";
pub const P_EPISTEMIC_TIER: &str = "q42:epistemicTier";
pub const P_OBSERVER_ROLE: &str = "q42:observerRole";
pub const P_UNCERTAINTY_VALUE: &str = "q42:uncertaintyValue";
pub const P_UNCERTAINTY_UNIT: &str = "q42:uncertaintyUnit";
pub const P_UNCERTAINTY_CONFIDENCE: &str = "q42:uncertaintyConfidence";
pub const P_MEDIA_REGION: &str = "q42:mediaRegion";
pub const P_PHENOMENON_TIME_MS: &str = "q42:phenomenonTimeMs";
pub const P_ACQUISITION_TIME_MS: &str = "q42:acquisitionTimeMs";
pub const P_EXTRACTION_TIME_MS: &str = "q42:extractionTimeMs";
pub const P_CALIBRATION_TIME_MS: &str = "q42:calibrationTimeMs";
pub const P_INFERENCE_FORMULA: &str = "q42:inferenceFormula";

/// Encodes a number as an inline `xsd:decimal` object (value x 10^6) when it is finite,
/// non-negative and fits 60 bits; otherwise hashes its canonical lexical form.
pub fn encode_decimal_object(v: f64) -> u64 {
    let scaled = v * 1_000_000.0;
    if v.is_finite() && v >= 0.0 && scaled <= PAYLOAD_60 as f64 {
        INLINE_TAG_DECIMAL | (scaled.round() as u64 & PAYLOAD_60)
    } else {
        q_hash_str(&format!("{v:?}^^xsd:decimal"))
    }
}

impl EvidenceDetail {
    /// Validates internal consistency. Numeric values must be finite, ranges ordered, and a
    /// `Measured` assertion must name its method.
    pub fn validate(&self) -> Result<(), String> {
        if self.property_iri.trim().is_empty() {
            return Err("evidence property_iri is empty".into());
        }
        match &self.value {
            Some(EvidenceValue::Quantity { value, unit_iri }) => {
                if !value.is_finite() {
                    return Err("evidence quantity is not finite".into());
                }
                if unit_iri.trim().is_empty() {
                    return Err("evidence quantity requires unit_iri".into());
                }
            }
            Some(EvidenceValue::Range { min, max, unit_iri }) => {
                if !min.is_finite() || !max.is_finite() || min > max {
                    return Err("evidence range must be finite with min <= max".into());
                }
                if unit_iri.trim().is_empty() {
                    return Err("evidence range requires unit_iri".into());
                }
            }
            Some(EvidenceValue::Text { text, .. }) if text.trim().is_empty() => {
                return Err("evidence text is empty".into());
            }
            Some(EvidenceValue::Iri { iri }) if iri.trim().is_empty() => {
                return Err("evidence iri is empty".into());
            }
            _ => {}
        }
        if self.basis == ClaimBasis::Unknown && self.value.is_some() {
            return Err("an unknown-basis assertion must not carry a value".into());
        }
        if self.basis == ClaimBasis::Measured
            && self.method_iri.is_none()
            && self.observation_method.is_none()
        {
            return Err("a measured assertion requires method_iri or observation_method".into());
        }
        if let Some(u) = &self.uncertainty {
            if let Some(sd) = u.standard_deviation {
                if !sd.is_finite() || sd < 0.0 {
                    return Err("uncertainty standard_deviation must be finite and >= 0".into());
                }
            }
            if let Some((lo, hi)) = u.interval {
                if !lo.is_finite() || !hi.is_finite() || lo > hi {
                    return Err("uncertainty interval must be finite with lo <= hi".into());
                }
            }
            if let Some(c) = u.interval_confidence {
                if !(0.0..=1.0).contains(&c) {
                    return Err("interval_confidence must be in 0.0..=1.0".into());
                }
            }
        }
        if let Some(axis) = &self.temporal_axis {
            if let Some(h) = axis.half_life_days {
                if !h.is_finite() || h <= 0.0 {
                    return Err("temporal_axis half_life_days must be finite and > 0".into());
                }
            }
        }
        Ok(())
    }

    /// Appends evidence quins for subject hash `s` into `out[start..]`; returns the new length.
    pub fn compile_to_quins(&self, s: u64, ctx: u64, meta: u64, out: &mut [NQuin], start: usize) -> usize {
        let mut n = start;
        let mut push = |p: &str, o: u64, n: &mut usize| {
            if *n < out.len() {
                out[*n] = NQuin::new(s, q_hash_str(p), o, ctx, meta);
                *n += 1;
            }
        };
        push(P_PROPERTY, q_hash_str(&self.property_iri), &mut n);
        push(P_BASIS, q_hash_str(self.basis.as_str()), &mut n);
        let value_unit_iri = match &self.value {
            Some(EvidenceValue::Quantity { value, unit_iri }) => {
                push(P_VALUE, encode_decimal_object(*value), &mut n);
                push(P_UNIT, q_hash_str(unit_iri), &mut n);
                Some(unit_iri.as_str())
            }
            Some(EvidenceValue::Range { min, max, unit_iri }) => {
                push(P_VALUE_MIN, encode_decimal_object(*min), &mut n);
                push(P_VALUE_MAX, encode_decimal_object(*max), &mut n);
                push(P_UNIT, q_hash_str(unit_iri), &mut n);
                Some(unit_iri.as_str())
            }
            Some(EvidenceValue::Text { text, .. }) => {
                push(P_VALUE, q_hash_str(text), &mut n);
                None
            }
            Some(EvidenceValue::Iri { iri }) => {
                push(P_VALUE, q_hash_str(iri), &mut n);
                None
            }
            None => None,
        };
        let optional = [
            (P_METHOD, &self.method_iri),
            (P_TAXON, &self.scope.taxon_concept_iri),
            (P_PART, &self.scope.part_iri),
            (P_LIFE_STAGE, &self.scope.life_stage_iri),
            (P_SEX, &self.scope.sex_iri),
            (P_POPULATION, &self.scope.population_iri),
            (P_INDIVIDUAL, &self.scope.individual_iri),
            (P_SOURCE_RECORD, &self.source_record_id),
            (P_OBSERVED_AT, &self.observed_at),
            (P_VALID_FROM, &self.valid_from),
            (P_VALID_UNTIL, &self.valid_until),
            (P_VALID_DURING, &self.valid_during),
            (P_MEDIA_REGION, &self.media_region),
            (P_INFERENCE_FORMULA, &self.inference_formula_iri),
        ];
        for (p, v) in optional {
            if let Some(v) = v {
                push(p, q_hash_str(v), &mut n);
            }
        }
        if let Some(tier) = self.epistemic_tier {
            push(P_EPISTEMIC_TIER, q_hash_str(tier.as_iri()), &mut n);
        }
        if let Some(method) = self.observation_method {
            push(P_OBSERVATION_METHOD, q_hash_str(method.as_curie()), &mut n);
        }
        if let Some(role) = self.observer_role {
            push(P_OBSERVER_ROLE, q_hash_str(role.as_iri()), &mut n);
        }
        if let Some(u) = &self.uncertainty {
            if let Some(sd) = u.standard_deviation {
                push(P_UNCERTAINTY_VALUE, encode_decimal_object(sd), &mut n);
                if let Some(unit_iri) = value_unit_iri {
                    push(P_UNCERTAINTY_UNIT, q_hash_str(unit_iri), &mut n);
                }
            }
            if let Some(c) = u.interval_confidence {
                push(P_UNCERTAINTY_CONFIDENCE, encode_decimal_object(c), &mut n);
            }
        }
        if let Some(axis) = &self.temporal_axis {
            push(P_PHENOMENON_TIME_MS, q_hash_str(&axis.phenomenon_time_ms.to_string()), &mut n);
            if let Some(t) = axis.acquisition_time_ms {
                push(P_ACQUISITION_TIME_MS, q_hash_str(&t.to_string()), &mut n);
            }
            if let Some(t) = axis.extraction_time_ms {
                push(P_EXTRACTION_TIME_MS, q_hash_str(&t.to_string()), &mut n);
            }
            if let Some(t) = axis.calibration_time_ms {
                push(P_CALIBRATION_TIME_MS, q_hash_str(&t.to_string()), &mut n);
            }
        }
        n
    }
}

/// A contextual assertion together with the content of the fact it asserts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidencedAssertion {
    pub assertion: crate::contextual_assertion::ContextualAssertion,
    pub evidence: EvidenceDetail,
}

impl EvidencedAssertion {
    /// Validates the evidence and its consistency with the assertion's statement type.
    pub fn validate(&self) -> Result<(), String> {
        use crate::contextual_assertion::StatementType;
        let a = &self.assertion;
        if a.id.trim().is_empty() || a.subject_uri.trim().is_empty() {
            return Err("assertion id and subject_uri are required".into());
        }
        self.evidence.validate()?;
        match self.evidence.basis {
            ClaimBasis::Measured | ClaimBasis::Described if a.root_source_id.trim().is_empty() => {
                Err("measured or described evidence requires root_source_id".into())
            }
            ClaimBasis::ModelAssumption | ClaimBasis::ArtisticChoice
                if matches!(
                    a.statement_type,
                    StatementType::Observation | StatementType::AdjudicatedFinding
                ) =>
            {
                Err("a model assumption or artistic choice cannot be an observation or finding".into())
            }
            ClaimBasis::Derived
                if a.statement_type != StatementType::Inference
                    && a.derived_from_assertion_ids.is_empty() =>
            {
                Err("derived evidence must be an inference or name derived_from_assertion_ids".into())
            }
            _ => Ok(()),
        }
    }

    /// Writes the assertion quins followed by the evidence quins; returns the count written.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        let a = &self.assertion;
        let n = a.compile_to_quins(out);
        let s = q_hash_str(&a.id);
        let ctx = q_hash_str(&a.scope_domain);
        let meta = a.sensitivity.to_metadata_mask() | (a.asserted_at_unix as u64);
        self.evidence.compile_to_quins(s, ctx, meta, out, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contextual_assertion::{
        AssertionStatus, ContextualAssertion, StatementType, VerificationStatus,
    };
    use crate::record::SensitivityClass;

    fn assertion(st: StatementType, root: &str) -> ContextualAssertion {
        ContextualAssertion {
            id: "ea-1".into(),
            subject_uri: "https://biodiversity.org.au/afd/taxa/Wallabia_bicolor".into(),
            statement_type: st,
            assertion_status: AssertionStatus::Active,
            verification_status: VerificationStatus::Proposed,
            claim_summary: "head-body length".into(),
            author_did: "did:example:curator".into(),
            root_source_id: root.into(),
            derived_from_assertion_ids: Vec::new(),
            scope_domain: "species:wallabia-bicolor".into(),
            sensitivity: SensitivityClass::Public,
            asserted_at_unix: 1_700_000_000,
            expires_at_unix: None,
            superseded_by_id: None,
            adjudicated_by_human_did: None,
        }
    }

    fn measured() -> EvidenceDetail {
        EvidenceDetail {
            property_iri: "http://purl.obolibrary.org/obo/VT_0002257".into(),
            basis: ClaimBasis::Measured,
            value: Some(EvidenceValue::Range {
                min: 0.665,
                max: 0.847,
                unit_iri: "http://qudt.org/vocab/unit/M".into(),
            }),
            uncertainty: None,
            method_iri: Some("urn:example:method:caliper".into()),
            observed_at: None,
            valid_during: None,
            scope: BiologicalScope {
                life_stage_iri: Some("http://purl.obolibrary.org/obo/UBERON_0000113".into()),
                ..Default::default()
            },
            source_record_id: Some("record-42".into()),
            media_region: None,
            ..Default::default()
        }
    }

    #[test]
    fn measured_evidence_validates_and_compiles() {
        let ea = EvidencedAssertion {
            assertion: assertion(StatementType::Observation, "src:example-dataset"),
            evidence: measured(),
        };
        ea.validate().unwrap();
        let mut out = [NQuin::default(); 32];
        let n = ea.compile_to_quins(&mut out);
        // 5 assertion + property, basis, min, max, unit, method, life stage, source record.
        assert_eq!(n, 13);
        let min = out[7];
        assert_eq!(min.predicate, q_hash_str(P_VALUE_MIN));
        assert_eq!(min.object, INLINE_TAG_DECIMAL | 665_000);
        assert!(out[..n].iter().all(|q| q.parity == q.subject ^ q.predicate ^ q.object ^ q.context));
    }

    #[test]
    fn evidence_validation_fails_closed() {
        let mut e = measured();
        e.method_iri = None;
        assert!(e.validate().unwrap_err().contains("method"));

        let mut e = measured();
        e.basis = ClaimBasis::Unknown;
        e.method_iri = None;
        assert!(e.validate().unwrap_err().contains("unknown"));

        let mut e = measured();
        e.value = Some(EvidenceValue::Range { min: 2.0, max: 1.0, unit_iri: "u".into() });
        assert!(e.validate().is_err());

        let mut e = measured();
        e.value = Some(EvidenceValue::Quantity { value: 1.0, unit_iri: String::new() });
        assert!(e.validate().unwrap_err().contains("unit"));
    }

    #[test]
    fn assertion_consistency_is_enforced() {
        let no_source = EvidencedAssertion {
            assertion: assertion(StatementType::Observation, ""),
            evidence: measured(),
        };
        assert!(no_source.validate().unwrap_err().contains("root_source_id"));

        let mut assumption = measured();
        assumption.basis = ClaimBasis::ModelAssumption;
        let bad = EvidencedAssertion {
            assertion: assertion(StatementType::Observation, "src"),
            evidence: assumption.clone(),
        };
        assert!(bad.validate().unwrap_err().contains("model assumption"));
        let ok = EvidencedAssertion {
            assertion: assertion(StatementType::Attestation, "src"),
            evidence: assumption,
        };
        ok.validate().unwrap();

        let mut derived = measured();
        derived.basis = ClaimBasis::Derived;
        let bad = EvidencedAssertion {
            assertion: assertion(StatementType::Observation, "src"),
            evidence: derived,
        };
        assert!(bad.validate().unwrap_err().contains("derived"));
    }

    #[test]
    fn negative_values_fall_back_to_lexical_hash() {
        let o = encode_decimal_object(-0.2);
        assert_eq!(o, q_hash_str("-0.2^^xsd:decimal"));
        assert_eq!(encode_decimal_object(0.95), INLINE_TAG_DECIMAL | 950_000);
    }

    #[test]
    fn epistemic_tiers_and_observer_roles_cover_all_seven_levels() {
        let tiers = [
            EpistemicTier::DirectlyObserved,
            EpistemicTier::InferredFromObservation,
            EpistemicTier::ExtractedFromRecord,
            EpistemicTier::LongitudinallyProjected,
            EpistemicTier::StatisticallyCalibrated,
            EpistemicTier::PhylogeneticallyBracketed,
            EpistemicTier::AuthorialIntent,
        ];
        for (idx, tier) in tiers.iter().enumerate() {
            assert_eq!(tier.rank() as usize, idx);
            assert!(tier.as_iri().starts_with("https://ns.webizen.org/epistemic#"));
            assert!(!tier.as_str().is_empty());
        }
        assert!(EpistemicTier::DirectlyObserved < EpistemicTier::AuthorialIntent);

        let roles = [
            ObserverRole::SelfMeasured,
            ObserverRole::PeerMeasured,
            ObserverRole::ClinicianMeasured,
            ObserverRole::IsakCertified,
            ObserverRole::AutomatedSensor,
            ObserverRole::CuratorExtracted,
        ];
        for role in roles {
            assert!(role.as_iri().starts_with("https://ns.webizen.org/epistemic#"));
            assert!(!role.as_str().is_empty());
        }
    }

    #[test]
    fn observation_methods_cover_all_q42m_taxonomies() {
        let samples = [
            (ObservationMethod::FlexibleTapeCircumference, "q42m:FlexibleTapeCircumference", 1),
            (ObservationMethod::DexaArealDensitometry, "q42m:DEXA_ArealDensitometry", 2),
            (ObservationMethod::ClinicalCtQuantitativeQct, "q42m:ClinicalCT_Quantitative_QCT", 2),
            (ObservationMethod::ClinicalMriDixonPdff, "q42m:ClinicalMRI_DixonPDFF", 2),
            (ObservationMethod::DiagnosticUltrasound3D, "q42m:DiagnosticUltrasound_3D", 2),
            (ObservationMethod::BiplanarFluoroscopyXromm, "q42m:BiplanarFluoroscopy_XROMM", 2),
            (ObservationMethod::TerrestrialLidarTls, "q42m:TerrestrialLiDAR_TLS", 2),
            (ObservationMethod::InertialImuArray, "q42m:InertialIMU_Array", 3),
            (ObservationMethod::IsokineticDynamometry, "q42m:IsokineticDynamometry", 3),
            (ObservationMethod::GaitMatPressureWalkway, "q42m:GaitMat_PressureWalkway", 3),
            (ObservationMethod::ElectronicHealthRecordFhir, "q42m:ElectronicHealthRecord_FHIR", 4),
            (ObservationMethod::IcdCodedDiagnosis, "q42m:ICD_CodedDiagnosis", 4),
            (ObservationMethod::PathologyReport, "q42m:PathologyReport", 4),
            (ObservationMethod::SubjectSelfReportQuestionnaire, "q42m:SubjectSelfReport_Questionnaire", 4),
        ];
        for (method, expected_curie, expected_cat) in samples {
            assert_eq!(method.as_curie(), expected_curie);
            assert_eq!(method.category_code(), expected_cat);
            assert!(method.as_iri().starts_with("https://ns.webizen.org/epistemic/method#"));
        }
    }

    #[test]
    fn temporal_observation_axis_exponential_decay() {
        let day_ms: i64 = 86_400_000;
        let t0: i64 = 1_700_000_000_000;
        let axis = TemporalObservationAxis {
            phenomenon_time_ms: t0,
            acquisition_time_ms: Some(t0 + 5_000),
            extraction_time_ms: Some(t0 + 60_000),
            calibration_time_ms: Some(t0 + 120_000),
            half_life_days: Some(7.0),
        };
        assert!((axis.compute_decayed_confidence(t0) - 1.0).abs() < 1e-12);
        assert!((axis.compute_decayed_confidence(t0 - day_ms) - 1.0).abs() < 1e-12);
        assert!((axis.compute_decayed_confidence(t0 + 7 * day_ms) - 0.5).abs() < 1e-12);
        assert!((axis.compute_decayed_confidence(t0 + 14 * day_ms) - 0.25).abs() < 1e-12);
        assert!((axis.compute_decayed_confidence_from_base(0.8, t0 + 7 * day_ms) - 0.4).abs() < 1e-12);

        let invariant = TemporalObservationAxis::with_stability_class(
            t0,
            TemporalStabilityClass::SkeletallyMatureInvariant,
        );
        assert!((invariant.compute_decayed_confidence(t0 + 3650 * day_ms) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn compile_to_quins_emits_all_epistemic_and_temporal_provenance() {
        let t0: i64 = 1_700_000_000_000;
        let detail = EvidenceDetail {
            property_iri: "https://ns.webizen.org/dens#arealBMD".into(),
            basis: ClaimBasis::Measured,
            value: Some(EvidenceValue::Quantity {
                value: 1.045,
                unit_iri: "http://qudt.org/vocab/unit/GM-PER-CentiM2".into(),
            }),
            uncertainty: Some(Uncertainty {
                standard_deviation: Some(0.012),
                interval: Some((1.021, 1.069)),
                interval_confidence: Some(0.95),
                sample_size: Some(1),
            }),
            method_iri: None,
            observed_at: Some("2026-11-15T10:00:00Z".into()),
            valid_during: Some("2026-11-15T10:00:00Z/2027-11-15T10:00:00Z".into()),
            valid_from: Some("2026-11-15T10:00:00Z".into()),
            valid_until: Some("2027-11-15T10:00:00Z".into()),
            scope: BiologicalScope::default(),
            source_record_id: Some("dexa-l1-l4-01".into()),
            media_region: Some("xywh=120,240,80,160".into()),
            epistemic_tier: Some(EpistemicTier::DirectlyObserved),
            observation_method: Some(ObservationMethod::DexaArealDensitometry),
            observer_role: Some(ObserverRole::ClinicianMeasured),
            temporal_axis: Some(TemporalObservationAxis {
                phenomenon_time_ms: t0,
                acquisition_time_ms: Some(t0 + 1_000),
                extraction_time_ms: Some(t0 + 2_000),
                calibration_time_ms: Some(t0 + 3_000),
                half_life_days: Some(365.0),
            }),
            stability_class: Some(TemporalStabilityClass::OntogeneticAndSkeletalRemodeling),
            inference_formula_iri: None,
            derived_from_observation_ids: Vec::new(),
        };
        detail.validate().unwrap();

        let mut out = [NQuin::default(); 32];
        let s = q_hash_str("ea-dexa-1");
        let ctx = q_hash_str("subject:human-1");
        let n = detail.compile_to_quins(s, ctx, 0, &mut out, 0);
        let preds: Vec<u64> = out[..n].iter().map(|q| q.predicate).collect();

        for required in [
            P_OBSERVED_AT,
            P_VALID_FROM,
            P_VALID_UNTIL,
            P_OBSERVATION_METHOD,
            P_EPISTEMIC_TIER,
            P_OBSERVER_ROLE,
            P_UNCERTAINTY_VALUE,
            P_UNCERTAINTY_UNIT,
            P_UNCERTAINTY_CONFIDENCE,
            P_MEDIA_REGION,
            P_PHENOMENON_TIME_MS,
            P_ACQUISITION_TIME_MS,
            P_EXTRACTION_TIME_MS,
            P_CALIBRATION_TIME_MS,
        ] {
            assert!(
                preds.contains(&q_hash_str(required)),
                "missing predicate {required}"
            );
        }
        assert!(out[..n].iter().all(|q| q.parity == q.subject ^ q.predicate ^ q.object ^ q.context));
    }
}
