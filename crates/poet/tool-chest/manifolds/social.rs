//! Social manifold seed — chat graphs, live peers, connection requests.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use super::registry::{ManifoldSeed, SeedContainer, SeedPanel, DockPosition};

/// Social manifold seed: social graph + connection requests + reputation.
pub fn social_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "social".into(),
        label: "Social".into(),
        icon: "users".into(),
        ontology_prefix: "soc".into(),
        description: "Social graph, connection requests, communities, reputation, and vulnerable person protection.".into(),
        containers: vec![
            SeedContainer {
                container_type: "social".into(),
                title: "Social Graph".into(),
                x: 80.0,
                y: 60.0,
                width: 480.0,
                height: 360.0,
                z: 100.0,
                honesty: "live".into(),
            },
            SeedContainer {
                container_type: "connection-requests".into(),
                title: "Connection Requests".into(),
                x: 580.0,
                y: 60.0,
                width: 380.0,
                height: 280.0,
                z: 100.0,
                honesty: "missing".into(),
            },
            SeedContainer {
                container_type: "reputation".into(),
                title: "Reputation".into(),
                x: 580.0,
                y: 360.0,
                width: 380.0,
                height: 200.0,
                z: 100.0,
                honesty: "missing".into(),
            },
        ],
        panels: vec![
            SeedPanel {
                panel_type: "inspector".into(),
                dock: DockPosition::Right,
            },
            SeedPanel {
                panel_type: "pulse-panel".into(),
                dock: DockPosition::Bottom,
            },
        ],
    }
}
