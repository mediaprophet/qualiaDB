//! Fixture-backed ChEBI bind for the chemical explorer (AST-06b).
//!
//! **Design choice:** Poet WASM does not depend on `qualia-core-db`, and no
//! Host/Vibe capability ID exists for ChEBI under the vibe-host freeze. This
//! module therefore loads a **caller-selected local** AST-03 `compounds.tsv`
//! (file input or paste), builds an in-memory record index, and answers
//! search / entity / relationship / evidence queries against that index only.
//!
//! Shapes mirror `q42::chebi_query` hit surfaces. No network. No invented rows.
//! Absent asset → [`LoadState::NoAsset`].

use super::model::{
    ChemicalHitView, EvidenceHitView, RelationHitView, UncertaintyLabel, CHEBI_LICENCE_CATALOGUE_NOTE,
};

/// Honest load lifecycle for a local compounds asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadState {
    NoAsset,
    Loading,
    Ready,
    Denied,
    Fault,
}

impl LoadState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoAsset => "no_asset",
            Self::Loading => "loading",
            Self::Ready => "ready",
            Self::Denied => "denied",
            Self::Fault => "fault",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw {
            "loading" => Self::Loading,
            "ready" => Self::Ready,
            "denied" => Self::Denied,
            "fault" => Self::Fault,
            _ => Self::NoAsset,
        }
    }

    /// True when search/select may run against a local index.
    pub fn allows_query(self) -> bool {
        self == Self::Ready
    }
}

/// Cold compound row (AST-03 surface) held in the explorer session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureCompound {
    pub id: u64,
    pub accession: String,
    pub name: String,
    pub parent_id: Option<u64>,
    pub source_line: u32,
    pub status: String,
}

/// Bounded local asset session after a successful fixture load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalChebiSession {
    pub release_label: String,
    pub licence_note: String,
    pub compounds: Vec<FixtureCompound>,
}

impl LocalChebiSession {
    pub fn compound_count(&self) -> usize {
        self.compounds.len()
    }

    pub fn find_by_accession(&self, accession: &str) -> Option<&FixtureCompound> {
        let canonical = normalize_accession_query(accession)?;
        self.compounds
            .iter()
            .find(|c| c.accession.eq_ignore_ascii_case(&canonical))
    }

    /// Resolve by accession / bare id, or case-insensitive name substring.
    /// Empty query → empty hits (honest). Cap at `max_hits`.
    pub fn search(&self, query: &str, max_hits: usize) -> Vec<ChemicalHitView> {
        let q = query.trim();
        if q.is_empty() || max_hits == 0 {
            return Vec::new();
        }
        if let Some(acc) = normalize_accession_query(q) {
            return self
                .find_by_accession(&acc)
                .map(|c| vec![self.hit_view(c)])
                .unwrap_or_default();
        }
        let needle = q.to_ascii_lowercase();
        let mut out = Vec::new();
        for compound in &self.compounds {
            if compound.name.to_ascii_lowercase().contains(&needle) {
                out.push(self.hit_view(compound));
                if out.len() >= max_hits {
                    break;
                }
            }
        }
        out
    }

    pub fn relations_for(&self, accession: &str, max_hits: usize) -> Vec<RelationHitView> {
        let Some(selected) = self.find_by_accession(accession) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if let Some(pid) = selected.parent_id {
            if let Some(parent) = self.compounds.iter().find(|c| c.id == pid) {
                out.push(RelationHitView {
                    child_accession: selected.accession.clone(),
                    parent_accession: parent.accession.clone(),
                    release_label: self.release_label.clone(),
                    source_line: selected.source_line,
                    uncertainty: UncertaintyLabel::Known,
                    licence_note: self.licence_note.clone(),
                });
            } else {
                // Parent id present but not in this fixture slice → partial edge.
                out.push(RelationHitView {
                    child_accession: selected.accession.clone(),
                    parent_accession: format!("CHEBI:{pid}"),
                    release_label: self.release_label.clone(),
                    source_line: selected.source_line,
                    uncertainty: UncertaintyLabel::Partial,
                    licence_note: self.licence_note.clone(),
                });
            }
        }
        for child in &self.compounds {
            if child.parent_id == Some(selected.id) {
                out.push(RelationHitView {
                    child_accession: child.accession.clone(),
                    parent_accession: selected.accession.clone(),
                    release_label: self.release_label.clone(),
                    source_line: child.source_line,
                    uncertainty: UncertaintyLabel::Known,
                    licence_note: self.licence_note.clone(),
                });
                if out.len() >= max_hits {
                    break;
                }
            }
        }
        out.truncate(max_hits);
        out
    }

    pub fn evidence_for(&self, accession: &str) -> Vec<EvidenceHitView> {
        let Some(selected) = self.find_by_accession(accession) else {
            return Vec::new();
        };
        vec![EvidenceHitView {
            accession: selected.accession.clone(),
            release_label: self.release_label.clone(),
            source_line: selected.source_line,
            uncertainty: UncertaintyLabel::Known,
            licence_note: self.licence_note.clone(),
        }]
    }

    fn hit_view(&self, compound: &FixtureCompound) -> ChemicalHitView {
        let parent_accession = compound.parent_id.map(|pid| {
            self.compounds
                .iter()
                .find(|c| c.id == pid)
                .map(|c| c.accession.clone())
                .unwrap_or_else(|| format!("CHEBI:{pid}"))
        });
        ChemicalHitView {
            accession: compound.accession.clone(),
            name: compound.name.clone(),
            parent_accession,
            release_label: self.release_label.clone(),
            source_line: compound.source_line,
            uncertainty: UncertaintyLabel::Known,
            licence_note: self.licence_note.clone(),
        }
    }
}

/// Result of attempting to ingest local TSV bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindOutcome {
    Ready(LocalChebiSession),
    Denied { reason: String },
    Fault { reason: String },
}

const REQUIRED_COLUMNS: usize = 7;
const COMPOUNDS_HEADER: [&str; 7] = [
    "ID",
    "STATUS",
    "CHEBI_ACCESSION",
    "SOURCE",
    "PARENT_ID",
    "NAME",
    "DEFINITION",
];

/// Default fixture budgets (tiny synthetic / local paste only).
pub const DEFAULT_MAX_BYTES: usize = 256 * 1024;
pub const DEFAULT_MAX_RECORDS: usize = 2_048;
pub const DEFAULT_MAX_LINE_BYTES: usize = 8_192;
pub const DEFAULT_MAX_SEARCH_HITS: usize = 32;

/// Parse AST-03 `compounds.tsv` text into a local session (no network).
pub fn load_compounds_tsv(
    text: &str,
    release_label: &str,
    max_bytes: usize,
    max_records: usize,
) -> BindOutcome {
    let bytes = text.len();
    if bytes == 0 {
        return BindOutcome::Denied {
            reason: "Empty input — provide a local compounds.tsv (AST-03 header).".into(),
        };
    }
    if bytes > max_bytes {
        return BindOutcome::Denied {
            reason: format!("Byte budget exceeded ({bytes} > {max_bytes})."),
        };
    }
    if release_label.trim().is_empty() {
        return BindOutcome::Fault {
            reason: "Release label is required for provenance.".into(),
        };
    }

    let mut lines = text.lines();
    let Some(header_line) = lines.next() else {
        return BindOutcome::Fault {
            reason: "Missing header row.".into(),
        };
    };
    let header_fields: Vec<&str> = header_line.split('\t').collect();
    if !header_matches(&header_fields) {
        return BindOutcome::Fault {
            reason: "Bad header — expected ID STATUS CHEBI_ACCESSION SOURCE PARENT_ID NAME DEFINITION."
                .into(),
        };
    }

    let mut compounds = Vec::new();
    let mut source_line: u32 = 1;
    for line in lines {
        source_line = source_line.saturating_add(1);
        if line.trim().is_empty() {
            continue;
        }
        if line.len() > DEFAULT_MAX_LINE_BYTES {
            return BindOutcome::Denied {
                reason: format!("Line {source_line} exceeds line budget."),
            };
        }
        match parse_data_row(source_line, line) {
            Ok(compound) => {
                if compounds.len() >= max_records {
                    return BindOutcome::Denied {
                        reason: format!("Record budget exceeded (max {max_records})."),
                    };
                }
                compounds.push(compound);
            }
            Err(_) => {
                // Quarantine silently for explorer bind — do not invent rows.
                continue;
            }
        }
    }

    if compounds.is_empty() {
        return BindOutcome::Fault {
            reason: "No accepted compound rows after parse (all quarantined or empty).".into(),
        };
    }

    BindOutcome::Ready(LocalChebiSession {
        release_label: release_label.trim().to_string(),
        licence_note: CHEBI_LICENCE_CATALOGUE_NOTE.to_string(),
        compounds,
    })
}

/// Convenience: default budgets + release label `local-fixture`.
pub fn load_compounds_tsv_default(text: &str) -> BindOutcome {
    load_compounds_tsv(text, "local-fixture", DEFAULT_MAX_BYTES, DEFAULT_MAX_RECORDS)
}

/// Tiny synthetic AST-03 fixture used by model tests (not bundled as product data).
pub fn synthetic_compounds_tsv() -> &'static str {
    "ID\tSTATUS\tCHEBI_ACCESSION\tSOURCE\tPARENT_ID\tNAME\tDEFINITION\n\
15377\tC\tCHEBI:15377\tChEBI\t\twater\t\n\
16236\tC\tCHEBI:16236\tChEBI\t15377\tethanol\t\n\
24431\tC\tCHEBI:24431\tChEBI\t\tchemical entity\t\n"
}

fn header_matches(fields: &[&str]) -> bool {
    if fields.len() < REQUIRED_COLUMNS {
        return false;
    }
    COMPOUNDS_HEADER
        .iter()
        .zip(fields.iter().take(REQUIRED_COLUMNS))
        .all(|(expected, got)| *expected == *got)
}

fn accession_ok(accession: &str, id: u64) -> bool {
    let Some(rest) = accession.strip_prefix("CHEBI:") else {
        return false;
    };
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    match rest.parse::<u64>() {
        Ok(n) => n == id,
        Err(_) => false,
    }
}

fn parse_data_row(source_line: u32, line: &str) -> Result<FixtureCompound, ()> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() < REQUIRED_COLUMNS {
        return Err(());
    }
    let id: u64 = fields[0].parse().map_err(|_| ())?;
    let status = fields[1];
    if status == "D" {
        return Err(());
    }
    let accession = fields[2];
    if !accession_ok(accession, id) {
        return Err(());
    }
    let parent_id = if fields[4].is_empty() {
        None
    } else {
        Some(fields[4].parse().map_err(|_| ())?)
    };
    let name = fields[5];
    if name.trim().is_empty() {
        return Err(());
    }
    Ok(FixtureCompound {
        id,
        accession: accession.to_string(),
        name: name.to_string(),
        parent_id,
        source_line,
        status: status.to_string(),
    })
}

/// Mirror of `chebi_query::normalize_accession_query` (accession / bare id only).
pub fn normalize_accession_query(query: &str) -> Option<String> {
    let q = query.trim();
    if q.is_empty() {
        return None;
    }
    let upper = q.to_ascii_uppercase();
    if let Some(rest) = upper.strip_prefix("CHEBI:") {
        let rest = rest.trim();
        if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let id: u64 = rest.parse().ok()?;
        return Some(format!("CHEBI:{id}"));
    }
    if q.bytes().all(|b| b.is_ascii_digit()) {
        let id: u64 = q.parse().ok()?;
        return Some(format!("CHEBI:{id}"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_tsv_loads_ready() {
        let outcome = load_compounds_tsv_default(synthetic_compounds_tsv());
        let BindOutcome::Ready(session) = outcome else {
            panic!("expected Ready, got {outcome:?}");
        };
        assert_eq!(session.compound_count(), 3);
        assert_eq!(session.release_label, "local-fixture");
    }

    #[test]
    fn empty_and_bad_header_are_honest() {
        assert!(matches!(
            load_compounds_tsv_default(""),
            BindOutcome::Denied { .. }
        ));
        assert!(matches!(
            load_compounds_tsv_default("not\ta\theader\n"),
            BindOutcome::Fault { .. }
        ));
    }

    #[test]
    fn search_accession_and_name_round_trip() {
        let BindOutcome::Ready(session) =
            load_compounds_tsv_default(synthetic_compounds_tsv())
        else {
            panic!("load");
        };
        let by_acc = session.search("CHEBI:15377", 8);
        assert_eq!(by_acc.len(), 1);
        assert_eq!(by_acc[0].name, "water");
        let by_name = session.search("eth", 8);
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].accession, "CHEBI:16236");
        assert!(session.search("CHEBI:99999", 8).is_empty());
        assert!(session.search("", 8).is_empty());
    }

    #[test]
    fn relations_and_evidence_from_mapped_fixture() {
        let BindOutcome::Ready(session) =
            load_compounds_tsv_default(synthetic_compounds_tsv())
        else {
            panic!("load");
        };
        let rels = session.relations_for("CHEBI:16236", 16);
        assert!(rels.iter().any(|r| r.parent_accession == "CHEBI:15377"));
        let ev = session.evidence_for("CHEBI:16236");
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].release_label, "local-fixture");
        assert_eq!(ev[0].source_line, 3);
    }

    #[test]
    fn deleted_rows_quarantined_not_invented() {
        let tsv = "ID\tSTATUS\tCHEBI_ACCESSION\tSOURCE\tPARENT_ID\tNAME\tDEFINITION\n\
1\tD\tCHEBI:1\tChEBI\t\tgone\t\n\
2\tC\tCHEBI:2\tChEBI\t\tkeep\t\n";
        let BindOutcome::Ready(session) = load_compounds_tsv_default(tsv) else {
            panic!("load");
        };
        assert_eq!(session.compound_count(), 1);
        assert_eq!(session.compounds[0].accession, "CHEBI:2");
    }
}
