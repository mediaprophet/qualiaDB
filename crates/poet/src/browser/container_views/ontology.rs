//! Ontology browser container — live graph stats, not a fabricated class tree.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::browser::cop_records::{build_family_panel, CopField};
use crate::browser::live_invoke;

/// Ontology browser container — live graph stats, not a fabricated class tree.
pub fn build_ontology_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("ontology-tree");
    let note = document.create_element("div").unwrap();
    note.set_text_content(Some(
        "Live graph stats from GraphDatabase.stats. Class trees come from COP ontology records and N3 authoring, not a canned prefix list.",
    ));
    let note_el: HtmlElement = note.clone().dyn_into().unwrap();
    note_el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); padding: 8px;",
    );
    wrapper.append_child(&note).unwrap();
    wrapper
        .append_child(&live_invoke::action_bar(
            document,
            &[
                (
                    "GraphDatabase.stats",
                    "GraphDatabase.stats",
                    serde_json::json!({}),
                ),
                (
                    "SHACL.extensions",
                    "SHACL.extensions",
                    serde_json::json!({}),
                ),
            ],
        ))
        .unwrap();
    let panel = build_family_panel(
        document,
        "ontology_term",
        "Ontology terms you record. Natural persons are rdfs:Class, never owl:Thing.",
        &[
            CopField {
                key: "iri",
                placeholder: "IRI",
            },
            CopField {
                key: "kind",
                placeholder: "Kind (class|property)",
            },
            CopField {
                key: "paradigm",
                placeholder: "Paradigm (rdfs|shacl|shex)",
            },
        ],
    );
    wrapper.append_child(&panel).unwrap();
    wrapper
}
