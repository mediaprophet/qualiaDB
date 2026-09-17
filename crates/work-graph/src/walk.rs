//! File list, crate manifests, and doc-page selection.

use std::fs;
use std::path::Path;

use crate::git;
use crate::model::{CrateNode, FileNode};
use crate::IndexError;

/// Skip cite-scrape on huge generated HTML (byte budget, fail closed for that page).
pub const MAX_DOC_SCRAPE_BYTES: usize = 4 * 1024 * 1024;

pub fn list_project_files(root: &Path) -> Result<Vec<FileNode>, IndexError> {
    let paths = git::ls_files(root)?;
    Ok(paths
        .into_iter()
        .map(|path| FileNode {
            lang: lang_for(&path),
            path,
        })
        .collect())
}

pub fn list_crates(root: &Path, files: &[FileNode]) -> Result<Vec<CrateNode>, IndexError> {
    let mut crates = Vec::new();
    for file in files {
        if !is_crate_manifest(&file.path) {
            continue;
        }
        let abs = root.join(&file.path);
        let Ok(src) = fs::read_to_string(&abs) else {
            continue;
        };
        if let Some(name) = package_name(&src) {
            crates.push(CrateNode {
                name,
                manifest: file.path.clone(),
            });
        }
    }
    crates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(crates)
}

pub fn is_doc_page(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    if !(lower.ends_with(".md") || lower.ends_with(".html") || lower.ends_with(".htm")) {
        return false;
    }
    lower.starts_with("docs/")
        || lower.starts_with("skills/")
        || lower.starts_with(".agents/")
        || lower == "readme.md"
        || lower.starts_with("crates/work-graph/")
}

pub fn is_crate_manifest(path: &str) -> bool {
    if !path.ends_with("Cargo.toml") {
        return false;
    }
    // Workspace root + each crate / tool manifest. Skip vendor copies.
    !path.starts_with("vendor/")
}

pub fn lang_for(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "rust",
        "toml" => "toml",
        "md" => "markdown",
        "html" | "htm" => "html",
        "ts" | "tsx" => "typescript",
        "js" | "mjs" => "javascript",
        "vibe" => "vibe",
        "n3" => "n3",
        "ttl" => "turtle",
        "json" => "json",
        "wgsl" => "wgsl",
        "css" => "css",
        "py" => "python",
        "sh" => "shell",
        "ps1" => "powershell",
        "yml" | "yaml" => "yaml",
        _ => "other",
    }
}

fn package_name(src: &str) -> Option<String> {
    let pkg = src.find("[package]")?;
    let rest = &src[pkg + "[package]".len()..];
    let next_table = rest.find("\n[").unwrap_or(rest.len());
    let section = &rest[..next_table];
    for line in section.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("name") {
            let val = val.trim().trim_start_matches('=').trim();
            if let Some(name) = unquote(val) {
                return Some(name);
            }
        }
    }
    None
}

fn unquote(s: &str) -> Option<String> {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        return Some(s[1..s.len() - 1].to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_manifests_skip_vendor() {
        assert!(is_crate_manifest("crates/vibe/Cargo.toml"));
        assert!(is_crate_manifest("Cargo.toml"));
        assert!(!is_crate_manifest("vendor/imap-proto/Cargo.toml"));
        assert!(!is_crate_manifest("crates/vibe/src/lib.rs"));
    }

    #[test]
    fn doc_pages_are_docs_and_skills() {
        assert!(is_doc_page("docs/vibe/SKILL.md"));
        assert!(is_doc_page("docs/index.html"));
        assert!(is_doc_page("README.md"));
        assert!(!is_doc_page("crates/vibe/src/lib.rs"));
        assert!(!is_doc_page("docs/data/schemaorg/foo.json"));
    }

    #[test]
    fn package_name_reads_first_table() {
        let src = "[workspace]\nmembers=[\"a\"]\n\n[package]\nname = \"work-graph\"\nversion = \"0.0.38\"\n";
        assert_eq!(package_name(src).as_deref(), Some("work-graph"));
    }
}
