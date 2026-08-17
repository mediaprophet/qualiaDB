//! Future seam: `qualia-graph` (`query/`, `sparql_library/`, `daemon_graph`).

mod shacl;
mod sparql;
mod stats;

pub use shacl::{extensions as shacl_extensions, validate as shacl_validate};
pub use sparql::query as sparql;
pub use stats::stats;
