//! Future seam: `qualia-social`. Roster/mesh persist is client-core; LWW is engine.

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
mod lww;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod dynamics;

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use lww::merge as lww_merge;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use dynamics::{degree_centrality, gini, lorenz, malfeasance_delta, narrative_divergence};

#[cfg(not(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
)))]
pub fn lww_merge(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(poet_vibe::Diagnostic::new(
        poet_vibe::DiagCode::E300,
        span,
        "Social.lww needs native or wasm-logic",
    ))
}
