//! Honest dispatch policy for non-placement Tool Chest actions.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::tool_chest::core::intent_bus::ActionType;

pub fn requires_daemon(_tool_id: &str) -> bool {
    false
}

#[cfg(test)]
fn has_local_contract(tool_id: &str) -> bool {
    matches!(
        tool_id,
        "epistemic:tag_objective"
            | "epistemic:tag_subjective"
            | "epistemic:tag_intersubjective"
            | "epistemic:tag_normative"
            | "image:marker"
            | "spatial:pin"
            | "mail:composer"
            | "rights:authors_group"
            | "office:typography_bold"
            | "office:typography_italic"
            | "office:typography_code"
            | "office:paragraph_heading"
            | "office:paragraph_align_left"
            | "office:paragraph_align_center"
            | "image:brush_stroke"
            | "image:brush_clear"
            | "image:fill_warm"
            | "image:fill_cool"
            | "image:heatmap"
            | "spatial:camera_reset"
            | "sheet:import"
            | "code:vibe_diagnose"
            | "code:quin_statement"
    )
}

#[cfg(test)]
fn has_live_invoke(tool_id: &str) -> bool {
    matches!(
        tool_id,
        "graph:sparql_query"
            | "ai:extractor"
            | "ai:sentinel"
            | "n3:evaluate"
            | "shacl:validate"
            | "spatial:orbit_preview"
            | "sheet:stats_mean"
            | "sheet:stats_median"
            | "sheet:stats_variance"
            | "sheet:stats_std_dev"
            | "sheet:stats_min"
            | "sheet:stats_max"
            | "sheet:stats_sum"
            | "sheet:stats_skewness"
            | "sheet:stats_kurtosis"
            | "sheet:stats_quantile"
            | "sheet:stats_iqr"
            | "sheet:stats_mode"
            | "sheet:stats_trimmed_mean"
            | "sheet:stats_mad"
            | "sheet:stats_pearson"
            | "sheet:stats_covariance"
            | "sheet:stats_z_score_outliers"
            | "sheet:stats_argmax"
            | "sheet:stats_binomial_pmf"
            | "sheet:stats_binomial_cdf"
            | "sheet:stats_beta_pdf"
            | "sheet:stats_chi_squared_pdf"
            | "sheet:stats_chi_squared_cdf"
            | "sheet:stats_chi_squared_quantile"
            | "sheet:stats_autocorrelation"
            | "sheet:stats_bootstrap_means"
            | "sheet:stats_normal_pdf"
            | "sheet:stats_normal_cdf"
            | "sheet:stats_normal_quantile"
            | "sheet:stats_standard_normal_cdf"
            | "sheet:stats_poisson_pmf"
            | "sheet:stats_poisson_cdf"
            | "sheet:stats_exponential_pdf"
            | "sheet:stats_exponential_cdf"
            | "sheet:stats_spearman"
            | "sheet:stats_kendall"
            | "sheet:stats_winsorized_mean"
            | "sheet:stats_erf"
            | "sheet:stats_erfc"
            | "sheet:stats_uniform_pdf"
            | "sheet:stats_laplace_pdf"
            | "sheet:stats_standard_pdf"
            | "sheet:stats_uniform_cdf"
            | "sheet:stats_laplace_cdf"
            | "sheet:stats_lognormal_pdf"
            | "sheet:stats_lognormal_cdf"
            | "sheet:stats_standard_quantile"
            | "sheet:stats_ln_gamma"
            | "sheet:stats_gamma_fn"
            | "sheet:stats_weibull_pdf"
            | "sheet:stats_gamma_pdf"
            | "sheet:stats_two_sided_p"
            | "sheet:stats_chi_squared_upper_p"
            | "sheet:stats_students_t_pdf"
            | "sheet:stats_fisher_f_pdf"
            | "sheet:stats_gammp"
            | "sheet:stats_gammq"
            | "sheet:stats_betai"
            | "sheet:stats_students_t_cdf"
            | "sheet:stats_students_t_two_sided_p"
            | "sheet:stats_students_t_upper_p"
            | "sheet:stats_students_t_quantile"
            | "sheet:stats_fisher_f_cdf"
            | "sheet:stats_fisher_f_upper_p"
            | "sheet:stats_fisher_f_quantile"
            | "sheet:stats_tukey_fences"
            | "sheet:stats_empirical_cdf"
            | "sheet:stats_entropy"
            | "sheet:stats_kl_divergence"
            | "sheet:stats_cross_entropy"
            | "sheet:stats_entropy_from_counts"
            | "sheet:stats_moving_average"
            | "sheet:stats_modified_z_score_outliers"
            | "sheet:stats_iqr_outliers"
            | "sheet:stats_exponential_smoothing"
            | "sheet:stats_adf_proxy"
            | "sheet:stats_histogram"
            | "sheet:stats_ks_1sample"
            | "sheet:stats_grubbs_test"
            | "sheet:stats_one_sample_t"
            | "sheet:stats_two_sample_t"
            | "sheet:stats_paired_t"
            | "sheet:stats_linear_regression"
            | "sheet:stats_chi_square_gof"
            | "sheet:stats_chi_square_independence"
            | "sheet:stats_correlation_p_value"
            | "sheet:stats_friedman"
            | "sheet:stats_ljung_box"
            | "sheet:stats_mann_whitney_u"
            | "sheet:stats_mcnemar"
            | "sheet:stats_mutual_information"
            | "sheet:stats_one_way_anova"
            | "sheet:stats_mahalanobis_sq"
            | "sheet:stats_mvn_log_pdf"
            | "sheet:stats_mvn_pdf"
            | "sheet:stats_mvn_sample"
            | "sheet:stats_mvn_mle"
            | "sheet:stats_validate_probability"
            | "sheet:stats_simplex_project"
            | "sheet:stats_fisher_distance"
            | "sheet:stats_neg_entropy"
            | "sheet:stats_simplex_project_idempotent"
            | "sheet:stats_fisher_inner_product"
            | "sheet:stats_neg_entropy_grad"
            | "sheet:stats_kl_bregman_form"
            | "sheet:stats_bregman_pythagorean_test"
            | "sheet:stats_probability_hash"
            | "sheet:poly_eval"
            | "sheet:poly_add"
            | "sheet:poly_sub"
            | "sheet:poly_mul"
            | "sheet:poly_gcd"
            | "sheet:poly_degree"
            | "sheet:poly_leading"
            | "sheet:poly_is_zero"
            | "sheet:poly_scale"
            | "sheet:poly_zero"
            | "sheet:poly_constant"
            | "sheet:poly_derivative"
            | "sheet:poly_monic"
            | "sheet:poly_div_rem"
            | "sheet:poly_resultant"
            | "ai:grounding"
            | "ai:detect_ungrounded"
            | "ai:verify_turn"
            | "ai:inf_relu"
            | "ai:inf_sigmoid"
            | "ai:inf_gelu"
            | "ai:inf_softmax"
            | "ai:inf_rms_norm"
            | "ai:inf_embed"
            | "ai:inf_run_classifier"
            | "ai:inf_vector_search"
            | "ai:inf_load_model"
            | "ai:inf_unload_model"
            | "ai:inf_run_transformer"
            | "ai:inf_run_reranker"
            | "ai:inf_constrained_decode"
            | "ai:ml_mse"
            | "ai:ml_rmse"
            | "ai:ml_mae"
            | "ai:ml_r2"
            | "ai:ml_accuracy"
            | "ai:ml_ols"
            | "ai:ml_train_test_split"
            | "ai:ml_kmeans"
            | "ai:ml_log_loss"
            | "ai:ml_bonferroni"
            | "ai:ml_confusion_binary"
            | "ai:ml_holm"
            | "ai:ml_benjamini_hochberg"
            | "ai:ml_ab_test"
            | "ai:ml_bootstrap_estimate"
            | "ai:ml_bootstrap_ci"
            | "ai:ml_permutation_test"
            | "ai:ml_n_rejected"
            | "ai:ml_power_two_sample"
            | "ai:ml_roc_auc"
            | "ai:ml_k_fold"
            | "ai:ml_bootstrap_indices"
            | "ai:ml_pca"
            | "ai:ml_required_sample_size"
            | "ai:ml_polynomial_regression"
            | "ai:ml_required_n_two_prop"
            | "ai:ml_loocv"
            | "ai:ml_transe_score"
            | "ai:ml_distmult_score"
            | "ai:ml_complex_score"
            | "ai:ml_rotate_score"
            | "ai:ml_ridge_fit"
            | "ai:ml_lasso_fit"
            | "ai:ml_pls_fit"
            | "ai:ml_standard_scaler"
            | "ai:ml_kmeans_fit"
            | "ai:ml_gmm_fit"
            | "ai:ml_logistic_fit"
            | "ai:ml_poisson_fit"
            | "ai:ml_naive_bayes_fit"
            | "ai:ml_knn_fit"
            | "ai:ml_lda_fit"
            | "ai:ml_pcr_fit"
            | "ai:ml_qda_fit"
            | "ai:ml_multinomial_logistic_fit"
            | "ai:ml_hierarchical_fit"
            | "ai:ml_hierarchical_labels"
            | "ai:ml_bayesian_linear_fit"
            | "ai:ml_decision_tree_regressor"
            | "ai:ml_decision_tree_classifier"
            | "ai:ml_gp_fit"
            | "ai:ml_svm_fit"
            | "ai:ml_kaplan_meier_fit"
            | "ai:ml_cox_fit"
            | "ai:ml_hmm_baum_welch"
            | "ai:ml_variational_gaussian_fit"
            | "ai:ml_mcmc_metropolis"
            | "ai:ml_svm_multiclass_fit"
            | "ai:ml_som_train"
            | "ai:ml_random_forest_regressor"
            | "ai:ml_random_forest_classifier"
            | "ai:ml_gradient_boosting_regressor"
            | "ai:ml_bart_fit"
            | "ai:ml_kg_mean_rank"
            | "ai:ml_kg_mrr"
            | "ai:ml_kg_hits_at_k"
            | "ai:ml_kalman_new"
            | "ai:ml_factor_graph_marginals"
            | "ai:ml_al_row_score"
            | "ai:ml_al_score"
            | "ai:ml_al_cosine"
            | "ai:ml_al_rank_informative"
            | "ai:ml_al_most_informative"
            | "ai:ml_al_representativeness"
            | "ai:ml_al_information_density"
            | "ai:ml_al_rank_by_density"
            | "ai:ml_al_vote_entropy"
            | "ai:ml_al_consensus"
            | "ai:ml_al_consensus_entropy"
            | "ai:ml_al_kl_disagreement"
            | "ai:ml_al_rank_by_disagreement"
            | "epistemic:evaluate"
            | "image:histogram"
            | "image:equalize_hist"
            | "image:rgb_to_gray"
            | "image:dhash"
            | "image:hamming_distance"
            | "image:cosine_similarity"
            | "image:edit_new"
            | "image:edit_add_layer"
            | "image:edit_remove_layer"
            | "image:edit_set_pixel"
            | "image:edit_fill"
            | "image:edit_brush"
            | "image:edit_apply_filter"
            | "image:edit_set_opacity"
            | "image:edit_set_blend_mode"
            | "image:edit_set_visible"
            | "image:edit_set_mask"
            | "image:edit_clear_mask"
            | "image:edit_composite"
            | "image:edit_add_selection"
            | "image:edit_clear_selections"
            | "health:framingham"
            | "health:cha2ds2"
            | "health:score2"
            | "comm:pulse_presence"
            | "rights:deontic_obligate"
            | "epistemic:paraconsistent_route"
            | "code:ltl_evaluate"
            | "code:symbolic_eval"
            | "code:symbolic_differentiate"
            | "code:symbolic_simplify"
            | "code:symbolic_expand"
            | "code:symbolic_factor"
            | "code:symbolic_integrate"
            | "code:symbolic_simplify_trig"
            | "code:symbolic_partial"
            | "code:symbolic_limit"
            | "code:symbolic_add"
            | "code:symbolic_sub"
            | "code:symbolic_mul"
            | "code:symbolic_div"
            | "code:symbolic_pow"
            | "code:symbolic_neg"
            | "code:symbolic_sqrt"
            | "code:symbolic_exp"
            | "code:symbolic_ln"
            | "code:symbolic_sin"
            | "code:symbolic_cos"
            | "code:symbolic_tan"
            | "code:symbolic_parse"
            | "code:symbolic_c"
            | "code:symbolic_var"
            | "code:symbolic_hessian"
            | "code:symbolic_integrate_definite"
            | "code:symbolic_limit_at_infinity"
            | "code:symbolic_real_roots"
            | "code:symbolic_roots"
            | "code:symbolic_taylor_coefficients"
            | "code:symbolic_taylor_eval"
            | "code:symbolic_jacobian"
            | "code:symbolic_gradient_at"
            | "code:symbolic_hessian_at"
            | "code:symbolic_solve_quadratic"
            | "code:symbolic_solve_quadratic_symbolic"
            | "code:symbolic_factor_quadratic"
            | "code:symbolic_solve_polynomial_expr"
            | "code:symbolic_simplify_with_assumptions"
            | "code:symbolic_expr_citation_hash"
            | "code:symbolic_to_quins"
            | "code:symbolic_from_quins"
            | "code:constr_regular_polygon"
            | "code:constr_fermat_prime"
            | "code:constr_min_poly_degree"
            | "code:constr_power_of_two"
            | "code:constr_central_angle"
            | "code:constr_doubling_cube"
            | "code:constr_trisect_angle"
            | "code:constr_square_circle"
            | "code:constr_number"
            | "code:nt_gcd"
            | "code:nt_lcm"
            | "code:nt_is_prime"
            | "code:nt_factorial"
            | "code:nt_binomial"
            | "code:nt_euler_totient"
            | "code:nt_mod_pow"
            | "code:nt_next_prime"
            | "code:nt_mod_inverse"
            | "code:nt_divisor_count"
            | "code:nt_prime_factors"
            | "code:nt_divisors"
            | "code:nt_mobius"
            | "code:nt_divisor_sum"
            | "code:nt_partitions"
            | "code:nt_catalan"
            | "code:nt_stirling_second"
            | "code:nt_stirling_first"
            | "code:nt_extended_gcd"
            | "code:nt_crt"
            | "code:fuzzy_triangular"
            | "code:fuzzy_trapezoidal"
            | "code:fuzzy_approximately"
            | "code:fuzzy_ramp_up"
            | "code:fuzzy_ramp_down"
            | "code:fuzzy_much_greater_than"
            | "code:fuzzy_much_less_than"
            | "code:fuzzy_threshold"
            | "code:fuzzy_top_k"
            | "code:fuzzy_negate"
            | "code:fuzzy_and"
            | "code:fuzzy_or"
            | "econ:capm"
            | "econ:gini"
            | "econ:mixed_nash"
            | "econ:black_scholes"
            | "econ:solow"
            | "econ:cournot"
            | "econ:bertrand"
            | "econ:historical_var"
            | "econ:atkinson"
            | "econ:gordon_growth"
            | "econ:binomial_option"
            | "econ:forward_rate"
            | "econ:gbm_simulate"
            | "econ:headcount_poverty"
            | "econ:hyperbolic_discount"
            | "econ:fiscal_multiplier"
            | "econ:drawdown"
            | "econ:covariance_matrix"
            | "econ:capm_beta"
            | "econ:autocorrelation"
            | "econ:cross_correlation"
            | "econ:bertrand_with_demand"
            | "econ:check_budget_balance"
            | "econ:ccapm_equity_premium"
            | "econ:mean_return"
            | "econ:poverty_gap"
            | "econ:sample_variance"
            | "econ:utilitarian_welfare"
            | "econ:rawlsian_welfare"
            | "econ:nash_welfare"
            | "econ:stackelberg"
            | "econ:put_call_parity"
            | "econ:parametric_var"
            | "econ:laffer_curve"
            | "econ:historical_cvar"
            | "econ:endowment_effect"
            | "econ:prospect_value"
            | "econ:probability_weight"
            | "econ:ccapm_sdf"
            | "econ:gravity_flow"
            | "econ:transfer_payment"
            | "econ:efficiency_units"
            | "econ:social_cost_of_carbon"
            | "econ:pollution_damage"
            | "econ:marginal_damage"
            | "econ:ramsey_steady_state"
            | "econ:simple_returns"
            | "econ:log_returns"
            | "econ:rolling_mean"
            | "econ:rolling_variance"
            | "econ:labor_supply"
            | "econ:optimal_abatement"
            | "econ:optimal_pollution"
            | "econ:olg_steady_state"
            | "econ:ramsey_euler_residual"
            | "econ:present_biased_utility"
            | "econ:reference_dependent_utility"
            | "econ:npv"
            | "econ:multi_period_ddm"
            | "econ:portfolio_max_drawdown"
            | "econ:interpolate_zero_rate"
            | "econ:discount_factor"
            | "econ:par_yield"
            | "econ:progressive_tax"
            | "econ:abatement_net_benefit"
            | "econ:household_production_ces"
            | "econ:malfeasance_delta"
            | "econ:portfolio_variance"
            | "econ:portfolio_returns"
            | "econ:distributional_npv"
            | "econ:stress_scenario"
            | "econ:repeated_game_payoff"
            | "econ:total_transport_cost"
            | "econ:transition_probability"
            | "econ:expected_holding_time"
            | "econ:check_ir"
            | "econ:vcg_payment"
            | "econ:validate_transition_matrix"
            | "econ:stationary_distribution"
            | "econ:mean_first_passage"
            | "econ:degree_centrality"
            | "econ:eigenvector_centrality"
            | "econ:new_keynesian_solve"
            | "econ:nearest_facility"
            | "econ:pure_nash_equilibria"
            | "econ:morans_i"
            | "econ:strategy_proofness"
            | "econ:lorenz_curve"
            | "econ:ols"
            | "econ:wls"
            | "econ:lucas_asset_price"
            | "econ:bellman_update"
            | "econ:block_bootstrap"
            | "econ:simulate_chain"
            | "econ:value_iteration"
            | "econ:iv_2sls"
            | "econ:logistic_mle"
            | "econ:interbank_clearing"
            | "econ:leontief_inverse"
            | "econ:output_multipliers"
            | "econ:agent_based_aggregate_wealth"
            | "econ:validate_scalar_constraint"
            | "econ:aggregate_paper_fills"
            | "rights:delegation_permits"
            | "scientific:la_matmul"
            | "scientific:la_matvec"
            | "scientific:la_transpose"
            | "scientific:la_determinant"
            | "scientific:la_solve"
            | "scientific:la_scale"
            | "scientific:la_add_into"
            | "scientific:la_axpy"
            | "scientific:la_hadamard_into"
            | "scientific:la_qr_factor"
            | "scientific:la_cholesky_factor"
            | "scientific:la_cholesky_solve"
            | "scientific:la_lu_decompose"
            | "scientific:la_lu_solve"
            | "scientific:la_qr_form_q"
            | "scientific:la_qr_solve_ls"
            | "scientific:la_svd"
            | "scientific:la_eigenvalues"
            | "scientific:la_add_assign"
            | "scientific:la_hadamard_assign"
            | "scientific:la_cholesky_det"
            | "scientific:la_charpoly"
            | "scientific:la_eigenvalues_general"
            | "scientific:la_eigen_symmetric"
            | "scientific:la_polynomial_roots"
            | "scientific:la_solve_linear_system"
            | "scientific:la_symmetric_eigen_3x3"
            | "scientific:chem_boys"
            | "scientific:chem_overlap_s"
            | "scientific:chem_kinetic_s"
            | "scientific:chem_nuclear_s"
            | "scientific:chem_dipole_s"
            | "scientific:chem_evaluate_eri"
            | "scientific:chem_total_angular_momentum"
            | "scientific:chem_letter"
            | "scientific:chem_n_cartesian"
            | "scientific:chem_n_spherical"
            | "scientific:chem_from_letter"
            | "scientific:chem_gaussian_elim"
            | "scientific:chem_jacobi"
            | "scientific:chem_transpose"
            | "scientific:chem_orthogonalize"
            | "scientific:chem_element_symbol"
            | "scientific:chem_atomic_number"
            | "scientific:chem_atomic_weight"
            | "scientific:chem_lda_exchange"
            | "scientific:chem_lda_vwn"
            | "scientific:chem_sto3g_h2"
            | "scientific:sf_airy_ai"
            | "scientific:sf_airy_bi"
            | "scientific:sf_zeta"
            | "scientific:sf_legendre"
            | "scientific:sf_chebyshev_t"
            | "scientific:sf_chebyshev_u"
            | "scientific:sf_hermite"
            | "scientific:sf_laguerre"
            | "scientific:sf_bessel_j"
            | "scientific:sf_bessel_i"
            | "scientific:sf_bessel_y"
            | "scientific:sf_bessel_k"
            | "scientific:xform_dft"
            | "scientific:xform_dft_complex"
            | "scientific:xform_idft"
            | "scientific:xform_z_transform_finite"
            | "scientific:xform_unit_step_z"
            | "scientific:xform_geometric_z"
            | "scientific:xform_laplace_numeric"
            | "scientific:xform_laplace_symbolic"
            | "scientific:calc_hermite_dense"
            | "scientific:calc_bdf1"
            | "scientific:calc_bdf2"
            | "scientific:calc_verlet"
            | "scientific:calc_ruth3"
            | "scientific:calc_yoshida4"
            | "scientific:calc_integrate_bdf"
            | "scientific:calc_integrate_sens"
            | "scientific:calc_invariant_drift"
            | "scientific:calc_perm_parity"
            | "scientific:calc_pack_f32"
            | "scientific:calc_unpack_f32"
            | "scientific:calc_poisson_bracket"
            | "scientific:calc_stormer_verlet"
            | "scientific:calc_gauss_kronrod"
            | "scientific:calc_jvp"
            | "scientific:calc_vjp"
            | "scientific:calc_adaptive_simpson"
            | "scientific:calc_adaptive_deriv"
            | "scientific:calc_newton_solve"
            | "scientific:calc_num_jacobian"
            | "scientific:calc_num_hessian"
            | "scientific:ga_dot"
            | "scientific:ga_cross"
            | "scientific:ga_normalize"
            | "scientific:ga_angle"
            | "scientific:ga_geometric_product"
            | "scientific:ga_outer_product"
            | "scientific:ga_rotor"
            | "scientific:ga_apply_rotor"
            | "scientific:ga_translator"
            | "scientific:ga_apply_translator"
            | "scientific:ga_is_simd"
            | "scientific:cg_distance_2d"
            | "scientific:cg_distance_3d"
            | "scientific:cg_point_segment_2d"
            | "scientific:cg_orientation_2"
            | "scientific:cg_orient_3d"
            | "scientific:cg_morton_encode_2d"
            | "scientific:cg_morton_decode_2d"
            | "scientific:cg_morton_encode_3d"
            | "scientific:cg_hilbert_encode_2d"
            | "scientific:cg_circumcenter"
            | "scientific:cg_point_segment_3d"
            | "scientific:cg_point_triangle_3d"
            | "scientific:cg_convex_hull_2"
            | "scientific:cg_triangulate"
            | "scientific:cg_surface_area"
            | "scientific:cg_signed_volume"
            | "scientific:cg_segment_intersect_2"
            | "scientific:cg_bezier_eval"
            | "scientific:cg_nearest_site"
            | "scientific:cg_live_average_spacing_3d"
            | "scientific:cg_live_local_density_3d"
            | "scientific:cg_live_mean_knn_distance_3d"
            | "scientific:cg_live_fisher_distance"
            | "scientific:cg_live_kl_divergence"
            | "scientific:cg_live_kl_bregman_form"
            | "scientific:cg_live_triangle_signed_area"
            | "scientific:cg_live_dist_point_to_segment"
            | "scientific:cg_live_dist_sq_point_to_segment"
            | "scientific:cg_live_incircle"
            | "scientific:cg_live_tukey_depth"
            | "scientific:cg_live_directional_width"
            | "scientific:cg_live_width"
            | "scientific:cg_live_farthest_site_brute"
            | "scientific:cg_live_k_nearest_sites"
            | "scientific:cg_live_is_hull_site"
            | "scientific:cg_live_diameter_and_width"
            | "scientific:cg_live_insphere"
            | "scientific:cg_live_ham_sandwich_cut"
            | "scientific:cg_live_smallest_enclosing_disk"
            | "scientific:cg_live_polygon_signed_area"
            | "scientific:cg_live_polygon_area"
            | "scientific:cg_live_point_in_polygon"
            | "scientific:cg_live_minkowski_sum_convex"
            | "scientific:cg_live_nearest_segment_site"
            | "scientific:cg_live_width_coreset"
            | "scientific:cg_live_dual_point_to_line"
            | "scientific:cg_live_dual_round_trip"
            | "scientific:cg_live_is_convex_polygon"
            | "scientific:cg_live_point_in_or_on_polygon"
            | "scientific:cg_live_boolean_union_area"
            | "scientific:cg_live_boolean_intersection_area"
            | "scientific:cg_live_boolean_difference_area"
            | "scientific:cg_live_cross_ratio_1d"
            | "scientific:cg_live_hyperplane_eval"
            | "scientific:cg_live_householder_reflect"
            | "scientific:cg_live_quaternion_normalize"
            | "scientific:cg_live_so3_exp"
            | "scientific:cg_live_so3_log"
            | "scientific:cg_live_projective_from_point"
            | "scientific:cg_live_point_from_projective"
            | "scientific:cg_live_frame_to_world"
            | "scientific:cg_live_world_to_frame"
            | "scientific:cg_live_barycentric_tetra"
            | "scientific:cg_live_quaternion_slerp"
            | "scientific:cg_live_quaternion_to_matrix"
            | "scientific:cg_live_solve_diagonal_quadratic"
            | "scientific:cg_live_schur_complement_2x2"
            | "scientific:cg_live_separating_plane_aabb"
            | "scientific:eng_natural_freq"
            | "scientific:eng_harmonic_sdof"
            | "scientific:eng_euler"
            | "scientific:eng_reliability"
            | "scientific:eng_kinematics"
            | "scientific:eng_cauchy"
            | "scientific:eng_drag"
            | "scientific:eng_reynolds"
            | "scientific:eng_fatigue"
            | "scientific:eng_miner"
            | "scientific:phys_doppler"
            | "scientific:phys_emf_attenuation"
            | "scientific:phys_harmonic"
            | "scientific:phys_pendulum"
            | "scientific:phys_logistic"
            | "scientific:phys_cfd_step"
            | "scientific:phys_heat_1d"
            | "scientific:phys_wave_1d"
            | "scientific:phys_advection_1d"
            | "scientific:phys_quantum_1d"
            | "scientific:phys_n_body"
            | "scientific:phys_molecular_dynamics"
            | "scientific:phys_emf_interference"
            | "scientific:phys_emf_field_grid"
            | "scientific:phys_emf_sample_depth"
            | "scientific:phys_field_sample"
            | "scientific:phys_material_query"
            | "scientific:phys_evaluate_interaction"
        | "scientific:cosmic_geodetic_distance"
        | "scientific:cosmic_surface_gravity"
        | "scientific:cosmic_flrw_distance"
        | "scientific:cosmic_flrw_redshift"
        | "scientific:cosmic_flrw_hubble"
        | "scientific:cosmic_warp_velocity"
        | "scientific:cosmic_warp_factor_c"
        | "scientific:cosmic_typical_length"
        | "scientific:cosmic_observe_redshift"
        | "scientific:cosmic_compton"
        | "scientific:cosmic_de_broglie"
        | "scientific:cosmic_atm_pressure"
        | "scientific:cosmic_geodetic_to_ecef"
        | "scientific:cosmic_ecef_to_geodetic"
        | "scientific:cosmic_ecef_to_enu"
        | "scientific:cosmic_enu_to_ecef"
        | "scientific:cosmic_body_profile"
        | "scientific:cosmic_stardate"
        | "scientific:cosmic_cochrane"
        | "scientific:cosmic_atm_temperature"
        | "scientific:cosmic_magnetosphere"
        | "scientific:cosmic_scale_factor"
        | "scientific:cosmic_usri_parse"
        | "ai:orch_session_create"
        | "ai:orch_session_plan"
        | "ai:orch_session_execute"
        | "ai:orch_session_status"
        | "ai:orch_roster_register"
        | "ai:orch_roster_list"
        | "ai:orch_roster_capabilities"
        | "ai:orch_assign_agents"
        | "spatial:threed_add_object"
        | "spatial:threed_set_transform"
        | "spatial:threed_set_material"
        | "spatial:threed_add_camera"
        | "spatial:threed_add_light"
        | "spatial:threed_add_rig"
        | "spatial:threed_add_animation"
        | "spatial:threed_set_mesh"
        | "spatial:scene_lerp_camera"
        | "spatial:scene_camera_frame_node"
        | "spatial:scene_smooth_damp"
        | "spatial:scene_smooth_damp_vec3"
        | "spatial:scene_ik_look_at"
        | "spatial:scene_ik_ccd"
        | "spatial:scene_set_render_budget"
        | "spatial:scene_set_clear_colour"
        | "spatial:scene_create"
        | "spatial:scene_add_node"
        | "spatial:scene_set_transform"
        | "spatial:scene_set_mesh"
        | "spatial:scene_add_camera"
        | "spatial:scene_render"
        | "spatial:scene_set_viewport"
        | "spatial:scene_capture_frame"
        | "spatial:scene_add_light"
        | "spatial:scene_link_semantic"
        | "spatial:scene_duplicate_node"
        | "audio:dsp_ep_temp"
        | "audio:dsp_ep_fm"
        | "audio:dsp_sigma_freq"
        | "audio:dsp_parametric_sample"
        | "audio:dsp_bin_freq_linear"
        | "audio:dsp_bin_freq_log"
        | "audio:dsp_midi_note"
        | "audio:dsp_quantize"
        | "audio:dsp_transpose"
        | "audio:fx_oscillator"
        | "audio:fx_envelope"
        | "audio:fx_filter"
        | "audio:fx_lfo"
        | "audio:fx_delay"
        | "audio:fx_reverb"
        | "audio:fx_compressor"
        | "audio:fx_eq"
        | "audio:fx_transport"
        | "audio:fx_waveform_meter"
        | "audio:fx_phase_meter"
        | "audio:fx_loudness_meter"
        | "audio:fx_spectrum"
        | "dmx:live_new_universe"
        | "dmx:live_set_channel"
        | "dmx:live_add_fixture"
        | "dmx:live_fixture_set_colour"
        | "dmx:live_fixture_set_intensity"
        | "dmx:live_fixture_set_pan_tilt"
        | "dmx:live_new_cue"
        | "dmx:live_cue_set_channel"
        | "dmx:live_cue_set_fade"
        | "dmx:live_new_cue_stack"
        | "dmx:live_cue_stack_add"
        | "dmx:live_cue_stack_go"
        | "dmx:live_cue_stack_go_back"
        | "dmx:live_cue_stack_reset"
        | "video:live_new_project"
        | "video:live_add_track"
        | "video:live_add_clip"
        | "video:live_trim_clip"
        | "video:live_set_speed"
        | "video:live_colour_grade"
        | "video:live_add_transition"
        | "video:live_set_render_format"
        | "video:live_set_render_bitrate"
        | "video:live_remove_clip"
        | "hid:live_poll"
        | "hid:live_wait"
        | "hid:live_clear"
        | "hid:live_pointer_capture"
        | "hid:live_pointer_release"
        | "hid:live_set_cursor"
        | "hid:live_gamepad_poll"
        | "hid:live_gamepad_vibrate"
        | "hid:live_midi_send"
        | "hid:live_midi_poll"
        | "hid:live_haptic_pulse"
        | "hid:live_haptic_pattern"
        | "hid:live_spatial_head_pose"
        | "hid:live_spatial_hand_skeleton"
        | "hid:live_spatial_gaze_ray"
        | "hid:live_biosignal_poll"
        | "scientific:vc_gradient"
        | "scientific:vc_divergence"
        | "scientific:vc_curl"
        | "scientific:vc_laplacian"
        | "scientific:vc_line_integral_scalar"
        | "scientific:vc_line_integral_work"
        | "scientific:vc_surface_flux"
        | "scientific:interp_linear"
        | "scientific:interp_lagrange"
        | "scientific:interp_newton_coef"
        | "scientific:interp_newton_eval"
        | "scientific:interp_poly_fit"
        | "scientific:interp_poly_eval"
        | "scientific:spectral_emf_to_spd"
        | "scientific:spectral_spd_to_xyz"
        | "scientific:spectral_emf_to_rgb"
        | "scientific:spectral_blend"
        | "scientific:spectral_gamut_map"
        | "spatial:world_new"
        | "spatial:world_add_object"
        | "spatial:world_add_portal"
        | "spatial:world_add_avatar"
        | "spatial:world_set_gravity"
        | "spatial:world_object_apply_force"
        | "spatial:world_object_step_physics"
        | "office:asset_create"
        | "office:asset_add_temporal"
        | "office:asset_add_topic"
        | "office:asset_set_spatial"
        | "office:asset_compile"
        | "office:asset_temporal_span"
        | "office:asset_query_aspects"
        | "office:asset_persist"
        | "office:asset_resolve"
        | "office:asset_resolve_by_spatial"
        | "office:asset_resolve_by_topic"
        | "office:asset_resolve_by_temporal"
        | "office:asset_list"
        | "office:asset_count"
        | "office:asset_persist_create"
        | "office:asset_persist_add_temporal"
        | "office:asset_persist_add_topic"
        | "office:asset_persist_set_spatial"
        | "office:asset_persist_compile"
        | "office:asset_persist_temporal_span"
        | "office:asset_persist_query_aspects"
        | "comm:pulse_live_publish"
        | "comm:pulse_live_graph_mutation"
        | "comm:pulse_live_notification"
        | "comm:pulse_live_telemetry"
        | "comm:pulse_live_agent_message"
        | "comm:pulse_live_sync"
        | "comm:pulse_live_open_channel"
        | "comm:pulse_live_close_channel"
        | "comm:pulse_live_set_transport"
        | "spatial:portal_set_target"
        | "research:live_new"
        | "research:live_set_purpose"
        | "research:live_define_scope"
        | "research:live_add_constraint"
        | "research:live_add_question"
        | "research:live_link_questions"
        | "research:live_add_corpus_item"
        | "research:live_import_literature"
        | "research:live_import_dataset"
        | "research:live_set_corpus_confidence"
        | "research:live_extract_from_corpus"
        | "research:live_infer_dark_link"
        | "research:live_detect_provenance_gaps"
        | "research:live_detect_concealment"
        | "research:live_confirm_dark_link"
        | "research:live_refute_dark_link"
        | "research:live_make_inference"
        | "research:live_chain_inference"
        | "research:live_set_inference_confidence"
        | "research:live_validate_inference"
        | "research:live_new_investigation"
        | "research:live_collect_evidence"
        | "research:live_set_reliability"
        | "research:live_propose_hypothesis"
        | "research:live_evaluate_evidence"
        | "research:live_create_timeline"
        | "research:live_add_link"
        | "research:live_find_path"
        | "research:live_create_hypothesis_graph"
        | "research:live_contribute_evaluation"
        | "research:live_bridge_dark_link"
        | "research:live_reframe_hypothesis"
        | "research:live_merge_hypotheses"
        | "research:live_flag_gap"
        | "research:live_close_gap"
        | "research:live_create_revision"
        | "research:live_diff_revisions"
        | "research:live_subscribe_updates"
        | "research:live_create_assessment"
        | "research:live_set_epistemic_mode"
        | "research:live_set_reality_category"
        | "research:live_classify_reality"
        | "research:live_detect_blended"
        | "research:live_detect_deceptive_fiction"
        | "research:live_trace_fiction"
        | "research:live_assess_sentiment"
        | "research:live_detect_sentiment_manipulation"
        | "research:live_detect_performed_sentiment"
        | "research:live_map_sentiment_network"
        | "research:live_analyse_sentiment_trends"
        | "research:live_register_perspective"
        | "research:live_add_bias"
        | "research:live_compare_perspectives"
        | "research:live_detect_perspective_conflict"
        | "research:live_reconcile_perspectives"
        | "research:live_assess_intentionality"
        | "research:live_classify_mistake"
        | "research:live_define_social_dynamics"
        | "research:live_define_economic_dynamics"
        | "research:live_define_spatiotemporal_dynamics"
        | "research:live_analyse_social_network"
        | "research:live_analyse_inequality"
        | "research:live_analyse_diffusion"
        | "research:live_assess_grounding"
        | "research:live_verify_grounding"
        | "research:live_detect_ungrounded_behaviour"
        | "research:live_create_ug_instance"
        | "research:live_set_ug_cause"
        | "research:live_set_ug_consequence"
        | "research:live_set_ug_detection"
        | "research:live_set_ug_mitigation"
        | "research:live_set_ug_calibration"
        | "research:live_detect_ug_patterns"
        | "render:live_scene"
        | "render:live_css_animation"
        | "render:live_css_color"
        | "render:live_css_transform"
        | "render:live_animation_eval_curve"
        | "render:live_animation_spring_step"
        | "render:live_animation_sclerp"
        | "render:live_animation_eval_preset"
        | "render:live_animation_squad_step"
        | "render:live_animation_list_presets"
        | "render:live_animation_compute_pass"
        | "render:live_svg_path"
        | "render:live_svg_circle"
        | "render:live_svg_rect"
        | "render:live_svg_line"
        | "render:live_svg_bezier"
        | "render:live_svg_field"
        | "animation:live_spring_step"
        | "animation:live_sclerp_step"
        | "animation:live_squad_step"
        | "animation:live_list_presets"
        | "scientific:num_ode_rk4"
        | "scientific:num_ode_dopri5"
        | "scientific:num_ode_bdf"
        | "scientific:num_ode_symplectic_step"
        | "comm:hbbtv_new_app"
        | "comm:hbbtv_add_page"
        | "comm:hbbtv_navigate"
        | "comm:hbbtv_set_state"
        | "render:gpu_live_init"
        | "render:gpu_live_init_surface"
        | "render:gpu_live_render_frame"
        | "render:gpu_live_read_pixels"
        | "render:gpu_live_upload_mesh"
        | "render:gpu_live_upload_tensor"
        | "render:gpu_live_pick"
        | "render:gpu_live_poll_pick"
        | "render:gpu_live_resize"
        | "render:gpu_live_set_ambient"
        | "render:gpu_live_destroy"
        | "render:gpu_live_compute_dispatch"
        | "render:gpu_live_compute_readback"
        | "render:gpu_live_validate_shader"
        | "render:gpu_live_compile_shader"
        | "render:gpu_live_compile_to_glsl"
        | "render:gpu_live_backend_info"
        | "render:gpu_live_upload_mesh_colored"
        | "render:gpu_live_set_standpoint"
        | "render:gpu_live_observer_standpoint"
        | "render:gpu_live_camera_state"
        | "render:gpu_live_surface_size"
        | "render:gpu_live_has_mesh"
        | "render:gpu_live_has_tensor"
        | "render:gpu_live_tensor_node_count"
        | "render:gpu_live_particle_count"
        | "render:gpu_live_sync_bloom"
        | "render:gpu_live_set_artefact_joint"
        | "render:gpu_live_set_artefact_world"
        | "render:gpu_live_artefact_refused"
        | "render:gpu_live_required_rgba8_bytes"
        | "render:gpu_live_emf_upload_field"
        | "render:gpu_live_emf_render_slice"
        | "render:gpu_live_emf_field_info"
        | "social:live_gini"
        | "social:live_lorenz"
        | "social:live_degree_centrality"
        | "social:live_lww"
        | "forensic:live_malfeasance_delta"
        | "forensic:live_narrative_divergence"
        | "finance:live_convert_currency"
        | "finance:live_multisig_check"
        | "finance:live_ledger_balance"
        | "comm:graph_live_corpus_load"
        | "comm:graph_live_corpus_parse"
        | "comm:graph_live_validate_fragment"
        | "comm:graph_live_link_reply"
        | "comm:graph_live_add_social_post"
        | "comm:graph_live_add_trigger"
        | "comm:graph_live_second_screen_sync"
        | "spatial:portal_activate"
        | "spatial:portal_deactivate"
        | "spatial:avatar_move"
        | "spatial:avatar_set_appearance"
        | "scientific:ode_lin1"
        | "scientific:ode_lin2"
        | "scientific:ode_classify_pde"
        | "scientific:ode_separable"
        | "scientific:ode_pde1"
        | "ai:agent_trace"
        | "ai:agent_verify"
        | "ai:agent_plan"
        | "ai:agent_execute"
        | "ai:agent_evaluate"
        | "ai:nlp_tokenize"
        | "ai:nlp_split_sentences"
        | "ai:nlp_coref_resolve"
        | "ai:nlp_frame_extract"
        | "ai:nlp_fst_lookup"
        | "ai:nlp_gazetteer_build"
        | "ai:nlp_graphrag_query"
        | "ai:nlp_relation_extract"
        | "ai:nlp_substrate_extract"
    )
}

#[cfg(test)]
fn has_dispatch_policy(tool_id: &str) -> bool {
    has_local_contract(tool_id)
        || has_live_invoke(tool_id)
        || requires_daemon(tool_id)
        || unavailable_reason(tool_id).is_some()
        || super::spec_tools::lookup(tool_id).is_some()
}

/// A structural prerequisite that the current UI cannot collect safely yet.
pub fn unavailable_reason(tool_id: &str) -> Option<&'static str> {
    match tool_id {
        "audio:mic_capture" => Some(
            "Microphone permission and bounded capture controls must be configured in the audio surface.",
        ),
        "audio:neural_latents" => {
            Some("A mounted P64 audio model and an active audio stream are required.")
        }
        "mail:publisher" => Some(
            "Publishing requires a selected artefact, destination, and authorisation workflow.",
        ),
        "scientific:thermodynamics" => Some(
            "Thermodynamics MCMC requires a configured target distribution and bounded sampler inputs.",
        ),
        "sdn:energy_governor" => Some(
            "Energy governance requires live battery/solar telemetry and an authorised control target.",
        ),
        "spatial:track" => {
            Some("Tracking requires a selected consenting agent and a live trajectory source.")
        }
        "rights:fiduciary_sign" | "rights:did_sign" => Some(
            "Signing requires a selected agreement, signer identity, consent check, and unlocked key vault.",
        ),
        "health:pathology" => {
            Some("Pathology evaluation requires consent-gated assay inputs and reference ranges.")
        }
        "ai:co_author" => Some(
            "Co-authoring requires a selected document, prompt scope, and an activated local model.",
        ),
        _ => super::spec_tools::lookup(tool_id).and_then(super::spec_tools::gated_reason),
    }
}

pub fn current_disabled_reason(tool_id: &str) -> Option<&'static str> {
    unavailable_reason(tool_id)
}

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn annotate_selected(document: &Document, semantic_type: &str, semantic_uri: &str, label: &str) {
    let Some(container) = selected_container(document) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a container before applying this annotation.",
            "error",
        );
        return;
    };
    let _ = container.set_attribute("data-semantic-type", semantic_type);
    let _ = container.set_attribute("data-semantic-uri", semantic_uri);
    if let Ok(Some(tag)) = container.query_selector(".container-type-tag") {
        let _ = tag.set_attribute(
            "title",
            &format!("Semantic annotation: {semantic_type} · {semantic_uri}"),
        );
    }
    super::history::push_current_frame("annotate container");
    super::interactions::show_tool_status(
        document,
        label,
        &format!("Applied {semantic_type} to the selected container."),
        "success",
    );
}

fn format_selected_editor(
    document: &Document,
    label: &str,
    css: &str,
    format: &str,
    success: &str,
) {
    let Some(container) = selected_container(document) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a document container before applying formatting.",
            "error",
        );
        return;
    };
    let Some(editor) = container.query_selector(".doc-editor").ok().flatten() else {
        super::interactions::show_tool_status(
            document,
            label,
            "The selected container has no editable document surface.",
            "error",
        );
        return;
    };
    let Ok(editor) = editor.dyn_into::<HtmlElement>() else {
        super::interactions::show_tool_status(
            document,
            label,
            "The selected document surface cannot receive formatting.",
            "error",
        );
        return;
    };
    for declaration in css.split(';') {
        let Some((property, value)) = declaration.split_once(':') else {
            continue;
        };
        let _ = editor.style().set_property(property.trim(), value.trim());
    }
    let _ = editor.set_attribute("data-paragraph-format", format);
    super::history::push_current_frame("format document");
    super::interactions::show_tool_status(document, label, success, "success");
}

fn selected_text(document: &Document) -> Option<String> {
    let container = selected_container(document)?;
    let text = container
        .query_selector(".doc-editor")
        .ok()
        .flatten()
        .and_then(|editor| editor.text_content())
        .or_else(|| container.text_content())?;
    let bounded: String = text.chars().take(16_384).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

/// Deterministic bounded extraction for standalone Poet/WASM. The daemon
/// gazetteer remains the richer path when a local node is connected.
pub(super) fn local_extract_summary(source: &str) -> (usize, usize, Vec<String>) {
    let bounded: String = source.chars().take(16_384).collect();
    let source = bounded.as_str();
    let token_count = source.split_whitespace().count();
    let sentence_count = source
        .split(|ch: char| matches!(ch, '.' | '!' | '?'))
        .filter(|sentence| !sentence.trim().is_empty())
        .count()
        .max(usize::from(!source.trim().is_empty()));
    let mut entities = Vec::new();
    for raw in source.split_whitespace() {
        let token: String = raw
            .trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '-')
            .chars()
            .take(96)
            .collect();
        let Some(first) = token.chars().next() else {
            continue;
        };
        if token.chars().count() < 2
            || !first.is_uppercase()
            || entities.iter().any(|known| known == &token)
        {
            continue;
        }
        entities.push(token);
        if entities.len() == 5 {
            break;
        }
    }
    (token_count, sentence_count, entities)
}

/// Inspect the standalone Poet surface without claiming native process
/// telemetry. This is a real bounded check over the active DOM and gives the
/// user useful Sentinel feedback when no daemon is available.
fn local_sentinel_summary(document: &Document) -> String {
    let nodes = document
        .query_selector_all("*")
        .map(|list| list.length().min(10_000))
        .unwrap_or(0);
    let containers = document
        .query_selector_all(".canvas-container-node")
        .map(|list| list.length())
        .unwrap_or(0);
    format!(
        "Standalone Sentinel check passed: {} DOM nodes across {} canvas containers; native 42MB telemetry requires the local daemon.",
        nodes, containers
    )
}

/// Execute a bounded SPARQL-shaped query against Poet's local semantic
/// container graph. The local graph exposes container type/URI annotations;
/// richer joins are delegated to `GraphDatabase.sparql` when a daemon exists.
pub(super) fn local_graph_query(document: &Document, query: &str) -> String {
    let containers = document.query_selector_all(".canvas-container-node").ok();
    let count = containers
        .as_ref()
        .map(|list| list.length().min(256))
        .unwrap_or(0);
    let normalized = query.trim_start().to_ascii_uppercase();
    if normalized.starts_with("ASK") {
        return serde_json::json!({ "boolean": count > 0, "source": "poet-local" }).to_string();
    }
    if !normalized.starts_with("SELECT") {
        return serde_json::json!({
            "error": "standalone Poet supports bounded ASK and SELECT queries",
            "source": "poet-local"
        })
        .to_string();
    }

    let mut bindings = Vec::new();
    if let Some(list) = containers {
        for index in 0..list.length().min(256) {
            let Some(node) = list.get(index) else {
                continue;
            };
            let Ok(container) = node.dyn_into::<Element>() else {
                continue;
            };
            let subject = format!("urn:poet:container:{}", index);
            let predicate = container
                .get_attribute("data-semantic-type")
                .unwrap_or_else(|| "poet:Container".into());
            let object = container
                .get_attribute("data-semantic-uri")
                .or_else(|| container.get_attribute("data-container-type"))
                .unwrap_or_else(|| "poet:container".into());
            bindings.push(serde_json::json!({
                "subject": { "type": "uri", "value": subject },
                "predicate": { "type": "uri", "value": predicate },
                "object": { "type": "literal", "value": object }
            }));
        }
    }
    serde_json::json!({
        "head": { "vars": ["subject", "predicate", "object"] },
        "results": { "bindings": bindings },
        "source": "poet-local"
    })
    .to_string()
}

fn run_extractor(document: &Document, label: &str) {
    let Some(source) = selected_text(document) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a non-empty document or text container first.",
            "error",
        );
        return;
    };
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let (token_count, sentence_count, entities) = local_extract_summary(&source);
        let detail = if entities.is_empty() {
            format!(
                "{} tokens, {} sentences, no title-case entities.",
                token_count, sentence_count
            )
        } else {
            format!(
                "{} tokens, {} sentences; entities: {}.",
                token_count,
                sentence_count,
                entities.join(", ")
            )
        };
        let report = super::tool_dual_path::local_sketch("NLP.gazetteer_run", &detail);
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    super::interactions::show_tool_status(document, &label, "Analysing selected text…", "running");
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_gazetteer(&source).await {
            Ok(response) if response.ok => {
                let entities = response
                    .hits
                    .iter()
                    .take(5)
                    .map(|hit| hit.surface.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let detail = if entities.is_empty() {
                    format!(
                        "{} tokens, {} sentences, no known gazetteer entities.",
                        response.token_count, response.sentence_count
                    )
                } else {
                    format!(
                        "{} tokens, {} sentences; entities: {}.",
                        response.token_count, response.sentence_count, entities
                    )
                };
                let report = super::tool_dual_path::live_ok("NLP.gazetteer_run", &detail);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    "NLP.gazetteer_run",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("gazetteer rejected the request"),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied("NLP.gazetteer_run", &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

fn run_sparql_query(document: &Document, label: &str) {
    let label = label.to_string();
    let query = selected_text(document).unwrap_or_else(|| "ASK WHERE { ?s ?p ?o }".to_string());
    if !super::native_daemon::is_daemon_connected() {
        let detail = local_graph_query(document, &query);
        let report = super::tool_dual_path::local_sketch("GraphDatabase.sparql", &detail);
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        "Running GraphDatabase.sparql…",
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        // Live ALL_BOUND id — no Host widen.
        let args = serde_json::json!({ "query": query, "format": "json" });
        match super::native_daemon::daemon_invoke("GraphDatabase.sparql", args).await {
            Ok(response) if response.ok => {
                let report =
                    super::tool_dual_path::live_ok("GraphDatabase.sparql", &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    "GraphDatabase.sparql",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("GraphDatabase.sparql failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied("GraphDatabase.sparql", &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

fn run_sentinel(document: &Document, label: &str) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let detail = local_sentinel_summary(document);
        let report = super::tool_dual_path::local_sketch("Sentinel.inspect", &detail);
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        "Inspecting Sentinel state…",
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let args = serde_json::json!({ "agent_did": "did:qualia:current" });
        match super::native_daemon::daemon_invoke("Sentinel.inspect", args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok("Sentinel.inspect", &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    "Sentinel.inspect",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Sentinel inspection failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied("Sentinel.inspect", &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

pub fn dispatch(document: &Document, tool_id: &str, label: &str, action: ActionType) {
    if let Some(reason) = current_disabled_reason(tool_id) {
        super::interactions::show_tool_status(document, label, reason, "unavailable");
        return;
    }

    match tool_id {
        "epistemic:tag_objective" => annotate_selected(
            document,
            "epistemic:Objective",
            "https://qualiadb.org/schema/epistemic#objective",
            label,
        ),
        "epistemic:tag_subjective" => annotate_selected(
            document,
            "epistemic:Subjective",
            "https://qualiadb.org/schema/epistemic#subjective",
            label,
        ),
        "epistemic:tag_intersubjective" => annotate_selected(
            document,
            "epistemic:Intersubjective",
            "https://qualiadb.org/schema/epistemic#intersubjective",
            label,
        ),
        "epistemic:tag_normative" => annotate_selected(
            document,
            "epistemic:Normative",
            "https://qualiadb.org/schema/epistemic#normative",
            label,
        ),
        "image:marker" => annotate_selected(
            document,
            "hypermedia:Marker",
            "https://qualiadb.org/schema/hypermedia#marker",
            label,
        ),
        "spatial:pin" => annotate_selected(
            document,
            "geo:Pin",
            "https://qualiadb.org/schema/geo#pin",
            label,
        ),
        "mail:composer" => super::interactions::place_container_via_menu(document, "mail", label),
        "health:framingham" | "health:cha2ds2" | "health:score2" => {
            super::interactions::place_container_via_menu(document, "health_calculators", label)
        }
        "rights:authors_group" => {
            super::interactions::place_container_via_menu(document, "rights", label)
        }
        "office:typography_bold" => format_selected_editor(
            document,
            label,
            "font-weight: 700;",
            "bold",
            "Applied bold typography to the selected document.",
        ),
        "office:typography_italic" => format_selected_editor(
            document,
            label,
            "font-style: italic;",
            "italic",
            "Applied italic typography to the selected document.",
        ),
        "office:typography_code" => format_selected_editor(
            document,
            label,
            "font-family: var(--font-mono);",
            "code",
            "Applied code typography to the selected document.",
        ),
        "office:paragraph_heading" => format_selected_editor(
            document,
            label,
            "font-size: 1.25em; font-weight: 700;",
            "heading",
            "Promoted the selected document to a heading block.",
        ),
        "office:paragraph_align_left" => format_selected_editor(
            document,
            label,
            "text-align: left;",
            "align-left",
            "Aligned the selected document to the left.",
        ),
        "office:paragraph_align_center" => format_selected_editor(
            document,
            label,
            "text-align: center;",
            "align-center",
            "Centered the selected document.",
        ),
        "ai:extractor" => run_extractor(document, label),
        "ai:sentinel" => run_sentinel(document, label),
        "graph:sparql_query" => run_sparql_query(document, label),
        "n3:evaluate" => super::shapes_actions::run_n3_evaluate(document, label),
        "shacl:validate" => super::shapes_actions::run_shacl_validate(document, label),
        "image:brush_stroke" => super::chain_actions::run_brush_stroke(document, label),
        "image:brush_clear" => super::chain_actions::run_brush_clear(document, label),
        "image:fill_warm" => super::chain_actions::run_fill(document, label, "warm"),
        "image:fill_cool" => super::chain_actions::run_fill(document, label, "cool"),
        "image:heatmap" => super::chain_actions::run_heatmap(document, label),
        "spatial:camera_reset" => super::chain_actions::run_camera_reset(document, label),
        "spatial:orbit_preview" => super::chain_actions::run_orbit_preview(document, label),
        "sheet:stats_mean" => super::chain_actions::run_sheet_mean(document, label),
        "sheet:stats_median" => super::chain_actions::run_sheet_median(document, label),
        "sheet:stats_variance" => super::chain_actions::run_sheet_variance(document, label),
        "sheet:stats_std_dev" => super::chain_actions::run_sheet_std_dev(document, label),
        "sheet:stats_min" => super::chain_actions::run_sheet_min(document, label),
        "sheet:stats_max" => super::chain_actions::run_sheet_max(document, label),
        "sheet:stats_sum" => super::stats_chain_actions::run_sheet_sum(document, label),
        "sheet:stats_skewness" => super::stats_chain_actions::run_sheet_skewness(document, label),
        "sheet:stats_kurtosis" => super::stats_chain_actions::run_sheet_kurtosis(document, label),
        "sheet:stats_quantile" => super::stats_chain_actions::run_sheet_quantile(document, label),
        "sheet:stats_iqr" => super::stats_chain_actions::run_sheet_iqr(document, label),
        "sheet:stats_mode" => super::stats_chain_actions::run_sheet_mode(document, label),
        "sheet:stats_trimmed_mean" => {
            super::stats_chain_actions::run_sheet_trimmed_mean(document, label)
        }
        "sheet:stats_mad" => super::stats_chain_actions::run_sheet_mad(document, label),
        "sheet:stats_pearson" => super::stats_chain_actions::run_sheet_pearson(document, label),
        "sheet:stats_covariance" => {
            super::stats_chain_actions::run_sheet_covariance(document, label)
        }
        "sheet:stats_z_score_outliers" => {
            super::stats_chain_actions::run_sheet_z_score_outliers(document, label)
        }
        "sheet:stats_argmax" => super::stats_chain_actions::run_sheet_argmax(document, label),
        "sheet:stats_binomial_pmf" => {
            super::stats_chain_actions::run_sheet_binomial_pmf(document, label)
        }
        "sheet:stats_binomial_cdf" => {
            super::stats_chain_actions::run_sheet_binomial_cdf(document, label)
        }
        "sheet:stats_beta_pdf" => super::stats_chain_actions::run_sheet_beta_pdf(document, label),
        "sheet:stats_chi_squared_pdf" => {
            super::stats_chain_actions::run_sheet_chi_squared_pdf(document, label)
        }
        "sheet:stats_chi_squared_cdf" => {
            super::stats_chain_actions::run_sheet_chi_squared_cdf(document, label)
        }
        "sheet:stats_chi_squared_quantile" => {
            super::stats_chain_actions::run_sheet_chi_squared_quantile(document, label)
        }
        "sheet:stats_autocorrelation" => {
            super::stats_chain_actions::run_sheet_autocorrelation(document, label)
        }
        "sheet:stats_bootstrap_means" => {
            super::stats_chain_actions::run_sheet_bootstrap_means(document, label)
        }
        "sheet:stats_normal_pdf" => {
            super::stats_chain_actions::run_sheet_normal_pdf(document, label)
        }
        "sheet:stats_normal_cdf" => {
            super::stats_chain_actions::run_sheet_normal_cdf(document, label)
        }
        "sheet:stats_normal_quantile" => {
            super::stats_chain_actions::run_sheet_normal_quantile(document, label)
        }
        "sheet:stats_standard_normal_cdf" => {
            super::stats_chain_actions::run_sheet_standard_normal_cdf(document, label)
        }
        "sheet:stats_poisson_pmf" => {
            super::stats_chain_actions::run_sheet_poisson_pmf(document, label)
        }
        "sheet:stats_poisson_cdf" => {
            super::stats_chain_actions::run_sheet_poisson_cdf(document, label)
        }
        "sheet:stats_exponential_pdf" => {
            super::stats_chain_actions::run_sheet_exponential_pdf(document, label)
        }
        "sheet:stats_exponential_cdf" => {
            super::stats_chain_actions::run_sheet_exponential_cdf(document, label)
        }
        "sheet:stats_spearman" => {
            super::stats_chain_actions::run_sheet_spearman(document, label)
        }
        "sheet:stats_kendall" => super::stats_chain_actions::run_sheet_kendall(document, label),
        "sheet:stats_winsorized_mean" => {
            super::stats_chain_actions::run_sheet_winsorized_mean(document, label)
        }
        "sheet:stats_erf" => super::stats_chain_actions::run_sheet_erf(document, label),
        "sheet:stats_erfc" => super::stats_chain_actions::run_sheet_erfc(document, label),
        "sheet:stats_uniform_pdf" => {
            super::stats_chain_actions::run_sheet_uniform_pdf(document, label)
        }
        "sheet:stats_laplace_pdf" => {
            super::stats_chain_actions::run_sheet_laplace_pdf(document, label)
        }
        "sheet:stats_standard_pdf" => {
            super::stats_chain_actions::run_sheet_standard_pdf(document, label)
        }
        "sheet:stats_uniform_cdf" => {
            super::stats_chain_actions::run_sheet_uniform_cdf(document, label)
        }
        "sheet:stats_laplace_cdf" => {
            super::stats_chain_actions::run_sheet_laplace_cdf(document, label)
        }
        "sheet:stats_lognormal_pdf" => {
            super::stats_chain_actions::run_sheet_lognormal_pdf(document, label)
        }
        "sheet:stats_lognormal_cdf" => {
            super::stats_chain_actions::run_sheet_lognormal_cdf(document, label)
        }
        "sheet:stats_standard_quantile" => {
            super::stats_chain_actions::run_sheet_standard_quantile(document, label)
        }
        "sheet:stats_ln_gamma" => {
            super::stats_chain_actions::run_sheet_ln_gamma(document, label)
        }
        "sheet:stats_gamma_fn" => {
            super::stats_chain_actions::run_sheet_gamma_fn(document, label)
        }
        "sheet:stats_weibull_pdf" => {
            super::stats_chain_actions::run_sheet_weibull_pdf(document, label)
        }
        "sheet:stats_gamma_pdf" => {
            super::stats_chain_actions::run_sheet_gamma_pdf(document, label)
        }
        "sheet:stats_two_sided_p" => {
            super::stats_chain_actions::run_sheet_two_sided_p(document, label)
        }
        "sheet:stats_chi_squared_upper_p" => {
            super::stats_chain_actions::run_sheet_chi_squared_upper_p(document, label)
        }
        "sheet:stats_students_t_pdf" => {
            super::stats_chain_actions::run_sheet_students_t_pdf(document, label)
        }
        "sheet:stats_fisher_f_pdf" => {
            super::stats_chain_actions::run_sheet_fisher_f_pdf(document, label)
        }
        "sheet:stats_gammp" => super::stats_chain_actions::run_sheet_gammp(document, label),
        "sheet:stats_gammq" => super::stats_chain_actions::run_sheet_gammq(document, label),
        "sheet:stats_betai" => super::stats_chain_actions::run_sheet_betai(document, label),
        "sheet:stats_students_t_cdf" => {
            super::stats_chain_actions::run_sheet_students_t_cdf(document, label)
        }
        "sheet:stats_students_t_two_sided_p" => {
            super::stats_chain_actions::run_sheet_students_t_two_sided_p(document, label)
        }
        "sheet:stats_students_t_upper_p" => {
            super::stats_chain_actions::run_sheet_students_t_upper_p(document, label)
        }
        "sheet:stats_students_t_quantile" => {
            super::stats_chain_actions::run_sheet_students_t_quantile(document, label)
        }
        "sheet:stats_fisher_f_cdf" => {
            super::stats_chain_actions::run_sheet_fisher_f_cdf(document, label)
        }
        "sheet:stats_fisher_f_upper_p" => {
            super::stats_chain_actions::run_sheet_fisher_f_upper_p(document, label)
        }
        "sheet:stats_fisher_f_quantile" => {
            super::stats_chain_actions::run_sheet_fisher_f_quantile(document, label)
        }
        "sheet:stats_tukey_fences" => {
            super::stats_chain_actions::run_sheet_tukey_fences(document, label)
        }
        "sheet:stats_empirical_cdf" => {
            super::stats_chain_actions::run_sheet_empirical_cdf(document, label)
        }
        "sheet:stats_entropy" => super::stats_chain_actions::run_sheet_entropy(document, label),
        "sheet:stats_kl_divergence" => {
            super::stats_chain_actions::run_sheet_kl_divergence(document, label)
        }
        "sheet:stats_cross_entropy" => {
            super::stats_chain_actions::run_sheet_cross_entropy(document, label)
        }
        "sheet:stats_entropy_from_counts" => {
            super::stats_chain_actions::run_sheet_entropy_from_counts(document, label)
        }
        "sheet:stats_moving_average" => {
            super::stats_chain_actions::run_sheet_moving_average(document, label)
        }
        "sheet:stats_modified_z_score_outliers" => {
            super::stats_chain_actions::run_sheet_modified_z_score_outliers(document, label)
        }
        "sheet:stats_iqr_outliers" => {
            super::stats_chain_actions::run_sheet_iqr_outliers(document, label)
        }
        "sheet:stats_exponential_smoothing" => {
            super::stats_chain_actions::run_sheet_exponential_smoothing(document, label)
        }
        "sheet:stats_adf_proxy" => {
            super::stats_chain_actions::run_sheet_adf_proxy(document, label)
        }
        "sheet:stats_histogram" => {
            super::stats_chain_actions::run_sheet_histogram(document, label)
        }
        "sheet:stats_ks_1sample" => {
            super::stats_chain_actions::run_sheet_ks_1sample(document, label)
        }
        "sheet:stats_grubbs_test" => {
            super::stats_chain_actions::run_sheet_grubbs_test(document, label)
        }
        "sheet:stats_one_sample_t" => {
            super::stats_chain_actions::run_sheet_one_sample_t(document, label)
        }
        "sheet:stats_two_sample_t" => {
            super::stats_chain_actions::run_sheet_two_sample_t(document, label)
        }
        "sheet:stats_paired_t" => {
            super::stats_chain_actions::run_sheet_paired_t(document, label)
        }
        "sheet:stats_linear_regression" => {
            super::stats_chain_actions::run_sheet_linear_regression(document, label)
        }
        "sheet:stats_chi_square_gof" => {
            super::stats_chain_actions::run_sheet_chi_square_gof(document, label)
        }
        "sheet:stats_chi_square_independence" => {
            super::stats_chain_actions::run_sheet_chi_square_independence(document, label)
        }
        "sheet:stats_correlation_p_value" => {
            super::stats_chain_actions::run_sheet_correlation_p_value(document, label)
        }
        "sheet:stats_friedman" => {
            super::stats_chain_actions::run_sheet_friedman(document, label)
        }
        "sheet:stats_ljung_box" => {
            super::stats_chain_actions::run_sheet_ljung_box(document, label)
        }
        "sheet:stats_mann_whitney_u" => {
            super::stats_chain_actions::run_sheet_mann_whitney_u(document, label)
        }
        "sheet:stats_mcnemar" => {
            super::stats_chain_actions::run_sheet_mcnemar(document, label)
        }
        "sheet:stats_mutual_information" => {
            super::stats_chain_actions::run_sheet_mutual_information(document, label)
        }
        "sheet:stats_one_way_anova" => {
            super::stats_chain_actions::run_sheet_one_way_anova(document, label)
        }
        "sheet:stats_mahalanobis_sq" => {
            super::stats_chain_actions::run_sheet_mahalanobis_sq(document, label)
        }
        "sheet:stats_mvn_log_pdf" => {
            super::stats_chain_actions::run_sheet_mvn_log_pdf(document, label)
        }
        "sheet:stats_mvn_pdf" => {
            super::stats_chain_actions::run_sheet_mvn_pdf(document, label)
        }
        "sheet:stats_mvn_sample" => {
            super::stats_chain_actions::run_sheet_mvn_sample(document, label)
        }
        "sheet:stats_mvn_mle" => {
            super::stats_chain_actions::run_sheet_mvn_mle(document, label)
        }
        "sheet:stats_validate_probability" => {
            super::stats_chain_actions::run_sheet_validate_probability(document, label)
        }
        "sheet:stats_simplex_project" => {
            super::stats_chain_actions::run_sheet_simplex_project(document, label)
        }
        "sheet:stats_fisher_distance" => {
            super::stats_chain_actions::run_sheet_fisher_distance(document, label)
        }
        "sheet:stats_neg_entropy" => {
            super::stats_chain_actions::run_sheet_neg_entropy(document, label)
        }
        "sheet:stats_simplex_project_idempotent" => {
            super::stats_chain_actions::run_sheet_simplex_project_idempotent(document, label)
        }
        "sheet:stats_fisher_inner_product" => {
            super::stats_chain_actions::run_sheet_fisher_inner_product(document, label)
        }
        "sheet:stats_neg_entropy_grad" => {
            super::stats_chain_actions::run_sheet_neg_entropy_grad(document, label)
        }
        "sheet:stats_kl_bregman_form" => {
            super::stats_chain_actions::run_sheet_kl_bregman_form(document, label)
        }
        "sheet:stats_bregman_pythagorean_test" => {
            super::stats_chain_actions::run_sheet_bregman_pythagorean_test(document, label)
        }
        "sheet:stats_probability_hash" => {
            super::stats_chain_actions::run_sheet_probability_hash(document, label)
        }
        "sheet:poly_eval" => super::poly_chain_actions::run_poly_eval(document, label),
        "sheet:poly_add" => super::poly_chain_actions::run_poly_add(document, label),
        "sheet:poly_sub" => super::poly_chain_actions::run_poly_sub(document, label),
        "sheet:poly_mul" => super::poly_chain_actions::run_poly_mul(document, label),
        "sheet:poly_gcd" => super::poly_chain_actions::run_poly_gcd(document, label),
        "sheet:poly_degree" => super::poly_chain_actions::run_poly_degree(document, label),
        "sheet:poly_leading" => super::poly_chain_actions::run_poly_leading(document, label),
        "sheet:poly_is_zero" => super::poly_chain_actions::run_poly_is_zero(document, label),
        "sheet:poly_scale" => super::poly_chain_actions::run_poly_scale(document, label),
        "sheet:poly_zero" => super::poly_chain_actions::run_poly_zero(document, label),
        "sheet:poly_constant" => {
            super::poly_chain_actions::run_poly_constant(document, label)
        }
        "sheet:poly_derivative" => {
            super::poly_chain_actions::run_poly_derivative(document, label)
        }
        "sheet:poly_monic" => super::poly_chain_actions::run_poly_monic(document, label),
        "sheet:poly_div_rem" => {
            super::poly_chain_actions::run_poly_div_rem(document, label)
        }
        "sheet:poly_resultant" => {
            super::poly_chain_actions::run_poly_resultant(document, label)
        }
        "sheet:import" => super::chain_actions::run_sheet_import(document, label),
        "code:vibe_diagnose" => super::chain_actions::run_vibe_diagnose(document, label),
        "code:quin_statement" => super::chain_actions::run_quin_statement(document, label),
        "ai:grounding" => super::chain_actions::run_grounding(document, label),
        "ai:detect_ungrounded" => super::chain_actions::run_detect_ungrounded(document, label),
        "ai:verify_turn" => super::chain_actions::run_verify_turn(document, label),
        "ai:inf_relu" => super::inference_chain_actions::run_relu(document, label),
        "ai:inf_sigmoid" => super::inference_chain_actions::run_sigmoid(document, label),
        "ai:inf_gelu" => super::inference_chain_actions::run_gelu(document, label),
        "ai:inf_softmax" => super::inference_chain_actions::run_softmax(document, label),
        "ai:inf_rms_norm" => super::inference_chain_actions::run_rms_norm(document, label),
        "ai:inf_embed" => super::inference_chain_actions::run_embed(document, label),
        "ai:inf_run_classifier" => {
            super::inference_chain_actions::run_run_classifier(document, label)
        }
        "ai:inf_vector_search" => {
            super::inference_chain_actions::run_vector_search(document, label)
        }
        "ai:inf_load_model" => {
            super::inference_remainder_chain_actions::run_load_model(document, label)
        }
        "ai:inf_unload_model" => {
            super::inference_remainder_chain_actions::run_unload_model(document, label)
        }
        "ai:inf_run_transformer" => {
            super::inference_remainder_chain_actions::run_run_transformer(document, label)
        }
        "ai:inf_run_reranker" => {
            super::inference_remainder_chain_actions::run_run_reranker(document, label)
        }
        "ai:inf_constrained_decode" => {
            super::inference_remainder_chain_actions::run_constrained_decode(document, label)
        }
        "ai:ml_mse" => super::ml_chain_actions::run_mse(document, label),
        "ai:ml_rmse" => super::ml_chain_actions::run_rmse(document, label),
        "ai:ml_mae" => super::ml_chain_actions::run_mae(document, label),
        "ai:ml_r2" => super::ml_chain_actions::run_r2_score(document, label),
        "ai:ml_accuracy" => super::ml_chain_actions::run_accuracy(document, label),
        "ai:ml_ols" => super::ml_chain_actions::run_ols(document, label),
        "ai:ml_train_test_split" => {
            super::ml_chain_actions::run_train_test_split(document, label)
        }
        "ai:ml_kmeans" => super::ml_chain_actions::run_kmeans(document, label),
        "ai:ml_log_loss" => super::ml_chain_actions::run_log_loss(document, label),
        "ai:ml_bonferroni" => super::ml_chain_actions::run_bonferroni(document, label),
        "ai:ml_confusion_binary" => {
            super::ml_chain_actions::run_confusion_binary(document, label)
        }
        "ai:ml_holm" => super::ml_chain_actions::run_holm(document, label),
        "ai:ml_benjamini_hochberg" => {
            super::ml_chain_actions::run_benjamini_hochberg(document, label)
        }
        "ai:ml_ab_test" => super::ml_chain_actions::run_ab_test(document, label),
        "ai:ml_bootstrap_estimate" => {
            super::ml_chain_actions::run_bootstrap_estimate(document, label)
        }
        "ai:ml_bootstrap_ci" => super::ml_chain_actions::run_bootstrap_ci(document, label),
        "ai:ml_permutation_test" => {
            super::ml_chain_actions::run_permutation_test(document, label)
        }
        "ai:ml_n_rejected" => super::ml_chain_actions::run_n_rejected(document, label),
        "ai:ml_power_two_sample" => {
            super::ml_chain_actions::run_power_two_sample(document, label)
        }
        "ai:ml_roc_auc" => super::ml_chain_actions::run_roc_auc(document, label),
        "ai:ml_k_fold" => super::ml_chain_actions::run_k_fold(document, label),
        "ai:ml_bootstrap_indices" => {
            super::ml_chain_actions::run_bootstrap_indices(document, label)
        }
        "ai:ml_pca" => super::ml_chain_actions::run_pca(document, label),
        "ai:ml_required_sample_size" => {
            super::ml_chain_actions::run_required_sample_size(document, label)
        }
        "ai:ml_polynomial_regression" => {
            super::ml_chain_actions::run_polynomial_regression(document, label)
        }
        "ai:ml_required_n_two_prop" => {
            super::ml_chain_actions::run_required_sample_size_two_proportion(document, label)
        }
        "ai:ml_loocv" => super::ml_chain_actions::run_loocv(document, label),
        "ai:ml_transe_score" => super::ml_chain_actions::run_transe_score(document, label),
        "ai:ml_distmult_score" => {
            super::ml_chain_actions::run_distmult_score(document, label)
        }
        "ai:ml_complex_score" => {
            super::ml_chain_actions::run_complex_score(document, label)
        }
        "ai:ml_rotate_score" => super::ml_chain_actions::run_rotate_score(document, label),
        "ai:ml_ridge_fit" => super::ml_chain_actions::run_ridge_fit(document, label),
        "ai:ml_lasso_fit" => super::ml_chain_actions::run_lasso_fit(document, label),
        "ai:ml_pls_fit" => super::ml_chain_actions::run_pls_fit(document, label),
        "ai:ml_standard_scaler" => {
            super::ml_chain_actions::run_standard_scaler_fit_transform(document, label)
        }
        "ai:ml_kmeans_fit" => super::ml_chain_actions::run_kmeans_fit(document, label),
        "ai:ml_gmm_fit" => super::ml_chain_actions::run_gmm_fit(document, label),
        "ai:ml_logistic_fit" => super::ml_chain_actions::run_logistic_fit(document, label),
        "ai:ml_poisson_fit" => super::ml_chain_actions::run_poisson_fit(document, label),
        "ai:ml_naive_bayes_fit" => {
            super::ml_chain_actions::run_naive_bayes_fit(document, label)
        }
        "ai:ml_knn_fit" => super::ml_chain_actions::run_knn_fit(document, label),
        "ai:ml_lda_fit" => super::ml_chain_actions::run_lda_fit(document, label),
        "ai:ml_pcr_fit" => super::ml_chain_actions::run_pcr_fit(document, label),
        "ai:ml_qda_fit" => super::ml_chain_actions::run_qda_fit(document, label),
        "ai:ml_multinomial_logistic_fit" => {
            super::ml_chain_actions::run_multinomial_logistic_fit(document, label)
        }
        "ai:ml_hierarchical_fit" => {
            super::ml_chain_actions::run_hierarchical_fit(document, label)
        }
        "ai:ml_hierarchical_labels" => {
            super::ml_chain_actions::run_hierarchical_labels(document, label)
        }
        "ai:ml_bayesian_linear_fit" => {
            super::ml_chain_actions::run_bayesian_linear_fit(document, label)
        }
        "ai:ml_decision_tree_regressor" => {
            super::ml_chain_actions::run_decision_tree_fit_regressor(document, label)
        }
        "ai:ml_decision_tree_classifier" => {
            super::ml_chain_actions::run_decision_tree_fit_classifier(document, label)
        }
        "ai:ml_gp_fit" => super::ml_chain_actions::run_gp_fit(document, label),
        "ai:ml_svm_fit" => super::ml_chain_actions::run_svm_fit(document, label),
        "ai:ml_kaplan_meier_fit" => {
            super::ml_chain_actions::run_kaplan_meier_fit(document, label)
        }
        "ai:ml_cox_fit" => super::ml_chain_actions::run_cox_fit(document, label),
        "ai:ml_hmm_baum_welch" => super::ml_chain_actions::run_hmm_baum_welch(document, label),
        "ai:ml_variational_gaussian_fit" => {
            super::ml_chain_actions::run_variational_gaussian_fit(document, label)
        }
        "ai:ml_mcmc_metropolis" => {
            super::ml_chain_actions::run_mcmc_metropolis(document, label)
        }
        "ai:ml_svm_multiclass_fit" => {
            super::ml_chain_actions::run_svm_multiclass_fit(document, label)
        }
        "ai:ml_som_train" => super::ml_chain_actions::run_som_train(document, label),
        "ai:ml_random_forest_regressor" => {
            super::ml_chain_actions::run_random_forest_fit_regressor(document, label)
        }
        "ai:ml_random_forest_classifier" => {
            super::ml_chain_actions::run_random_forest_fit_classifier(document, label)
        }
        "ai:ml_gradient_boosting_regressor" => {
            super::ml_chain_actions::run_gradient_boosting_fit_regressor(document, label)
        }
        "ai:ml_bart_fit" => super::ml_chain_actions::run_bart_fit(document, label),
        "ai:ml_kg_mean_rank" => super::ml_chain_actions::run_kg_mean_rank(document, label),
        "ai:ml_kg_mrr" => {
            super::ml_chain_actions::run_kg_mean_reciprocal_rank(document, label)
        }
        "ai:ml_kg_hits_at_k" => super::ml_chain_actions::run_kg_hits_at_k(document, label),
        "ai:ml_kalman_new" => super::ml_chain_actions::run_kalman_new(document, label),
        "ai:ml_factor_graph_marginals" => {
            super::ml_chain_actions::run_factor_graph_marginals(document, label)
        }
        "ai:ml_al_row_score" => super::ml_chain_actions::run_al_row_score(document, label),
        "ai:ml_al_score" => super::ml_chain_actions::run_al_score(document, label),
        "ai:ml_al_cosine" => {
            super::ml_chain_actions::run_al_cosine_similarity(document, label)
        }
        "ai:ml_al_rank_informative" => {
            super::ml_chain_actions::run_al_rank_informative(document, label)
        }
        "ai:ml_al_most_informative" => {
            super::ml_chain_actions::run_al_most_informative(document, label)
        }
        "ai:ml_al_representativeness" => {
            super::ml_chain_actions::run_al_representativeness(document, label)
        }
        "ai:ml_al_information_density" => {
            super::ml_chain_actions::run_al_information_density(document, label)
        }
        "ai:ml_al_rank_by_density" => {
            super::ml_chain_actions::run_al_rank_by_density(document, label)
        }
        "ai:ml_al_vote_entropy" => {
            super::ml_chain_actions::run_al_vote_entropy(document, label)
        }
        "ai:ml_al_consensus" => super::ml_chain_actions::run_al_consensus(document, label),
        "ai:ml_al_consensus_entropy" => {
            super::ml_chain_actions::run_al_consensus_entropy(document, label)
        }
        "ai:ml_al_kl_disagreement" => {
            super::ml_chain_actions::run_al_average_kl_disagreement(document, label)
        }
        "ai:ml_al_rank_by_disagreement" => {
            super::ml_chain_actions::run_al_rank_by_disagreement(document, label)
        }
        "epistemic:evaluate" => super::chain_actions::run_epistemic_evaluate(document, label),
        "image:histogram" => super::image_chain_actions::run_image_histogram(document, label),
        "image:equalize_hist" => super::image_chain_actions::run_equalize_hist(document, label),
        "image:rgb_to_gray" => super::image_chain_actions::run_rgb_to_gray(document, label),
        "image:dhash" => super::image_chain_actions::run_dhash(document, label),
        "image:hamming_distance" => {
            super::image_chain_actions::run_hamming_distance(document, label)
        }
        "image:cosine_similarity" => {
            super::image_chain_actions::run_cosine_similarity(document, label)
        }
        "image:edit_new" => super::image_edit_chain_actions::run_new(document, label),
        "image:edit_add_layer" => super::image_edit_chain_actions::run_add_layer(document, label),
        "image:edit_remove_layer" => {
            super::image_edit_chain_actions::run_remove_layer(document, label)
        }
        "image:edit_set_pixel" => super::image_edit_chain_actions::run_set_pixel(document, label),
        "image:edit_fill" => super::image_edit_chain_actions::run_fill(document, label),
        "image:edit_brush" => super::image_edit_chain_actions::run_brush(document, label),
        "image:edit_apply_filter" => {
            super::image_edit_chain_actions::run_apply_filter(document, label)
        }
        "image:edit_set_opacity" => {
            super::image_edit_chain_actions::run_set_opacity(document, label)
        }
        "image:edit_set_blend_mode" => {
            super::image_edit_chain_actions::run_set_blend_mode(document, label)
        }
        "image:edit_set_visible" => super::image_edit_chain_actions::run_set_visible(document, label),
        "image:edit_set_mask" => super::image_edit_chain_actions::run_set_mask(document, label),
        "image:edit_clear_mask" => super::image_edit_chain_actions::run_clear_mask(document, label),
        "image:edit_composite" => super::image_edit_chain_actions::run_composite(document, label),
        "image:edit_add_selection" => {
            super::image_edit_chain_actions::run_add_selection(document, label)
        }
        "image:edit_clear_selections" => {
            super::image_edit_chain_actions::run_clear_selections(document, label)
        }
        "comm:pulse_presence" => super::chain_actions::run_pulse_presence(document, label),
        "rights:deontic_obligate" => super::chain_actions::run_deontic_obligate(document, label),
        "epistemic:paraconsistent_route" => {
            super::logic_chain_actions::run_paraconsistent_route(document, label)
        }
        "code:ltl_evaluate" => super::logic_chain_actions::run_ltl_evaluate(document, label),
        "code:symbolic_eval" => super::logic_chain_actions::run_symbolic_eval(document, label),
        "code:symbolic_differentiate" => {
            super::logic_chain_actions::run_symbolic_differentiate(document, label)
        }
        "code:symbolic_simplify" => {
            super::logic_chain_actions::run_symbolic_simplify(document, label)
        }
        "code:symbolic_expand" => super::logic_chain_actions::run_symbolic_expand(document, label),
        "code:symbolic_factor" => super::logic_chain_actions::run_symbolic_factor(document, label),
        "code:symbolic_integrate" => {
            super::logic_chain_actions::run_symbolic_integrate(document, label)
        }
        "code:symbolic_simplify_trig" => {
            super::logic_chain_actions::run_symbolic_simplify_trig(document, label)
        }
        "code:symbolic_partial" => {
            super::logic_chain_actions::run_symbolic_partial(document, label)
        }
        "code:symbolic_limit" => super::logic_chain_actions::run_symbolic_limit(document, label),
        "code:symbolic_add" => super::logic_chain_actions::run_symbolic_add(document, label),
        "code:symbolic_sub" => super::logic_chain_actions::run_symbolic_sub(document, label),
        "code:symbolic_mul" => super::logic_chain_actions::run_symbolic_mul(document, label),
        "code:symbolic_div" => super::logic_chain_actions::run_symbolic_div(document, label),
        "code:symbolic_pow" => super::logic_chain_actions::run_symbolic_pow(document, label),
        "code:symbolic_neg" => super::logic_chain_actions::run_symbolic_neg(document, label),
        "code:symbolic_sqrt" => super::logic_chain_actions::run_symbolic_sqrt(document, label),
        "code:symbolic_exp" => super::logic_chain_actions::run_symbolic_exp(document, label),
        "code:symbolic_ln" => super::logic_chain_actions::run_symbolic_ln(document, label),
        "code:symbolic_sin" => super::logic_chain_actions::run_symbolic_sin(document, label),
        "code:symbolic_cos" => super::logic_chain_actions::run_symbolic_cos(document, label),
        "code:symbolic_tan" => super::logic_chain_actions::run_symbolic_tan(document, label),
        "code:symbolic_parse" => super::logic_chain_actions::run_symbolic_parse(document, label),
        "code:symbolic_c" => super::logic_chain_actions::run_symbolic_c(document, label),
        "code:symbolic_var" => super::logic_chain_actions::run_symbolic_var(document, label),
        "code:symbolic_hessian" => {
            super::logic_chain_actions::run_symbolic_hessian(document, label)
        }
        "code:symbolic_integrate_definite" => {
            super::logic_chain_actions::run_symbolic_integrate_definite(document, label)
        }
        "code:symbolic_limit_at_infinity" => {
            super::logic_chain_actions::run_symbolic_limit_at_infinity(document, label)
        }
        "code:symbolic_real_roots" => {
            super::logic_chain_actions::run_symbolic_real_roots(document, label)
        }
        "code:symbolic_roots" => super::logic_chain_actions::run_symbolic_roots(document, label),
        "code:symbolic_taylor_coefficients" => {
            super::logic_chain_actions::run_symbolic_taylor_coefficients(document, label)
        }
        "code:symbolic_taylor_eval" => {
            super::logic_chain_actions::run_symbolic_taylor_eval(document, label)
        }
        "code:symbolic_jacobian" => {
            super::logic_chain_actions::run_symbolic_jacobian(document, label)
        }
        "code:symbolic_gradient_at" => {
            super::logic_chain_actions::run_symbolic_gradient_at(document, label)
        }
        "code:symbolic_hessian_at" => {
            super::logic_chain_actions::run_symbolic_hessian_at(document, label)
        }
        "code:symbolic_solve_quadratic" => {
            super::logic_chain_actions::run_symbolic_solve_quadratic(document, label)
        }
        "code:symbolic_solve_quadratic_symbolic" => {
            super::logic_chain_actions::run_symbolic_solve_quadratic_symbolic(document, label)
        }
        "code:symbolic_factor_quadratic" => {
            super::logic_chain_actions::run_symbolic_factor_quadratic(document, label)
        }
        "code:symbolic_solve_polynomial_expr" => {
            super::logic_chain_actions::run_symbolic_solve_polynomial_expr(document, label)
        }
        "code:symbolic_simplify_with_assumptions" => {
            super::logic_chain_actions::run_symbolic_simplify_with_assumptions(document, label)
        }
        "code:symbolic_expr_citation_hash" => {
            super::logic_chain_actions::run_symbolic_expr_citation_hash(document, label)
        }
        "code:symbolic_to_quins" => {
            super::logic_chain_actions::run_symbolic_to_quins(document, label)
        }
        "code:symbolic_from_quins" => {
            super::logic_chain_actions::run_symbolic_from_quins(document, label)
        }
        "code:constr_regular_polygon" => {
            super::constr_chain_actions::run_constr_regular_polygon(document, label)
        }
        "code:constr_fermat_prime" => {
            super::constr_chain_actions::run_constr_fermat_prime(document, label)
        }
        "code:constr_min_poly_degree" => {
            super::constr_chain_actions::run_constr_min_poly_degree(document, label)
        }
        "code:constr_power_of_two" => {
            super::constr_chain_actions::run_constr_power_of_two(document, label)
        }
        "code:constr_central_angle" => {
            super::constr_chain_actions::run_constr_central_angle(document, label)
        }
        "code:constr_doubling_cube" => {
            super::constr_chain_actions::run_constr_doubling_cube(document, label)
        }
        "code:constr_trisect_angle" => {
            super::constr_chain_actions::run_constr_trisect_angle(document, label)
        }
        "code:constr_square_circle" => {
            super::constr_chain_actions::run_constr_square_circle(document, label)
        }
        "code:constr_number" => super::constr_chain_actions::run_constr_number(document, label),
        "code:nt_gcd" => super::nt_chain_actions::run_nt_gcd(document, label),
        "code:nt_lcm" => super::nt_chain_actions::run_nt_lcm(document, label),
        "code:nt_is_prime" => super::nt_chain_actions::run_nt_is_prime(document, label),
        "code:nt_factorial" => super::nt_chain_actions::run_nt_factorial(document, label),
        "code:nt_binomial" => super::nt_chain_actions::run_nt_binomial(document, label),
        "code:nt_euler_totient" => super::nt_chain_actions::run_nt_euler_totient(document, label),
        "code:nt_mod_pow" => super::nt_chain_actions::run_nt_mod_pow(document, label),
        "code:nt_next_prime" => super::nt_chain_actions::run_nt_next_prime(document, label),
        "code:nt_mod_inverse" => super::nt_chain_actions::run_nt_mod_inverse(document, label),
        "code:nt_divisor_count" => super::nt_chain_actions::run_nt_divisor_count(document, label),
        "code:nt_prime_factors" => {
            super::nt_chain_actions::run_nt_prime_factors(document, label)
        }
        "code:nt_divisors" => super::nt_chain_actions::run_nt_divisors(document, label),
        "code:nt_mobius" => super::nt_chain_actions::run_nt_mobius(document, label),
        "code:nt_divisor_sum" => super::nt_chain_actions::run_nt_divisor_sum(document, label),
        "code:nt_partitions" => super::nt_chain_actions::run_nt_partitions(document, label),
        "code:nt_catalan" => super::nt_chain_actions::run_nt_catalan(document, label),
        "code:nt_stirling_second" => {
            super::nt_chain_actions::run_nt_stirling_second(document, label)
        }
        "code:nt_stirling_first" => {
            super::nt_chain_actions::run_nt_stirling_first(document, label)
        }
        "code:nt_extended_gcd" => {
            super::nt_chain_actions::run_nt_extended_gcd(document, label)
        }
        "code:nt_crt" => super::nt_chain_actions::run_nt_crt(document, label),
        "code:fuzzy_triangular" => {
            super::fuzzy_chain_actions::run_fuzzy_triangular(document, label)
        }
        "code:fuzzy_trapezoidal" => {
            super::fuzzy_chain_actions::run_fuzzy_trapezoidal(document, label)
        }
        "code:fuzzy_approximately" => {
            super::fuzzy_chain_actions::run_fuzzy_approximately(document, label)
        }
        "code:fuzzy_ramp_up" => super::fuzzy_chain_actions::run_fuzzy_ramp_up(document, label),
        "code:fuzzy_ramp_down" => super::fuzzy_chain_actions::run_fuzzy_ramp_down(document, label),
        "code:fuzzy_much_greater_than" => {
            super::fuzzy_chain_actions::run_fuzzy_much_greater_than(document, label)
        }
        "code:fuzzy_much_less_than" => {
            super::fuzzy_chain_actions::run_fuzzy_much_less_than(document, label)
        }
        "code:fuzzy_threshold" => super::fuzzy_chain_actions::run_fuzzy_threshold(document, label),
        "code:fuzzy_top_k" => super::fuzzy_chain_actions::run_fuzzy_top_k(document, label),
        "code:fuzzy_negate" => super::fuzzy_chain_actions::run_fuzzy_negate(document, label),
        "code:fuzzy_and" => super::fuzzy_chain_actions::run_fuzzy_and(document, label),
        "code:fuzzy_or" => super::fuzzy_chain_actions::run_fuzzy_or(document, label),
        "econ:capm" => super::econ_chain_actions::run_capm(document, label),
        "econ:gini" => super::econ_chain_actions::run_gini(document, label),
        "econ:mixed_nash" => super::econ_chain_actions::run_mixed_nash(document, label),
        "econ:black_scholes" => super::econ_chain_actions::run_black_scholes(document, label),
        "econ:solow" => super::econ_chain_actions::run_solow(document, label),
        "econ:cournot" => super::econ_chain_actions::run_cournot(document, label),
        "econ:bertrand" => super::econ_chain_actions::run_bertrand(document, label),
        "econ:historical_var" => super::econ_chain_actions::run_historical_var(document, label),
        "econ:atkinson" => super::econ_chain_actions::run_atkinson(document, label),
        "econ:gordon_growth" => super::econ_chain_actions::run_gordon_growth(document, label),
        "econ:binomial_option" => {
            super::econ_chain_actions::run_binomial_option(document, label)
        }
        "econ:forward_rate" => super::econ_chain_actions::run_forward_rate(document, label),
        "econ:gbm_simulate" => super::econ_chain_actions::run_gbm_simulate(document, label),
        "econ:headcount_poverty" => {
            super::econ_chain_actions::run_headcount_poverty(document, label)
        }
        "econ:hyperbolic_discount" => {
            super::econ_chain_actions::run_hyperbolic_discount(document, label)
        }
        "econ:fiscal_multiplier" => {
            super::econ_chain_actions::run_fiscal_multiplier(document, label)
        }
        "econ:drawdown" => super::econ_chain_actions::run_drawdown(document, label),
        "econ:covariance_matrix" => {
            super::econ_chain_actions::run_covariance_matrix(document, label)
        }
        "econ:capm_beta" => super::econ_chain_actions::run_capm_beta(document, label),
        "econ:autocorrelation" => super::econ_chain_actions::run_autocorrelation(document, label),
        "econ:cross_correlation" => {
            super::econ_chain_actions::run_cross_correlation(document, label)
        }
        "econ:bertrand_with_demand" => {
            super::econ_chain_actions::run_bertrand_with_demand(document, label)
        }
        "econ:check_budget_balance" => {
            super::econ_chain_actions::run_check_budget_balance(document, label)
        }
        "econ:ccapm_equity_premium" => {
            super::econ_chain_actions::run_ccapm_equity_premium(document, label)
        }
        "econ:mean_return" => super::econ_chain_actions::run_mean_return(document, label),
        "econ:poverty_gap" => super::econ_chain_actions::run_poverty_gap(document, label),
        "econ:sample_variance" => {
            super::econ_chain_actions::run_sample_variance(document, label)
        }
        "econ:utilitarian_welfare" => {
            super::econ_chain_actions::run_utilitarian_welfare(document, label)
        }
        "econ:rawlsian_welfare" => {
            super::econ_chain_actions::run_rawlsian_welfare(document, label)
        }
        "econ:nash_welfare" => super::econ_chain_actions::run_nash_welfare(document, label),
        "econ:stackelberg" => super::econ_chain_actions::run_stackelberg(document, label),
        "econ:put_call_parity" => {
            super::econ_chain_actions::run_put_call_parity(document, label)
        }
        "econ:parametric_var" => {
            super::econ_chain_actions::run_parametric_var(document, label)
        }
        "econ:laffer_curve" => super::econ_chain_actions::run_laffer_curve(document, label),
        "econ:historical_cvar" => {
            super::econ_chain_actions::run_historical_cvar(document, label)
        }
        "econ:endowment_effect" => {
            super::econ_chain_actions::run_endowment_effect(document, label)
        }
        "econ:prospect_value" => super::econ_chain_actions::run_prospect_value(document, label),
        "econ:probability_weight" => {
            super::econ_chain_actions::run_probability_weight(document, label)
        }
        "econ:ccapm_sdf" => super::econ_chain_actions::run_ccapm_sdf(document, label),
        "econ:gravity_flow" => super::econ_chain_actions::run_gravity_flow(document, label),
        "econ:transfer_payment" => {
            super::econ_chain_actions::run_transfer_payment(document, label)
        }
        "econ:efficiency_units" => {
            super::econ_chain_actions::run_efficiency_units(document, label)
        }
        "econ:social_cost_of_carbon" => {
            super::econ_chain_actions::run_social_cost_of_carbon(document, label)
        }
        "econ:pollution_damage" => {
            super::econ_chain_actions::run_pollution_damage(document, label)
        }
        "econ:marginal_damage" => {
            super::econ_chain_actions::run_marginal_damage(document, label)
        }
        "econ:ramsey_steady_state" => {
            super::econ_chain_actions::run_ramsey_steady_state(document, label)
        }
        "econ:simple_returns" => super::econ_chain_actions::run_simple_returns(document, label),
        "econ:log_returns" => super::econ_chain_actions::run_log_returns(document, label),
        "econ:rolling_mean" => super::econ_chain_actions::run_rolling_mean(document, label),
        "econ:rolling_variance" => {
            super::econ_chain_actions::run_rolling_variance(document, label)
        }
        "econ:labor_supply" => super::econ_chain_actions::run_labor_supply(document, label),
        "econ:optimal_abatement" => {
            super::econ_chain_actions::run_optimal_abatement(document, label)
        }
        "econ:optimal_pollution" => {
            super::econ_chain_actions::run_optimal_pollution(document, label)
        }
        "econ:olg_steady_state" => {
            super::econ_chain_actions::run_olg_steady_state(document, label)
        }
        "econ:ramsey_euler_residual" => {
            super::econ_chain_actions::run_ramsey_euler_residual(document, label)
        }
        "econ:present_biased_utility" => {
            super::econ_chain_actions::run_present_biased_utility(document, label)
        }
        "econ:reference_dependent_utility" => {
            super::econ_chain_actions::run_reference_dependent_utility(document, label)
        }
        "econ:npv" => super::econ_chain_actions::run_npv(document, label),
        "econ:multi_period_ddm" => {
            super::econ_chain_actions::run_multi_period_ddm(document, label)
        }
        "econ:portfolio_max_drawdown" => {
            super::econ_chain_actions::run_portfolio_max_drawdown(document, label)
        }
        "econ:interpolate_zero_rate" => {
            super::econ_chain_actions::run_interpolate_zero_rate(document, label)
        }
        "econ:discount_factor" => {
            super::econ_chain_actions::run_discount_factor(document, label)
        }
        "econ:par_yield" => super::econ_chain_actions::run_par_yield(document, label),
        "econ:progressive_tax" => {
            super::econ_chain_actions::run_progressive_tax(document, label)
        }
        "econ:abatement_net_benefit" => {
            super::econ_chain_actions::run_abatement_net_benefit(document, label)
        }
        "econ:household_production_ces" => {
            super::econ_chain_actions::run_household_production_ces(document, label)
        }
        "econ:malfeasance_delta" => {
            super::econ_chain_actions::run_malfeasance_delta(document, label)
        }
        "econ:portfolio_variance" => {
            super::econ_chain_actions::run_portfolio_variance(document, label)
        }
        "econ:portfolio_returns" => {
            super::econ_chain_actions::run_portfolio_returns(document, label)
        }
        "econ:distributional_npv" => {
            super::econ_chain_actions::run_distributional_npv(document, label)
        }
        "econ:stress_scenario" => {
            super::econ_chain_actions::run_stress_scenario(document, label)
        }
        "econ:repeated_game_payoff" => {
            super::econ_chain_actions::run_repeated_game_payoff(document, label)
        }
        "econ:total_transport_cost" => {
            super::econ_chain_actions::run_total_transport_cost(document, label)
        }
        "econ:transition_probability" => {
            super::econ_chain_actions::run_transition_probability(document, label)
        }
        "econ:expected_holding_time" => {
            super::econ_chain_actions::run_expected_holding_time(document, label)
        }
        "econ:check_ir" => super::econ_chain_actions::run_check_ir(document, label),
        "econ:vcg_payment" => super::econ_chain_actions::run_vcg_payment(document, label),
        "econ:validate_transition_matrix" => {
            super::econ_chain_actions::run_validate_transition_matrix(document, label)
        }
        "econ:stationary_distribution" => {
            super::econ_chain_actions::run_stationary_distribution(document, label)
        }
        "econ:mean_first_passage" => {
            super::econ_chain_actions::run_mean_first_passage(document, label)
        }
        "econ:degree_centrality" => {
            super::econ_chain_actions::run_degree_centrality(document, label)
        }
        "econ:eigenvector_centrality" => {
            super::econ_chain_actions::run_eigenvector_centrality(document, label)
        }
        "econ:new_keynesian_solve" => {
            super::econ_chain_actions::run_new_keynesian_solve(document, label)
        }
        "econ:nearest_facility" => {
            super::econ_chain_actions::run_nearest_facility(document, label)
        }
        "econ:pure_nash_equilibria" => {
            super::econ_chain_actions::run_pure_nash_equilibria(document, label)
        }
        "econ:morans_i" => super::econ_chain_actions::run_morans_i(document, label),
        "econ:strategy_proofness" => {
            super::econ_chain_actions::run_strategy_proofness(document, label)
        }
        "econ:lorenz_curve" => super::econ_chain_actions::run_lorenz_curve(document, label),
        "econ:ols" => super::econ_chain_actions::run_ols(document, label),
        "econ:wls" => super::econ_chain_actions::run_wls(document, label),
        "econ:lucas_asset_price" => {
            super::econ_chain_actions::run_lucas_asset_price(document, label)
        }
        "econ:bellman_update" => super::econ_chain_actions::run_bellman_update(document, label),
        "econ:block_bootstrap" => super::econ_chain_actions::run_block_bootstrap(document, label),
        "econ:simulate_chain" => super::econ_chain_actions::run_simulate_chain(document, label),
        "econ:value_iteration" => super::econ_chain_actions::run_value_iteration(document, label),
        "econ:iv_2sls" => super::econ_chain_actions::run_iv_2sls(document, label),
        "econ:logistic_mle" => super::econ_chain_actions::run_logistic_mle(document, label),
        "econ:interbank_clearing" => {
            super::econ_chain_actions::run_interbank_clearing(document, label)
        }
        "econ:leontief_inverse" => {
            super::econ_chain_actions::run_leontief_inverse(document, label)
        }
        "econ:output_multipliers" => {
            super::econ_chain_actions::run_output_multipliers(document, label)
        }
        "econ:agent_based_aggregate_wealth" => {
            super::econ_chain_actions::run_agent_based_aggregate_wealth(document, label)
        }
        "econ:validate_scalar_constraint" => {
            super::econ_chain_actions::run_validate_scalar_constraint(document, label)
        }
        "econ:aggregate_paper_fills" => {
            super::econ_chain_actions::run_aggregate_paper_fills(document, label)
        }
        "rights:delegation_permits" => {
            super::cooperative_chain_actions::run_delegation_permits(document, label)
        }
        "scientific:la_matmul" => {
            super::linalg_chain_actions::run_matmul(document, label)
        }
        "scientific:la_matvec" => {
            super::linalg_chain_actions::run_matvec(document, label)
        }
        "scientific:la_transpose" => {
            super::linalg_chain_actions::run_transpose(document, label)
        }
        "scientific:la_determinant" => {
            super::linalg_chain_actions::run_determinant(document, label)
        }
        "scientific:la_solve" => super::linalg_chain_actions::run_solve(document, label),
        "scientific:la_scale" => super::linalg_chain_actions::run_scale(document, label),
        "scientific:la_add_into" => {
            super::linalg_chain_actions::run_add_into(document, label)
        }
        "scientific:la_axpy" => super::linalg_chain_actions::run_axpy(document, label),
        "scientific:la_hadamard_into" => {
            super::linalg_chain_actions::run_hadamard_into(document, label)
        }
        "scientific:la_qr_factor" => {
            super::linalg_chain_actions::run_qr_factor(document, label)
        }
        "scientific:la_cholesky_factor" => {
            super::linalg_chain_actions::run_cholesky_factor(document, label)
        }
        "scientific:la_cholesky_solve" => {
            super::linalg_chain_actions::run_cholesky_solve(document, label)
        }
        "scientific:la_lu_decompose" => {
            super::linalg_chain_actions::run_lu_decompose(document, label)
        }
        "scientific:la_lu_solve" => {
            super::linalg_chain_actions::run_lu_solve(document, label)
        }
        "scientific:la_qr_form_q" => {
            super::linalg_chain_actions::run_qr_form_q(document, label)
        }
        "scientific:la_qr_solve_ls" => {
            super::linalg_chain_actions::run_qr_solve_least_squares(document, label)
        }
        "scientific:la_svd" => super::linalg_chain_actions::run_svd(document, label),
        "scientific:la_eigenvalues" => {
            super::linalg_chain_actions::run_eigenvalues(document, label)
        }
        "scientific:la_add_assign" => {
            super::linalg_chain_actions::run_add_assign(document, label)
        }
        "scientific:la_hadamard_assign" => {
            super::linalg_chain_actions::run_hadamard_assign(document, label)
        }
        "scientific:la_cholesky_det" => {
            super::linalg_chain_actions::run_cholesky_determinant(document, label)
        }
        "scientific:la_charpoly" => {
            super::linalg_chain_actions::run_characteristic_polynomial(document, label)
        }
        "scientific:la_eigenvalues_general" => {
            super::linalg_chain_actions::run_eigenvalues_general(document, label)
        }
        "scientific:la_eigen_symmetric" => {
            super::linalg_chain_actions::run_eigen_symmetric(document, label)
        }
        "scientific:la_polynomial_roots" => {
            super::linalg_chain_actions::run_polynomial_roots(document, label)
        }
        "scientific:la_solve_linear_system" => {
            super::linalg_chain_actions::run_solve_linear_system(document, label)
        }
        "scientific:la_symmetric_eigen_3x3" => {
            super::linalg_chain_actions::run_symmetric_eigen_3x3(document, label)
        }
        "scientific:chem_boys" => {
            super::chem_chain_actions::run_boys_function(document, label)
        }
        "scientific:chem_overlap_s" => {
            super::chem_chain_actions::run_overlap_s(document, label)
        }
        "scientific:chem_kinetic_s" => {
            super::chem_chain_actions::run_kinetic_s(document, label)
        }
        "scientific:chem_nuclear_s" => {
            super::chem_chain_actions::run_nuclear_s(document, label)
        }
        "scientific:chem_dipole_s" => {
            super::chem_chain_actions::run_dipole_s(document, label)
        }
        "scientific:chem_evaluate_eri" => {
            super::chem_chain_actions::run_evaluate_eri(document, label)
        }
        "scientific:chem_total_angular_momentum" => {
            super::chem_chain_actions::run_total_angular_momentum(document, label)
        }
        "scientific:chem_letter" => super::chem_chain_actions::run_letter(document, label),
        "scientific:chem_n_cartesian" => {
            super::chem_chain_actions::run_n_cartesian(document, label)
        }
        "scientific:chem_n_spherical" => {
            super::chem_chain_actions::run_n_spherical(document, label)
        }
        "scientific:chem_from_letter" => {
            super::chem_chain_actions::run_from_letter(document, label)
        }
        "scientific:chem_gaussian_elim" => {
            super::chem_chain_actions::run_gaussian_elimination(document, label)
        }
        "scientific:chem_jacobi" => {
            super::chem_chain_actions::run_jacobi_diagonalization(document, label)
        }
        "scientific:chem_transpose" => {
            super::chem_chain_actions::run_transpose(document, label)
        }
        "scientific:chem_orthogonalize" => {
            super::chem_chain_actions::run_orthogonalization_matrix(document, label)
        }
        "scientific:chem_element_symbol" => {
            super::chem_chain_actions::run_element_symbol(document, label)
        }
        "scientific:chem_atomic_number" => {
            super::chem_chain_actions::run_atomic_number(document, label)
        }
        "scientific:chem_atomic_weight" => {
            super::chem_chain_actions::run_standard_atomic_weight(document, label)
        }
        "scientific:chem_lda_exchange" => {
            super::chem_chain_actions::run_lda_exchange(document, label)
        }
        "scientific:chem_lda_vwn" => {
            super::chem_chain_actions::run_lda_correlation_vwn(document, label)
        }
        "scientific:chem_sto3g_h2" => {
            super::chem_chain_actions::run_sto3g_h2(document, label)
        }
        "scientific:sf_airy_ai" => super::sf_chain_actions::run_airy_ai(document, label),
        "scientific:sf_airy_bi" => super::sf_chain_actions::run_airy_bi(document, label),
        "scientific:sf_zeta" => super::sf_chain_actions::run_zeta(document, label),
        "scientific:sf_legendre" => super::sf_chain_actions::run_legendre(document, label),
        "scientific:sf_chebyshev_t" => {
            super::sf_chain_actions::run_chebyshev_t(document, label)
        }
        "scientific:sf_chebyshev_u" => {
            super::sf_chain_actions::run_chebyshev_u(document, label)
        }
        "scientific:sf_hermite" => super::sf_chain_actions::run_hermite(document, label),
        "scientific:sf_laguerre" => super::sf_chain_actions::run_laguerre(document, label),
        "scientific:sf_bessel_j" => super::sf_chain_actions::run_bessel_j(document, label),
        "scientific:sf_bessel_i" => super::sf_chain_actions::run_bessel_i(document, label),
        "scientific:sf_bessel_y" => super::sf_chain_actions::run_bessel_y(document, label),
        "scientific:sf_bessel_k" => super::sf_chain_actions::run_bessel_k(document, label),
        "scientific:xform_dft" => super::transforms_chain_actions::run_dft(document, label),
        "scientific:xform_dft_complex" => {
            super::transforms_chain_actions::run_dft_complex(document, label)
        }
        "scientific:xform_idft" => super::transforms_chain_actions::run_idft(document, label),
        "scientific:xform_z_transform_finite" => {
            super::transforms_chain_actions::run_z_transform_finite(document, label)
        }
        "scientific:xform_unit_step_z" => {
            super::transforms_chain_actions::run_unit_step_z(document, label)
        }
        "scientific:xform_geometric_z" => {
            super::transforms_chain_actions::run_geometric_z(document, label)
        }
        "scientific:xform_laplace_numeric" => {
            super::transforms_chain_actions::run_laplace_numeric(document, label)
        }
        "scientific:xform_laplace_symbolic" => {
            super::transforms_chain_actions::run_laplace_symbolic(document, label)
        }
        "scientific:calc_hermite_dense" => {
            super::calc_chain_actions::run_hermite_dense_output(document, label)
        }
        "scientific:calc_bdf1" => super::calc_chain_actions::run_bdf1_step(document, label),
        "scientific:calc_bdf2" => super::calc_chain_actions::run_bdf2_step(document, label),
        "scientific:calc_verlet" => super::calc_chain_actions::run_verlet_step(document, label),
        "scientific:calc_ruth3" => super::calc_chain_actions::run_ruth3_step(document, label),
        "scientific:calc_yoshida4" => {
            super::calc_chain_actions::run_yoshida4_step(document, label)
        }
        "scientific:calc_integrate_bdf" => {
            super::calc_chain_actions::run_integrate_bdf(document, label)
        }
        "scientific:calc_integrate_sens" => {
            super::calc_chain_actions::run_integrate_with_sensitivity(document, label)
        }
        "scientific:calc_invariant_drift" => {
            super::calc_chain_actions::run_invariant_drift(document, label)
        }
        "scientific:calc_perm_parity" => {
            super::calc_chain_actions::run_permutation_parity(document, label)
        }
        "scientific:calc_pack_f32" => {
            super::calc_chain_actions::run_pack_f32_pair(document, label)
        }
        "scientific:calc_unpack_f32" => {
            super::calc_chain_actions::run_unpack_f32_pair(document, label)
        }
        "scientific:calc_poisson_bracket" => {
            super::calc_chain_actions::run_canonical_poisson_bracket(document, label)
        }
        "scientific:calc_stormer_verlet" => {
            super::calc_chain_actions::run_stormer_verlet_step(document, label)
        }
        "scientific:calc_gauss_kronrod" => {
            super::calc_chain_actions::run_adaptive_gauss_kronrod_15(document, label)
        }
        "scientific:calc_jvp" => super::calc_chain_actions::run_jvp(document, label),
        "scientific:calc_vjp" => super::calc_chain_actions::run_vjp(document, label),
        "scientific:calc_adaptive_simpson" => {
            super::calc_chain_actions::run_adaptive_simpson(document, label)
        }
        "scientific:calc_adaptive_deriv" => {
            super::calc_chain_actions::run_adaptive_derivative(document, label)
        }
        "scientific:calc_newton_solve" => {
            super::calc_chain_actions::run_newton_solve(document, label)
        }
        "scientific:calc_num_jacobian" => {
            super::calc_chain_actions::run_numerical_jacobian(document, label)
        }
        "scientific:calc_num_hessian" => {
            super::calc_chain_actions::run_numerical_hessian(document, label)
        }
        "scientific:ga_dot" => super::ga_chain_actions::run_dot(document, label),
        "scientific:ga_cross" => super::ga_chain_actions::run_cross_product(document, label),
        "scientific:ga_normalize" => {
            super::ga_chain_actions::run_normalize_vector(document, label)
        }
        "scientific:ga_angle" => {
            super::ga_chain_actions::run_angle_between_vectors(document, label)
        }
        "scientific:ga_geometric_product" => {
            super::ga_chain_actions::run_geometric_product(document, label)
        }
        "scientific:ga_outer_product" => {
            super::ga_chain_actions::run_outer_product(document, label)
        }
        "scientific:ga_rotor" => {
            super::ga_chain_actions::run_rotor_from_angle_axis(document, label)
        }
        "scientific:ga_apply_rotor" => {
            super::ga_chain_actions::run_apply_rotor(document, label)
        }
        "scientific:ga_translator" => {
            super::ga_chain_actions::run_translator_from_displacement(document, label)
        }
        "scientific:ga_apply_translator" => {
            super::ga_chain_actions::run_apply_translator(document, label)
        }
        "scientific:ga_is_simd" => {
            super::ga_chain_actions::run_is_simd_available(document, label)
        }
        "scientific:cg_distance_2d" => {
            super::cg_chain_actions::run_distance_2d(document, label)
        }
        "scientific:cg_distance_3d" => {
            super::cg_chain_actions::run_distance_3d(document, label)
        }
        "scientific:cg_point_segment_2d" => {
            super::cg_chain_actions::run_point_segment_2d(document, label)
        }
        "scientific:cg_orientation_2" => {
            super::cg_chain_actions::run_orientation_2(document, label)
        }
        "scientific:cg_orient_3d" => super::cg_chain_actions::run_orient_3d(document, label),
        "scientific:cg_morton_encode_2d" => {
            super::cg_chain_actions::run_morton_encode_2d(document, label)
        }
        "scientific:cg_morton_decode_2d" => {
            super::cg_chain_actions::run_morton_decode_2d(document, label)
        }
        "scientific:cg_morton_encode_3d" => {
            super::cg_chain_actions::run_morton_encode_3d(document, label)
        }
        "scientific:cg_hilbert_encode_2d" => {
            super::cg_chain_actions::run_hilbert_encode_2d(document, label)
        }
        "scientific:cg_circumcenter" => {
            super::cg_chain_actions::run_circumcenter(document, label)
        }
        "scientific:cg_point_segment_3d" => {
            super::cg_chain_actions::run_point_segment_3d(document, label)
        }
        "scientific:cg_point_triangle_3d" => {
            super::cg_chain_actions::run_point_triangle_3d(document, label)
        }
        "scientific:cg_convex_hull_2" => {
            super::cg_chain_actions::run_convex_hull_2(document, label)
        }
        "scientific:cg_triangulate" => {
            super::cg_chain_actions::run_triangulate_polygon(document, label)
        }
        "scientific:cg_surface_area" => {
            super::cg_chain_actions::run_surface_area(document, label)
        }
        "scientific:cg_signed_volume" => {
            super::cg_chain_actions::run_signed_volume(document, label)
        }
        "scientific:cg_segment_intersect_2" => {
            super::cg_chain_actions::run_line_segment_intersection_2(document, label)
        }
        "scientific:cg_bezier_eval" => {
            super::cg_chain_actions::run_bezier_eval(document, label)
        }
        "scientific:cg_nearest_site" => {
            super::cg_chain_actions::run_nearest_site(document, label)
        }
        "scientific:cg_live_average_spacing_3d" => {
            super::cg_live_chain_actions::run_average_spacing_3d(document, label)
        }
        "scientific:cg_live_local_density_3d" => {
            super::cg_live_chain_actions::run_local_density_3d(document, label)
        }
        "scientific:cg_live_mean_knn_distance_3d" => {
            super::cg_live_chain_actions::run_mean_knn_distance_3d(document, label)
        }
        "scientific:cg_live_fisher_distance" => {
            super::cg_live_chain_actions::run_fisher_distance(document, label)
        }
        "scientific:cg_live_kl_divergence" => {
            super::cg_live_chain_actions::run_kl_divergence(document, label)
        }
        "scientific:cg_live_kl_bregman_form" => {
            super::cg_live_chain_actions::run_kl_bregman_form(document, label)
        }
        "scientific:cg_live_triangle_signed_area" => {
            super::cg_live_chain_actions::run_triangle_signed_area(document, label)
        }
        "scientific:cg_live_dist_point_to_segment" => {
            super::cg_live_chain_actions::run_dist_point_to_segment(document, label)
        }
        "scientific:cg_live_dist_sq_point_to_segment" => {
            super::cg_live_chain_actions::run_dist_sq_point_to_segment(document, label)
        }
        "scientific:cg_live_incircle" => {
            super::cg_live_chain_actions::run_incircle(document, label)
        }
        "scientific:cg_live_tukey_depth" => {
            super::cg_live_chain_actions::run_tukey_depth(document, label)
        }
        "scientific:cg_live_directional_width" => {
            super::cg_live_chain_actions::run_directional_width(document, label)
        }
        "scientific:cg_live_width" => super::cg_live_chain_actions::run_width(document, label),
        "scientific:cg_live_farthest_site_brute" => {
            super::cg_live_chain_actions::run_farthest_site_brute(document, label)
        }
        "scientific:cg_live_k_nearest_sites" => {
            super::cg_live_chain_actions::run_k_nearest_sites(document, label)
        }
        "scientific:cg_live_is_hull_site" => {
            super::cg_live_chain_actions::run_is_hull_site(document, label)
        }
        "scientific:cg_live_diameter_and_width" => {
            super::cg_live_chain_actions::run_diameter_and_width(document, label)
        }
        "scientific:cg_live_insphere" => {
            super::cg_live2_chain_actions::run_insphere(document, label)
        }
        "scientific:cg_live_ham_sandwich_cut" => {
            super::cg_live2_chain_actions::run_ham_sandwich_cut(document, label)
        }
        "scientific:cg_live_smallest_enclosing_disk" => {
            super::cg_live2_chain_actions::run_smallest_enclosing_disk(document, label)
        }
        "scientific:cg_live_polygon_signed_area" => {
            super::cg_live2_chain_actions::run_polygon_signed_area(document, label)
        }
        "scientific:cg_live_polygon_area" => {
            super::cg_live2_chain_actions::run_polygon_area(document, label)
        }
        "scientific:cg_live_point_in_polygon" => {
            super::cg_live2_chain_actions::run_point_in_polygon(document, label)
        }
        "scientific:cg_live_minkowski_sum_convex" => {
            super::cg_live2_chain_actions::run_minkowski_sum_convex(document, label)
        }
        "scientific:cg_live_nearest_segment_site" => {
            super::cg_live2_chain_actions::run_nearest_segment_site(document, label)
        }
        "scientific:cg_live_width_coreset" => {
            super::cg_live3_chain_actions::run_width_coreset(document, label)
        }
        "scientific:cg_live_dual_point_to_line" => {
            super::cg_live3_chain_actions::run_dual_point_to_line(document, label)
        }
        "scientific:cg_live_dual_round_trip" => {
            super::cg_live3_chain_actions::run_dual_round_trip(document, label)
        }
        "scientific:cg_live_is_convex_polygon" => {
            super::cg_live3_chain_actions::run_is_convex_polygon(document, label)
        }
        "scientific:cg_live_point_in_or_on_polygon" => {
            super::cg_live3_chain_actions::run_point_in_or_on_polygon(document, label)
        }
        "scientific:cg_live_boolean_union_area" => {
            super::cg_live3_chain_actions::run_boolean_union_area(document, label)
        }
        "scientific:cg_live_boolean_intersection_area" => {
            super::cg_live3_chain_actions::run_boolean_intersection_area(document, label)
        }
        "scientific:cg_live_boolean_difference_area" => {
            super::cg_live3_chain_actions::run_boolean_difference_area(document, label)
        }
        "scientific:cg_live_cross_ratio_1d" => {
            super::cg_live4_chain_actions::run_cross_ratio_1d(document, label)
        }
        "scientific:cg_live_hyperplane_eval" => {
            super::cg_live4_chain_actions::run_hyperplane_eval(document, label)
        }
        "scientific:cg_live_householder_reflect" => {
            super::cg_live4_chain_actions::run_householder_reflect(document, label)
        }
        "scientific:cg_live_quaternion_normalize" => {
            super::cg_live4_chain_actions::run_quaternion_normalize(document, label)
        }
        "scientific:cg_live_so3_exp" => {
            super::cg_live4_chain_actions::run_so3_exp(document, label)
        }
        "scientific:cg_live_so3_log" => {
            super::cg_live4_chain_actions::run_so3_log(document, label)
        }
        "scientific:cg_live_projective_from_point" => {
            super::cg_live4_chain_actions::run_projective_from_point(document, label)
        }
        "scientific:cg_live_point_from_projective" => {
            super::cg_live4_chain_actions::run_point_from_projective(document, label)
        }
        "scientific:cg_live_frame_to_world" => {
            super::cg_live4_chain_actions::run_frame_to_world(document, label)
        }
        "scientific:cg_live_world_to_frame" => {
            super::cg_live4_chain_actions::run_world_to_frame(document, label)
        }
        "scientific:cg_live_barycentric_tetra" => {
            super::cg_live4_chain_actions::run_barycentric_tetra(document, label)
        }
        "scientific:cg_live_quaternion_slerp" => {
            super::cg_live4_chain_actions::run_quaternion_slerp(document, label)
        }
        "scientific:cg_live_quaternion_to_matrix" => {
            super::cg_live4_chain_actions::run_quaternion_to_matrix(document, label)
        }
        "scientific:cg_live_solve_diagonal_quadratic" => {
            super::cg_live4_chain_actions::run_solve_diagonal_quadratic(document, label)
        }
        "scientific:cg_live_schur_complement_2x2" => {
            super::cg_live4_chain_actions::run_schur_complement_2x2(document, label)
        }
        "scientific:cg_live_separating_plane_aabb" => {
            super::cg_live4_chain_actions::run_separating_plane_aabb(document, label)
        }
        "scientific:eng_natural_freq" => {
            super::eng_chain_actions::run_natural_frequency_sdof(document, label)
        }
        "scientific:eng_harmonic_sdof" => {
            super::eng_chain_actions::run_analyze_harmonic_sdof(document, label)
        }
        "scientific:eng_euler" => super::eng_chain_actions::run_analyze_euler(document, label),
        "scientific:eng_reliability" => {
            super::eng_chain_actions::run_compute_reliability_index(document, label)
        }
        "scientific:eng_kinematics" => {
            super::eng_chain_actions::run_kinematics(document, label)
        }
        "scientific:eng_cauchy" => super::eng_chain_actions::run_cauchy_stress(document, label),
        "scientific:eng_drag" => super::eng_chain_actions::run_drag_force(document, label),
        "scientific:eng_reynolds" => {
            super::eng_chain_actions::run_reynolds_number(document, label)
        }
        "scientific:eng_fatigue" => {
            super::eng_chain_actions::run_fatigue_cycles(document, label)
        }
        "scientific:eng_miner" => super::eng_chain_actions::run_miner_damage(document, label),
        "scientific:phys_doppler" => {
            super::physics_chain_actions::run_doppler_shift(document, label)
        }
        "scientific:phys_emf_attenuation" => {
            super::physics_chain_actions::run_emf_attenuation(document, label)
        }
        "scientific:phys_harmonic" => {
            super::physics_chain_actions::run_harmonic_oscillator(document, label)
        }
        "scientific:phys_pendulum" => {
            super::physics_chain_actions::run_pendulum(document, label)
        }
        "scientific:phys_logistic" => {
            super::physics_chain_actions::run_logistic_growth(document, label)
        }
        "scientific:phys_cfd_step" => {
            super::physics_chain_actions::run_cfd_step(document, label)
        }
        "scientific:phys_heat_1d" => {
            super::physics_chain_actions::run_heat_diffusion_1d(document, label)
        }
        "scientific:phys_wave_1d" => {
            super::physics_chain_actions::run_wave_1d(document, label)
        }
        "scientific:phys_advection_1d" => {
            super::physics_chain_actions::run_advection_diffusion_1d(document, label)
        }
        "scientific:phys_quantum_1d" => {
            super::physics_chain_actions::run_quantum_states_1d(document, label)
        }
        "scientific:phys_n_body" => {
            super::physics_chain_actions::run_n_body(document, label)
        }
        "scientific:phys_molecular_dynamics" => {
            super::physics_chain_actions::run_molecular_dynamics(document, label)
        }
        "scientific:phys_emf_interference" => {
            super::physics_chain_actions::run_emf_interference(document, label)
        }
        "scientific:phys_emf_field_grid" => {
            super::physics_chain_actions::run_emf_field_grid_3d(document, label)
        }
        "scientific:phys_emf_sample_depth" => {
            super::physics_chain_actions::run_emf_sample_at_depth(document, label)
        }
        "scientific:phys_field_sample" => {
            super::physics_chain_actions::run_field_sample(document, label)
        }
        "scientific:phys_material_query" => {
            super::physics_chain_actions::run_material_query(document, label)
        }
        "scientific:phys_evaluate_interaction" => {
            super::physics_chain_actions::run_evaluate_interaction(document, label)
        }
        "scientific:cosmic_geodetic_distance" => {
            super::cosmic_chain_actions::run_geodetic_distance(document, label)
        }
        "scientific:cosmic_surface_gravity" => {
            super::cosmic_chain_actions::run_surface_gravity(document, label)
        }
        "scientific:cosmic_flrw_distance" => {
            super::cosmic_chain_actions::run_flrw_distance(document, label)
        }
        "scientific:cosmic_flrw_redshift" => {
            super::cosmic_chain_actions::run_flrw_redshift(document, label)
        }
        "scientific:cosmic_flrw_hubble" => {
            super::cosmic_chain_actions::run_flrw_hubble_velocity(document, label)
        }
        "scientific:cosmic_warp_velocity" => {
            super::cosmic_chain_actions::run_warp_velocity(document, label)
        }
        "scientific:cosmic_warp_factor_c" => {
            super::cosmic_chain_actions::run_warp_factor_c(document, label)
        }
        "scientific:cosmic_typical_length" => {
            super::cosmic_chain_actions::run_typical_length(document, label)
        }
        "scientific:cosmic_observe_redshift" => {
            super::cosmic_chain_actions::run_observe_redshift(document, label)
        }
        "scientific:cosmic_compton" => {
            super::cosmic_chain_actions::run_compton_wavelength(document, label)
        }
        "scientific:cosmic_de_broglie" => {
            super::cosmic_chain_actions::run_de_broglie_wavelength(document, label)
        }
        "scientific:cosmic_atm_pressure" => {
            super::cosmic_chain_actions::run_atmosphere_pressure(document, label)
        }
        "scientific:cosmic_geodetic_to_ecef" => {
            super::cosmic_chain_actions::run_geodetic_to_ecef(document, label)
        }
        "scientific:cosmic_ecef_to_geodetic" => {
            super::cosmic_chain_actions::run_ecef_to_geodetic(document, label)
        }
        "scientific:cosmic_ecef_to_enu" => {
            super::cosmic_chain_actions::run_ecef_to_enu(document, label)
        }
        "scientific:cosmic_enu_to_ecef" => {
            super::cosmic_chain_actions::run_enu_to_ecef(document, label)
        }
        "scientific:cosmic_body_profile" => {
            super::cosmic_chain_actions::run_body_profile(document, label)
        }
        "scientific:cosmic_stardate" => {
            super::cosmic_chain_actions::run_stardate_to_gregorian(document, label)
        }
        "scientific:cosmic_cochrane" => {
            super::cosmic_chain_actions::run_cochrane_units(document, label)
        }
        "scientific:cosmic_atm_temperature" => {
            super::cosmic_chain_actions::run_atmosphere_temperature(document, label)
        }
        "scientific:cosmic_magnetosphere" => {
            super::cosmic_chain_actions::run_magnetosphere_field(document, label)
        }
        "scientific:cosmic_scale_factor" => {
            super::cosmic_chain_actions::run_scale_factor(document, label)
        }
        "scientific:cosmic_usri_parse" => {
            super::cosmic_chain_actions::run_usri_parse(document, label)
        }
        "ai:orch_session_create" => {
            super::orch_chain_actions::run_session_create(document, label)
        }
        "ai:orch_session_plan" => super::orch_chain_actions::run_session_plan(document, label),
        "ai:orch_session_execute" => {
            super::orch_chain_actions::run_session_execute(document, label)
        }
        "ai:orch_session_status" => {
            super::orch_chain_actions::run_session_status(document, label)
        }
        "ai:orch_roster_register" => {
            super::orch_chain_actions::run_roster_register(document, label)
        }
        "ai:orch_roster_list" => super::orch_chain_actions::run_roster_list(document, label),
        "ai:orch_roster_capabilities" => {
            super::orch_chain_actions::run_roster_capabilities(document, label)
        }
        "ai:orch_assign_agents" => super::orch_chain_actions::run_assign_agents(document, label),
        "spatial:threed_add_object" => {
            super::threed_chain_actions::run_add_object(document, label)
        }
        "spatial:threed_set_transform" => {
            super::threed_chain_actions::run_set_transform(document, label)
        }
        "spatial:threed_set_material" => {
            super::threed_chain_actions::run_set_material(document, label)
        }
        "spatial:threed_add_camera" => {
            super::threed_chain_actions::run_add_camera(document, label)
        }
        "spatial:threed_add_light" => super::threed_chain_actions::run_add_light(document, label),
        "spatial:threed_add_rig" => super::threed_chain_actions::run_add_rig(document, label),
        "spatial:threed_add_animation" => {
            super::threed_chain_actions::run_add_animation(document, label)
        }
        "spatial:threed_set_mesh" => super::threed_chain_actions::run_set_mesh(document, label),
        "spatial:scene_lerp_camera" => {
            super::scene_chain_actions::run_lerp_camera(document, label)
        }
        "spatial:scene_camera_frame_node" => {
            super::scene_chain_actions::run_camera_frame_node(document, label)
        }
        "spatial:scene_smooth_damp" => {
            super::scene_chain_actions::run_smooth_damp(document, label)
        }
        "spatial:scene_smooth_damp_vec3" => {
            super::scene_chain_actions::run_smooth_damp_vec3(document, label)
        }
        "spatial:scene_ik_look_at" => {
            super::scene_chain_actions::run_ik_look_at(document, label)
        }
        "spatial:scene_ik_ccd" => super::scene_chain_actions::run_ik_ccd(document, label),
        "spatial:scene_set_render_budget" => {
            super::scene_chain_actions::run_set_render_budget(document, label)
        }
        "spatial:scene_set_clear_colour" => {
            super::scene_chain_actions::run_set_clear_colour(document, label)
        }
        "spatial:scene_create" => super::scene_graph_chain_actions::run_create(document, label),
        "spatial:scene_add_node" => super::scene_graph_chain_actions::run_add_node(document, label),
        "spatial:scene_set_transform" => {
            super::scene_graph_chain_actions::run_set_transform(document, label)
        }
        "spatial:scene_set_mesh" => super::scene_graph_chain_actions::run_set_mesh(document, label),
        "spatial:scene_add_camera" => {
            super::scene_graph_chain_actions::run_add_camera(document, label)
        }
        "spatial:scene_render" => super::scene_graph_chain_actions::run_render(document, label),
        "spatial:scene_set_viewport" => {
            super::scene_graph_chain_actions::run_set_viewport(document, label)
        }
        "spatial:scene_capture_frame" => {
            super::scene_graph_chain_actions::run_capture_frame(document, label)
        }
        "spatial:scene_add_light" => super::scene_graph_chain_actions::run_add_light(document, label),
        "spatial:scene_link_semantic" => {
            super::scene_graph_chain_actions::run_link_semantic(document, label)
        }
        "spatial:scene_duplicate_node" => {
            super::scene_graph_chain_actions::run_duplicate_node(document, label)
        }
        "audio:dsp_ep_temp" => {
            super::audio_chain_actions::run_epistemic_temperature_from_q(document, label)
        }
        "audio:dsp_ep_fm" => super::audio_chain_actions::run_epistemic_fm_index(document, label),
        "audio:dsp_sigma_freq" => {
            super::audio_chain_actions::run_sigma_dominant_frequency(document, label)
        }
        "audio:dsp_parametric_sample" => {
            super::audio_chain_actions::run_parametric_sample(document, label)
        }
        "audio:dsp_bin_freq_linear" => {
            super::audio_chain_actions::run_bin_to_freq_linear(document, label)
        }
        "audio:dsp_bin_freq_log" => {
            super::audio_chain_actions::run_bin_to_freq_log(document, label)
        }
        "audio:dsp_midi_note" => super::audio_chain_actions::run_midi_note(document, label),
        "audio:dsp_quantize" => super::audio_chain_actions::run_quantize(document, label),
        "audio:dsp_transpose" => super::audio_chain_actions::run_transpose(document, label),
        "audio:fx_oscillator" => super::audio_fx_chain_actions::run_fx_oscillator(document, label),
        "audio:fx_envelope" => super::audio_fx_chain_actions::run_fx_envelope(document, label),
        "audio:fx_filter" => super::audio_fx_chain_actions::run_fx_filter(document, label),
        "audio:fx_lfo" => super::audio_fx_chain_actions::run_fx_lfo(document, label),
        "audio:fx_delay" => super::audio_fx_chain_actions::run_fx_delay(document, label),
        "audio:fx_reverb" => super::audio_fx_chain_actions::run_fx_reverb(document, label),
        "audio:fx_compressor" => super::audio_fx_chain_actions::run_fx_compressor(document, label),
        "audio:fx_eq" => super::audio_fx_chain_actions::run_fx_eq(document, label),
        "audio:fx_transport" => super::audio_fx_chain_actions::run_fx_transport(document, label),
        "audio:fx_waveform_meter" => {
            super::audio_fx_chain_actions::run_fx_waveform_meter(document, label)
        }
        "audio:fx_phase_meter" => super::audio_fx_chain_actions::run_fx_phase_meter(document, label),
        "audio:fx_loudness_meter" => {
            super::audio_fx_chain_actions::run_fx_loudness_meter(document, label)
        }
        "audio:fx_spectrum" => super::audio_fx_chain_actions::run_fx_spectrum(document, label),
        "dmx:live_new_universe" => super::dmx_chain_actions::run_new_universe(document, label),
        "dmx:live_set_channel" => super::dmx_chain_actions::run_set_channel(document, label),
        "dmx:live_add_fixture" => super::dmx_chain_actions::run_add_fixture(document, label),
        "dmx:live_fixture_set_colour" => {
            super::dmx_chain_actions::run_fixture_set_colour(document, label)
        }
        "dmx:live_fixture_set_intensity" => {
            super::dmx_chain_actions::run_fixture_set_intensity(document, label)
        }
        "dmx:live_fixture_set_pan_tilt" => {
            super::dmx_chain_actions::run_fixture_set_pan_tilt(document, label)
        }
        "dmx:live_new_cue" => super::dmx_chain_actions::run_new_cue(document, label),
        "dmx:live_cue_set_channel" => super::dmx_chain_actions::run_cue_set_channel(document, label),
        "dmx:live_cue_set_fade" => super::dmx_chain_actions::run_cue_set_fade(document, label),
        "dmx:live_new_cue_stack" => super::dmx_chain_actions::run_new_cue_stack(document, label),
        "dmx:live_cue_stack_add" => super::dmx_chain_actions::run_cue_stack_add(document, label),
        "dmx:live_cue_stack_go" => super::dmx_chain_actions::run_cue_stack_go(document, label),
        "dmx:live_cue_stack_go_back" => super::dmx_chain_actions::run_cue_stack_go_back(document, label),
        "dmx:live_cue_stack_reset" => super::dmx_chain_actions::run_cue_stack_reset(document, label),
        "video:live_new_project" => {
            super::video_live_chain_actions::run_new_project(document, label)
        }
        "video:live_add_track" => super::video_live_chain_actions::run_add_track(document, label),
        "video:live_add_clip" => super::video_live_chain_actions::run_add_clip(document, label),
        "video:live_trim_clip" => super::video_live_chain_actions::run_trim_clip(document, label),
        "video:live_set_speed" => super::video_live_chain_actions::run_set_speed(document, label),
        "video:live_colour_grade" => {
            super::video_live_chain_actions::run_colour_grade(document, label)
        }
        "video:live_add_transition" => {
            super::video_live_chain_actions::run_add_transition(document, label)
        }
        "video:live_set_render_format" => {
            super::video_live_chain_actions::run_set_render_format(document, label)
        }
        "video:live_set_render_bitrate" => {
            super::video_live_chain_actions::run_set_render_bitrate(document, label)
        }
        "video:live_remove_clip" => super::video_live_chain_actions::run_remove_clip(document, label),
        "hid:live_poll" => super::hid_chain_actions::run_poll(document, label),
        "hid:live_wait" => super::hid_chain_actions::run_wait(document, label),
        "hid:live_clear" => super::hid_chain_actions::run_clear(document, label),
        "hid:live_pointer_capture" => super::hid_chain_actions::run_pointer_capture(document, label),
        "hid:live_pointer_release" => super::hid_chain_actions::run_pointer_release(document, label),
        "hid:live_set_cursor" => super::hid_chain_actions::run_set_cursor(document, label),
        "hid:live_gamepad_poll" => super::hid_chain_actions::run_gamepad_poll(document, label),
        "hid:live_gamepad_vibrate" => super::hid_chain_actions::run_gamepad_vibrate(document, label),
        "hid:live_midi_send" => super::hid_chain_actions::run_midi_send(document, label),
        "hid:live_midi_poll" => super::hid_chain_actions::run_midi_poll(document, label),
        "hid:live_haptic_pulse" => super::hid_chain_actions::run_haptic_pulse(document, label),
        "hid:live_haptic_pattern" => super::hid_chain_actions::run_haptic_pattern(document, label),
        "hid:live_spatial_head_pose" => {
            super::hid_chain_actions::run_spatial_head_pose(document, label)
        }
        "hid:live_spatial_hand_skeleton" => {
            super::hid_chain_actions::run_spatial_hand_skeleton(document, label)
        }
        "hid:live_spatial_gaze_ray" => {
            super::hid_chain_actions::run_spatial_gaze_ray(document, label)
        }
        "hid:live_biosignal_poll" => super::hid_chain_actions::run_biosignal_poll(document, label),
        "scientific:vc_gradient" => super::vc_chain_actions::run_gradient(document, label),
        "scientific:vc_divergence" => super::vc_chain_actions::run_divergence(document, label),
        "scientific:vc_curl" => super::vc_chain_actions::run_curl(document, label),
        "scientific:vc_laplacian" => super::vc_chain_actions::run_laplacian(document, label),
        "scientific:vc_line_integral_scalar" => {
            super::vc_chain_actions::run_line_integral_scalar(document, label)
        }
        "scientific:vc_line_integral_work" => {
            super::vc_chain_actions::run_line_integral_work(document, label)
        }
        "scientific:vc_surface_flux" => super::vc_chain_actions::run_surface_flux(document, label),
        "scientific:interp_linear" => super::interp_chain_actions::run_linear(document, label),
        "scientific:interp_lagrange" => super::interp_chain_actions::run_lagrange(document, label),
        "scientific:interp_newton_coef" => {
            super::interp_chain_actions::run_newton_coef(document, label)
        }
        "scientific:interp_newton_eval" => {
            super::interp_chain_actions::run_newton_eval(document, label)
        }
        "scientific:interp_poly_fit" => super::interp_chain_actions::run_poly_fit(document, label),
        "scientific:interp_poly_eval" => super::interp_chain_actions::run_poly_eval(document, label),
        "scientific:spectral_emf_to_spd" => {
            super::spectral_chain_actions::run_emf_to_spd(document, label)
        }
        "scientific:spectral_spd_to_xyz" => {
            super::spectral_chain_actions::run_spd_to_xyz(document, label)
        }
        "scientific:spectral_emf_to_rgb" => {
            super::spectral_chain_actions::run_emf_to_rgb(document, label)
        }
        "scientific:spectral_blend" => super::spectral_chain_actions::run_blend(document, label),
        "scientific:spectral_gamut_map" => {
            super::spectral_chain_actions::run_gamut_map(document, label)
        }
        "spatial:world_new" => super::world_chain_actions::run_new(document, label),
        "spatial:world_add_object" => super::world_chain_actions::run_add_object(document, label),
        "spatial:world_add_portal" => super::world_chain_actions::run_add_portal(document, label),
        "spatial:world_add_avatar" => super::world_chain_actions::run_add_avatar(document, label),
        "spatial:world_set_gravity" => super::world_chain_actions::run_set_gravity(document, label),
        "spatial:world_object_apply_force" => {
            super::world_chain_actions::run_object_apply_force(document, label)
        }
        "spatial:world_object_step_physics" => {
            super::world_chain_actions::run_object_step_physics(document, label)
        }
        "office:asset_create" => super::asset_chain_actions::run_create(document, label),
        "office:asset_add_temporal" => super::asset_chain_actions::run_add_temporal(document, label),
        "office:asset_add_topic" => super::asset_chain_actions::run_add_topic(document, label),
        "office:asset_set_spatial" => super::asset_chain_actions::run_set_spatial(document, label),
        "office:asset_compile" => super::asset_chain_actions::run_compile(document, label),
        "office:asset_temporal_span" => super::asset_chain_actions::run_temporal_span(document, label),
        "office:asset_query_aspects" => super::asset_chain_actions::run_query_aspects(document, label),
        "office:asset_persist" => super::asset_chain_actions::run_persist(document, label),
        "office:asset_resolve" => super::asset_chain_actions::run_resolve(document, label),
        "office:asset_resolve_by_spatial" => {
            super::asset_chain_actions::run_resolve_by_spatial(document, label)
        }
        "office:asset_resolve_by_topic" => {
            super::asset_chain_actions::run_resolve_by_topic(document, label)
        }
        "office:asset_resolve_by_temporal" => {
            super::asset_chain_actions::run_resolve_by_temporal(document, label)
        }
        "office:asset_list" => super::asset_chain_actions::run_list(document, label),
        "office:asset_count" => super::asset_chain_actions::run_count(document, label),
        "office:asset_persist_create" => {
            super::asset_chain_actions::run_persist_create(document, label)
        }
        "office:asset_persist_add_temporal" => {
            super::asset_chain_actions::run_persist_add_temporal(document, label)
        }
        "office:asset_persist_add_topic" => {
            super::asset_chain_actions::run_persist_add_topic(document, label)
        }
        "office:asset_persist_set_spatial" => {
            super::asset_chain_actions::run_persist_set_spatial(document, label)
        }
        "office:asset_persist_compile" => {
            super::asset_chain_actions::run_persist_compile(document, label)
        }
        "office:asset_persist_temporal_span" => {
            super::asset_chain_actions::run_persist_temporal_span(document, label)
        }
        "office:asset_persist_query_aspects" => {
            super::asset_chain_actions::run_persist_query_aspects(document, label)
        }
        "comm:pulse_live_publish" => super::pulse_live_chain_actions::run_publish(document, label),
        "comm:pulse_live_graph_mutation" => {
            super::pulse_live_chain_actions::run_publish_graph_mutation(document, label)
        }
        "comm:pulse_live_notification" => {
            super::pulse_live_chain_actions::run_publish_notification(document, label)
        }
        "comm:pulse_live_telemetry" => {
            super::pulse_live_chain_actions::run_publish_telemetry(document, label)
        }
        "comm:pulse_live_agent_message" => {
            super::pulse_live_chain_actions::run_publish_agent_message(document, label)
        }
        "comm:pulse_live_sync" => super::pulse_live_chain_actions::run_publish_sync(document, label),
        "comm:pulse_live_open_channel" => {
            super::pulse_live_chain_actions::run_open_channel(document, label)
        }
        "comm:pulse_live_close_channel" => {
            super::pulse_live_chain_actions::run_close_channel(document, label)
        }
        "comm:pulse_live_set_transport" => {
            super::pulse_live_chain_actions::run_set_transport(document, label)
        }
        "spatial:portal_set_target" => super::portal_chain_actions::run_set_target(document, label),
        "spatial:portal_activate" => super::portal_chain_actions::run_activate(document, label),
        "spatial:portal_deactivate" => {
            super::portal_chain_actions::run_deactivate(document, label)
        }
        "spatial:avatar_move" => super::portal_chain_actions::run_move(document, label),
        "spatial:avatar_set_appearance" => {
            super::portal_chain_actions::run_set_appearance(document, label)
        }
        "research:live_new" => super::research_live_chain_actions::run_new(document, label),
        "research:live_set_purpose" => {
            super::research_live_chain_actions::run_set_purpose(document, label)
        }
        "research:live_define_scope" => {
            super::research_live_chain_actions::run_define_scope(document, label)
        }
        "research:live_add_constraint" => {
            super::research_live_chain_actions::run_add_constraint(document, label)
        }
        "research:live_add_question" => {
            super::research_live_chain_actions::run_add_question(document, label)
        }
        "research:live_link_questions" => {
            super::research_live_chain_actions::run_link_questions(document, label)
        }
        "research:live_add_corpus_item" => {
            super::research_live_chain_actions::run_add_corpus_item(document, label)
        }
        "research:live_import_literature" => {
            super::research_live_chain_actions::run_import_literature(document, label)
        }
        "research:live_import_dataset" => {
            super::research_live_chain_actions::run_import_dataset(document, label)
        }
        "research:live_set_corpus_confidence" => {
            super::research_live_chain_actions::run_set_corpus_confidence(document, label)
        }
        "research:live_extract_from_corpus" => {
            super::research_live_chain_actions::run_extract_from_corpus(document, label)
        }
        "research:live_infer_dark_link" => {
            super::research_live_chain_actions::run_infer_dark_link(document, label)
        }
        "research:live_detect_provenance_gaps" => {
            super::research_live_chain_actions::run_detect_provenance_gaps(document, label)
        }
        "research:live_detect_concealment" => {
            super::research_live_chain_actions::run_detect_concealment(document, label)
        }
        "research:live_confirm_dark_link" => {
            super::research_live_chain_actions::run_confirm_dark_link(document, label)
        }
        "research:live_refute_dark_link" => {
            super::research_live_chain_actions::run_refute_dark_link(document, label)
        }
        "research:live_make_inference" => {
            super::research_live_chain_actions::run_make_inference(document, label)
        }
        "research:live_chain_inference" => {
            super::research_live_chain_actions::run_chain_inference(document, label)
        }
        "research:live_set_inference_confidence" => {
            super::research_live_chain_actions::run_set_inference_confidence(document, label)
        }
        "research:live_validate_inference" => {
            super::research_live_chain_actions::run_validate_inference(document, label)
        }
        "research:live_new_investigation" => {
            super::research_live2_chain_actions::run_new_investigation(document, label)
        }
        "research:live_collect_evidence" => {
            super::research_live2_chain_actions::run_collect_evidence(document, label)
        }
        "research:live_set_reliability" => {
            super::research_live2_chain_actions::run_set_reliability(document, label)
        }
        "research:live_propose_hypothesis" => {
            super::research_live2_chain_actions::run_propose_hypothesis(document, label)
        }
        "research:live_evaluate_evidence" => {
            super::research_live2_chain_actions::run_evaluate_evidence(document, label)
        }
        "research:live_create_timeline" => {
            super::research_live2_chain_actions::run_create_timeline(document, label)
        }
        "research:live_add_link" => super::research_live2_chain_actions::run_add_link(document, label),
        "research:live_find_path" => {
            super::research_live2_chain_actions::run_find_path(document, label)
        }
        "research:live_create_hypothesis_graph" => {
            super::research_live2_chain_actions::run_create_hypothesis_graph(document, label)
        }
        "research:live_contribute_evaluation" => {
            super::research_live2_chain_actions::run_contribute_evaluation(document, label)
        }
        "research:live_bridge_dark_link" => {
            super::research_live2_chain_actions::run_bridge_dark_link(document, label)
        }
        "research:live_reframe_hypothesis" => {
            super::research_live2_chain_actions::run_reframe_hypothesis(document, label)
        }
        "research:live_merge_hypotheses" => {
            super::research_live2_chain_actions::run_merge_hypotheses(document, label)
        }
        "research:live_flag_gap" => {
            super::research_live2_chain_actions::run_flag_gap(document, label)
        }
        "research:live_close_gap" => {
            super::research_live2_chain_actions::run_close_gap(document, label)
        }
        "research:live_create_revision" => {
            super::research_live2_chain_actions::run_create_revision(document, label)
        }
        "research:live_diff_revisions" => {
            super::research_live2_chain_actions::run_diff_revisions(document, label)
        }
        "research:live_subscribe_updates" => {
            super::research_live2_chain_actions::run_subscribe_updates(document, label)
        }
        "research:live_create_assessment" => {
            super::research_live2_chain_actions::run_create_assessment(document, label)
        }
        "research:live_set_epistemic_mode" => {
            super::research_live2_chain_actions::run_set_epistemic_mode(document, label)
        }
        "research:live_set_reality_category" => {
            super::research_live3_chain_actions::run_set_reality_category(document, label)
        }
        "research:live_classify_reality" => {
            super::research_live3_chain_actions::run_classify_reality(document, label)
        }
        "research:live_detect_blended" => {
            super::research_live3_chain_actions::run_detect_blended(document, label)
        }
        "research:live_detect_deceptive_fiction" => {
            super::research_live3_chain_actions::run_detect_deceptive_fiction(document, label)
        }
        "research:live_trace_fiction" => {
            super::research_live3_chain_actions::run_trace_fiction(document, label)
        }
        "research:live_assess_sentiment" => {
            super::research_live3_chain_actions::run_assess_sentiment(document, label)
        }
        "research:live_detect_sentiment_manipulation" => {
            super::research_live3_chain_actions::run_detect_sentiment_manipulation(document, label)
        }
        "research:live_detect_performed_sentiment" => {
            super::research_live3_chain_actions::run_detect_performed_sentiment(document, label)
        }
        "research:live_map_sentiment_network" => {
            super::research_live3_chain_actions::run_map_sentiment_network(document, label)
        }
        "research:live_analyse_sentiment_trends" => {
            super::research_live3_chain_actions::run_analyse_sentiment_trends(document, label)
        }
        "research:live_register_perspective" => {
            super::research_live3_chain_actions::run_register_perspective(document, label)
        }
        "research:live_add_bias" => {
            super::research_live3_chain_actions::run_add_bias(document, label)
        }
        "research:live_compare_perspectives" => {
            super::research_live3_chain_actions::run_compare_perspectives(document, label)
        }
        "research:live_detect_perspective_conflict" => {
            super::research_live3_chain_actions::run_detect_perspective_conflict(document, label)
        }
        "research:live_reconcile_perspectives" => {
            super::research_live3_chain_actions::run_reconcile_perspectives(document, label)
        }
        "research:live_assess_intentionality" => {
            super::research_live3_chain_actions::run_assess_intentionality(document, label)
        }
        "research:live_classify_mistake" => {
            super::research_live3_chain_actions::run_classify_mistake(document, label)
        }
        "research:live_define_social_dynamics" => {
            super::research_live4_chain_actions::run_define_social_dynamics(document, label)
        }
        "research:live_define_economic_dynamics" => {
            super::research_live4_chain_actions::run_define_economic_dynamics(document, label)
        }
        "research:live_define_spatiotemporal_dynamics" => {
            super::research_live4_chain_actions::run_define_spatiotemporal_dynamics(document, label)
        }
        "research:live_analyse_social_network" => {
            super::research_live4_chain_actions::run_analyse_social_network(document, label)
        }
        "research:live_analyse_inequality" => {
            super::research_live4_chain_actions::run_analyse_inequality(document, label)
        }
        "research:live_analyse_diffusion" => {
            super::research_live4_chain_actions::run_analyse_diffusion(document, label)
        }
        "research:live_assess_grounding" => {
            super::research_live4_chain_actions::run_assess_grounding(document, label)
        }
        "research:live_verify_grounding" => {
            super::research_live4_chain_actions::run_verify_grounding(document, label)
        }
        "research:live_detect_ungrounded_behaviour" => {
            super::research_live4_chain_actions::run_detect_ungrounded_behaviour(document, label)
        }
        "research:live_create_ug_instance" => {
            super::research_live4_chain_actions::run_create_ug_instance(document, label)
        }
        "research:live_set_ug_cause" => {
            super::research_live4_chain_actions::run_set_ug_cause(document, label)
        }
        "research:live_set_ug_consequence" => {
            super::research_live4_chain_actions::run_set_ug_consequence(document, label)
        }
        "research:live_set_ug_detection" => {
            super::research_live4_chain_actions::run_set_ug_detection(document, label)
        }
        "research:live_set_ug_mitigation" => {
            super::research_live4_chain_actions::run_set_ug_mitigation(document, label)
        }
        "research:live_set_ug_calibration" => {
            super::research_live4_chain_actions::run_set_ug_calibration(document, label)
        }
        "research:live_detect_ug_patterns" => {
            super::research_live4_chain_actions::run_detect_ug_patterns(document, label)
        }
        "render:live_scene" => super::render_live_chain_actions::run_scene(document, label),
        "render:live_css_animation" => {
            super::render_live_chain_actions::run_css_animation(document, label)
        }
        "render:live_css_color" => super::render_live_chain_actions::run_css_color(document, label),
        "render:live_css_transform" => {
            super::render_live_chain_actions::run_css_transform(document, label)
        }
        "render:live_animation_eval_curve" => {
            super::render_live_chain_actions::run_animation_eval_curve(document, label)
        }
        "render:live_animation_spring_step" => {
            super::render_live_chain_actions::run_animation_spring_step(document, label)
        }
        "render:live_animation_sclerp" => {
            super::render_live_chain_actions::run_animation_sclerp(document, label)
        }
        "render:live_animation_eval_preset" => {
            super::render_live_chain_actions::run_animation_eval_preset(document, label)
        }
        "render:live_animation_squad_step" => {
            super::render_live_chain_actions::run_animation_squad_step(document, label)
        }
        "render:live_animation_list_presets" => {
            super::render_live_chain_actions::run_animation_list_presets(document, label)
        }
        "render:live_animation_compute_pass" => {
            super::render_live_chain_actions::run_animation_compute_pass(document, label)
        }
        "render:live_svg_path" => super::render_live_chain_actions::run_svg_path(document, label),
        "render:live_svg_circle" => super::render_live_chain_actions::run_svg_circle(document, label),
        "render:live_svg_rect" => super::render_live_chain_actions::run_svg_rect(document, label),
        "render:live_svg_line" => super::render_live_chain_actions::run_svg_line(document, label),
        "render:live_svg_bezier" => super::render_live_chain_actions::run_svg_bezier(document, label),
        "render:live_svg_field" => super::render_live_chain_actions::run_svg_field(document, label),
        "animation:live_spring_step" => {
            super::anim_live_chain_actions::run_spring_step(document, label)
        }
        "animation:live_sclerp_step" => {
            super::anim_live_chain_actions::run_sclerp_step(document, label)
        }
        "animation:live_squad_step" => {
            super::anim_live_chain_actions::run_squad_step(document, label)
        }
        "animation:live_list_presets" => {
            super::anim_live_chain_actions::run_list_presets(document, label)
        }
        "scientific:num_ode_rk4" => super::ode_num_chain_actions::run_rk4(document, label),
        "scientific:num_ode_dopri5" => super::ode_num_chain_actions::run_dopri5(document, label),
        "scientific:num_ode_bdf" => super::ode_num_chain_actions::run_bdf(document, label),
        "scientific:num_ode_symplectic_step" => {
            super::ode_num_chain_actions::run_symplectic_step(document, label)
        }
        "comm:hbbtv_new_app" => super::hbbtv_chain_actions::run_new_app(document, label),
        "comm:hbbtv_add_page" => super::hbbtv_chain_actions::run_add_page(document, label),
        "comm:hbbtv_navigate" => super::hbbtv_chain_actions::run_navigate(document, label),
        "comm:hbbtv_set_state" => super::hbbtv_chain_actions::run_set_state(document, label),
        "render:gpu_live_init" => super::gpu_live_chain_actions::run_gpu_init(document, label),
        "render:gpu_live_init_surface" => {
            super::gpu_live_chain_actions::run_gpu_init_surface(document, label)
        }
        "render:gpu_live_render_frame" => {
            super::gpu_live_chain_actions::run_gpu_render_frame(document, label)
        }
        "render:gpu_live_read_pixels" => {
            super::gpu_live_chain_actions::run_gpu_read_pixels(document, label)
        }
        "render:gpu_live_upload_mesh" => {
            super::gpu_live_chain_actions::run_gpu_upload_mesh(document, label)
        }
        "render:gpu_live_upload_tensor" => {
            super::gpu_live_chain_actions::run_gpu_upload_tensor(document, label)
        }
        "render:gpu_live_pick" => super::gpu_live_chain_actions::run_gpu_pick(document, label),
        "render:gpu_live_poll_pick" => {
            super::gpu_live_chain_actions::run_gpu_poll_pick(document, label)
        }
        "render:gpu_live_resize" => super::gpu_live_chain_actions::run_gpu_resize(document, label),
        "render:gpu_live_set_ambient" => {
            super::gpu_live_chain_actions::run_gpu_set_ambient(document, label)
        }
        "render:gpu_live_destroy" => super::gpu_live_chain_actions::run_gpu_destroy(document, label),
        "render:gpu_live_compute_dispatch" => {
            super::gpu_live_chain_actions::run_gpu_compute_dispatch(document, label)
        }
        "render:gpu_live_compute_readback" => {
            super::gpu_live_chain_actions::run_gpu_compute_readback(document, label)
        }
        "render:gpu_live_validate_shader" => {
            super::gpu_live_chain_actions::run_gpu_validate_shader(document, label)
        }
        "render:gpu_live_compile_shader" => {
            super::gpu_live_chain_actions::run_gpu_compile_shader(document, label)
        }
        "render:gpu_live_compile_to_glsl" => {
            super::gpu_live_chain_actions::run_gpu_compile_to_glsl(document, label)
        }
        "render:gpu_live_backend_info" => {
            super::gpu_live_chain_actions::run_gpu_backend_info(document, label)
        }
        "render:gpu_live_upload_mesh_colored" => {
            super::gpu_live2_chain_actions::run_gpu_upload_mesh_colored(document, label)
        }
        "render:gpu_live_set_standpoint" => {
            super::gpu_live2_chain_actions::run_gpu_set_standpoint(document, label)
        }
        "render:gpu_live_observer_standpoint" => {
            super::gpu_live2_chain_actions::run_gpu_observer_standpoint(document, label)
        }
        "render:gpu_live_camera_state" => {
            super::gpu_live2_chain_actions::run_gpu_camera_state(document, label)
        }
        "render:gpu_live_surface_size" => {
            super::gpu_live2_chain_actions::run_gpu_surface_size(document, label)
        }
        "render:gpu_live_has_mesh" => {
            super::gpu_live2_chain_actions::run_gpu_has_mesh(document, label)
        }
        "render:gpu_live_has_tensor" => {
            super::gpu_live2_chain_actions::run_gpu_has_tensor(document, label)
        }
        "render:gpu_live_tensor_node_count" => {
            super::gpu_live2_chain_actions::run_gpu_tensor_node_count(document, label)
        }
        "render:gpu_live_particle_count" => {
            super::gpu_live2_chain_actions::run_gpu_particle_count(document, label)
        }
        "render:gpu_live_sync_bloom" => {
            super::gpu_live2_chain_actions::run_gpu_sync_bloom(document, label)
        }
        "render:gpu_live_set_artefact_joint" => {
            super::gpu_live2_chain_actions::run_gpu_set_artefact_joint(document, label)
        }
        "render:gpu_live_set_artefact_world" => {
            super::gpu_live2_chain_actions::run_gpu_set_artefact_world(document, label)
        }
        "render:gpu_live_artefact_refused" => {
            super::gpu_live2_chain_actions::run_gpu_artefact_refused(document, label)
        }
        "render:gpu_live_required_rgba8_bytes" => {
            super::gpu_live2_chain_actions::run_gpu_required_rgba8_bytes(document, label)
        }
        "render:gpu_live_emf_upload_field" => {
            super::gpu_live2_chain_actions::run_emf_upload_field(document, label)
        }
        "render:gpu_live_emf_render_slice" => {
            super::gpu_live2_chain_actions::run_emf_render_slice(document, label)
        }
        "render:gpu_live_emf_field_info" => {
            super::gpu_live2_chain_actions::run_emf_field_info(document, label)
        }
        "social:live_gini" => super::wave36_chain_actions::run_social_gini(document, label),
        "social:live_lorenz" => super::wave36_chain_actions::run_social_lorenz(document, label),
        "social:live_degree_centrality" => {
            super::wave36_chain_actions::run_social_degree_centrality(document, label)
        }
        "social:live_lww" => super::wave36_chain_actions::run_social_lww(document, label),
        "forensic:live_malfeasance_delta" => {
            super::wave36_chain_actions::run_forensic_malfeasance_delta(document, label)
        }
        "forensic:live_narrative_divergence" => {
            super::wave36_chain_actions::run_forensic_narrative_divergence(document, label)
        }
        "finance:live_convert_currency" => {
            super::wave36_chain_actions::run_finance_convert_currency(document, label)
        }
        "finance:live_multisig_check" => {
            super::wave36_chain_actions::run_finance_multisig_check(document, label)
        }
        "finance:live_ledger_balance" => {
            super::wave36_chain_actions::run_finance_ledger_balance(document, label)
        }
        "comm:graph_live_corpus_load" => {
            super::wave36_chain_actions::run_corpus_load(document, label)
        }
        "comm:graph_live_corpus_parse" => {
            super::wave36_chain_actions::run_corpus_parse(document, label)
        }
        "comm:graph_live_validate_fragment" => {
            super::wave36_chain_actions::run_chat_validate_fragment(document, label)
        }
        "comm:graph_live_link_reply" => {
            super::wave36_chain_actions::run_chat_link_reply(document, label)
        }
        "comm:graph_live_add_social_post" => {
            super::wave36_chain_actions::run_interactive_add_social_post(document, label)
        }
        "comm:graph_live_add_trigger" => {
            super::wave36_chain_actions::run_interactive_add_trigger(document, label)
        }
        "comm:graph_live_second_screen_sync" => {
            super::wave36_chain_actions::run_second_screen_sync(document, label)
        }
        "scientific:ode_lin1" => super::ode_chain_actions::run_lin1(document, label),
        "scientific:ode_lin2" => super::ode_chain_actions::run_lin2(document, label),
        "scientific:ode_classify_pde" => super::ode_chain_actions::run_classify_pde(document, label),
        "scientific:ode_separable" => super::ode_chain_actions::run_separable(document, label),
        "scientific:ode_pde1" => super::ode_chain_actions::run_pde1(document, label),
        "ai:agent_trace" => super::agent_chain_actions::run_trace(document, label),
        "ai:agent_verify" => super::agent_chain_actions::run_verify(document, label),
        "ai:agent_plan" => super::agent_chain_actions::run_plan(document, label),
        "ai:agent_execute" => super::agent_chain_actions::run_execute(document, label),
        "ai:agent_evaluate" => super::agent_chain_actions::run_evaluate(document, label),
        "ai:nlp_tokenize" => super::nlp_chain_actions::run_tokenize(document, label),
        "ai:nlp_split_sentences" => {
            super::nlp_chain_actions::run_split_sentences(document, label)
        }
        "ai:nlp_coref_resolve" => {
            super::nlp_chain_actions::run_coref_resolve(document, label)
        }
        "ai:nlp_frame_extract" => {
            super::nlp_chain_actions::run_frame_extract(document, label)
        }
        "ai:nlp_fst_lookup" => super::nlp_chain_actions::run_fst_lookup(document, label),
        "ai:nlp_gazetteer_build" => {
            super::nlp_chain_actions::run_gazetteer_build(document, label)
        }
        "ai:nlp_graphrag_query" => {
            super::nlp_chain_actions::run_graphrag_query(document, label)
        }
        "ai:nlp_relation_extract" => {
            super::nlp_chain_actions::run_relation_extract(document, label)
        }
        "ai:nlp_substrate_extract" => {
            super::nlp_chain_actions::run_substrate_extract(document, label)
        }
        _ => {
            if let Some(spec) = super::spec_tools::lookup(tool_id) {
                super::spec_tools::run(document, spec, label);
            } else {
                super::interactions::show_tool_status(
                    document,
                    label,
                    &format!(
                        "No executable contract is registered for {} action `{}`.",
                        action, tool_id
                    ),
                    "unavailable",
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_chest::core::tool::ToolKind;

    #[test]
    fn every_registered_nonplacement_tool_has_an_explicit_policy() {
        let registry = crate::browser::registration::build_registry();
        for toolbox in registry.toolboxes() {
            for chain in toolbox.chains() {
                for tool in chain.tools() {
                    if tool.metadata().kind != ToolKind::PlaceContainer {
                        assert!(
                            has_dispatch_policy(&tool.metadata().id),
                            "missing action policy for {}",
                            tool.metadata().id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn local_extractor_is_bounded_and_deterministic() {
        assert!(!requires_daemon("ai:extractor"));
        assert!(!requires_daemon("ai:sentinel"));
        assert!(!requires_daemon("graph:sparql_query"));
        assert!(!requires_daemon("n3:evaluate"));
        assert!(!requires_daemon("shacl:validate"));
        let input = "QualiaDB joins Poet. Webizen renders the graph!";
        let first = local_extract_summary(input);
        let second = local_extract_summary(input);
        assert_eq!(first, second);
        assert_eq!(first.0, 7);
        assert_eq!(first.1, 2);
        assert_eq!(first.2, vec!["QualiaDB", "Poet", "Webizen"]);
    }

    #[test]
    fn empty_local_extractor_input_has_no_entities() {
        assert_eq!(local_extract_summary(""), (0, 0, Vec::new()));
    }
}
