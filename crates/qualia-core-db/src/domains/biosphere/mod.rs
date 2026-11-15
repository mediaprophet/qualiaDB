//! Biosphere Domain Engine for QualiaDB.
//!
//! Provides zero-heap living-systems capabilities across fungal mycelial networks,
//! microbial multi-scale colony dynamics, and trophic ecological coupling.
//!
//! # Submodules
//! - [`funga_network`]: Mycelial nutrient transport and sporocarp phenology.
//! - [`microbial_colony`]: Microbial biofilm dynamics and strict scale-band transformation.
//! - [`ecological_coupling`]: Trophic and mycorrhizal nutrient exchange rules.

pub mod ecological_coupling;
pub mod funga_network;
pub mod microbial_colony;

#[allow(ambiguous_glob_reexports)]
pub use ecological_coupling::*;
pub use funga_network::*;
#[allow(ambiguous_glob_reexports)]
pub use microbial_colony::*;
