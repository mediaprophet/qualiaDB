//! Health manifold — overview, documents (NLP + library), share, conditions.
//!
//! Not a nested clinical EHR. Record kinds are tools/containers on this
//! manifold. Conditions are possessions of a Principal, not owl:Thing.

use super::super::core::registry::{
    DockPosition, ManifoldSeed, SeedConnection, SeedContainer, SeedPanel,
};

pub fn health_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: "health".into(),
        label: "Health".into(),
        icon: "health".into(),
        ontology_prefix: "med".into(),
        description: "Consent-gated health records on the Semantic Library: classified/secret \
             by default, permissive share to a named clinician DID, NLP ingest of \
             extracted PDF/report text. Not a nested EHR."
            .into(),
        containers: vec![
            SeedContainer {
                container_type: "health_overview".into(),
                title: "Health overview".into(),
                x: 40.0,
                y: 30.0,
                width: 720.0,
                height: 360.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "health_documents".into(),
                title: "Health documents".into(),
                x: 780.0,
                y: 30.0,
                width: 560.0,
                height: 360.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "disclosure_log".into(),
                title: "Share / disclosure".into(),
                x: 40.0,
                y: 410.0,
                width: 720.0,
                height: 280.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "conditions".into(),
                title: "Conditions".into(),
                x: 780.0,
                y: 410.0,
                width: 560.0,
                height: 160.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "health_calculators".into(),
                title: "Clinical calculators".into(),
                x: 40.0,
                y: 710.0,
                width: 720.0,
                height: 360.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "chemical_explorer".into(),
                title: "Compound evidence".into(),
                x: 780.0,
                y: 710.0,
                width: 560.0,
                height: 360.0,
                z: 1.0,
                honesty: "present".into(),
                ..Default::default()
            },
            SeedContainer {
                container_type: "nested_manifold".into(),
                title: "Anatomy manifold".into(),
                x: 780.0,
                y: 590.0,
                width: 560.0,
                height: 100.0,
                z: 1.0,
                honesty: "live".into(),
                target_manifold: "anatomy".into(),
                ..Default::default()
            },
        ],
        connections: vec![
            SeedConnection {
                id: "wire-h-docs".into(),
                from: 0,
                to: 1,
                wire_type: "active".into(),
                label: "med:hasDocuments".into(),
            },
            SeedConnection {
                id: "wire-h-share".into(),
                from: 0,
                to: 2,
                wire_type: "active".into(),
                label: "med:hasDisclosure".into(),
            },
            SeedConnection {
                id: "wire-h-cond".into(),
                from: 0,
                to: 3,
                wire_type: "active".into(),
                label: "med:hasCondition".into(),
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
    fn health_manifold_is_four_sessions_not_an_ehr() {
        let seed = health_manifold_seed();
        assert!(seed.containers.len() >= 4);
        let types: Vec<_> = seed
            .containers
            .iter()
            .map(|c| c.container_type.as_str())
            .collect();
        assert!(types.contains(&"health_overview"));
        assert!(types.contains(&"health_documents"));
        assert!(types.contains(&"disclosure_log"));
        assert!(types.contains(&"conditions"));
        assert!(types.contains(&"health_calculators"));
        assert!(types.contains(&"chemical_explorer"));
        assert!(types.contains(&"nested_manifold"));
        assert!(seed
            .containers
            .iter()
            .any(|c| c.target_manifold == "anatomy"));
    }
}
