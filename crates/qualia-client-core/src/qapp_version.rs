//! Semver-style version parsing and comparison for `qapp.json` `version` fields.
//!
//! Supports `major.minor.patch` with optional `-pre` suffix (e.g. `0.0.8-dev`).
//! A stable release (`0.0.8`) sorts newer than the same numeric pre-release (`0.0.8-dev`).

use std::cmp::Ordering;

/// Parsed qapp manifest version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    /// `None` = stable; `Some(label)` = pre-release (lower than stable at same numbers).
    pub pre_release: Option<String>,
}

/// Normalize user-facing version text (`v0.0.8-dev`, ` 0.0.8 `).
pub fn normalize_version_label(raw: &str) -> String {
    let trimmed = raw.trim().trim_start_matches('v').trim();
    let core = trimmed.split('+').next().unwrap_or(trimmed);
    core.to_string()
}

/// Parse a version string; returns `None` when no numeric triple is found.
pub fn parse_version(raw: &str) -> Option<ParsedVersion> {
    let label = normalize_version_label(raw);
    if label.is_empty() {
        return None;
    }

    let (numeric, pre) = match label.split_once('-') {
        Some((n, p)) if !p.is_empty() => (n, Some(p.to_string())),
        _ => (label.as_str(), None),
    };

    let mut parts = numeric.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;

    Some(ParsedVersion {
        major,
        minor,
        patch,
        pre_release: pre,
    })
}

pub fn compare_parsed(a: &ParsedVersion, b: &ParsedVersion) -> Ordering {
    (a.major, a.minor, a.patch)
        .cmp(&(b.major, b.minor, b.patch))
        .then_with(|| compare_pre_release(&a.pre_release, &b.pre_release))
}

fn compare_pre_release(a: &Option<String>, b: &Option<String>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => a.cmp(b),
    }
}

/// Compare two version labels. Returns `None` if either side fails to parse.
pub fn compare_versions(a: &str, b: &str) -> Option<Ordering> {
    let pa = parse_version(a)?;
    let pb = parse_version(b)?;
    Some(compare_parsed(&pa, &pb))
}

/// True when `candidate` is strictly newer than `installed`.
pub fn is_version_newer(candidate: &str, installed: &str) -> bool {
    matches!(
        compare_versions(candidate, installed),
        Some(Ordering::Greater)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_beats_prerelease_at_same_numbers() {
        assert!(is_version_newer("0.0.8", "0.0.8-dev"));
        assert!(!is_version_newer("0.0.8-dev", "0.0.8"));
    }

    #[test]
    fn patch_bump_is_newer() {
        assert!(is_version_newer("0.0.9", "0.0.8"));
        assert!(!is_version_newer("0.0.7", "0.0.8"));
    }

    #[test]
    fn v_prefix_and_whitespace() {
        assert!(parse_version(" v0.0.8 ").is_some());
        assert!(is_version_newer("v0.0.8", "0.0.7"));
    }

    #[test]
    fn equal_versions_not_newer() {
        assert!(!is_version_newer("0.0.8", "0.0.8"));
        assert!(!is_version_newer("0.0.8-dev", "0.0.8-dev"));
    }
}
