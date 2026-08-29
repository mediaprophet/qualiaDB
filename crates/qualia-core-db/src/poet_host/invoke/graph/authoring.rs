//! RDF-Star and ontology authoring adapters for POET.

use super::super::args;
use crate::q_hash;
use crate::sparql_library::rdf_formats::{parse_rdf, QuinCollector, RdfFormat};
use base64::Engine as _;
use std::io::Cursor;
use vibe::{Diagnostic, Span, Value};

const MAX_SOURCE_BYTES: usize = 256 * 1024;

pub fn author(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let source = args::rec_str(args_v, "source")
        .ok_or_else(|| args::bad(span, "GraphAuthoring.process needs source"))?;
    if source.is_empty() || source.len() > MAX_SOURCE_BYTES {
        return Err(args::bad(
            span,
            "source must contain 1..=262144 UTF-8 bytes",
        ));
    }
    if let Some(message) = super::personhood::owl_person_source_violation(source) {
        return Err(args::bad(span, message));
    }
    let mode = args::rec_str(args_v, "mode").unwrap_or("rdfstar_resolve");
    let format = RdfFormat::from_str(args::rec_str(args_v, "format").unwrap_or("turtle"))
        .ok_or_else(|| args::bad(span, "unsupported RDF format"))?;
    let context = q_hash(args::rec_str(args_v, "context").unwrap_or("urn:poet:authoring"));
    let mut collector = Box::new(QuinCollector::new());
    let parsed = parse_rdf(
        format,
        Cursor::new(source.as_bytes()),
        context,
        &mut collector,
    )
    .map_err(|error| args::bad(span, format!("RDF parse failed: {error}")))?;
    match mode {
        "rdfstar_resolve" => resolve(source, parsed, collector.as_slice()),
        "ontology_compile" => compile(args_v, parsed, collector.as_slice(), span),
        "ontology_validate" => validate(parsed, collector.as_slice()),
        _ => Err(args::bad(
            span,
            format!("unknown graph-authoring mode `{mode}`"),
        )),
    }
}

fn resolve(source: &str, parsed: u64, quins: &[crate::NQuin]) -> Result<Value, Diagnostic> {
    let quoted = source.lines().filter(|line| line.contains("<<")).count() as u64;
    let statements = quins
        .iter()
        .take(32)
        .map(|quin| {
            args::record([
                ("subject_hash", Value::U64(quin.subject)),
                ("predicate_hash", Value::U64(quin.predicate)),
                ("object_hash", Value::U64(quin.object)),
                ("context_hash", Value::U64(quin.context)),
            ])
        })
        .collect();
    Ok(args::record([
        ("format", Value::String("rdf-star".into())),
        ("statement_count", Value::U64(parsed)),
        ("quoted_source_lines", Value::U64(quoted)),
        ("preview_truncated", Value::Bool(quins.len() > 32)),
        ("statements", Value::List(statements)),
    ]))
}

fn compile(
    args_v: &Value,
    parsed: u64,
    quins: &[crate::NQuin],
    span: Span,
) -> Result<Value, Diagnostic> {
    let mut cbor = Vec::new();
    crate::sparql_library::serialisers::rdf_serializers::serialize_to_cborld(&mut cbor, quins)
        .map_err(|error| args::bad(span, error))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&cbor);
    Ok(args::record([
        ("format", Value::String("application/cbor-ld".into())),
        (
            "prefix",
            Value::String(args::rec_str(args_v, "prefix").unwrap_or("").into()),
        ),
        (
            "namespace",
            Value::String(args::rec_str(args_v, "namespace").unwrap_or("").into()),
        ),
        ("statement_count", Value::U64(parsed)),
        ("byte_count", Value::U64(cbor.len() as u64)),
        ("cbor_ld_base64", Value::String(encoded)),
    ]))
}

fn validate(parsed: u64, quins: &[crate::NQuin]) -> Result<Value, Diagnostic> {
    let rdf_type = q_hash("rdf:type");
    let turtle_type = q_hash("a");
    let owl_class = q_hash("owl:Class");
    let rdfs_class = q_hash("rdfs:Class");
    let node_shape = q_hash("sh:NodeShape");
    let owl_object_property = q_hash("owl:ObjectProperty");
    let rdf_property = q_hash("rdf:Property");
    let typed = |object: u64| {
        quins
            .iter()
            .filter(|q| {
                (q.predicate == rdf_type || q.predicate == turtle_type) && q.object == object
            })
            .count()
    };
    Ok(args::record([
        ("structurally_valid", Value::Bool(parsed > 0)),
        ("statement_count", Value::U64(parsed)),
        (
            "declared_rdfs_classes",
            Value::U64(typed(rdfs_class) as u64),
        ),
        ("declared_node_shapes", Value::U64(typed(node_shape) as u64)),
        ("declared_owl_classes", Value::U64(typed(owl_class) as u64)),
        (
            "declared_object_properties",
            Value::U64((typed(owl_object_property) + typed(rdf_property)) as u64),
        ),
        (
            "personhood",
            Value::String(
                "natural persons use rdfs:Class + SHACL/ShEx; owl:Thing is rejected for persons"
                    .into(),
            ),
        ),
        (
            "validation_scope",
            Value::String(
                "bounded RDF/RDFS/SHACL structural validation; OWL class counts are artifacts-only"
                    .into(),
            ),
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_turtle_to_real_cbor_ld() {
        let args_v = args::record([
            ("mode", Value::String("ontology_compile".into())),
            (
                "source",
                Value::String("<urn:A> <rdf:type> <owl:Class> .".into()),
            ),
            ("format", Value::String("turtle".into())),
        ]);
        match author(&args_v, Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => {
                assert!(matches!(r.get("byte_count"), Some(Value::U64(n)) if *n > 0))
            }
            _ => panic!(),
        }
    }

    #[test]
    fn rejects_owl_class_typing_of_a_natural_person() {
        let args_v = args::record([
            ("mode", Value::String("ontology_validate".into())),
            (
                "source",
                Value::String("coop:Contributor a owl:Class ; rdfs:subClassOf soc:Person .".into()),
            ),
            ("format", Value::String("turtle".into())),
        ]);
        assert!(author(&args_v, Span { start: 0, end: 0 }).is_err());
    }

    #[test]
    fn accepts_rdfs_principal_and_shacl_not_owl_thing() {
        let args_v = args::record([
            ("mode", Value::String("ontology_validate".into())),
            (
                "source",
                Value::String(
                    "q42:Principal a rdfs:Class .\nq42:PrincipalShape a sh:NodeShape ; sh:not [ sh:class owl:Thing ] .".into(),
                ),
            ),
            ("format", Value::String("turtle".into())),
        ]);
        assert!(author(&args_v, Span { start: 0, end: 0 }).is_ok());
    }
}
