//! Grounded ALL_BOUND / catalog extract. Parses tree text — does not invent ids.

use std::collections::BTreeMap;
use std::fmt;

use crate::model::FamilyNode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractError {
    MissingAllBound,
    MissingCatalog,
    Unresolved { names: Vec<String> },
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtractError::MissingAllBound => write!(f, "ALL_BOUND array not found"),
            ExtractError::MissingCatalog => write!(f, "ALL_INVOKE_IDS array not found"),
            ExtractError::Unresolved { names } => {
                write!(f, "ALL_BOUND names had no string literal: {names:?}")
            }
        }
    }
}

impl std::error::Error for ExtractError {}

/// `pub const NAME: &str = "Family.method";` including a wrapped literal line.
pub fn parse_str_consts(src: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &src[i..];
        let Some(rel) = rest.find("pub const ") else {
            break;
        };
        i += rel + "pub const ".len();
        let ident = take_ident(&src[i..]);
        if ident.is_empty() {
            continue;
        }
        i += ident.len();
        let after = skip_ws(&src[i..]);
        i += src[i..].len() - after.len();
        if !after.starts_with(": &str") {
            continue;
        }
        i += ": &str".len();
        let after = skip_ws(&src[i..]);
        i += src[i..].len() - after.len();
        if !after.starts_with('=') {
            continue;
        }
        i += 1;
        let after = skip_ws(&src[i..]);
        i += src[i..].len() - after.len();
        if !after.starts_with('"') {
            continue;
        }
        i += 1;
        if let Some((lit, consumed)) = take_string_lit(&src[i..]) {
            out.insert(ident.to_string(), lit);
            i += consumed;
        }
    }
    out
}

/// Resolve `ALL_BOUND` const names to `Family.method` strings.
pub fn extract_all_bound(src: &str) -> Result<Vec<String>, ExtractError> {
    let consts = parse_str_consts(src);
    let names = parse_slice_idents(src, "ALL_BOUND").ok_or(ExtractError::MissingAllBound)?;
    let mut unresolved = Vec::new();
    let mut ids = Vec::new();
    for name in names {
        match consts.get(&name) {
            Some(id) => ids.push(id.clone()),
            None => unresolved.push(name),
        }
    }
    if !unresolved.is_empty() {
        return Err(ExtractError::Unresolved { names: unresolved });
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

/// Quoted strings inside `ALL_INVOKE_IDS`.
pub fn extract_catalog_ids(src: &str) -> Result<Vec<String>, ExtractError> {
    let ids = parse_slice_strings(src, "ALL_INVOKE_IDS").ok_or(ExtractError::MissingCatalog)?;
    let mut ids = ids;
    ids.sort();
    ids.dedup();
    Ok(ids)
}

pub fn families_from_ids(ids: &[String]) -> Vec<FamilyNode> {
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for id in ids {
        if let Some(fam) = family_of(id) {
            *counts.entry(fam.to_string()).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .map(|(name, member_count)| FamilyNode { name, member_count })
        .collect()
}

pub fn family_of(id: &str) -> Option<&str> {
    let dot = id.find('.')?;
    if dot == 0 {
        return None;
    }
    Some(&id[..dot])
}

/// Exact `Family.method` (or longer dotted) strings found in a doc page.
pub fn cited_invoke_ids(text: &str, invoke_ids: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for id in invoke_ids {
        if text.contains(id.as_str()) {
            found.push(id.clone());
        }
    }
    found
}

/// Family cite only when the catalog prefix `Family.` appears (avoids bare-word hits).
pub fn cited_families(text: &str, families: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for fam in families {
        let prefix = format!("{fam}.");
        if text.contains(&prefix) {
            found.push(fam.clone());
        }
    }
    found
}

fn parse_slice_idents(src: &str, const_name: &str) -> Option<Vec<String>> {
    let body = slice_body(src, const_name)?;
    let mut names = Vec::new();
    let mut i = 0;
    let bytes = body.as_bytes();
    while i < bytes.len() {
        if body[i..].starts_with("//") {
            if let Some(nl) = body[i..].find('\n') {
                i += nl + 1;
                continue;
            }
            break;
        }
        let c = bytes[i];
        if is_ident_start(c) {
            let ident = take_ident(&body[i..]);
            names.push(ident.to_string());
            i += ident.len();
            continue;
        }
        i += 1;
    }
    Some(names)
}

fn parse_slice_strings(src: &str, const_name: &str) -> Option<Vec<String>> {
    let body = slice_body(src, const_name)?;
    let mut ids = Vec::new();
    let mut i = 0;
    while i < body.len() {
        if body[i..].starts_with("//") {
            if let Some(nl) = body[i..].find('\n') {
                i += nl + 1;
                continue;
            }
            break;
        }
        if body.as_bytes()[i] == b'"' {
            i += 1;
            if let Some((lit, consumed)) = take_string_lit(&body[i..]) {
                ids.push(lit);
                i += consumed;
                continue;
            }
        }
        i += 1;
    }
    Some(ids)
}

fn slice_body<'a>(src: &'a str, const_name: &str) -> Option<&'a str> {
    let needle = format!("pub const {const_name}");
    let start = src.find(&needle)?;
    let after = &src[start + needle.len()..];
    let open = after.find("=[")
        .or_else(|| after.find("= ["))
        .or_else(|| after.find("= &[") )?;
    let after_eq = &after[open..];
    let bracket = after_eq.find('[')?;
    let inner_start = start + needle.len() + open + bracket + 1;
    let rest = &src[inner_start..];
    let close = find_slice_close(rest)?;
    Some(&rest[..close])
}

fn find_slice_close(rest: &str) -> Option<usize> {
    rest.find("];")
}

fn take_ident(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.is_empty() || !is_ident_start(bytes[0]) {
        return "";
    }
    let mut n = 1;
    while n < bytes.len() && is_ident_cont(bytes[n]) {
        n += 1;
    }
    &s[..n]
}

fn take_string_lit(after_open_quote: &str) -> Option<(String, usize)> {
    let bytes = after_open_quote.as_bytes();
    let mut i = 0;
    let mut out = String::new();
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                i += 1;
                if i >= bytes.len() {
                    return None;
                }
                match bytes[i] {
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    b'r' => out.push('\r'),
                    b'\\' => out.push('\\'),
                    b'"' => out.push('"'),
                    other => out.push(other as char),
                }
                i += 1;
            }
            b'"' => return Some((out, i + 1)),
            c => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    None
}

fn skip_ws(s: &str) -> &str {
    s.trim_start()
}

fn is_ident_start(c: u8) -> bool {
    c == b'_' || c.is_ascii_alphabetic()
}

fn is_ident_cont(c: u8) -> bool {
    c == b'_' || c.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
pub const DISCOVERY_LIST: &str = "CapabilityDiscovery.list";
pub const CLIN_FRAMINGHAM: &str =
    "ClinicalRisk.framingham";
pub const LTL_EVALUATE: &str = "TemporalAndDescriptionLogic.ltl.evaluate";

pub const ALL_BOUND: &[&str] = &[
    DISCOVERY_LIST,
    CLIN_FRAMINGHAM,
    // comment
    LTL_EVALUATE,
];

pub fn family_bound(name: &str) -> bool { true }
"#;

    const CATALOG: &str = r#"
pub const ALL_INVOKE_IDS: &[&str] = &[
    "CapabilityDiscovery.list",
    "ClinicalRisk.framingham",
    "TemporalAndDescriptionLogic.ltl.evaluate",
];
"#;

    #[test]
    fn extracts_wrapped_and_commented_all_bound() {
        let ids = extract_all_bound(FIXTURE).unwrap();
        assert_eq!(
            ids,
            vec![
                "CapabilityDiscovery.list".to_string(),
                "ClinicalRisk.framingham".to_string(),
                "TemporalAndDescriptionLogic.ltl.evaluate".to_string(),
            ]
        );
    }

    #[test]
    fn extracts_catalog_strings() {
        let ids = extract_catalog_ids(CATALOG).unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"ClinicalRisk.framingham".to_string()));
    }

    #[test]
    fn unresolved_const_fails_closed() {
        let src = r#"
pub const FOO: &str = "A.b";
pub const ALL_BOUND: &[&str] = &[FOO, MISSING];
"#;
        match extract_all_bound(src) {
            Err(ExtractError::Unresolved { names }) => assert_eq!(names, vec!["MISSING".to_string()]),
            other => panic!("expected unresolved, got {other:?}"),
        }
    }

    #[test]
    fn missing_all_bound_fails() {
        assert_eq!(
            extract_all_bound("pub const FOO: &str = \"A.b\";"),
            Err(ExtractError::MissingAllBound)
        );
    }

    #[test]
    fn family_grouping() {
        let ids = vec![
            "Statistics.mean".into(),
            "Statistics.median".into(),
            "SHACL.validate".into(),
        ];
        let fams = families_from_ids(&ids);
        assert_eq!(fams.len(), 2);
        assert_eq!(fams.iter().find(|f| f.name == "Statistics").unwrap().member_count, 2);
    }

    #[test]
    fn doc_cites_require_family_dot_prefix() {
        let text = "See ClinicalRisk.framingham and also Statistics in passing.";
        let ids = vec!["ClinicalRisk.framingham".into()];
        let fams = vec!["ClinicalRisk".into(), "Statistics".into()];
        assert_eq!(cited_invoke_ids(text, &ids), vec!["ClinicalRisk.framingham".to_string()]);
        assert_eq!(cited_families(text, &fams), vec!["ClinicalRisk".to_string()]);
    }

    #[test]
    fn empty_all_bound_is_ok() {
        let src = r#"pub const ALL_BOUND: &[&str] = &[];"#;
        assert_eq!(extract_all_bound(src).unwrap(), Vec::<String>::new());
    }
}
