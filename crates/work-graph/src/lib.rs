//! Work graph (Implementation Graph) Beat B MVP indexer.
//!
//! Indexes what is **Present** in a QualiaDB checkout: git tip, crates, files,
//! ALL_BOUND / vibe catalog invoke ids, and simple doc-page cites.
//!
//! Honesty: this emit is **Present** only. It does not stamp Live or Planned.
//! No `ImplGraph.*` Host ids are invented.

pub mod emit;
pub mod extract;
pub mod git;
pub mod model;
pub mod walk;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::model::{DocCite, FamilyNode, FileNode, Honesty, Project, WorkGraph};

pub const ALL_BOUND_REL: &str = "crates/qualia-core-db/src/poet_host/invoke/ids.rs";
pub const CATALOG_REL: &str = "crates/vibe/src/catalog/ids.rs";
pub const DEFAULT_OUT_REL: &str = "target/work-graph";

#[derive(Debug)]
pub enum IndexError {
    Io(io::Error),
    Git(String),
    Extract(extract::ExtractError),
    MissingSoT { path: PathBuf, what: &'static str },
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexError::Io(e) => write!(f, "io: {e}"),
            IndexError::Git(e) => write!(f, "git: {e}"),
            IndexError::Extract(e) => write!(f, "extract: {e}"),
            IndexError::MissingSoT { path, what } => {
                write!(f, "missing {what} at {}", path.display())
            }
        }
    }
}

impl std::error::Error for IndexError {}

impl From<io::Error> for IndexError {
    fn from(value: io::Error) -> Self {
        IndexError::Io(value)
    }
}

impl From<extract::ExtractError> for IndexError {
    fn from(value: extract::ExtractError) -> Self {
        IndexError::Extract(value)
    }
}

/// Index a QualiaDB (or QualiaDB-shaped) checkout into a presence graph.
pub fn index_root(root: &Path) -> Result<WorkGraph, IndexError> {
    let root = root
        .canonicalize()
        .map_err(|e| IndexError::Io(io::Error::new(e.kind(), format!("root {}: {e}", root.display()))))?;

    let tip = git::read_tip(&root)?;
    let files = walk::list_project_files(&root)?;
    let crates = walk::list_crates(&root, &files)?;

    let all_bound_path = root.join(ALL_BOUND_REL);
    if !all_bound_path.is_file() {
        return Err(IndexError::MissingSoT {
            path: all_bound_path,
            what: "ALL_BOUND SoT (poet_host/invoke/ids.rs)",
        });
    }
    let all_bound_src = fs::read_to_string(&all_bound_path)?;
    let invoke_ids = extract::extract_all_bound(&all_bound_src)?;

    let catalog_path = root.join(CATALOG_REL);
    let (catalog_ids, catalog_present) = if catalog_path.is_file() {
        let src = fs::read_to_string(&catalog_path)?;
        (extract::extract_catalog_ids(&src)?, true)
    } else {
        (Vec::new(), false)
    };

    let families = extract::families_from_ids(&invoke_ids);
    let family_names: Vec<String> = families.iter().map(|f| f.name.clone()).collect();

    let doc_pages: Vec<FileNode> = files
        .iter()
        .filter(|f| walk::is_doc_page(&f.path))
        .cloned()
        .collect();

    let mut doc_cites = Vec::new();
    for page in &doc_pages {
        let abs = root.join(&page.path);
        let Ok(text) = fs::read_to_string(&abs) else {
            continue;
        };
        if text.len() > walk::MAX_DOC_SCRAPE_BYTES {
            continue;
        }
        for invoke in extract::cited_invoke_ids(&text, &invoke_ids) {
            doc_cites.push(DocCite {
                about: invoke,
                doc_path: page.path.clone(),
                kind: "invoke",
            });
        }
        for family in extract::cited_families(&text, &family_names) {
            doc_cites.push(DocCite {
                about: family,
                doc_path: page.path.clone(),
                kind: "family",
            });
        }
    }

    Ok(WorkGraph {
        honesty: Honesty::presence_index(),
        project: Project {
            root: root.to_string_lossy().into_owned(),
            tip_sha: tip.sha,
            branch: tip.branch,
            dirty: tip.dirty,
            indexed_at_unix: unix_now(),
        },
        files,
        crates,
        invoke_ids,
        catalog_ids,
        catalog_present,
        families,
        doc_pages: doc_pages.into_iter().map(|f| f.path).collect(),
        doc_cites,
        sources: model::Sources {
            all_bound: ALL_BOUND_REL.to_string(),
            catalog: if catalog_present {
                Some(CATALOG_REL.to_string())
            } else {
                None
            },
        },
    })
}

pub fn default_out_dir(root: &Path) -> PathBuf {
    root.join(DEFAULT_OUT_REL)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Small helper used by tests: family member counts stay aligned with invoke ids.
pub fn family_count(families: &[FamilyNode], name: &str) -> Option<u32> {
    families.iter().find(|f| f.name == name).map(|f| f.member_count)
}
