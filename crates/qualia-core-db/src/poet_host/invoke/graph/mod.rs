//! Future seam: `qualia-graph` (`query/`, `sparql_library/`, `daemon_graph`).

mod n3;
mod personhood;
mod shacl;
mod sparql;
mod stats;
mod volume;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod activation;
mod authoring;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod path;

pub use n3::evaluate as n3_evaluate;
pub use shacl::{extensions as shacl_extensions, validate as shacl_validate};
pub use sparql::query as sparql;
pub use stats::stats;
pub use volume::{commit as volume_commit, open as volume_open};

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use activation::spreading_activation;
pub use authoring::author as graph_authoring;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use path::shortest_path;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
fn missing(span: vibe::Span, family: &str) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, family))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn shortest_path(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "GraphReasoning")
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn spreading_activation(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "GraphReasoning")
}
