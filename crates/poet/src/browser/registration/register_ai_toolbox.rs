//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_ai_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:co_author".into(),
                label: "Co-Author".into(),
                icon: "coauthor".into(),
                kind: ToolKind::RunAction,
                capability_scope: None,
                ontology_prefix: "ai".into(),
                description: "Invoke co-author assistance.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:extractor".into(),
                label: "Extractor".into(),
                icon: "extractor".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("NLP.gazetteer_run".into()),
                ontology_prefix: "ai".into(),
                description: "Extract entities from text.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:sentinel".into(),
                label: "Sentinel Guard".into(),
                icon: "sentinel".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Sentinel.inspect".into()),
                ontology_prefix: "ai".into(),
                description: "Invoke sentinel monitoring.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:triad".into(),
                label: "+ Triad Viewport".into(),
                icon: "triad".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "ai".into(),
                description: "Place a triad (q42+p64+d10) container.".into(),
            },
            ActionType::Query,
        )),
    ];

    let ml_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_mse".into(),
                label: "MSE".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.mse".into()),
                ontology_prefix: "ai".into(),
                description: "Mean squared error from paired y_true / y_pred numbers on the surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_rmse".into(),
                label: "RMSE".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.rmse".into()),
                ontology_prefix: "ai".into(),
                description: "Root mean squared error from paired predictions on the surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_mae".into(),
                label: "MAE".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.mae".into()),
                ontology_prefix: "ai".into(),
                description: "Mean absolute error from paired predictions on the surface.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_r2".into(),
                label: "R² score".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.r2_score".into()),
                ontology_prefix: "ai".into(),
                description: "Coefficient of determination from paired y_true / y_pred.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_accuracy".into(),
                label: "Accuracy".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.accuracy".into()),
                ontology_prefix: "ai".into(),
                description: "Classification accuracy from paired label integers.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_ols".into(),
                label: "OLS fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.ols".into()),
                ontology_prefix: "ai".into(),
                description: "Ordinary least squares from feature / response pairs on the surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_train_test_split".into(),
                label: "Train/test split".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.train_test_split".into()),
                ontology_prefix: "ai".into(),
                description: "Split indices for train and test rows (n and test_ratio).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_kmeans".into(),
                label: "k-means".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.kmeans".into()),
                ontology_prefix: "ai".into(),
                description: "Cluster row-major points with MachineLearning.kmeans.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_log_loss".into(),
                label: "Log loss".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.log_loss".into()),
                ontology_prefix: "ai".into(),
                description: "Cross-entropy from probabilities and boolean labels.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_bonferroni".into(),
                label: "Bonferroni".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.bonferroni".into()),
                ontology_prefix: "ai".into(),
                description: "Bonferroni-correct p-values from the selected surface.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_confusion_binary".into(),
                label: "Confusion matrix".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.confusion_binary".into()),
                ontology_prefix: "ai".into(),
                description: "Binary confusion matrix from paired true / predicted labels.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_holm".into(),
                label: "Holm".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.holm".into()),
                ontology_prefix: "ai".into(),
                description: "Holm–Bonferroni-correct p-values from the selected surface.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_benjamini_hochberg".into(),
                label: "BH FDR".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.benjamini_hochberg".into()),
                ontology_prefix: "ai".into(),
                description: "Benjamini–Hochberg FDR correction on surface p-values.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_ab_test".into(),
                label: "A/B test".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.ab_test".into()),
                ontology_prefix: "ai".into(),
                description: "Two-proportion A/B test from conversion counts on the surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_bootstrap_estimate".into(),
                label: "Bootstrap SE".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.bootstrap_estimate".into()),
                ontology_prefix: "ai".into(),
                description: "Bootstrap standard error and bias for the sample mean.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_bootstrap_ci".into(),
                label: "Bootstrap CI".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.bootstrap_ci".into()),
                ontology_prefix: "ai".into(),
                description: "Bootstrap percentile confidence interval for the sample mean."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_permutation_test".into(),
                label: "Permutation test".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.permutation_test".into()),
                ontology_prefix: "ai".into(),
                description: "Two-sample permutation test from paired group halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_n_rejected".into(),
                label: "n rejected".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.n_rejected".into()),
                ontology_prefix: "ai".into(),
                description: "Count hypotheses rejected from adjusted p-values at alpha.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_power_two_sample".into(),
                label: "Power (2-sample)".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.power_two_sample".into()),
                ontology_prefix: "ai".into(),
                description: "Statistical power of a two-sample test for effect size d.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_roc_auc".into(),
                label: "ROC AUC".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.roc_auc".into()),
                ontology_prefix: "ai".into(),
                description: "ROC AUC from paired scores and boolean labels on the surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_k_fold".into(),
                label: "k-fold".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.k_fold".into()),
                ontology_prefix: "ai".into(),
                description: "Build k-fold cross-validation index splits for n samples.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_bootstrap_indices".into(),
                label: "Bootstrap indices".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.bootstrap_indices".into()),
                ontology_prefix: "ai".into(),
                description: "Draw a bootstrap resample of indices of length n.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_pca".into(),
                label: "PCA".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.pca".into()),
                ontology_prefix: "ai".into(),
                description: "Fit principal components on row-major points from the surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_required_sample_size".into(),
                label: "Required n".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.required_sample_size".into()),
                ontology_prefix: "ai".into(),
                description: "Required sample size for a two-sample test at target power."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_polynomial_regression".into(),
                label: "Poly fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.polynomial_regression".into()),
                ontology_prefix: "ai".into(),
                description: "Fit a degree-d polynomial from x / y halves on the surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_required_n_two_prop".into(),
                label: "Required n (2-prop)".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some(
                    "MachineLearning.required_sample_size_two_proportion".into(),
                ),
                ontology_prefix: "ai".into(),
                description: "Required sample size for a two-proportion test at target power."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_loocv".into(),
                label: "LOOCV".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.loocv".into()),
                ontology_prefix: "ai".into(),
                description: "Leave-one-out cross-validation folds for n samples.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_transe_score".into(),
                label: "TransE score".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.transe_score".into()),
                ontology_prefix: "ai".into(),
                description: "Score a knowledge-graph triple with TransE embeddings.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_distmult_score".into(),
                label: "DistMult score".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.distmult_score".into()),
                ontology_prefix: "ai".into(),
                description: "Score a knowledge-graph triple with DistMult embeddings.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_complex_score".into(),
                label: "ComplEx score".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.complex_score".into()),
                ontology_prefix: "ai".into(),
                description: "Score a knowledge-graph triple with ComplEx embeddings.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_rotate_score".into(),
                label: "RotatE score".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.rotate_score".into()),
                ontology_prefix: "ai".into(),
                description: "Score a knowledge-graph triple with RotatE embeddings.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_ridge_fit".into(),
                label: "Ridge fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.ridge_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit ridge regression from x / y halves on the surface.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_lasso_fit".into(),
                label: "Lasso fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.lasso_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit lasso regression from x / y halves on the surface.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_pls_fit".into(),
                label: "PLS fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.pls_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit PLS1 regression from x / y halves on the surface.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_standard_scaler".into(),
                label: "Standard scaler".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.standard_scaler_fit_transform".into()),
                ontology_prefix: "ai".into(),
                description: "Z-score standardize row-major points from the surface.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_kmeans_fit".into(),
                label: "k-means fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.kmeans_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit k-means++ clusters on row-major points from the surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_gmm_fit".into(),
                label: "GMM fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.gmm_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit a diagonal Gaussian mixture on row-major points.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_logistic_fit".into(),
                label: "Logistic fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.logistic_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit logistic regression from x / label halves on the surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_poisson_fit".into(),
                label: "Poisson fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.poisson_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit Poisson regression from x / count halves on the surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_naive_bayes_fit".into(),
                label: "Naive Bayes".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.naive_bayes_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit Gaussian naive Bayes from x / class-label halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_knn_fit".into(),
                label: "k-NN fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.knn_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit a k-nearest-neighbours classifier from x / label halves."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_lda_fit".into(),
                label: "LDA fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.lda_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit linear discriminant analysis from x / label halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_pcr_fit".into(),
                label: "PCR fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.pcr_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit principal component regression from x / y halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_qda_fit".into(),
                label: "QDA fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.qda_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit quadratic discriminant analysis from x / label halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_multinomial_logistic_fit".into(),
                label: "Multinomial logistic".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.multinomial_logistic_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit multinomial logistic regression from x / label halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_hierarchical_fit".into(),
                label: "Hierarchical fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.hierarchical_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit agglomerative hierarchical clustering on row-major points."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_hierarchical_labels".into(),
                label: "Hierarchical labels".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.hierarchical_labels".into()),
                ontology_prefix: "ai".into(),
                description: "Cut a hierarchical dendrogram into k cluster labels.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_bayesian_linear_fit".into(),
                label: "Bayesian linear".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.bayesian_linear_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit Bayesian linear regression from x / y halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_decision_tree_regressor".into(),
                label: "Tree regressor".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.decision_tree_fit_regressor".into()),
                ontology_prefix: "ai".into(),
                description: "Fit a regression decision tree from x / y halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_decision_tree_classifier".into(),
                label: "Tree classifier".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.decision_tree_fit_classifier".into()),
                ontology_prefix: "ai".into(),
                description: "Fit a classification decision tree from x / label halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_gp_fit".into(),
                label: "GP fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.gp_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit a Gaussian process regressor from x / y halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_svm_fit".into(),
                label: "SVM fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.svm_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit a soft-margin SVM from x / boolean-label halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_kaplan_meier_fit".into(),
                label: "Kaplan–Meier".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.kaplan_meier_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit a Kaplan–Meier survival curve from time / event halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_cox_fit".into(),
                label: "Cox PH".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.cox_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit Cox proportional hazards from x / times / event thirds.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_hmm_baum_welch".into(),
                label: "HMM Baum–Welch".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.hmm_baum_welch".into()),
                ontology_prefix: "ai".into(),
                description: "Learn HMM parameters by Baum–Welch from discrete observations.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_variational_gaussian_fit".into(),
                label: "Variational Gaussian".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.variational_gaussian_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Mean-field variational inference for a univariate Gaussian.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_mcmc_metropolis".into(),
                label: "MCMC Metropolis".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.mcmc_metropolis".into()),
                ontology_prefix: "ai".into(),
                description: "Random-walk Metropolis–Hastings on a standard normal target.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_svm_multiclass_fit".into(),
                label: "Multiclass SVM".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.svm_multiclass_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit a one-vs-rest multiclass SVM from x / label halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_som_train".into(),
                label: "SOM train".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.som_train".into()),
                ontology_prefix: "ai".into(),
                description: "Train a self-organizing map on row-major points.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_random_forest_regressor".into(),
                label: "RF regressor".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.random_forest_fit_regressor".into()),
                ontology_prefix: "ai".into(),
                description: "Fit a random forest regressor from x / y halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_random_forest_classifier".into(),
                label: "RF classifier".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.random_forest_fit_classifier".into()),
                ontology_prefix: "ai".into(),
                description: "Fit a random forest classifier from x / label halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_gradient_boosting_regressor".into(),
                label: "GBM regressor".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.gradient_boosting_fit_regressor".into()),
                ontology_prefix: "ai".into(),
                description: "Fit gradient boosting regression from x / y halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_bart_fit".into(),
                label: "BART fit".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.bart_fit".into()),
                ontology_prefix: "ai".into(),
                description: "Fit Bayesian additive regression trees from x / y halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_kg_mean_rank".into(),
                label: "KG mean rank".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.kg_mean_rank".into()),
                ontology_prefix: "ai".into(),
                description: "Mean rank of knowledge-graph test triples under an embedding table."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_kg_mrr".into(),
                label: "KG MRR".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.kg_mean_reciprocal_rank".into()),
                ontology_prefix: "ai".into(),
                description: "Mean reciprocal rank of knowledge-graph test triples.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_kg_hits_at_k".into(),
                label: "KG Hits@k".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.kg_hits_at_k".into()),
                ontology_prefix: "ai".into(),
                description: "Hits@k of knowledge-graph test triples under an embedding table."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_kalman_new".into(),
                label: "Kalman step".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.kalman_new".into()),
                ontology_prefix: "ai".into(),
                description: "Run one Kalman filter predict/update cycle (1-D demo or surface z)."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_factor_graph_marginals".into(),
                label: "Factor-graph BP".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.factor_graph_marginals".into()),
                ontology_prefix: "ai".into(),
                description: "Sum-product belief propagation marginals on a factor graph.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_al_row_score".into(),
                label: "AL row score".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.al_row_score".into()),
                ontology_prefix: "ai".into(),
                description: "Active-learning uncertainty score for one probability row.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_al_score".into(),
                label: "AL score pool".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.al_score".into()),
                ontology_prefix: "ai".into(),
                description: "Active-learning uncertainty scores for a pool of probability rows."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_al_cosine".into(),
                label: "AL cosine".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.al_cosine_similarity".into()),
                ontology_prefix: "ai".into(),
                description: "Cosine similarity of two feature vectors from surface halves.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_al_rank_informative".into(),
                label: "AL rank informative".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.al_rank_informative".into()),
                ontology_prefix: "ai".into(),
                description: "Rank an active-learning pool by uncertainty (most informative first)."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_al_most_informative".into(),
                label: "AL most informative".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.al_most_informative".into()),
                ontology_prefix: "ai".into(),
                description: "Index of the most informative sample in an uncertainty pool.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_al_representativeness".into(),
                label: "AL representativeness".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.al_representativeness".into()),
                ontology_prefix: "ai".into(),
                description: "Mean representativeness score per feature point in the pool.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_al_information_density".into(),
                label: "AL information density".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.al_information_density".into()),
                ontology_prefix: "ai".into(),
                description: "Combine uncertainty with representativeness^β for density scores."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_al_rank_by_density".into(),
                label: "AL rank by density".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.al_rank_by_density".into()),
                ontology_prefix: "ai".into(),
                description: "Rank an active-learning pool by information density.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_al_vote_entropy".into(),
                label: "AL vote entropy".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.al_vote_entropy".into()),
                ontology_prefix: "ai".into(),
                description: "Vote entropy of a committee over discrete class votes.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_al_consensus".into(),
                label: "AL consensus".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.al_consensus".into()),
                ontology_prefix: "ai".into(),
                description: "Consensus probability distribution of a query-by-committee.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_al_consensus_entropy".into(),
                label: "AL consensus entropy".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.al_consensus_entropy".into()),
                ontology_prefix: "ai".into(),
                description: "Entropy of the committee consensus distribution.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_al_kl_disagreement".into(),
                label: "AL KL disagreement".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.al_average_kl_disagreement".into()),
                ontology_prefix: "ai".into(),
                description: "Mean KL divergence of committee members from consensus.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:ml_al_rank_by_disagreement".into(),
                label: "AL rank by disagreement".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("MachineLearning.al_rank_by_disagreement".into()),
                ontology_prefix: "ai".into(),
                description: "Rank a pool by committee KL disagreement.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    let inf_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:inf_relu".into(),
                label: "ReLU".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Inference.relu".into()),
                ontology_prefix: "ai".into(),
                description: "Element-wise ReLU on surface numbers (Host Inference.relu).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:inf_sigmoid".into(),
                label: "Sigmoid".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Inference.sigmoid".into()),
                ontology_prefix: "ai".into(),
                description: "Element-wise logistic sigmoid on surface numbers.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:inf_gelu".into(),
                label: "GELU".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Inference.gelu".into()),
                ontology_prefix: "ai".into(),
                description: "Element-wise GELU (tanh approx) on surface numbers.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:inf_softmax".into(),
                label: "Softmax".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Inference.softmax".into()),
                ontology_prefix: "ai".into(),
                description: "Softmax over surface numbers (sums to 1).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:inf_rms_norm".into(),
                label: "RMS norm".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Inference.rms_norm".into()),
                ontology_prefix: "ai".into(),
                description: "RMS normalization with weight (data-weight / data-eps).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:inf_embed".into(),
                label: "Embed text".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Inference.embed".into()),
                ontology_prefix: "ai".into(),
                description: "Hash-ngram embed selected text via Inference.embed.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:inf_run_classifier".into(),
                label: "Run classifier".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Inference.run_classifier".into()),
                ontology_prefix: "ai".into(),
                description: "KNN/NB/SVM classify from surface features (default knn).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:inf_vector_search".into(),
                label: "Vector search".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Inference.vector_search".into()),
                ontology_prefix: "ai".into(),
                description: "Nearest-neighbour search over corpus lines (data-query / data-k)."
                    .into(),
            },
            ActionType::Invoke,
        )),
    ];

    let orch_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:orch_session_create".into(),
                label: "Session create".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Orchestration.session_create".into()),
                ontology_prefix: "ai".into(),
                description: "Create an orchestration session from data-id/data-task or surface tokens."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:orch_session_plan".into(),
                label: "Session plan".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Orchestration.session_plan".into()),
                ontology_prefix: "ai".into(),
                description: "Plan the selected orchestration session's task DAG.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:orch_session_execute".into(),
                label: "Session execute".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Orchestration.session_execute".into()),
                ontology_prefix: "ai".into(),
                description: "Execute the planned orchestration session DAG.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:orch_session_status".into(),
                label: "Session status".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Orchestration.session_status".into()),
                ontology_prefix: "ai".into(),
                description: "Query orchestration session status and summary.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:orch_roster_register".into(),
                label: "Roster register".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Orchestration.roster_register".into()),
                ontology_prefix: "ai".into(),
                description: "Register an agent on the session roster.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:orch_roster_list".into(),
                label: "Roster list".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Orchestration.roster_list".into()),
                ontology_prefix: "ai".into(),
                description: "List agents registered on the session roster.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:orch_roster_capabilities".into(),
                label: "Roster capabilities".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Orchestration.roster_capabilities".into()),
                ontology_prefix: "ai".into(),
                description: "List all capabilities present on the session roster.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:orch_assign_agents".into(),
                label: "Assign agents".into(),
                icon: "ai".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Orchestration.assign_agents".into()),
                ontology_prefix: "ai".into(),
                description: "Assign roster agents to planned orchestration steps.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "ai".into(),
            label: "AI Co-Pilot & Sentinel".into(),
            icon: "ai".into(),
            ontology_prefix: "ai".into(),
            description: "Resident GGUF LLMs, Epistemic Halo guard, and Triad execution.".into(),
            enabled_by_default: true,
            family: "ai".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "ai:copilot".into(),
                    label: "Sentinel Guard & Model".into(),
                    icon: "ai".into(),
                    description:
                        "Select resident GGUF model, halo confidence threshold, and temperature."
                            .into(),
                },
                vec![
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "ai:grounding".into(),
                            label: "Ground generation".into(),
                            icon: "sentinel".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("Inference.grounding".into()),
                            ontology_prefix: "ai".into(),
                            description:
                                "Check selected generation text against Inference.grounding."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "ai:detect_ungrounded".into(),
                            label: "Find ungrounded text".into(),
                            icon: "sentinel".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("Inference.detect_ungrounded".into()),
                            ontology_prefix: "ai".into(),
                            description:
                                "Flag selected generation text with Inference.detect_ungrounded."
                                    .into(),
                        },
                        ActionType::Query,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "ai:verify_turn".into(),
                            label: "Verify this turn".into(),
                            icon: "sentinel".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("Inference.verify_turn".into()),
                            ontology_prefix: "ai".into(),
                            description: "Run Inference.verify_turn on the selected generation."
                                .into(),
                        },
                        ActionType::Query,
                    )),
                ],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "ai:inf".into(),
                    label: "Live inference".into(),
                    icon: "ai".into(),
                    description:
                        "Curated Inference.* activations, embed, classifier, and vector search."
                            .into(),
                },
                inf_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "ai:ml".into(),
                    label: "Live machine learning".into(),
                    icon: "ai".into(),
                    description:
                        "Curated MachineLearning.* metrics, OLS, k-means, and resampling binds."
                            .into(),
                },
                ml_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "ai:orch".into(),
                    label: "Live orchestration".into(),
                    icon: "ai".into(),
                    description:
                        "Curated Orchestration.* session, roster, and assignment binds.".into(),
                },
                orch_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "ai:nlp".into(),
                    label: "Live natural language".into(),
                    icon: "ai".into(),
                    description:
                        "Curated NLP.* tokenization, coreference, frames, and GraphRAG binds."
                            .into(),
                },
                register_ai_nlp::nlp_tools(),
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "ai:tools".into(),
                    label: "Co-Pilot Capabilities".into(),
                    icon: "tools".into(),
                    description: "Invoke co-authoring, text extraction, and triad viewports."
                        .into(),
                },
                tools,
            ),
        ],
    ));
}
