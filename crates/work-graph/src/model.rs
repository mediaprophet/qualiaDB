//! Work graph node kinds for the Beat B presence emit.
//!
//! `ig:claimsStatus` is **not** written onto symbols. This graph is a Present
//! extract (found in the tree). Live and Planned belong on later claim nodes.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Honesty {
    pub vocabulary: [&'static str; 3],
    pub this_emit: &'static str,
    pub note: &'static str,
}

impl Honesty {
    pub fn presence_index() -> Self {
        Self {
            vocabulary: ["Present", "Live", "Planned"],
            this_emit: "Present",
            note: "Indexer found these in the tree. Presence is not Live. No Live/Planned stamps in MVP. No held/not-yet amber theatre.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub root: String,
    pub tip_sha: String,
    pub branch: String,
    pub dirty: bool,
    pub indexed_at_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileNode {
    pub path: String,
    pub lang: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateNode {
    pub name: String,
    pub manifest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyNode {
    pub name: String,
    pub member_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocCite {
    pub about: String,
    pub doc_path: String,
    pub kind: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sources {
    pub all_bound: String,
    pub catalog: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkGraph {
    pub honesty: Honesty,
    pub project: Project,
    pub files: Vec<FileNode>,
    pub crates: Vec<CrateNode>,
    pub invoke_ids: Vec<String>,
    pub catalog_ids: Vec<String>,
    pub catalog_present: bool,
    pub families: Vec<FamilyNode>,
    pub doc_pages: Vec<String>,
    pub doc_cites: Vec<DocCite>,
    pub sources: Sources,
}

impl WorkGraph {
    pub fn catalog_only_ids(&self) -> Vec<&str> {
        let bound: std::collections::BTreeSet<&str> =
            self.invoke_ids.iter().map(String::as_str).collect();
        self.catalog_ids
            .iter()
            .map(String::as_str)
            .filter(|id| !bound.contains(id))
            .collect()
    }

    pub fn bound_missing_from_catalog(&self) -> Vec<&str> {
        if !self.catalog_present {
            return Vec::new();
        }
        let catalog: std::collections::BTreeSet<&str> =
            self.catalog_ids.iter().map(String::as_str).collect();
        self.invoke_ids
            .iter()
            .map(String::as_str)
            .filter(|id| !catalog.contains(id))
            .collect()
    }
}
