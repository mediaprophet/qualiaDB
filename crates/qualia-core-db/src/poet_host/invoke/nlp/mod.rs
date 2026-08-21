//! Future seam: document NLP (`qualia-core-db::nlp` today).

mod analyze;
mod gazetteer;

pub use analyze::analyze;
pub use gazetteer::{gazetteer_build, gazetteer_run};
