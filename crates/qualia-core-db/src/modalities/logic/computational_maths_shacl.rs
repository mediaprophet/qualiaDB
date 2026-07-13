//! SHACL constraint surface for the **computational-mathematics engine**.
//!
//! Mirrors, in the SHACL/`SlgOpcode` validation layer, the STEM capability libraries built
//! out under the CALCULUS plan + the computational-engine gap analysis: units & dimensional
//! analysis, number theory &
//! combinatorics, special functions, interpolation, integral transforms, vector calculus,
//! exact/arbitrary-precision arithmetic, the CAS calculus extensions (symbolic integration /
//! series / limits / equation-solving / assumptions / trig / ODE-PDE / multivariable diff),
//! and the general-dimension numerical methods.
//!
//! Each capability has a typed `*Configuration` (compiled to a bounded `Vec<SlgOpcode>` at
//! config time, off the hot path) and a SHACL `NodeShape` in [`get_computational_maths_shacl_ttl`].
//! Wired into the runtime via [`super::shacl::shacl_extension_bridge::append_extension_opcodes`].

use crate::webizen::SlgOpcode;

/// `q42:UnitsConfiguration` — dimensional-analysis / unit-conversion validation.
#[derive(Debug, Clone)]
pub struct UnitsConfiguration {
    /// SI base-dimension vector length (mass, length, time, current, temp, amount, luminous = 7).
    pub dimension_components: u8,
    pub require_dimensional_consistency: bool,
    pub allowed_unit_systems: Vec<String>, // ["si", "cgs", "imperial"]
}

/// `q42:NumberTheoryConfiguration` — primality / factorization / modular bounds.
#[derive(Debug, Clone)]
pub struct NumberTheoryConfiguration {
    pub max_input_bits: u32, // Miller-Rabin / Pollard-rho input bound
    pub max_factorization_iterations: u32,
    pub allowed_operations: Vec<String>, // ["primality","factorization","gcd","totient","modular","combinatorics"]
}

/// `q42:SpecialFunctionConfiguration` — special-function evaluation parameters.
#[derive(Debug, Clone)]
pub struct SpecialFunctionConfiguration {
    pub max_series_terms: u32,
    pub convergence_tolerance: f64,
    pub allowed_families: Vec<String>, // ["bessel","airy","zeta","legendre","chebyshev","hermite","laguerre"]
}

/// `q42:InterpolationConfiguration` — interpolation / approximation parameters.
#[derive(Debug, Clone)]
pub struct InterpolationConfiguration {
    pub max_nodes: u32,
    pub require_distinct_nodes: bool,
    pub allowed_methods: Vec<String>, // ["lagrange","newton","cubic_spline","least_squares"]
}

/// `q42:IntegralTransformConfiguration` — DFT / Laplace / Z transform parameters.
#[derive(Debug, Clone)]
pub struct IntegralTransformConfiguration {
    pub max_samples: u32,
    pub require_invertibility_check: bool,
    pub allowed_transforms: Vec<String>, // ["dft","laplace","ztransform"]
}

/// `q42:VectorCalculusConfiguration` — grad/div/curl + line/surface integral parameters.
#[derive(Debug, Clone)]
pub struct VectorCalculusConfiguration {
    pub max_spatial_dimension: u8, // 2 or 3
    pub require_field_smoothness: bool,
    pub allowed_operators: Vec<String>, // ["gradient","divergence","curl","laplacian","line_integral","surface_integral"]
}

/// `q42:ExactArithmeticConfiguration` — arbitrary-precision integer / rational parameters.
#[derive(Debug, Clone)]
pub struct ExactArithmeticConfiguration {
    pub max_digits: u32,            // precision ceiling
    pub require_exact: bool,        // no silent float fallback
    pub allowed_types: Vec<String>, // ["bigint","bigrational"]
}

/// `q42:SymbolicCalculusConfiguration` — CAS calculus operations (integration, series,
/// limits, equation-solving, ODE/PDE, multivariable differentiation).
#[derive(Debug, Clone)]
pub struct SymbolicCalculusConfiguration {
    pub max_order: u32,                       // Taylor / derivative order ceiling
    pub require_roundtrip_verification: bool, // the honesty gate (d/dx ∘ ∫, residual checks)
    pub allowed_operations: Vec<String>, // ["integrate","series","limit","solve","ode_solve","pde_classify","gradient","jacobian","hessian"]
}

/// `q42:AssumptionConfiguration` — simplify-under-assumptions soundness parameters.
#[derive(Debug, Clone)]
pub struct AssumptionConfiguration {
    pub require_sound_rewrite: bool, // no rewrite unless the sign side-condition is proven
    pub allowed_signs: Vec<String>, // ["positive","nonnegative","negative","nonpositive","nonzero"]
}

/// `q42:NumericalMethodConfiguration` — general-dimension RK4 / Simpson / shooting-BVP.
#[derive(Debug, Clone)]
pub struct NumericalMethodConfiguration {
    pub max_state_dimension: u32,
    pub max_steps: u32,
    pub convergence_tolerance: f64,
    pub allowed_integrators: Vec<String>, // ["rk4","simpson","shooting_bvp"]
}

// ── Opcode generation ──────────────────────────────────────────────────────────

impl UnitsConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(
            self.dimension_components as f64,
        )]
    }
}
impl NumberTheoryConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_input_bits as f64),
            SlgOpcode::CheckMaxInclusive(self.max_factorization_iterations as f64),
        ]
    }
}
impl SpecialFunctionConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_series_terms as f64),
            SlgOpcode::CheckMinInclusive(self.convergence_tolerance),
        ]
    }
}
impl InterpolationConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(self.max_nodes as f64)]
    }
}
impl IntegralTransformConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(self.max_samples as f64)]
    }
}
impl VectorCalculusConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(
            self.max_spatial_dimension as f64,
        )]
    }
}
impl ExactArithmeticConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(self.max_digits as f64)]
    }
}
impl SymbolicCalculusConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(self.max_order as f64)]
    }
}
impl AssumptionConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        // A single representative allowed sign is bound-checked via has-value.
        let first = self.allowed_signs.first().cloned().unwrap_or_default();
        vec![SlgOpcode::CheckHasValue(crate::q_hash(&first))]
    }
}
impl NumericalMethodConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_state_dimension as f64),
            SlgOpcode::CheckMaxInclusive(self.max_steps as f64),
        ]
    }
}

/// Comprehensive SHACL TTL vocabulary for the computational-mathematics engine.
pub fn get_computational_maths_shacl_ttl() -> &'static str {
    r#"
@prefix q42: <https://webizen.org/q42#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

q42:UnitsConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:dimensionComponents ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 7 ;
        sh:message "SI base-dimension vector has 7 components" ;
    ] ;
    sh:property [
        sh:path q42:allowedUnitSystems ;
        sh:in ("si" "cgs" "imperial") ;
        sh:message "Unit system must be supported" ;
    ] .

q42:NumberTheoryConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxInputBits ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 4096 ;
        sh:message "Number-theory input must be within the supported bit width" ;
    ] ;
    sh:property [
        sh:path q42:allowedOperations ;
        sh:in ("primality" "factorization" "gcd" "totient" "modular" "combinatorics") ;
        sh:message "Operation must be a supported number-theory primitive" ;
    ] .

q42:SpecialFunctionConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxSeriesTerms ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 100000 ;
        sh:message "Series truncation must be within bounds" ;
    ] ;
    sh:property [
        sh:path q42:allowedFamilies ;
        sh:in ("bessel" "airy" "zeta" "legendre" "chebyshev" "hermite" "laguerre") ;
        sh:message "Special-function family must be supported" ;
    ] .

q42:InterpolationConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxNodes ;
        sh:datatype xsd:integer ;
        sh:minInclusive 2 ;
        sh:maxInclusive 1000000 ;
        sh:message "Interpolation node count must be between 2 and 1,000,000" ;
    ] ;
    sh:property [
        sh:path q42:allowedMethods ;
        sh:in ("lagrange" "newton" "cubic_spline" "least_squares") ;
        sh:message "Interpolation method must be supported" ;
    ] .

q42:IntegralTransformConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxSamples ;
        sh:datatype xsd:integer ;
        sh:minInclusive 2 ;
        sh:maxInclusive 16777216 ;
        sh:message "Transform sample count must be within bounds" ;
    ] ;
    sh:property [
        sh:path q42:allowedTransforms ;
        sh:in ("dft" "laplace" "ztransform") ;
        sh:message "Transform must be supported" ;
    ] .

q42:VectorCalculusConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxSpatialDimension ;
        sh:datatype xsd:integer ;
        sh:minInclusive 2 ;
        sh:maxInclusive 3 ;
        sh:message "Vector calculus supports 2D and 3D fields" ;
    ] ;
    sh:property [
        sh:path q42:allowedOperators ;
        sh:in ("gradient" "divergence" "curl" "laplacian" "line_integral" "surface_integral") ;
        sh:message "Vector-calculus operator must be supported" ;
    ] .

q42:ExactArithmeticConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxDigits ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000000 ;
        sh:message "Arbitrary-precision digit count must be within bounds" ;
    ] ;
    sh:property [
        sh:path q42:allowedTypes ;
        sh:in ("bigint" "bigrational") ;
        sh:message "Exact-arithmetic type must be supported" ;
    ] .

q42:SymbolicCalculusConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxOrder ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1024 ;
        sh:message "Symbolic calculus order must be within bounds" ;
    ] ;
    sh:property [
        sh:path q42:allowedOperations ;
        sh:in ("integrate" "series" "limit" "solve" "ode_solve" "pde_classify" "gradient" "jacobian" "hessian") ;
        sh:message "Symbolic-calculus operation must be supported" ;
    ] .

q42:AssumptionConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:requireSoundRewrite ;
        sh:datatype xsd:boolean ;
        sh:message "Assumption-gated rewrites must remain sound" ;
    ] ;
    sh:property [
        sh:path q42:allowedSigns ;
        sh:in ("positive" "nonnegative" "negative" "nonpositive" "nonzero") ;
        sh:message "Sign assumption must be a supported domain predicate" ;
    ] .

q42:NumericalMethodConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxStateDimension ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 100000 ;
        sh:message "ODE state dimension must be within bounds" ;
    ] ;
    sh:property [
        sh:path q42:allowedIntegrators ;
        sh:in ("rk4" "simpson" "shooting_bvp") ;
        sh:message "Numerical integrator must be supported" ;
    ] .
"#
}

/// Every NodeShape name in this module's vocabulary (full-coverage assertion target).
pub const COMPUTATIONAL_MATHS_SHAPES: &[&str] = &[
    "q42:UnitsConfigurationShape",
    "q42:NumberTheoryConfigurationShape",
    "q42:SpecialFunctionConfigurationShape",
    "q42:InterpolationConfigurationShape",
    "q42:IntegralTransformConfigurationShape",
    "q42:VectorCalculusConfigurationShape",
    "q42:ExactArithmeticConfigurationShape",
    "q42:SymbolicCalculusConfigurationShape",
    "q42:AssumptionConfigurationShape",
    "q42:NumericalMethodConfigurationShape",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_capability_has_a_shape() {
        let ttl = get_computational_maths_shacl_ttl();
        for shape in COMPUTATIONAL_MATHS_SHAPES {
            assert!(ttl.contains(shape), "missing SHACL shape: {shape}");
        }
    }

    #[test]
    fn every_config_generates_opcodes() {
        assert!(!UnitsConfiguration {
            dimension_components: 7,
            require_dimensional_consistency: true,
            allowed_unit_systems: vec!["si".into()],
        }
        .to_opcodes()
        .is_empty());
        assert!(!NumberTheoryConfiguration {
            max_input_bits: 256,
            max_factorization_iterations: 100_000,
            allowed_operations: vec!["primality".into()],
        }
        .to_opcodes()
        .is_empty());
        assert!(!SpecialFunctionConfiguration {
            max_series_terms: 1000,
            convergence_tolerance: 1e-12,
            allowed_families: vec!["bessel".into()],
        }
        .to_opcodes()
        .is_empty());
        assert!(!InterpolationConfiguration {
            max_nodes: 1024,
            require_distinct_nodes: true,
            allowed_methods: vec!["cubic_spline".into()],
        }
        .to_opcodes()
        .is_empty());
        assert!(!IntegralTransformConfiguration {
            max_samples: 4096,
            require_invertibility_check: true,
            allowed_transforms: vec!["dft".into()],
        }
        .to_opcodes()
        .is_empty());
        assert!(!VectorCalculusConfiguration {
            max_spatial_dimension: 3,
            require_field_smoothness: true,
            allowed_operators: vec!["curl".into()],
        }
        .to_opcodes()
        .is_empty());
        assert!(!ExactArithmeticConfiguration {
            max_digits: 10_000,
            require_exact: true,
            allowed_types: vec!["bigint".into()],
        }
        .to_opcodes()
        .is_empty());
        assert!(!SymbolicCalculusConfiguration {
            max_order: 16,
            require_roundtrip_verification: true,
            allowed_operations: vec!["integrate".into()],
        }
        .to_opcodes()
        .is_empty());
        assert!(!AssumptionConfiguration {
            require_sound_rewrite: true,
            allowed_signs: vec!["positive".into()],
        }
        .to_opcodes()
        .is_empty());
        assert!(!NumericalMethodConfiguration {
            max_state_dimension: 64,
            max_steps: 10_000,
            convergence_tolerance: 1e-9,
            allowed_integrators: vec!["rk4".into()],
        }
        .to_opcodes()
        .is_empty());
    }
}
