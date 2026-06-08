//! OWL → SHACL conversion for QualiaDB ontology alignment.
//!
//! RadLex and DICOM healthcare vocabularies arrive as OWL (RDF/XML or Turtle/N3).
//! The Webizen Sentinel consumes **SHACL**, not OWL classifiers directly. This module
//! lowers OWL axioms into `sh:NodeShape` graphs while preserving the agency invariant:
//! **`q42:Principal` may have `q42:Thing` possessions but is not itself a Thing.**

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;
use rio_api::model::Triple;
use rio_api::parser::TriplesParser;
use rio_turtle::TurtleParser;

const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
const OWL_DATATYPE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
const OWL_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
const OWL_EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
const OWL_EQUIVALENT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// Prefixes emitted on every generated shapes file.
pub const SHAPE_PREFIXES: &str = r#"@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix q42: <https://qualia.id/ns/> .
@prefix hc: <http://purl.org/healthcarevocab/v1#> .
@prefix radlex: <http://www.radlex.org/RID/> .
"#;

#[derive(Debug, Default, Clone)]
struct OwlClass {
    labels: Vec<String>,
    equivalent: Option<String>,
}

#[derive(Debug, Default, Clone)]
struct OwlProperty {
    labels: Vec<String>,
    kinds: BTreeSet<String>,
    domains: BTreeSet<String>,
    ranges: BTreeSet<String>,
    equivalent: Option<String>,
}

#[derive(Debug, Default)]
pub struct HealthcareOwlModel {
    classes: BTreeMap<String, OwlClass>,
    properties: BTreeMap<String, OwlProperty>,
}

#[derive(Debug, Clone)]
pub struct RadlexRelation {
    pub subject_rid: String,
    pub predicate_local: String,
    pub object_rid: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OwlToShaclError {
    Io(String),
    Parse(String),
    EmptyInput,
}

impl std::fmt::Display for OwlToShaclError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "IO error: {msg}"),
            Self::Parse(msg) => write!(f, "parse error: {msg}"),
            Self::EmptyInput => write!(f, "no triples or relations parsed"),
        }
    }
}

impl std::error::Error for OwlToShaclError {}

fn is_blank_or_anon(subject: &str) -> bool {
    subject.starts_with("_:") || subject.starts_with('[')
}

fn local_name(uri: &str) -> String {
    uri.rsplit(['#', '/']).next().unwrap_or(uri).to_string()
}

fn shape_name_for_uri(uri: &str) -> String {
    let local = local_name(uri);
    if uri.contains("healthcarevocab") {
        format!("hc:{local}Shape")
    } else if uri.contains("radlex.org") {
        format!("radlex:{local}Shape")
    } else {
        format!("ex:{local}Shape")
    }
}

fn curie_for_uri(uri: &str) -> String {
    if let Some(tag) = uri.strip_prefix("http://purl.org/healthcarevocab/v1#") {
        return format!("hc:{tag}");
    }
    if let Some(tag) = uri.strip_prefix("http://www.radlex.org/RID/") {
        return format!("radlex:{tag}");
    }
    if uri.starts_with("https://qualia.id/ns/") {
        let tag = uri.trim_start_matches("https://qualia.id/ns/");
        return format!("q42:{tag}");
    }
    format!("<{uri}>")
}

fn xsd_from_range(range: &str) -> Option<&'static str> {
    match range {
        "http://www.w3.org/2001/XMLSchema#date" => Some("xsd:date"),
        "http://www.w3.org/2001/XMLSchema#dateTime" => Some("xsd:dateTime"),
        "http://www.w3.org/2001/XMLSchema#long" | "http://www.w3.org/2001/XMLSchema#integer" => {
            Some("xsd:integer")
        }
        "http://www.w3.org/2001/XMLSchema#double"
        | "http://www.w3.org/2001/XMLSchema#decimal"
        | "http://www.w3.org/2001/XMLSchema#float" => Some("xsd:decimal"),
        "http://www.w3.org/2001/XMLSchema#boolean" => Some("xsd:boolean"),
        _ => None,
    }
}

fn ingest_owl_triple(model: &mut HealthcareOwlModel, s: String, p: String, o: String) {
    if p == RDF_TYPE {
        if o == OWL_CLASS {
            model.classes.entry(s).or_default();
        } else if o == OWL_DATATYPE_PROPERTY || o == OWL_OBJECT_PROPERTY {
            model.properties.entry(s).or_default().kinds.insert(o);
        }
    } else if p == RDFS_LABEL {
        if let Some(class) = model.classes.get_mut(&s) {
            class.labels.push(o.trim_matches('"').to_string());
        } else if let Some(prop) = model.properties.get_mut(&s) {
            prop.labels.push(o.trim_matches('"').to_string());
        }
    } else if p == OWL_EQUIVALENT_CLASS {
        model.classes.entry(s).or_default().equivalent = Some(o);
    } else if p == OWL_EQUIVALENT_PROPERTY {
        model.properties.entry(s).or_default().equivalent = Some(o);
    } else if p == RDFS_DOMAIN {
        model.properties.entry(s).or_default().domains.insert(o);
    } else if p == RDFS_RANGE {
        model.properties.entry(s).or_default().ranges.insert(o);
    }
}

/// Parse Turtle healthcare vocabulary.
pub fn parse_healthcare_owl_turtle(path: &Path) -> Result<HealthcareOwlModel, OwlToShaclError> {
    let bytes = fs::read(path).map_err(|e| OwlToShaclError::Io(e.to_string()))?;
    let sanitized = trim_incomplete_turtle_tail(&bytes);
    let mut model = HealthcareOwlModel::default();
    let mut on_triple = |t: Triple| -> Result<(), std::io::Error> {
        ingest_owl_triple(
            &mut model,
            t.subject.to_string(),
            t.predicate.to_string(),
            t.object.to_string(),
        );
        Ok(())
    };
    let cursor = std::io::Cursor::new(sanitized);
    let mut parser = TurtleParser::new(cursor, None);
    if let Err(e) = parser.parse_all(&mut on_triple) {
        if model.classes.is_empty() && model.properties.is_empty() {
            return Err(OwlToShaclError::Parse(format!("{e}")));
        }
    }
    if model.classes.is_empty() && model.properties.is_empty() {
        return Err(OwlToShaclError::EmptyInput);
    }
    Ok(model)
}

/// Parse `.n3` healthcare excerpt (OWL in Notation3/Turtle syntax; tolerates truncated exports).
pub fn parse_healthcare_owl_n3(path: &Path) -> Result<HealthcareOwlModel, OwlToShaclError> {
    let file = File::open(path).map_err(|e| OwlToShaclError::Io(e.to_string()))?;
    let mut model = HealthcareOwlModel::default();
    let mut parser = crate::n3_parser::N3Parser::new(BufReader::new(file));
    let result = parser.parse_all(|event| {
        if let crate::n3_parser::N3Event::StaticTriple(triple) = event {
            let subject = term_uri(&triple.subject);
            let predicate = term_uri(&triple.predicate);
            let object = term_uri(&triple.object);
            ingest_owl_triple(&mut model, subject, predicate, object);
        }
        Ok(())
    });
    if result.is_err() && model.classes.is_empty() && model.properties.is_empty() {
        return parse_healthcare_owl_turtle(path);
    }
    if model.classes.is_empty() && model.properties.is_empty() {
        return Err(OwlToShaclError::EmptyInput);
    }
    Ok(model)
}

/// Line-oriented parser for healthcare vocab exports (handles truncated tails and `unionOf` domains).
pub fn parse_healthcare_owl_lines(path: &Path) -> Result<HealthcareOwlModel, OwlToShaclError> {
    let bytes = fs::read(path).map_err(|e| OwlToShaclError::Io(e.to_string()))?;
    let sanitized = trim_incomplete_turtle_tail(&bytes);
    let text = String::from_utf8_lossy(&sanitized).into_owned();
    let mut model = HealthcareOwlModel::default();
    let mut current_subject: Option<String> = None;

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('@') {
            continue;
        }
        if line.starts_with('<')
            && line.ends_with('>')
            && !line.contains(';')
            && !line.contains(' ')
        {
            current_subject = Some(line.trim_matches(|c| c == '<' || c == '>').to_string());
            continue;
        }
        let Some(subject) = current_subject.clone() else {
            continue;
        };
        if let Some(kind) = line.strip_prefix("a ") {
            let kind = kind.trim().trim_end_matches(';').trim();
            let expanded = match kind {
                "owl:Class" => OWL_CLASS.to_string(),
                "owl:DatatypeProperty" => OWL_DATATYPE_PROPERTY.to_string(),
                "owl:ObjectProperty" => OWL_OBJECT_PROPERTY.to_string(),
                other if other.starts_with("http") => other.to_string(),
                _ => continue,
            };
            ingest_owl_triple(&mut model, subject, RDF_TYPE.to_string(), expanded);
        } else if line.strip_prefix("rdfs:label").is_some() {
            if let Some(value) = extract_turtle_string_literal(line) {
                ingest_owl_triple(&mut model, subject, RDFS_LABEL.to_string(), value);
            }
        } else if line.contains("owl:equivalentClass") {
            if let Some(uri) = extract_first_uri(line) {
                ingest_owl_triple(&mut model, subject, OWL_EQUIVALENT_CLASS.to_string(), uri);
            }
        } else if line.contains("owl:equivalentProperty") {
            if let Some(uri) = extract_first_uri(line) {
                ingest_owl_triple(
                    &mut model,
                    subject,
                    OWL_EQUIVALENT_PROPERTY.to_string(),
                    uri,
                );
            }
        } else if line.contains("rdfs:domain") {
            if let Some(uri) = extract_first_uri(line) {
                ingest_owl_triple(&mut model, subject, RDFS_DOMAIN.to_string(), uri);
            }
        } else if line.contains("rdfs:range") {
            if let Some(uri) = extract_first_uri(line) {
                ingest_owl_triple(&mut model, subject, RDFS_RANGE.to_string(), uri);
            }
        } else if line.ends_with('.') && !line.starts_with('<') {
            current_subject = None;
        }
    }

    if model.classes.is_empty() && model.properties.is_empty() {
        return Err(OwlToShaclError::EmptyInput);
    }
    Ok(model)
}

fn extract_first_uri(line: &str) -> Option<String> {
    let start = line.find('<')? + 1;
    let end = line[start..].find('>')? + start;
    Some(line[start..end].to_string())
}

fn extract_turtle_string_literal(line: &str) -> Option<String> {
    let start = line.find('"')? + 1;
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Parse healthcare OWL from `.n3` or `.ttl`.
pub fn parse_healthcare_owl(path: &Path) -> Result<HealthcareOwlModel, OwlToShaclError> {
    parse_healthcare_owl_lines(path)
        .or_else(|_| parse_healthcare_owl_turtle(path))
        .or_else(|_| parse_healthcare_owl_n3(path))
}

fn term_uri(term: &crate::n3_parser::Term) -> String {
    match term {
        crate::n3_parser::Term::Uri(s)
        | crate::n3_parser::Term::Literal(s)
        | crate::n3_parser::Term::Variable(s) => s.clone(),
    }
}

/// Drop a truncated final subject line (common in partial DICOM OWL exports).
fn trim_incomplete_turtle_tail(bytes: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(bytes);
    if text.trim_end().ends_with('.') {
        return bytes.to_vec();
    }
    let mut lines: Vec<&str> = text.lines().collect();
    while let Some(last) = lines.last() {
        if last.trim().starts_with('<') && !last.trim().ends_with('.') {
            lines.pop();
        } else {
            break;
        }
    }
    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.into_bytes()
}

/// Emit SHACL node shapes for healthcare IE.* classes and DICOM-tag properties.
pub fn healthcare_owl_to_shacl_ttl(model: &HealthcareOwlModel) -> String {
    let mut out = String::from(SHAPE_PREFIXES);
    out.push_str("\n# Generated from healthcare OWL — DICOM tag alignment\n\n");

    for (class_uri, class) in &model.classes {
        if is_blank_or_anon(class_uri) {
            continue;
        }
        let local = local_name(class_uri);
        if !local.starts_with("IE.") && !local.starts_with("SequenceItem.") {
            continue;
        }
        let shape = shape_name_for_uri(class_uri);
        let target = curie_for_uri(class_uri);
        out.push_str(&format!(
            "{shape} a sh:NodeShape ;\n    sh:targetClass {target}"
        ));
        if let Some(label) = class.labels.first() {
            out.push_str(&format!(" ;\n    rdfs:label \"{label}\""));
        }
        if let Some(eq) = &class.equivalent {
            out.push_str(&format!(
                " ;\n    sh:property [ sh:path owl:equivalentClass ; sh:hasValue {} ]",
                curie_for_uri(eq)
            ));
        }
        out.push_str(" .\n\n");
    }

    let mut by_domain: BTreeMap<String, Vec<(String, OwlProperty)>> = BTreeMap::new();
    for (prop_uri, prop) in &model.properties {
        if is_blank_or_anon(prop_uri) {
            continue;
        }
        let domains: Vec<String> = if prop.domains.is_empty() {
            vec!["http://purl.org/healthcarevocab/v1#IE.Image".to_string()]
        } else {
            prop.domains.iter().cloned().collect()
        };
        for domain in domains {
            if is_blank_or_anon(&domain) {
                continue;
            }
            by_domain
                .entry(domain)
                .or_default()
                .push((prop_uri.clone(), prop.clone()));
        }
    }

    for (domain_uri, props) in by_domain {
        let domain_local = local_name(&domain_uri);
        if !domain_local.starts_with("IE.") {
            continue;
        }
        let shape = format!("hc:{domain_local}PropertyShape");
        let target = curie_for_uri(&domain_uri);
        out.push_str(&format!(
            "{shape} a sh:NodeShape ;\n    sh:targetClass {target}"
        ));
        for (prop_uri, prop) in props {
            let path = curie_for_uri(&prop_uri);
            out.push_str(" ;\n    sh:property [");
            out.push_str(&format!("\n        sh:path {path}"));
            if let Some(label) = prop.labels.first() {
                out.push_str(&format!(" ;\n        sh:name \"{label}\""));
            }
            if let Some(eq) = &prop.equivalent {
                out.push_str(&format!(
                    " ;\n        sh:qualifiedValueShape [ sh:hasValue {} ]",
                    curie_for_uri(eq)
                ));
            }
            if prop.kinds.contains(OWL_DATATYPE_PROPERTY) {
                if let Some(range) = prop.ranges.iter().next() {
                    if let Some(dt) = xsd_from_range(range) {
                        out.push_str(&format!(" ;\n        sh:datatype {dt}"));
                    } else if *range == "http://www.w3.org/2000/01/rdf-schema#Literal" {
                        out.push_str(" ;\n        sh:datatype xsd:string");
                    }
                }
            } else if prop.kinds.contains(OWL_OBJECT_PROPERTY) {
                out.push_str(" ;\n        sh:nodeKind sh:BlankNodeOrIRI");
            }
            out.push_str("\n    ]");
        }
        out.push_str(" .\n\n");
    }

    out.push_str(
        "# Principal may reference imaging entities but is not an IE.* class.\n\
q42:PrincipalImagingLinkShape a sh:NodeShape ;\n\
    sh:targetClass q42:Principal ;\n\
    sh:property [\n\
        sh:path q42:hasImagingStudy ;\n\
        sh:class hc:IE.Study ;\n\
        sh:minCount 0 ;\n\
        sh:message \"Imaging studies are possessions of the Principal, not the Principal itself.\" ;\n\
    ] ;\n\
    sh:property [\n\
        sh:path q42:hasDicomSeries ;\n\
        sh:class hc:IE.Series ;\n\
        sh:minCount 0 ;\n\
    ] .\n\n",
    );

    out
}

/// Stream-parse RadLex Pun OWL (RDF/XML) relation axioms (`RID:Part_Of`, `RID:Has_Part`, …).
pub fn parse_radlex_relations_xml(
    path: &Path,
    max_relations: usize,
) -> Result<Vec<RadlexRelation>, OwlToShaclError> {
    let file = File::open(path).map_err(|e| OwlToShaclError::Io(e.to_string()))?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(true);

    let mut relations = Vec::new();
    let mut buf = Vec::new();
    let mut current_subject: Option<String> = None;
    let mut in_description = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "rdf:Description" || name.ends_with("Description") {
                    in_description = true;
                    current_subject = None;
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        if key.ends_with("about") {
                            current_subject =
                                Some(String::from_utf8_lossy(&attr.value).to_string());
                        }
                    }
                } else if in_description {
                    if let Some(subject) = current_subject.clone() {
                        if let Some(pred_local) = name.strip_prefix("RID:") {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                if key.ends_with("resource") {
                                    let object = String::from_utf8_lossy(&attr.value).to_string();
                                    relations.push(RadlexRelation {
                                        subject_rid: subject.clone(),
                                        predicate_local: pred_local.to_string(),
                                        object_rid: object,
                                    });
                                    if relations.len() >= max_relations {
                                        return Ok(relations);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "rdf:Description" || name.ends_with("Description") {
                    in_description = false;
                    current_subject = None;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(OwlToShaclError::Parse(format!("{e}"))),
            _ => {}
        }
        buf.clear();
    }

    if relations.is_empty() {
        return Err(OwlToShaclError::EmptyInput);
    }
    Ok(relations)
}

/// Group RadLex relations by subject RID and emit SHACL node shapes.
pub fn radlex_relations_to_shacl_ttl(relations: &[RadlexRelation], max_shapes: usize) -> String {
    let mut grouped: BTreeMap<String, Vec<RadlexRelation>> = BTreeMap::new();
    for rel in relations {
        grouped
            .entry(rel.subject_rid.clone())
            .or_default()
            .push(rel.clone());
    }

    let mut out = String::from(SHAPE_PREFIXES);
    out.push_str(
        "\n# Generated from RadLex OWL — anatomical possession graph (Things, not Principals)\n\n",
    );
    out.push_str(
        "radlex:AnatomicalEntity a owl:Class ;\n\
    rdfs:subClassOf q42:Thing ;\n\
    rdfs:comment \"RadLex RID tokens are clinical Things a Principal may have findings about.\" .\n\n",
    );

    for (idx, (subject, rels)) in grouped.iter().enumerate() {
        if idx >= max_shapes {
            out.push_str(&format!(
                "# ... truncated ({max_shapes} shapes shown; increase max_shapes for full export)\n"
            ));
            break;
        }
        let rid_local = local_name(subject);
        let shape = format!("radlex:{rid_local}Shape");
        let target = curie_for_uri(subject);
        out.push_str(&format!(
            "{shape} a sh:NodeShape ;\n    sh:targetClass {target} ;\n    sh:class radlex:AnatomicalEntity"
        ));
        for rel in rels {
            let pred = format!("radlex:{}", rel.predicate_local);
            let obj = curie_for_uri(&rel.object_rid);
            out.push_str(&format!(
                " ;\n    sh:property [ sh:path {pred} ; sh:hasValue {obj} ]"
            ));
        }
        out.push_str(" .\n\n");
    }

    out.push_str(
        "q42:PrincipalRadLexFindingShape a sh:NodeShape ;\n\
    sh:targetClass q42:Principal ;\n\
    sh:property [\n\
        sh:path q42:hasFinding ;\n\
        sh:class radlex:AnatomicalEntity ;\n\
        sh:minCount 0 ;\n\
        sh:message \"RadLex findings attach to a Principal; the Principal is not an anatomical entity.\" ;\n\
    ] .\n\n",
    );

    out
}

/// Write agency baseline + converted ontologies to an output directory.
pub fn write_anatomy_shape_bundle(
    healthcare_owl: Option<&Path>,
    radlex_owl: Option<&Path>,
    out_dir: &Path,
    radlex_max_shapes: usize,
    radlex_max_relations: usize,
) -> Result<Vec<String>, OwlToShaclError> {
    fs::create_dir_all(out_dir).map_err(|e| OwlToShaclError::Io(e.to_string()))?;
    let mut written = Vec::new();

    let agency_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("shapes/qualia-agency.shacl.ttl");
    let agency_dst = out_dir.join("qualia-agency.shacl.ttl");
    fs::copy(&agency_src, &agency_dst).map_err(|e| OwlToShaclError::Io(e.to_string()))?;
    written.push(agency_dst.display().to_string());

    if let Some(path) = healthcare_owl {
        let model = parse_healthcare_owl(path)?;
        let ttl = healthcare_owl_to_shacl_ttl(&model);
        let dst = out_dir.join("dicom-healthcare.shacl.ttl");
        write_text(&dst, &ttl)?;
        written.push(dst.display().to_string());
    }

    if let Some(path) = radlex_owl {
        let relations = parse_radlex_relations_xml(path, radlex_max_relations)?;
        let ttl = radlex_relations_to_shacl_ttl(&relations, radlex_max_shapes);
        let dst = out_dir.join("radlex-anatomy.shacl.ttl");
        write_text(&dst, &ttl)?;
        written.push(dst.display().to_string());
    }

    Ok(written)
}

fn write_text(path: &Path, content: &str) -> Result<(), OwlToShaclError> {
    let mut f = File::create(path).map_err(|e| OwlToShaclError::Io(e.to_string()))?;
    f.write_all(content.as_bytes())
        .map_err(|e| OwlToShaclError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_healthcare() -> Option<std::path::PathBuf> {
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for candidate in [
            manifest.join("../../app-development/2015-01-11.n3"),
            std::path::PathBuf::from("app-development/2015-01-11.n3"),
        ] {
            if candidate.is_file() {
                return candidate.canonicalize().ok().or(Some(candidate));
            }
        }
        None
    }

    fn fixture_radlex() -> Option<std::path::PathBuf> {
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for candidate in [
            manifest.join("../../app-development/PunRadLex_Owl4.3/PunRadLex4.3.owl"),
            std::path::PathBuf::from("app-development/PunRadLex_Owl4.3/PunRadLex4.3.owl"),
        ] {
            if candidate.is_file() {
                return candidate.canonicalize().ok().or(Some(candidate));
            }
        }
        None
    }

    #[test]
    fn healthcare_owl_parses_and_emits_ie_shapes() {
        let Some(path) = fixture_healthcare() else {
            eprintln!("skip: 2015-01-11.n3 not present");
            return;
        };
        let model = parse_healthcare_owl(&path).expect("parse healthcare owl");
        assert!(!model.classes.is_empty());
        assert!(!model.properties.is_empty());
        let ttl = healthcare_owl_to_shacl_ttl(&model);
        assert!(ttl.contains("hc:IE.ImagePropertyShape"));
        assert!(ttl.contains("q42:PrincipalImagingLinkShape"));
        assert!(ttl.contains("sh:targetClass q42:Principal"));
    }

    #[test]
    fn radlex_xml_parses_part_of_relations() {
        let Some(path) = fixture_radlex() else {
            eprintln!("skip: PunRadLex4.3.owl not present");
            return;
        };
        let rels = parse_radlex_relations_xml(&path, 64).expect("parse radlex");
        assert!(!rels.is_empty());
        assert!(rels.iter().any(|r| r.predicate_local == "Part_Of"));
        let ttl = radlex_relations_to_shacl_ttl(&rels, 8);
        assert!(ttl.contains("radlex:AnatomicalEntity"));
        assert!(ttl.contains("q42:PrincipalRadLexFindingShape"));
        assert!(ttl.contains("sh:targetClass q42:Principal"));
    }

    #[test]
    fn agency_shape_file_exists() {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("shapes/qualia-agency.shacl.ttl");
        let text = fs::read_to_string(p).unwrap();
        assert!(text.contains("q42:PrincipalShape"));
        assert!(text.contains("sh:not"));
        assert!(text.contains("q42:hasCondition"));
    }
}
