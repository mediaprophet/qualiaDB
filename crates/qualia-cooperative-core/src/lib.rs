//! Transport-neutral cooperative domain shared by the WellFair desktop panels and the
//! (forthcoming) standalone Cooperative Qapp.
//!
//! See `docs/plans/cooperative-qapps-desktop-implementation-plan.md` §8. `Project`,
//! `ProjectMembership`, `Contribution`, and the finance ledger already exist in
//! `wellfare-core` and are re-used as-is (this crate re-exports them for a single import
//! surface). This crate hosts the genuinely-new cooperative domains — work items now;
//! phases, roadmaps, budgets, and agreements next.
//!
//! Numeric and merge discipline (plan §8.4): money in signed integer minor units, effort in
//! integer minutes, records merge by stable id before aggregation, and corrections append
//! superseding records rather than mutating signed history.

pub mod taxonomy;
pub mod authority_type;
pub mod work_item;

/// Re-export the shared record envelope base so cooperative consumers can depend on this
/// crate alone. (The types physically live in `wellfare-core` today.)
pub use wellfare_core::record;

/// Re-export the existing cooperative project + finance domains for a single import surface.
pub use wellfare_core::{finance, projects};
