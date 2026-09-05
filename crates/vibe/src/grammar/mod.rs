//! Grammar artifacts for agents (core §3). Files under `grammar/` are the text.

pub const EBNF: &str = include_str!("../../grammar/vibe-0.1.ebnf");
pub const GBNF: &str = include_str!("../../grammar/vibe-0.1.gbnf");

pub const SOURCE_SCHEMA_JSON: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "vibe-0.1-source",
  "type": "object",
  "additionalProperties": false,
  "required": ["language", "kind", "source"],
  "properties": {
    "language": { "const": "vibe-0.1" },
    "kind": { "enum": ["cell", "module"] },
    "source": { "type": "string", "minLength": 1 },
    "locales": {
      "type": "array",
      "items": { "type": "string" }
    },
    "using": {
      "type": "array",
      "items": { "type": "string" }
    }
  }
}"#;

pub const DIAGNOSTIC_SCHEMA_JSON: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "vibe-0.1-diagnostic",
  "type": "object",
  "required": ["valid", "kind"],
  "properties": {
    "valid": { "type": "boolean" },
    "kind": { "enum": ["cell", "module"] },
    "error_code": { "type": ["string", "null"] },
    "span": { "type": ["array", "null"], "items": { "type": "integer" }, "minItems": 2, "maxItems": 2 },
    "message": { "type": ["string", "null"] },
    "suggested_fix": { "type": ["string", "null"] },
    "evidential": { "type": ["array", "null"] },
    "shacl_violations": { "type": "array" },
    "errors": { "type": "array" }
  }
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebnf_and_gbnf_are_0_1() {
        assert!(EBNF.contains("Program"));
        assert!(EBNF.contains("BindDecl"));
        assert!(EBNF.contains("LambdaExpr"));
        assert!(EBNF.contains("TweenExpr"));
        assert!(EBNF.contains("Expression     ::= Pipeline"));
        assert!(EBNF.contains("PrefixedName"));
        assert!(GBNF.contains("root"));
        assert!(GBNF.contains("bind-decl"));
        assert!(GBNF.contains("lambda"));
        assert!(GBNF.contains("tween ::="));
        assert!(GBNF.contains("expr ::= tween"));
        assert!(!GBNF.contains("<<["));
        assert!(SOURCE_SCHEMA_JSON.contains("vibe-0.1"));
        assert!(DIAGNOSTIC_SCHEMA_JSON.contains("\"errors\""));
        assert!(DIAGNOSTIC_SCHEMA_JSON.contains("evidential"));
    }

    #[test]
    fn ebnf_file_matches_vibescript_core_section_3() {
        let core = include_str!("../../../../docs/manuals/standards/vibescript-core.md")
            .replace("\r\n", "\n");
        let marker = "```ebnf\n";
        let start = core.find(marker).expect("core.md §3 fenced ebnf") + marker.len();
        let rest = &core[start..];
        let end = rest.find("```").expect("closing ebnf fence");
        let from_spec = rest[..end].trim();
        let from_file = EBNF
            .replace("\r\n", "\n")
            .lines()
            .filter(|line| !line.starts_with("(* vibe-0.1"))
            .collect::<Vec<_>>()
            .join("\n");
        let from_file = from_file.trim();
        assert_eq!(
            from_file, from_spec,
            "crates/vibe/grammar/vibe-0.1.ebnf must stay a copy of vibescript-core.md §3"
        );
    }
}
