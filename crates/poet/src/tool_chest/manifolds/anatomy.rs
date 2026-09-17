//! Anatomy nested manifold — former bundled Anatomy QApp as a POET surface.

use super::super::core::registry::{ManifoldSeed, SeedConnection, SeedContainer};

pub fn anatomy_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "anatomy".into(),
        label: "Anatomy".into(),
        icon: "health".into(),
        ontology_prefix: "med".into(),
        description: "Anatomy is a manifold on the Health construct. Consent-gated; no fabricated physiology. Not a construct and not a nested EHR."
            .into(),
        containers: vec![
            SeedContainer {
                container_type: "anatomy".into(),
                title: "Anatomy".into(),
                x: 40.0,
                y: 30.0,
                width: 720.0,
                height: 420.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "health_documents".into(),
                title: "Health documents".into(),
                x: 780.0,
                y: 30.0,
                width: 520.0,
                height: 200.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "nested_manifold".into(),
                title: "Return to Health overview".into(),
                x: 780.0,
                y: 250.0,
                width: 520.0,
                height: 200.0,
                z: 1.0,
                honesty: "live".into(),
                target_manifold: "health".into(),
                ..Default::default()
            },
        ],
        connections: vec![SeedConnection {
            id: "wire-anat-docs".into(),
            from: 0,
            to: 1,
            wire_type: "active".into(),
            label: "med:hasDocuments".into(),
        }],
        panels: vec![],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anatomy_is_a_manifold_not_a_qapp_runtime() {
        let seed = anatomy_manifold_seed();
        assert_eq!(seed.id, "anatomy");
        assert!(seed
            .containers
            .iter()
            .any(|c| c.container_type == "anatomy"));
        assert!(seed
            .containers
            .iter()
            .any(|c| c.target_manifold == "health"));
    }
}
