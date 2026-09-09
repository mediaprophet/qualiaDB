//! QRoute SPF and forwarding generations.
//!
//! Production admission is constraint-first (`validate` → `index` → `plan`).
//! [`compute_spf`] remains the 16-node demonstration / oracle helper.

pub mod constraints;
pub mod flood;
pub mod forwarding;
pub mod index;
pub mod overlap;
pub mod plan;
pub mod select;
pub mod spf;
pub mod validate;

pub use constraints::{edge_feasible, path_feasible, PathConstraint};
pub use flood::{FloodTable, LsaRecord};
pub use forwarding::ForwardingGeneration;
pub use index::{insert_edge, AdjacencyIndex, MAX_ADMITTED_EDGES};
pub use overlap::GenerationPair;
pub use plan::plan_routes;
pub use select::{pareto_select, CandidatePath, PathMetrics, MAX_SELECTED};
pub use spf::{compute_spf, LinkMetric, SpfTable};
pub use validate::{insert, validate_record, TopologyRecord, ValidatedEdge};
