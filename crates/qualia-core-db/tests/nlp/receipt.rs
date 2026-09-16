//! JSON and Markdown evaluation receipts (NLP-004).
//!
//! Semantic receipts are timestamp-free: `clock` is a dummy instant so CI
//! diffs stay stable. Do not embed source text.

use super::manifest::{DUMMY_CLOCK, FROZEN_EVAL_SEED, SOURCE_REVISION_UNCOMMITTED};

pub const SUITE_ID: &str = "nlp-004";
pub const SUITE_COMMAND: &str = "cargo test -p qualia-core-db --test nlp_suite";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalReceipt {
    pub suite_id: String,
    pub source_revision: String,
    pub command: String,
    pub passed: u32,
    pub failed: u32,
    pub clock: String,
    pub seed: u64,
    pub artifact_paths: Vec<String>,
    pub notes: Vec<String>,
}

impl EvalReceipt {
    pub fn scaffold(passed: u32, failed: u32, artifact_paths: Vec<String>) -> Self {
        Self {
            suite_id: SUITE_ID.into(),
            source_revision: SOURCE_REVISION_UNCOMMITTED.into(),
            command: SUITE_COMMAND.into(),
            passed,
            failed,
            clock: DUMMY_CLOCK.into(),
            seed: FROZEN_EVAL_SEED,
            artifact_paths,
            notes: vec![
                "In-process plumbing counts, not F1/accuracy.".into(),
                "Dummy clock; semantic receipt is timestamp-free.".into(),
            ],
        }
    }

    pub fn to_json(&self) -> String {
        let mut out = String::from("{\n");
        push_str_field(&mut out, "suite_id", &self.suite_id, true);
        push_str_field(&mut out, "source_revision", &self.source_revision, true);
        push_str_field(&mut out, "command", &self.command, true);
        out.push_str(&format!("  \"passed\": {},\n", self.passed));
        out.push_str(&format!("  \"failed\": {},\n", self.failed));
        push_str_field(&mut out, "clock", &self.clock, true);
        out.push_str(&format!("  \"seed\": {},\n", self.seed));
        out.push_str("  \"artifact_paths\": [\n");
        for (i, p) in self.artifact_paths.iter().enumerate() {
            let comma = if i + 1 == self.artifact_paths.len() {
                ""
            } else {
                ","
            };
            out.push_str(&format!("    \"{}\"{comma}\n", json_escape(p)));
        }
        out.push_str("  ],\n");
        out.push_str("  \"notes\": [\n");
        for (i, n) in self.notes.iter().enumerate() {
            let comma = if i + 1 == self.notes.len() { "" } else { "," };
            out.push_str(&format!("    \"{}\"{comma}\n", json_escape(n)));
        }
        out.push_str("  ]\n}\n");
        out
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::from("# NLP evaluation receipt\n\n");
        md.push_str(&format!("- suite_id: {}\n", self.suite_id));
        md.push_str(&format!("- source_revision: {}\n", self.source_revision));
        md.push_str(&format!("- command: `{}`\n", self.command));
        md.push_str(&format!("- passed: {}\n", self.passed));
        md.push_str(&format!("- failed: {}\n", self.failed));
        md.push_str(&format!("- clock: {}\n", self.clock));
        md.push_str(&format!("- seed: {}\n", self.seed));
        md.push_str("- artifact_paths:\n");
        for p in &self.artifact_paths {
            md.push_str(&format!("  - `{p}`\n"));
        }
        md.push_str("- notes:\n");
        for n in &self.notes {
            md.push_str(&format!("  - {n}\n"));
        }
        md
    }
}

fn push_str_field(out: &mut String, key: &str, value: &str, trailing_comma: bool) {
    let comma = if trailing_comma { "," } else { "" };
    out.push_str(&format!("  \"{key}\": \"{}\"{comma}\n", json_escape(value)));
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

/// Validate a receipt object parsed from JSON (in-process; no repo-root writes).
pub fn receipt_from_json(json_text: &str) -> Result<EvalReceipt, String> {
    use super::json_lite::{parse_json, Json};

    let root = parse_json(json_text).map_err(|e| e.0)?;
    let obj = root.as_object().ok_or("receipt must be a JSON object")?;
    let req_str = |key: &str| -> Result<String, String> {
        obj.get(key)
            .and_then(Json::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("receipt missing string {key}"))
    };
    let req_u32 = |key: &str| -> Result<u32, String> {
        obj.get(key)
            .and_then(Json::as_i64)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| format!("receipt missing u32 {key}"))
    };
    let str_array = |key: &str| -> Result<Vec<String>, String> {
        let items = obj
            .get(key)
            .and_then(Json::as_array)
            .ok_or_else(|| format!("receipt missing array {key}"))?;
        let mut out = Vec::new();
        for item in items {
            out.push(
                item.as_str()
                    .ok_or_else(|| format!("{key} entries must be strings"))?
                    .to_string(),
            );
        }
        Ok(out)
    };

    Ok(EvalReceipt {
        suite_id: req_str("suite_id")?,
        source_revision: req_str("source_revision")?,
        command: req_str("command")?,
        passed: req_u32("passed")?,
        failed: req_u32("failed")?,
        clock: req_str("clock")?,
        seed: obj
            .get("seed")
            .and_then(Json::as_i64)
            .and_then(|n| u64::try_from(n).ok())
            .ok_or("receipt missing seed")?,
        artifact_paths: str_array("artifact_paths")?,
        notes: str_array("notes")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::json_lite::parse_json;

    const FIXTURE_JSON: &str = include_str!("fixtures/receipt_dummy.json");
    const FIXTURE_MD: &str = include_str!("fixtures/receipt_dummy.md");

    #[test]
    fn json_and_markdown_receipt_shape_is_stable() {
        let receipt = EvalReceipt::scaffold(
            2,
            0,
            vec![
                "benchmarks/nlp/manifests/nlp-004-smoke-tokenize-en.json".into(),
                "benchmarks/nlp/manifests/nlp-004-ud-en-ewt-unavailable.json".into(),
            ],
        );
        let json = receipt.to_json();
        assert_eq!(json, FIXTURE_JSON.replace("\r\n", "\n"));
        assert_eq!(receipt.to_markdown(), FIXTURE_MD.replace("\r\n", "\n"));
        let parsed = receipt_from_json(&json).expect("round-trip JSON");
        assert_eq!(parsed, receipt);
        assert_eq!(parsed.suite_id, SUITE_ID);
        assert_eq!(parsed.source_revision, SOURCE_REVISION_UNCOMMITTED);
        assert_eq!(parsed.command, SUITE_COMMAND);
        assert_eq!(parsed.clock, DUMMY_CLOCK);
        assert_eq!(parsed.seed, FROZEN_EVAL_SEED);
        assert_eq!(parsed.passed, 2);
        assert_eq!(parsed.failed, 0);
        assert_eq!(parsed.artifact_paths.len(), 2);

        let root = parse_json(&json).unwrap();
        assert!(root.field("suite_id").is_some());
        assert!(root.field("source_revision").is_some());
        assert!(root.field("command").is_some());
        assert!(root.field("passed").is_some());
        assert!(root.field("failed").is_some());
        assert!(root.field("artifact_paths").is_some());

        let md = receipt.to_markdown();
        assert!(md.starts_with("# NLP evaluation receipt"));
        assert!(md.contains("suite_id: nlp-004"));
        assert!(md.contains("source_revision: uncommitted"));
        assert!(md.contains("passed: 2"));
        assert!(md.contains("failed: 0"));
        assert!(md.contains(DUMMY_CLOCK));
        assert!(md.contains("nlp-004-smoke-tokenize-en.json"));
    }
}
