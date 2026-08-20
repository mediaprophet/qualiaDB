//! Future seam: `qualia-graph` (`query/`, `sparql_library/`, `daemon_graph`).

mod shacl;
mod sparql;
mod stats;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod activation;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod path;

pub use shacl::{extensions as shacl_extensions, validate as shacl_validate};
pub use sparql::query as sparql;
pub use stats::stats;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use activation::spreading_activation;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use path::shortest_path;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
fn missing(span: poet_vibe::Span, family: &str) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, family))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn shortest_path(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "GraphReasoning")
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn spreading_activation(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "GraphReasoning")
}
