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
pub const BIOSIGNAL_DP_FILTER: &str = "biosignal.dp_filter";
pub const BIOSIGNAL_DP_CONFIG: &str = "biosignal.dp_config";
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

// ── WebGPU invoke surface (wraps render::gpu::PortalGpu) ──────────────────
pub const GPU_ADAPTER_INFO: &str = "Render.gpu_adapter_info";
pub const GPU_INIT: &str = "Render.gpu_init";
pub const GPU_RENDER_FRAME: &str = "Render.gpu_render_frame";
pub const GPU_READ_PIXELS: &str = "Render.gpu_read_pixels";
pub const GPU_UPLOAD_MESH: &str = "Render.gpu_upload_mesh";
pub const GPU_UPLOAD_TENSOR: &str = "Render.gpu_upload_tensor";
pub const GPU_SET_CAMERA: &str = "Render.gpu_set_camera";
pub const GPU_PICK: &str = "Render.gpu_pick";
pub const GPU_POLL_PICK: &str = "Render.gpu_poll_pick";
pub const GPU_RESIZE: &str = "Render.gpu_resize";
pub const GPU_SET_AMBIENT: &str = "Render.gpu_set_ambient";
pub const GPU_DESTROY: &str = "Render.gpu_destroy";
pub const GPU_COMPUTE_DISPATCH: &str = "Render.gpu_compute_dispatch";
pub const GPU_COMPUTE_READBACK: &str = "Render.gpu_compute_readback";
pub const GPU_VALIDATE_SHADER: &str = "Render.gpu_validate_shader";
pub const GPU_COMPILE_SHADER: &str = "Render.gpu_compile_shader";
pub const GPU_COMPILE_TO_GLSL: &str = "Render.gpu_compile_to_glsl";
pub const GPU_BACKEND_INFO: &str = "Render.gpu_backend_info";
pub const EMF_UPLOAD_FIELD: &str = "Render.emf_upload_field";
pub const EMF_RENDER_SLICE: &str = "Render.emf_render_slice";
pub const EMF_FIELD_INFO: &str = "Render.emf_field_info";

// ── GBNF constrained sampler (T53/W11) ─────────────────────────────────────
pub const SAMPLER_CONFIGURE: &str = "sampler.configure";
pub const SAMPLER_CONSTRAIN_ENABLE: &str = "sampler.constrain_enable";
pub const SAMPLER_CONSTRAIN_DISABLE: &str = "sampler.constrain_disable";
pub const SAMPLER_CONSTRAIN_RESET: &str = "sampler.constrain_reset";
pub const SAMPLER_SAMPLE: &str = "sampler.sample";

// ── Asset aspect sub-graphs (spatial assets with temporal assertions) ─────
pub const ASSET_CREATE: &str = "Asset.create";
pub const ASSET_ADD_TEMPORAL: &str = "Asset.add_temporal";
pub const ASSET_ADD_TOPIC: &str = "Asset.add_topic";
pub const ASSET_SET_SPATIAL: &str = "Asset.set_spatial";
pub const ASSET_COMPILE: &str = "Asset.compile";
pub const ASSET_TEMPORAL_SPAN: &str = "Asset.temporal_span";
pub const ASSET_QUERY_ASPECTS: &str = "Asset.query_aspects";

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

// ── Agent DAG orchestration (R3) ──────────────────────────────────────────
pub const DAG_EXECUTE: &str = "agent.dag.execute";
pub const DAG_VALIDATE: &str = "agent.dag.validate";
pub const DAG_STATUS: &str = "agent.dag.status";

// ── Cosmic coordinate system (OCS) bindings ───────────────────────────────
pub const COSMIC_GEODETIC_TO_ECEF: &str = "Cosmic.geodetic_to_ecef";
pub const COSMIC_ECEF_TO_GEODETIC: &str = "Cosmic.ecef_to_geodetic";
pub const COSMIC_ECEF_TO_ENU: &str = "Cosmic.ecef_to_enu";
pub const COSMIC_ENU_TO_ECEF: &str = "Cosmic.enu_to_ecef";
pub const COSMIC_GEODETIC_DISTANCE: &str = "Cosmic.geodetic_distance";
pub const COSMIC_BODY_PROFILE: &str = "Cosmic.body_profile";
pub const COSMIC_SURFACE_GRAVITY: &str = "Cosmic.surface_gravity";
pub const COSMIC_FLRW_DISTANCE: &str = "Cosmic.flrw_distance";
pub const COSMIC_FLRW_REDSHIFT: &str = "Cosmic.flrw_redshift";
pub const COSMIC_FLRW_HUBBLE_VELOCITY: &str = "Cosmic.flrw_hubble_velocity";
pub const COSMIC_STARDATE_TO_GREGORIAN: &str = "Cosmic.stardate_to_gregorian";
pub const COSMIC_WARP_VELOCITY: &str = "Cosmic.warp_velocity";
pub const COSMIC_COCHRANE_UNITS: &str = "Cosmic.cochrane_units";
pub const COSMIC_ATMOSPHERE_PRESSURE: &str = "Cosmic.atmosphere_pressure";
pub const COSMIC_ATMOSPHERE_TEMPERATURE: &str = "Cosmic.atmosphere_temperature";
pub const COSMIC_MAGNETOSPHERE_FIELD: &str = "Cosmic.magnetosphere_field";
pub const COSMIC_SCALE_FACTOR: &str = "Cosmic.scale_factor";
pub const COSMIC_COMPTON_WAVELENGTH: &str = "Cosmic.compton_wavelength";
pub const COSMIC_DE_BROGLIE: &str = "Cosmic.de_broglie_wavelength";
pub const COSMIC_USRI_PARSE: &str = "Cosmic.usri_parse";

// ── N1: Expose-only bindings for Poet interface gap closure ────────────────
pub const NLP_GAZETTEER_RUN: &str = "NLP.gazetteer_run";
pub const NLP_GAZETTEER_BUILD: &str = "NLP.gazetteer_build";
pub const INFERENCE_EMBED: &str = "Inference.embed";
pub const INFERENCE_GROUNDING: &str = "Inference.grounding";
pub const INFERENCE_VERIFY_TURN: &str = "Inference.verify_turn";
pub const INFERENCE_DETECT_UNGROUNDED: &str = "Inference.detect_ungrounded";
pub const FINANCE_CONVERT_CURRENCY: &str = "Finance.convert_currency";
pub const FINANCE_MULTISIG_CHECK: &str = "Finance.multisig_check";
pub const FINANCE_LEDGER_BALANCE: &str = "Finance.ledger_balance";
pub const CAPABILITY_GRANT: &str = "Capability.grant";
pub const CAPABILITY_REVOKE: &str = "Capability.revoke";
pub const CAPABILITY_TEST_GATING: &str = "Capability.test_gating";
pub const CAPABILITY_AUDIT: &str = "Capability.audit";
pub const SENTINEL_INSPECT: &str = "Sentinel.inspect";
pub const SENTINEL_GATE: &str = "Sentinel.gate";
pub const AGENT_TRACE: &str = "Agent.trace";
pub const AGENT_VERIFY: &str = "Agent.verify";
pub const IDENTITY_CURRENT_USER: &str = "Identity.current_user";
pub const AUDIO_SPECTRUM: &str = "Audio.spectrum";
pub const SCENE_CREATE: &str = "Scene.create";
pub const SCENE_ADD_NODE: &str = "Scene.add_node";
pub const SCENE_SET_TRANSFORM: &str = "Scene.set_transform";
pub const SCENE_SET_MESH: &str = "Scene.set_mesh";
pub const SCENE_ADD_CAMERA: &str = "Scene.add_camera";
pub const SCENE_RENDER: &str = "Scene.render";
pub const SCENE_SET_VIEWPORT: &str = "Scene.set_viewport";
pub const SCENE_SET_CLEAR_COLOUR: &str = "Scene.set_clear_colour";
pub const SCENE_CAPTURE_FRAME: &str = "Scene.capture_frame";

// ── N3: FST morphology, coreference, frames, relations, substrate, graphrag ─
pub const NLP_FST_LOOKUP: &str = "NLP.fst_lookup";
pub const NLP_COREF_RESOLVE: &str = "NLP.coref_resolve";
pub const NLP_FRAME_EXTRACT: &str = "NLP.frame_extract";
pub const NLP_RELATION_EXTRACT: &str = "NLP.relation_extract";
pub const NLP_SUBSTRATE_EXTRACT: &str = "NLP.substrate_extract";
pub const NLP_GRAPHRAG_QUERY: &str = "NLP.graphrag_query";

// ── N2: Partial extensions — social dynamics, forensic economics ───────────
pub const SOCIAL_GINI: &str = "Social.gini";
pub const SOCIAL_LORENZ: &str = "Social.lorenz";
pub const SOCIAL_DEGREE_CENTRALITY: &str = "Social.degree_centrality";
pub const FORENSIC_MALFEASANCE_DELTA: &str = "Forensic.malfeasance_delta";
pub const FORENSIC_NARRATIVE_DIVERGENCE: &str = "Forensic.narrative_divergence";

// ── N4: Agent runtime build-new — planner, corpus, evaluator, agency ──────
pub const AGENT_PLAN: &str = "Agent.plan";
pub const AGENT_EXECUTE: &str = "Agent.execute";
pub const AGENT_EVALUATE: &str = "Agent.evaluate";
pub const CORPUS_LOAD: &str = "Corpus.load";
pub const CORPUS_PARSE: &str = "Corpus.parse";
pub const AGENCY_EVALUATE: &str = "Agency.evaluate";

// ── N5: Neural / LLM inference — load, unload, transformer, classifier, reranker ─
pub const INFERENCE_LOAD_MODEL: &str = "Inference.load_model";
pub const INFERENCE_UNLOAD_MODEL: &str = "Inference.unload_model";
pub const INFERENCE_RUN_TRANSFORMER: &str = "Inference.run_transformer";
pub const INFERENCE_RUN_CLASSIFIER: &str = "Inference.run_classifier";
pub const INFERENCE_RUN_RERANKER: &str = "Inference.run_reranker";
pub const INFERENCE_VECTOR_SEARCH: &str = "Inference.vector_search";
pub const INFERENCE_CONSTRAINED_DECODE: &str = "Inference.constrained_decode";

// ── N6: Audio DAW — oscillator, envelope, filter, LFO, effects, MIDI, transport, meters ─
pub const AUDIO_OSCILLATOR: &str = "Audio.oscillator";
pub const AUDIO_ENVELOPE: &str = "Audio.envelope";
pub const AUDIO_FILTER: &str = "Audio.filter";
pub const AUDIO_LFO: &str = "Audio.lfo";
pub const AUDIO_DELAY: &str = "Audio.delay";
pub const AUDIO_REVERB: &str = "Audio.reverb";
pub const AUDIO_COMPRESSOR: &str = "Audio.compressor";
pub const AUDIO_EQ: &str = "Audio.eq";
pub const AUDIO_MIDI_NOTE: &str = "Audio.midi_note";
pub const AUDIO_QUANTIZE: &str = "Audio.quantize";
pub const AUDIO_TRANSPOSE: &str = "Audio.transpose";
pub const AUDIO_TRANSPORT: &str = "Audio.transport";
pub const AUDIO_WAVEFORM_METER: &str = "Audio.waveform_meter";
pub const AUDIO_PHASE_METER: &str = "Audio.phase_meter";
pub const AUDIO_LOUDNESS_METER: &str = "Audio.loudness_meter";

pub const ALL_BOUND: &[&str] = &[
    DAG_EXECUTE,
    DAG_VALIDATE,
    DAG_STATUS,
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
    GPU_ADAPTER_INFO,
    GPU_INIT,
    GPU_RENDER_FRAME,
    GPU_READ_PIXELS,
    GPU_UPLOAD_MESH,
    GPU_UPLOAD_TENSOR,
    GPU_SET_CAMERA,
    GPU_PICK,
    GPU_POLL_PICK,
    GPU_RESIZE,
    GPU_SET_AMBIENT,
    GPU_DESTROY,
    GPU_COMPUTE_DISPATCH,
    GPU_COMPUTE_READBACK,
    GPU_VALIDATE_SHADER,
    GPU_COMPILE_SHADER,
    GPU_COMPILE_TO_GLSL,
    GPU_BACKEND_INFO,
    EMF_UPLOAD_FIELD,
    EMF_RENDER_SLICE,
    EMF_FIELD_INFO,
    SAMPLER_CONFIGURE,
    SAMPLER_CONSTRAIN_ENABLE,
    SAMPLER_CONSTRAIN_DISABLE,
    SAMPLER_CONSTRAIN_RESET,
    SAMPLER_SAMPLE,
    ASSET_CREATE,
    ASSET_ADD_TEMPORAL,
    ASSET_ADD_TOPIC,
    ASSET_SET_SPATIAL,
    ASSET_COMPILE,
    ASSET_TEMPORAL_SPAN,
    ASSET_QUERY_ASPECTS,
    COSMIC_GEODETIC_TO_ECEF,
    COSMIC_ECEF_TO_GEODETIC,
    COSMIC_ECEF_TO_ENU,
    COSMIC_ENU_TO_ECEF,
    COSMIC_GEODETIC_DISTANCE,
    COSMIC_BODY_PROFILE,
    COSMIC_SURFACE_GRAVITY,
    COSMIC_FLRW_DISTANCE,
    COSMIC_FLRW_REDSHIFT,
    COSMIC_FLRW_HUBBLE_VELOCITY,
    COSMIC_STARDATE_TO_GREGORIAN,
    COSMIC_WARP_VELOCITY,
    COSMIC_COCHRANE_UNITS,
    COSMIC_ATMOSPHERE_PRESSURE,
    COSMIC_ATMOSPHERE_TEMPERATURE,
    COSMIC_MAGNETOSPHERE_FIELD,
    COSMIC_SCALE_FACTOR,
    COSMIC_COMPTON_WAVELENGTH,
    COSMIC_DE_BROGLIE,
    COSMIC_USRI_PARSE,
    NLP_GAZETTEER_RUN,
    NLP_GAZETTEER_BUILD,
    INFERENCE_EMBED,
    INFERENCE_GROUNDING,
    INFERENCE_VERIFY_TURN,
    INFERENCE_DETECT_UNGROUNDED,
    FINANCE_CONVERT_CURRENCY,
    FINANCE_MULTISIG_CHECK,
    FINANCE_LEDGER_BALANCE,
    CAPABILITY_GRANT,
    CAPABILITY_REVOKE,
    CAPABILITY_TEST_GATING,
    CAPABILITY_AUDIT,
    SENTINEL_INSPECT,
    SENTINEL_GATE,
    AGENT_TRACE,
    AGENT_VERIFY,
    IDENTITY_CURRENT_USER,
    AUDIO_SPECTRUM,
    SCENE_CREATE,
    SCENE_ADD_NODE,
    SCENE_SET_TRANSFORM,
    SCENE_SET_MESH,
    SCENE_ADD_CAMERA,
    SCENE_RENDER,
    SCENE_SET_VIEWPORT,
    SCENE_SET_CLEAR_COLOUR,
    SCENE_CAPTURE_FRAME,
    SOCIAL_GINI,
    SOCIAL_LORENZ,
    SOCIAL_DEGREE_CENTRALITY,
    FORENSIC_MALFEASANCE_DELTA,
    FORENSIC_NARRATIVE_DIVERGENCE,
    NLP_FST_LOOKUP,
    NLP_COREF_RESOLVE,
    NLP_FRAME_EXTRACT,
    NLP_RELATION_EXTRACT,
    NLP_SUBSTRATE_EXTRACT,
    NLP_GRAPHRAG_QUERY,
    AGENT_PLAN,
    AGENT_EXECUTE,
    AGENT_EVALUATE,
    CORPUS_LOAD,
    CORPUS_PARSE,
    AGENCY_EVALUATE,
    INFERENCE_LOAD_MODEL,
    INFERENCE_UNLOAD_MODEL,
    INFERENCE_RUN_TRANSFORMER,
    INFERENCE_RUN_CLASSIFIER,
    INFERENCE_RUN_RERANKER,
    INFERENCE_VECTOR_SEARCH,
    INFERENCE_CONSTRAINED_DECODE,
    AUDIO_OSCILLATOR,
    AUDIO_ENVELOPE,
    AUDIO_FILTER,
    AUDIO_LFO,
    AUDIO_DELAY,
    AUDIO_REVERB,
    AUDIO_COMPRESSOR,
    AUDIO_EQ,
    AUDIO_MIDI_NOTE,
    AUDIO_QUANTIZE,
    AUDIO_TRANSPOSE,
    AUDIO_TRANSPORT,
    AUDIO_WAVEFORM_METER,
    AUDIO_PHASE_METER,
    AUDIO_LOUDNESS_METER,
];

/// Future extract target for an invoke id. Not a crate today.
pub fn seam_for(id: &str) -> &'static str {
    match id {
        DAG_EXECUTE | DAG_VALIDATE | DAG_STATUS => "agent",
        DISCOVERY_LIST | HASH_IRI | COVERAGE_MATRIX | CATALOG_TTL => "runtime",
        SHACL_VALIDATE
        | SHACL_EXTENSIONS
        | GRAPH_STATS
        | GRAPH_SPARQL
        | GRAPH_SHORTEST_PATH
        | GRAPH_SPREADING_ACTIVATION => "graph",
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
        PHYS_WAVE_1D
        | PHYS_HEAT_DIFFUSION_1D
        | PHYS_ADVECTION_DIFFUSION_1D
        | PHYS_HARMONIC_OSCILLATOR
        | PHYS_PENDULUM
        | PHYS_N_BODY
        | PHYS_MOLECULAR_DYNAMICS
        | PHYS_CFD_STEP
        | PHYS_QUANTUM_STATES_1D
        | PHYS_LOGISTIC_GROWTH
        | PHYS_EMF_INTERFERENCE
        | PHYS_EMF_ATTENUATION
        | PHYS_DOPPLER_SHIFT
        | PHYS_EMF_FIELD_GRID_3D
        | PHYS_EMF_SAMPLE_AT_DEPTH => "physics",
        SPECTRAL_EMF_TO_SPD | SPECTRAL_SPD_TO_XYZ | SPECTRAL_EMF_TO_RGB | SPECTRAL_BLEND
        | SPECTRAL_GAMUT_MAP => "spectral",
        CLIN_FRAMINGHAM => "clinical",
        BIOSIGNAL_DP_FILTER | BIOSIGNAL_DP_CONFIG => "biosignal",
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
        | RENDER_SVG_BEZIER | RENDER_SVG_FIELD | GPU_ADAPTER_INFO | GPU_INIT | GPU_RENDER_FRAME
        | GPU_READ_PIXELS | GPU_UPLOAD_MESH | GPU_UPLOAD_TENSOR | GPU_SET_CAMERA | GPU_PICK
        | GPU_POLL_PICK | GPU_RESIZE | GPU_SET_AMBIENT | GPU_DESTROY | GPU_COMPUTE_DISPATCH
        | GPU_COMPUTE_READBACK | GPU_VALIDATE_SHADER | GPU_COMPILE_SHADER | GPU_COMPILE_TO_GLSL
        | GPU_BACKEND_INFO | EMF_UPLOAD_FIELD | EMF_RENDER_SLICE | EMF_FIELD_INFO => "render",
        SAMPLER_CONFIGURE
        | SAMPLER_CONSTRAIN_ENABLE
        | SAMPLER_CONSTRAIN_DISABLE
        | SAMPLER_CONSTRAIN_RESET
        | SAMPLER_SAMPLE => "sampler",
        ASSET_CREATE | ASSET_ADD_TEMPORAL | ASSET_ADD_TOPIC | ASSET_SET_SPATIAL | ASSET_COMPILE
        | ASSET_TEMPORAL_SPAN | ASSET_QUERY_ASPECTS => "asset",
        COSMIC_GEODETIC_TO_ECEF
        | COSMIC_ECEF_TO_GEODETIC
        | COSMIC_ECEF_TO_ENU
        | COSMIC_ENU_TO_ECEF
        | COSMIC_GEODETIC_DISTANCE
        | COSMIC_BODY_PROFILE
        | COSMIC_SURFACE_GRAVITY
        | COSMIC_FLRW_DISTANCE
        | COSMIC_FLRW_REDSHIFT
        | COSMIC_FLRW_HUBBLE_VELOCITY
        | COSMIC_STARDATE_TO_GREGORIAN
        | COSMIC_WARP_VELOCITY
        | COSMIC_COCHRANE_UNITS
        | COSMIC_ATMOSPHERE_PRESSURE
        | COSMIC_ATMOSPHERE_TEMPERATURE
        | COSMIC_MAGNETOSPHERE_FIELD
        | COSMIC_SCALE_FACTOR
        | COSMIC_COMPTON_WAVELENGTH
        | COSMIC_DE_BROGLIE
        | COSMIC_USRI_PARSE => "cosmic",
        NLP_GAZETTEER_RUN | NLP_GAZETTEER_BUILD => "nlp",
        NLP_FST_LOOKUP
        | NLP_COREF_RESOLVE
        | NLP_FRAME_EXTRACT
        | NLP_RELATION_EXTRACT
        | NLP_SUBSTRATE_EXTRACT
        | NLP_GRAPHRAG_QUERY => "nlp",
        INFERENCE_EMBED
        | INFERENCE_GROUNDING
        | INFERENCE_VERIFY_TURN
        | INFERENCE_DETECT_UNGROUNDED
        | INFERENCE_LOAD_MODEL
        | INFERENCE_UNLOAD_MODEL
        | INFERENCE_RUN_TRANSFORMER
        | INFERENCE_RUN_CLASSIFIER
        | INFERENCE_RUN_RERANKER
        | INFERENCE_VECTOR_SEARCH
        | INFERENCE_CONSTRAINED_DECODE => "inference",
        FINANCE_CONVERT_CURRENCY | FINANCE_MULTISIG_CHECK | FINANCE_LEDGER_BALANCE => "econ",
        CAPABILITY_GRANT
        | CAPABILITY_REVOKE
        | CAPABILITY_TEST_GATING
        | CAPABILITY_AUDIT
        | SENTINEL_INSPECT
        | SENTINEL_GATE
        | AGENT_TRACE
        | AGENT_VERIFY
        | IDENTITY_CURRENT_USER => "governance",
        AUDIO_SPECTRUM => "audio",
        AUDIO_OSCILLATOR | AUDIO_ENVELOPE | AUDIO_FILTER | AUDIO_LFO | AUDIO_DELAY
        | AUDIO_REVERB | AUDIO_COMPRESSOR | AUDIO_EQ | AUDIO_MIDI_NOTE | AUDIO_QUANTIZE
        | AUDIO_TRANSPOSE | AUDIO_TRANSPORT | AUDIO_WAVEFORM_METER | AUDIO_PHASE_METER
        | AUDIO_LOUDNESS_METER => "audio",
        SCENE_CREATE
        | SCENE_ADD_NODE
        | SCENE_SET_TRANSFORM
        | SCENE_SET_MESH
        | SCENE_ADD_CAMERA
        | SCENE_RENDER
        | SCENE_SET_VIEWPORT
        | SCENE_SET_CLEAR_COLOUR
        | SCENE_CAPTURE_FRAME => "render",
        SOCIAL_GINI | SOCIAL_LORENZ | SOCIAL_DEGREE_CENTRALITY => "social",
        FORENSIC_MALFEASANCE_DELTA | FORENSIC_NARRATIVE_DIVERGENCE => "social",
        AGENT_PLAN | AGENT_EXECUTE | AGENT_EVALUATE => "agent",
        CORPUS_LOAD | CORPUS_PARSE => "agent",
        AGENCY_EVALUATE => "governance",
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
