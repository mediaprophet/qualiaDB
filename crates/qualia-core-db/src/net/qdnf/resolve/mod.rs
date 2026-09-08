//! QResolve records and Qualia Scoped Rendezvous.

pub mod qsr;
pub mod records;
pub mod query;

pub use query::{
    compact_hash_is_routing_authority, continuation_resets_budget, lookup_with_budget,
    provider_silence_outcome, same_as_merges_people, unsupported_policy_is_allow, QueryBudget,
};
pub use records::{select_routes, ResolveOutcome, RouteAdvert};
