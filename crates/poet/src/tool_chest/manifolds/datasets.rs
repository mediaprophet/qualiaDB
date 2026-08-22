//! Datasets manifold seed — complex dataset curation containers (Workstream D).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::super::core::registry::{
    DockPosition, ManifoldSeed, SeedConnection, SeedContainer, SeedPanel,
};

pub fn datasets_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "datasets".into(),
        label: "Datasets".into(),
        icon: "datasets".into(),
        ontology_prefix: "dat".into(),
        description: "Complex dataset curation: import, registry, presentation editor, \
             view canvas with multiple renderer kinds. Provenance-tracked."
            .into(),
        containers: vec![
            SeedContainer {
                container_type: "dataset_registry".into(),
                title: "Dataset Registry".into(),
                x: 30.0,
                y: 30.0,
                width: 660.0,
                height: 300.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "dataset_importer".into(),
                title: "Dataset Importer".into(),
                x: 710.0,
                y: 30.0,
                width: 660.0,
                height: 300.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "presentation_editor".into(),
                title: "Presentation Editor".into(),
                x: 30.0,
                y: 350.0,
                width: 660.0,
                height: 300.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "view_canvas".into(),
                title: "View Canvas".into(),
                x: 710.0,
                y: 350.0,
                width: 660.0,
                height: 400.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "annotation_panel".into(),
                title: "Annotation Panel".into(),
                x: 30.0,
                y: 770.0,
                width: 660.0,
                height: 300.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "lineage_graph".into(),
                title: "Lineage Graph".into(),
                x: 710.0,
                y: 770.0,
                width: 660.0,
                height: 300.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
        ],
        connections: vec![
            SeedConnection {
                id: "wire-d1".into(),
                from: 0,
                to: 1,
                wire_type: "active".into(),
                label: "dat:importsTo".into(),
            },
            SeedConnection {
                id: "wire-d2".into(),
                from: 0,
                to: 2,
                wire_type: "active".into(),
                label: "dat:usedInPresentation".into(),
            },
            SeedConnection {
                id: "wire-d3".into(),
                from: 2,
                to: 3,
                wire_type: "active".into(),
                label: "dat:rendersToCanvas".into(),
            },
        ],
        panels: vec![SeedPanel {
            panel_type: "aura".into(),
            dock: DockPosition::Right,
        }],
    }
}
