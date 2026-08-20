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
mod polynomial;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod special;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod symbolic;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod transforms;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod units;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use calculus::integrate as simpson;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use ga::dot as ga_dot;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use linear::{
    determinant as la_determinant, eigen_symmetric as la_eigen_symmetric,
    eigenvalues as la_eigenvalues, multiply as matmul, solve as la_solve, svd as la_svd,
    transpose as la_transpose,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use number::{gcd, is_prime, lcm};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use optimize::hill_climb;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use polynomial::roots as polynomial_roots;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use special::bessel_jn;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use symbolic::{
    differentiate as cas_differentiate, eval_poly, expand as cas_expand, factor as cas_factor,
    simplify as cas_simplify, solve_quadratic as cas_solve_quadratic,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use transforms::dft;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use units::convert_unit;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
fn missing(span: poet_vibe::Span, family: &str) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
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
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn polynomial_roots(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "PolynomialRoots")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_transpose(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_determinant(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_solve(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_eigen_symmetric(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_eigenvalues(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_svd(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_differentiate(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_simplify(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_expand(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_factor(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_solve_quadratic(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn dft(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "IntegralTransforms")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn convert_unit(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    missing(span, "PhysicalUnits")
}
