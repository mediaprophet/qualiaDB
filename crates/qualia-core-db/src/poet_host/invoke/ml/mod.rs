//! Machine learning invoke seams.
//!
//! Exposes `solvers::learning` through VibeScript invoke IDs
//! in the `MachineLearning.*` namespace.

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod active;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod extended;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod extra;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod fitters;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod fitters2;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod fitters3;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod more;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod ols;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use active::*;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use extended::{accuracy, kmeans, mae, mse, r2_score, rmse, roc_auc, train_test_split};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use extra::*;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use fitters::*;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use fitters2::*;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use fitters3::*;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use more::*;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use ols::fit_ols;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn fit_ols(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "MachineLearning"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
macro_rules! ml_stub {
    ($($name:ident),*) => {
        $(
            pub fn $name(
                _args: &poet_vibe::Value,
                span: poet_vibe::Span,
            ) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
                Err(super::args::need_scientific(span, "MachineLearning"))
            }
        )*
    };
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
ml_stub!(
    mse,
    rmse,
    mae,
    r2_score,
    accuracy,
    roc_auc,
    kmeans,
    train_test_split,
    log_loss,
    confusion_binary,
    k_fold,
    bootstrap_indices,
    bonferroni,
    holm,
    benjamini_hochberg,
    pca,
    ab_test,
    power_two_sample,
    required_sample_size,
    transe_score,
    distmult_score,
    // more
    complex_score,
    rotate_score,
    kg_mean_rank,
    kg_mean_reciprocal_rank,
    kg_hits_at_k,
    polynomial_regression,
    bootstrap_estimate,
    bootstrap_ci,
    permutation_test,
    required_sample_size_two_proportion,
    loocv,
    n_rejected,
    // active learning
    al_row_score,
    al_score,
    al_rank_informative,
    al_most_informative,
    al_cosine_similarity,
    al_representativeness,
    al_information_density,
    al_rank_by_density,
    al_vote_entropy,
    al_consensus,
    al_consensus_entropy,
    al_average_kl_disagreement,
    al_rank_by_disagreement,
    // fitters
    ridge_fit,
    lasso_fit,
    pls_fit,
    kmeans_fit,
    gmm_fit,
    logistic_fit,
    poisson_fit,
    cox_fit,
    svm_fit,
    // fitters2
    decision_tree_fit_regressor,
    decision_tree_fit_classifier,
    hmm_baum_welch,
    variational_gaussian_fit,
    mcmc_metropolis,
    gp_fit,
    // fitters3
    naive_bayes_fit,
    knn_fit,
    lda_fit,
    qda_fit,
    multinomial_logistic_fit,
    svm_multiclass_fit,
    hierarchical_fit,
    hierarchical_labels,
    kaplan_meier_fit,
    pcr_fit,
    bayesian_linear_fit,
    som_train,
    kalman_new,
    random_forest_fit_regressor,
    random_forest_fit_classifier,
    gradient_boosting_fit_regressor
);
