//! Future seam: `qualia-math` (`solvers/` + CAS today).
//! Native / wasm-scientific only — solvers are not on wasm-ontology.

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod calculus;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod ga;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod linear;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod number;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod optimize;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod special;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod symbolic;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use calculus::integrate as simpson;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use ga::dot as ga_dot;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use linear::multiply as matmul;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use number::{gcd, is_prime, lcm};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use optimize::hill_climb;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use special::bessel_jn;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use symbolic::eval_poly;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
fn missing(
    span: poet_vibe::Span,
    family: &str,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, family))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn gcd(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "NumberTheory")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn lcm(
    args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    gcd(args, span)
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn is_prime(
    args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    gcd(args, span)
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn matmul(
    args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn eval_poly(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn simpson(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "NumericalCalculus")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn hill_climb(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "Optimization")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn ga_dot(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "GeometricAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn bessel_jn(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "SpecialFunctionsAndTransforms")
}
