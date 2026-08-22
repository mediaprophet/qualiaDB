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
    "shacl_violations": { "type": "array" }
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
    }
}
