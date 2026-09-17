//! Dataset views — complex dataset curation containers (Workstream D).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Registries persist on the COP `/records` ledger. RDF/N3 ingest uses the
//! Semantic Library. Render/CAD/media sessions stay unbound until registered.

pub mod persist;

pub mod annotation_panel;
pub mod cad_curation;
pub mod dataset_importer;
pub mod dataset_registry;
pub mod lineage_graph;
pub mod presentation_editor;
pub mod presentation_publish;
pub mod super_resolve;
pub mod video_view;
pub mod view_canvas;
