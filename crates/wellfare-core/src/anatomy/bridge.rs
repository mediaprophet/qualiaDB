//! S4 (bridge) — map a person's **WellFair records → anatomy factors** via the knowledge base.
//!
//! The host layer (client-core) normalizes each health record into a [`RecordRef`] (its id, journal
//! kind, human label, and — for the temporal view — when it happened and a dose scaler). This module,
//! pure and testable, resolves each ref to a [`FactorKnowledge`](super::knowledge::FactorKnowledge)
//! entry by kind + slugified label and instantiates a [`Factor`] or [`FactorEvent`]. Records with no
//! matching knowledge entry are **reported, not silently dropped** — an honest "we don't have a
//! mapping for this yet" (expected until Timothy's corpus lands for meds / diet / herbs).
//!
//! The knowledge lookup is by candidate keys per kind (e.g. a `"diet"` record tries `food:…`,
//! `whole-food:…`, `herb:…`, `tea:…`, `supplement:…`), so one diet log can resolve to a food, a herb,
//! or a tea entry without the host having to know which.

use serde::{Deserialize, Serialize};

use super::factor::Factor;
use super::knowledge::KnowledgeBase;
use super::lens::{build_view, AnatomyView, Lens};
use super::temporal::Timeline;

/// A normalized reference to one health record — the minimum the bridge needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordRef {
    pub id: String,
    /// Journal kind: `"condition"`, `"medication"`, `"diet"`, `"clinical_report"`, etc.
    pub kind: String,
    /// Human label / name / description carried by the record.
    pub label: String,
    /// Timeline position in minutes (for the temporal view); `None` = treat as standing/non-temporal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_minute: Option<i64>,
    /// Dose scaler for the temporal event (percent; `None` = 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dose_scale_pct: Option<u32>,
}

impl RecordRef {
    pub fn new(id: impl Into<String>, kind: impl Into<String>, label: impl Into<String>) -> Self {
        Self { id: id.into(), kind: kind.into(), label: label.into(), at_minute: None, dose_scale_pct: None }
    }

    pub fn at(mut self, at_minute: i64) -> Self {
        self.at_minute = Some(at_minute);
        self
    }

    pub fn dosed(mut self, dose_scale_pct: u32) -> Self {
        self.dose_scale_pct = Some(dose_scale_pct);
        self
    }
}

/// Candidate knowledge-key prefixes to try for a given journal kind, most specific first.
fn key_prefixes(kind: &str) -> &'static [&'static str] {
    match kind {
        "condition" | "disputed_diagnosis" => &["cond"],
        "medication" | "med_administration" => &["med"],
        // A diet log could be any ingestible — try the food/botanical/supplement families.
        "diet" | "food" | "nutrition" => &["food", "whole-food", "herb", "tea", "supplement", "nutrient"],
        "herb" => &["herb"],
        "tea" => &["tea"],
        "supplement" => &["supplement"],
        "nutrient" => &["nutrient"],
        _ => &[],
    }
}

/// The knowledge key candidates for a record (kind-appropriate prefixes × slugified label).
pub fn knowledge_key_candidates(kind: &str, label: &str) -> Vec<String> {
    let slug = super::slugify(label);
    if slug.is_empty() {
        return Vec::new();
    }
    key_prefixes(kind).iter().map(|p| format!("{p}:{slug}")).collect()
}

/// The outcome of mapping records to factors: what resolved, and what didn't (honestly surfaced).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeResult {
    pub factors: Vec<Factor>,
    /// `(kind, label)` of records with no knowledge mapping — shown as "not mapped yet", not dropped.
    pub unmapped: Vec<(String, String)>,
}

/// Map records to non-temporal [`Factor`]s via the knowledge base. Each factor keeps the record's id
/// (so a view can trace back to the source record).
pub fn records_to_factors(records: &[RecordRef], kb: &KnowledgeBase) -> BridgeResult {
    let mut result = BridgeResult::default();
    for r in records {
        match resolve(kb, &r.kind, &r.label) {
            Some(entry) => result.factors.push(entry.to_factor(r.id.clone())),
            None => result.unmapped.push((r.kind.clone(), r.label.clone())),
        }
    }
    result
}

/// Map records to a temporal [`Timeline`] via the knowledge base (uses each record's `at_minute` /
/// `dose_scale_pct`, defaulting to 0 / 100). Unmapped records are returned alongside.
pub fn records_to_timeline(records: &[RecordRef], kb: &KnowledgeBase) -> (Timeline, Vec<(String, String)>) {
    let mut tl = Timeline::new();
    let mut unmapped = Vec::new();
    for r in records {
        match resolve(kb, &r.kind, &r.label) {
            Some(entry) => {
                let ev = entry.to_event(
                    r.id.clone(),
                    r.at_minute.unwrap_or(0),
                    r.dose_scale_pct.unwrap_or(100),
                );
                tl = tl.with_event(ev);
            }
            None => unmapped.push((r.kind.clone(), r.label.clone())),
        }
    }
    (tl, unmapped)
}

/// One-shot: records → factors → an [`AnatomyView`] for a lens. Returns the view plus the unmapped set.
pub fn build_view_from_records(
    records: &[RecordRef],
    kb: &KnowledgeBase,
    lens: Lens,
    convergence_threshold: usize,
) -> (AnatomyView, Vec<(String, String)>) {
    let bridge = records_to_factors(records, kb);
    let view = build_view(&bridge.factors, lens, convergence_threshold);
    (view, bridge.unmapped)
}

fn resolve<'a>(
    kb: &'a KnowledgeBase,
    kind: &str,
    label: &str,
) -> Option<&'a super::knowledge::FactorKnowledge> {
    knowledge_key_candidates(kind, label).into_iter().find_map(|k| kb.get(&k))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::seed_knowledge_base;

    #[test]
    fn candidate_keys_are_kind_appropriate() {
        assert_eq!(knowledge_key_candidates("condition", "Type 2 Diabetes"), vec!["cond:type-2-diabetes"]);
        assert_eq!(knowledge_key_candidates("medication", "Warfarin"), vec!["med:warfarin"]);
        // A diet log fans out across ingestible families.
        let diet = knowledge_key_candidates("diet", "Chamomile");
        assert!(diet.contains(&"tea:chamomile".to_string()));
        assert!(diet.contains(&"food:chamomile".to_string()));
        // An empty/punctuation-only label yields no candidates.
        assert!(knowledge_key_candidates("diet", "  ").is_empty());
    }

    #[test]
    fn records_resolve_against_the_seed_and_report_unmapped() {
        let kb = seed_knowledge_base();
        let records = vec![
            RecordRef::new("rec:beer-1", "diet", "Beer"),
            RecordRef::new("rec:mt-1", "diet", "Milk thistle"),
            RecordRef::new("rec:unknown", "medication", "Some Med We Lack"),
        ];
        let res = records_to_factors(&records, &kb);
        // Beer (food:beer) and milk thistle (herb:milk-thistle) resolve; the unknown med does not.
        assert_eq!(res.factors.len(), 2);
        assert!(res.factors.iter().any(|f| f.id == "rec:beer-1"));
        assert!(res.factors.iter().any(|f| f.id == "rec:mt-1"));
        assert_eq!(res.unmapped, vec![("medication".to_string(), "Some Med We Lack".to_string())]);
    }

    #[test]
    fn beer_label_resolves_via_slug_match() {
        // The seed key is "food:beer" with label "Beer (alcohol)" → slug "beer-alcohol" would NOT match.
        // The bridge slugs the *record* label, so the record must be "Beer" to hit "food:beer".
        let kb = seed_knowledge_base();
        let hit = records_to_factors(&[RecordRef::new("r", "diet", "Beer")], &kb);
        assert_eq!(hit.factors.len(), 1, "record label 'Beer' → food:beer");
        let miss = records_to_factors(&[RecordRef::new("r", "diet", "Beer (alcohol)")], &kb);
        assert_eq!(miss.factors.len(), 0, "the fuller label slugs differently — honestly unmapped");
        assert_eq!(miss.unmapped.len(), 1);
    }

    #[test]
    fn temporal_bridge_builds_a_timeline_that_loads_systems() {
        let kb = seed_knowledge_base();
        let records = vec![
            RecordRef::new("r:beer", "diet", "Beer").at(0).dosed(300),
            RecordRef::new("r:water", "diet", "Water + electrolytes").at(120),
        ];
        let (tl, unmapped) = records_to_timeline(&records, &kb);
        assert!(unmapped.is_empty());
        let dig = tl.burden_at(60).into_iter().find(|b| b.system_id == "digestive").unwrap();
        assert!(dig.net_milli > 0);
    }

    #[test]
    fn end_to_end_person_view_from_records() {
        let kb = seed_knowledge_base();
        let records = vec![RecordRef::new("r:beer", "diet", "Beer")];
        let (view, unmapped) = build_view_from_records(&records, &kb, Lens::Person, 1);
        assert!(unmapped.is_empty());
        // Beer loads digestive + urinary; the person view lists them plainly.
        assert!(view.systems.iter().any(|s| s.system_id == "digestive"));
        assert!(view.boundary.contains("not medical advice"));
    }
}
