//! Future seam: `qualia-geometry` (`specialized_libs/computational_geometry` today).

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod hull;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use hull::hull2;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn hull2(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputationalGeometry"))
}
