//! Text-driven SHACL validation — the entry point the docs playground / any
//! "paste shapes + data, get a report" caller uses.
//!
//! * **Data** is N3/N-Triples text (parsed by the engine's own [`N3Parser`]).
//!   Numeric object literals are inline-encoded (so range constraints work);
//!   IRIs/strings are `q_hash`ed and retained in a resolver (so string
//!   constraints — pattern/length/language — work).
//! * **Shapes** are a compact JSON list ([`ShapeSpec`]) mapping 1:1 onto the
//!   [`ShaclConstraint`] vocabulary.
//!
//! [`validate_json`] runs the comprehensive [`ShaclEngine`] and returns the
//! [`ValidationReport`] as JSON.

use std::collections::HashMap;

use super::shacl_types::{CompiledShape, ShaclConstraint, ShaclSeverity, ValidationReport};
use super::validate::ShaclEngine;
use crate::frame_layout::{pack_float_object, INLINE_TAG_INTEGER, INLINE_VALUE_MASK};
use crate::modalities::logic::n3_parser::{N3Event, N3Parser, Term};
use crate::{q_hash, NQuin};

/// One shape in the playground/JSON input.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShapeSpec {
    /// `sh:targetClass` IRI (instances of this class are the focus nodes).
    pub target_class: String,
    /// Property path the value constraints apply to (`""` = a node shape on the
    /// focus node itself, for `class`/`nodeKind`/`closed`/logical constraints).
    #[serde(default)]
    pub path: String,
    /// `Violation` (default) | `Warning` | `Info`.
    #[serde(default)]
    pub severity: Option<String>,
    pub constraints: Vec<ConstraintSpec>,
}

/// One constraint in a [`ShapeSpec`]. Exactly the field for the `kind` is read.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ConstraintSpec {
    /// e.g. `minInclusive`, `minCount`, `class`, `datatype`, `nodeKind`,
    /// `pattern`, `in`, `hasValue`, `equals`, `lessThan`, `node`, `not`, `and`,
    /// `or`, `xone`, `languageIn`, `uniqueLang`, `closed`, `minLength`…
    pub kind: String,
    #[serde(default)]
    pub num: Option<f64>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub list: Option<Vec<String>>,
}

fn term_str<'a>(t: &Term<'a>) -> &'a str {
    match t {
        Term::Uri(s) | Term::Variable(s) | Term::Literal(s) | Term::Formula(s) => s,
    }
}

fn intern(s: &str, r: &mut HashMap<u64, String>) -> u64 {
    let h = q_hash(s);
    r.entry(h).or_insert_with(|| s.to_string());
    h
}

/// Encode a triple object: numeric literals → inline-typed value; everything
/// else → `q_hash` (retained in the resolver for string constraints).
fn encode_object(t: &Term, r: &mut HashMap<u64, String>) -> u64 {
    let s = term_str(t);
    if let Term::Literal(_) = t {
        if let Ok(i) = s.parse::<i64>() {
            return INLINE_TAG_INTEGER | ((i as u64) & INLINE_VALUE_MASK);
        }
        if let Ok(f) = s.parse::<f64>() {
            return pack_float_object(f as f32);
        }
    }
    intern(s, r)
}

/// Parse N3/N-Triples text into a quin graph plus a hash→lexical resolver.
pub fn build_graph(data: &str) -> (Vec<NQuin>, HashMap<u64, String>) {
    let mut quins = Vec::new();
    let mut resolver = HashMap::new();
    let mut parser = N3Parser::new(data);
    let _ = parser.parse_all(|ev| {
        if let N3Event::StaticTriple(t) = ev {
            let subject = intern(term_str(&t.subject), &mut resolver);
            let predicate = intern(term_str(&t.predicate), &mut resolver);
            let object = encode_object(&t.object, &mut resolver);
            quins.push(NQuin {
                subject,
                predicate,
                object,
                context: 0,
                metadata: 0,
                parity: 0,
            });
        }
        Ok(())
    });
    (quins, resolver)
}

fn severity_of(s: &Option<String>) -> ShaclSeverity {
    match s.as_deref() {
        Some("Warning") | Some("warning") => ShaclSeverity::Warning,
        Some("Info") | Some("info") => ShaclSeverity::Info,
        _ => ShaclSeverity::Violation,
    }
}

fn constraint_of(c: &ConstraintSpec) -> Option<ShaclConstraint> {
    let num = c.num.unwrap_or(0.0);
    let u = num as u32;
    let text = || c.text.clone().unwrap_or_default();
    let list = || c.list.clone().unwrap_or_default();
    Some(match c.kind.as_str() {
        "minInclusive" => ShaclConstraint::MinInclusive(num),
        "maxInclusive" => ShaclConstraint::MaxInclusive(num),
        "minExclusive" => ShaclConstraint::MinExclusive(num),
        "maxExclusive" => ShaclConstraint::MaxExclusive(num),
        "minCount" => ShaclConstraint::MinCount(u),
        "maxCount" => ShaclConstraint::MaxCount(u),
        "minLength" => ShaclConstraint::MinLength(u),
        "maxLength" => ShaclConstraint::MaxLength(u),
        "pattern" => ShaclConstraint::Pattern(text()),
        "class" => ShaclConstraint::Class(text()),
        "datatype" => ShaclConstraint::DataType(text()),
        "nodeKind" => ShaclConstraint::NodeKind(text()),
        "hasValue" => ShaclConstraint::HasValue(text()),
        "in" => ShaclConstraint::In(list()),
        "equals" => ShaclConstraint::Equals(text()),
        "lessThan" => ShaclConstraint::LessThan(text()),
        "lessThanOrEquals" => ShaclConstraint::LessThanOrEquals(text()),
        "greaterThan" => ShaclConstraint::GreaterThan(text()),
        "greaterThanOrEquals" => ShaclConstraint::GreaterThanOrEquals(text()),
        "node" => ShaclConstraint::Node(text()),
        "not" => ShaclConstraint::Not(text()),
        "and" => ShaclConstraint::And(list()),
        "or" => ShaclConstraint::Or(list()),
        "xone" => ShaclConstraint::Xone(list()),
        "languageIn" => ShaclConstraint::LanguageIn(list()),
        "uniqueLang" => ShaclConstraint::UniqueLang,
        "closed" => ShaclConstraint::Closed { ignored_properties: list() },
        _ => return None,
    })
}

/// Build a [`CompiledShape`] from a [`ShapeSpec`].
pub fn shape_from_spec(spec: &ShapeSpec) -> CompiledShape {
    let constraints: Vec<ShaclConstraint> =
        spec.constraints.iter().filter_map(constraint_of).collect();
    let mut shape =
        CompiledShape::new(spec.target_class.clone(), constraints, severity_of(&spec.severity));
    shape.property_path = spec.path.clone();
    shape
}

/// Validate N3/N-Triples `data` against `specs` and return the report.
pub fn validate_text(data: &str, specs: &[ShapeSpec]) -> ValidationReport {
    let (quins, resolver) = build_graph(data);
    let shapes: Vec<CompiledShape> = specs.iter().map(shape_from_spec).collect();
    let engine = ShaclEngine::new(&quins, &shapes);
    engine.validate(&|h| resolver.get(&h).cloned())
}

/// JSON-in / JSON-out convenience for the playground & WASM boundary.
pub fn validate_json(data: &str, shapes_json: &str) -> Result<String, String> {
    let specs: Vec<ShapeSpec> =
        serde_json::from_str(shapes_json).map_err(|e| format!("invalid shapes JSON: {e}"))?;
    let report = validate_text(data, &specs);
    serde_json::to_string(&report).map_err(|e| format!("serialize report: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_to_end_age_mininclusive() {
        let data = ":alice a :Person .\n:alice :age 15 .\n:bob a :Person .\n:bob :age 40 .";
        let shapes = r#"[
            {"targetClass":":Person","path":":age","severity":"Violation",
             "constraints":[{"kind":"minInclusive","num":18}]}
        ]"#;
        let out = validate_json(data, shapes).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["conforms"], false, "alice (15) must violate minInclusive 18");
        let results = v["results"].as_array().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["source_constraint_component"], "sh:MinInclusiveConstraintComponent");
    }

    #[test]
    fn end_to_end_pattern_with_resolver() {
        let data = r#":u a :User . :u :email "alice@example.org" ."#;
        let shapes = r#"[
            {"targetClass":":User","path":":email",
             "constraints":[{"kind":"pattern","text":"^[^@]+@[^@]+\\.[a-z]+$"}]}
        ]"#;
        let out = validate_json(data, shapes).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["conforms"], true, "a well-formed email matches the pattern");
    }

    #[test]
    fn end_to_end_class_and_node_kind() {
        // pet must be an instance of :Dog; here it's a :Cat → violation.
        let data = ":o a :Owner . :o :pet :rex . :rex a :Cat .";
        let shapes = r#"[
            {"targetClass":":Owner","path":":pet",
             "constraints":[{"kind":"class","text":":Dog"}]}
        ]"#;
        let v: serde_json::Value = serde_json::from_str(&validate_json(data, shapes).unwrap()).unwrap();
        assert_eq!(v["conforms"], false);
    }

    #[test]
    fn invalid_json_is_reported() {
        assert!(validate_json(":a :b :c .", "{not json").is_err());
    }
}
