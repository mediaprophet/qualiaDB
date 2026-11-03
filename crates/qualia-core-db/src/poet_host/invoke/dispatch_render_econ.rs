//! Render, GPU, sampler, asset, cosmic, NLP, inference, and computational economics dispatch arms.

use super::super::PoetSnapshot;
use super::{
    asset_bind, asset_store, cosmic_bind, econ, ids, inference, nlp, render, sampler_bind,
};
use vibe::{Diagnostic, Span, Value};

pub(super) fn try_dispatch(
    snap: &mut PoetSnapshot,
    id: &str,
    args: &Value,
    span: Span,
) -> Option<Result<Value, Diagnostic>> {
    let res = match id {
        ids::RENDER_SCENE => render::scene(snap, args, span),
        ids::RENDER_CSS_ANIMATION => render::css_animation(args, span),
        ids::RENDER_CSS_COLOR => render::css_color(args, span),
        ids::RENDER_CSS_TRANSFORM => render::css_transform(args, span),
        ids::RENDER_ANIMATION_EVAL_CURVE | ids::ANIMATION_BEZIER_EVAL | ids::ANIMATION_EASING => {
            render::animation_eval_curve(args, span)
        }
        ids::RENDER_ANIMATION_SPRING_STEP | ids::ANIMATION_SPRING_STEP => {
            render::animation_spring_step(args, span)
        }
        ids::RENDER_ANIMATION_SCLERP | ids::ANIMATION_SCLERP_STEP => {
            render::animation_sclerp(args, span)
        }
        ids::RENDER_ANIMATION_SQUAD_STEP | ids::ANIMATION_SQUAD_STEP => {
            render::animation_squad_step(args, span)
        }
        ids::RENDER_ANIMATION_EVAL_PRESET | ids::ANIMATION_EVALUATE_PRESET => {
            render::animation_eval_preset(args, span)
        }
        ids::RENDER_ANIMATION_LIST_PRESETS | ids::ANIMATION_LIST_PRESETS => {
            render::animation_list_presets(args, span)
        }
        #[cfg(feature = "gpu-runtime")]
        ids::RENDER_ANIMATION_COMPUTE_PASS => render::animation_compute_pass(args, span),
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
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_ADAPTER_INFO => render::gpu_adapter_info(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_INIT => render::gpu_init(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_INIT_SURFACE => render::gpu_init_surface(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_RENDER_FRAME => render::gpu_render_frame(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_READ_PIXELS => render::gpu_read_pixels(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_UPLOAD_MESH => render::gpu_upload_mesh(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_UPLOAD_TENSOR => render::gpu_upload_tensor(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_SET_CAMERA => render::gpu_set_camera(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_PICK => render::gpu_pick(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_POLL_PICK => render::gpu_poll_pick(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_RESIZE => render::gpu_resize(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_SET_AMBIENT => render::gpu_set_ambient(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_DESTROY => render::gpu_destroy(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_COMPUTE_DISPATCH => render::gpu_compute_dispatch(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::GPU_COMPUTE_READBACK => render::gpu_compute_readback(args, span),
        ids::GPU_VALIDATE_SHADER => render::gpu_validate_shader(args, span),
        ids::GPU_COMPILE_SHADER => render::gpu_compile_shader(args, span),
        ids::GPU_COMPILE_TO_GLSL => render::gpu_compile_to_glsl(args, span),
        ids::GPU_BACKEND_INFO => render::gpu_backend_info(args, span),
        ids::GPU_UPLOAD_MESH_COLORED => render::gpu_upload_mesh_colored(args, span),
        ids::GPU_SET_STANDPOINT => render::gpu_set_standpoint(args, span),
        ids::GPU_OBSERVER_STANDPOINT => render::gpu_observer_standpoint(args, span),
        ids::GPU_CAMERA_STATE => render::gpu_camera_state(args, span),
        ids::GPU_SURFACE_SIZE => render::gpu_surface_size(args, span),
        ids::GPU_HAS_MESH => render::gpu_has_mesh(args, span),
        ids::GPU_HAS_TENSOR => render::gpu_has_tensor(args, span),
        ids::GPU_TENSOR_NODE_COUNT => render::gpu_tensor_node_count(args, span),
        ids::GPU_PARTICLE_COUNT => render::gpu_particle_count(args, span),
        ids::GPU_SYNC_BLOOM => render::gpu_sync_bloom(args, span),
        ids::GPU_SET_ARTEFACT_JOINT => render::gpu_set_artefact_joint(args, span),
        ids::GPU_SET_ARTEFACT_WORLD => render::gpu_set_artefact_world(args, span),
        ids::GPU_ARTEFACT_REFUSED => render::gpu_artefact_refused(args, span),
        ids::GPU_REQUIRED_RGBA8_BYTES => render::gpu_required_rgba8_bytes(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::EMF_UPLOAD_FIELD => render::emf_upload_field(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::EMF_RENDER_SLICE => render::emf_render_slice(args, span),
        #[cfg(feature = "gpu-runtime")]
        ids::EMF_FIELD_INFO => render::emf_field_info(args, span),
        ids::RENDER_TEMPORAL_SET_MODE => render::render_temporal_set_mode(args, span),
        ids::RENDER_TEMPORAL_STATUS => render::render_temporal_status(args, span),
        ids::RENDER_SET_ENVIRONMENT_PROBE => render::render_set_environment_probe(args, span),
        ids::RENDER_WATER_SET_PARAMETERS => render::render_water_set_parameters(args, span),
        ids::RENDER_VEGETATION_PUSH_COLLIDER => render::render_vegetation_push_collider(args, span),
        ids::RENDER_VEGETATION_CLEAR_COLLIDERS => {
            render::render_vegetation_clear_colliders(args, span)
        }
        ids::RENDER_TEXTURE_TRANSCODE_CAPS => render::render_texture_transcode_caps(args, span),
        ids::SAMPLER_CONFIGURE => sampler_bind::configure(args, span),
        ids::SAMPLER_CONSTRAIN_ENABLE => sampler_bind::constrain_enable(args, span),
        ids::SAMPLER_CONSTRAIN_DISABLE => sampler_bind::constrain_disable(args, span),
        ids::SAMPLER_CONSTRAIN_RESET => sampler_bind::constrain_reset(args, span),
        ids::SAMPLER_SAMPLE => sampler_bind::sample(args, span),
        ids::ASSET_CREATE => asset_bind::create(args, span),
        ids::ASSET_ADD_TEMPORAL => asset_bind::add_temporal(args, span),
        ids::ASSET_ADD_TOPIC => asset_bind::add_topic(args, span),
        ids::ASSET_SET_SPATIAL => asset_bind::set_spatial(args, span),
        ids::ASSET_COMPILE => asset_bind::compile(args, span),
        ids::ASSET_TEMPORAL_SPAN => asset_bind::temporal_span(args, span),
        ids::ASSET_QUERY_ASPECTS => asset_bind::query_aspects(args, span),
        ids::ASSET_PERSIST => asset_store::persist_seam(args, span),
        ids::ASSET_RESOLVE => asset_store::resolve_seam(args, span),
        ids::ASSET_RESOLVE_BY_SPATIAL => asset_store::resolve_by_spatial_seam(args, span),
        ids::ASSET_RESOLVE_BY_TOPIC => asset_store::resolve_by_topic_seam(args, span),
        ids::ASSET_RESOLVE_BY_TEMPORAL => asset_store::resolve_by_temporal_seam(args, span),
        ids::ASSET_LIST => asset_store::list_seam(args, span),
        ids::ASSET_COUNT => asset_store::count_seam(args, span),
        ids::ASSET_PERSIST_CREATE => asset_store::persist_create_seam(args, span),
        ids::ASSET_PERSIST_ADD_TEMPORAL => asset_store::persist_add_temporal_seam(args, span),
        ids::ASSET_PERSIST_ADD_TOPIC => asset_store::persist_add_topic_seam(args, span),
        ids::ASSET_PERSIST_SET_SPATIAL => asset_store::persist_set_spatial_seam(args, span),
        ids::ASSET_PERSIST_COMPILE => asset_store::persist_compile_seam(args, span),
        ids::ASSET_PERSIST_TEMPORAL_SPAN => asset_store::persist_temporal_span_seam(args, span),
        ids::ASSET_PERSIST_QUERY_ASPECTS => asset_store::persist_query_aspects_seam(args, span),
        ids::COSMIC_GEODETIC_TO_ECEF => cosmic_bind::geodetic_to_ecef(args, span),
        ids::COSMIC_ECEF_TO_GEODETIC => cosmic_bind::ecef_to_geodetic(args, span),
        ids::COSMIC_ECEF_TO_ENU => cosmic_bind::ecef_to_enu(args, span),
        ids::COSMIC_ENU_TO_ECEF => cosmic_bind::enu_to_ecef(args, span),
        ids::COSMIC_GEODETIC_DISTANCE => cosmic_bind::geodetic_distance(args, span),
        ids::COSMIC_BODY_PROFILE => cosmic_bind::body_profile(args, span),
        ids::COSMIC_SURFACE_GRAVITY => cosmic_bind::surface_gravity(args, span),
        ids::COSMIC_FLRW_DISTANCE => cosmic_bind::flrw_distance(args, span),
        ids::COSMIC_FLRW_REDSHIFT => cosmic_bind::flrw_redshift(args, span),
        ids::COSMIC_FLRW_HUBBLE_VELOCITY => cosmic_bind::flrw_hubble_velocity(args, span),
        ids::COSMIC_STARDATE_TO_GREGORIAN => cosmic_bind::stardate_to_gregorian(args, span),
        ids::COSMIC_WARP_VELOCITY => cosmic_bind::warp_velocity(args, span),
        ids::COSMIC_COCHRANE_UNITS => cosmic_bind::cochrane_units(args, span),
        ids::COSMIC_ATMOSPHERE_PRESSURE => cosmic_bind::atmosphere_pressure(args, span),
        ids::COSMIC_ATMOSPHERE_TEMPERATURE => cosmic_bind::atmosphere_temperature(args, span),
        ids::COSMIC_MAGNETOSPHERE_FIELD => cosmic_bind::magnetosphere_field(args, span),
        ids::COSMIC_SCALE_FACTOR => cosmic_bind::scale_factor(args, span),
        ids::COSMIC_COMPTON_WAVELENGTH => cosmic_bind::compton_wavelength(args, span),
        ids::COSMIC_DE_BROGLIE => cosmic_bind::de_broglie_wavelength(args, span),
        ids::COSMIC_USRI_PARSE => cosmic_bind::usri_parse(args, span),
        ids::COSMIC_WARP_FACTOR_C => cosmic_bind::warp_factor_c(args, span),
        ids::COSMIC_TYPICAL_LENGTH => cosmic_bind::typical_length(args, span),
        ids::COSMIC_OBSERVE_REDSHIFT => cosmic_bind::observe_redshift(args, span),
        ids::NLP_ANALYZE => nlp::analyze(args, span),
        ids::NLP_GAZETTEER_RUN => nlp::gazetteer_run(args, span),
        ids::NLP_GAZETTEER_BUILD => nlp::gazetteer_build(args, span),
        ids::NLP_TOKENIZE => nlp::tokenize(args, span),
        ids::NLP_SPLIT_SENTENCES => nlp::split_sentences(args, span),
        ids::NLP_FST_LOOKUP => nlp::fst_lookup(args, span),
        ids::NLP_COREF_RESOLVE => nlp::coref_resolve(args, span),
        ids::NLP_FRAME_EXTRACT => nlp::frame_extract(args, span),
        ids::NLP_RELATION_EXTRACT => nlp::relation_extract(args, span),
        ids::NLP_SUBSTRATE_EXTRACT => nlp::substrate_extract(args, span),
        ids::NLP_GRAPHRAG_QUERY => nlp::graphrag_query(args, span),
        ids::CONDITIONING_VALIDATE => inference::conditioning_validate(args, span),
        ids::CONDITIONING_COMPILE => inference::conditioning_compile(args, span),
        ids::CONDITIONING_INSPECT => inference::conditioning_inspect(args, span),
        ids::CONDITIONING_EVALUATE => inference::conditioning_evaluate(args, span),
        ids::CONDITIONING_ACTIVATE => inference::conditioning_activate(args, span),
        ids::CONDITIONING_ROLLBACK => inference::conditioning_rollback(args, span),
        ids::INFERENCE_EMBED => inference::embed(args, span),
        ids::INFERENCE_GROUNDING => inference::grounding(args, span),
        ids::INFERENCE_VERIFY_TURN => inference::verify_turn(args, span),
        ids::INFERENCE_DETECT_UNGROUNDED => inference::detect_ungrounded(args, span),
        ids::INFERENCE_LOAD_MODEL => inference::load_model(args, span),
        ids::INFERENCE_UNLOAD_MODEL => inference::unload_model(args, span),
        ids::INFERENCE_RUN_TRANSFORMER => inference::run_transformer(args, span),
        ids::INFERENCE_RUN_CLASSIFIER => inference::run_classifier(args, span),
        ids::INFERENCE_RUN_RERANKER => inference::run_reranker(args, span),
        ids::INFERENCE_VECTOR_SEARCH => inference::vector_search(args, span),
        ids::INFERENCE_CONSTRAINED_DECODE => inference::constrained_decode(args, span),
        ids::INFERENCE_RELU => inference::relu(args, span),
        ids::INFERENCE_SIGMOID => inference::sigmoid(args, span),
        ids::INFERENCE_GELU => inference::gelu(args, span),
        ids::INFERENCE_SOFTMAX => inference::softmax(args, span),
        ids::INFERENCE_RMS_NORM => inference::rms_norm(args, span),
        ids::FINANCE_CONVERT_CURRENCY => econ::convert_currency(args, span),
        ids::FINANCE_MULTISIG_CHECK => econ::multisig_check(args, span),
        ids::FINANCE_LEDGER_BALANCE => econ::ledger_balance(args, span),
        ids::ECON_CAPM_EXPECTED_RETURN => econ::econ::econ_capm_expected_return(args, span),
        ids::ECON_CAPM_BETA => econ::econ::econ_capm_beta(args, span),
        ids::ECON_GORDON_GROWTH => econ::econ::econ_gordon_growth(args, span),
        ids::ECON_MULTI_PERIOD_DDM => econ::econ::econ_multi_period_ddm(args, span),
        ids::ECON_CCAPM_EQUITY_PREMIUM => econ::econ::econ_ccapm_equity_premium(args, span),
        ids::ECON_CCAPM_SDF => econ::econ::econ_ccapm_sdf(args, span),
        ids::ECON_PROSPECT_VALUE => econ::econ::econ_prospect_value(args, span),
        ids::ECON_PROBABILITY_WEIGHT => econ::econ::econ_probability_weight(args, span),
        ids::ECON_HYPERBOLIC_DISCOUNT => econ::econ::econ_hyperbolic_discount(args, span),
        ids::ECON_ENDOWMENT_EFFECT => econ::econ::econ_endowment_effect(args, span),
        ids::ECON_BLACK_SCHOLES => econ::econ::econ_black_scholes(args, span),
        ids::ECON_PUT_CALL_PARITY => econ::econ::econ_put_call_parity(args, span),
        ids::ECON_BINOMIAL_OPTION => econ::econ::econ_binomial_option(args, span),
        ids::ECON_MIXED_NASH_2X2 => econ::econ::econ_mixed_nash_2x2(args, span),
        ids::ECON_COURNOT_DUOPOLY => econ::econ::econ_cournot_duopoly(args, span),
        ids::ECON_BERTRAND_DUOPOLY => econ::econ::econ_bertrand_duopoly(args, span),
        ids::ECON_STACKELBERG_DUOPOLY => econ::econ::econ_stackelberg_duopoly(args, span),
        ids::ECON_SOLOW_STEADY_STATE => econ::econ::econ_solow_steady_state(args, span),
        ids::ECON_RAMSEY_STEADY_STATE => econ::econ::econ_ramsey_steady_state(args, span),
        ids::ECON_OLG_STEADY_STATE => econ::econ::econ_olg_steady_state(args, span),
        ids::ECON_GINI => econ::econ::econ_gini(args, span),
        ids::ECON_ATKINSON => econ::econ::econ_atkinson(args, span),
        ids::ECON_HEADCOUNT_POVERTY => econ::econ::econ_headcount_poverty(args, span),
        ids::ECON_POVERTY_GAP => econ::econ::econ_poverty_gap(args, span),
        ids::ECON_UTILITARIAN_WELFARE => econ::econ::econ_utilitarian_welfare(args, span),
        ids::ECON_RAWLSIAN_WELFARE => econ::econ::econ_rawlsian_welfare(args, span),
        ids::ECON_NASH_WELFARE => econ::econ::econ_nash_welfare(args, span),
        ids::ECON_NPV => econ::econ::econ_npv(args, span),
        ids::ECON_MEAN_RETURN => econ::econ::econ_mean_return(args, span),
        ids::ECON_SAMPLE_VARIANCE => econ::econ::econ_sample_variance(args, span),
        ids::ECON_PORTFOLIO_MAX_DRAWDOWN => econ::econ::econ_portfolio_max_drawdown(args, span),
        ids::ECON_HISTORICAL_VAR => econ::econ::econ_historical_var(args, span),
        ids::ECON_HISTORICAL_CVAR => econ::econ::econ_historical_cvar(args, span),
        ids::ECON_PARAMETRIC_VAR => econ::econ::econ_parametric_var(args, span),
        ids::ECON_AUTOCORRELATION => econ::econ::econ_autocorrelation(args, span),
        ids::ECON_CROSS_CORRELATION => econ::econ::econ_cross_correlation(args, span),
        ids::ECON_INTERPOLATE_ZERO_RATE => econ::econ::econ_interpolate_zero_rate(args, span),
        ids::ECON_DISCOUNT_FACTOR => econ::econ::econ_discount_factor(args, span),
        ids::ECON_FORWARD_RATE => econ::econ::econ_forward_rate(args, span),
        ids::ECON_GRAVITY_FLOW => econ::econ::econ_gravity_flow(args, span),
        ids::ECON_MORANS_I => econ::econ::econ_morans_i(args, span),
        ids::ECON_TRANSFER_PAYMENT => econ::econ::econ_transfer_payment(args, span),
        ids::ECON_FISCAL_MULTIPLIER => econ::econ::econ_fiscal_multiplier(args, span),
        ids::ECON_LAFFER_CURVE => econ::econ::econ_laffer_curve(args, span),
        ids::ECON_CHECK_IR => econ::econ::econ_check_ir(args, span),
        ids::ECON_CHECK_BUDGET_BALANCE => econ::econ::econ_check_budget_balance(args, span),
        ids::ECON_VALIDATE_TRANSITION_MATRIX => {
            econ::econ::econ_validate_transition_matrix(args, span)
        }
        ids::ECON_TRANSITION_PROBABILITY => econ::econ::econ_transition_probability(args, span),
        ids::ECON_EXPECTED_HOLDING_TIME => econ::econ::econ_expected_holding_time(args, span),
        ids::ECON_LABOR_SUPPLY => econ::econ::econ_labor_supply(args, span),
        ids::ECON_EFFICIENCY_UNITS => econ::econ::econ_efficiency_units(args, span),
        ids::ECON_SOCIAL_COST_OF_CARBON => econ::econ::econ_social_cost_of_carbon(args, span),
        ids::ECON_OPTIMAL_POLLUTION => econ::econ::econ_optimal_pollution(args, span),
        ids::ECON_OPTIMAL_ABATEMENT => econ::econ::econ_optimal_abatement(args, span),
        ids::ECON_BELLMAN_UPDATE => econ::econ::econ_bellman_update(args, span),
        ids::ECON_MALFEASANCE_DELTA => econ::econ::econ_malfeasance_delta(args, span),
        ids::ECON_OLS => econ::econ::econ_ols(args, span),
        ids::ECON_AGGREGATE_WEALTH => econ::econ::econ_aggregate_wealth(args, span),
        ids::ECON_TOTAL_TRANSPORT_COST => econ::econ::econ_total_transport_cost(args, span),
        ids::ECON_LUCAS_ASSET_PRICE => econ::econ::econ_lucas_asset_price(args, span),
        ids::ECON_PRESENT_BIASED_UTILITY => econ::econ::econ_present_biased_utility(args, span),
        ids::ECON_REFERENCE_DEPENDENT_UTILITY => {
            econ::econ::econ_reference_dependent_utility(args, span)
        }
        ids::ECON_PURE_NASH_EQUILIBRIA => econ::econ::econ_pure_nash_equilibria(args, span),
        ids::ECON_REPEATED_GAME_PAYOFF => econ::econ::econ_repeated_game_payoff(args, span),
        ids::ECON_BERTRAND_WITH_DEMAND => econ::econ::econ_bertrand_with_demand(args, span),
        ids::ECON_RAMSEY_EULER_RESIDUAL => econ::econ::econ_ramsey_euler_residual(args, span),
        ids::ECON_NEW_KEYNESIAN_SOLVE => econ::econ::econ_new_keynesian_solve(args, span),
        ids::ECON_LORENZ_CURVE => econ::econ::econ_lorenz_curve(args, span),
        ids::ECON_DISTRIBUTIONAL_NPV => econ::econ::econ_distributional_npv(args, span),
        ids::ECON_PORTFOLIO_RETURNS => econ::econ::econ_portfolio_returns(args, span),
        ids::ECON_COVARIANCE_MATRIX => econ::econ::econ_covariance_matrix(args, span),
        ids::ECON_PORTFOLIO_VARIANCE => econ::econ::econ_portfolio_variance(args, span),
        ids::ECON_SIMPLE_RETURNS => econ::econ::econ_simple_returns(args, span),
        ids::ECON_LOG_RETURNS => econ::econ::econ_log_returns(args, span),
        ids::ECON_CUMULATIVE_WEALTH => econ::econ::econ_cumulative_wealth(args, span),
        ids::ECON_DRAWDOWN => econ::econ::econ_drawdown(args, span),
        ids::ECON_ROLLING_MEAN => econ::econ::econ_rolling_mean(args, span),
        ids::ECON_ROLLING_VARIANCE => econ::econ::econ_rolling_variance(args, span),
        ids::ECON_GBM_SIMULATE => econ::econ::econ_gbm_simulate(args, span),
        ids::ECON_STRESS_SCENARIO => econ::econ::econ_stress_scenario(args, span),
        ids::ECON_BLOCK_BOOTSTRAP => econ::econ::econ_block_bootstrap(args, span),
        ids::ECON_PAR_YIELD => econ::econ::econ_par_yield(args, span),
        ids::ECON_NEAREST_FACILITY => econ::econ::econ_nearest_facility(args, span),
        ids::ECON_PROGRESSIVE_TAX => econ::econ::econ_progressive_tax(args, span),
        ids::ECON_VCG_PAYMENT => econ::econ::econ_vcg_payment(args, span),
        ids::ECON_STRATEGY_PROOFNESS => econ::econ::econ_strategy_proofness(args, span),
        ids::ECON_STATIONARY_DISTRIBUTION => econ::econ::econ_stationary_distribution(args, span),
        ids::ECON_SIMULATE_CHAIN => econ::econ::econ_simulate_chain(args, span),
        ids::ECON_MEAN_FIRST_PASSAGE => econ::econ::econ_mean_first_passage(args, span),
        ids::ECON_HOUSEHOLD_PRODUCTION_CES => econ::econ::econ_household_production_ces(args, span),
        ids::ECON_POLLUTION_DAMAGE => econ::econ::econ_pollution_damage(args, span),
        ids::ECON_MARGINAL_DAMAGE => econ::econ::econ_marginal_damage(args, span),
        ids::ECON_ABATEMENT_NET_BENEFIT => econ::econ::econ_abatement_net_benefit(args, span),
        ids::ECON_WLS => econ::econ::econ_wls(args, span),
        ids::ECON_IV_2SLS => econ::econ::econ_iv_2sls(args, span),
        ids::ECON_LOGISTIC_MLE => econ::econ::econ_logistic_mle(args, span),
        ids::ECON_VALUE_ITERATION => econ::econ::econ_value_iteration(args, span),
        ids::ECON_NARRATIVE_DIVERGENCE => econ::econ::econ_narrative_divergence(args, span),
        ids::ECON_EIGENVECTOR_CENTRALITY => econ::econ::econ_eigenvector_centrality(args, span),
        ids::ECON_DEGREE_CENTRALITY => econ::econ::econ_degree_centrality(args, span),
        ids::ECON_INTERBANK_CLEARING => econ::econ::econ_interbank_clearing(args, span),
        ids::ECON_LEONTIEF_INVERSE => econ::econ::econ_leontief_inverse(args, span),
        ids::ECON_OUTPUT_MULTIPLIERS => econ::econ::econ_output_multipliers(args, span),
        ids::ECON_AGENT_BASED_AGGREGATE_WEALTH => {
            econ::econ::econ_agent_based_aggregate_wealth(args, span)
        }
        ids::ECON_VALIDATE_SCALAR_CONSTRAINT => {
            econ::econ::econ_validate_scalar_constraint(args, span)
        }
        ids::ECON_AGGREGATE_PAPER_FILLS => econ::econ::econ_aggregate_paper_fills(args, span),
        _ => return None,
    };
    Some(res)
}
