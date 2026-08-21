//! Future seam: document NLP (`qualia-core-db::nlp` today).

mod analyze;
mod coref;
mod frame;
mod fst;
mod gazetteer;
mod graphrag;
mod relation;
mod substrate;

pub use analyze::analyze;
pub use coref::coref_resolve;
pub use frame::frame_extract;
pub use fst::fst_lookup;
pub use gazetteer::{gazetteer_build, gazetteer_run};
pub use graphrag::graphrag_query;
pub use relation::relation_extract;
pub use substrate::substrate_extract;
