use super::*;

pub fn statistical_analysis(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::statistical_computing::{
        CorrelationMethod, DataType, DataValue, HypothesisType, PrivacyLevel,
        StatisticalComputingLibrary,
    };

    let v = parse_tool_args(args)?;
    let stat = json_str(&v, "stat", "mean");
    let dataset_id = v
        .get("dataset_id")
        .and_then(Value::as_str)
        .unwrap_or("ds")
        .to_string();
    let columns: Vec<String> = v
        .get("columns")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_else(|| vec!["x".to_string(), "y".to_string()]);

    let rows_val = v
        .get("rows")
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    let mut data = Vec::with_capacity(rows_val.len());
    for row in rows_val {
        let row_arr = row.as_array().ok_or(McpSystemError::InvalidParameters)?;
        let mut row_data = Vec::with_capacity(row_arr.len());
        for cell in row_arr {
            let dv = if let Some(n) = cell.as_f64() {
                DataValue::Float(n)
            } else if let Some(s) = cell.as_str() {
                DataValue::String(s.to_string())
            } else if let Some(b) = cell.as_bool() {
                DataValue::Boolean(b)
            } else {
                return Err(McpSystemError::InvalidParameters);
            };
            row_data.push(dv);
        }
        data.push(row_data);
    }

    let col_types: Vec<DataType> = columns.iter().map(|_| DataType::Float64).collect();

    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;
    lib.create_dataset(
        dataset_id.clone(),
        data,
        columns.clone(),
        col_types,
        PrivacyLevel::Public,
    )
    .map_err(|_| McpSystemError::InvalidParameters)?;

    let column = v
        .get("column")
        .and_then(Value::as_str)
        .unwrap_or(columns.first().map(|s| s.as_str()).unwrap_or("x"));
    let column_y = v
        .get("column_y")
        .and_then(Value::as_str)
        .unwrap_or(columns.get(1).map(|s| s.as_str()).unwrap_or("y"));

    // Multi-column selectors for the supervised/grouped operations. Each
    // defaults to the dataset's declared columns so a caller can rely on order.
    let str_array = |key: &str| -> Vec<String> {
        v.get(key)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|c| c.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };
    let feature_columns: Vec<String> = {
        let f = str_array("feature_columns");
        if f.is_empty() {
            vec![column.to_string()]
        } else {
            f
        }
    };
    let feature_refs: Vec<&str> = feature_columns.iter().map(String::as_str).collect();
    let group_columns: Vec<String> = {
        let g = str_array("group_columns");
        if g.is_empty() {
            columns.clone()
        } else {
            g
        }
    };
    let group_refs: Vec<&str> = group_columns.iter().map(String::as_str).collect();
    let label_column = v
        .get("label_column")
        .and_then(Value::as_str)
        .unwrap_or(column_y);
    let sample = json_bool(&v, "sample", true);

    let result = match stat {
        "variance" => {
            let r = lib
                .variance(&dataset_id, column, sample, false)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "variance", "column": column, "value": r.result})
        }
        "correlation" => {
            let method = match json_str(&v, "method", "pearson") {
                "spearman" => CorrelationMethod::Spearman,
                "kendall" => CorrelationMethod::Kendall,
                _ => CorrelationMethod::Pearson,
            };
            let r = lib
                .correlation(&dataset_id, column, column_y, method, false)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({
                "stat": "correlation",
                "column_x": column,
                "column_y": column_y,
                "value": r.result
            })
        }
        "mean" => {
            let r = lib
                .mean(&dataset_id, column, false)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "mean", "column": column, "value": r.result})
        }
        "median" => {
            let r = lib
                .median(&dataset_id, column, false)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "median", "column": column, "value": r.result})
        }
        "mode" => {
            let r = lib
                .mode(&dataset_id, column)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "mode", "column": column, "value": r.value, "count": r.count, "n": r.sample_size})
        }
        "std" | "stddev" | "standard_deviation" => {
            let r = lib
                .standard_deviation(&dataset_id, column, sample)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "standard_deviation", "column": column, "value": r.result, "sample": sample})
        }
        "skewness" => {
            let r = lib
                .skewness(&dataset_id, column)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "skewness", "column": column, "value": r.result})
        }
        "kurtosis" => {
            let r = lib
                .kurtosis(&dataset_id, column)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "kurtosis", "column": column, "value": r.result})
        }
        "quantile" => {
            let q = v.get("q").and_then(Value::as_f64).unwrap_or(0.5);
            let r = lib
                .quantile(&dataset_id, column, q)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "quantile", "column": column, "q": q, "value": r.result})
        }
        "percentile" => {
            let p = v.get("percentile").and_then(Value::as_f64).unwrap_or(50.0);
            let r = lib
                .quantile(&dataset_id, column, p / 100.0)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "percentile", "column": column, "percentile": p, "value": r.result})
        }
        "covariance" => {
            let r = lib
                .covariance(&dataset_id, column, column_y, sample)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "covariance", "column_x": column, "column_y": column_y, "value": r.result, "sample": sample})
        }
        "histogram" => {
            let bins = v.get("bins").and_then(Value::as_u64).unwrap_or(10) as usize;
            let r = lib
                .histogram(&dataset_id, column, bins, false)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({
                "stat": "histogram", "column": column, "bins": r.result.bins,
                "counts": r.result.counts, "min": r.result.min_value,
                "max": r.result.max_value, "bin_width": r.result.bin_width
            })
        }
        "ttest" | "t_test" => {
            let hyp = match json_str(&v, "hypothesis", "one_sample") {
                "two_sample" => HypothesisType::TwoSample,
                "paired" => HypothesisType::Paired,
                "independent" => HypothesisType::Independent,
                _ => HypothesisType::OneSample,
            };
            let r = lib
                .t_test(&dataset_id, column, hyp, false)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({
                "stat": "t_test", "column": column,
                "t_statistic": r.result.t_statistic, "p_value": r.result.p_value,
                "degrees_of_freedom": r.result.degrees_of_freedom
            })
        }
        "linear_regression" | "regression" => {
            let r = lib
                .linear_regression(&dataset_id, column, column_y)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({
                "stat": "linear_regression", "x": column, "y": column_y,
                "slope": r.slope, "intercept": r.intercept, "r_squared": r.r_squared,
                "slope_std_error": r.slope_std_error, "slope_t": r.slope_t,
                "slope_p_value": r.slope_p_value, "n": r.n
            })
        }
        "polynomial_regression" | "poly" => {
            let degree = v.get("degree").and_then(Value::as_u64).unwrap_or(2) as usize;
            let r = lib
                .polynomial_regression(&dataset_id, column, column_y, degree)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({
                "stat": "polynomial_regression", "x": column, "y": column_y,
                "degree": r.degree, "coefficients": r.coefficients,
                "r_squared": r.r_squared, "n": r.n
            })
        }
        "logistic_regression" | "logistic" => {
            let fit_intercept = json_bool(&v, "fit_intercept", true);
            let r = lib
                .logistic_regression(&dataset_id, &feature_refs, label_column, fit_intercept)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({
                "stat": "logistic_regression", "features": feature_columns,
                "label": label_column, "coefficients": r.coefficients,
                "std_errors": r.std_errors, "z_values": r.z_values, "p_values": r.p_values,
                "converged": r.converged, "deviance": r.deviance, "n": r.n
            })
        }
        "anova" => {
            let r = lib
                .anova(&dataset_id, &group_refs)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({
                "stat": "anova", "groups": group_columns,
                "f_statistic": r.f_statistic, "p_value": r.p_value,
                "df_between": r.df_between, "df_within": r.df_within,
                "ms_between": r.ms_between, "ms_within": r.ms_within
            })
        }
        "chi_square" | "chi_square_gof" => {
            let expected = v.get("expected_column").and_then(Value::as_str);
            let r = lib
                .chi_square_gof(&dataset_id, column, expected)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({
                "stat": "chi_square_gof", "observed": column, "expected_column": expected,
                "statistic": r.statistic, "p_value": r.p_value, "dof": r.dof
            })
        }
        "chi_square_independence" => {
            let r = lib
                .chi_square_independence(&dataset_id, &group_refs)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({
                "stat": "chi_square_independence", "columns": group_columns,
                "statistic": r.statistic, "p_value": r.p_value, "dof": r.dof
            })
        }
        "autocorrelation" | "acf" => {
            let lag = v.get("lag").and_then(Value::as_u64).unwrap_or(1) as usize;
            let r = lib
                .autocorrelation(&dataset_id, column, lag)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "autocorrelation", "column": column, "lag": lag, "value": r.result})
        }
        "moving_average" | "sma" => {
            let window = v.get("window").and_then(Value::as_u64).unwrap_or(3) as usize;
            let series = lib
                .moving_average(&dataset_id, column, window)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "moving_average", "column": column, "window": window, "series": series})
        }
        "exponential_smoothing" | "ewma" => {
            let alpha = v.get("alpha").and_then(Value::as_f64).unwrap_or(0.3);
            let series = lib
                .exponential_smoothing(&dataset_id, column, alpha)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({"stat": "exponential_smoothing", "column": column, "alpha": alpha, "series": series})
        }
        "kmeans" => {
            let k = v.get("k").and_then(Value::as_u64).unwrap_or(2) as usize;
            let max_iter = v.get("max_iter").and_then(Value::as_u64).unwrap_or(50) as usize;
            let seed = v.get("seed").and_then(Value::as_u64).unwrap_or(42);
            let m = lib
                .kmeans(&dataset_id, &feature_refs, k, max_iter, seed)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({
                "stat": "kmeans", "features": feature_columns, "k": m.k,
                "labels": m.labels, "centroids": m.centroids, "inertia": m.inertia,
                "n_iter": m.n_iter, "converged": m.converged
            })
        }
        "svm" | "linear_svm" => {
            let c = v.get("c").and_then(Value::as_f64).unwrap_or(1.0);
            let r = lib
                .linear_svm(&dataset_id, &feature_refs, label_column, c)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({
                "stat": "linear_svm", "features": feature_columns, "label": label_column,
                "n_support_vectors": r.n_support_vectors, "train_accuracy": r.train_accuracy,
                "n": r.n, "n_features": r.n_features
            })
        }
        "random_forest" => {
            let n_trees = v.get("n_trees").and_then(Value::as_u64).unwrap_or(64) as usize;
            let classifier = json_bool(&v, "classifier", false);
            let seed = v.get("seed").and_then(Value::as_u64).unwrap_or(42);
            let r = lib
                .random_forest(
                    &dataset_id,
                    &feature_refs,
                    label_column,
                    n_trees,
                    classifier,
                    seed,
                )
                .map_err(|_| McpSystemError::InvalidParameters)?;
            json!({
                "stat": "random_forest", "features": feature_columns, "target": label_column,
                "n_trees": r.n_trees, "classifier": r.classifier,
                "train_metric": r.train_metric, "n": r.n, "n_features": r.n_features
            })
        }
        _ => return Err(McpSystemError::InvalidParameters),
    };

    Ok(result.to_string())
}
