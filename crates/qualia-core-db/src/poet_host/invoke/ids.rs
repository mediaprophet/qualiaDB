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
pub const GEOM_DISTANCE_2D: &str = "ComputationalGeometry.distance_2d";
pub const GEOM_DISTANCE_3D: &str = "ComputationalGeometry.distance_3d";
pub const GEOM_POINT_SEGMENT_DISTANCE_2D: &str = "ComputationalGeometry.point_segment_distance_2d";
pub const GEOM_POINT_SEGMENT_DISTANCE_3D: &str = "ComputationalGeometry.point_segment_distance_3d";
pub const GEOM_POINT_TRIANGLE_DISTANCE_3D: &str =
    "ComputationalGeometry.point_triangle_distance_3d";
pub const VISION_AHASH: &str = "ComputerVision.ahash";
pub const VISION_GAUSSIAN_BLUR: &str = "ComputerVision.gaussian_blur";
pub const VISION_SOBEL_MAGNITUDE: &str = "ComputerVision.sobel_magnitude";
pub const VISION_CANNY_EDGES: &str = "ComputerVision.canny_edges";
pub const VISION_HISTOGRAM: &str = "ComputerVision.histogram";
pub const VISION_EQUALIZE_HIST: &str = "ComputerVision.equalize_hist";
pub const VISION_RGB_TO_GRAY: &str = "ComputerVision.rgb_to_gray";
pub const VISION_DHASH: &str = "ComputerVision.dhash";
pub const VISION_HAMMING_DISTANCE: &str = "ComputerVision.hamming_distance";
pub const VISION_COSINE_SIMILARITY: &str = "ComputerVision.cosine_similarity";
pub const NT_GCD: &str = "NumberTheory.gcd";
pub const NT_LCM: &str = "NumberTheory.lcm";
pub const NT_PRIME: &str = "NumberTheory.is_prime";
pub const SPEC_BESSEL: &str = "SpecialFunctionsAndTransforms.bessel_j";
pub const STAT_MEAN: &str = "Statistics.mean";
pub const STAT_PEARSON: &str = "Statistics.pearson";
pub const STAT_MEDIAN: &str = "Statistics.median";
pub const STAT_VARIANCE: &str = "Statistics.variance";
pub const STAT_STD_DEV: &str = "Statistics.std_dev";
pub const STAT_SKEWNESS: &str = "Statistics.skewness";
pub const STAT_KURTOSIS: &str = "Statistics.kurtosis";
pub const STAT_QUANTILE: &str = "Statistics.quantile";
pub const STAT_COVARIANCE: &str = "Statistics.covariance";
pub const STAT_MIN: &str = "Statistics.min";
pub const STAT_MAX: &str = "Statistics.max";
pub const STAT_SUM: &str = "Statistics.sum";
pub const STAT_SPEARMAN: &str = "Statistics.spearman";
pub const STAT_KENDALL: &str = "Statistics.kendall";
pub const STAT_ONE_SAMPLE_T: &str = "Statistics.one_sample_t";
pub const STAT_TWO_SAMPLE_T: &str = "Statistics.two_sample_t";
pub const STAT_PAIRED_T: &str = "Statistics.paired_t";
pub const STAT_CHI_SQUARE_GOF: &str = "Statistics.chi_square_gof";
pub const STAT_ONE_WAY_ANOVA: &str = "Statistics.one_way_anova";
pub const STAT_AUTOCORRELATION: &str = "Statistics.autocorrelation";
pub const STAT_MOVING_AVERAGE: &str = "Statistics.moving_average";
pub const STAT_EXPONENTIAL_SMOOTHING: &str = "Statistics.exponential_smoothing";
pub const STAT_TRIMMED_MEAN: &str = "Statistics.trimmed_mean";
pub const STAT_IQR: &str = "Statistics.iqr";
pub const STAT_MAD: &str = "Statistics.median_abs_deviation";
pub const STAT_ENTROPY: &str = "Statistics.entropy";
pub const STAT_KL_DIVERGENCE: &str = "Statistics.kl_divergence";
pub const STAT_Z_SCORE_OUTLIERS: &str = "Statistics.z_score_outliers";
// Distributions
pub const STAT_NORMAL_PDF: &str = "Statistics.normal_pdf";
pub const STAT_NORMAL_CDF: &str = "Statistics.normal_cdf";
pub const STAT_NORMAL_QUANTILE: &str = "Statistics.normal_quantile";
pub const STAT_STANDARD_NORMAL_CDF: &str = "Statistics.standard_normal_cdf";
pub const STAT_TWO_SIDED_P: &str = "Statistics.two_sided_p";
pub const STAT_STUDENTS_T_PDF: &str = "Statistics.students_t_pdf";
pub const STAT_STUDENTS_T_CDF: &str = "Statistics.students_t_cdf";
pub const STAT_STUDENTS_T_TWO_SIDED_P: &str = "Statistics.students_t_two_sided_p";
pub const STAT_CHI_SQUARED_PDF: &str = "Statistics.chi_squared_pdf";
pub const STAT_CHI_SQUARED_CDF: &str = "Statistics.chi_squared_cdf";
pub const STAT_CHI_SQUARED_UPPER_P: &str = "Statistics.chi_squared_upper_p";
pub const STAT_FISHER_F_PDF: &str = "Statistics.fisher_f_pdf";
pub const STAT_FISHER_F_CDF: &str = "Statistics.fisher_f_cdf";
pub const STAT_FISHER_F_UPPER_P: &str = "Statistics.fisher_f_upper_p";
pub const STAT_BINOMIAL_PMF: &str = "Statistics.binomial_pmf";
pub const STAT_BINOMIAL_CDF: &str = "Statistics.binomial_cdf";
pub const STAT_POISSON_PMF: &str = "Statistics.poisson_pmf";
pub const STAT_POISSON_CDF: &str = "Statistics.poisson_cdf";
pub const STAT_EXPONENTIAL_PDF: &str = "Statistics.exponential_pdf";
pub const STAT_EXPONENTIAL_CDF: &str = "Statistics.exponential_cdf";
pub const STAT_GAMMA_PDF: &str = "Statistics.gamma_pdf";
pub const STAT_BETA_PDF: &str = "Statistics.beta_pdf";
pub const STAT_WEIBULL_PDF: &str = "Statistics.weibull_pdf";
pub const STAT_LOGNORMAL_PDF: &str = "Statistics.lognormal_pdf";
pub const STAT_UNIFORM_PDF: &str = "Statistics.uniform_pdf";
pub const STAT_LAPLACE_PDF: &str = "Statistics.laplace_pdf";
pub const STAT_LN_GAMMA: &str = "Statistics.ln_gamma";
pub const STAT_GAMMA_FN: &str = "Statistics.gamma_fn";
pub const STAT_ERF: &str = "Statistics.erf";
pub const STAT_ERFC: &str = "Statistics.erfc";
pub const STAT_EMPIRICAL_CDF: &str = "Statistics.empirical_cdf";
// Extra stats
pub const STAT_MODE: &str = "Statistics.mode";
pub const STAT_WINSORIZED_MEAN: &str = "Statistics.winsorized_mean";
pub const STAT_CROSS_ENTROPY: &str = "Statistics.cross_entropy";
pub const STAT_MUTUAL_INFORMATION: &str = "Statistics.mutual_information";
pub const STAT_HISTOGRAM: &str = "Statistics.histogram";
pub const STAT_CORRELATION_P_VALUE: &str = "Statistics.correlation_p_value";
pub const STAT_CHI_SQUARE_INDEPENDENCE: &str = "Statistics.chi_square_independence";
pub const STAT_MODIFIED_Z_SCORE_OUTLIERS: &str = "Statistics.modified_z_score_outliers";
pub const STAT_IQR_OUTLIERS: &str = "Statistics.iqr_outliers";
pub const STAT_GRUBBS_TEST: &str = "Statistics.grubbs_test";
pub const STAT_MANN_WHITNEY_U: &str = "Statistics.mann_whitney_u";
pub const STAT_KS_1SAMPLE: &str = "Statistics.ks_1sample";
pub const STAT_FRIEDMAN: &str = "Statistics.friedman";
pub const STAT_MCNEMAR: &str = "Statistics.mcnemar";
pub const STAT_BOOTSTRAP_MEANS: &str = "Statistics.bootstrap_means";
pub const STAT_LJUNG_BOX: &str = "Statistics.ljung_box";
pub const STAT_ADF_PROXY: &str = "Statistics.adf_proxy";
pub const ML_OLS: &str = "MachineLearning.ols";
pub const ML_MSE: &str = "MachineLearning.mse";
pub const ML_RMSE: &str = "MachineLearning.rmse";
pub const ML_MAE: &str = "MachineLearning.mae";
pub const ML_R2: &str = "MachineLearning.r2_score";
pub const ML_ACCURACY: &str = "MachineLearning.accuracy";
pub const ML_ROC_AUC: &str = "MachineLearning.roc_auc";
pub const ML_KMEANS: &str = "MachineLearning.kmeans";
pub const ML_TRAIN_TEST_SPLIT: &str = "MachineLearning.train_test_split";
pub const ML_LOG_LOSS: &str = "MachineLearning.log_loss";
pub const ML_CONFUSION_BINARY: &str = "MachineLearning.confusion_binary";
pub const ML_K_FOLD: &str = "MachineLearning.k_fold";
pub const ML_BOOTSTRAP_INDICES: &str = "MachineLearning.bootstrap_indices";
pub const ML_BONFERRONI: &str = "MachineLearning.bonferroni";
pub const ML_HOLM: &str = "MachineLearning.holm";
pub const ML_BH: &str = "MachineLearning.benjamini_hochberg";
pub const ML_PCA: &str = "MachineLearning.pca";
pub const ML_AB_TEST: &str = "MachineLearning.ab_test";
pub const ML_POWER_TWO_SAMPLE: &str = "MachineLearning.power_two_sample";
pub const ML_REQUIRED_SAMPLE_SIZE: &str = "MachineLearning.required_sample_size";
pub const ML_TRANSE_SCORE: &str = "MachineLearning.transe_score";
pub const ML_DISTMULT_SCORE: &str = "MachineLearning.distmult_score";
pub const BIOSIGNAL_DP_FILTER: &str = "biosignal.dp_filter";
pub const BIOSIGNAL_DP_CONFIG: &str = "biosignal.dp_config";
pub const PHYS_PROJECTILE: &str = "PhysicsAndODE.projectile";
pub const BIO_ALIGN: &str = "Bioinformatics.align";
pub const CHEM_SMILES: &str = "OrganicChemistry.validate_smiles";
pub const CLIN_FRAMINGHAM: &str = "ClinicalRisk.framingham";
pub const FIN_BS: &str = "FinancialModeling.black_scholes";
pub const ENG_KIN: &str = "EngineeringAnalysis.kinematics";
pub const ENG_CAUCHY_STRESS: &str = "EngineeringAnalysis.cauchy_stress";
pub const ENG_DRAG_FORCE: &str = "EngineeringAnalysis.drag_force";
pub const ENG_REYNOLDS: &str = "EngineeringAnalysis.reynolds_number";
pub const ENG_FATIGUE_CYCLES: &str = "EngineeringAnalysis.fatigue_cycles";
pub const ENG_MINER_DAMAGE: &str = "EngineeringAnalysis.miner_damage";
pub const CHEM_ELEMENT_SYMBOL: &str = "Chemistry.element_symbol";
pub const CHEM_ATOMIC_NUMBER: &str = "Chemistry.atomic_number";
pub const CHEM_ATOMIC_WEIGHT: &str = "Chemistry.standard_atomic_weight";
pub const CHEM_LDA_EXCHANGE: &str = "Chemistry.lda_exchange";
pub const CHEM_LDA_CORRELATION_VWN: &str = "Chemistry.lda_correlation_vwn";
pub const MED_TANIMOTO: &str = "Medical.tanimoto";
pub const MED_STRUCTURAL_FINGERPRINT: &str = "Medical.structural_fingerprint";
pub const MED_ANALYZE_INTENSITY_GRID: &str = "Medical.analyze_intensity_grid";
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
// N11: Pulse payload types, channels, transports
pub const PULSE_PUBLISH: &str = "Pulse.publish";
pub const PULSE_PUBLISH_GRAPH_MUTATION: &str = "Pulse.publish_graph_mutation";
pub const PULSE_PUBLISH_NOTIFICATION: &str = "Pulse.publish_notification";
pub const PULSE_PUBLISH_TELEMETRY: &str = "Pulse.publish_telemetry";
pub const PULSE_PUBLISH_AGENT_MESSAGE: &str = "Pulse.publish_agent_message";
pub const PULSE_PUBLISH_PRESENCE: &str = "Pulse.publish_presence";
pub const PULSE_PUBLISH_SYNC: &str = "Pulse.publish_sync";
pub const PULSE_OPEN_CHANNEL: &str = "Pulse.open_channel";
pub const PULSE_CLOSE_CHANNEL: &str = "Pulse.close_channel";
pub const PULSE_SET_TRANSPORT: &str = "Pulse.set_transport";
pub const FIN_PORTFOLIO: &str = "FinancialModeling.portfolio_risk";
pub const COVERAGE_MATRIX: &str = "CapabilityDiscovery.coverage";
pub const CATALOG_TTL: &str = "CapabilityDiscovery.catalog";
pub const RENDER_SCENE: &str = "Render.scene";
pub const RENDER_CSS_ANIMATION: &str = "Render.css_animation";
pub const RENDER_CSS_COLOR: &str = "Render.css_color";
pub const RENDER_CSS_TRANSFORM: &str = "Render.css_transform";
pub const RENDER_ANIMATION_EVAL_CURVE: &str = "Render.animation_eval_curve";
pub const RENDER_ANIMATION_SPRING_STEP: &str = "Render.animation_spring_step";
pub const RENDER_ANIMATION_SCLERP: &str = "Render.animation_sclerp";
pub const RENDER_ANIMATION_EVAL_PRESET: &str = "Render.animation_eval_preset";
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
// N16: Persistent asset aspect-graph store
pub const ASSET_PERSIST: &str = "Asset.persist";
pub const ASSET_RESOLVE: &str = "Asset.resolve";
pub const ASSET_RESOLVE_BY_SPATIAL: &str = "Asset.resolve_by_spatial";
pub const ASSET_RESOLVE_BY_TOPIC: &str = "Asset.resolve_by_topic";
pub const ASSET_RESOLVE_BY_TEMPORAL: &str = "Asset.resolve_by_temporal";
pub const ASSET_LIST: &str = "Asset.list";
pub const ASSET_COUNT: &str = "Asset.count";
pub const ASSET_PERSIST_CREATE: &str = "Asset.persist_create";
pub const ASSET_PERSIST_ADD_TEMPORAL: &str = "Asset.persist_add_temporal";
pub const ASSET_PERSIST_ADD_TOPIC: &str = "Asset.persist_add_topic";
pub const ASSET_PERSIST_SET_SPATIAL: &str = "Asset.persist_set_spatial";
pub const ASSET_PERSIST_COMPILE: &str = "Asset.persist_compile";
pub const ASSET_PERSIST_TEMPORAL_SPAN: &str = "Asset.persist_temporal_span";
pub const ASSET_PERSIST_QUERY_ASPECTS: &str = "Asset.persist_query_aspects";

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
pub const PHYS_FIELD_SAMPLE: &str = "Physics.field_sample";
pub const PHYS_MATERIAL_QUERY: &str = "Physics.material_query";
pub const PHYS_EVALUATE_INTERACTION: &str = "Physics.evaluate_interaction";

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
pub const PRIVACY_GAUSSIAN_SIGMA: &str = "Privacy.gaussian_sigma";

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
// N15: Multi-agent orchestration
pub const ORCH_SESSION_CREATE: &str = "Orchestration.session_create";
pub const ORCH_SESSION_PLAN: &str = "Orchestration.session_plan";
pub const ORCH_SESSION_EXECUTE: &str = "Orchestration.session_execute";
pub const ORCH_SESSION_STATUS: &str = "Orchestration.session_status";
pub const ORCH_ROSTER_REGISTER: &str = "Orchestration.roster_register";
pub const ORCH_ROSTER_LIST: &str = "Orchestration.roster_list";
pub const ORCH_ROSTER_CAPABILITIES: &str = "Orchestration.roster_capabilities";
pub const ORCH_ASSIGN_AGENTS: &str = "Orchestration.assign_agents";

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
// N14: Computational economics — ~50 key functions from 22 submodules
pub const ECON_CAPM_EXPECTED_RETURN: &str = "Econ.capm_expected_return";
pub const ECON_CAPM_BETA: &str = "Econ.capm_beta";
pub const ECON_GORDON_GROWTH: &str = "Econ.gordon_growth";
pub const ECON_MULTI_PERIOD_DDM: &str = "Econ.multi_period_ddm";
pub const ECON_CCAPM_EQUITY_PREMIUM: &str = "Econ.ccapm_equity_premium";
pub const ECON_CCAPM_SDF: &str = "Econ.ccapm_sdf";
pub const ECON_PROSPECT_VALUE: &str = "Econ.prospect_value";
pub const ECON_PROBABILITY_WEIGHT: &str = "Econ.probability_weight";
pub const ECON_HYPERBOLIC_DISCOUNT: &str = "Econ.hyperbolic_discount";
pub const ECON_ENDOWMENT_EFFECT: &str = "Econ.endowment_effect";
pub const ECON_BLACK_SCHOLES: &str = "Econ.black_scholes";
pub const ECON_PUT_CALL_PARITY: &str = "Econ.put_call_parity";
pub const ECON_BINOMIAL_OPTION: &str = "Econ.binomial_option";
pub const ECON_MIXED_NASH_2X2: &str = "Econ.mixed_nash_2x2";
pub const ECON_COURNOT_DUOPOLY: &str = "Econ.cournot_duopoly";
pub const ECON_BERTRAND_DUOPOLY: &str = "Econ.bertrand_duopoly";
pub const ECON_STACKELBERG_DUOPOLY: &str = "Econ.stackelberg_duopoly";
pub const ECON_SOLOW_STEADY_STATE: &str = "Econ.solow_steady_state";
pub const ECON_RAMSEY_STEADY_STATE: &str = "Econ.ramsey_steady_state";
pub const ECON_OLG_STEADY_STATE: &str = "Econ.olg_steady_state";
pub const ECON_GINI: &str = "Econ.gini";
pub const ECON_ATKINSON: &str = "Econ.atkinson";
pub const ECON_HEADCOUNT_POVERTY: &str = "Econ.headcount_poverty";
pub const ECON_POVERTY_GAP: &str = "Econ.poverty_gap";
pub const ECON_UTILITARIAN_WELFARE: &str = "Econ.utilitarian_welfare";
pub const ECON_RAWLSIAN_WELFARE: &str = "Econ.rawlsian_welfare";
pub const ECON_NASH_WELFARE: &str = "Econ.nash_welfare";
pub const ECON_NPV: &str = "Econ.npv";
pub const ECON_MEAN_RETURN: &str = "Econ.mean_return";
pub const ECON_SAMPLE_VARIANCE: &str = "Econ.sample_variance";
pub const ECON_PORTFOLIO_MAX_DRAWDOWN: &str = "Econ.portfolio_max_drawdown";
pub const ECON_HISTORICAL_VAR: &str = "Econ.historical_var";
pub const ECON_HISTORICAL_CVAR: &str = "Econ.historical_cvar";
pub const ECON_PARAMETRIC_VAR: &str = "Econ.parametric_var";
pub const ECON_AUTOCORRELATION: &str = "Econ.autocorrelation";
pub const ECON_CROSS_CORRELATION: &str = "Econ.cross_correlation";
pub const ECON_INTERPOLATE_ZERO_RATE: &str = "Econ.interpolate_zero_rate";
pub const ECON_DISCOUNT_FACTOR: &str = "Econ.discount_factor";
pub const ECON_FORWARD_RATE: &str = "Econ.forward_rate";
pub const ECON_GRAVITY_FLOW: &str = "Econ.gravity_flow";
pub const ECON_MORANS_I: &str = "Econ.morans_i";
pub const ECON_TRANSFER_PAYMENT: &str = "Econ.transfer_payment";
pub const ECON_FISCAL_MULTIPLIER: &str = "Econ.fiscal_multiplier";
pub const ECON_LAFFER_CURVE: &str = "Econ.laffer_curve";
pub const ECON_CHECK_IR: &str = "Econ.check_ir";
pub const ECON_CHECK_BUDGET_BALANCE: &str = "Econ.check_budget_balance";
pub const ECON_VALIDATE_TRANSITION_MATRIX: &str = "Econ.validate_transition_matrix";
pub const ECON_TRANSITION_PROBABILITY: &str = "Econ.transition_probability";
pub const ECON_EXPECTED_HOLDING_TIME: &str = "Econ.expected_holding_time";
pub const ECON_LABOR_SUPPLY: &str = "Econ.labor_supply";
pub const ECON_EFFICIENCY_UNITS: &str = "Econ.efficiency_units";
pub const ECON_SOCIAL_COST_OF_CARBON: &str = "Econ.social_cost_of_carbon";
pub const ECON_OPTIMAL_POLLUTION: &str = "Econ.optimal_pollution";
pub const ECON_OPTIMAL_ABATEMENT: &str = "Econ.optimal_abatement";
pub const ECON_BELLMAN_UPDATE: &str = "Econ.bellman_update";
pub const ECON_MALFEASANCE_DELTA: &str = "Econ.malfeasance_delta";
pub const ECON_OLS: &str = "Econ.ols";
pub const ECON_AGGREGATE_WEALTH: &str = "Econ.aggregate_wealth";
pub const ECON_TOTAL_TRANSPORT_COST: &str = "Econ.total_transport_cost";
pub const ECON_LUCAS_ASSET_PRICE: &str = "Econ.lucas_asset_price";
pub const ECON_PRESENT_BIASED_UTILITY: &str = "Econ.present_biased_utility";
pub const ECON_REFERENCE_DEPENDENT_UTILITY: &str = "Econ.reference_dependent_utility";
pub const ECON_PURE_NASH_EQUILIBRIA: &str = "Econ.pure_nash_equilibria";
pub const ECON_REPEATED_GAME_PAYOFF: &str = "Econ.repeated_game_payoff";
pub const ECON_BERTRAND_WITH_DEMAND: &str = "Econ.bertrand_with_demand";
pub const ECON_RAMSEY_EULER_RESIDUAL: &str = "Econ.ramsey_euler_residual";
pub const ECON_NEW_KEYNESIAN_SOLVE: &str = "Econ.new_keynesian_solve";
pub const ECON_LORENZ_CURVE: &str = "Econ.lorenz_curve";
pub const ECON_DISTRIBUTIONAL_NPV: &str = "Econ.distributional_npv";
pub const ECON_PORTFOLIO_RETURNS: &str = "Econ.portfolio_returns";
pub const ECON_COVARIANCE_MATRIX: &str = "Econ.covariance_matrix";
pub const ECON_PORTFOLIO_VARIANCE: &str = "Econ.portfolio_variance";
pub const ECON_SIMPLE_RETURNS: &str = "Econ.simple_returns";
pub const ECON_LOG_RETURNS: &str = "Econ.log_returns";
pub const ECON_CUMULATIVE_WEALTH: &str = "Econ.cumulative_wealth";
pub const ECON_DRAWDOWN: &str = "Econ.drawdown";
pub const ECON_ROLLING_MEAN: &str = "Econ.rolling_mean";
pub const ECON_ROLLING_VARIANCE: &str = "Econ.rolling_variance";
pub const ECON_GBM_SIMULATE: &str = "Econ.gbm_simulate";
pub const ECON_STRESS_SCENARIO: &str = "Econ.stress_scenario";
pub const ECON_BLOCK_BOOTSTRAP: &str = "Econ.block_bootstrap";
pub const ECON_PAR_YIELD: &str = "Econ.par_yield";
pub const ECON_NEAREST_FACILITY: &str = "Econ.nearest_facility";
pub const ECON_PROGRESSIVE_TAX: &str = "Econ.progressive_tax";
pub const ECON_VCG_PAYMENT: &str = "Econ.vcg_payment";
pub const ECON_STRATEGY_PROOFNESS: &str = "Econ.strategy_proofness";
pub const ECON_STATIONARY_DISTRIBUTION: &str = "Econ.stationary_distribution";
pub const ECON_SIMULATE_CHAIN: &str = "Econ.simulate_chain";
pub const ECON_MEAN_FIRST_PASSAGE: &str = "Econ.mean_first_passage";
pub const ECON_HOUSEHOLD_PRODUCTION_CES: &str = "Econ.household_production_ces";
pub const ECON_POLLUTION_DAMAGE: &str = "Econ.pollution_damage";
pub const ECON_MARGINAL_DAMAGE: &str = "Econ.marginal_damage";
pub const ECON_ABATEMENT_NET_BENEFIT: &str = "Econ.abatement_net_benefit";
pub const ECON_WLS: &str = "Econ.wls";
pub const ECON_IV_2SLS: &str = "Econ.iv_2sls";
pub const ECON_LOGISTIC_MLE: &str = "Econ.logistic_mle";
pub const ECON_VALUE_ITERATION: &str = "Econ.value_iteration";
pub const ECON_NARRATIVE_DIVERGENCE: &str = "Econ.narrative_divergence";
pub const ECON_EIGENVECTOR_CENTRALITY: &str = "Econ.eigenvector_centrality";
pub const ECON_DEGREE_CENTRALITY: &str = "Econ.degree_centrality";
pub const ECON_INTERBANK_CLEARING: &str = "Econ.interbank_clearing";
pub const ECON_LEONTIEF_INVERSE: &str = "Econ.leontief_inverse";
pub const ECON_OUTPUT_MULTIPLIERS: &str = "Econ.output_multipliers";
pub const ECON_AGENT_BASED_AGGREGATE_WEALTH: &str = "Econ.agent_based_aggregate_wealth";
pub const ECON_VALIDATE_SCALAR_CONSTRAINT: &str = "Econ.validate_scalar_constraint";
pub const ECON_AGGREGATE_PAPER_FILLS: &str = "Econ.aggregate_paper_fills";
pub const CAPABILITY_GRANT: &str = "Capability.grant";
pub const CAPABILITY_REVOKE: &str = "Capability.revoke";
pub const CAPABILITY_TEST_GATING: &str = "Capability.test_gating";
pub const CAPABILITY_AUDIT: &str = "Capability.audit";
pub const CAPABILITY_DECLARE: &str = "Capability.declare";
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

// ── N7: Scene graph build-new — lights, semantic links, duplication, IK, smooth damp ─
pub const SCENE_ADD_LIGHT: &str = "Scene.add_light";
pub const SCENE_LINK_SEMANTIC: &str = "Scene.link_semantic";
pub const SCENE_DUPLICATE_NODE: &str = "Scene.duplicate_node";
pub const SCENE_SET_RENDER_BUDGET: &str = "Scene.set_render_budget";
pub const SCENE_IK_LOOK_AT: &str = "Scene.ik_look_at";
pub const SCENE_IK_CCD: &str = "Scene.ik_ccd";
pub const SCENE_SMOOTH_DAMP: &str = "Scene.smooth_damp";
pub const SCENE_SMOOTH_DAMP_VEC3: &str = "Scene.smooth_damp_vec3";

// ── N8: Research / epistemics — enquiry, corpus, dark links, inference chains, investigation ──
pub const RESEARCH_NEW: &str = "Research.new";
pub const RESEARCH_SET_PURPOSE: &str = "Research.set_purpose";
pub const RESEARCH_DEFINE_SCOPE: &str = "Research.define_scope";
pub const RESEARCH_ADD_CONSTRAINT: &str = "Research.add_constraint";
pub const RESEARCH_ADD_QUESTION: &str = "Research.add_question";
pub const RESEARCH_LINK_QUESTIONS: &str = "Research.link_questions";
pub const RESEARCH_ADD_CORPUS_ITEM: &str = "Research.add_corpus_item";
pub const RESEARCH_IMPORT_LITERATURE: &str = "Research.import_literature";
pub const RESEARCH_IMPORT_DATASET: &str = "Research.import_dataset";
pub const RESEARCH_SET_CORPUS_CONFIDENCE: &str = "Research.set_corpus_confidence";
pub const RESEARCH_EXTRACT_FROM_CORPUS: &str = "Research.extract_from_corpus";
pub const RESEARCH_INFER_DARK_LINK: &str = "Research.infer_dark_link";
pub const RESEARCH_DETECT_PROVENANCE_GAPS: &str = "Research.detect_provenance_gaps";
pub const RESEARCH_DETECT_CONCEALMENT: &str = "Research.detect_concealment";
pub const RESEARCH_CONFIRM_DARK_LINK: &str = "Research.confirm_dark_link";
pub const RESEARCH_REFUTE_DARK_LINK: &str = "Research.refute_dark_link";
pub const RESEARCH_MAKE_INFERENCE: &str = "Research.make_inference";
pub const RESEARCH_CHAIN_INFERENCE: &str = "Research.chain_inference";
pub const RESEARCH_SET_INFERENCE_CONFIDENCE: &str = "Research.set_inference_confidence";
pub const RESEARCH_VALIDATE_INFERENCE: &str = "Research.validate_inference";
pub const RESEARCH_NEW_INVESTIGATION: &str = "Research.new_investigation";
pub const RESEARCH_COLLECT_EVIDENCE: &str = "Research.collect_evidence";
pub const RESEARCH_SET_RELIABILITY: &str = "Research.set_reliability";
pub const RESEARCH_PROPOSE_HYPOTHESIS: &str = "Research.propose_hypothesis";
pub const RESEARCH_EVALUATE_EVIDENCE: &str = "Research.evaluate_evidence";
pub const RESEARCH_CREATE_TIMELINE: &str = "Research.create_timeline";
pub const RESEARCH_ADD_LINK: &str = "Research.add_link";
pub const RESEARCH_FIND_PATH: &str = "Research.find_path";
pub const RESEARCH_CREATE_HYPOTHESIS_GRAPH: &str = "Research.create_hypothesis_graph";
pub const RESEARCH_CONTRIBUTE_EVALUATION: &str = "Research.contribute_evaluation";
pub const RESEARCH_BRIDGE_DARK_LINK: &str = "Research.bridge_dark_link";
pub const RESEARCH_REFRAME_HYPOTHESIS: &str = "Research.reframe_hypothesis";
pub const RESEARCH_MERGE_HYPOTHESES: &str = "Research.merge_hypotheses";
pub const RESEARCH_FLAG_GAP: &str = "Research.flag_gap";
pub const RESEARCH_CLOSE_GAP: &str = "Research.close_gap";
pub const RESEARCH_CREATE_REVISION: &str = "Research.create_revision";
pub const RESEARCH_DIFF_REVISIONS: &str = "Research.diff_revisions";
pub const RESEARCH_SUBSCRIBE_UPDATES: &str = "Research.subscribe_updates";
pub const RESEARCH_CREATE_ASSESSMENT: &str = "Research.create_assessment";
pub const RESEARCH_SET_EPISTEMIC_MODE: &str = "Research.set_epistemic_mode";
pub const RESEARCH_SET_REALITY_CATEGORY: &str = "Research.set_reality_category";
pub const RESEARCH_CLASSIFY_REALITY: &str = "Research.classify_reality";
pub const RESEARCH_DETECT_BLENDED: &str = "Research.detect_blended";
pub const RESEARCH_DETECT_DECEPTIVE_FICTION: &str = "Research.detect_deceptive_fiction";
pub const RESEARCH_TRACE_FICTION: &str = "Research.trace_fiction";
pub const RESEARCH_ASSESS_SENTIMENT: &str = "Research.assess_sentiment";
pub const RESEARCH_DETECT_SENTIMENT_MANIPULATION: &str = "Research.detect_sentiment_manipulation";
pub const RESEARCH_DETECT_PERFORMED_SENTIMENT: &str = "Research.detect_performed_sentiment";
pub const RESEARCH_MAP_SENTIMENT_NETWORK: &str = "Research.map_sentiment_network";
pub const RESEARCH_ANALYSE_SENTIMENT_TRENDS: &str = "Research.analyse_sentiment_trends";
// N10: Research gaps — perspective, intentionality, dynamics, grounding, UG
pub const RESEARCH_REGISTER_PERSPECTIVE: &str = "Research.register_perspective";
pub const RESEARCH_ADD_BIAS: &str = "Research.add_bias";
pub const RESEARCH_COMPARE_PERSPECTIVES: &str = "Research.compare_perspectives";
pub const RESEARCH_DETECT_PERSPECTIVE_CONFLICT: &str = "Research.detect_perspective_conflict";
pub const RESEARCH_RECONCILE_PERSPECTIVES: &str = "Research.reconcile_perspectives";
pub const RESEARCH_ASSESS_INTENTIONALITY: &str = "Research.assess_intentionality";
pub const RESEARCH_CLASSIFY_MISTAKE: &str = "Research.classify_mistake";
pub const RESEARCH_DEFINE_SOCIAL_DYNAMICS: &str = "Research.define_social_dynamics";
pub const RESEARCH_DEFINE_ECONOMIC_DYNAMICS: &str = "Research.define_economic_dynamics";
pub const RESEARCH_DEFINE_SPATIOTEMPORAL_DYNAMICS: &str = "Research.define_spatiotemporal_dynamics";
pub const RESEARCH_ANALYSE_SOCIAL_NETWORK: &str = "Research.analyse_social_network";
pub const RESEARCH_ANALYSE_INEQUALITY: &str = "Research.analyse_inequality";
pub const RESEARCH_ANALYSE_DIFFUSION: &str = "Research.analyse_diffusion";
pub const RESEARCH_ASSESS_GROUNDING: &str = "Research.assess_grounding";
pub const RESEARCH_VERIFY_GROUNDING: &str = "Research.verify_grounding";
pub const RESEARCH_DETECT_UNGROUNDED_BEHAVIOUR: &str = "Research.detect_ungrounded_behaviour";
pub const RESEARCH_CREATE_UG_INSTANCE: &str = "Research.create_ug_instance";
pub const RESEARCH_SET_UG_CAUSE: &str = "Research.set_ug_cause";
pub const RESEARCH_SET_UG_CONSEQUENCE: &str = "Research.set_ug_consequence";
pub const RESEARCH_SET_UG_DETECTION: &str = "Research.set_ug_detection";
pub const RESEARCH_SET_UG_MITIGATION: &str = "Research.set_ug_mitigation";
pub const RESEARCH_SET_UG_CALIBRATION: &str = "Research.set_ug_calibration";
pub const RESEARCH_DETECT_UG_PATTERNS: &str = "Research.detect_ug_patterns";

// ── N9: Hypermedia authoring — image, video, 3D, interactive, portals, DMX ─
// Image editing
pub const IMAGE_NEW: &str = "Image.new";
pub const IMAGE_ADD_LAYER: &str = "Image.add_layer";
pub const IMAGE_REMOVE_LAYER: &str = "Image.remove_layer";
pub const IMAGE_SET_PIXEL: &str = "Image.set_pixel";
pub const IMAGE_FILL: &str = "Image.fill";
pub const IMAGE_BRUSH: &str = "Image.brush";
pub const IMAGE_APPLY_FILTER: &str = "Image.apply_filter";
pub const IMAGE_SET_OPACITY: &str = "Image.set_opacity";
pub const IMAGE_SET_BLEND_MODE: &str = "Image.set_blend_mode";
pub const IMAGE_SET_VISIBLE: &str = "Image.set_visible";
pub const IMAGE_SET_MASK: &str = "Image.set_mask";
pub const IMAGE_CLEAR_MASK: &str = "Image.clear_mask";
pub const IMAGE_COMPOSITE: &str = "Image.composite";
pub const IMAGE_ADD_SELECTION: &str = "Image.add_selection";
pub const IMAGE_CLEAR_SELECTIONS: &str = "Image.clear_selections";
// Video
pub const VIDEO_NEW_PROJECT: &str = "Video.new_project";
pub const VIDEO_ADD_TRACK: &str = "Video.add_track";
pub const VIDEO_ADD_CLIP: &str = "Video.add_clip";
pub const VIDEO_TRIM_CLIP: &str = "Video.trim_clip";
pub const VIDEO_SET_SPEED: &str = "Video.set_speed";
pub const VIDEO_COLOUR_GRADE: &str = "Video.colour_grade";
pub const VIDEO_ADD_TRANSITION: &str = "Video.add_transition";
pub const VIDEO_SET_RENDER_FORMAT: &str = "Video.set_render_format";
pub const VIDEO_SET_RENDER_BITRATE: &str = "Video.set_render_bitrate";
pub const VIDEO_REMOVE_CLIP: &str = "Video.remove_clip";
// 3D
pub const THREE_D_ADD_OBJECT: &str = "ThreeD.add_object";
pub const THREE_D_SET_TRANSFORM: &str = "ThreeD.set_transform";
pub const THREE_D_SET_MATERIAL: &str = "ThreeD.set_material";
pub const THREE_D_ADD_CAMERA: &str = "ThreeD.add_camera";
pub const THREE_D_ADD_LIGHT: &str = "ThreeD.add_light";
pub const THREE_D_ADD_RIG: &str = "ThreeD.add_rig";
pub const THREE_D_ADD_ANIMATION: &str = "ThreeD.add_animation";
pub const THREE_D_SET_MESH: &str = "ThreeD.set_mesh";
// Interactive
pub const HBBTV_NEW_APP: &str = "HbbTV.new_app";
pub const HBBTV_ADD_PAGE: &str = "HbbTV.add_page";
pub const HBBTV_NAVIGATE: &str = "HbbTV.navigate";
pub const HBBTV_SET_STATE: &str = "HbbTV.set_state";
pub const SECOND_SCREEN_SYNC: &str = "SecondScreen.sync";
pub const INTERACTIVE_ADD_TRIGGER: &str = "Interactive.add_trigger";
pub const INTERACTIVE_ADD_SOCIAL_POST: &str = "Interactive.add_social_post";
// Portals / worlds
pub const WORLD_NEW: &str = "World.new";
pub const WORLD_ADD_OBJECT: &str = "World.add_object";
pub const WORLD_ADD_PORTAL: &str = "World.add_portal";
pub const WORLD_ADD_AVATAR: &str = "World.add_avatar";
pub const WORLD_SET_GRAVITY: &str = "World.set_gravity";
pub const WORLD_OBJECT_APPLY_FORCE: &str = "World.object_apply_force";
pub const WORLD_OBJECT_STEP_PHYSICS: &str = "World.object_step_physics";
pub const PORTAL_SET_TARGET: &str = "Portal.set_target";
pub const PORTAL_ACTIVATE: &str = "Portal.activate";
pub const PORTAL_DEACTIVATE: &str = "Portal.deactivate";
pub const AVATAR_MOVE: &str = "Avatar.move";
pub const AVATAR_SET_APPEARANCE: &str = "Avatar.set_appearance";
// DMX
pub const DMX_NEW_UNIVERSE: &str = "Dmx.new_universe";
pub const DMX_SET_CHANNEL: &str = "Dmx.set_channel";
pub const DMX_ADD_FIXTURE: &str = "Dmx.add_fixture";
pub const DMX_FIXTURE_SET_COLOUR: &str = "Dmx.fixture_set_colour";
pub const DMX_FIXTURE_SET_INTENSITY: &str = "Dmx.fixture_set_intensity";
pub const DMX_FIXTURE_SET_PAN_TILT: &str = "Dmx.fixture_set_pan_tilt";
pub const DMX_NEW_CUE: &str = "Dmx.new_cue";
pub const DMX_CUE_SET_CHANNEL: &str = "Dmx.cue_set_channel";
pub const DMX_CUE_SET_FADE: &str = "Dmx.cue_set_fade";
pub const DMX_NEW_CUE_STACK: &str = "Dmx.new_cue_stack";
pub const DMX_CUE_STACK_ADD: &str = "Dmx.cue_stack_add";
pub const DMX_CUE_STACK_GO: &str = "Dmx.cue_stack_go";
pub const DMX_CUE_STACK_GO_BACK: &str = "Dmx.cue_stack_go_back";
pub const DMX_CUE_STACK_RESET: &str = "Dmx.cue_stack_reset";

pub const ALL_BOUND: &[&str] = &[
    DAG_EXECUTE,
    DAG_VALIDATE,
    DAG_STATUS,
    ORCH_SESSION_CREATE,
    ORCH_SESSION_PLAN,
    ORCH_SESSION_EXECUTE,
    ORCH_SESSION_STATUS,
    ORCH_ROSTER_REGISTER,
    ORCH_ROSTER_LIST,
    ORCH_ROSTER_CAPABILITIES,
    ORCH_ASSIGN_AGENTS,
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
    GEOM_DISTANCE_2D,
    GEOM_DISTANCE_3D,
    GEOM_POINT_SEGMENT_DISTANCE_2D,
    GEOM_POINT_SEGMENT_DISTANCE_3D,
    GEOM_POINT_TRIANGLE_DISTANCE_3D,
    VISION_AHASH,
    VISION_GAUSSIAN_BLUR,
    VISION_SOBEL_MAGNITUDE,
    VISION_CANNY_EDGES,
    VISION_HISTOGRAM,
    VISION_EQUALIZE_HIST,
    VISION_RGB_TO_GRAY,
    VISION_DHASH,
    VISION_HAMMING_DISTANCE,
    VISION_COSINE_SIMILARITY,
    NT_GCD,
    NT_LCM,
    NT_PRIME,
    SPEC_BESSEL,
    STAT_MEAN,
    STAT_MEDIAN,
    STAT_VARIANCE,
    STAT_STD_DEV,
    STAT_SKEWNESS,
    STAT_KURTOSIS,
    STAT_QUANTILE,
    STAT_COVARIANCE,
    STAT_MIN,
    STAT_MAX,
    STAT_SUM,
    STAT_SPEARMAN,
    STAT_KENDALL,
    STAT_ONE_SAMPLE_T,
    STAT_TWO_SAMPLE_T,
    STAT_PAIRED_T,
    STAT_CHI_SQUARE_GOF,
    STAT_ONE_WAY_ANOVA,
    STAT_AUTOCORRELATION,
    STAT_MOVING_AVERAGE,
    STAT_EXPONENTIAL_SMOOTHING,
    STAT_TRIMMED_MEAN,
    STAT_IQR,
    STAT_MAD,
    STAT_ENTROPY,
    STAT_KL_DIVERGENCE,
    STAT_Z_SCORE_OUTLIERS,
    // Distributions
    STAT_NORMAL_PDF,
    STAT_NORMAL_CDF,
    STAT_NORMAL_QUANTILE,
    STAT_STANDARD_NORMAL_CDF,
    STAT_TWO_SIDED_P,
    STAT_STUDENTS_T_PDF,
    STAT_STUDENTS_T_CDF,
    STAT_STUDENTS_T_TWO_SIDED_P,
    STAT_CHI_SQUARED_PDF,
    STAT_CHI_SQUARED_CDF,
    STAT_CHI_SQUARED_UPPER_P,
    STAT_FISHER_F_PDF,
    STAT_FISHER_F_CDF,
    STAT_FISHER_F_UPPER_P,
    STAT_BINOMIAL_PMF,
    STAT_BINOMIAL_CDF,
    STAT_POISSON_PMF,
    STAT_POISSON_CDF,
    STAT_EXPONENTIAL_PDF,
    STAT_EXPONENTIAL_CDF,
    STAT_GAMMA_PDF,
    STAT_BETA_PDF,
    STAT_WEIBULL_PDF,
    STAT_LOGNORMAL_PDF,
    STAT_UNIFORM_PDF,
    STAT_LAPLACE_PDF,
    STAT_LN_GAMMA,
    STAT_GAMMA_FN,
    STAT_ERF,
    STAT_ERFC,
    STAT_EMPIRICAL_CDF,
    // Extra stats
    STAT_MODE,
    STAT_WINSORIZED_MEAN,
    STAT_CROSS_ENTROPY,
    STAT_MUTUAL_INFORMATION,
    STAT_HISTOGRAM,
    STAT_CORRELATION_P_VALUE,
    STAT_CHI_SQUARE_INDEPENDENCE,
    STAT_MODIFIED_Z_SCORE_OUTLIERS,
    STAT_IQR_OUTLIERS,
    STAT_GRUBBS_TEST,
    STAT_MANN_WHITNEY_U,
    STAT_KS_1SAMPLE,
    STAT_FRIEDMAN,
    STAT_MCNEMAR,
    STAT_BOOTSTRAP_MEANS,
    STAT_LJUNG_BOX,
    STAT_ADF_PROXY,
    STAT_PEARSON,
    ML_OLS,
    ML_MSE,
    ML_RMSE,
    ML_MAE,
    ML_R2,
    ML_ACCURACY,
    ML_ROC_AUC,
    ML_KMEANS,
    ML_TRAIN_TEST_SPLIT,
    ML_LOG_LOSS,
    ML_CONFUSION_BINARY,
    ML_K_FOLD,
    ML_BOOTSTRAP_INDICES,
    ML_BONFERRONI,
    ML_HOLM,
    ML_BH,
    ML_PCA,
    ML_AB_TEST,
    ML_POWER_TWO_SAMPLE,
    ML_REQUIRED_SAMPLE_SIZE,
    ML_TRANSE_SCORE,
    ML_DISTMULT_SCORE,
    PHYS_PROJECTILE,
    BIO_ALIGN,
    CHEM_SMILES,
    CLIN_FRAMINGHAM,
    FIN_BS,
    ENG_KIN,
    ENG_CAUCHY_STRESS,
    ENG_DRAG_FORCE,
    ENG_REYNOLDS,
    ENG_FATIGUE_CYCLES,
    ENG_MINER_DAMAGE,
    CHEM_ELEMENT_SYMBOL,
    CHEM_ATOMIC_NUMBER,
    CHEM_ATOMIC_WEIGHT,
    CHEM_LDA_EXCHANGE,
    CHEM_LDA_CORRELATION_VWN,
    MED_TANIMOTO,
    MED_STRUCTURAL_FINGERPRINT,
    MED_ANALYZE_INTENSITY_GRID,
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
    PULSE_PUBLISH,
    PULSE_PUBLISH_GRAPH_MUTATION,
    PULSE_PUBLISH_NOTIFICATION,
    PULSE_PUBLISH_TELEMETRY,
    PULSE_PUBLISH_AGENT_MESSAGE,
    PULSE_PUBLISH_PRESENCE,
    PULSE_PUBLISH_SYNC,
    PULSE_OPEN_CHANNEL,
    PULSE_CLOSE_CHANNEL,
    PULSE_SET_TRANSPORT,
    FIN_PORTFOLIO,
    COVERAGE_MATRIX,
    CATALOG_TTL,
    RENDER_SCENE,
    RENDER_CSS_ANIMATION,
    RENDER_CSS_COLOR,
    RENDER_CSS_TRANSFORM,
    RENDER_ANIMATION_EVAL_CURVE,
    RENDER_ANIMATION_SPRING_STEP,
    RENDER_ANIMATION_SCLERP,
    RENDER_ANIMATION_EVAL_PRESET,
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
    PRIVACY_GAUSSIAN_SIGMA,
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
    PHYS_FIELD_SAMPLE,
    PHYS_MATERIAL_QUERY,
    PHYS_EVALUATE_INTERACTION,
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
    ASSET_PERSIST,
    ASSET_RESOLVE,
    ASSET_RESOLVE_BY_SPATIAL,
    ASSET_RESOLVE_BY_TOPIC,
    ASSET_RESOLVE_BY_TEMPORAL,
    ASSET_LIST,
    ASSET_COUNT,
    ASSET_PERSIST_CREATE,
    ASSET_PERSIST_ADD_TEMPORAL,
    ASSET_PERSIST_ADD_TOPIC,
    ASSET_PERSIST_SET_SPATIAL,
    ASSET_PERSIST_COMPILE,
    ASSET_PERSIST_TEMPORAL_SPAN,
    ASSET_PERSIST_QUERY_ASPECTS,
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
    ECON_CAPM_EXPECTED_RETURN,
    ECON_CAPM_BETA,
    ECON_GORDON_GROWTH,
    ECON_MULTI_PERIOD_DDM,
    ECON_CCAPM_EQUITY_PREMIUM,
    ECON_CCAPM_SDF,
    ECON_PROSPECT_VALUE,
    ECON_PROBABILITY_WEIGHT,
    ECON_HYPERBOLIC_DISCOUNT,
    ECON_ENDOWMENT_EFFECT,
    ECON_BLACK_SCHOLES,
    ECON_PUT_CALL_PARITY,
    ECON_BINOMIAL_OPTION,
    ECON_MIXED_NASH_2X2,
    ECON_COURNOT_DUOPOLY,
    ECON_BERTRAND_DUOPOLY,
    ECON_STACKELBERG_DUOPOLY,
    ECON_SOLOW_STEADY_STATE,
    ECON_RAMSEY_STEADY_STATE,
    ECON_OLG_STEADY_STATE,
    ECON_GINI,
    ECON_ATKINSON,
    ECON_HEADCOUNT_POVERTY,
    ECON_POVERTY_GAP,
    ECON_UTILITARIAN_WELFARE,
    ECON_RAWLSIAN_WELFARE,
    ECON_NASH_WELFARE,
    ECON_NPV,
    ECON_MEAN_RETURN,
    ECON_SAMPLE_VARIANCE,
    ECON_PORTFOLIO_MAX_DRAWDOWN,
    ECON_HISTORICAL_VAR,
    ECON_HISTORICAL_CVAR,
    ECON_PARAMETRIC_VAR,
    ECON_AUTOCORRELATION,
    ECON_CROSS_CORRELATION,
    ECON_INTERPOLATE_ZERO_RATE,
    ECON_DISCOUNT_FACTOR,
    ECON_FORWARD_RATE,
    ECON_GRAVITY_FLOW,
    ECON_MORANS_I,
    ECON_TRANSFER_PAYMENT,
    ECON_FISCAL_MULTIPLIER,
    ECON_LAFFER_CURVE,
    ECON_CHECK_IR,
    ECON_CHECK_BUDGET_BALANCE,
    ECON_VALIDATE_TRANSITION_MATRIX,
    ECON_TRANSITION_PROBABILITY,
    ECON_EXPECTED_HOLDING_TIME,
    ECON_LABOR_SUPPLY,
    ECON_EFFICIENCY_UNITS,
    ECON_SOCIAL_COST_OF_CARBON,
    ECON_OPTIMAL_POLLUTION,
    ECON_OPTIMAL_ABATEMENT,
    ECON_BELLMAN_UPDATE,
    ECON_MALFEASANCE_DELTA,
    ECON_OLS,
    ECON_AGGREGATE_WEALTH,
    ECON_TOTAL_TRANSPORT_COST,
    ECON_LUCAS_ASSET_PRICE,
    ECON_PRESENT_BIASED_UTILITY,
    ECON_REFERENCE_DEPENDENT_UTILITY,
    ECON_PURE_NASH_EQUILIBRIA,
    ECON_REPEATED_GAME_PAYOFF,
    ECON_BERTRAND_WITH_DEMAND,
    ECON_RAMSEY_EULER_RESIDUAL,
    ECON_NEW_KEYNESIAN_SOLVE,
    ECON_LORENZ_CURVE,
    ECON_DISTRIBUTIONAL_NPV,
    ECON_PORTFOLIO_RETURNS,
    ECON_COVARIANCE_MATRIX,
    ECON_PORTFOLIO_VARIANCE,
    ECON_SIMPLE_RETURNS,
    ECON_LOG_RETURNS,
    ECON_CUMULATIVE_WEALTH,
    ECON_DRAWDOWN,
    ECON_ROLLING_MEAN,
    ECON_ROLLING_VARIANCE,
    ECON_GBM_SIMULATE,
    ECON_STRESS_SCENARIO,
    ECON_BLOCK_BOOTSTRAP,
    ECON_PAR_YIELD,
    ECON_NEAREST_FACILITY,
    ECON_PROGRESSIVE_TAX,
    ECON_VCG_PAYMENT,
    ECON_STRATEGY_PROOFNESS,
    ECON_STATIONARY_DISTRIBUTION,
    ECON_SIMULATE_CHAIN,
    ECON_MEAN_FIRST_PASSAGE,
    ECON_HOUSEHOLD_PRODUCTION_CES,
    ECON_POLLUTION_DAMAGE,
    ECON_MARGINAL_DAMAGE,
    ECON_ABATEMENT_NET_BENEFIT,
    ECON_WLS,
    ECON_IV_2SLS,
    ECON_LOGISTIC_MLE,
    ECON_VALUE_ITERATION,
    ECON_NARRATIVE_DIVERGENCE,
    ECON_EIGENVECTOR_CENTRALITY,
    ECON_DEGREE_CENTRALITY,
    ECON_INTERBANK_CLEARING,
    ECON_LEONTIEF_INVERSE,
    ECON_OUTPUT_MULTIPLIERS,
    ECON_AGENT_BASED_AGGREGATE_WEALTH,
    ECON_VALIDATE_SCALAR_CONSTRAINT,
    ECON_AGGREGATE_PAPER_FILLS,
    CAPABILITY_GRANT,
    CAPABILITY_REVOKE,
    CAPABILITY_TEST_GATING,
    CAPABILITY_AUDIT,
    CAPABILITY_DECLARE,
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
    SCENE_ADD_LIGHT,
    SCENE_LINK_SEMANTIC,
    SCENE_DUPLICATE_NODE,
    SCENE_SET_RENDER_BUDGET,
    SCENE_IK_LOOK_AT,
    SCENE_IK_CCD,
    SCENE_SMOOTH_DAMP,
    SCENE_SMOOTH_DAMP_VEC3,
    RESEARCH_NEW,
    RESEARCH_SET_PURPOSE,
    RESEARCH_DEFINE_SCOPE,
    RESEARCH_ADD_CONSTRAINT,
    RESEARCH_ADD_QUESTION,
    RESEARCH_LINK_QUESTIONS,
    RESEARCH_ADD_CORPUS_ITEM,
    RESEARCH_IMPORT_LITERATURE,
    RESEARCH_IMPORT_DATASET,
    RESEARCH_SET_CORPUS_CONFIDENCE,
    RESEARCH_EXTRACT_FROM_CORPUS,
    RESEARCH_INFER_DARK_LINK,
    RESEARCH_DETECT_PROVENANCE_GAPS,
    RESEARCH_DETECT_CONCEALMENT,
    RESEARCH_CONFIRM_DARK_LINK,
    RESEARCH_REFUTE_DARK_LINK,
    RESEARCH_MAKE_INFERENCE,
    RESEARCH_CHAIN_INFERENCE,
    RESEARCH_SET_INFERENCE_CONFIDENCE,
    RESEARCH_VALIDATE_INFERENCE,
    RESEARCH_NEW_INVESTIGATION,
    RESEARCH_COLLECT_EVIDENCE,
    RESEARCH_SET_RELIABILITY,
    RESEARCH_PROPOSE_HYPOTHESIS,
    RESEARCH_EVALUATE_EVIDENCE,
    RESEARCH_CREATE_TIMELINE,
    RESEARCH_ADD_LINK,
    RESEARCH_FIND_PATH,
    RESEARCH_CREATE_HYPOTHESIS_GRAPH,
    RESEARCH_CONTRIBUTE_EVALUATION,
    RESEARCH_BRIDGE_DARK_LINK,
    RESEARCH_REFRAME_HYPOTHESIS,
    RESEARCH_MERGE_HYPOTHESES,
    RESEARCH_FLAG_GAP,
    RESEARCH_CLOSE_GAP,
    RESEARCH_CREATE_REVISION,
    RESEARCH_DIFF_REVISIONS,
    RESEARCH_SUBSCRIBE_UPDATES,
    RESEARCH_CREATE_ASSESSMENT,
    RESEARCH_SET_EPISTEMIC_MODE,
    RESEARCH_SET_REALITY_CATEGORY,
    RESEARCH_CLASSIFY_REALITY,
    RESEARCH_DETECT_BLENDED,
    RESEARCH_DETECT_DECEPTIVE_FICTION,
    RESEARCH_TRACE_FICTION,
    RESEARCH_ASSESS_SENTIMENT,
    RESEARCH_DETECT_SENTIMENT_MANIPULATION,
    RESEARCH_DETECT_PERFORMED_SENTIMENT,
    RESEARCH_MAP_SENTIMENT_NETWORK,
    RESEARCH_ANALYSE_SENTIMENT_TRENDS,
    RESEARCH_REGISTER_PERSPECTIVE,
    RESEARCH_ADD_BIAS,
    RESEARCH_COMPARE_PERSPECTIVES,
    RESEARCH_DETECT_PERSPECTIVE_CONFLICT,
    RESEARCH_RECONCILE_PERSPECTIVES,
    RESEARCH_ASSESS_INTENTIONALITY,
    RESEARCH_CLASSIFY_MISTAKE,
    RESEARCH_DEFINE_SOCIAL_DYNAMICS,
    RESEARCH_DEFINE_ECONOMIC_DYNAMICS,
    RESEARCH_DEFINE_SPATIOTEMPORAL_DYNAMICS,
    RESEARCH_ANALYSE_SOCIAL_NETWORK,
    RESEARCH_ANALYSE_INEQUALITY,
    RESEARCH_ANALYSE_DIFFUSION,
    RESEARCH_ASSESS_GROUNDING,
    RESEARCH_VERIFY_GROUNDING,
    RESEARCH_DETECT_UNGROUNDED_BEHAVIOUR,
    RESEARCH_CREATE_UG_INSTANCE,
    RESEARCH_SET_UG_CAUSE,
    RESEARCH_SET_UG_CONSEQUENCE,
    RESEARCH_SET_UG_DETECTION,
    RESEARCH_SET_UG_MITIGATION,
    RESEARCH_SET_UG_CALIBRATION,
    RESEARCH_DETECT_UG_PATTERNS,
    IMAGE_NEW,
    IMAGE_ADD_LAYER,
    IMAGE_REMOVE_LAYER,
    IMAGE_SET_PIXEL,
    IMAGE_FILL,
    IMAGE_BRUSH,
    IMAGE_APPLY_FILTER,
    IMAGE_SET_OPACITY,
    IMAGE_SET_BLEND_MODE,
    IMAGE_SET_VISIBLE,
    IMAGE_SET_MASK,
    IMAGE_CLEAR_MASK,
    IMAGE_COMPOSITE,
    IMAGE_ADD_SELECTION,
    IMAGE_CLEAR_SELECTIONS,
    VIDEO_NEW_PROJECT,
    VIDEO_ADD_TRACK,
    VIDEO_ADD_CLIP,
    VIDEO_TRIM_CLIP,
    VIDEO_SET_SPEED,
    VIDEO_COLOUR_GRADE,
    VIDEO_ADD_TRANSITION,
    VIDEO_SET_RENDER_FORMAT,
    VIDEO_SET_RENDER_BITRATE,
    VIDEO_REMOVE_CLIP,
    THREE_D_ADD_OBJECT,
    THREE_D_SET_TRANSFORM,
    THREE_D_SET_MATERIAL,
    THREE_D_ADD_CAMERA,
    THREE_D_ADD_LIGHT,
    THREE_D_ADD_RIG,
    THREE_D_ADD_ANIMATION,
    THREE_D_SET_MESH,
    HBBTV_NEW_APP,
    HBBTV_ADD_PAGE,
    HBBTV_NAVIGATE,
    HBBTV_SET_STATE,
    SECOND_SCREEN_SYNC,
    INTERACTIVE_ADD_TRIGGER,
    INTERACTIVE_ADD_SOCIAL_POST,
    WORLD_NEW,
    WORLD_ADD_OBJECT,
    WORLD_ADD_PORTAL,
    WORLD_ADD_AVATAR,
    WORLD_SET_GRAVITY,
    WORLD_OBJECT_APPLY_FORCE,
    WORLD_OBJECT_STEP_PHYSICS,
    PORTAL_SET_TARGET,
    PORTAL_ACTIVATE,
    PORTAL_DEACTIVATE,
    AVATAR_MOVE,
    AVATAR_SET_APPEARANCE,
    DMX_NEW_UNIVERSE,
    DMX_SET_CHANNEL,
    DMX_ADD_FIXTURE,
    DMX_FIXTURE_SET_COLOUR,
    DMX_FIXTURE_SET_INTENSITY,
    DMX_FIXTURE_SET_PAN_TILT,
    DMX_NEW_CUE,
    DMX_CUE_SET_CHANNEL,
    DMX_CUE_SET_FADE,
    DMX_NEW_CUE_STACK,
    DMX_CUE_STACK_ADD,
    DMX_CUE_STACK_GO,
    DMX_CUE_STACK_GO_BACK,
    DMX_CUE_STACK_RESET,
];

/// Future extract target for an invoke id. Not a crate today.
pub fn seam_for(id: &str) -> &'static str {
    match id {
        DAG_EXECUTE
        | DAG_VALIDATE
        | DAG_STATUS
        | ORCH_SESSION_CREATE
        | ORCH_SESSION_PLAN
        | ORCH_SESSION_EXECUTE
        | ORCH_SESSION_STATUS
        | ORCH_ROSTER_REGISTER
        | ORCH_ROSTER_LIST
        | ORCH_ROSTER_CAPABILITIES
        | ORCH_ASSIGN_AGENTS => "agent",
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
        STAT_MEAN
        | STAT_PEARSON
        | STAT_LINEAR_REGRESSION
        | STAT_MEDIAN
        | STAT_VARIANCE
        | STAT_STD_DEV
        | STAT_SKEWNESS
        | STAT_KURTOSIS
        | STAT_QUANTILE
        | STAT_COVARIANCE
        | STAT_MIN
        | STAT_MAX
        | STAT_SUM
        | STAT_SPEARMAN
        | STAT_KENDALL
        | STAT_ONE_SAMPLE_T
        | STAT_TWO_SAMPLE_T
        | STAT_PAIRED_T
        | STAT_CHI_SQUARE_GOF
        | STAT_ONE_WAY_ANOVA
        | STAT_AUTOCORRELATION
        | STAT_MOVING_AVERAGE
        | STAT_EXPONENTIAL_SMOOTHING
        | STAT_TRIMMED_MEAN
        | STAT_IQR
        | STAT_MAD
        | STAT_ENTROPY
        | STAT_KL_DIVERGENCE
        | STAT_Z_SCORE_OUTLIERS
        | STAT_NORMAL_PDF
        | STAT_NORMAL_CDF
        | STAT_NORMAL_QUANTILE
        | STAT_STANDARD_NORMAL_CDF
        | STAT_TWO_SIDED_P
        | STAT_STUDENTS_T_PDF
        | STAT_STUDENTS_T_CDF
        | STAT_STUDENTS_T_TWO_SIDED_P
        | STAT_CHI_SQUARED_PDF
        | STAT_CHI_SQUARED_CDF
        | STAT_CHI_SQUARED_UPPER_P
        | STAT_FISHER_F_PDF
        | STAT_FISHER_F_CDF
        | STAT_FISHER_F_UPPER_P
        | STAT_BINOMIAL_PMF
        | STAT_BINOMIAL_CDF
        | STAT_POISSON_PMF
        | STAT_POISSON_CDF
        | STAT_EXPONENTIAL_PDF
        | STAT_EXPONENTIAL_CDF
        | STAT_GAMMA_PDF
        | STAT_BETA_PDF
        | STAT_WEIBULL_PDF
        | STAT_LOGNORMAL_PDF
        | STAT_UNIFORM_PDF
        | STAT_LAPLACE_PDF
        | STAT_LN_GAMMA
        | STAT_GAMMA_FN
        | STAT_ERF
        | STAT_ERFC
        | STAT_EMPIRICAL_CDF
        | STAT_MODE
        | STAT_WINSORIZED_MEAN
        | STAT_CROSS_ENTROPY
        | STAT_MUTUAL_INFORMATION
        | STAT_HISTOGRAM
        | STAT_CORRELATION_P_VALUE
        | STAT_CHI_SQUARE_INDEPENDENCE
        | STAT_MODIFIED_Z_SCORE_OUTLIERS
        | STAT_IQR_OUTLIERS
        | STAT_GRUBBS_TEST
        | STAT_MANN_WHITNEY_U
        | STAT_KS_1SAMPLE
        | STAT_FRIEDMAN
        | STAT_MCNEMAR
        | STAT_BOOTSTRAP_MEANS
        | STAT_LJUNG_BOX
        | STAT_ADF_PROXY => "stats",
        GEOM_HULL2
        | GEOM_DISTANCE_2D
        | GEOM_DISTANCE_3D
        | GEOM_POINT_SEGMENT_DISTANCE_2D
        | GEOM_POINT_SEGMENT_DISTANCE_3D
        | GEOM_POINT_TRIANGLE_DISTANCE_3D => "geometry",
        VISION_AHASH
        | VISION_GAUSSIAN_BLUR
        | VISION_SOBEL_MAGNITUDE
        | VISION_CANNY_EDGES
        | VISION_HISTOGRAM
        | VISION_EQUALIZE_HIST
        | VISION_RGB_TO_GRAY
        | VISION_DHASH
        | VISION_HAMMING_DISTANCE
        | VISION_COSINE_SIMILARITY => "vision",
        ML_OLS
        | ML_MSE
        | ML_RMSE
        | ML_MAE
        | ML_R2
        | ML_ACCURACY
        | ML_ROC_AUC
        | ML_KMEANS
        | ML_TRAIN_TEST_SPLIT
        | ML_LOG_LOSS
        | ML_CONFUSION_BINARY
        | ML_K_FOLD
        | ML_BOOTSTRAP_INDICES
        | ML_BONFERRONI
        | ML_HOLM
        | ML_BH
        | ML_PCA
        | ML_AB_TEST
        | ML_POWER_TWO_SAMPLE
        | ML_REQUIRED_SAMPLE_SIZE
        | ML_TRANSE_SCORE
        | ML_DISTMULT_SCORE => "ml",
        PHYS_PROJECTILE | BIO_ALIGN | CHEM_SMILES => "science",
        CHEM_ELEMENT_SYMBOL
        | CHEM_ATOMIC_NUMBER
        | CHEM_ATOMIC_WEIGHT
        | CHEM_LDA_EXCHANGE
        | CHEM_LDA_CORRELATION_VWN => "chemistry",
        MED_TANIMOTO | MED_STRUCTURAL_FINGERPRINT | MED_ANALYZE_INTENSITY_GRID => "medical",
        ENG_CAUCHY_STRESS | ENG_DRAG_FORCE | ENG_REYNOLDS | ENG_FATIGUE_CYCLES
        | ENG_MINER_DAMAGE => "engineering",
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
        | PHYS_EMF_SAMPLE_AT_DEPTH
        | PHYS_FIELD_SAMPLE
        | PHYS_MATERIAL_QUERY
        | PHYS_EVALUATE_INTERACTION => "physics",
        SPECTRAL_EMF_TO_SPD | SPECTRAL_SPD_TO_XYZ | SPECTRAL_EMF_TO_RGB | SPECTRAL_BLEND
        | SPECTRAL_GAMUT_MAP => "spectral",
        CLIN_FRAMINGHAM => "clinical",
        BIOSIGNAL_DP_FILTER | BIOSIGNAL_DP_CONFIG => "biosignal",
        FIN_BS => "econ",
        ENG_KIN => "engineering",
        ID_DID_Q42 => "governance",
        CRYPTO_SHA256 | CRYPTO_SHA512 | CRYPTO_BLAKE3 | PRIVACY_GAUSSIAN_SIGMA => "crypto",
        MANIFOLD_DISTANCE | MANIFOLD_AXES | MANIFOLD_PROJECT => "manifold",
        DOC_INGEST => "docs",
        SHEET_STATS | SHEET_SUM => "sheet",
        SOCIAL_LWW => "social",
        NET_PEER
        | NET_SONIC
        | PULSE_PUBLISH
        | PULSE_PUBLISH_GRAPH_MUTATION
        | PULSE_PUBLISH_NOTIFICATION
        | PULSE_PUBLISH_TELEMETRY
        | PULSE_PUBLISH_AGENT_MESSAGE
        | PULSE_PUBLISH_PRESENCE
        | PULSE_PUBLISH_SYNC
        | PULSE_OPEN_CHANNEL
        | PULSE_CLOSE_CHANNEL
        | PULSE_SET_TRANSPORT => "net",
        FIN_PORTFOLIO => "econ",
        RENDER_SCENE
        | RENDER_CSS_ANIMATION
        | RENDER_CSS_COLOR
        | RENDER_CSS_TRANSFORM
        | RENDER_ANIMATION_EVAL_CURVE
        | RENDER_ANIMATION_SPRING_STEP
        | RENDER_ANIMATION_SCLERP
        | RENDER_ANIMATION_EVAL_PRESET
        | RENDER_SVG_PATH
        | RENDER_SVG_CIRCLE
        | RENDER_SVG_RECT
        | RENDER_SVG_LINE
        | RENDER_SVG_BEZIER
        | RENDER_SVG_FIELD
        | GPU_ADAPTER_INFO
        | GPU_INIT
        | GPU_RENDER_FRAME
        | GPU_READ_PIXELS
        | GPU_UPLOAD_MESH
        | GPU_UPLOAD_TENSOR
        | GPU_SET_CAMERA
        | GPU_PICK
        | GPU_POLL_PICK
        | GPU_RESIZE
        | GPU_SET_AMBIENT
        | GPU_DESTROY
        | GPU_COMPUTE_DISPATCH
        | GPU_COMPUTE_READBACK
        | GPU_VALIDATE_SHADER
        | GPU_COMPILE_SHADER
        | GPU_COMPILE_TO_GLSL
        | GPU_BACKEND_INFO
        | EMF_UPLOAD_FIELD
        | EMF_RENDER_SLICE
        | EMF_FIELD_INFO => "render",
        SAMPLER_CONFIGURE
        | SAMPLER_CONSTRAIN_ENABLE
        | SAMPLER_CONSTRAIN_DISABLE
        | SAMPLER_CONSTRAIN_RESET
        | SAMPLER_SAMPLE => "sampler",
        ASSET_CREATE
        | ASSET_ADD_TEMPORAL
        | ASSET_ADD_TOPIC
        | ASSET_SET_SPATIAL
        | ASSET_COMPILE
        | ASSET_TEMPORAL_SPAN
        | ASSET_QUERY_ASPECTS
        | ASSET_PERSIST
        | ASSET_RESOLVE
        | ASSET_RESOLVE_BY_SPATIAL
        | ASSET_RESOLVE_BY_TOPIC
        | ASSET_RESOLVE_BY_TEMPORAL
        | ASSET_LIST
        | ASSET_COUNT
        | ASSET_PERSIST_CREATE
        | ASSET_PERSIST_ADD_TEMPORAL
        | ASSET_PERSIST_ADD_TOPIC
        | ASSET_PERSIST_SET_SPATIAL
        | ASSET_PERSIST_COMPILE
        | ASSET_PERSIST_TEMPORAL_SPAN
        | ASSET_PERSIST_QUERY_ASPECTS => "asset",
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
        FINANCE_CONVERT_CURRENCY
        | FINANCE_MULTISIG_CHECK
        | FINANCE_LEDGER_BALANCE
        | ECON_CAPM_EXPECTED_RETURN
        | ECON_CAPM_BETA
        | ECON_GORDON_GROWTH
        | ECON_MULTI_PERIOD_DDM
        | ECON_CCAPM_EQUITY_PREMIUM
        | ECON_CCAPM_SDF
        | ECON_PROSPECT_VALUE
        | ECON_PROBABILITY_WEIGHT
        | ECON_HYPERBOLIC_DISCOUNT
        | ECON_ENDOWMENT_EFFECT
        | ECON_BLACK_SCHOLES
        | ECON_PUT_CALL_PARITY
        | ECON_BINOMIAL_OPTION
        | ECON_MIXED_NASH_2X2
        | ECON_COURNOT_DUOPOLY
        | ECON_BERTRAND_DUOPOLY
        | ECON_STACKELBERG_DUOPOLY
        | ECON_SOLOW_STEADY_STATE
        | ECON_RAMSEY_STEADY_STATE
        | ECON_OLG_STEADY_STATE
        | ECON_GINI
        | ECON_ATKINSON
        | ECON_HEADCOUNT_POVERTY
        | ECON_POVERTY_GAP
        | ECON_UTILITARIAN_WELFARE
        | ECON_RAWLSIAN_WELFARE
        | ECON_NASH_WELFARE
        | ECON_NPV
        | ECON_MEAN_RETURN
        | ECON_SAMPLE_VARIANCE
        | ECON_PORTFOLIO_MAX_DRAWDOWN
        | ECON_HISTORICAL_VAR
        | ECON_HISTORICAL_CVAR
        | ECON_PARAMETRIC_VAR
        | ECON_AUTOCORRELATION
        | ECON_CROSS_CORRELATION
        | ECON_INTERPOLATE_ZERO_RATE
        | ECON_DISCOUNT_FACTOR
        | ECON_FORWARD_RATE
        | ECON_GRAVITY_FLOW
        | ECON_MORANS_I
        | ECON_TRANSFER_PAYMENT
        | ECON_FISCAL_MULTIPLIER
        | ECON_LAFFER_CURVE
        | ECON_CHECK_IR
        | ECON_CHECK_BUDGET_BALANCE
        | ECON_VALIDATE_TRANSITION_MATRIX
        | ECON_TRANSITION_PROBABILITY
        | ECON_EXPECTED_HOLDING_TIME
        | ECON_LABOR_SUPPLY
        | ECON_EFFICIENCY_UNITS
        | ECON_SOCIAL_COST_OF_CARBON
        | ECON_OPTIMAL_POLLUTION
        | ECON_OPTIMAL_ABATEMENT
        | ECON_BELLMAN_UPDATE
        | ECON_MALFEASANCE_DELTA
        | ECON_OLS
        | ECON_AGGREGATE_WEALTH
        | ECON_TOTAL_TRANSPORT_COST
        | ECON_LUCAS_ASSET_PRICE
        | ECON_PRESENT_BIASED_UTILITY
        | ECON_REFERENCE_DEPENDENT_UTILITY
        | ECON_PURE_NASH_EQUILIBRIA
        | ECON_REPEATED_GAME_PAYOFF
        | ECON_BERTRAND_WITH_DEMAND
        | ECON_RAMSEY_EULER_RESIDUAL
        | ECON_NEW_KEYNESIAN_SOLVE
        | ECON_LORENZ_CURVE
        | ECON_DISTRIBUTIONAL_NPV
        | ECON_PORTFOLIO_RETURNS
        | ECON_COVARIANCE_MATRIX
        | ECON_PORTFOLIO_VARIANCE
        | ECON_SIMPLE_RETURNS
        | ECON_LOG_RETURNS
        | ECON_CUMULATIVE_WEALTH
        | ECON_DRAWDOWN
        | ECON_ROLLING_MEAN
        | ECON_ROLLING_VARIANCE
        | ECON_GBM_SIMULATE
        | ECON_STRESS_SCENARIO
        | ECON_BLOCK_BOOTSTRAP
        | ECON_PAR_YIELD
        | ECON_NEAREST_FACILITY
        | ECON_PROGRESSIVE_TAX
        | ECON_VCG_PAYMENT
        | ECON_STRATEGY_PROOFNESS
        | ECON_STATIONARY_DISTRIBUTION
        | ECON_SIMULATE_CHAIN
        | ECON_MEAN_FIRST_PASSAGE
        | ECON_HOUSEHOLD_PRODUCTION_CES
        | ECON_POLLUTION_DAMAGE
        | ECON_MARGINAL_DAMAGE
        | ECON_ABATEMENT_NET_BENEFIT
        | ECON_WLS
        | ECON_IV_2SLS
        | ECON_LOGISTIC_MLE
        | ECON_VALUE_ITERATION
        | ECON_NARRATIVE_DIVERGENCE
        | ECON_EIGENVECTOR_CENTRALITY
        | ECON_DEGREE_CENTRALITY
        | ECON_INTERBANK_CLEARING
        | ECON_LEONTIEF_INVERSE
        | ECON_OUTPUT_MULTIPLIERS
        | ECON_AGENT_BASED_AGGREGATE_WEALTH
        | ECON_VALIDATE_SCALAR_CONSTRAINT
        | ECON_AGGREGATE_PAPER_FILLS => "econ",
        CAPABILITY_GRANT
        | CAPABILITY_REVOKE
        | CAPABILITY_TEST_GATING
        | CAPABILITY_AUDIT
        | CAPABILITY_DECLARE
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
        SCENE_ADD_LIGHT
        | SCENE_LINK_SEMANTIC
        | SCENE_DUPLICATE_NODE
        | SCENE_SET_RENDER_BUDGET
        | SCENE_IK_LOOK_AT
        | SCENE_IK_CCD
        | SCENE_SMOOTH_DAMP
        | SCENE_SMOOTH_DAMP_VEC3 => "render",
        SCENE_CREATE
        | SCENE_ADD_NODE
        | SCENE_SET_TRANSFORM
        | SCENE_SET_MESH
        | SCENE_ADD_CAMERA
        | SCENE_RENDER
        | SCENE_SET_VIEWPORT
        | SCENE_SET_CLEAR_COLOUR
        | SCENE_CAPTURE_FRAME => "render",
        RESEARCH_NEW
        | RESEARCH_SET_PURPOSE
        | RESEARCH_DEFINE_SCOPE
        | RESEARCH_ADD_CONSTRAINT
        | RESEARCH_ADD_QUESTION
        | RESEARCH_LINK_QUESTIONS
        | RESEARCH_ADD_CORPUS_ITEM
        | RESEARCH_IMPORT_LITERATURE
        | RESEARCH_IMPORT_DATASET
        | RESEARCH_SET_CORPUS_CONFIDENCE
        | RESEARCH_EXTRACT_FROM_CORPUS
        | RESEARCH_INFER_DARK_LINK
        | RESEARCH_DETECT_PROVENANCE_GAPS
        | RESEARCH_DETECT_CONCEALMENT
        | RESEARCH_CONFIRM_DARK_LINK
        | RESEARCH_REFUTE_DARK_LINK
        | RESEARCH_MAKE_INFERENCE
        | RESEARCH_CHAIN_INFERENCE
        | RESEARCH_SET_INFERENCE_CONFIDENCE
        | RESEARCH_VALIDATE_INFERENCE
        | RESEARCH_NEW_INVESTIGATION
        | RESEARCH_COLLECT_EVIDENCE
        | RESEARCH_SET_RELIABILITY
        | RESEARCH_PROPOSE_HYPOTHESIS
        | RESEARCH_EVALUATE_EVIDENCE
        | RESEARCH_CREATE_TIMELINE
        | RESEARCH_ADD_LINK
        | RESEARCH_FIND_PATH
        | RESEARCH_CREATE_HYPOTHESIS_GRAPH
        | RESEARCH_CONTRIBUTE_EVALUATION
        | RESEARCH_BRIDGE_DARK_LINK
        | RESEARCH_REFRAME_HYPOTHESIS
        | RESEARCH_MERGE_HYPOTHESES
        | RESEARCH_FLAG_GAP
        | RESEARCH_CLOSE_GAP
        | RESEARCH_CREATE_REVISION
        | RESEARCH_DIFF_REVISIONS
        | RESEARCH_SUBSCRIBE_UPDATES
        | RESEARCH_CREATE_ASSESSMENT
        | RESEARCH_SET_EPISTEMIC_MODE
        | RESEARCH_SET_REALITY_CATEGORY
        | RESEARCH_CLASSIFY_REALITY
        | RESEARCH_DETECT_BLENDED
        | RESEARCH_DETECT_DECEPTIVE_FICTION
        | RESEARCH_TRACE_FICTION
        | RESEARCH_ASSESS_SENTIMENT
        | RESEARCH_DETECT_SENTIMENT_MANIPULATION
        | RESEARCH_DETECT_PERFORMED_SENTIMENT
        | RESEARCH_MAP_SENTIMENT_NETWORK
        | RESEARCH_ANALYSE_SENTIMENT_TRENDS
        | RESEARCH_REGISTER_PERSPECTIVE
        | RESEARCH_ADD_BIAS
        | RESEARCH_COMPARE_PERSPECTIVES
        | RESEARCH_DETECT_PERSPECTIVE_CONFLICT
        | RESEARCH_RECONCILE_PERSPECTIVES
        | RESEARCH_ASSESS_INTENTIONALITY
        | RESEARCH_CLASSIFY_MISTAKE
        | RESEARCH_DEFINE_SOCIAL_DYNAMICS
        | RESEARCH_DEFINE_ECONOMIC_DYNAMICS
        | RESEARCH_DEFINE_SPATIOTEMPORAL_DYNAMICS
        | RESEARCH_ANALYSE_SOCIAL_NETWORK
        | RESEARCH_ANALYSE_INEQUALITY
        | RESEARCH_ANALYSE_DIFFUSION
        | RESEARCH_ASSESS_GROUNDING
        | RESEARCH_VERIFY_GROUNDING
        | RESEARCH_DETECT_UNGROUNDED_BEHAVIOUR
        | RESEARCH_CREATE_UG_INSTANCE
        | RESEARCH_SET_UG_CAUSE
        | RESEARCH_SET_UG_CONSEQUENCE
        | RESEARCH_SET_UG_DETECTION
        | RESEARCH_SET_UG_MITIGATION
        | RESEARCH_SET_UG_CALIBRATION
        | RESEARCH_DETECT_UG_PATTERNS => "research",
        IMAGE_NEW
        | IMAGE_ADD_LAYER
        | IMAGE_REMOVE_LAYER
        | IMAGE_SET_PIXEL
        | IMAGE_FILL
        | IMAGE_BRUSH
        | IMAGE_APPLY_FILTER
        | IMAGE_SET_OPACITY
        | IMAGE_SET_BLEND_MODE
        | IMAGE_SET_VISIBLE
        | IMAGE_SET_MASK
        | IMAGE_CLEAR_MASK
        | IMAGE_COMPOSITE
        | IMAGE_ADD_SELECTION
        | IMAGE_CLEAR_SELECTIONS
        | VIDEO_NEW_PROJECT
        | VIDEO_ADD_TRACK
        | VIDEO_ADD_CLIP
        | VIDEO_TRIM_CLIP
        | VIDEO_SET_SPEED
        | VIDEO_COLOUR_GRADE
        | VIDEO_ADD_TRANSITION
        | VIDEO_SET_RENDER_FORMAT
        | VIDEO_SET_RENDER_BITRATE
        | VIDEO_REMOVE_CLIP
        | THREE_D_ADD_OBJECT
        | THREE_D_SET_TRANSFORM
        | THREE_D_SET_MATERIAL
        | THREE_D_ADD_CAMERA
        | THREE_D_ADD_LIGHT
        | THREE_D_ADD_RIG
        | THREE_D_ADD_ANIMATION
        | THREE_D_SET_MESH
        | HBBTV_NEW_APP
        | HBBTV_ADD_PAGE
        | HBBTV_NAVIGATE
        | HBBTV_SET_STATE
        | SECOND_SCREEN_SYNC
        | INTERACTIVE_ADD_TRIGGER
        | INTERACTIVE_ADD_SOCIAL_POST
        | WORLD_NEW
        | WORLD_ADD_OBJECT
        | WORLD_ADD_PORTAL
        | WORLD_ADD_AVATAR
        | WORLD_SET_GRAVITY
        | WORLD_OBJECT_APPLY_FORCE
        | WORLD_OBJECT_STEP_PHYSICS
        | PORTAL_SET_TARGET
        | PORTAL_ACTIVATE
        | PORTAL_DEACTIVATE
        | AVATAR_MOVE
        | AVATAR_SET_APPEARANCE
        | DMX_NEW_UNIVERSE
        | DMX_SET_CHANNEL
        | DMX_ADD_FIXTURE
        | DMX_FIXTURE_SET_COLOUR
        | DMX_FIXTURE_SET_INTENSITY
        | DMX_FIXTURE_SET_PAN_TILT
        | DMX_NEW_CUE
        | DMX_CUE_SET_CHANNEL
        | DMX_CUE_SET_FADE
        | DMX_NEW_CUE_STACK
        | DMX_CUE_STACK_ADD
        | DMX_CUE_STACK_GO
        | DMX_CUE_STACK_GO_BACK
        | DMX_CUE_STACK_RESET => "hypermedia",
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
