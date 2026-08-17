//! Future seam: `qualia-science` (`domains/` + physics/chem/bio libraries).

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod bio;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod chem;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod physics;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use bio::align;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use chem::smiles;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use physics::projectile;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn projectile(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "PhysicsAndODE"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn align(
    args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    projectile(args, span)
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn smiles(
    args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    projectile(args, span)
}
