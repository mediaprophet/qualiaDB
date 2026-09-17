//! Rights & wallet manifold seed — agreements, identity, wallet.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::super::core::registry::{
    DockPosition, ManifoldSeed, SeedConnection, SeedContainer, SeedPanel,
};

pub fn rights_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "rights".into(),
        label: "Rights & Wallet".into(),
        icon: "rights".into(),
        ontology_prefix: "rights".into(),
        description: "Rights & Agreements (agreements, deontic norms, jural relations, breach log, consents), Wallet (balances, ILP/Lightning/XEC, tax suite, compute costs)."
            .into(),
        containers: vec![
            SeedContainer {
                container_type: "rights".into(),
                title: "Rights & Agreements".into(),
                x: 30.0,
                y: 30.0,
                width: 460.0,
                height: 380.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "wallet".into(),
                title: "Wallet".into(),
                x: 510.0,
                y: 30.0,
                width: 420.0,
                height: 380.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "pulse".into(),
                title: "Rights Activity Pulse".into(),
                x: 30.0,
                y: 430.0,
                width: 900.0,
                height: 180.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
        ],
        connections: vec![
            SeedConnection {
                id: "wire-rw1".into(),
                from: 0,
                to: 1,
                wire_type: "active".into(),
                label: "rights:hasWallet".into(),
            },
            SeedConnection {
                id: "wire-rw2".into(),
                from: 0,
                to: 2,
                wire_type: "subtle".into(),
                label: "rights:auditedBy".into(),
            },
        ],
        panels: vec![SeedPanel {
            panel_type: "aura".into(),
            dock: DockPosition::Right,
        }],
        ..Default::default()
    }
}
