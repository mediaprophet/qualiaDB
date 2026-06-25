//! OWL support for QualiaDB.
//!
//! Two distinct concerns, one per submodule:
//!
//! - [`shacl_convert`] — *vocabulary lowering*: parse OWL (RadLex/DICOM healthcare
//!   ontologies in RDF/XML, Turtle, or N3) and emit `sh:NodeShape` graphs for the
//!   Webizen Sentinel, preserving the agency invariant (a `q42:Principal` may *have*
//!   `q42:Thing` possessions but is never itself a Thing).
//! - [`materialize`] — *reasoning*: OWL 2 RL forward-chaining entailment closure
//!   over NQuin-style triples (zero-heap, datalog-style fixpoint), with disjointness
//!   contradiction isolation and property-chain unrolling.
//!
//! The public surface of both submodules is re-exported here so the historical
//! module path `crate::modalities::logic::owl::*` is preserved.

pub mod materialize;
pub mod shacl_convert;

pub use materialize::*;
pub use shacl_convert::*;
