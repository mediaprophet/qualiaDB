//! Reorganise the library: derive a clean, browsable on-disk layout from
//! container metadata. Containers are content-addressed (`<doc_id>.hmc`), which
//! is great for dedup but unreadable for a human. This produces a parallel,
//! human-facing tree organised by topic/tag with descriptive filenames, while
//! the content-addressed store stays the source of truth.
//!
//! Default operation is **copy** (non-destructive). Moving is opt-in.

use std::path::{Path, PathBuf};

use crate::container::HMC_EXTENSION;
use crate::library::{Entry, Library};

/// A single planned placement.
pub struct PlacementOp {
    pub from: PathBuf,
    pub to: PathBuf,
    pub category: String,
}

#[derive(Clone, Copy)]
pub enum ApplyMode {
    /// Copy the container to its organised location (source store untouched).
    Copy,
    /// Move it (the organised tree becomes the only copy).
    Move,
}

/// Build a placement plan rooting an organised tree at `out_root`.
/// `<out_root>/<category>/<safe_title>__<id8>.hmc`.
pub fn plan(library: &Library, out_root: &Path) -> Vec<PlacementOp> {
    library
        .entries
        .iter()
        .map(|e| {
            let category = category_for(e);
            let id8: String = e.manifest.doc_id.chars().take(8).collect();
            let filename = format!("{}__{}.{}", safe_title(e.title()), id8, HMC_EXTENSION);
            let to = out_root.join(&category).join(filename);
            PlacementOp { from: e.path.clone(), to, category }
        })
        .collect()
}

/// Execute a plan. Returns the number of placements performed.
pub fn apply(plan: &[PlacementOp], mode: ApplyMode) -> anyhow::Result<usize> {
    let mut n = 0;
    for op in plan {
        if let Some(parent) = op.to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if op.to.exists() {
            continue;
        }
        match mode {
            ApplyMode::Copy => {
                std::fs::copy(&op.from, &op.to)?;
            }
            ApplyMode::Move => {
                // rename across volumes can fail; fall back to copy+remove.
                if std::fs::rename(&op.from, &op.to).is_err() {
                    std::fs::copy(&op.from, &op.to)?;
                    std::fs::remove_file(&op.from)?;
                }
            }
        }
        n += 1;
    }
    Ok(n)
}

/// Choose a category folder for an entry. First explicit tag wins; otherwise a
/// coarse keyword heuristic over the title; otherwise "unsorted".
fn category_for(e: &Entry) -> String {
    if let Some(tag) = e.manifest.tags.first() {
        return safe_component(tag);
    }
    let t = e.title().to_ascii_lowercase();
    const BUCKETS: &[(&str, &[&str])] = &[
        ("logic", &["logic", "modal", "deontic", "defeasible", "proof", "inference", "argument"]),
        ("algebra", &["algebra", "category", "tensor", "matrix", "group", "lattice", "topology"]),
        ("optimization", &["optim", "convex", "gradient", "linear program", "newton"]),
        ("ml", &["neural", "transformer", "attention", "embedding", "learning", "nlp", "language model"]),
        ("crypto", &["zero-knowledge", "zk", "snark", "elliptic", "lattice-based", "signature", "homomorphic"]),
        ("semantic-web", &["ontology", "rdf", "shacl", "owl", "sparql", "knowledge graph"]),
        ("numerics", &["numerical", "finite element", "spectral", "interpolation", "quadrature", "ode", "pde"]),
    ];
    for (bucket, kws) in BUCKETS {
        if kws.iter().any(|k| t.contains(k)) {
            return bucket.to_string();
        }
    }
    "unsorted".to_string()
}

/// Make a human-readable but filesystem-safe title.
fn safe_title(title: &str) -> String {
    let s = safe_component(title);
    let truncated: String = s.chars().take(80).collect();
    if truncated.is_empty() {
        "untitled".to_string()
    } else {
        truncated
    }
}

fn safe_component(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}
