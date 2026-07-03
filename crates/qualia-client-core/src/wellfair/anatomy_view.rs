//! S4b (host) — turn a person's WellFair records into an [`AnatomyViewReport`] for the Anatomy Qapp.
//!
//! Reads the person's condition / medication / diet journal entries, normalizes each to a
//! [`RecordRef`] (extracting the human label from the entry's `summary` JSON projection), maps them to
//! factors through the anatomy knowledge base, and returns a lens-shaped view plus the lens-independent
//! per-system burden (which colours the 3D body in S5) and an honest account of what did **not** map.
//!
//! The host knowledge base is: the **bundled condition→system reference** (embedded via `include_str!`
//! so conditions map regardless of the runtime file layout — offline, no fetch) **plus** the
//! illustrative seed for food / herb / tea (pending Timothy's curated corpus). The `disclosure` field
//! says exactly that, so the UI never passes seed data off as authoritative.

use serde::{Deserialize, Serialize};

use wellfare_core::anatomy::{
    self, AnatomyView, KnowledgeBase, Lens, Provenance, RecordRef, SystemBurden,
};

use super::journal::JournalEntry;

/// The bundled condition→primary-system map, embedded at compile time (offline, layout-independent).
const BUNDLED_CONDITION_MAP: &str =
    include_str!("../../../../bundled/qapps/Anatomy/Knowledge/condition-map.json");

const DISCLOSURE: &str = "Conditions map via the bundled condition→system reference. Food, herb, and medication mappings currently use an illustrative seed set pending a curated knowledge corpus — treat those as examples, not authoritative. This is a general picture to explore with a clinician, not a diagnosis.";

/// A record that carried no knowledge mapping — surfaced honestly, never silently dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnmappedRecord {
    pub kind: String,
    pub label: String,
}

/// The full report the host returns for one lens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnatomyViewReport {
    /// The lens-shaped narrative (person wellbeing gist / clinician considerations).
    pub view: AnatomyView,
    /// Per-system burden, lens-independent — drives colour-by-load (S5).
    pub burdens: Vec<SystemBurden>,
    /// Records with no knowledge mapping yet.
    pub unmapped: Vec<UnmappedRecord>,
    /// How many records resolved to a factor.
    pub mapped_count: usize,
    /// How many records were considered in total.
    pub total_records: usize,
    /// Honest note on provenance/limits for the UI to show.
    pub disclosure: String,
}

/// Parse a lens string (`"clinician"` → clinician; anything else → the safe person default).
pub fn parse_lens(s: &str) -> Lens {
    match s.trim().to_ascii_lowercase().as_str() {
        "clinician" => Lens::Clinician,
        _ => Lens::Person,
    }
}

/// Build the host knowledge base: bundled conditions + the illustrative seed.
pub fn host_knowledge_base() -> KnowledgeBase {
    let mut kb = anatomy::seed_knowledge_base();
    let prov = Provenance {
        source_id: "clinical-reference".to_string(),
        source_title: "Bundled condition→system reference map".to_string(),
        citation: None,
        imported_at: None,
    };
    if let Ok(res) = anatomy::import_condition_map(BUNDLED_CONDITION_MAP, prov) {
        for entry in res.entries {
            kb.insert(entry);
        }
    }
    kb
}

/// Normalize condition / medication / diet journal entries into [`RecordRef`]s (ceased medications are
/// skipped — they are not a current factor).
pub fn record_refs_from_journal(
    conditions: &[JournalEntry],
    medications: &[JournalEntry],
    diet: &[JournalEntry],
) -> Vec<RecordRef> {
    let mut refs = Vec::new();
    for e in conditions {
        if let Some(label) = summary_str(e, "label") {
            refs.push(RecordRef::new(e.id.clone(), "condition", label));
        }
    }
    for e in medications {
        if summary_bool(e, "ceased") == Some(true) {
            continue;
        }
        if let Some(name) = summary_str(e, "name") {
            refs.push(RecordRef::new(e.id.clone(), "medication", name));
        }
    }
    for e in diet {
        if let Some(desc) = summary_str(e, "description") {
            refs.push(RecordRef::new(e.id.clone(), "diet", desc));
        }
    }
    refs
}

/// Build a report from already-normalized record refs and a lens.
pub fn build_report(records: Vec<RecordRef>, lens: Lens, convergence_threshold: usize) -> AnatomyViewReport {
    let total_records = records.len();
    let kb = host_knowledge_base();
    let bridge = anatomy::records_to_factors(&records, &kb);
    let burdens = anatomy::accumulate(&bridge.factors);
    let view = anatomy::build_view(&bridge.factors, lens, convergence_threshold);
    AnatomyViewReport {
        view,
        burdens,
        unmapped: bridge
            .unmapped
            .into_iter()
            .map(|(kind, label)| UnmappedRecord { kind, label })
            .collect(),
        mapped_count: bridge.factors.len(),
        total_records,
        disclosure: DISCLOSURE.to_string(),
    }
}

/// One-shot: journal entries → report for a lens.
pub fn build_report_from_journal(
    conditions: &[JournalEntry],
    medications: &[JournalEntry],
    diet: &[JournalEntry],
    lens: Lens,
    convergence_threshold: usize,
) -> AnatomyViewReport {
    let refs = record_refs_from_journal(conditions, medications, diet);
    build_report(refs, lens, convergence_threshold)
}

fn summary_value(entry: &JournalEntry) -> Option<serde_json::Value> {
    serde_json::from_str(entry.summary.as_ref()?).ok()
}

fn summary_str(entry: &JournalEntry, field: &str) -> Option<String> {
    summary_value(entry)?.get(field)?.as_str().map(|s| s.to_string())
}

fn summary_bool(entry: &JournalEntry, field: &str) -> Option<bool> {
    summary_value(entry)?.get(field)?.as_bool()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn je(id: &str, kind: &str, summary: serde_json::Value) -> JournalEntry {
        JournalEntry {
            id: id.to_string(),
            kind: kind.to_string(),
            asserted_time_unix: 0,
            evidence_type: "SelfReported".to_string(),
            sensitivity: "Restricted".to_string(),
            blob_hash: None,
            source: "test".to_string(),
            committed_unix: 0,
            summary: Some(summary.to_string()),
        }
    }

    #[test]
    fn bundled_condition_map_is_embedded_and_parses() {
        let kb = host_knowledge_base();
        // A well-known bundled condition resolves to its primary system.
        assert!(kb.get("cond:hypertension").is_some());
        assert_eq!(kb.get("cond:hypertension").unwrap().targets[0].system_id, "circulatory");
        // Integrity holds across the whole assembled base.
        assert!(kb.verify_integrity().is_empty());
    }

    #[test]
    fn real_conditions_map_and_unknown_records_are_reported() {
        let conditions = vec![
            je("did:wf:me:condition:1", "condition", serde_json::json!({"label": "Hypertension"})),
            je("did:wf:me:condition:2", "condition", serde_json::json!({"label": "Made-Up Disease"})),
        ];
        let meds = vec![je(
            "did:wf:me:medication:1",
            "medication",
            serde_json::json!({"name": "Warfarin", "ceased": false}),
        )];
        let diet = vec![je(
            "did:wf:me:diet:1",
            "diet",
            serde_json::json!({"description": "Beer", "meal_type": "drink"}),
        )];

        let report = build_report_from_journal(&conditions, &meds, &diet, Lens::Person, 1);
        // Hypertension → circulatory; Beer → digestive+urinary (seed). Made-Up + Warfarin(no seed) unmapped.
        assert!(report.burdens.iter().any(|b| b.system_id == "circulatory"));
        assert!(report.burdens.iter().any(|b| b.system_id == "digestive"));
        assert!(report.unmapped.iter().any(|u| u.label == "Made-Up Disease"));
        assert!(report.unmapped.iter().any(|u| u.label == "Warfarin"));
        assert_eq!(report.total_records, 4);
        assert!(report.disclosure.contains("illustrative seed"));
        // Person view carries the "not a diagnosis / not advice" boundary.
        assert!(report.view.boundary.contains("not medical advice"));
    }

    #[test]
    fn ceased_medications_are_skipped() {
        let meds = vec![je(
            "did:wf:me:medication:old",
            "medication",
            serde_json::json!({"name": "Warfarin", "ceased": true}),
        )];
        let refs = record_refs_from_journal(&[], &meds, &[]);
        assert!(refs.is_empty(), "a ceased medication is not a current factor");
    }

    #[test]
    fn clinician_lens_flags_the_herb_drug_style_convergence() {
        // Two conditions on the circulatory system converge at threshold 2 → a clinician flag.
        let conditions = vec![
            je("c1", "condition", serde_json::json!({"label": "Hypertension"})),
            je("c2", "condition", serde_json::json!({"label": "Atrial Fibrillation"})),
        ];
        let report = build_report_from_journal(&conditions, &[], &[], Lens::Clinician, 2);
        assert!(report.view.systems.iter().any(|s| s.system_id == "circulatory"));
        assert!(report.view.boundary.contains("not a diagnosis"));
    }

    #[test]
    fn report_serde_round_trips() {
        let report = build_report(vec![], Lens::Person, 2);
        let json = serde_json::to_string(&report).unwrap();
        let back: AnatomyViewReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, back);
    }
}
