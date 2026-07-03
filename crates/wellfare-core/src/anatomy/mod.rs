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
mod bridge;
mod factor;
mod knowledge;
mod lens;
mod systems;
mod temporal;

pub use accumulate::{
    accumulate, interactions, systemic_implications, Interaction, InteractionKind, SystemBurden,
    SystemicImplication,
};
pub use bridge::{
    build_view_from_records, knowledge_key_candidates, records_to_factors, records_to_timeline,
    BridgeResult, RecordRef,
};
pub use factor::{Effect, EvidenceTier, Factor, FactorKind, FactorTarget};
pub use lens::{build_view, AnatomyView, Lens, SystemView, WellbeingLevel};
pub use knowledge::{
    import_condition_map, import_entries, seed_knowledge_base, FactorKnowledge, ImportResult,
    KnowledgeBase, KnowledgeSource, KnowledgeTarget, Provenance,
};
pub use systems::{body_system, body_system_by_label, BodySystem, BODY_SYSTEMS};
pub use temporal::{
    recovery_band, EnvironmentModulator, FactorEvent, Kinetics, RecoveryBand, Timeline,
    TrajectoryPoint,
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
/// Shared by the knowledge importer and the record→factor bridge so keys line up.
pub(crate) fn slugify(s: &str) -> String {
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
