//! Zero-Heap Fixed-Capacity SOA Archetype ECS (`ecs`).
//!
//! Provides `Tick(pub u32)` change detection, `SimTable<T, CAP>` dense-row SOA storage,
//! and allocation-free `Changed<T>` / `Added<T>` reactive filter iterators.

pub mod query;
pub mod table;
pub mod tick;

pub use query::{Added, Changed};
pub use table::{SimTable, SimTableError, INVALID_ROW};
pub use tick::Tick;
