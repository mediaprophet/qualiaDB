//! Knowledge manifold seed — graph explorer, ontology browser, document.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::super::core::registry::{
    DockPosition, ManifoldSeed, SeedConnection, SeedContainer, SeedPanel,
};

pub fn knowledge_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "knowledge".into(),
        label: "Knowledge".into(),
        icon: "graph".into(),
        ontology_prefix: "ont".into(),
        description: "SPARQL explorer, ontology browser, and document annotation.".into(),
        containers: vec![
            SeedContainer {
                container_type: "graph".into(),
                title: "Graph Explorer".into(),
                x: 30.0,
                y: 30.0,
                width: 480.0,
                height: 360.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "ontology".into(),
                title: "Ontology Browser".into(),
                x: 530.0,
                y: 30.0,
                width: 320.0,
                height: 360.0,
                z: 1.0,
                honesty: "partial".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "doc".into(),
                title: "Document".into(),
                x: 30.0,
                y: 410.0,
                width: 820.0,
                height: 240.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
        ],
        connections: vec![
            SeedConnection {
                id: "wire-k1".into(),
                from: 0,
                to: 1,
                wire_type: "ontology".into(),
                label: "ont:usesOntology".into(),
            },
            SeedConnection {
                id: "wire-k2".into(),
                from: 2,
                to: 0,
                wire_type: "active".into(),
                label: "hm:annotatedBy".into(),
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
    }
}
