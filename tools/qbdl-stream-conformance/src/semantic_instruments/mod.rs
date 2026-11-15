//! Exact production semantic types and helpers; no copied test substitutes.
#[path = "../../../../crates/qualia-core-db/src/semantic_instruments/errors.rs"]
pub mod errors;
#[path = "../../../../crates/qualia-core-db/src/semantic_instruments/category.rs"]
pub mod category;
#[path = "../../../../crates/qualia-core-db/src/semantic_instruments/manifest.rs"]
pub mod manifest;
#[path = "../../../../crates/qualia-core-db/src/semantic_instruments/canonical.rs"]
pub mod canonical;
#[path = "../../../../crates/qualia-core-db/src/semantic_instruments/resolve/dependency.rs"]
pub mod dependency;
#[path = "../../../../crates/qualia-core-db/src/semantic_instruments/resolve/bounded_closure/mod.rs"]
pub mod bounded_closure;
pub mod resolve { pub use super::dependency; pub use super::bounded_closure; }
