//! Chemical evidence explorer state. Shapes mirror `chebi_query` hits; no live claims.

use super::bind::{
    load_compounds_tsv, BindOutcome, LoadState, LocalChebiSession, DEFAULT_MAX_BYTES,
    DEFAULT_MAX_RECORDS, DEFAULT_MAX_SEARCH_HITS,
};

/// Catalogue-only ChEBI licence note (AST-07 descriptor text). Not a redistribution claim.
pub const CHEBI_LICENCE_CATALOGUE_NOTE: &str = "Upstream release stated as CC BY 4.0; first \
production importer candidate. Catalogue entry only — do not bundle release bytes in-repo.";

pub const RESEARCH_EVIDENCE_BANNER: &str = "Research evidence — not medical advice";

pub const NO_ASSET_GUIDANCE: &str = "No local ChEBI asset is installed. Import a compounds.tsv \
file through the governed asset import path (AST-02/AST-03), or load a local fixture via the \
file picker / paste below. This explorer never fetches remote release bytes.";

/// Whether a local ChEBI compounds asset is present for query (legacy badge alias).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetPresence {
    NoAsset,
    Available,
}

impl From<LoadState> for AssetPresence {
    fn from(state: LoadState) -> Self {
        if state.allows_query() {
            Self::Available
        } else {
            Self::NoAsset
        }
    }
}

/// UI phase for search → entity → relationships → evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerPhase {
    NoAsset,
    Loading,
    Denied,
    Fault,
    EmptySearch,
    SelectedEntity,
}

/// Compact hit mirroring `chebi_query::ChemicalHit` surface fields (cold/UI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChemicalHitView {
    pub accession: String,
    pub name: String,
    pub parent_accession: Option<String>,
    pub release_label: String,
    pub source_line: u32,
    pub uncertainty: UncertaintyLabel,
    pub licence_note: String,
}

/// Parent/child edge for the relationships panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationHitView {
    pub child_accession: String,
    pub parent_accession: String,
    pub release_label: String,
    pub source_line: u32,
    pub uncertainty: UncertaintyLabel,
    pub licence_note: String,
}

/// Provenance row for the evidence drawer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceHitView {
    pub accession: String,
    pub release_label: String,
    pub source_line: u32,
    pub uncertainty: UncertaintyLabel,
    pub licence_note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UncertaintyLabel {
    Known,
    Partial,
    Unknown,
}

impl UncertaintyLabel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Known => "known",
            Self::Partial => "partial",
            Self::Unknown => "unknown",
        }
    }
}

/// Workspace state. Default is honest no-asset; never fabricates compound claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChemicalExplorerState {
    pub load: LoadState,
    pub query: String,
    pub hits: Vec<ChemicalHitView>,
    pub selected: Option<ChemicalHitView>,
    pub relations: Vec<RelationHitView>,
    pub evidence: Vec<EvidenceHitView>,
    pub licence_drawer_open: bool,
    /// In-memory local session when [`LoadState::Ready`].
    pub session: Option<LocalChebiSession>,
    /// Last fault / denied reason (surface copy only).
    pub load_message: String,
    /// Raw fixture text retained for DOM persistence / re-bind.
    pub fixture_text: String,
    pub release_label: String,
}

impl Default for ChemicalExplorerState {
    fn default() -> Self {
        Self {
            load: LoadState::NoAsset,
            query: String::new(),
            hits: Vec::new(),
            selected: None,
            relations: Vec::new(),
            evidence: Vec::new(),
            licence_drawer_open: false,
            session: None,
            load_message: String::new(),
            fixture_text: String::new(),
            release_label: "local-fixture".into(),
        }
    }
}

impl ChemicalExplorerState {
    /// Legacy alias: Available only when Ready.
    pub fn asset(&self) -> AssetPresence {
        self.load.into()
    }

    pub fn phase(&self) -> ExplorerPhase {
        match self.load {
            LoadState::NoAsset => ExplorerPhase::NoAsset,
            LoadState::Loading => ExplorerPhase::Loading,
            LoadState::Denied => ExplorerPhase::Denied,
            LoadState::Fault => ExplorerPhase::Fault,
            LoadState::Ready => {
                if self.selected.is_some() {
                    ExplorerPhase::SelectedEntity
                } else {
                    ExplorerPhase::EmptySearch
                }
            }
        }
    }

    pub fn set_query(&mut self, raw: &str) {
        self.query = raw.to_string();
        if !self.load.allows_query() {
            self.hits.clear();
            self.selected = None;
            self.relations.clear();
            self.evidence.clear();
        }
    }

    /// Begin a local fixture load (honest Loading before parse completes).
    pub fn begin_load(&mut self) {
        self.load = LoadState::Loading;
        self.load_message.clear();
        self.hits.clear();
        self.selected = None;
        self.relations.clear();
        self.evidence.clear();
        self.session = None;
    }

    /// Apply a bind outcome from local TSV parse (fixture path).
    pub fn apply_bind_outcome(&mut self, outcome: BindOutcome) {
        match outcome {
            BindOutcome::Ready(session) => {
                self.load = LoadState::Ready;
                self.load_message = format!(
                    "Local asset ready · {} compounds · release {}",
                    session.compound_count(),
                    session.release_label
                );
                self.release_label = session.release_label.clone();
                self.session = Some(session);
            }
            BindOutcome::Denied { reason } => {
                self.load = LoadState::Denied;
                self.load_message = reason;
                self.session = None;
                self.clear_results();
            }
            BindOutcome::Fault { reason } => {
                self.load = LoadState::Fault;
                self.load_message = reason;
                self.session = None;
                self.clear_results();
            }
        }
    }

    /// Parse pasted / file TSV and transition load state.
    pub fn ingest_fixture_tsv(&mut self, text: &str, release_label: &str) {
        self.begin_load();
        self.fixture_text = text.to_string();
        if !release_label.trim().is_empty() {
            self.release_label = release_label.trim().to_string();
        }
        let outcome = load_compounds_tsv(
            text,
            &self.release_label,
            DEFAULT_MAX_BYTES,
            DEFAULT_MAX_RECORDS,
        );
        self.apply_bind_outcome(outcome);
        if self.load.allows_query() && !self.query.trim().is_empty() {
            self.run_local_search();
        }
    }

    /// Clear asset back to NoAsset (user dismiss / unload).
    pub fn clear_asset(&mut self) {
        *self = Self {
            licence_drawer_open: self.licence_drawer_open,
            ..Self::default()
        };
    }

    /// Run search against the in-memory session (Ready only).
    pub fn run_local_search(&mut self) {
        if !self.load.allows_query() {
            self.clear_results();
            return;
        }
        let Some(session) = &self.session else {
            self.clear_results();
            return;
        };
        let hits = session.search(&self.query, DEFAULT_MAX_SEARCH_HITS);
        self.apply_search_hits(hits);
    }

    /// After select: fill relations + evidence from the local session.
    pub fn hydrate_selection_panels(&mut self) {
        if !self.load.allows_query() {
            self.relations.clear();
            self.evidence.clear();
            return;
        }
        let Some(selected) = self.selected.clone() else {
            self.relations.clear();
            self.evidence.clear();
            return;
        };
        let Some(session) = &self.session else {
            self.relations.clear();
            self.evidence.clear();
            return;
        };
        let relations = session.relations_for(&selected.accession, DEFAULT_MAX_SEARCH_HITS);
        let evidence = session.evidence_for(&selected.accession);
        self.relations = relations;
        self.evidence = evidence;
    }

    /// Apply caller-supplied hits (native/fixture). Empty when no asset — never invents rows.
    pub fn apply_search_hits(&mut self, hits: Vec<ChemicalHitView>) {
        if !self.load.allows_query() {
            self.clear_results();
            return;
        }
        self.hits = hits;
        if let Some(selected) = &self.selected {
            if !self
                .hits
                .iter()
                .any(|hit| hit.accession == selected.accession)
            {
                self.selected = None;
                self.relations.clear();
                self.evidence.clear();
            }
        }
    }

    pub fn select_hit(&mut self, accession: &str) -> bool {
        if !self.load.allows_query() {
            return false;
        }
        let Some(hit) = self
            .hits
            .iter()
            .find(|hit| hit.accession == accession)
            .cloned()
        else {
            return false;
        };
        self.selected = Some(hit);
        self.hydrate_selection_panels();
        true
    }

    pub fn clear_selection(&mut self) {
        self.selected = None;
        self.relations.clear();
        self.evidence.clear();
    }

    pub fn set_relations(&mut self, relations: Vec<RelationHitView>) {
        if self.selected.is_none() || !self.load.allows_query() {
            self.relations.clear();
            return;
        }
        self.relations = relations;
    }

    pub fn set_evidence(&mut self, evidence: Vec<EvidenceHitView>) {
        if self.selected.is_none() || !self.load.allows_query() {
            self.evidence.clear();
            return;
        }
        self.evidence = evidence;
    }

    pub fn toggle_licence_drawer(&mut self) {
        self.licence_drawer_open = !self.licence_drawer_open;
    }

    /// Mark Ready without a session (legacy test helper). Prefer [`Self::ingest_fixture_tsv`].
    pub fn mark_asset_available(&mut self) {
        self.load = LoadState::Ready;
        if self.session.is_none() {
            self.load_message = "Local asset marked available (no session index).".into();
        }
    }

    pub fn status_message(&self) -> String {
        match self.phase() {
            ExplorerPhase::NoAsset => NO_ASSET_GUIDANCE.to_string(),
            ExplorerPhase::Loading => "Loading local compounds.tsv…".into(),
            ExplorerPhase::Denied => {
                if self.load_message.is_empty() {
                    "Local asset load denied (budget or policy).".into()
                } else {
                    self.load_message.clone()
                }
            }
            ExplorerPhase::Fault => {
                if self.load_message.is_empty() {
                    "Local asset load failed.".into()
                } else {
                    self.load_message.clone()
                }
            }
            ExplorerPhase::EmptySearch if self.query.trim().is_empty() => {
                "Enter a ChEBI accession or name. Results stay empty until a local asset answers."
                    .into()
            }
            ExplorerPhase::EmptySearch => {
                "No matching compounds in the installed asset. Associations are research evidence only."
                    .into()
            }
            ExplorerPhase::SelectedEntity => RESEARCH_EVIDENCE_BANNER.to_string(),
        }
    }

    /// Serialize a hit list to JSON matching the cold chebi_query surface (fixture helper).
    pub fn hits_as_json_fixture(hits: &[ChemicalHitView]) -> String {
        let mut items = Vec::with_capacity(hits.len());
        for hit in hits {
            items.push(serde_json::json!({
                "accession": hit.accession,
                "name": hit.name,
                "parent_accession": hit.parent_accession,
                "release_label": hit.release_label,
                "source_line": hit.source_line,
                "uncertainty": hit.uncertainty.as_str(),
                "licence_note": hit.licence_note,
            }));
        }
        serde_json::to_string(&serde_json::json!({ "hits": items })).unwrap_or_else(|_| {
            "{\"hits\":[]}".into()
        })
    }

    fn clear_results(&mut self) {
        self.hits.clear();
        self.selected = None;
        self.relations.clear();
        self.evidence.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::health_views::chemical_explorer::bind::synthetic_compounds_tsv;

    fn sample_hit(accession: &str, name: &str) -> ChemicalHitView {
        ChemicalHitView {
            accession: accession.into(),
            name: name.into(),
            parent_accession: Some("CHEBI:24431".into()),
            release_label: "local-import".into(),
            source_line: 42,
            uncertainty: UncertaintyLabel::Partial,
            licence_note: CHEBI_LICENCE_CATALOGUE_NOTE.into(),
        }
    }

    #[test]
    fn default_is_no_asset() {
        let state = ChemicalExplorerState::default();
        assert_eq!(state.phase(), ExplorerPhase::NoAsset);
        assert_eq!(state.load, LoadState::NoAsset);
        let msg = state.status_message();
        assert!(msg.contains("compounds.tsv"));
        assert!(msg.contains("never fetches remote"));
        assert!(!msg.contains("href=\"http"));
    }

    #[test]
    fn empty_search_after_asset_available() {
        let mut state = ChemicalExplorerState::default();
        state.mark_asset_available();
        assert_eq!(state.phase(), ExplorerPhase::EmptySearch);
        state.set_query("  ");
        assert!(state.hits.is_empty());
        assert_eq!(state.phase(), ExplorerPhase::EmptySearch);
    }

    #[test]
    fn selected_entity_requires_hit_from_results() {
        let mut state = ChemicalExplorerState::default();
        state.mark_asset_available();
        assert!(!state.select_hit("CHEBI:15377"));
        state.apply_search_hits(vec![sample_hit("CHEBI:15377", "water")]);
        assert!(state.select_hit("CHEBI:15377"));
        assert_eq!(state.phase(), ExplorerPhase::SelectedEntity);
        assert_eq!(
            state.selected.as_ref().map(|h| h.name.as_str()),
            Some("water")
        );
    }

    #[test]
    fn no_asset_rejects_injected_hits_and_selection() {
        let mut state = ChemicalExplorerState::default();
        state.apply_search_hits(vec![sample_hit("CHEBI:15377", "water")]);
        assert!(state.hits.is_empty());
        assert!(!state.select_hit("CHEBI:15377"));
        assert_eq!(state.phase(), ExplorerPhase::NoAsset);
    }

    #[test]
    fn licence_drawer_toggles_without_changing_phase() {
        let mut state = ChemicalExplorerState::default();
        assert!(!state.licence_drawer_open);
        state.toggle_licence_drawer();
        assert!(state.licence_drawer_open);
        assert_eq!(state.phase(), ExplorerPhase::NoAsset);
    }

    #[test]
    fn fixture_json_is_empty_array_without_hits() {
        let json = ChemicalExplorerState::hits_as_json_fixture(&[]);
        assert_eq!(json, "{\"hits\":[]}");
    }

    #[test]
    fn research_banner_constant_is_non_advisory() {
        assert!(RESEARCH_EVIDENCE_BANNER.contains("not medical advice"));
        assert!(CHEBI_LICENCE_CATALOGUE_NOTE.contains("CC BY 4.0"));
        assert!(CHEBI_LICENCE_CATALOGUE_NOTE.contains("Catalogue entry only"));
    }

    #[test]
    fn load_state_transitions_parse_map_query_round_trip() {
        let mut state = ChemicalExplorerState::default();
        assert_eq!(state.load, LoadState::NoAsset);

        state.begin_load();
        assert_eq!(state.load, LoadState::Loading);
        assert_eq!(state.phase(), ExplorerPhase::Loading);

        state.ingest_fixture_tsv(synthetic_compounds_tsv(), "unit-test-release");
        assert_eq!(state.load, LoadState::Ready);
        assert_eq!(state.phase(), ExplorerPhase::EmptySearch);
        assert!(state.session.as_ref().map(|s| s.compound_count()) == Some(3));

        state.set_query("water");
        state.run_local_search();
        assert_eq!(state.hits.len(), 1);
        assert_eq!(state.hits[0].accession, "CHEBI:15377");

        assert!(state.select_hit("CHEBI:15377"));
        assert_eq!(state.phase(), ExplorerPhase::SelectedEntity);
        assert!(!state.evidence.is_empty());
        // water has no parent in fixture; ethanol is a child
        assert!(state
            .relations
            .iter()
            .any(|r| r.child_accession == "CHEBI:16236"));
        assert!(state.status_message().contains("not medical advice"));
    }

    #[test]
    fn fault_on_bad_header_stays_honest() {
        let mut state = ChemicalExplorerState::default();
        state.ingest_fixture_tsv("nope\n", "rel");
        assert_eq!(state.load, LoadState::Fault);
        assert_eq!(state.phase(), ExplorerPhase::Fault);
        assert!(state.hits.is_empty());
        assert!(state.session.is_none());
    }

    #[test]
    fn denied_on_empty_input() {
        let mut state = ChemicalExplorerState::default();
        state.ingest_fixture_tsv("", "rel");
        assert_eq!(state.load, LoadState::Denied);
        assert_eq!(state.phase(), ExplorerPhase::Denied);
    }

    #[test]
    fn clear_asset_returns_to_no_asset() {
        let mut state = ChemicalExplorerState::default();
        state.ingest_fixture_tsv(synthetic_compounds_tsv(), "rel");
        state.set_query("ethanol");
        state.run_local_search();
        assert!(!state.hits.is_empty());
        state.clear_asset();
        assert_eq!(state.load, LoadState::NoAsset);
        assert!(state.hits.is_empty());
        assert!(state.session.is_none());
    }
}
