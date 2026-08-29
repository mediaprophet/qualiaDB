//! Ontology views — visual ontology authoring workbench.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Natural persons are modelled with RDFS + SHACL/ShEx. `owl:Thing` is not a
//! person; OWL is artefact/class inference or a SHACL guard target only.

pub mod persist;
pub mod personhood;

pub mod graph_canvas;
pub mod n3_editor;
pub mod ontology_compare;
pub mod ontology_library;
pub mod project_ontology_selector;
pub mod relation_builder;
pub mod shacl_shapes;
pub mod shex_editor;
pub mod vocabulary_mapper;
