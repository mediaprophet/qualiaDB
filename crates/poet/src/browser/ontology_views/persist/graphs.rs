use web_sys::{Document, Element};

use super::super::super::cop_records::CopField;
use super::{banner, ledger};

pub fn build_ontology_graph_canvas_view(document: &Document) -> Element {
    let wrapper = ledger(
        document,
        "ontology_term",
        "Graph terms persist as COP records. Default modelling for persons is RDFS + SHACL, not OWL.",
        &[
            CopField {
                key: "paradigm",
                placeholder: "Paradigm (rdfs|shacl|shex|skos|owl-artefact)",
            },
            CopField {
                key: "kind",
                placeholder: "Kind (rdfs:Class|rdf:Property|sh:NodeShape|owl:Class)",
            },
            CopField {
                key: "iri",
                placeholder: "IRI",
            },
        ],
    );
    wrapper.append_child(&banner(
        document,
        "Do not type a Principal/Person as owl:Class. That imports owl:Thing. Use rdfs:Class + sh:NodeShape, with sh:not owl:Thing as the guard.",
    ))
    .unwrap();
    wrapper
}

pub fn build_vocabulary_mapper_view(document: &Document) -> Element {
    ledger(
        document,
        "ontology_mapping",
        "Vocabulary mappings persist as records. WordNet/FST lookup is unbound until that capability is registered.",
        &[
            CopField {
                key: "source",
                placeholder: "Source term",
            },
            CopField {
                key: "target",
                placeholder: "Target IRI",
            },
            CopField {
                key: "relation",
                placeholder: "Relation (skos:exactMatch|rdfs:subClassOf)",
            },
        ],
    )
}

pub fn build_ontology_compare_view(document: &Document) -> Element {
    ledger(
        document,
        "ontology_compare",
        "Compare jobs persist as records. Paste two IRIs; graph-diff invoke is GraphAuthoring.process.",
        &[
            CopField {
                key: "left",
                placeholder: "Left ontology IRI",
            },
            CopField {
                key: "right",
                placeholder: "Right ontology IRI",
            },
            CopField {
                key: "note",
                placeholder: "Note",
            },
        ],
    )
}

pub fn build_project_ontology_selector_view(document: &Document) -> Element {
    ledger(
        document,
        "ontology_binding",
        "Project ↔ ontology bindings persist here. Person-bearing ontologies must be RDFS/SHACL, not OWL.",
        &[
            CopField {
                key: "project",
                placeholder: "Project id",
            },
            CopField {
                key: "ontology",
                placeholder: "Ontology IRI",
            },
            CopField {
                key: "paradigm",
                placeholder: "Paradigm (rdfs|shacl|shex|owl-artefact)",
            },
        ],
    )
}
