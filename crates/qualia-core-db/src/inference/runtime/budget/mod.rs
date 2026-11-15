//! Joint memory budget planning and checked arithmetic.
//!
//! Provides explicit accounting for static model footprints, hardware runtime context,
//! static graph capture headroom, physical page block geometry, and admission reservations
//! (including input, reserved output, COW transients, and page boundary fragmentation).

pub mod model;
pub mod pools;
pub mod reservation;

#[cfg(test)]
mod tests;

pub use model::ModelMemoryProfile;
pub use pools::{BlockGeometry, MemoryPoolBudget};
pub use reservation::RequestReservation;

pub use etalon_inference::runtime::budget::BudgetError;
