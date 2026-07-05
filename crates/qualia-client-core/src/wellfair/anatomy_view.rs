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
    self, body_system_for_organ, burden_to_sigma, overlay_host_systems, system_representation,
    AnatomyView, KnowledgeBase, Lens, Provenance, RecordRef, SystemBurden, SystemRepresentation,
    WellbeingLevel,
};

use qualia_core_db::render::{acoustic, spectral};

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

/// The dual-modality percept for one body system (S5.1 colour-by-load). The accumulated burden is
/// encoded **once** to σ — a position on the shared EMF spectrum — and σ then drives *both* the visual
/// spectrum (`rgba`) and the sonic spectrum (`frequency_hz`) via the engine's parity oracles. So an
/// organ under strain is redder **and** lower-pitched: the 3D body can be seen and heard from one
/// source of truth, rather than a hand-picked swatch that would discard the audio path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemPercept {
    pub system_id: String,
    /// The coarse person-facing band (never a number to the person).
    pub level: WellbeingLevel,
    /// σ — the EMF spectrum position (0..1 over 400–700 nm) this burden encodes to. The one truth.
    pub sigma: f32,
    /// Visual encoding: normalized linear RGBA from `render::spectral`, ready for `upload_mesh_colored`.
    pub rgba: [f32; 4],
    /// Sonic encoding: centre frequency (Hz) from `render::acoustic` — settled/green higher, strain/red lower.
    pub frequency_hz: f32,
}

/// One organ mesh's resolved paint: which body system it belongs to, and the σ-derived dual-modality
/// [`SystemPercept`] (colour + pitch) for that system's current burden. This is what the renderer uses
/// to colour (and can sonify) each organ of the 3D body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganPercept {
    /// The organ key as supplied (e.g. a CCF asset name / file path).
    pub organ_key: String,
    /// The body system this organ belongs to (one of the 17).
    pub system_id: String,
    /// The system's dual-modality percept — settled baseline if the system carries no recorded burden.
    pub percept: SystemPercept,
}

/// A distributed-overlay system's paint (ECS / ENS / glymphatic) — a system with no standalone organ
/// mesh, rendered as a highlight over its host structures. Carries the same σ percept as any system
/// plus the host-system hints for where to place the overlay (empty = a whole-body cue).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayPercept {
    pub system_id: String,
    /// The σ-derived colour + pitch for this network's burden.
    pub percept: SystemPercept,
    /// Discrete systems to highlight this overlay over (empty = whole-body).
    pub host_systems: Vec<String>,
}

impl AnatomyViewReport {
    /// The per-system dual-modality percepts. For each accumulated burden, encode `net_milli` → σ once
    /// (`burden_to_sigma`), then derive the visual colour (`render::spectral`) and the sonic pitch
    /// (`render::acoustic`) from that single σ — the modality-first parity that lets the same anatomy
    /// state be rendered to sight or sound without re-deciding what it "means".
    pub fn system_percepts(&self) -> Vec<SystemPercept> {
        self.burdens
            .iter()
            .map(|b| system_percept(&b.system_id, b.net_milli))
            .collect()
    }

    /// Resolve the dual-modality paint for a set of organ meshes — the organs of the selected anatomy
    /// model (chosen from the user's XY/XX basis via `Karyotype::anatomy_model`; the model's file set is
    /// supplied by the loader). Each organ's body system is looked up (`body_system_for_organ`), then
    /// that system's percept. An organ on a system with **no** recorded burden gets the settled baseline
    /// (calm green / higher pitch) so the whole body still renders; an organ **not** in the curated
    /// organ→system map is returned in the second list — reported, never silently coloured.
    pub fn paint_organs(&self, organ_keys: &[&str]) -> (Vec<OrganPercept>, Vec<String>) {
        let percepts = self.system_percepts();
        let mut painted = Vec::new();
        let mut unmapped = Vec::new();
        for &organ in organ_keys {
            match body_system_for_organ(organ) {
                Some(system_id) => {
                    let percept = percepts
                        .iter()
                        .find(|p| p.system_id == system_id)
                        .cloned()
                        .unwrap_or_else(|| system_percept(system_id, 0));
                    painted.push(OrganPercept {
                        organ_key: organ.to_string(),
                        system_id: system_id.to_string(),
                        percept,
                    });
                }
                None => unmapped.push(organ.to_string()),
            }
        }
        (painted, unmapped)
    }

    /// The distributed-overlay systems' percepts (ECS / ENS / glymphatic) — the systems that have no
    /// standalone organ mesh and so are omitted by [`paint_organs`]. Each is rendered as a highlight
    /// over its host structures (see `host_systems`; empty = a whole-body cue). Together,
    /// `paint_organs` (discrete organs) + `overlay_percepts` (distributed networks) cover the whole
    /// body state — so nothing that carries burden is silently unrepresented.
    pub fn overlay_percepts(&self) -> Vec<OverlayPercept> {
        overlay_percepts_from_burdens(&self.burdens)
    }
}

/// The overlay percepts for the distributed-network systems present in a burden set. Split from the
/// method so it is testable with synthetic burdens.
fn overlay_percepts_from_burdens(burdens: &[SystemBurden]) -> Vec<OverlayPercept> {
    burdens
        .iter()
        .filter(|b| system_representation(&b.system_id) == SystemRepresentation::DistributedOverlay)
        .map(|b| OverlayPercept {
            host_systems: overlay_host_systems(&b.system_id)
                .iter()
                .map(|s| s.to_string())
                .collect(),
            percept: system_percept(&b.system_id, b.net_milli),
            system_id: b.system_id.clone(),
        })
        .collect()
}

/// One system's percept from its burden — the shared σ → {colour, pitch} step.
fn system_percept(system_id: &str, net_milli: u32) -> SystemPercept {
    let sigma = burden_to_sigma(net_milli);
    SystemPercept {
        system_id: system_id.to_string(),
        level: WellbeingLevel::from_net(net_milli),
        sigma,
        rgba: sigma_to_normalized_linear_rgba(sigma),
        frequency_hz: acoustic::sigma_to_center_frequency_hz(sigma),
    }
}

/// σ → normalized linear RGBA for the GPU mesh path. `render::spectral::sigma_to_linear_rgb` returns
/// raw linear sRGB whose luminance varies by hue; we normalize by the peak channel (the same move the
/// display oracle makes before gamma) so every hue reads at full strength as a categorical heat cue,
/// then pin alpha opaque. Linear (not sRGB) because `upload_mesh_colored` expects linear vertex colour.
fn sigma_to_normalized_linear_rgba(sigma: f32) -> [f32; 4] {
    let lin = spectral::sigma_to_linear_rgb(sigma);
    let scale = 1.0 / lin.iter().copied().fold(0.0_f32, f32::max).max(1e-6);
    [
        (lin[0] * scale).clamp(0.0, 1.0),
        (lin[1] * scale).clamp(0.0, 1.0),
        (lin[2] * scale).clamp(0.0, 1.0),
        1.0,
    ]
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
    fn percept_parity_strain_is_redder_and_lower_pitched() {
        // Two burdens through the one σ encoding, then both modalities derived from it.
        let settled_sigma = burden_to_sigma(0);
        let strained_sigma = burden_to_sigma(1000);
        let settled = sigma_to_normalized_linear_rgba(settled_sigma);
        let strained = sigma_to_normalized_linear_rgba(strained_sigma);
        // Visual: strain is red-dominant, settled is green-leaning — the heat cue is spectral, not hex.
        assert!(strained[0] > strained[1] && strained[0] > strained[2], "strain rgba={strained:?}");
        assert!(settled[1] >= settled[0], "settled rgba={settled:?}");
        assert_eq!(strained[3], 1.0, "opaque");
        // Sonic parity from the SAME σ: red/strain folds to a lower pitch than green/settled.
        let f_settled = acoustic::sigma_to_center_frequency_hz(settled_sigma);
        let f_strained = acoustic::sigma_to_center_frequency_hz(strained_sigma);
        assert!(f_strained < f_settled, "strain {f_strained}Hz should be below settled {f_settled}Hz");
    }

    #[test]
    fn system_percepts_cover_every_burden_and_stay_in_the_emf_band() {
        let conditions = vec![je(
            "did:wf:me:condition:1",
            "condition",
            serde_json::json!({"label": "Hypertension"}),
        )];
        let report = build_report_from_journal(&conditions, &[], &[], Lens::Person, 2);
        let percepts = report.system_percepts();
        // One percept per accumulated burden — nothing silently dropped.
        assert_eq!(percepts.len(), report.burdens.len());
        for p in &percepts {
            assert!(p.sigma >= 0.50 - 1e-6 && p.sigma <= 0.93 + 1e-6, "σ in EMF band: {}", p.sigma);
            assert_eq!(p.rgba[3], 1.0);
            assert!(p.frequency_hz > 0.0);
        }
        // The hypertension load lands on the circulatory system.
        assert!(percepts.iter().any(|p| p.system_id == "circulatory"));
    }

    #[test]
    fn paint_organs_colours_by_system_and_reports_unknown_organs() {
        let conditions = vec![je(
            "did:wf:me:condition:1",
            "condition",
            serde_json::json!({"label": "Hypertension"}),
        )];
        let report = build_report_from_journal(&conditions, &[], &[], Lens::Person, 2);
        // A VH_Male organ set: a burdened organ (blood-vasculature → circulatory), an unburdened one
        // (lung → respiratory), and one not in the curated map.
        let (painted, unmapped) = report.paint_organs(&[
            "3d-vh-m-blood-vasculature.glb",
            "3d-vh-m-lung.glb",
            "3d-vh-m-flux-capacitor.glb",
        ]);
        assert_eq!(painted.len(), 2);
        assert_eq!(unmapped, vec!["3d-vh-m-flux-capacitor.glb".to_string()]);

        let circ = painted.iter().find(|o| o.system_id == "circulatory").unwrap();
        let resp = painted.iter().find(|o| o.system_id == "respiratory").unwrap();
        // The hypertension load makes circulatory redder (higher σ) than the settled respiratory organ.
        assert!(circ.percept.sigma >= resp.percept.sigma);
        assert_eq!(resp.percept.level, WellbeingLevel::Settled, "no respiratory load → settled baseline");
        // Every painted organ has an opaque colour and an audible pitch (both encodings present).
        for o in &painted {
            assert_eq!(o.percept.rgba[3], 1.0);
            assert!(o.percept.frequency_hz > 0.0);
        }
    }

    #[test]
    fn overlay_percepts_surface_only_distributed_networks_with_host_hints() {
        let burdens = vec![
            SystemBurden { system_id: "glymphatic".to_string(), net_milli: 400, ..Default::default() },
            SystemBurden { system_id: "circulatory".to_string(), net_milli: 200, ..Default::default() },
            SystemBurden { system_id: "ens".to_string(), net_milli: 150, ..Default::default() },
        ];
        let overlays = overlay_percepts_from_burdens(&burdens);
        // Only the distributed networks appear — the discrete circulatory system is excluded (it
        // paints its own organ mesh via paint_organs instead).
        assert_eq!(overlays.len(), 2);
        assert!(overlays.iter().all(|o| o.system_id == "glymphatic" || o.system_id == "ens"));
        // Anatomical host hints for overlay placement.
        let ens = overlays.iter().find(|o| o.system_id == "ens").unwrap();
        assert_eq!(ens.host_systems, vec!["digestive".to_string()]);
        let gly = overlays.iter().find(|o| o.system_id == "glymphatic").unwrap();
        assert_eq!(gly.host_systems, vec!["nervous".to_string()]);
        // Still a real σ percept — burdened → redder than settled and audible.
        assert!(gly.percept.sigma > 0.5 && gly.percept.frequency_hz > 0.0);
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
