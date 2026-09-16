//! SI-05: local dependency resolution, lock, and installed-release registry.

pub mod dependency;
pub mod registry;
pub mod resolver;

pub use dependency::{
    version_matches, CatalogRecord, DependencyKind, DependencyLock, DependencyReq, LockEntry,
};
pub use registry::{InstalledRelease, LocalRegistry};
pub use resolver::plan_resolution;
