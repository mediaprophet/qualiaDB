//! Ontology surfaces persist on the COP ledger and call live graph capabilities.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlInputElement};

use super::super::cop_records::{build_family_panel, CopField};

pub const PERSON_SAFE_N3: &str = r#"@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>.
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#>.
@prefix owl: <http://www.w3.org/2002/07/owl#>.
@prefix sh: <http://www.w3.org/ns/shacl#>.
@prefix q42: <https://ns.webcivics.net/>.

# Natural persons are rdfs:Class, never owl:Class / owl:Thing.
q42:Principal a rdfs:Class ;
  rdfs:label "Principal (natural person with agency)" .

q42:Project a rdfs:Class ;
  rdfs:label "Project" ;
  rdfs:comment "An artefact a Principal may have. Not a person." .

q42:hasMember a rdf:Property ;
  rdfs:domain q42:Project ;
  rdfs:range q42:Principal .

# owl:Thing appears only as a SHACL guard target.
q42:PrincipalShape a sh:NodeShape ;
  sh:targetClass q42:Principal ;
  sh:not [ sh:class owl:Thing ] .
"#;

const RDFS_RELATIONS: &[(&str, &str)] = &[
    ("rdfs:subClassOf", "Class hierarchy (RDFS)"),
    ("rdfs:subPropertyOf", "Property hierarchy (RDFS)"),
    ("rdfs:domain", "Property domain (RDFS)"),
    ("rdfs:range", "Property range (RDFS)"),
    ("rdf:type", "Typing (use rdfs:Class for persons)"),
    ("sh:targetClass", "SHACL target class"),
    ("sh:not", "SHACL negation (owl:Thing guard)"),
    ("owl:equivalentClass", "OWL equivalence — artefacts only"),
    ("owl:disjointWith", "OWL disjoint — artefacts only"),
    ("owl:inverseOf", "OWL inverse — artefacts only"),
];

fn wrap(document: &Document, child: Element) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; overflow: auto; padding: 8px; gap: 8px;",
    );
    wrapper.append_child(&child).unwrap();
    wrapper
}

fn ledger(
    document: &Document,
    family: &'static str,
    heading: &str,
    fields: &'static [CopField],
) -> Element {
    wrap(
        document,
        build_family_panel(document, family, heading, fields),
    )
}

fn banner(document: &Document, text: &str) -> Element {
    let note = document.create_element("div").unwrap();
    note.set_text_content(Some(text));
    let el: HtmlElement = note.clone().dyn_into().unwrap();
    el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); \
         border: 1px solid var(--border-subtle); border-radius: 4px; padding: 6px 8px;",
    );
    note
}

fn input_value(root: &Element, selector: &str) -> String {
    root.query_selector(selector)
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}



mod library;
mod shapes;
mod graphs;

pub use graphs::*;
pub use library::*;
pub use shapes::*;
