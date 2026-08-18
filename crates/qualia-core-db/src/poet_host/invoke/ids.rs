//! Stable capability.invoke ids. Grammar does not grow; these strings do.
//!
//! Folder under `invoke/` is the future crate seam (D16). Do not invent workspace
//! crates until the principal asks to split the monorepo.

#[cfg(test)]
use crate::CAPABILITY_DESCRIPTORS;

pub const DISCOVERY_LIST: &str = "CapabilityDiscovery.list";
pub const SHACL_VALIDATE: &str = "SHACL.validate";
pub const SHACL_EXTENSIONS: &str = "SHACL.extensions";
pub const GRAPH_STATS: &str = "GraphDatabase.stats";
pub const GRAPH_SPARQL: &str = "GraphDatabase.sparql";
pub const DEONTIC_EVAL: &str = "DeonticLogic.evaluate";
pub const EPISTEMIC_EVAL: &str = "EpistemicLogic.evaluate";
pub const PARACONSISTENT_ROUTE: &str = "ParaconsistentLogic.route";
pub const LTL_GLOBALLY: &str = "TemporalAndDescriptionLogic.ltl.globally";
pub const LTL_FINALLY: &str = "TemporalAndDescriptionLogic.ltl.finally";
pub const DL_SUBSUMES: &str = "TemporalAndDescriptionLogic.subsumption";
pub const ASP_ENUMERATE: &str = "SymbolicAndDefeasibleLogic.asp";
pub const CAUSAL_CAUSED: &str = "CausalFuzzyAndControl.caused";
pub const FUZZY_TNORM: &str = "CausalFuzzyAndControl.t_norm";
pub const SYMBOLIC_EVAL: &str = "SymbolicAlgebra.eval";
pub const LINALG_MATMUL: &str = "LinearAlgebra.matmul";
pub const CALC_SIMPSON: &str = "NumericalCalculus.simpson";
pub const OPT_HILL: &str = "Optimization.hill_climb";
pub const GA_DOT: &str = "GeometricAlgebra.dot";
pub const GEOM_HULL2: &str = "ComputationalGeometry.convex_hull_2";
pub const VISION_AHASH: &str = "ComputerVision.ahash";
pub const NT_GCD: &str = "NumberTheory.gcd";
pub const NT_LCM: &str = "NumberTheory.lcm";
pub const NT_PRIME: &str = "NumberTheory.is_prime";
pub const SPEC_BESSEL: &str = "SpecialFunctionsAndTransforms.bessel_j";
pub const STAT_MEAN: &str = "Statistics.mean";
pub const STAT_PEARSON: &str = "Statistics.pearson";
pub const ML_OLS: &str = "MachineLearning.ols";
pub const PHYS_PROJECTILE: &str = "PhysicsAndODE.projectile";
pub const BIO_ALIGN: &str = "Bioinformatics.align";
pub const CHEM_SMILES: &str = "OrganicChemistry.validate_smiles";
pub const CLIN_FRAMINGHAM: &str = "ClinicalRisk.framingham";
pub const FIN_BS: &str = "FinancialModeling.black_scholes";
pub const ENG_KIN: &str = "EngineeringAnalysis.kinematics";
pub const ID_DID_Q42: &str = "ContractsIdentityAndConsensus.parse_did_q42";
pub const CRYPTO_SHA256: &str = "QuantumAndCryptographic.sha256";
pub const NLP_ANALYZE: &str = "nlp.analyze";
pub const HASH_IRI: &str = "hash.iri";
pub const MANIFOLD_DISTANCE: &str = "Manifold.distance";
pub const MANIFOLD_AXES: &str = "Manifold.axes";
pub const MANIFOLD_PROJECT: &str = "Manifold.project";
pub const DOC_INGEST: &str = "Document.ingest";
pub const SHEET_STATS: &str = "Sheet.stats";
pub const SHEET_SUM: &str = "Sheet.sum_range";
pub const SOCIAL_LWW: &str = "Social.lww";
pub const NET_PEER: &str = "Net.peer_hash";
pub const NET_SONIC: &str = "Net.sonic_pack";
pub const FIN_PORTFOLIO: &str = "FinancialModeling.portfolio_risk";
pub const COVERAGE_MATRIX: &str = "CapabilityDiscovery.coverage";
pub const CATALOG_TTL: &str = "CapabilityDiscovery.catalog";
pub const RENDER_SCENE: &str = "Render.scene";
pub const RENDER_CSS_ANIMATION: &str = "Render.css_animation";
pub const RENDER_CSS_COLOR: &str = "Render.css_color";
pub const RENDER_CSS_TRANSFORM: &str = "Render.css_transform";
pub const RENDER_SVG_PATH: &str = "Render.svg_path";
pub const RENDER_SVG_CIRCLE: &str = "Render.svg_circle";
pub const RENDER_SVG_RECT: &str = "Render.svg_rect";
pub const RENDER_SVG_LINE: &str = "Render.svg_line";
pub const RENDER_SVG_BEZIER: &str = "Render.svg_bezier";
pub const RENDER_SVG_FIELD: &str = "Render.svg_field";

// ── Physics wrappers (wrap specialized_libs::physics_simulation) ───────────
pub const PHYS_WAVE_1D: &str = "Physics.wave_1d";
pub const PHYS_HEAT_DIFFUSION_1D: &str = "Physics.heat_diffusion_1d";
pub const PHYS_ADVECTION_DIFFUSION_1D: &str = "Physics.advection_diffusion_1d";
pub const PHYS_HARMONIC_OSCILLATOR: &str = "Physics.harmonic_oscillator";
pub const PHYS_PENDULUM: &str = "Physics.pendulum";
pub const PHYS_N_BODY: &str = "Physics.n_body";
pub const PHYS_MOLECULAR_DYNAMICS: &str = "Physics.molecular_dynamics";
pub const PHYS_CFD_STEP: &str = "Physics.cfd_step";
pub const PHYS_QUANTUM_STATES_1D: &str = "Physics.quantum_states_1d";
pub const PHYS_LOGISTIC_GROWTH: &str = "Physics.logistic_growth";
pub const PHYS_EMF_INTERFERENCE: &str = "Physics.emf_interference";
pub const PHYS_EMF_ATTENUATION: &str = "Physics.emf_attenuation";
pub const PHYS_DOPPLER_SHIFT: &str = "Physics.doppler_shift";
pub const PHYS_EMF_FIELD_GRID_3D: &str = "Physics.emf_field_grid_3d";
pub const PHYS_EMF_SAMPLE_AT_DEPTH: &str = "Physics.emf_sample_at_depth";

// ── Spectral/EMF wrappers (wrap render::spectral_kernel + spectral_blend) ──
pub const SPECTRAL_EMF_TO_SPD: &str = "Spectral.emf_to_spd";
pub const SPECTRAL_SPD_TO_XYZ: &str = "Spectral.spd_to_xyz";
pub const SPECTRAL_EMF_TO_RGB: &str = "Spectral.emf_to_rgb";
pub const SPECTRAL_BLEND: &str = "Spectral.blend";
pub const SPECTRAL_GAMUT_MAP: &str = "Spectral.gamut_map";

// ── Linear algebra extensions (wrap solvers::linear_algebra) ──────────────
pub const LA_TRANSPOSE: &str = "LinearAlgebra.transpose";
pub const LA_DET: &str = "LinearAlgebra.determinant";
pub const LA_SOLVE: &str = "LinearAlgebra.solve";
pub const LA_EIGEN_SYM: &str = "LinearAlgebra.eigen_symmetric";
pub const LA_EIGENVALUES: &str = "LinearAlgebra.eigenvalues";
pub const LA_SVD: &str = "LinearAlgebra.svd";
pub const LA_POLY_ROOTS: &str = "LinearAlgebra.polynomial_roots";

// ── CAS extensions (wrap specialized_libs::symbolic_algebra) ──────────────
pub const CAS_DIFFERENTIATE: &str = "SymbolicAlgebra.differentiate";
pub const CAS_SIMPLIFY: &str = "SymbolicAlgebra.simplify";
pub const CAS_EXPAND: &str = "SymbolicAlgebra.expand";
pub const CAS_FACTOR: &str = "SymbolicAlgebra.factor";
pub const CAS_SOLVE_QUADRATIC: &str = "SymbolicAlgebra.solve_quadratic";

// ── Crypto extensions (wrap sha2 / blake3) ────────────────────────────────
pub const CRYPTO_SHA512: &str = "QuantumAndCryptographic.sha512";
pub const CRYPTO_BLAKE3: &str = "QuantumAndCryptographic.blake3";

// ── Stats extension (wrap solvers::statistics::regression) ────────────────
pub const STAT_LINEAR_REGRESSION: &str = "Statistics.linear_regression";

// ── Integral transforms (wrap solvers::transforms::fourier) ───────────────
pub const XFORM_DFT: &str = "IntegralTransforms.dft";

// ── Physical units (wrap solvers::units::conversion) ──────────────────────
pub const UNITS_CONVERT: &str = "PhysicalUnits.convert";

// ── Graph reasoning (wrap solvers::graph_opt) ─────────────────────────────
pub const GRAPH_SHORTEST_PATH: &str = "GraphReasoning.shortest_path";
pub const GRAPH_SPREADING_ACTIVATION: &str = "GraphReasoning.spreading_activation";

pub const ALL_BOUND: &[&str] = &[
    DISCOVERY_LIST,
    SHACL_VALIDATE,
    SHACL_EXTENSIONS,
    GRAPH_STATS,
    GRAPH_SPARQL,
    DEONTIC_EVAL,
    EPISTEMIC_EVAL,
    PARACONSISTENT_ROUTE,
    LTL_GLOBALLY,
    LTL_FINALLY,
    DL_SUBSUMES,
    ASP_ENUMERATE,
    CAUSAL_CAUSED,
    FUZZY_TNORM,
    SYMBOLIC_EVAL,
    LINALG_MATMUL,
    CALC_SIMPSON,
    OPT_HILL,
    GA_DOT,
    GEOM_HULL2,
    VISION_AHASH,
    NT_GCD,
    NT_LCM,
    NT_PRIME,
    SPEC_BESSEL,
    STAT_MEAN,
    STAT_PEARSON,
    ML_OLS,
    PHYS_PROJECTILE,
    BIO_ALIGN,
    CHEM_SMILES,
    CLIN_FRAMINGHAM,
    FIN_BS,
    ENG_KIN,
    ID_DID_Q42,
    CRYPTO_SHA256,
    NLP_ANALYZE,
    HASH_IRI,
    MANIFOLD_DISTANCE,
    MANIFOLD_AXES,
    MANIFOLD_PROJECT,
    DOC_INGEST,
    SHEET_STATS,
    SHEET_SUM,
    SOCIAL_LWW,
    NET_PEER,
    NET_SONIC,
    FIN_PORTFOLIO,
    COVERAGE_MATRIX,
    CATALOG_TTL,
    RENDER_SCENE,
    RENDER_CSS_ANIMATION,
    RENDER_CSS_COLOR,
    RENDER_CSS_TRANSFORM,
    RENDER_SVG_PATH,
    RENDER_SVG_CIRCLE,
    RENDER_SVG_RECT,
    RENDER_SVG_LINE,
    RENDER_SVG_BEZIER,
    RENDER_SVG_FIELD,
    LA_TRANSPOSE,
    LA_DET,
    LA_SOLVE,
    LA_EIGEN_SYM,
    LA_EIGENVALUES,
    LA_SVD,
    LA_POLY_ROOTS,
    CAS_DIFFERENTIATE,
    CAS_SIMPLIFY,
    CAS_EXPAND,
    CAS_FACTOR,
    CAS_SOLVE_QUADRATIC,
    CRYPTO_SHA512,
    CRYPTO_BLAKE3,
    STAT_LINEAR_REGRESSION,
    XFORM_DFT,
    UNITS_CONVERT,
    GRAPH_SHORTEST_PATH,
    GRAPH_SPREADING_ACTIVATION,
    PHYS_WAVE_1D,
    PHYS_HEAT_DIFFUSION_1D,
    PHYS_ADVECTION_DIFFUSION_1D,
    PHYS_HARMONIC_OSCILLATOR,
    PHYS_PENDULUM,
    PHYS_N_BODY,
    PHYS_MOLECULAR_DYNAMICS,
    PHYS_CFD_STEP,
    PHYS_QUANTUM_STATES_1D,
    PHYS_LOGISTIC_GROWTH,
    PHYS_EMF_INTERFERENCE,
    PHYS_EMF_ATTENUATION,
    PHYS_DOPPLER_SHIFT,
    PHYS_EMF_FIELD_GRID_3D,
    PHYS_EMF_SAMPLE_AT_DEPTH,
    SPECTRAL_EMF_TO_SPD,
    SPECTRAL_SPD_TO_XYZ,
    SPECTRAL_EMF_TO_RGB,
    SPECTRAL_BLEND,
    SPECTRAL_GAMUT_MAP,
];

/// Future extract target for an invoke id. Not a crate today.
pub fn seam_for(id: &str) -> &'static str {
    match id {
        DISCOVERY_LIST | HASH_IRI | COVERAGE_MATRIX | CATALOG_TTL => "runtime",
        SHACL_VALIDATE | SHACL_EXTENSIONS | GRAPH_STATS | GRAPH_SPARQL
        | GRAPH_SHORTEST_PATH | GRAPH_SPREADING_ACTIVATION => "graph",
        DEONTIC_EVAL | EPISTEMIC_EVAL | PARACONSISTENT_ROUTE | LTL_GLOBALLY | LTL_FINALLY
        | DL_SUBSUMES | ASP_ENUMERATE | CAUSAL_CAUSED | FUZZY_TNORM => "logic",
        NLP_ANALYZE => "nlp",
        NT_GCD | NT_LCM | NT_PRIME | LINALG_MATMUL | SYMBOLIC_EVAL | CALC_SIMPSON | OPT_HILL
        | GA_DOT | SPEC_BESSEL | LA_TRANSPOSE | LA_DET | LA_SOLVE | LA_EIGEN_SYM
        | LA_EIGENVALUES | LA_SVD | LA_POLY_ROOTS | CAS_DIFFERENTIATE | CAS_SIMPLIFY
        | CAS_EXPAND | CAS_FACTOR | CAS_SOLVE_QUADRATIC | XFORM_DFT | UNITS_CONVERT => "math",
        STAT_MEAN | STAT_PEARSON | STAT_LINEAR_REGRESSION => "stats",
        GEOM_HULL2 => "geometry",
        VISION_AHASH => "vision",
        ML_OLS => "ml",
        PHYS_PROJECTILE | BIO_ALIGN | CHEM_SMILES => "science",
        PHYS_WAVE_1D | PHYS_HEAT_DIFFUSION_1D | PHYS_ADVECTION_DIFFUSION_1D
        | PHYS_HARMONIC_OSCILLATOR | PHYS_PENDULUM | PHYS_N_BODY | PHYS_MOLECULAR_DYNAMICS
        | PHYS_CFD_STEP | PHYS_QUANTUM_STATES_1D | PHYS_LOGISTIC_GROWTH
        | PHYS_EMF_INTERFERENCE | PHYS_EMF_ATTENUATION | PHYS_DOPPLER_SHIFT
        | PHYS_EMF_FIELD_GRID_3D | PHYS_EMF_SAMPLE_AT_DEPTH => "physics",
        SPECTRAL_EMF_TO_SPD | SPECTRAL_SPD_TO_XYZ | SPECTRAL_EMF_TO_RGB | SPECTRAL_BLEND
        | SPECTRAL_GAMUT_MAP => "spectral",
        CLIN_FRAMINGHAM => "clinical",
        FIN_BS => "econ",
        ENG_KIN => "engineering",
        ID_DID_Q42 => "governance",
        CRYPTO_SHA256 | CRYPTO_SHA512 | CRYPTO_BLAKE3 => "crypto",
        MANIFOLD_DISTANCE | MANIFOLD_AXES | MANIFOLD_PROJECT => "manifold",
        DOC_INGEST => "docs",
        SHEET_STATS | SHEET_SUM => "sheet",
        SOCIAL_LWW => "social",
        NET_PEER | NET_SONIC => "net",
        FIN_PORTFOLIO => "econ",
        RENDER_SCENE | RENDER_CSS_ANIMATION | RENDER_CSS_COLOR | RENDER_CSS_TRANSFORM
        | RENDER_SVG_PATH | RENDER_SVG_CIRCLE | RENDER_SVG_RECT | RENDER_SVG_LINE
        | RENDER_SVG_BEZIER | RENDER_SVG_FIELD => "render",
        _ => "unbound",
    }
}

/// Every `CAPABILITY_DESCRIPTORS` family has at least one bound invoke id.
pub fn family_bound(name: &str) -> bool {
    ALL_BOUND.iter().any(|id| id.starts_with(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_descriptor_family_has_an_invoke() {
        for d in CAPABILITY_DESCRIPTORS {
            assert!(
                family_bound(d.name),
                "family {} has no capability.invoke id — add invoke/<seam>/<family>.rs",
                d.name
            );
        }
    }

    #[test]
    fn seams_are_named_extract_targets() {
        assert_eq!(seam_for(DEONTIC_EVAL), "logic");
        assert_eq!(seam_for(PHYS_PROJECTILE), "science");
        assert_eq!(seam_for(VISION_AHASH), "vision");
        assert_eq!(seam_for(ML_OLS), "ml");
        assert_eq!(seam_for("DoesNotExist.nope"), "unbound");
    }
}
