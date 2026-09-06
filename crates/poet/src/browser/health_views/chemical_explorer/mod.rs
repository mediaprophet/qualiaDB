//! Food/compound evidence explorer (AST-06 / AST-06b). Research evidence only.

mod bind;
mod model;
mod workspace;

pub use bind::{
    load_compounds_tsv, load_compounds_tsv_default, normalize_accession_query,
    synthetic_compounds_tsv, BindOutcome, FixtureCompound, LoadState, LocalChebiSession,
    DEFAULT_MAX_BYTES, DEFAULT_MAX_RECORDS, DEFAULT_MAX_SEARCH_HITS,
};
pub use model::{
    AssetPresence, ChemicalExplorerState, ChemicalHitView, EvidenceHitView, ExplorerPhase,
    RelationHitView, UncertaintyLabel, CHEBI_LICENCE_CATALOGUE_NOTE, NO_ASSET_GUIDANCE,
    RESEARCH_EVIDENCE_BANNER,
};

pub fn build_chemical_explorer_view(document: &web_sys::Document) -> web_sys::Element {
    workspace::build(document)
}
