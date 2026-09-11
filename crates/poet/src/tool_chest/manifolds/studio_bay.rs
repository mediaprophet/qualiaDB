//! First-arrive studio bay — empty teaching lens, not Research.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use super::super::core::registry::ManifoldSeed;

/// Cold-load first paint. Empty on purpose — Frame A sayables teach the room.
pub const STUDIO_BAY_ID: &str = "studio-bay";
pub const STUDIO_BAY_LABEL: &str = "Studio bay";

pub fn studio_bay_manifold_seed() -> ManifoldSeed {
    ManifoldSeed {
        id: STUDIO_BAY_ID.into(),
        label: STUDIO_BAY_LABEL.into(),
        icon: "studio".into(),
        ontology_prefix: String::new(),
        description: "You're in the studio bay. Ask a graph, Keep a volume, Play a cell.".into(),
        containers: vec![],
        connections: vec![],
        panels: vec![],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_arrive_seed_is_empty_studio_bay_not_research() {
        let seed = studio_bay_manifold_seed();
        assert_eq!(seed.id, STUDIO_BAY_ID);
        assert_eq!(seed.label, STUDIO_BAY_LABEL);
        assert!(seed.containers.is_empty());
        assert_ne!(seed.id, "research");
        assert!(!seed.description.contains("qualia."));
        assert!(!seed.description.contains("GraphDatabase"));
    }
}
