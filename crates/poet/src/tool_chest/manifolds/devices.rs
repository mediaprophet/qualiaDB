//! Devices manifold seed — multi-device workspace management.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::super::core::registry::{ManifoldSeed, SeedConnection, SeedContainer};

pub fn devices_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "devices".into(),
        label: "Devices".into(),
        icon: "devices".into(),
        ontology_prefix: "dev".into(),
        description: "Multi-device workspace: pair devices, assign roles, \
                       sync workspace state, manage multi-monitor layouts."
            .into(),
        containers: vec![
            SeedContainer {
                container_type: "device_manager".into(),
                title: "Device Manager".into(),
                x: 30.0,
                y: 30.0,
                width: 520.0,
                height: 420.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "display_layout".into(),
                title: "Display Layout".into(),
                x: 570.0,
                y: 30.0,
                width: 520.0,
                height: 420.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "workspace_sync".into(),
                title: "Workspace Sync".into(),
                x: 30.0,
                y: 470.0,
                width: 520.0,
                height: 360.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "device_role_assigner".into(),
                title: "Device Role Assigner".into(),
                x: 570.0,
                y: 470.0,
                width: 520.0,
                height: 360.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "remote_control".into(),
                title: "Remote Control".into(),
                x: 30.0,
                y: 860.0,
                width: 1060.0,
                height: 200.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
        ],
        connections: vec![
            SeedConnection {
                id: "dev-to-display".into(),
                from: 0,
                to: 1,
                wire_type: "active".into(),
                label: "displays".into(),
            },
            SeedConnection {
                id: "dev-to-sync".into(),
                from: 0,
                to: 2,
                wire_type: "event".into(),
                label: "syncs".into(),
            },
            SeedConnection {
                id: "dev-to-roles".into(),
                from: 0,
                to: 3,
                wire_type: "active".into(),
                label: "roles".into(),
            },
            SeedConnection {
                id: "roles-to-remote".into(),
                from: 3,
                to: 4,
                wire_type: "event".into(),
                label: "controls".into(),
            },
        ],
        panels: vec![],
    }
}
