//! capability.invoke.
//!
//! Folders are **future crate seams** (D16). Do not extract workspace crates yet.
//! Add a family by adding a file in the matching seam folder and one match arm.

pub mod ids;
pub mod agent;
mod args;
mod clinical;
pub mod coverage;
mod crypto;
mod docs;
mod econ;
mod engineering;
mod geometry;
mod governance;
mod graph;
mod logic;
mod manifold;
mod math;
mod ml;
mod net;
mod nlp;
mod render;
mod runtime;
mod science;
mod sheet;
mod social;
mod stats;
mod vision;

use super::PoetSnapshot;
use poet_vibe::{DiagCode, Diagnostic, Span, Value};

pub fn dispatch(
    snap: &mut PoetSnapshot,
    id: &str,
    args: &Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    match id {
        ids::DAG_EXECUTE => agent::dag_execute(args, span),
        ids::DAG_VALIDATE => agent::dag_validate(args, span),
        ids::DAG_STATUS => agent::dag_status(args, span),
        ids::DISCOVERY_LIST | "CapabilityDiscovery" | "list_capabilities" => Ok(runtime::list()),
        ids::HASH_IRI => runtime::iri(args, span),
        ids::SHACL_VALIDATE => graph::shacl_validate(snap, args, span),
        ids::SHACL_EXTENSIONS => graph::shacl_extensions(args, span),
        ids::GRAPH_STATS | "get_graph_stats" => Ok(graph::stats(snap)),
        ids::GRAPH_SPARQL => graph::sparql(snap, args, span),
        ids::GRAPH_SHORTEST_PATH => graph::shortest_path(args, span),
        ids::GRAPH_SPREADING_ACTIVATION => graph::spreading_activation(args, span),
        ids::DEONTIC_EVAL => logic::deontic_evaluate(snap, span),
        ids::EPISTEMIC_EVAL => logic::epistemic_evaluate(snap, span),
        ids::PARACONSISTENT_ROUTE => logic::paraconsistent_route(snap, span),
        ids::LTL_GLOBALLY => logic::ltl_globally(snap, args, span),
        ids::LTL_FINALLY => logic::ltl_finally(snap, args, span),
        ids::DL_SUBSUMES => logic::subsumption_check(snap, args, span),
        ids::ASP_ENUMERATE => logic::asp_enumerate(snap, span),
        ids::CAUSAL_CAUSED => logic::caused(snap, args, span),
        ids::FUZZY_TNORM => logic::t_norm(args, span),
        ids::NLP_ANALYZE => nlp::analyze(args, span),
        ids::NT_GCD => math::gcd(args, span),
        ids::NT_LCM => math::lcm(args, span),
        ids::NT_PRIME => math::is_prime(args, span),
        ids::LINALG_MATMUL => math::matmul(args, span),
        ids::LA_TRANSPOSE => math::la_transpose(args, span),
        ids::LA_DET => math::la_determinant(args, span),
        ids::LA_SOLVE => math::la_solve(args, span),
        ids::LA_EIGEN_SYM => math::la_eigen_symmetric(args, span),
        ids::LA_EIGENVALUES => math::la_eigenvalues(args, span),
        ids::LA_SVD => math::la_svd(args, span),
        ids::SYMBOLIC_EVAL => math::eval_poly(args, span),
        ids::CAS_DIFFERENTIATE => math::cas_differentiate(args, span),
        ids::CAS_SIMPLIFY => math::cas_simplify(args, span),
        ids::CAS_EXPAND => math::cas_expand(args, span),
        ids::CAS_FACTOR => math::cas_factor(args, span),
        ids::CAS_SOLVE_QUADRATIC => math::cas_solve_quadratic(args, span),
        ids::CALC_SIMPSON => math::simpson(args, span),
        ids::OPT_HILL => math::hill_climb(args, span),
        ids::GA_DOT => math::ga_dot(args, span),
        ids::SPEC_BESSEL => math::bessel_jn(args, span),
        ids::XFORM_DFT => math::dft(args, span),
        ids::UNITS_CONVERT => math::convert_unit(args, span),
        ids::LA_POLY_ROOTS => math::polynomial_roots(args, span),
        ids::STAT_MEAN => stats::arithmetic_mean(args, span),
        ids::STAT_PEARSON => stats::pearson_r(args, span),
        ids::STAT_LINEAR_REGRESSION => stats::linear_regression(args, span),
        ids::GEOM_HULL2 => geometry::hull2(args, span),
        ids::VISION_AHASH => vision::ahash(args, span),
        ids::ML_OLS => ml::fit_ols(args, span),
        ids::PHYS_PROJECTILE => science::projectile(args, span),
        ids::PHYS_WAVE_1D => science::wave_1d(args, span),
        ids::PHYS_HEAT_DIFFUSION_1D => science::heat_diffusion_1d(args, span),
        ids::PHYS_ADVECTION_DIFFUSION_1D => science::advection_diffusion_1d(args, span),
        ids::PHYS_HARMONIC_OSCILLATOR => science::harmonic_oscillator(args, span),
        ids::PHYS_PENDULUM => science::pendulum(args, span),
        ids::PHYS_N_BODY => science::n_body(args, span),
        ids::PHYS_MOLECULAR_DYNAMICS => science::molecular_dynamics(args, span),
        ids::PHYS_CFD_STEP => science::cfd_step(args, span),
        ids::PHYS_QUANTUM_STATES_1D => science::quantum_states_1d(args, span),
        ids::PHYS_LOGISTIC_GROWTH => science::logistic_growth(args, span),
        ids::PHYS_EMF_INTERFERENCE => science::emf_interference(args, span),
        ids::PHYS_EMF_ATTENUATION => science::emf_attenuation(args, span),
        ids::PHYS_DOPPLER_SHIFT => science::doppler_shift(args, span),
        ids::PHYS_EMF_FIELD_GRID_3D => science::emf_field_grid_3d(args, span),
        ids::PHYS_EMF_SAMPLE_AT_DEPTH => science::emf_sample_at_depth(args, span),
        ids::BIO_ALIGN => science::align(args, span),
        ids::CHEM_SMILES => science::smiles(args, span),
        ids::CLIN_FRAMINGHAM => clinical::framingham(args, span),
        ids::FIN_BS => econ::black_scholes(args, span),
        ids::ENG_KIN => engineering::kinematics(args, span),
        ids::ID_DID_Q42 => governance::parse_did_q42(args, span),
        ids::CRYPTO_SHA256 => crypto::sha256(args, span),
        ids::CRYPTO_SHA512 => crypto::sha512(args, span),
        ids::CRYPTO_BLAKE3 => crypto::blake3(args, span),
        ids::MANIFOLD_DISTANCE => manifold::distance(args, span),
        ids::MANIFOLD_AXES => manifold::axes(args, span),
        ids::MANIFOLD_PROJECT => manifold::project(args, span),
        ids::DOC_INGEST => docs::ingest(args, span),
        ids::SHEET_STATS => sheet::stats(args, span),
        ids::SHEET_SUM => sheet::sum_range(args, span),
        ids::SOCIAL_LWW => social::lww_merge(args, span),
        ids::NET_PEER => net::peer_hash(args, span),
        ids::NET_SONIC => net::sonic_pack(args, span),
        ids::FIN_PORTFOLIO => econ::portfolio_risk(args, span),
        ids::COVERAGE_MATRIX => Ok(coverage::as_value()),
        ids::CATALOG_TTL => Ok(Value::String(crate::poet_host::catalog_ttl::vibe_catalog_ttl())),
        ids::RENDER_SCENE => render::scene(snap, args, span),
        ids::RENDER_CSS_ANIMATION => render::css_animation(args, span),
        ids::RENDER_CSS_COLOR => render::css_color(args, span),
        ids::RENDER_CSS_TRANSFORM => render::css_transform(args, span),
        ids::RENDER_SVG_PATH => render::svg_path(args, span),
        ids::RENDER_SVG_CIRCLE => render::svg_circle(args, span),
        ids::RENDER_SVG_RECT => render::svg_rect(args, span),
        ids::RENDER_SVG_LINE => render::svg_line(args, span),
        ids::RENDER_SVG_BEZIER => render::svg_bezier(args, span),
        ids::RENDER_SVG_FIELD => render::svg_field(args, span),
        ids::SPECTRAL_EMF_TO_SPD => render::spectral::emf_to_spd_fn(args, span),
        ids::SPECTRAL_SPD_TO_XYZ => render::spectral::spd_to_xyz_fn(args, span),
        ids::SPECTRAL_EMF_TO_RGB => render::spectral::emf_to_rgb_fn(args, span),
        ids::SPECTRAL_BLEND => render::spectral::blend_fn(args, span),
        ids::SPECTRAL_GAMUT_MAP => render::spectral::gamut_map_fn(args, span),
        ids::GPU_ADAPTER_INFO => render::gpu_adapter_info(args, span),
        ids::GPU_INIT => render::gpu_init(args, span),
        ids::GPU_RENDER_FRAME => render::gpu_render_frame(args, span),
        ids::GPU_READ_PIXELS => render::gpu_read_pixels(args, span),
        ids::GPU_UPLOAD_MESH => render::gpu_upload_mesh(args, span),
        ids::GPU_UPLOAD_TENSOR => render::gpu_upload_tensor(args, span),
        ids::GPU_SET_CAMERA => render::gpu_set_camera(args, span),
        ids::GPU_PICK => render::gpu_pick(args, span),
        ids::GPU_POLL_PICK => render::gpu_poll_pick(args, span),
        ids::GPU_RESIZE => render::gpu_resize(args, span),
        ids::GPU_SET_AMBIENT => render::gpu_set_ambient(args, span),
        ids::GPU_DESTROY => render::gpu_destroy(args, span),
        ids::GPU_COMPUTE_DISPATCH => render::gpu_compute_dispatch(args, span),
        ids::GPU_COMPUTE_READBACK => render::gpu_compute_readback(args, span),
        ids::GPU_VALIDATE_SHADER => render::gpu_validate_shader(args, span),
        ids::GPU_COMPILE_SHADER => render::gpu_compile_shader(args, span),
        ids::GPU_COMPILE_TO_GLSL => render::gpu_compile_to_glsl(args, span),
        ids::GPU_BACKEND_INFO => render::gpu_backend_info(args, span),
        ids::EMF_UPLOAD_FIELD => render::emf_upload_field(args, span),
        ids::EMF_RENDER_SLICE => render::emf_render_slice(args, span),
        ids::EMF_FIELD_INFO => render::emf_field_info(args, span),
        other => Err(Diagnostic::new(
            DiagCode::E300,
            span,
            format!(
                "capability.invoke({other}): unbound; add poet_host/invoke/<seam>/<family>.rs"
            ),
        )),
    }
}
