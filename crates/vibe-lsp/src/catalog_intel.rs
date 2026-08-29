//! Workshop-dialect completion and hover from the Vibe catalog.

use serde_json::{json, Value};
use vibe::catalog::{
    canonical_id, describe, families, family_of, methods_for_family, ALL_INVOKE_IDS,
};

const KEYWORDS: &[(&str, &str)] = &[
    ("cell", "Reactive cell declaration (`cell name := expr;`)"),
    ("fn", "Function declaration"),
    ("pure", "Pure effect modifier"),
    ("effect", "External effect modifier"),
    ("hot", "Hot zero-heap performance modifier"),
    ("cold", "Cold construction modifier"),
    ("using", "Lease a catalog family (`using Animation;`)"),
    ("requires", "Capability requirements clause"),
    ("import", "import \"vibe:0.1/math\" as math;"),
    ("prefix", "prefix p: <iri>;"),
    ("locale", "Opt-in keyword locale (`locale zh;`)"),
    ("present", "Presentation sheaf surface"),
    ("graph", "Embedded graph pattern"),
    (
        "graph?",
        "Embedded SPARQL ASK (fail-closed without GraphDatabase)",
    ),
    ("law", "law Name when expr => consequence;"),
    ("material", "material Name { ... }"),
    ("field", "field name: Type { ... }"),
    (
        "obligate",
        "Deontic obligation — lowers to DeonticLogic.evaluate when leased",
    ),
    (
        "permit",
        "Deontic permission — lowers to DeonticLogic.evaluate when leased",
    ),
    (
        "forbid",
        "Deontic prohibition — lowers to DeonticLogic.evaluate when leased",
    ),
    (
        "knows",
        "Epistemic knowledge — lowers to EpistemicLogic.evaluate when leased",
    ),
    (
        "believes",
        "Epistemic belief — lowers to EpistemicLogic.evaluate when leased",
    ),
    (
        "always",
        "LTL G(φ) — lowers to TemporalAndDescriptionLogic.ltl.globally when leased",
    ),
    (
        "eventually",
        "LTL F(φ) — lowers to TemporalAndDescriptionLogic.ltl.finally when leased",
    ),
    ("until", "LTL until"),
];

pub fn completions_at(src: &str, line: usize, character: usize) -> Vec<Value> {
    let offset = position_to_offset(src, line, character);
    let prefix = &src[..offset.min(src.len())];
    if let Some(family) = family_before_dot(prefix) {
        let methods = methods_for_family(family);
        if !methods.is_empty() {
            return methods
                .into_iter()
                .map(|id| {
                    let method = id.rsplit('.').next().unwrap_or(id);
                    json!({
                        "label": method,
                        "kind": 3,
                        "detail": describe(id),
                        "insertText": method,
                        "documentation": format!("`using {family};` leases this invoke"),
                    })
                })
                .collect();
        }
    }
    let mut out = Vec::new();
    for (label, detail) in KEYWORDS {
        out.push(json!({
            "label": label,
            "kind": 14,
            "detail": detail,
        }));
    }
    for fam in families() {
        out.push(json!({
            "label": fam,
            "kind": 9,
            "detail": format!("Catalog family — `using {fam};` then `{fam}.method(...)`"),
            "insertText": fam,
        }));
    }
    for id in ALL_INVOKE_IDS.iter().copied().take(32) {
        out.push(json!({
            "label": id,
            "kind": 3,
            "detail": describe(id),
        }));
    }
    out
}

pub fn hover_at(src: &str, line: usize, character: usize) -> String {
    let offset = position_to_offset(src, line, character);
    let token = ident_path_at(src, offset);
    if token.is_empty() {
        return "**VibeScript** (`vibe-0.1`)\n\nWorkshop dialect: `using Family;` then `Family.method(...)`. Modal verbs stay terms unless the engine family is leased.".into();
    }
    if let Some(id) = canonical_id(&token) {
        let lease = family_of(id)
            .map(|f| format!("\n\nLease: `using {f};`"))
            .unwrap_or_default();
        return format!(
            "**`{id}`**\n\n{}\n\nType: host invoke (fail-closed).{lease}",
            describe(&token)
        );
    }
    for (kw, detail) in KEYWORDS {
        if *kw == token {
            return format!("**`{kw}`**\n\n{detail}");
        }
    }
    format!("**`{token}`**\n\n{}", describe(&token))
}

pub fn workspace_edit_for_fix(uri: &str, diagnostic: &Value, src: &str) -> Option<Value> {
    let fix = diagnostic
        .pointer("/data/suggested_fix")
        .and_then(|v| v.as_str())?;
    let range = diagnostic.get("range").cloned().unwrap_or_else(|| {
        json!({
            "start": { "line": 0, "character": 0 },
            "end": { "line": 0, "character": 0 }
        })
    });
    let insert = if fix.starts_with("add `using") || fix.contains("using Family") {
        let fam = family_from_source_diagnostic(src).unwrap_or("Animation");
        format!("using {fam};\n")
    } else if fix.starts_with("add requires") {
        format!("{fix}\n")
    } else {
        format!("/* {fix} */\n")
    };
    let insert_range = if insert.starts_with("using ")
        || insert.starts_with("add requires")
        || insert.starts_with("requires")
    {
        json!({ "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } })
    } else {
        range
    };
    Some(json!({
        "changes": {
            uri: [{
                "range": insert_range,
                "newText": insert,
            }]
        }
    }))
}

fn family_from_source_diagnostic(src: &str) -> Option<&'static str> {
    for fam in families() {
        if src.contains(fam) {
            return Some(fam);
        }
    }
    None
}

fn family_before_dot(prefix: &str) -> Option<&str> {
    let trimmed = prefix.trim_end();
    let trimmed = trimmed.strip_suffix('.')?;
    let start = trimmed
        .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .map(|i| i + 1)
        .unwrap_or(0);
    let fam = &trimmed[start..];
    if fam.is_empty() {
        return None;
    }
    Some(fam)
}

fn ident_path_at(src: &str, offset: usize) -> String {
    let bytes = src.as_bytes();
    if bytes.is_empty() {
        return String::new();
    }
    let mut i = offset.min(bytes.len().saturating_sub(1));
    while i > 0 {
        let c = bytes[i] as char;
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
            break;
        }
        i = i.saturating_sub(1);
    }
    let mut start = i;
    while start > 0 {
        let c = bytes[start - 1] as char;
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
            start -= 1;
        } else {
            break;
        }
    }
    let mut end = i;
    while end < bytes.len() {
        let c = bytes[end] as char;
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
            end += 1;
        } else {
            break;
        }
    }
    src[start..end].to_string()
}

pub fn position_to_offset(src: &str, line: usize, character: usize) -> usize {
    let mut cur_line = 0;
    let mut cur_char = 0;
    for (i, ch) in src.char_indices() {
        if cur_line == line && cur_char >= character {
            return i;
        }
        if ch == '\n' {
            cur_line += 1;
            cur_char = 0;
            if cur_line > line {
                return i;
            }
        } else {
            cur_char += ch.len_utf16();
        }
    }
    src.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_dot_completes_methods() {
        let items = completions_at("using Render;\nRender.", 1, 7);
        let labels: Vec<String> = items
            .iter()
            .filter_map(|v| v.get("label").and_then(|l| l.as_str()).map(str::to_string))
            .collect();
        assert!(
            labels
                .iter()
                .any(|l| l.contains("gpu_init") || l == "gpu_init"),
            "expected Render.gpu_* methods, got {labels:?}"
        );
        assert!(!labels.iter().any(|l| l == "cell"));
    }

    #[test]
    fn hover_catalog_path() {
        let src = "using Animation;\nAnimation.orbit_spin(t)";
        let text = hover_at(src, 1, 12);
        assert!(text.contains("orbit_spin") || text.contains("evaluate_preset"));
        assert!(text.contains("using"));
    }
}
