//! QResolve records and Qualia Scoped Rendezvous.

pub mod qsr;
pub mod records;

pub use records::{select_routes, ResolveOutcome, RouteAdvert};
