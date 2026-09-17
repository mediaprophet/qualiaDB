//! Ontology manifold seed — visual ontology authoring workbench.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::super::core::registry::{
    DockPosition, ManifoldSeed, SeedConnection, SeedContainer, SeedPanel,
};

pub fn ontology_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "ontology".into(),
        label: "Ontology".into(),
        icon: "ontology".into(),
        ontology_prefix: "ont".into(),
        description:
            "Visual ontology authoring, vocabulary mapping, and project ontology selection.".into(),
        containers: vec![
            SeedContainer {
                container_type: "ontology_graph_canvas".into(),
                title: "Semantic Graph Canvas".into(),
                x: 30.0,
                y: 30.0,
                width: 660.0,
                height: 480.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "ontology_library".into(),
                title: "Ontology Library".into(),
                x: 710.0,
                y: 30.0,
                width: 380.0,
                height: 480.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "vocabulary_mapper".into(),
                title: "Vocabulary Mapper".into(),
                x: 30.0,
                y: 530.0,
                width: 520.0,
                height: 360.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "relation_builder".into(),
                title: "Relation Builder".into(),
                x: 570.0,
                y: 530.0,
                width: 520.0,
                height: 360.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "shacl_shapes".into(),
                title: "SHACL Shapes".into(),
                x: 1110.0,
                y: 30.0,
                width: 380.0,
                height: 400.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "n3_editor".into(),
                title: "N3 Editor".into(),
                x: 1110.0,
                y: 450.0,
                width: 380.0,
                height: 440.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "shex_editor".into(),
                title: "ShEx Editor".into(),
                x: 1110.0,
                y: 910.0,
                width: 380.0,
                height: 320.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "ontology_compare".into(),
                title: "Ontology Compare".into(),
                x: 30.0,
                y: 910.0,
                width: 660.0,
                height: 320.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "project_ontology_selector".into(),
                title: "Project Ontology Selector".into(),
                x: 710.0,
                y: 910.0,
                width: 780.0,
                height: 320.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
        ],
        connections: vec![
            SeedConnection {
                id: "wire-o1".into(),
                from: 1,
                to: 0,
                wire_type: "ontology".into(),
                label: "ont:loadedInto".into(),
            },
            SeedConnection {
                id: "wire-o2".into(),
                from: 2,
                to: 0,
                wire_type: "active".into(),
                label: "ont:vocabMapped".into(),
            },
            SeedConnection {
                id: "wire-o3".into(),
                from: 3,
                to: 0,
                wire_type: "active".into(),
                label: "ont:relationAdded".into(),
            },
            SeedConnection {
                id: "wire-o4".into(),
                from: 4,
                to: 5,
                wire_type: "ontology".into(),
                label: "ont:shapesFor".into(),
            },
            SeedConnection {
                id: "wire-o5".into(),
                from: 7,
                to: 1,
                wire_type: "ontology".into(),
                label: "ont:selectsFrom".into(),
            },
        ],
        panels: vec![
            SeedPanel {
                panel_type: "aura".into(),
                dock: DockPosition::Right,
            },
            SeedPanel {
                panel_type: "pulse".into(),
                dock: DockPosition::Right,
            },
        ],
        ..Default::default()
    }
}
