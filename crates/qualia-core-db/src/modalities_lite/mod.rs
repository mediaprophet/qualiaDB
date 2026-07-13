//! WASM ontology reasoning kernel.
//!
//! Keep this module list explicit. Adding a module here opts it into the
//! ontology-site WASM/MCP binary and therefore requires a wasm32 build check.

#[path = "../modalities/asp.rs"]
pub mod asp;
#[path = "../modalities/dl.rs"]
pub mod dl;
#[path = "../modalities/epistemic.rs"]
pub mod epistemic;
#[path = "../modalities/interaction_governance.rs"]
pub mod interaction_governance;
#[path = "../modalities/linear.rs"]
pub mod linear;
pub mod logic;
#[path = "../modalities/modal.rs"]
pub mod modal;
#[path = "../modalities/paraconsistent.rs"]
pub mod paraconsistent;
#[path = "../modalities/temporal_ltl.rs"]
pub mod temporal_ltl;
