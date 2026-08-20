//! 3D Anatomy Qapp — **factor / body-system model** (the domain core both audience lenses build on).
//!
//! A [`Factor`] — any of {pathology finding, condition, medication, food, herb, tea, nutrient,
//! supplement, lifestyle, environmental} — maps onto one or more **body systems** with an [`Effect`]
//! (adverse / supportive / modulating), an [`EvidenceTier`], and a magnitude. From a person's active
//! factors, [`accumulate`] rolls per-system burden, [`interactions`] finds compounding / opposing /
//! herb–drug pairs, and [`systemic_implications`] emits **proposals** — never diagnoses.
//!
//! The [`temporal`] layer turns static factors into time-stamped [`FactorEvent`]s with **kinetics**
//! (onset → clearance), [`EnvironmentModulator`]s, and per-system recovery trajectories — so the view
//! can show that *different subsystems recover on different clocks and respond to different
//! interventions* (the hot-week / beer / water example).
//!
//! **Honesty boundaries baked in:** every emitted [`SystemicImplication`] carries
//! [`EpistemicStatus::Hypothesis`] and the dominant evidence tier of its contributors; community /
//! anecdotal claims sit at the lowest tier; temporal projection is coarse ([`RecoveryBand`] "hours vs
//! days", never a BAC or a fitness-to-operate claim); no advice. The 17 systems mirror
//! `bundled/qapps/Anatomy/Knowledge/system-map.json` so the native 3D view and this engine agree on
//! identity.

mod accumulate;
mod birth;
mod bridge;
mod constitution;
mod coverage;
mod dyad;
mod factor;
mod knowledge;
mod knowledge_context;
mod lens;
mod measurements;
mod model;
mod observations;
mod pathway;
mod physiology;
mod registry;
mod scorecard;
mod systems;
mod temporal;

pub use accumulate::{
    Interaction, InteractionKind, SystemBurden, SystemicImplication, accumulate, interactions,
    systemic_implications,
};
pub use birth::{
    AgencyStage, BiometricClass, BirthRecordInvalid, CredentialRef, DigitalBirthRecord,
    Guardianship, GuardianshipCredential, Steward, StewardBasis, StewardRole,
};
pub use bridge::{
    BridgeResult, RecordRef, build_view_from_records, knowledge_key_candidates, records_to_factors,
    records_to_timeline,
};
pub use constitution::{
    AbsenceReason, AbsentPart, BodyAttributes, BodyCharacteristics, BodyConstitution, BodyFit,
    BodyMeasurements, DominantSide, ReferenceGirths, VH_FEMALE_STATURE_MM, VH_MALE_STATURE_MM,
};
pub use coverage::{SystemCoverage, seed_system_coverage, seed_system_coverage_markdown};
pub use dyad::{
    ConsiderationKind, DyadConsideration, DyadInvalid, EmergingChild, InterfaceKind, MaternalBody,
    MaternalFetalDyad, Parentage, PrincipalRef, Progenitor, RightsStage, SocialRightsThreshold,
};
pub use factor::{Effect, EvidenceTier, Factor, FactorKind, FactorTarget};
pub use knowledge::{
    FactorKnowledge, ImportResult, KnowledgeBase, KnowledgeSource, KnowledgeTarget, Provenance,
    import_condition_map, import_entries, seed_knowledge_base,
};
pub use knowledge_context::{
    AncestryInvalid, AncestryRecord, ContextEdgeKind, EthnicityAffiliation, KnowledgeConsideration,
    KnowledgeContextEdge, SubjectKnowledgeContext, affiliations_from_observations,
    considerations_for_context, illustrative_context_pack,
};
pub use lens::{AnatomyView, Lens, SystemView, WellbeingLevel, build_view, burden_to_sigma};
pub use measurements::{
    CIRC_MM_RANGE, MEASUREMENT_CATALOG, MeasurementGroup, MeasurementInputUnit, MeasurementSpec,
    MeasurementUse, STATURE_MM_RANGE, WEIGHT_G_RANGE, measurement_catalog_json,
};
pub use model::{
    AnatomyModel, Karyotype, SystemRepresentation, body_system_for_organ, normalize_organ_key,
    overlay_host_systems, system_memberships_for_organ, system_representation,
};
pub use observations::{
    BodyObservation, InstrumentKind, KNOWN_OBSERVATION_CODES, KnownObservationCode,
    RepresentationBind, bind_for_code, is_known_code,
};
pub use pathway::{
    Hypothesis, InvestigativePathway, InvestigativeStep, RankedStep, StepKind,
    hypotheses_from_implications, investigative_pathway, value_of_information,
};
pub use physiology::{
    CyclePhase, EngagementLevel, PhysiologicalState, ReproductiveState, StateModulator,
    SystemEngagement, Trimester, state_modulator, whole_body_profile,
};
pub use registry::{
    NEUTRAL_SYSTEM_RGBA, SystemDef, SystemProvenance, SystemRegistry, SystemRelation,
    SystemRelationKind, SystemTier, default_registry,
};
pub use scorecard::{
    Aspect, AspectScore, Contribution, ContributionKind, ForumClass, ScoreBand, ScoreCard,
    SystemAspectWeight, WeightModel, score_card, seed_weight_model,
};
pub use systems::{BODY_SYSTEMS, BodySystem, body_system, body_system_by_label};
pub use temporal::{
    EnvironmentModulator, FactorEvent, Kinetics, RecoveryBand, Timeline, TrajectoryPoint,
    recovery_band,
};

/// Canonical key for a system id (trims surrounding whitespace; identity otherwise). Shared so the
/// accumulation and temporal layers group systems identically.
pub(crate) fn system_key(id: &str) -> &str {
    id.trim()
}

/// Push `id` into `v` iff absent (preserves first-seen order without a set allocation).
pub(crate) fn push_unique(v: &mut Vec<String>, id: &str) {
    if !v.iter().any(|x| x == id) {
        v.push(id.to_string());
    }
}

/// Lower-kebab a human label for use in a knowledge key (`"Type 2 Diabetes"` → `"type-2-diabetes"`).
/// Shared by the knowledge importer, ethnicity tokens, and the record→factor bridge so keys line up.
pub fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}
