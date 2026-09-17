//! ML fitter invoke seams (part 3) — remaining classifiers (naive Bayes,
//! k-NN, LDA, QDA, multinomial logistic, multiclass SVM), hierarchical
//! clustering, Kaplan–Meier, PCR, Bayesian linear, SOM, Kalman filter,
//! random forest, gradient boosting.

use super::super::args;
use super::fitters::parse_matrix;
use crate::solvers::learning as ml;
use vibe::{Diagnostic, Span, Value};

// ── Classifiers ─────────────────────────────────────────────────────

/// `MachineLearning.naive_bayes_fit` — Gaussian naive Bayes classifier.
/// Args: { x: [[f64]], y: [u64] }
pub fn naive_bayes_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) = parse_matrix(args, "x")
        .ok_or_else(|| args::bad(span, "naive_bayes_fit needs x: [[f64]]"))?;
    let y_u64 =
        args::rec_u64_list(args, "y").ok_or_else(|| args::bad(span, "naive_bayes_fit needs y"))?;
    let y: Vec<usize> = y_u64.iter().map(|&v| v as usize).collect();
    match ml::classification::naive_bayes::GaussianNb::fit(&x, &y, n, p) {
        Ok(model) => {
            let preds = model.predict(&x, n);
            Ok(args::record([
                (
                    "classes",
                    Value::List(
                        model
                            .classes
                            .iter()
                            .map(|&c| Value::U64(c as u64))
                            .collect(),
                    ),
                ),
                ("n", Value::U64(n as u64)),
                ("p", Value::U64(p as u64)),
                (
                    "predictions",
                    Value::List(preds.iter().map(|&p| Value::U64(p as u64)).collect()),
                ),
            ]))
        }
        Err(e) => Err(args::bad(span, format!("naive_bayes_fit: {e:?}"))),
    }
}

/// `MachineLearning.knn_fit` — k-nearest-neighbours classifier.
/// Args: { x: [[f64]], y: [u64], k: u64 }
pub fn knn_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "knn_fit needs x: [[f64]]"))?;
    let y_u64 = args::rec_u64_list(args, "y").ok_or_else(|| args::bad(span, "knn_fit needs y"))?;
    let y: Vec<usize> = y_u64.iter().map(|&v| v as usize).collect();
    let k = args::rec_u64(args, "k").ok_or_else(|| args::bad(span, "knn_fit needs k"))? as usize;
    match ml::classification::knn::KnnClassifier::fit(&x, &y, n, p, k) {
        Ok(model) => {
            let preds = model.predict(&x, n);
            Ok(args::record([
                ("n", Value::U64(n as u64)),
                ("p", Value::U64(p as u64)),
                ("k", Value::U64(k as u64)),
                (
                    "predictions",
                    Value::List(preds.iter().map(|&p| Value::U64(p as u64)).collect()),
                ),
            ]))
        }
        Err(e) => Err(args::bad(span, format!("knn_fit: {e:?}"))),
    }
}

/// `MachineLearning.lda_fit` — Linear Discriminant Analysis classifier.
/// Args: { x: [[f64]], y: [u64] }
pub fn lda_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "lda_fit needs x: [[f64]]"))?;
    let y_u64 = args::rec_u64_list(args, "y").ok_or_else(|| args::bad(span, "lda_fit needs y"))?;
    let y: Vec<usize> = y_u64.iter().map(|&v| v as usize).collect();
    match ml::classification::discriminant::LdaModel::fit(&x, &y, n, p) {
        Ok(model) => {
            let preds = model.predict(&x, n);
            Ok(args::record([
                (
                    "classes",
                    Value::List(
                        model
                            .classes
                            .iter()
                            .map(|&c| Value::U64(c as u64))
                            .collect(),
                    ),
                ),
                ("n", Value::U64(n as u64)),
                ("p", Value::U64(p as u64)),
                (
                    "predictions",
                    Value::List(preds.iter().map(|&p| Value::U64(p as u64)).collect()),
                ),
            ]))
        }
        Err(e) => Err(args::bad(span, format!("lda_fit: {e:?}"))),
    }
}

/// `MachineLearning.qda_fit` — Quadratic Discriminant Analysis classifier.
/// Args: { x: [[f64]], y: [u64] }
pub fn qda_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "qda_fit needs x: [[f64]]"))?;
    let y_u64 = args::rec_u64_list(args, "y").ok_or_else(|| args::bad(span, "qda_fit needs y"))?;
    let y: Vec<usize> = y_u64.iter().map(|&v| v as usize).collect();
    match ml::classification::discriminant::QdaModel::fit(&x, &y, n, p) {
        Ok(model) => {
            let preds = model.predict(&x, n);
            Ok(args::record([
                (
                    "classes",
                    Value::List(
                        model
                            .classes
                            .iter()
                            .map(|&c| Value::U64(c as u64))
                            .collect(),
                    ),
                ),
                ("n", Value::U64(n as u64)),
                ("p", Value::U64(p as u64)),
                (
                    "predictions",
                    Value::List(preds.iter().map(|&p| Value::U64(p as u64)).collect()),
                ),
            ]))
        }
        Err(e) => Err(args::bad(span, format!("qda_fit: {e:?}"))),
    }
}

/// `MachineLearning.multinomial_logistic_fit` — softmax regression.
/// Args: { x: [[f64]], y: [u64], intercept: bool, lr: f64, l2: f64, max_iter: u64 }
pub fn multinomial_logistic_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) = parse_matrix(args, "x")
        .ok_or_else(|| args::bad(span, "multinomial_logistic_fit needs x: [[f64]]"))?;
    let y_u64 = args::rec_u64_list(args, "y")
        .ok_or_else(|| args::bad(span, "multinomial_logistic_fit needs y"))?;
    let y: Vec<usize> = y_u64.iter().map(|&v| v as usize).collect();
    let intercept = args::rec_bool(args, "intercept").unwrap_or(true);
    let lr = args::rec_f64(args, "lr").unwrap_or(0.1);
    let l2 = args::rec_f64(args, "l2").unwrap_or(0.0);
    let max_iter = args::rec_u64(args, "max_iter").unwrap_or(200) as usize;
    match ml::glm::multinomial::MultinomialLogistic::fit(&x, &y, n, p, intercept, lr, l2, max_iter)
    {
        Ok(model) => {
            let preds: Vec<usize> = (0..n)
                .map(|i| model.predict_row(&x[i * p..(i + 1) * p]))
                .collect();
            Ok(args::record([
                (
                    "classes",
                    Value::List(
                        model
                            .classes
                            .iter()
                            .map(|&c| Value::U64(c as u64))
                            .collect(),
                    ),
                ),
                ("n", Value::U64(n as u64)),
                ("p", Value::U64(p as u64)),
                (
                    "predictions",
                    Value::List(preds.iter().map(|&p| Value::U64(p as u64)).collect()),
                ),
            ]))
        }
        Err(e) => Err(args::bad(span, format!("multinomial_logistic_fit: {e:?}"))),
    }
}

/// `MachineLearning.svm_multiclass_fit` — one-vs-rest multiclass SVM.
/// Args: { x: [[f64]], y: [u64], c: f64, kernel: "linear"|"rbf", gamma: f64, max_passes: u64, tol: f64 }
pub fn svm_multiclass_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) = parse_matrix(args, "x")
        .ok_or_else(|| args::bad(span, "svm_multiclass_fit needs x: [[f64]]"))?;
    let y_u64 = args::rec_u64_list(args, "y")
        .ok_or_else(|| args::bad(span, "svm_multiclass_fit needs y"))?;
    let y: Vec<usize> = y_u64.iter().map(|&v| v as usize).collect();
    let c = args::rec_f64(args, "c").unwrap_or(1.0);
    let kernel_str = args::rec_str(args, "kernel").unwrap_or("linear");
    let gamma = args::rec_f64(args, "gamma").unwrap_or(0.5);
    let kernel = match kernel_str {
        "rbf" | "Rbf" | "RBF" => ml::classification::svm::Kernel::Rbf { gamma },
        _ => ml::classification::svm::Kernel::Linear,
    };
    let max_passes = args::rec_u64(args, "max_passes").unwrap_or(5) as usize;
    let tol = args::rec_f64(args, "tol").unwrap_or(1e-3);
    match ml::classification::svm_multiclass::MulticlassSvm::fit_one_vs_rest(
        &x, &y, n, p, c, kernel, max_passes, tol,
    ) {
        Ok(model) => {
            let preds = model.predict(&x, n);
            Ok(args::record([
                ("n", Value::U64(n as u64)),
                ("p", Value::U64(p as u64)),
                ("kernel", Value::String(kernel_str.to_string())),
                (
                    "predictions",
                    Value::List(preds.iter().map(|&p| Value::U64(p as u64)).collect()),
                ),
            ]))
        }
        Err(e) => Err(args::bad(span, format!("svm_multiclass_fit: {e:?}"))),
    }
}

// ── Hierarchical clustering ─────────────────────────────────────────

/// `MachineLearning.hierarchical_fit` — agglomerative hierarchical clustering.
/// Args: { x: [[f64]], linkage: "single"|"complete"|"average" }
pub fn hierarchical_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) = parse_matrix(args, "x")
        .ok_or_else(|| args::bad(span, "hierarchical_fit needs x: [[f64]]"))?;
    let linkage_str = args::rec_str(args, "linkage").unwrap_or("complete");
    let linkage = match linkage_str {
        "single" | "Single" => ml::clustering::hierarchical::Linkage::Single,
        "average" | "Average" => ml::clustering::hierarchical::Linkage::Average,
        _ => ml::clustering::hierarchical::Linkage::Complete,
    };
    match ml::clustering::hierarchical::Hierarchical::fit(&x, n, p, linkage) {
        Ok(model) => Ok(args::record([
            ("n", Value::U64(n as u64)),
            ("p", Value::U64(p as u64)),
            ("linkage", Value::String(linkage_str.to_string())),
            ("n_merges", Value::U64(model.n_merges() as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("hierarchical_fit: {e:?}"))),
    }
}

/// `MachineLearning.hierarchical_labels` — cut a dendrogram into k clusters.
/// Args: { x: [[f64]], linkage: "...", k: u64 }
pub fn hierarchical_labels(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) = parse_matrix(args, "x")
        .ok_or_else(|| args::bad(span, "hierarchical_labels needs x: [[f64]]"))?;
    let linkage_str = args::rec_str(args, "linkage").unwrap_or("complete");
    let linkage = match linkage_str {
        "single" | "Single" => ml::clustering::hierarchical::Linkage::Single,
        "average" | "Average" => ml::clustering::hierarchical::Linkage::Average,
        _ => ml::clustering::hierarchical::Linkage::Complete,
    };
    let k = args::rec_u64(args, "k")
        .ok_or_else(|| args::bad(span, "hierarchical_labels needs k"))? as usize;
    match ml::clustering::hierarchical::Hierarchical::fit(&x, n, p, linkage) {
        Ok(model) => {
            let labels = model.labels(k);
            Ok(args::record([
                ("k", Value::U64(k as u64)),
                (
                    "labels",
                    Value::List(labels.iter().map(|&l| Value::U64(l as u64)).collect()),
                ),
            ]))
        }
        Err(e) => Err(args::bad(span, format!("hierarchical_labels: {e:?}"))),
    }
}

// ── Kaplan–Meier ────────────────────────────────────────────────────

/// `MachineLearning.kaplan_meier_fit` — Kaplan–Meier survival estimator.
/// Args: { times: [f64], event: [bool] }
pub fn kaplan_meier_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let times = args::rec_f64_list(args, "times")
        .ok_or_else(|| args::bad(span, "kaplan_meier_fit needs times"))?;
    let event = args::rec_bool_list(args, "event")
        .ok_or_else(|| args::bad(span, "kaplan_meier_fit needs event"))?;
    match ml::survival::kaplan_meier::KaplanMeier::fit(&times, &event) {
        Ok(model) => Ok(args::record([
            ("event_times", args::f64_list_value(model.event_times)),
            ("survival", args::f64_list_value(model.survival)),
            (
                "at_risk",
                Value::List(
                    model
                        .at_risk
                        .iter()
                        .map(|&a| Value::U64(a as u64))
                        .collect(),
                ),
            ),
            (
                "events",
                Value::List(model.events.iter().map(|&e| Value::U64(e as u64)).collect()),
            ),
        ])),
        Err(e) => Err(args::bad(span, format!("kaplan_meier_fit: {e:?}"))),
    }
}

// ── PCR ─────────────────────────────────────────────────────────────

/// `MachineLearning.pcr_fit` — principal component regression.
/// Args: { x: [[f64]], y: [f64], n_components: u64 }
pub fn pcr_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "pcr_fit needs x: [[f64]]"))?;
    let y = args::rec_f64_list(args, "y").ok_or_else(|| args::bad(span, "pcr_fit needs y"))?;
    let n_components = args::rec_u64(args, "n_components").unwrap_or(p as u64) as usize;
    match ml::regression::pcr::PcrModel::fit(&x, &y, n, p, n_components) {
        Ok(model) => {
            let preds = model.predict(&x, n);
            Ok(args::record([
                ("n_components", Value::U64(n_components as u64)),
                ("n", Value::U64(n as u64)),
                ("p", Value::U64(p as u64)),
                ("predictions", args::f64_list_value(preds)),
            ]))
        }
        Err(e) => Err(args::bad(span, format!("pcr_fit: {e:?}"))),
    }
}

// ── Bayesian linear regression ──────────────────────────────────────

/// `MachineLearning.bayesian_linear_fit` — Bayesian linear regression.
/// Args: { x: [[f64]], y: [f64], alpha: f64, beta: f64, intercept: bool }
pub fn bayesian_linear_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) = parse_matrix(args, "x")
        .ok_or_else(|| args::bad(span, "bayesian_linear_fit needs x: [[f64]]"))?;
    let y = args::rec_f64_list(args, "y")
        .ok_or_else(|| args::bad(span, "bayesian_linear_fit needs y"))?;
    let alpha = args::rec_f64(args, "alpha").unwrap_or(1.0);
    let beta = args::rec_f64(args, "beta").unwrap_or(100.0);
    let intercept = args::rec_bool(args, "intercept").unwrap_or(true);
    match ml::regression::bayesian::BayesianLinear::fit(&x, &y, n, p, alpha, beta, intercept) {
        Ok(model) => Ok(args::record([
            ("mean", args::f64_list_value(model.mean)),
            ("cov", args::f64_list_value(model.cov)),
            ("beta", Value::F64(model.beta)),
            ("p", Value::U64(p as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("bayesian_linear_fit: {e:?}"))),
    }
}

// ── SOM ─────────────────────────────────────────────────────────────

/// `MachineLearning.som_train` — self-organizing map training.
/// Args: { data: [[f64]], grid_w: u64, grid_h: u64, epochs: u64, lr0: f64, sigma0: f64, seed: u64 }
pub fn som_train(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (data, n, dim) = parse_matrix(args, "data")
        .ok_or_else(|| args::bad(span, "som_train needs data: [[f64]]"))?;
    let grid_w = args::rec_u64(args, "grid_w").unwrap_or(5) as usize;
    let grid_h = args::rec_u64(args, "grid_h").unwrap_or(5) as usize;
    let epochs = args::rec_u64(args, "epochs").unwrap_or(100) as usize;
    let lr0 = args::rec_f64(args, "lr0").unwrap_or(0.1);
    let sigma0 = args::rec_f64(args, "sigma0").unwrap_or(1.0);
    let seed = args::rec_u64(args, "seed").unwrap_or(0);
    match ml::dimensionality::som::Som::train(
        &data, n, dim, grid_w, grid_h, epochs, lr0, sigma0, seed,
    ) {
        Ok(som) => Ok(args::record([
            ("grid_w", Value::U64(som.grid_w as u64)),
            ("grid_h", Value::U64(som.grid_h as u64)),
            ("dim", Value::U64(som.dim as u64)),
            ("n", Value::U64(n as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("som_train: {e:?}"))),
    }
}

// ── Kalman filter ───────────────────────────────────────────────────

/// `MachineLearning.kalman_new` — construct a Kalman filter and run one predict/update cycle.
/// Args: { f: [f64], h: [f64], q: [f64], r: [f64], x0: [f64], p0: [f64], nx: u64, nz: u64, z: [f64] }
pub fn kalman_new(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let f = args::rec_f64_list(args, "f").ok_or_else(|| args::bad(span, "kalman_new needs f"))?;
    let h = args::rec_f64_list(args, "h").ok_or_else(|| args::bad(span, "kalman_new needs h"))?;
    let q = args::rec_f64_list(args, "q").ok_or_else(|| args::bad(span, "kalman_new needs q"))?;
    let r = args::rec_f64_list(args, "r").ok_or_else(|| args::bad(span, "kalman_new needs r"))?;
    let x0 =
        args::rec_f64_list(args, "x0").ok_or_else(|| args::bad(span, "kalman_new needs x0"))?;
    let p0 =
        args::rec_f64_list(args, "p0").ok_or_else(|| args::bad(span, "kalman_new needs p0"))?;
    let nx =
        args::rec_u64(args, "nx").ok_or_else(|| args::bad(span, "kalman_new needs nx"))? as usize;
    let nz =
        args::rec_u64(args, "nz").ok_or_else(|| args::bad(span, "kalman_new needs nz"))? as usize;
    let z = args::rec_f64_list(args, "z").ok_or_else(|| args::bad(span, "kalman_new needs z"))?;
    let mut kf = ml::sequential::kalman::KalmanFilter::new(f, h, q, r, x0, p0, nx, nz)
        .map_err(|e| args::bad(span, format!("kalman_new: {e:?}")))?;
    kf.predict()
        .map_err(|e| args::bad(span, format!("kalman_new predict: {e:?}")))?;
    kf.update(&z)
        .map_err(|e| args::bad(span, format!("kalman_new update: {e:?}")))?;
    Ok(args::record([
        ("state", args::f64_list_value(kf.state().to_vec())),
        ("covariance", args::f64_list_value(kf.covariance().to_vec())),
        ("nx", Value::U64(nx as u64)),
        ("nz", Value::U64(nz as u64)),
    ]))
}

// ── Random forest ───────────────────────────────────────────────────

/// `MachineLearning.random_forest_fit_regressor` — regression random forest.
/// Args: { x: [[f64]], y: [f64], n_trees: u64, max_depth: u64, min_samples_split: u64, min_samples_leaf: u64, seed: u64 }
pub fn random_forest_fit_regressor(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) = parse_matrix(args, "x")
        .ok_or_else(|| args::bad(span, "random_forest_fit_regressor needs x: [[f64]]"))?;
    let y = args::rec_f64_list(args, "y")
        .ok_or_else(|| args::bad(span, "random_forest_fit_regressor needs y"))?;
    let n_trees = args::rec_u64(args, "n_trees").unwrap_or(10) as usize;
    let params = ml::trees::decision_tree::TreeParams {
        max_depth: args::rec_u64(args, "max_depth").unwrap_or(8) as usize,
        min_samples_split: args::rec_u64(args, "min_samples_split").unwrap_or(2) as usize,
        min_samples_leaf: args::rec_u64(args, "min_samples_leaf").unwrap_or(1) as usize,
        max_features: None,
        seed: args::rec_u64(args, "seed").unwrap_or(0),
    };
    let seed = args::rec_u64(args, "seed").unwrap_or(0);
    match ml::trees::random_forest::RandomForest::fit_regressor(&x, &y, n, p, n_trees, params, seed)
    {
        Ok(model) => {
            let preds = model.predict(&x, n);
            Ok(args::record([
                ("n_trees", Value::U64(model.n_trees() as u64)),
                ("n", Value::U64(n as u64)),
                ("p", Value::U64(p as u64)),
                ("predictions", args::f64_list_value(preds)),
            ]))
        }
        Err(e) => Err(args::bad(
            span,
            format!("random_forest_fit_regressor: {e:?}"),
        )),
    }
}

/// `MachineLearning.random_forest_fit_classifier` — classification random forest.
/// Args: { x: [[f64]], y: [u64], n_trees: u64, max_depth: u64, min_samples_split: u64, min_samples_leaf: u64, seed: u64 }
pub fn random_forest_fit_classifier(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) = parse_matrix(args, "x")
        .ok_or_else(|| args::bad(span, "random_forest_fit_classifier needs x: [[f64]]"))?;
    let y_u64 = args::rec_u64_list(args, "y")
        .ok_or_else(|| args::bad(span, "random_forest_fit_classifier needs y"))?;
    let y: Vec<usize> = y_u64.iter().map(|&v| v as usize).collect();
    let n_trees = args::rec_u64(args, "n_trees").unwrap_or(10) as usize;
    let params = ml::trees::decision_tree::TreeParams {
        max_depth: args::rec_u64(args, "max_depth").unwrap_or(8) as usize,
        min_samples_split: args::rec_u64(args, "min_samples_split").unwrap_or(2) as usize,
        min_samples_leaf: args::rec_u64(args, "min_samples_leaf").unwrap_or(1) as usize,
        max_features: None,
        seed: args::rec_u64(args, "seed").unwrap_or(0),
    };
    let seed = args::rec_u64(args, "seed").unwrap_or(0);
    match ml::trees::random_forest::RandomForest::fit_classifier(
        &x, &y, n, p, n_trees, params, seed,
    ) {
        Ok(model) => {
            let preds: Vec<usize> = (0..n)
                .map(|i| model.predict_class(&x[i * p..(i + 1) * p]))
                .collect();
            Ok(args::record([
                ("n_trees", Value::U64(model.n_trees() as u64)),
                ("n", Value::U64(n as u64)),
                ("p", Value::U64(p as u64)),
                (
                    "predictions",
                    Value::List(preds.iter().map(|&p| Value::U64(p as u64)).collect()),
                ),
            ]))
        }
        Err(e) => Err(args::bad(
            span,
            format!("random_forest_fit_classifier: {e:?}"),
        )),
    }
}

// ── Gradient boosting ───────────────────────────────────────────────

/// `MachineLearning.gradient_boosting_fit_regressor` — stage-wise tree boosting.
/// Args: { x: [[f64]], y: [f64], n_estimators: u64, learning_rate: f64, max_depth: u64, min_samples_split: u64, min_samples_leaf: u64, seed: u64 }
pub fn gradient_boosting_fit_regressor(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) = parse_matrix(args, "x")
        .ok_or_else(|| args::bad(span, "gradient_boosting_fit_regressor needs x: [[f64]]"))?;
    let y = args::rec_f64_list(args, "y")
        .ok_or_else(|| args::bad(span, "gradient_boosting_fit_regressor needs y"))?;
    let n_estimators = args::rec_u64(args, "n_estimators").unwrap_or(50) as usize;
    let learning_rate = args::rec_f64(args, "learning_rate").unwrap_or(0.1);
    let params = ml::trees::decision_tree::TreeParams {
        max_depth: args::rec_u64(args, "max_depth").unwrap_or(3) as usize,
        min_samples_split: args::rec_u64(args, "min_samples_split").unwrap_or(2) as usize,
        min_samples_leaf: args::rec_u64(args, "min_samples_leaf").unwrap_or(1) as usize,
        max_features: None,
        seed: args::rec_u64(args, "seed").unwrap_or(0),
    };
    match ml::trees::boosting::GradientBoosting::fit_regressor(
        &x,
        &y,
        n,
        p,
        n_estimators,
        learning_rate,
        params,
    ) {
        Ok(model) => {
            let preds = model.predict(&x, n);
            Ok(args::record([
                ("n_estimators", Value::U64(model.n_estimators() as u64)),
                ("n", Value::U64(n as u64)),
                ("p", Value::U64(p as u64)),
                ("predictions", args::f64_list_value(preds)),
            ]))
        }
        Err(e) => Err(args::bad(
            span,
            format!("gradient_boosting_fit_regressor: {e:?}"),
        )),
    }
}
