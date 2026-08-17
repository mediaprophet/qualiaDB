//! Future seam: already a workspace crate (`qualia-vision`). Bindings stay here until extract.

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod ahash;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use ahash::ahash;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn ahash(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputerVision"))
}
