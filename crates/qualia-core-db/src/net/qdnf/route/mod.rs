//! QRoute SPF and forwarding generations.

pub mod forwarding;
pub mod spf;

pub use forwarding::ForwardingGeneration;
pub use spf::{compute_spf, LinkMetric, SpfTable};
