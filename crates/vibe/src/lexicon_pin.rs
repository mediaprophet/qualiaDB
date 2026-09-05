//! Lexicon pack pin + diagnose hooks (G-LEXICON-0).
//!
//! `lexicon:` is **not** an EBNF production. The pin is package metadata:
//! a comment/header `// lexicon: "packId@SemVer"` and/or
//! `const lexicon = "packId@SemVer";`.
//!
//! [`diagnose`](crate::diagnose) stays parse+check only (no disk, no invoke).
//! Missing / unknown pack at the `GraphDatabase.lexicon_manifest` seam is
//! documented here as DiagnoseReport JSON: **held / not yet** (`E300`) +
//! `suggested_fix` that names “open lexicon pack”. Alias rows ride in
//! `suggested_fix` as JSON `{from, to, framing}`. Living framing is never
//! rewritten as artifact on upgrade.
//!
//! No Host widen. No dotted `qualia.*`. No in-binary WordNet.

use crate::diagnose::DiagnoseReport;
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;

/// Soft why-text for missing / unknown pack. Never "broken".
pub const HELD_OPEN_PACK: &str = "held / not yet — open lexicon pack";

/// Example pin matching `docs/manuals/standards/lexicon-pack-manifest-example.json`.
pub const EXAMPLE_PIN: &str = "en-core@0.1.0";

/// Live bind id (already in `ALL_BOUND`). Catalog-honest; not a new Host method.
pub const LEXICON_MANIFEST_ID: &str = "GraphDatabase.lexicon_manifest";

/// Recorded `lexicon: "packId@SemVer"` pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexiconPin {
    pub pack_id: String,
    pub pack_semver: String,
}

impl LexiconPin {
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim().trim_matches('"').trim_end_matches(';').trim();
        if raw.is_empty() {
            return None;
        }
        let (pack_id, pack_semver) = raw.split_once('@')?;
        let pack_id = pack_id.trim();
        let pack_semver = pack_semver.trim();
        if pack_id.is_empty() || !looks_like_semver(pack_semver) {
            return None;
        }
        Some(Self {
            pack_id: pack_id.to_string(),
            pack_semver: pack_semver.to_string(),
        })
    }

    pub fn as_pin_str(&self) -> String {
        format!("{}@{}", self.pack_id, self.pack_semver)
    }
}

fn looks_like_semver(s: &str) -> bool {
    let mut parts = s.split('.');
    let major = parts.next().unwrap_or("");
    let minor = parts.next().unwrap_or("");
    let patch = parts.next().unwrap_or("");
    !major.is_empty()
        && major.chars().all(|c| c.is_ascii_digit())
        && !minor.is_empty()
        && minor.chars().all(|c| c.is_ascii_digit())
        && !patch.is_empty()
        && patch.chars().next().is_some_and(|c| c.is_ascii_digit())
}

/// Pack framing — living senses must not be Thing-washed as artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexiconFraming {
    LivingShacl,
    ArtifactOwl,
    Mixed,
}

impl LexiconFraming {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LivingShacl => "living-SHACL",
            Self::ArtifactOwl => "artifact-OWL",
            Self::Mixed => "mixed",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "living-SHACL" | "living_shacl" | "living" => Some(Self::LivingShacl),
            "artifact-OWL" | "artifact_owl" | "artifact" => Some(Self::ArtifactOwl),
            "mixed" => Some(Self::Mixed),
            _ => None,
        }
    }
}

/// Alias row for migrate / `suggested_fix` (shape doc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasRow {
    pub from: String,
    pub to: String,
    pub framing: LexiconFraming,
}

impl AliasRow {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"from\":\"{}\",\"to\":\"{}\",\"framing\":\"{}\"}}",
            escape_json(&self.from),
            escape_json(&self.to),
            self.framing.as_str()
        )
    }
}

/// Scan comment/header `lexicon: "packId@SemVer"` and `const lexicon = "…"`.
pub fn parse_lexicon_pin_from_source(src: &str) -> Option<LexiconPin> {
    for line in src.lines() {
        let t = trim_line_comment(line);
        if let Some(pin) = extract_lexicon_pragma(t) {
            return Some(pin);
        }
        if let Some(pin) = extract_const_lexicon(t) {
            return Some(pin);
        }
    }
    None
}

fn trim_line_comment(line: &str) -> &str {
    let t = line.trim();
    t.strip_prefix("//").map(str::trim).unwrap_or(t)
}

fn extract_lexicon_pragma(t: &str) -> Option<LexiconPin> {
    let rest = t.strip_prefix("lexicon:")?.trim();
    LexiconPin::parse(rest)
}

fn extract_const_lexicon(t: &str) -> Option<LexiconPin> {
    let rest = t.strip_prefix("const")?.trim();
    let rest = rest.strip_prefix("lexicon")?.trim();
    let rest = rest.strip_prefix('=')?.trim();
    LexiconPin::parse(rest)
}

/// Expected DiagnoseReport when `lexicon_manifest` is missing / unknown.
///
/// `diagnose()` does not invoke the host. Neo maps the live E300 onto this shape.
pub fn missing_pack_report(span: Span, message: impl Into<String>) -> DiagnoseReport {
    let message = message.into();
    let error = Diagnostic::new(DiagCode::E300, span, message).with_fix(HELD_OPEN_PACK);
    DiagnoseReport {
        valid: false,
        kind: "module",
        errors: vec![error.clone()],
        error: Some(error),
    }
}

/// Pin-aware diagnose hook for Neo (no disk).
///
/// `pack_available = false` → held / not yet + open-pack `suggested_fix`.
/// `pack_available = true` → valid module; caller records the pin separately.
pub fn diagnose_lexicon_pin(src: &str, pack_available: bool) -> DiagnoseReport {
    match parse_lexicon_pin_from_source(src) {
        None => DiagnoseReport {
            valid: true,
            kind: "module",
            error: None,
            errors: Vec::new(),
        },
        Some(_) if pack_available => DiagnoseReport {
            valid: true,
            kind: "module",
            error: None,
            errors: Vec::new(),
        },
        Some(pin) => missing_pack_report(
            Span::point(0),
            format!("lexicon pack not found (pin {})", pin.as_pin_str()),
        ),
    }
}

/// Encode alias rows as `suggested_fix` JSON array.
pub fn alias_rows_suggested_fix(rows: &[AliasRow]) -> String {
    let mut s = String::from("[");
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&row.to_json());
    }
    s.push(']');
    s
}

/// Round-trip alias rows out of a `suggested_fix` JSON array (or mixed text).
pub fn parse_alias_rows_json(src: &str) -> Vec<AliasRow> {
    let mut rows = Vec::new();
    let mut rest = src;
    while let Some(start) = rest.find('{') {
        let from_here = &rest[start..];
        let Some(end) = from_here.find('}') else {
            break;
        };
        let obj = &from_here[..=end];
        if let (Some(from), Some(to), Some(fr)) = (
            json_string_field(obj, "from"),
            json_string_field(obj, "to"),
            json_string_field(obj, "framing"),
        ) {
            if let Some(framing) = LexiconFraming::parse(&fr) {
                rows.push(AliasRow { from, to, framing });
            }
        }
        rest = &from_here[1..];
    }
    rows
}

/// Held report whose `suggested_fix` is alias-row JSON (migrate / upgrade).
pub fn alias_migrate_report(span: Span, rows: &[AliasRow]) -> DiagnoseReport {
    let fix = alias_rows_suggested_fix(rows);
    let error = Diagnostic::new(
        DiagCode::E300,
        span,
        "lexicon pack upgrade held — alias map required",
    )
    .with_fix(fix);
    DiagnoseReport {
        valid: false,
        kind: "module",
        errors: vec![error.clone()],
        error: Some(error),
    }
}

/// Upgrade remap: living-SHACL never becomes artifact-OWL.
pub fn apply_upgrade_framing(
    existing: LexiconFraming,
    requested: LexiconFraming,
) -> LexiconFraming {
    if existing == LexiconFraming::LivingShacl && requested == LexiconFraming::ArtifactOwl {
        LexiconFraming::LivingShacl
    } else {
        requested
    }
}

pub fn apply_upgrade_map(rows: &[AliasRow], requested: &[LexiconFraming]) -> Vec<AliasRow> {
    rows.iter()
        .zip(requested.iter().copied().chain(std::iter::repeat(
            requested.last().copied().unwrap_or(LexiconFraming::Mixed),
        )))
        .map(|(row, req)| AliasRow {
            from: row.from.clone(),
            to: row.to.clone(),
            framing: apply_upgrade_framing(row.framing, req),
        })
        .collect()
}

pub fn living_rewritten_as_artifact(before: &[AliasRow], after: &[AliasRow]) -> bool {
    before.iter().any(|old| {
        old.framing == LexiconFraming::LivingShacl
            && after
                .iter()
                .any(|new| new.from == old.from && new.framing == LexiconFraming::ArtifactOwl)
    })
}

fn json_string_field(obj: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":");
    let idx = obj.find(&needle)?;
    let rest = obj[idx + needle.len()..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            }
            '"' => break,
            c => out.push(c),
        }
    }
    Some(out)
}

fn escape_json(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out
}

/// Minimal pack-manifest field read (fixture / example JSON; no serde).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackManifest {
    pub pack_id: String,
    pub pack_semver: String,
    pub framing: LexiconFraming,
    pub uplift_from: String,
    pub concept_ids: Vec<String>,
}

pub fn parse_pack_manifest_json(raw: &str) -> Option<PackManifest> {
    let pack_id = json_string_field(raw, "packId")?;
    let pack_semver = json_string_field(raw, "packSemVer")?;
    let framing = LexiconFraming::parse(&json_string_field(raw, "framing")?)?;
    let uplift_from = json_string_field(raw, "upliftFrom").unwrap_or_default();
    let concept_ids = parse_string_array_field(raw, "conceptIds");
    Some(PackManifest {
        pack_id,
        pack_semver,
        framing,
        uplift_from,
        concept_ids,
    })
}

fn parse_string_array_field(raw: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\":");
    let Some(idx) = raw.find(&needle) else {
        return Vec::new();
    };
    let rest = raw[idx + needle.len()..].trim_start();
    let Some(rest) = rest.strip_prefix('[') else {
        return Vec::new();
    };
    let end = rest.find(']').unwrap_or(rest.len());
    rest[..end]
        .split(',')
        .filter_map(|part| {
            let s = part.trim().trim_matches('"');
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_parses_en_core() {
        let pin = LexiconPin::parse(EXAMPLE_PIN).unwrap();
        assert_eq!(pin.pack_id, "en-core");
        assert_eq!(pin.pack_semver, "0.1.0");
        assert_eq!(pin.as_pin_str(), EXAMPLE_PIN);
    }

    #[test]
    fn held_voice_never_says_broken() {
        let r = missing_pack_report(Span::point(0), "lexicon pack not found");
        let json = r.to_json();
        assert!(!r.valid);
        assert_eq!(r.error.as_ref().unwrap().code, DiagCode::E300);
        assert!(json.contains("\"error_code\":\"E300\""));
        assert!(json.contains("held / not yet"));
        assert!(json.contains("open lexicon pack"));
        assert!(!json.to_ascii_lowercase().contains("broken"));
        assert!(json.contains("\"suggested_fix\""));
        assert!(json.contains("\"errors\":["));
    }

    #[test]
    fn alias_rows_round_trip() {
        let rows = [
            AliasRow {
                from: "arrive".into(),
                to: "concept:arrive".into(),
                framing: LexiconFraming::LivingShacl,
            },
            AliasRow {
                from: "volume".into(),
                to: "concept:volume".into(),
                framing: LexiconFraming::ArtifactOwl,
            },
        ];
        let fix = alias_rows_suggested_fix(&rows);
        let back = parse_alias_rows_json(&fix);
        assert_eq!(back, rows);
    }

    #[test]
    fn living_not_rewritten_as_artifact() {
        let before = [AliasRow {
            from: "person".into(),
            to: "concept:person".into(),
            framing: LexiconFraming::LivingShacl,
        }];
        let after = apply_upgrade_map(&before, &[LexiconFraming::ArtifactOwl]);
        assert_eq!(after[0].framing, LexiconFraming::LivingShacl);
        assert!(!living_rewritten_as_artifact(&before, &after));
    }
}
