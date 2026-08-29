//! Manifold seeds: initial layouts for each work surface.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

pub mod anatomy;
pub mod communications;
pub mod datasets;
pub mod devices;
pub mod health;
pub mod knowledge;
pub mod media;
pub mod ontology;
pub mod projects;
pub mod research;
pub mod rights;
pub mod sanctuary;
pub mod settings;
pub mod social;
pub mod studio;
pub mod vibe;

pub use anatomy::anatomy_manifold_seed;
pub use communications::communications_manifold_seed;
pub use datasets::datasets_manifold_seed;
pub use devices::devices_manifold_seed;
pub use health::health_manifold_seed;
pub use knowledge::knowledge_manifold_seed;
pub use media::media_manifold_seed;
pub use ontology::ontology_manifold_seed;
pub use projects::projects_manifold_seed;
pub use research::research_manifold_seed;
pub use rights::rights_manifold_seed;
pub use sanctuary::sanctuary_manifold_seed;
pub use settings::settings_manifold_seed;
pub use social::social_manifold_seed;
pub use studio::studio_manifold_seed;
pub use vibe::vibe_manifold_seed;

use super::core::registry::ManifoldSeed;

/// All predefined manifold seeds in display order.
pub fn all_seeds() -> Vec<ManifoldSeed> {
    vec![
        research_manifold_seed(),
        media_manifold_seed(),
        social_manifold_seed(),
        communications_manifold_seed(),
        knowledge_manifold_seed(),
        ontology_manifold_seed(),
        projects_manifold_seed(),
        rights_manifold_seed(),
        sanctuary_manifold_seed(),
        health_manifold_seed(),
        anatomy_manifold_seed(),
        studio_manifold_seed(),
        datasets_manifold_seed(),
        settings_manifold_seed(),
        devices_manifold_seed(),
        vibe_manifold_seed(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anatomy_is_a_seeded_manifold() {
        let seeds = all_seeds();
        assert!(seeds.iter().any(|s| s.id == "anatomy"));
        assert!(seeds.iter().any(|s| s.id == "health"));
        assert_ne!(
            seeds
                .iter()
                .find(|s| s.id == "anatomy")
                .map(|s| s.id.as_str()),
            Some("health")
        );
    }

    #[test]
    fn projects_social_health_personal() {
        let projects = projects_manifold_seed();
        let health = health_manifold_seed();
        assert!(projects.is_social());
        assert!(!health.is_social());
        assert!(social_manifold_seed().is_social());
        assert!(!anatomy_manifold_seed().is_social());
    }
}
