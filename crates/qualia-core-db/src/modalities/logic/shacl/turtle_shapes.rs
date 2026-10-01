//! Compile RDF Turtle / N3 SHACL shapes into [`CompiledShape`] (UE-050).
//!
//! Prefer JSON ShapeSpec for Node/Civics admission (already complete). This
//! path admits classical `sh:NodeShape` documents so ontology authors can keep
//! Turtle sources. Parsing reuses [`super::text_input::build_graph`]; only the
//! SHACL core + Qualia modality predicates listed below are mapped — unknown
//! constraint predicates are skipped (fail-open on compile, fail-closed later
//! if the shape has zero usable constraints).

use std::collections::{HashMap, HashSet};

use super::shacl_types::{CompiledShape, ShaclConstraint, ShaclSeverity};
use super::text_input::build_graph;
use crate::{q_hash, NQuin};

/// Predicates we recognise (prefixed and absolute IRI forms).
fn pred_aliases(local: &str) -> Vec<u64> {
    let full = format!("http://www.w3.org/ns/shacl#{local}");
    let short = format!("sh:{local}");
    vec![q_hash(&full), q_hash(&short), q_hash(local)]
}

fn is_pred(p: u64, local: &str) -> bool {
    pred_aliases(local).contains(&p)
}

fn rdf_type() -> u64 {
    // N3 `a` keyword is kept as bare `a` by the parser.
    q_hash("a")
}

fn is_node_shape_type(o: u64, resolve: &HashMap<u64, String>) -> bool {
    if let Some(s) = resolve.get(&o) {
        return s == "sh:NodeShape"
            || s.ends_with("#NodeShape")
            || s.ends_with(":NodeShape")
            || s == "NodeShape";
    }
    pred_aliases("NodeShape").contains(&o)
}

fn lexical(h: u64, resolve: &HashMap<u64, String>) -> String {
    resolve
        .get(&h)
        .cloned()
        .unwrap_or_else(|| format!("node:{h:016x}"))
}

fn object_f64(object: u64, resolve: &HashMap<u64, String>) -> Option<f64> {
    if let Some(s) = resolve.get(&object) {
        if let Ok(i) = s.parse::<i64>() {
            return Some(i as f64);
        }
        if let Ok(f) = s.parse::<f64>() {
            return Some(f);
        }
    }
    crate::modalities::logic::shacl::validate::object_as_f64(object)
}

fn objects_of<'a>(quins: &'a [NQuin], subject: u64, pred_local: &str) -> Vec<&'a NQuin> {
    quins
        .iter()
        .filter(|q| q.subject == subject && is_pred(q.predicate, pred_local))
        .collect()
}

fn first_object(quins: &[NQuin], subject: u64, pred_local: &str) -> Option<u64> {
    objects_of(quins, subject, pred_local)
        .first()
        .map(|q| q.object)
}

fn constraint_from_property(
    quins: &[NQuin],
    prop_node: u64,
    resolve: &HashMap<u64, String>,
) -> Vec<ShaclConstraint> {
    let mut out = Vec::new();
    for q in quins.iter().filter(|q| q.subject == prop_node) {
        let Some(pname) = resolve.get(&q.predicate) else {
            // Still try well-known hashes without lexical.
            push_constraint_by_hash(q.predicate, q.object, resolve, &mut out);
            continue;
        };
        let local = pname.rsplit(['#', ':', '/']).next().unwrap_or(pname.as_str());
        push_constraint(local, q.object, resolve, &mut out);
    }
    out
}

fn push_constraint_by_hash(
    pred: u64,
    object: u64,
    resolve: &HashMap<u64, String>,
    out: &mut Vec<ShaclConstraint>,
) {
    for local in [
        "minInclusive",
        "maxInclusive",
        "minExclusive",
        "maxExclusive",
        "minCount",
        "maxCount",
        "minLength",
        "maxLength",
        "pattern",
        "class",
        "datatype",
        "nodeKind",
        "hasValue",
        "datatype",
    ] {
        if is_pred(pred, local) {
            push_constraint(local, object, resolve, out);
            return;
        }
    }
}

fn push_constraint(
    local: &str,
    object: u64,
    resolve: &HashMap<u64, String>,
    out: &mut Vec<ShaclConstraint>,
) {
    let text = || lexical(object, resolve);
    let num = || object_f64(object, resolve).unwrap_or(0.0);
    let u = || num() as u32;
    match local {
        "minInclusive" => out.push(ShaclConstraint::MinInclusive(num())),
        "maxInclusive" => out.push(ShaclConstraint::MaxInclusive(num())),
        "minExclusive" => out.push(ShaclConstraint::MinExclusive(num())),
        "maxExclusive" => out.push(ShaclConstraint::MaxExclusive(num())),
        "minCount" => out.push(ShaclConstraint::MinCount(u())),
        "maxCount" => out.push(ShaclConstraint::MaxCount(u())),
        "minLength" => out.push(ShaclConstraint::MinLength(u())),
        "maxLength" => out.push(ShaclConstraint::MaxLength(u())),
        "pattern" => out.push(ShaclConstraint::Pattern(text())),
        "class" => out.push(ShaclConstraint::Class(text())),
        "datatype" => out.push(ShaclConstraint::DataType(text())),
        "nodeKind" => out.push(ShaclConstraint::NodeKind(text())),
        "hasValue" => out.push(ShaclConstraint::HasValue(text())),
        "equals" => out.push(ShaclConstraint::Equals(text())),
        "lessThan" => out.push(ShaclConstraint::LessThan(text())),
        "lessThanOrEquals" => out.push(ShaclConstraint::LessThanOrEquals(text())),
        "greaterThan" => out.push(ShaclConstraint::GreaterThan(text())),
        "greaterThanOrEquals" => out.push(ShaclConstraint::GreaterThanOrEquals(text())),
        "node" => out.push(ShaclConstraint::Node(text())),
        "not" => out.push(ShaclConstraint::Not(text())),
        "uniqueLang" => out.push(ShaclConstraint::UniqueLang),
        "deonticObligate" | "DeonticObligate" => out.push(ShaclConstraint::DeonticObligate),
        "deonticPermit" | "DeonticPermit" => out.push(ShaclConstraint::DeonticPermit),
        "deonticForbid" | "DeonticForbid" => out.push(ShaclConstraint::DeonticForbid),
        "valuesConsentNonCoerced" | "ValuesConsentNonCoerced" => {
            let n = num();
            out.push(ShaclConstraint::ValuesConsentNonCoerced {
                max_imbalance: if n.is_finite() && n > 0.0 { n } else { 0.5 },
            })
        }
        "valuesHarmBelowCeiling" | "ValuesHarmBelowCeiling" => {
            out.push(ShaclConstraint::ValuesHarmBelowCeiling { max_harm: num() })
        }
        "path" | "targetClass" | "property" | "severity" | "name" | "description" => {}
        _ => {}
    }
}

/// Compile a Turtle / N3 SHACL document into [`CompiledShape`] values.
///
/// Returns an error when no NodeShape / targetClass shape can be recovered.
pub fn shapes_from_turtle(turtle: &str) -> Result<Vec<CompiledShape>, String> {
    let (quins, resolve) = build_graph(turtle);
    if quins.is_empty() {
        return Err("empty SHACL Turtle / N3 document".into());
    }

    let mut shape_nodes: HashSet<u64> = HashSet::new();
    for q in &quins {
        if q.predicate == rdf_type() && is_node_shape_type(q.object, &resolve) {
            shape_nodes.insert(q.subject);
        }
        if is_pred(q.predicate, "targetClass") {
            shape_nodes.insert(q.subject);
        }
    }
    if shape_nodes.is_empty() {
        return Err("no sh:NodeShape / sh:targetClass found".into());
    }

    let mut shapes = Vec::new();
    for shape_id in shape_nodes {
        let target = first_object(&quins, shape_id, "targetClass")
            .map(|o| lexical(o, &resolve))
            .unwrap_or_else(|| lexical(shape_id, &resolve));

        let severity = match first_object(&quins, shape_id, "severity")
            .and_then(|o| resolve.get(&o).map(|s| s.as_str()))
        {
            Some(s) if s.contains("Warning") || s.ends_with("warning") => ShaclSeverity::Warning,
            Some(s) if s.contains("Info") || s.ends_with("info") => ShaclSeverity::Info,
            _ => ShaclSeverity::Violation,
        };

        let prop_nodes: Vec<u64> = objects_of(&quins, shape_id, "property")
            .into_iter()
            .map(|q| q.object)
            .collect();

        let mut emitted = 0usize;
        for prop in &prop_nodes {
            let path = first_object(&quins, *prop, "path")
                .map(|o| lexical(o, &resolve))
                .unwrap_or_default();
            let constraints = constraint_from_property(&quins, *prop, &resolve);
            if constraints.is_empty() {
                continue;
            }
            let mut shape = CompiledShape::new(target.clone(), constraints, severity);
            shape.property_path = path;
            shape.name = Some(lexical(shape_id, &resolve));
            shapes.push(shape);
            emitted += 1;
        }

        if emitted == 0 {
            // Node shape with direct constraints (no sh:property).
            let constraints = constraint_from_property(&quins, shape_id, &resolve);
            if constraints.is_empty() {
                continue;
            }
            let mut shape = CompiledShape::new(target, constraints, severity);
            shape.property_path = String::new();
            shape.name = Some(lexical(shape_id, &resolve));
            shapes.push(shape);
        }
    }

    if shapes.is_empty() {
        return Err("SHACL Turtle compiled to zero usable constraints".into());
    }
    Ok(shapes)
}

/// Validate N3/N-Triples `data` against shapes compiled from Turtle `shapes_ttl`.
pub fn validate_turtle_shapes(data: &str, shapes_ttl: &str) -> Result<super::shacl_types::ValidationReport, String> {
    let shapes = shapes_from_turtle(shapes_ttl)?;
    let (quins, resolver) = build_graph(data);
    let engine = super::validate::ShaclEngine::new(&quins, &shapes);
    Ok(engine.validate(&|h| resolver.get(&h).cloned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turtle_nodeshape_mininclusive_rejects_underage() {
        let shapes = r#"
:PersonShape a sh:NodeShape .
:PersonShape sh:targetClass :Person .
:PersonShape sh:property :ageProp .
:ageProp sh:path :age .
:ageProp sh:minInclusive 18 .
"#;
        let compiled = shapes_from_turtle(shapes).expect("compile");
        assert_eq!(compiled.len(), 1);
        assert_eq!(compiled[0].shape_class, ":Person");
        assert_eq!(compiled[0].property_path, ":age");
        assert!(matches!(
            compiled[0].constraints[0],
            ShaclConstraint::MinInclusive(v) if (v - 18.0).abs() < 1e-9
        ));

        let data = ":alice a :Person .\n:alice :age 15 .\n:bob a :Person .\n:bob :age 40 .";
        let report = validate_turtle_shapes(data, shapes).expect("validate");
        assert!(!report.conforms, "15 < 18 must violate");
    }

    #[test]
    fn empty_turtle_fails_closed() {
        assert!(shapes_from_turtle("").is_err());
        assert!(shapes_from_turtle(":x :y :z .").is_err());
    }
}
