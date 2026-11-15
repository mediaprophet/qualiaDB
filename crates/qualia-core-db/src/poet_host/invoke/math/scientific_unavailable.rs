use vibe::{Diagnostic, Span, Value};

macro_rules! unavailable {
    ($family:literal; $($name:ident),+ $(,)?) => {
        $(pub fn $name(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
            Err(super::super::args::need_scientific(span, $family))
        })+
    };
}

unavailable!("IntegralTransforms"; dft_complex);
unavailable!("PolynomialAlgebra"; poly_coeffs);
unavailable!("Collections"; graph_top_k);
unavailable!("Calculus";
    calc_hermite_dense_output,
    calc_bdf1_step,
    calc_bdf2_step,
    calc_invariant_drift,
    calc_permutation_parity,
    calc_pack_f32_pair,
    calc_unpack_f32_pair,
    calc_verlet_step,
    calc_ruth3_step,
    calc_yoshida4_step,
    calc_integrate_bdf,
    calc_integrate_with_sensitivity,
    calc_canonical_poisson_bracket,
    calc_stormer_verlet_step,
    calc_adaptive_gauss_kronrod_15,
    calc_jvp,
    calc_vjp,
);
