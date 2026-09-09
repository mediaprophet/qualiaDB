//! QRoute SPF and forwarding generations.
//!
//! Production admission is constraint-first (`validate` → `index` → `plan`).
//! [`compute_spf`] remains the 16-node demonstration / oracle helper.
//! Hold-down and untrusted ads are applied after [`plan_routes`], never instead of it.

pub mod ads;
pub mod constraints;
pub mod flood;
pub mod forwarding;
pub mod hysteresis;
pub mod index;
pub mod overlap;
pub mod plan;
pub mod realm;
pub mod select;
pub mod spf;
pub mod validate;

pub use ads::{
    admit_advert, explore_untrusted, AdvertisedMetric, MetricProvenance, PathAdvert, MAX_EXPLORE,
};
pub use constraints::{edge_feasible, path_feasible, PathConstraint};
pub use flood::{FloodTable, LsaRecord};
pub use forwarding::ForwardingGeneration;
pub use hysteresis::{
    admit_healed_routes, decide_replace, hold_planned_routes, hop_is_held_down,
    should_replace, suppress_held_hops, HopHoldDown, HysteresisPolicy,
};
pub use index::{insert_edge, AdjacencyIndex, MAX_ADMITTED_EDGES};
pub use overlap::GenerationPair;
pub use plan::plan_routes;
pub use realm::{authenticate_realm_policy, RealmRouteTable};
pub use select::{pareto_select, CandidatePath, PathMetrics, MAX_SELECTED};
pub use spf::{compute_spf, LinkMetric, SpfTable};
pub use validate::{insert, validate_record, TopologyRecord, ValidatedEdge};
