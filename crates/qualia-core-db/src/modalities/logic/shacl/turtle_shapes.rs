//! Compile RDF Turtle / N3 SHACL shapes into [`CompiledShape`] (UE-050).
//!
//! Prefer JSON ShapeSpec for Node/Civics admission (already complete). This
//! path admits classical `sh:NodeShape` documents so ontology authors can keep
//! Turtle sources. Parsing reuses [`super::text_input::build_graph`]; all
//! SHACL core + Qualia modality predicates listed below are mapped, and unknown
//! `sh:*` constraint predicates fail closed on compile.

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

/// True when `p` is any recognised spelling (full IRI, `sh:` CURIE, or
/// bare local) of SHACL predicate `local`.
fn is_pred(p: u64, local: &str) -> bool {
    pred_aliases(local).contains(&p)
}

/// Hash of the `rdf:type` predicate — the N3 `a` keyword is kept as the
/// bare `a` token by the parser.
fn rdf_type() -> u64 {
    // N3 `a` keyword is kept as bare `a` by the parser.
    q_hash("a")
}

/// True when object `o` denotes `sh:NodeShape`, checked lexically via the
/// resolver first then by predicate-alias hash.
fn is_node_shape_type(o: u64, resolve: &HashMap<u64, String>) -> bool {
    if let Some(s) = resolve.get(&o) {
        return s == "sh:NodeShape"
            || s.ends_with("#NodeShape")
            || s.ends_with(":NodeShape")
            || s == "NodeShape";
    }
    pred_aliases("NodeShape").contains(&o)
}

/// Recover the lexical form of hash `h` from the resolver; falls back to
/// a `node:<hex>` placeholder when the term was never interned.
fn lexical(h: u64, resolve: &HashMap<u64, String>) -> String {
    resolve
        .get(&h)
        .cloned()
        .unwrap_or_else(|| format!("node:{h:016x}"))
}

/// Coerce an object to `f64`: parse its resolved lexical form as int or
/// float, else decode inline-tagged numeric objects via the validator's
/// `object_as_f64`. `None` for non-numeric terms.
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

/// All quins whose `subject` matches and whose predicate is any spelling
/// of `pred_local`.
fn objects_of<'a>(quins: &'a [NQuin], subject: u64, pred_local: &str) -> Vec<&'a NQuin> {
    quins
        .iter()
        .filter(|q| q.subject == subject && is_pred(q.predicate, pred_local))
        .collect()
}

/// First object value of predicate `pred_local` on `subject`, or `None`
/// when the fact is absent.
fn first_object(quins: &[NQuin], subject: u64, pred_local: &str) -> Option<u64> {
    objects_of(quins, subject, pred_local)
        .first()
        .map(|q| q.object)
}

/// Collect `ShaclConstraint`s declared on a `sh:property` node or `sh:NodeShape`:
/// resolves each predicate's lexical local name when available, else
/// falls back to hash matching against the well-known constraint names.
/// Fails closed on unknown `sh:*` predicates.
fn constraint_from_property(
    quins: &[NQuin],
    prop_node: u64,
    resolve: &HashMap<u64, String>,
) -> Result<Vec<ShaclConstraint>, String> {
    let mut out = Vec::new();
    let mut in_values: Vec<String> = Vec::new();
    let mut and_values: Vec<String> = Vec::new();
    let mut or_values: Vec<String> = Vec::new();
    let mut xone_values: Vec<String> = Vec::new();
    let mut lang_in_values: Vec<String> = Vec::new();
    let mut is_closed = false;
    let mut ignored_props: Vec<String> = Vec::new();

    for q in quins.iter().filter(|q| q.subject == prop_node) {
        let Some(pname) = resolve.get(&q.predicate) else {
            // Still try well-known hashes without lexical.
            push_constraint_by_hash(
                quins,
                q.predicate,
                q.object,
                resolve,
                &mut out,
                &mut in_values,
            );
            continue;
        };
        let is_shacl_ns =
            pname.starts_with("sh:") || pname.starts_with("http://www.w3.org/ns/shacl#");
        let local = pname
            .rsplit(['#', ':', '/'])
            .next()
            .unwrap_or(pname.as_str());

        if is_shacl_ns && !is_known_shacl_local(local) {
            return Err(format!("unknown SHACL predicate: {pname}"));
        }

        match local {
            "in" => push_list_items(q.object, resolve, &mut in_values),
            "and" => push_shape_ref_items(quins, q.object, resolve, &mut and_values),
            "or" => push_shape_ref_items(quins, q.object, resolve, &mut or_values),
            "xone" => push_shape_ref_items(quins, q.object, resolve, &mut xone_values),
            "languageIn" => push_list_items(q.object, resolve, &mut lang_in_values),
            "closed" => {
                let v = lexical(q.object, resolve);
                if v == "true" || v == "1" || object_f64(q.object, resolve) == Some(1.0) {
                    is_closed = true;
                }
            }
            "ignoredProperties" => push_list_items(q.object, resolve, &mut ignored_props),
            _ => push_constraint(quins, local, q.object, resolve, &mut out),
        }
    }

    if !in_values.is_empty() {
        out.push(ShaclConstraint::In(in_values));
    }
    if !and_values.is_empty() {
        out.push(ShaclConstraint::And(and_values));
    }
    if !or_values.is_empty() {
        out.push(ShaclConstraint::Or(or_values));
    }
    if !xone_values.is_empty() {
        out.push(ShaclConstraint::Xone(xone_values));
    }
    if !lang_in_values.is_empty() {
        out.push(ShaclConstraint::LanguageIn(lang_in_values));
    }
    if is_closed {
        out.push(ShaclConstraint::Closed {
            ignored_properties: ignored_props,
        });
    }

    Ok(out)
}

/// True when `local` is a recognised SHACL constraint or structural predicate.
fn is_known_shacl_local(local: &str) -> bool {
    matches!(
        local,
        "minInclusive"
            | "maxInclusive"
            | "minExclusive"
            | "maxExclusive"
            | "minCount"
            | "maxCount"
            | "minLength"
            | "maxLength"
            | "pattern"
            | "class"
            | "datatype"
            | "nodeKind"
            | "hasValue"
            | "in"
            | "equals"
            | "lessThan"
            | "lessThanOrEquals"
            | "greaterThan"
            | "greaterThanOrEquals"
            | "node"
            | "not"
            | "and"
            | "or"
            | "xone"
            | "languageIn"
            | "uniqueLang"
            | "closed"
            | "ignoredProperties"
            | "path"
            | "targetClass"
            | "targetNode"
            | "targetObjectsOf"
            | "targetSubjectsOf"
            | "property"
            | "severity"
            | "name"
            | "description"
            | "message"
            | "sparql"
            | "select"
            | "prefixes"
            | "declare"
            | "prefix"
            | "namespace"
            | "order"
            | "group"
            | "defaultValue"
            | "flags"
            | "deactivated"
            | "NodeShape"
            | "PropertyShape"
            | "deonticObligate"
            | "DeonticObligate"
            | "deonticPermit"
            | "DeonticPermit"
            | "deonticForbid"
            | "DeonticForbid"
            | "valuesConsentNonCoerced"
            | "ValuesConsentNonCoerced"
            | "valuesHarmBelowCeiling"
            | "ValuesHarmBelowCeiling"
    )
}

/// Resolve a shape reference (`sh:not`, `sh:node`, `sh:and`, `sh:or`, `sh:xone`)
/// to the target class name when the referenced `sh:NodeShape` declares
/// `sh:targetClass`, or to its own lexical identifier otherwise.
fn resolve_shape_ref(quins: &[NQuin], object: u64, resolve: &HashMap<u64, String>) -> String {
    first_object(quins, object, "targetClass")
        .map(|tc| lexical(tc, resolve))
        .unwrap_or_else(|| lexical(object, resolve))
}

fn push_list_items(object: u64, resolve: &HashMap<u64, String>, out: &mut Vec<String>) {
    let raw = lexical(object, resolve);
    let trimmed = raw.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len() - 1];
        for tok in inner.split_whitespace() {
            let clean = tok.trim_matches('"');
            if !clean.is_empty() {
                out.push(clean.to_string());
            }
        }
    } else if !trimmed.is_empty() {
        out.push(trimmed.trim_matches('"').to_string());
    }
}

fn push_shape_ref_items(
    quins: &[NQuin],
    object: u64,
    resolve: &HashMap<u64, String>,
    out: &mut Vec<String>,
) {
    out.push(resolve_shape_ref(quins, object, resolve));
}

/// Hash-only fallback for `constraint_from_property`: match `pred`
/// against the well-known SHACL constraint predicate hashes and emit the
/// corresponding constraint for `object`.
fn push_constraint_by_hash(
    quins: &[NQuin],
    pred: u64,
    object: u64,
    resolve: &HashMap<u64, String>,
    out: &mut Vec<ShaclConstraint>,
    in_values: &mut Vec<String>,
) {
    if is_pred(pred, "in") {
        push_list_items(object, resolve, in_values);
        return;
    }
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
        "node",
        "not",
    ] {
        if is_pred(pred, local) {
            push_constraint(quins, local, object, resolve, out);
            return;
        }
    }
}

/// Emit the `ShaclConstraint` for a resolved predicate local name and
/// object — numeric locals read `object_f64` (defaulting to 0.0 on
/// non-numeric objects), text locals read the lexical form.
fn push_constraint(
    quins: &[NQuin],
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
        "node" => out.push(ShaclConstraint::Node(resolve_shape_ref(
            quins, object, resolve,
        ))),
        "not" => out.push(ShaclConstraint::Not(resolve_shape_ref(
            quins, object, resolve,
        ))),
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
        "path" | "targetClass" | "property" | "severity" | "name" | "description" | "message" => {}
        _ => {}
    }
}

/// Compile a Turtle / N3 SHACL document into [`CompiledShape`] values.
///
/// Returns an error when no NodeShape / targetClass shape can be recovered
/// or when an unknown `sh:*` predicate is encountered.
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

        for prop in &prop_nodes {
            let path = first_object(&quins, *prop, "path")
                .map(|o| lexical(o, &resolve))
                .unwrap_or_default();
            let constraints = constraint_from_property(&quins, *prop, &resolve)?;
            if constraints.is_empty() {
                continue;
            }
            let mut shape = CompiledShape::new(target.clone(), constraints, severity);
            shape.property_path = path;
            shape.name = Some(lexical(shape_id, &resolve));
            shapes.push(shape);
        }

        // Always compile direct node-level constraints (such as `sh:not`, `sh:class`,
        // `sh:in`, etc.) even when `sh:property` constraints were also emitted.
        let node_constraints = constraint_from_property(&quins, shape_id, &resolve)?;
        if !node_constraints.is_empty() {
            let mut shape = CompiledShape::new(target, node_constraints, severity);
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
pub fn validate_turtle_shapes(
    data: &str,
    shapes_ttl: &str,
) -> Result<super::shacl_types::ValidationReport, String> {
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

    #[test]
    fn nodeshape_compiles_both_property_and_node_level_not() {
        let shapes = r#"
idf:NaturalAgentShape a sh:NodeShape ;
    sh:targetClass idf:NaturalAgent ;
    sh:property idf:NaturalAgent-planeProp ;
    sh:not idf:NaturalAgent-notOwlThing .
idf:NaturalAgent-planeProp sh:path idf:plane ;
    sh:minCount 1 ;
    sh:hasValue "NaturalAgent" .
idf:NaturalAgent-notOwlThing a sh:NodeShape ;
    sh:class owl:Thing .
"#;
        let ok_data = r#"
:alice a idf:NaturalAgent ;
    idf:plane "NaturalAgent" .
"#;
        let ok_report = validate_turtle_shapes(ok_data, shapes).expect("validate ok");
        assert!(ok_report.conforms, "valid NaturalAgent must conform");

        let bad_thing_data = r#"
:alice a idf:NaturalAgent ;
    a owl:Thing ;
    idf:plane "NaturalAgent" .
"#;
        let bad_report = validate_turtle_shapes(bad_thing_data, shapes).expect("validate bad");
        assert!(
            !bad_report.conforms,
            "NaturalAgent also typed owl:Thing must violate sh:not even when sh:property conforms"
        );
    }

    #[test]
    fn sh_in_and_unknown_shacl_predicate_fail_closed() {
        let shapes = r#"
idf:KeyRoleShape a sh:NodeShape ;
    sh:targetClass idf:KeyGrant ;
    sh:property idf:KeyRole-prop .
idf:KeyRole-prop sh:path idf:memberKeyRole ;
    sh:minCount 1 ;
    sh:in "SessionAuthentication" , "RouteUpdate" , "TransportAead" .
"#;
        let ok_data = ":g1 a idf:KeyGrant ; idf:memberKeyRole \"SessionAuthentication\" .";
        assert!(validate_turtle_shapes(ok_data, shapes).unwrap().conforms);

        let bad_data = ":g2 a idf:KeyGrant ; idf:memberKeyRole \"UnknownRole\" .";
        assert!(!validate_turtle_shapes(bad_data, shapes).unwrap().conforms);

        let unknown_pred_shapes = r#"
idf:BadShape a sh:NodeShape ;
    sh:targetClass idf:Bad ;
    sh:property idf:Bad-prop .
idf:Bad-prop sh:path idf:x ;
    sh:unknownInventedConstraint 42 .
"#;
        assert!(
            shapes_from_turtle(unknown_pred_shapes).is_err(),
            "unknown sh:* predicate must fail closed"
        );
    }
}
