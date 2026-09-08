//! Future seam: `qualia-math` (`solvers/` + CAS today).
//! Native / wasm-scientific only — solvers are not on wasm-ontology.

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod calculus;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod calculus_workbench;
pub mod closure_solvers;
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
mod poly_algebra;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod gemm_host;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod la_app;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod wave14_host;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod wave15_host;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod wave18_host;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod qr_vector;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod special;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod symbolic;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod cas_ext;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod cas_wave5;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod cas_wave6;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod cas_wave7;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod cas_wave8;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod cas_wave9;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod cas_wave10;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod cas_wave12;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod transforms;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod units;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use calculus::integrate as simpson;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use calculus_workbench::compute as calculus_compute;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use ga::{
    angle_between_vectors_host as ga_angle_between_vectors,
    cross_product_host as ga_cross_product, dot as ga_dot,
    normalize_vector_host as ga_normalize_vector,
};
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
pub use qr_vector::{
    add_assign as la_add_assign, add_into as la_add_into, axpy as la_axpy,
    cholesky_solve as la_cholesky_solve, hadamard_assign as la_hadamard_assign,
    hadamard_into as la_hadamard_into, matvec as la_matvec, qr_factor as la_qr_factor,
    qr_form_q_host as la_qr_form_q, qr_solve_least_squares_host as la_qr_solve_least_squares,
    scale as la_scale, symmetric_eigen_3x3 as la_symmetric_eigen_3x3,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use poly_algebra::{
    add as poly_add, coeffs as poly_coeffs, constant as poly_constant, degree as poly_degree,
    derivative as poly_derivative, div_rem as poly_div_rem, eval as poly_eval, gcd as poly_gcd,
    is_zero as poly_is_zero, leading as poly_leading, monic as poly_monic, mul as poly_mul,
    resultant as poly_resultant, scale as poly_scale, sub as poly_sub, zero as poly_zero,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use gemm_host::gemm_host as la_gemm;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use la_app::{
    la_dot, la_identity, la_inverse, la_norm, la_trace,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use wave14_host::{
    bdf1_step_host as calc_bdf1_step, bdf2_step_host as calc_bdf2_step,
    hermite_dense_output_host as calc_hermite_dense_output,
    invariant_drift_host as calc_invariant_drift, pack_f32_pair_host as calc_pack_f32_pair,
    permutation_parity_host as calc_permutation_parity, top_k_host as graph_top_k,
    unpack_f32_pair_host as calc_unpack_f32_pair,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use wave15_host::{
    integrate_bdf_host as calc_integrate_bdf,
    integrate_with_sensitivity_host as calc_integrate_with_sensitivity,
    ruth3_step_host as calc_ruth3_step, verlet_step_host as calc_verlet_step,
    yoshida4_step_host as calc_yoshida4_step,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use wave18_host::{
    adaptive_gauss_kronrod_15_host as calc_adaptive_gauss_kronrod_15,
    canonical_poisson_bracket_host as calc_canonical_poisson_bracket,
    jvp_host as calc_jvp, stormer_verlet_step_host as calc_stormer_verlet_step,
    vjp_host as calc_vjp,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use special::bessel_jn;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use symbolic::{
    curl, differentiate as cas_differentiate, divergence, eval_poly, expand as cas_expand,
    factor as cas_factor, gradient, integrate as cas_integrate, laplacian, limit as cas_limit,
    simplify as cas_simplify, simplify_trig as cas_simplify_trig,
    solve_quadratic as cas_solve_quadratic, taylor_coefficients as cas_taylor_coefficients,
    taylor_eval as cas_taylor_eval,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use cas_ext::{
    classify_second_order_pde as ode_classify_second_order_pde,
    integrate_definite as cas_integrate_definite, limit_at_infinity as cas_limit_at_infinity,
    real_roots as cas_real_roots, solve_linear_first_order as ode_solve_linear_first_order,
    solve_linear_second_order as ode_solve_linear_second_order,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use cas_wave5::{
    expr_citation_hash as cas_expr_citation_hash, roots as cas_roots,
    simplify_with_assumptions as cas_simplify_with_assumptions,
    solve_first_order_linear_pde as ode_solve_first_order_linear_pde,
    solve_linear_system as la_solve_linear_system,
    solve_polynomial_expr as cas_solve_polynomial_expr,
    solve_separable as ode_solve_separable,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use cas_wave6::{
    constructible_from_min_poly_degree as constr_from_min_poly_degree,
    gradient_at as cas_gradient_at, hessian as cas_hessian, hessian_at as cas_hessian_at,
    is_fermat_prime as constr_is_fermat_prime,
    is_regular_polygon_constructible as constr_is_regular_polygon_constructible,
    jacobian as cas_jacobian, partial as cas_partial,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use cas_wave7::{
    doubling_the_cube_constructible as constr_doubling_the_cube,
    factor_quadratic as cas_factor_quadratic,
    is_central_angle_constructible as constr_is_central_angle,
    is_constructible_number as constr_is_constructible_number,
    is_power_of_two as constr_is_power_of_two,
    solve_quadratic_symbolic as cas_solve_quadratic_symbolic,
    squaring_the_circle_constructible as constr_squaring_the_circle,
    trisecting_general_angle_constructible as constr_trisecting_general_angle,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use cas_wave8::{
    cos as cas_cos, exp as cas_exp, ln as cas_ln, neg as cas_neg, pow as cas_pow,
    sin as cas_sin, sqrt as cas_sqrt, tan as cas_tan,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use cas_wave9::{
    add as cas_add, c as cas_c, div as cas_div, mul as cas_mul, sub as cas_sub, var as cas_var,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use cas_wave10::parse as cas_parse;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use cas_wave12::{from_quins as cas_from_quins, to_quins as cas_to_quins};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use transforms::{dft, dft_complex};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use units::convert_unit;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
fn missing(span: vibe::Span, family: &str) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, family))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn gcd(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "NumberTheory")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn lcm(args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    gcd(args, span)
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn is_prime(args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    gcd(args, span)
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn matmul(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn eval_poly(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn simpson(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "NumericalCalculus")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn calculus_compute(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "CalculusWorkbench")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn hill_climb(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "Optimization")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn ga_dot(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "GeometricAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn ga_cross_product(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "GeometricAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn ga_normalize_vector(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "GeometricAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn ga_angle_between_vectors(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "GeometricAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn bessel_jn(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SpecialFunctionsAndTransforms")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn polynomial_roots(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialRoots")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_transpose(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_determinant(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_solve(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_eigen_symmetric(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_eigenvalues(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_svd(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_qr_factor(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_qr_form_q(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_qr_solve_least_squares(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_add_into(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_cholesky_solve(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_axpy(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_hadamard_into(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_add_assign(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_hadamard_assign(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_scale(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_matvec(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_symmetric_eigen_3x3(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_gemm(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_dot(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_norm(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_trace(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_identity(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_inverse(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_div_rem(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_derivative(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_monic(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_resultant(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_add(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_sub(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_mul(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_degree(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_leading(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_is_zero(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_gcd(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_scale(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_eval(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_zero(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn poly_constant(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PolynomialAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_parse(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_to_quins(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_from_quins(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn ode_solve_separable(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicODE")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn ode_solve_first_order_linear_pde(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicODE")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_solve_polynomial_expr(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_simplify_with_assumptions(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_roots(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn la_solve_linear_system(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "LinearAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_expr_citation_hash(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_partial(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_jacobian(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_hessian(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_gradient_at(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_hessian_at(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn constr_is_regular_polygon_constructible(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "Constructibility")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn constr_is_fermat_prime(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "Constructibility")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn constr_from_min_poly_degree(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "Constructibility")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn constr_is_power_of_two(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "Constructibility")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn constr_is_central_angle(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "Constructibility")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn constr_doubling_the_cube(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "Constructibility")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn constr_trisecting_general_angle(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "Constructibility")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn constr_squaring_the_circle(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "Constructibility")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn constr_is_constructible_number(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "Constructibility")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_solve_quadratic_symbolic(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_factor_quadratic(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_pow(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_neg(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_sqrt(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_exp(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_ln(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_sin(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_cos(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_tan(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_c(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_var(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_add(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_sub(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_mul(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_div(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_differentiate(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_simplify(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_simplify_trig(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_integrate(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_taylor_coefficients(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_taylor_eval(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_limit(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_integrate_definite(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_limit_at_infinity(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_real_roots(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn ode_solve_linear_first_order(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicODE")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn ode_solve_linear_second_order(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicODE")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn ode_classify_second_order_pde(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicODE")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_expand(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_factor(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cas_solve_quadratic(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "SymbolicAlgebra")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn dft(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "IntegralTransforms")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn convert_unit(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "PhysicalUnits")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn gradient(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "VectorCalculus")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn divergence(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "VectorCalculus")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn curl(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "VectorCalculus")
}
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn laplacian(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    missing(span, "VectorCalculus")
}
