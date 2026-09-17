//! Studio is not a nested DAW. Dual Studio, Scene session, and Audio session
//! sit on this manifold as ordinary POET containers; lighting/transport/meters
//! are tools on those containers.

use super::super::core::registry::{
    DockPosition, ManifoldSeed, SeedConnection, SeedContainer, SeedPanel,
};

pub fn studio_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "studio".into(),
        label: "Studio".into(),
        icon: "studio".into(),
        ontology_prefix: "vis".into(),
        description: "Absorbed into POET: Dual Studio (VibeScript + GPU), a Scene session, \
             and an Audio session. Channel strips, routing matrices, and DCC editors \
             are tools on those containers — not a second DAW/DCC application."
            .into(),
        containers: vec![
            SeedContainer {
                container_type: "dual_studio".into(),
                title: "Dual Studio".into(),
                x: 40.0,
                y: 30.0,
                width: 920.0,
                height: 520.0,
                z: 1.0,
                honesty: "live".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "scene_view".into(),
                title: "Scene session".into(),
                x: 980.0,
                y: 30.0,
                width: 420.0,
                height: 250.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "audio_session".into(),
                title: "Audio session".into(),
                x: 980.0,
                y: 300.0,
                width: 420.0,
                height: 250.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
        ],
        connections: vec![
            SeedConnection {
                id: "wire-studio-scene".into(),
                from: 0,
                to: 1,
                wire_type: "active".into(),
                label: "vis:sceneSession".into(),
            },
            SeedConnection {
                id: "wire-studio-audio".into(),
                from: 0,
                to: 2,
                wire_type: "active".into(),
                label: "aud:audioSession".into(),
            },
        ],
        panels: vec![SeedPanel {
            panel_type: "aura".into(),
            dock: DockPosition::Right,
        }],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn studio_manifold_is_three_sessions_not_a_daw() {
        let seed = studio_manifold_seed();
        assert_eq!(seed.containers.len(), 3);
        let types: Vec<_> = seed
            .containers
            .iter()
            .map(|c| c.container_type.as_str())
            .collect();
        assert_eq!(types, ["dual_studio", "scene_view", "audio_session"]);
        assert!(!types.iter().any(|t| {
            matches!(
                *t,
                "channel_strip" | "routing_matrix" | "meter_bridge" | "desk_surface"
            )
        }));
    }
}
