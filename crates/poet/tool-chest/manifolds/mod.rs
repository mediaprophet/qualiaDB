//! Manifold seeds: initial layouts for each work surface.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

pub mod social;
pub mod settings;
pub mod communications;
pub mod research;
pub mod media;
pub mod vibe;

pub use social::social_manifold_seed;
pub use settings::settings_manifold_seed;
pub use communications::communications_manifold_seed;
pub use research::research_manifold_seed;
pub use media::media_manifold_seed;
pub use vibe::vibe_manifold_seed;

use super::registry::{ManifoldSeed, SeedContainer, SeedPanel, DockPosition};

/// All predefined manifold seeds in display order.
pub fn all_seeds() -> Vec<ManifoldSeed> {
    vec![
        research_manifold_seed(),
        media_manifold_seed(),
        social_manifold_seed(),
        communications_manifold_seed(),
        settings_manifold_seed(),
        vibe_manifold_seed(),
    ]
}
