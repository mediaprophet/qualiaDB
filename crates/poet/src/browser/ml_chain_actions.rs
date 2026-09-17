//! Dual-path Tool Chest actions for curated `MachineLearning.*` ALL_BOUND ids.
//!
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.

use serde_json::json;
use web_sys::{Document, Element};

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn selected_source(document: &Document) -> Option<String> {
    let container = selected_container(document)?;
    let text = container
        .query_selector(".vibe-editor, .vibe-editor-textarea, .doc-editor, .sheet-grid")
        .ok()
        .flatten()
        .and_then(|editor| editor.text_content())
        .or_else(|| container.text_content())?;
    let bounded: String = text.chars().take(16_384).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn parse_numbers(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(4096)
        .collect()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
}

/// Split surface numbers into equal y_true / y_pred halves (drop odd tail).
fn split_pair(nums: &[f64]) -> Option<(Vec<f64>, Vec<f64>)> {
    if nums.len() < 2 {
        return None;
    }
    let half = nums.len() / 2;
    if half == 0 {
        return None;
    }
    Some((nums[..half].to_vec(), nums[half..half * 2].to_vec()))
}

fn local_mse(y_true: &[f64], y_pred: &[f64]) -> Option<f64> {
    if y_true.is_empty() || y_true.len() != y_pred.len() {
        return None;
    }
    let n = y_true.len() as f64;
    let sum: f64 = y_true
        .iter()
        .zip(y_pred.iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum();
    let v = sum / n;
    v.is_finite().then_some(v)
}

fn local_mae(y_true: &[f64], y_pred: &[f64]) -> Option<f64> {
    if y_true.is_empty() || y_true.len() != y_pred.len() {
        return None;
    }
    let n = y_true.len() as f64;
    let sum: f64 = y_true
        .iter()
        .zip(y_pred.iter())
        .map(|(a, b)| (a - b).abs())
        .sum();
    let v = sum / n;
    v.is_finite().then_some(v)
}

fn local_r2(y_true: &[f64], y_pred: &[f64]) -> Option<f64> {
    if y_true.len() < 2 || y_true.len() != y_pred.len() {
        return None;
    }
    let mean = y_true.iter().sum::<f64>() / y_true.len() as f64;
    let ss_tot: f64 = y_true.iter().map(|y| (y - mean) * (y - mean)).sum();
    let ss_res: f64 = y_true
        .iter()
        .zip(y_pred.iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum();
    if ss_tot <= 0.0 {
        return None;
    }
    let v = 1.0 - ss_res / ss_tot;
    v.is_finite().then_some(v)
}

fn local_accuracy(y_true: &[u64], y_pred: &[u64]) -> Option<f64> {
    if y_true.is_empty() || y_true.len() != y_pred.len() {
        return None;
    }
    let hits = y_true
        .iter()
        .zip(y_pred.iter())
        .filter(|(a, b)| a == b)
        .count();
    Some(hits as f64 / y_true.len() as f64)
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: serde_json::Value,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
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
        &format!("Running {cap_id}…"),
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_invoke(cap_id, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(cap_id, &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
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

fn need_pair(document: &Document, label: &str) -> Option<(Vec<f64>, Vec<f64>)> {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    match split_pair(&nums) {
        Some(pair) => Some(pair),
        None => {
            super::interactions::show_tool_status(
                document,
                label,
                "Select a sheet or document with an even count of numbers (y_true then y_pred).",
                "error",
            );
            None
        }
    }
}

/// `MachineLearning.mse` — first half y_true, second half y_pred.
pub(super) fn run_mse(document: &Document, label: &str) {
    let Some((y_true, y_pred)) = need_pair(document, label) else {
        return;
    };
    let sketch = match local_mse(&y_true, &y_pred) {
        Some(v) => format!("MSE sketch over {} pairs: {v:.6}", y_true.len()),
        None => format!("MSE sketch inputs n={}", y_true.len()),
    };
    invoke_dual(
        document,
        label,
        "MachineLearning.mse",
        sketch,
        json!({ "y_true": y_true, "y_pred": y_pred }),
    );
}

/// `MachineLearning.rmse`
pub(super) fn run_rmse(document: &Document, label: &str) {
    let Some((y_true, y_pred)) = need_pair(document, label) else {
        return;
    };
    let sketch = match local_mse(&y_true, &y_pred) {
        Some(v) => format!("RMSE sketch over {} pairs: {:.6}", y_true.len(), v.sqrt()),
        None => format!("RMSE sketch inputs n={}", y_true.len()),
    };
    invoke_dual(
        document,
        label,
        "MachineLearning.rmse",
        sketch,
        json!({ "y_true": y_true, "y_pred": y_pred }),
    );
}

/// `MachineLearning.mae`
pub(super) fn run_mae(document: &Document, label: &str) {
    let Some((y_true, y_pred)) = need_pair(document, label) else {
        return;
    };
    let sketch = match local_mae(&y_true, &y_pred) {
        Some(v) => format!("MAE sketch over {} pairs: {v:.6}", y_true.len()),
        None => format!("MAE sketch inputs n={}", y_true.len()),
    };
    invoke_dual(
        document,
        label,
        "MachineLearning.mae",
        sketch,
        json!({ "y_true": y_true, "y_pred": y_pred }),
    );
}

/// `MachineLearning.r2_score`
pub(super) fn run_r2_score(document: &Document, label: &str) {
    let Some((y_true, y_pred)) = need_pair(document, label) else {
        return;
    };
    let sketch = match local_r2(&y_true, &y_pred) {
        Some(v) => format!("R² sketch over {} pairs: {v:.6}", y_true.len()),
        None => format!("R² sketch inputs n={} (need variance in y_true)", y_true.len()),
    };
    invoke_dual(
        document,
        label,
        "MachineLearning.r2_score",
        sketch,
        json!({ "y_true": y_true, "y_pred": y_pred }),
    );
}

/// `MachineLearning.accuracy` — rounded ints from paired halves.
pub(super) fn run_accuracy(document: &Document, label: &str) {
    let Some((a, b)) = need_pair(document, label) else {
        return;
    };
    let y_true: Vec<u64> = a.iter().map(|v| v.round().max(0.0) as u64).collect();
    let y_pred: Vec<u64> = b.iter().map(|v| v.round().max(0.0) as u64).collect();
    let sketch = match local_accuracy(&y_true, &y_pred) {
        Some(v) => format!("Accuracy sketch over {} labels: {v:.4}", y_true.len()),
        None => format!("Accuracy sketch inputs n={}", y_true.len()),
    };
    invoke_dual(
        document,
        label,
        "MachineLearning.accuracy",
        sketch,
        json!({ "y_true": y_true, "y_pred": y_pred }),
    );
}

/// `MachineLearning.ols` — pairs (x_i, y_i) as 1-feature regression; else demo line.
pub(super) fn run_ols(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x_rows, y) = if nums.len() >= 4 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        let xs = nums[..half].to_vec();
        let ys = nums[half..].to_vec();
        let rows: Vec<Vec<f64>> = xs.into_iter().map(|x| vec![x]).collect();
        (rows, ys)
    } else {
        (
            vec![vec![1.0], vec![2.0], vec![3.0], vec![4.0]],
            vec![3.0, 5.0, 7.0, 9.0],
        )
    };
    invoke_dual(
        document,
        label,
        "MachineLearning.ols",
        format!(
            "OLS sketch: {} rows × 1 feature (intercept on). Connect QualiaDB for live fit.",
            x_rows.len()
        ),
        json!({
            "x": x_rows,
            "y": y,
            "intercept": true,
        }),
    );
}

/// `MachineLearning.train_test_split` — n from surface count or `data-n`.
pub(super) fn run_train_test_split(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let n = numeric_attr(container.as_ref(), "data-n")
        .map(|v| v.round().max(2.0) as u64)
        .unwrap_or_else(|| nums.len().max(10) as u64);
    let test_ratio = numeric_attr(container.as_ref(), "data-test-ratio").unwrap_or(0.25);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    let test_n = ((n as f64) * test_ratio).round() as u64;
    invoke_dual(
        document,
        label,
        "MachineLearning.train_test_split",
        format!("Train/test split sketch: n={n} test_ratio={test_ratio} → ~{test_n} test rows"),
        json!({
            "n": n,
            "test_ratio": test_ratio,
            "seed": seed,
        }),
    );
}

/// `MachineLearning.kmeans` — row-major data; `data-n` / `data-p` / `data-k`.
pub(super) fn run_kmeans(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let mut data = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(2);
    let k = numeric_attr(container.as_ref(), "data-k")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(2);
    let n = if !data.is_empty() && p > 0 {
        (data.len() as u64) / p
    } else {
        numeric_attr(container.as_ref(), "data-n")
            .map(|v| v.round().max(1.0) as u64)
            .unwrap_or(4)
    };
    if data.len() < (n * p) as usize {
        // Compact 2-D demo cloud when the surface has no matrix.
        data = vec![0.0, 0.0, 0.1, 0.2, 5.0, 5.0, 5.1, 4.9];
    }
    let n = (data.len() as u64) / p;
    let max_iter = numeric_attr(container.as_ref(), "data-max-iter")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(100);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    invoke_dual(
        document,
        label,
        "MachineLearning.kmeans",
        format!("k-means sketch: n={n} p={p} k={k} (offline message only)"),
        json!({
            "data": data,
            "n": n,
            "p": p,
            "k": k,
            "max_iter": max_iter,
            "seed": seed,
        }),
    );
}

/// `MachineLearning.log_loss` — first half probs, second half labels (>0.5 → true).
pub(super) fn run_log_loss(document: &Document, label: &str) {
    let Some((probs, raw_labels)) = need_pair(document, label) else {
        return;
    };
    let labels: Vec<bool> = raw_labels.iter().map(|v| *v > 0.5).collect();
    let clipped: Vec<f64> = probs
        .iter()
        .map(|p| p.clamp(1e-9, 1.0 - 1e-9))
        .collect();
    let sketch_loss = {
        let n = clipped.len() as f64;
        let sum: f64 = clipped
            .iter()
            .zip(labels.iter())
            .map(|(p, y)| {
                if *y {
                    -p.ln()
                } else {
                    -(1.0 - p).ln()
                }
            })
            .sum();
        sum / n
    };
    invoke_dual(
        document,
        label,
        "MachineLearning.log_loss",
        format!(
            "Log-loss sketch over {} pairs: {sketch_loss:.6}",
            clipped.len()
        ),
        json!({ "probs": clipped, "labels": labels }),
    );
}

/// `MachineLearning.bonferroni` — p-values from surface numbers.
pub(super) fn run_bonferroni(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let mut p = parse_numbers(&source);
    if p.is_empty() {
        p = vec![0.01, 0.04, 0.03, 0.20];
    }
    let m = p.len() as f64;
    let adjusted: Vec<f64> = p.iter().map(|v| (v * m).min(1.0)).collect();
    invoke_dual(
        document,
        label,
        "MachineLearning.bonferroni",
        format!(
            "Bonferroni sketch over {} p-values → {:?}",
            p.len(),
            adjusted
                .iter()
                .map(|v| format!("{v:.4}"))
                .collect::<Vec<_>>()
        ),
        json!({ "p": p }),
    );
}

/// `MachineLearning.confusion_binary` — paired halves as bools (>0.5).
pub(super) fn run_confusion_binary(document: &Document, label: &str) {
    let Some((a, b)) = need_pair(document, label) else {
        return;
    };
    let y_true: Vec<bool> = a.iter().map(|v| *v > 0.5).collect();
    let y_pred: Vec<bool> = b.iter().map(|v| *v > 0.5).collect();
    let mut tp = 0u64;
    let mut fp = 0u64;
    let mut tn = 0u64;
    let mut fn_ = 0u64;
    for (t, p) in y_true.iter().zip(y_pred.iter()) {
        match (*t, *p) {
            (true, true) => tp += 1,
            (false, true) => fp += 1,
            (false, false) => tn += 1,
            (true, false) => fn_ += 1,
        }
    }
    invoke_dual(
        document,
        label,
        "MachineLearning.confusion_binary",
        format!("Confusion sketch: tp={tp} fp={fp} tn={tn} fn={fn_}"),
        json!({ "y_true": y_true, "y_pred": y_pred }),
    );
}

/// Holm step-down adjusted p-values (original order). Offline sketch only.
fn local_holm(p: &[f64]) -> Vec<f64> {
    let m = p.len();
    if m == 0 {
        return Vec::new();
    }
    let mut order: Vec<usize> = (0..m).collect();
    order.sort_by(|&a, &b| {
        p[a]
            .partial_cmp(&p[b])
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    let mut adj = vec![0.0; m];
    let mut running = 0.0_f64;
    for (rank, &idx) in order.iter().enumerate() {
        let val = ((m - rank) as f64 * p[idx]).min(1.0);
        running = running.max(val);
        adj[idx] = running;
    }
    adj
}

/// Benjamini–Hochberg FDR adjusted p-values (original order). Offline sketch only.
fn local_benjamini_hochberg(p: &[f64]) -> Vec<f64> {
    let m = p.len();
    if m == 0 {
        return Vec::new();
    }
    let mut order: Vec<usize> = (0..m).collect();
    order.sort_by(|&a, &b| {
        p[a]
            .partial_cmp(&p[b])
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    let mf = m as f64;
    let mut adj = vec![0.0; m];
    let mut running = f64::INFINITY;
    for rank in (0..m).rev() {
        let idx = order[rank];
        let k = (rank + 1) as f64;
        let val = (mf / k * p[idx]).min(1.0);
        running = running.min(val);
        adj[idx] = running;
    }
    adj
}

fn local_n_rejected(adjusted: &[f64], alpha: f64) -> usize {
    adjusted.iter().filter(|&&q| q <= alpha).count()
}

fn local_mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let m = values.iter().sum::<f64>() / values.len() as f64;
    m.is_finite().then_some(m)
}

fn format_p_list(values: &[f64]) -> Vec<String> {
    values.iter().map(|v| format!("{v:.4}")).collect()
}

/// `MachineLearning.holm` — Holm–Bonferroni correction.
pub(super) fn run_holm(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let mut p = parse_numbers(&source);
    if p.is_empty() {
        p = vec![0.01, 0.04, 0.03, 0.20];
    }
    let adjusted = local_holm(&p);
    invoke_dual(
        document,
        label,
        "MachineLearning.holm",
        format!(
            "Holm sketch over {} p-values → {:?}",
            p.len(),
            format_p_list(&adjusted)
        ),
        json!({ "p": p }),
    );
}

/// `MachineLearning.benjamini_hochberg` — BH FDR correction.
pub(super) fn run_benjamini_hochberg(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let mut p = parse_numbers(&source);
    if p.is_empty() {
        p = vec![0.01, 0.04, 0.03, 0.20];
    }
    let adjusted = local_benjamini_hochberg(&p);
    invoke_dual(
        document,
        label,
        "MachineLearning.benjamini_hochberg",
        format!(
            "BH FDR sketch over {} p-values → {:?}",
            p.len(),
            format_p_list(&adjusted)
        ),
        json!({ "p": p }),
    );
}

/// `MachineLearning.ab_test` — two-proportion z-test.
/// Surface: four ints `conv_a n_a conv_b n_b`, or `data-conv-a` / `data-n-a` / …
pub(super) fn run_ab_test(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (conv_a, n_a, conv_b, n_b) = if nums.len() >= 4 {
        (
            nums[0].round().max(0.0) as u64,
            nums[1].round().max(1.0) as u64,
            nums[2].round().max(0.0) as u64,
            nums[3].round().max(1.0) as u64,
        )
    } else {
        (
            numeric_attr(container.as_ref(), "data-conv-a")
                .map(|v| v.round().max(0.0) as u64)
                .unwrap_or(45),
            numeric_attr(container.as_ref(), "data-n-a")
                .map(|v| v.round().max(1.0) as u64)
                .unwrap_or(1_000),
            numeric_attr(container.as_ref(), "data-conv-b")
                .map(|v| v.round().max(0.0) as u64)
                .unwrap_or(60),
            numeric_attr(container.as_ref(), "data-n-b")
                .map(|v| v.round().max(1.0) as u64)
                .unwrap_or(1_000),
        )
    };
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(0.05);
    let rate_a = conv_a as f64 / n_a as f64;
    let rate_b = conv_b as f64 / n_b as f64;
    invoke_dual(
        document,
        label,
        "MachineLearning.ab_test",
        format!(
            "A/B sketch: rate_a={rate_a:.4} rate_b={rate_b:.4} Δ={:.4} (α={alpha})",
            rate_b - rate_a
        ),
        json!({
            "conv_a": conv_a,
            "n_a": n_a,
            "conv_b": conv_b,
            "n_b": n_b,
            "alpha": alpha,
        }),
    );
}

/// `MachineLearning.bootstrap_estimate` — SE / bias for mean (default).
pub(super) fn run_bootstrap_estimate(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let mut data = parse_numbers(&source);
    if data.is_empty() {
        data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    }
    let b = numeric_attr(container.as_ref(), "data-b")
        .map(|v| v.round().max(10.0) as u64)
        .unwrap_or(200);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    let mean = local_mean(&data).unwrap_or(0.0);
    let var = if data.len() > 1 {
        let n = data.len() as f64;
        data.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (n - 1.0)
    } else {
        0.0
    };
    let se_sketch = (var / data.len() as f64).sqrt();
    invoke_dual(
        document,
        label,
        "MachineLearning.bootstrap_estimate",
        format!(
            "Bootstrap estimate sketch: mean≈{mean:.4} SE≈{se_sketch:.4} (b={b}, offline)"
        ),
        json!({
            "data": data,
            "b": b,
            "seed": seed,
            "statistic": "mean",
        }),
    );
}

/// `MachineLearning.bootstrap_ci` — percentile CI for the mean.
pub(super) fn run_bootstrap_ci(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let mut data = parse_numbers(&source);
    if data.is_empty() {
        data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    }
    let b = numeric_attr(container.as_ref(), "data-b")
        .map(|v| v.round().max(10.0) as u64)
        .unwrap_or(200);
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(0.05);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    let mean = local_mean(&data).unwrap_or(0.0);
    let var = if data.len() > 1 {
        let n = data.len() as f64;
        data.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (n - 1.0)
    } else {
        0.0
    };
    let se = (var / data.len() as f64).sqrt();
    let z = 1.96; // rough normal sketch for 95% when alpha≈0.05
    let half = z * se;
    invoke_dual(
        document,
        label,
        "MachineLearning.bootstrap_ci",
        format!(
            "Bootstrap CI sketch: mean≈{mean:.4} ≈[{:.4}, {:.4}] (α={alpha}, offline)",
            mean - half,
            mean + half
        ),
        json!({
            "data": data,
            "b": b,
            "alpha": alpha,
            "seed": seed,
            "method": "percentile",
            "statistic": "mean",
        }),
    );
}

/// `MachineLearning.permutation_test` — two-sample mean difference.
/// Surface: equal halves as groups a / b.
pub(super) fn run_permutation_test(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (a, b) = match split_pair(&nums) {
        Some(pair) => pair,
        None => (
            vec![1.0, 2.0, 3.0, 4.0],
            vec![3.0, 4.0, 5.0, 6.0],
        ),
    };
    let n_perm = numeric_attr(container.as_ref(), "data-n-perm")
        .map(|v| v.round().max(10.0) as u64)
        .unwrap_or(500);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    let ma = local_mean(&a).unwrap_or(0.0);
    let mb = local_mean(&b).unwrap_or(0.0);
    invoke_dual(
        document,
        label,
        "MachineLearning.permutation_test",
        format!(
            "Permutation sketch: observed mean Δ={:.4} (|a|={}, |b|={}, n_perm={n_perm})",
            ma - mb,
            a.len(),
            b.len()
        ),
        json!({
            "a": a,
            "b": b,
            "n_perm": n_perm,
            "seed": seed,
            "statistic": "mean",
        }),
    );
}

/// `MachineLearning.n_rejected` — count adjusted p ≤ alpha.
pub(super) fn run_n_rejected(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let mut adjusted = parse_numbers(&source);
    if adjusted.is_empty() {
        adjusted = vec![0.01, 0.04, 0.12, 0.40];
    }
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(0.05);
    let count = local_n_rejected(&adjusted, alpha);
    invoke_dual(
        document,
        label,
        "MachineLearning.n_rejected",
        format!("n_rejected sketch: {count} of {} at α={alpha}", adjusted.len()),
        json!({ "adjusted": adjusted, "alpha": alpha }),
    );
}

/// `MachineLearning.power_two_sample` — power for effect size d.
pub(super) fn run_power_two_sample(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let n = numeric_attr(container.as_ref(), "data-n")
        .map(|v| v.round().max(2.0) as u64)
        .or_else(|| nums.first().map(|v| v.round().max(2.0) as u64))
        .unwrap_or(50);
    let d = numeric_attr(container.as_ref(), "data-d")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(0.5);
    let alpha = numeric_attr(container.as_ref(), "data-alpha")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(0.05);
    // Rough normal-power sketch: Φ(z_{1-α/2} terms omitted) — message only.
    let sketch = format!("Power sketch inputs: n={n} d={d:.3} α={alpha} (connect for live)");
    invoke_dual(
        document,
        label,
        "MachineLearning.power_two_sample",
        sketch,
        json!({ "n": n, "d": d, "alpha": alpha }),
    );
}

/// `MachineLearning.roc_auc` — first half scores, second half labels (>0.5 → true).
pub(super) fn run_roc_auc(document: &Document, label: &str) {
    let Some((scores, raw_labels)) = need_pair(document, label) else {
        return;
    };
    let labels: Vec<bool> = raw_labels.iter().map(|v| *v > 0.5).collect();
    let pos = labels.iter().filter(|&&y| y).count();
    let neg = labels.len().saturating_sub(pos);
    invoke_dual(
        document,
        label,
        "MachineLearning.roc_auc",
        format!(
            "ROC AUC sketch: {} scores ({} pos / {} neg) — connect for live",
            scores.len(),
            pos,
            neg
        ),
        json!({ "scores": scores, "labels": labels }),
    );
}

/// `MachineLearning.k_fold` — n / k from surface count or `data-n` / `data-k`.
pub(super) fn run_k_fold(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let n = numeric_attr(container.as_ref(), "data-n")
        .map(|v| v.round().max(2.0) as u64)
        .unwrap_or_else(|| nums.len().max(10) as u64);
    let k = numeric_attr(container.as_ref(), "data-k")
        .map(|v| v.round().max(2.0) as u64)
        .or_else(|| nums.first().map(|v| v.round().max(2.0) as u64))
        .unwrap_or(5)
        .min(n);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    let fold_size = n / k;
    invoke_dual(
        document,
        label,
        "MachineLearning.k_fold",
        format!("k-fold sketch: n={n} k={k} ≈{fold_size} test rows/fold (shuffle on)"),
        json!({
            "n": n,
            "k": k,
            "shuffle": true,
            "seed": seed,
        }),
    );
}

/// `MachineLearning.bootstrap_indices` — resampled index list of length n.
pub(super) fn run_bootstrap_indices(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let n = numeric_attr(container.as_ref(), "data-n")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or_else(|| nums.len().max(8) as u64);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    invoke_dual(
        document,
        label,
        "MachineLearning.bootstrap_indices",
        format!("Bootstrap indices sketch: n={n} seed={seed} (offline message only)"),
        json!({ "n": n, "seed": seed }),
    );
}

/// `MachineLearning.pca` — row-major data; `data-n` / `data-p`.
pub(super) fn run_pca(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let mut data = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(2);
    let n = if !data.is_empty() && p > 0 {
        (data.len() as u64) / p
    } else {
        numeric_attr(container.as_ref(), "data-n")
            .map(|v| v.round().max(2.0) as u64)
            .unwrap_or(4)
    };
    if data.len() < (n * p) as usize {
        // Compact 2-D demo cloud when the surface has no matrix.
        data = vec![0.0, 0.0, 0.1, 0.2, 5.0, 5.0, 5.1, 4.9];
    }
    let n = (data.len() as u64) / p;
    invoke_dual(
        document,
        label,
        "MachineLearning.pca",
        format!("PCA sketch: n={n} p={p} (offline message only)"),
        json!({
            "data": data,
            "n": n,
            "p": p,
        }),
    );
}

/// `MachineLearning.required_sample_size` — n for two-sample power (d, α, power).
pub(super) fn run_required_sample_size(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let d = numeric_attr(container.as_ref(), "data-d")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.5);
    let alpha = numeric_attr(container.as_ref(), "data-alpha")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(0.05);
    let power = numeric_attr(container.as_ref(), "data-power")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(0.8);
    // Rough normal sketch: n ≈ 2 * ((z_{1-α/2}+z_power)/d)^2 — message only.
    let z_a = 1.96;
    let z_p = 0.84; // ~80% power
    let n_sketch = (2.0 * ((z_a + z_p) / d).powi(2)).ceil();
    invoke_dual(
        document,
        label,
        "MachineLearning.required_sample_size",
        format!("Required n sketch: d={d:.3} α={alpha} power={power} → ≈{n_sketch:.0}"),
        json!({ "d": d, "alpha": alpha, "power": power }),
    );
}

/// `MachineLearning.polynomial_regression` — degree-d fit from x / y halves.
pub(super) fn run_polynomial_regression(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let degree = numeric_attr(container.as_ref(), "data-degree")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(2);
    let (x, y) = if nums.len() >= 4 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else {
        (
            vec![0.0, 1.0, 2.0, 3.0, 4.0],
            vec![1.0, 2.0, 5.0, 10.0, 17.0],
        )
    };
    let n = x.len().min(y.len()) as u64;
    invoke_dual(
        document,
        label,
        "MachineLearning.polynomial_regression",
        format!("Polynomial regression sketch: n={n} degree={degree} (connect for live)"),
        json!({
            "x": x,
            "y": y,
            "n": n,
            "degree": degree,
        }),
    );
}

/// `MachineLearning.required_sample_size_two_proportion` — n for two rates.
pub(super) fn run_required_sample_size_two_proportion(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p1 = numeric_attr(container.as_ref(), "data-p1")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.10);
    let p2 = numeric_attr(container.as_ref(), "data-p2")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(0.15);
    let alpha = numeric_attr(container.as_ref(), "data-alpha")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(0.05);
    let power = numeric_attr(container.as_ref(), "data-power")
        .or_else(|| nums.get(3).copied())
        .unwrap_or(0.8);
    let delta = (p2 - p1).abs();
    invoke_dual(
        document,
        label,
        "MachineLearning.required_sample_size_two_proportion",
        format!(
            "Two-proportion n sketch: p1={p1:.3} p2={p2:.3} |Δ|={delta:.3} α={alpha} power={power}"
        ),
        json!({ "p1": p1, "p2": p2, "alpha": alpha, "power": power }),
    );
}

/// `MachineLearning.loocv` — leave-one-out folds for n samples.
pub(super) fn run_loocv(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let n = numeric_attr(container.as_ref(), "data-n")
        .map(|v| v.round().max(2.0) as u64)
        .unwrap_or_else(|| nums.len().max(5) as u64);
    invoke_dual(
        document,
        label,
        "MachineLearning.loocv",
        format!("LOOCV sketch: n={n} → {n} folds of 1 test row each"),
        json!({ "n": n }),
    );
}

/// Split surface numbers into three equal embedding vectors (h / r / t).
fn embedding_hrt(nums: &[f64], dim: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let dim = dim.max(1);
    if nums.len() >= dim * 3 {
        let h = nums[..dim].to_vec();
        let r = nums[dim..dim * 2].to_vec();
        let t = nums[dim * 2..dim * 3].to_vec();
        return (h, r, t);
    }
    // Compact demo embeddings when the surface has no triple.
    let mut h = vec![0.0; dim];
    let mut r = vec![0.0; dim];
    let mut t = vec![0.0; dim];
    if dim >= 1 {
        h[0] = 1.0;
        r[0] = 0.5;
        t[0] = 1.5;
    }
    if dim >= 2 {
        h[1] = 0.0;
        r[1] = 1.0;
        t[1] = 1.0;
    }
    (h, r, t)
}

/// 1-feature regression design from x / y halves (OLS-style), or a demo line.
fn regression_xy_rows(nums: &[f64]) -> (Vec<Vec<f64>>, Vec<f64>) {
    if nums.len() >= 4 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        let xs = &nums[..half];
        let ys = nums[half..].to_vec();
        let rows: Vec<Vec<f64>> = xs.iter().copied().map(|x| vec![x]).collect();
        (rows, ys)
    } else {
        (
            vec![vec![1.0], vec![2.0], vec![3.0], vec![4.0]],
            vec![3.0, 5.0, 7.0, 9.0],
        )
    }
}

/// Row-major matrix → list-of-rows using `data-p` (default 2).
fn matrix_rows_from_flat(nums: &[f64], p: usize) -> Vec<Vec<f64>> {
    let p = p.max(1);
    if nums.len() >= p * 2 {
        let n = nums.len() / p;
        (0..n)
            .map(|i| nums[i * p..(i + 1) * p].to_vec())
            .collect()
    } else {
        vec![
            vec![0.0, 0.0],
            vec![0.1, 0.2],
            vec![5.0, 5.0],
            vec![5.1, 4.9],
        ]
    }
}

/// `MachineLearning.transe_score` — TransE triple score from h / r / t thirds.
pub(super) fn run_transe_score(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let dim = numeric_attr(container.as_ref(), "data-dim")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or_else(|| {
            if nums.len() >= 6 {
                nums.len() / 3
            } else {
                2
            }
        });
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(2);
    let (h, r, t) = embedding_hrt(&nums, dim);
    invoke_dual(
        document,
        label,
        "MachineLearning.transe_score",
        format!("TransE sketch: dim={} p={p} (connect for live score)", h.len()),
        json!({ "h": h, "r": r, "t": t, "p": p }),
    );
}

/// `MachineLearning.distmult_score` — DistMult triple score.
pub(super) fn run_distmult_score(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let dim = numeric_attr(container.as_ref(), "data-dim")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or_else(|| {
            if nums.len() >= 6 {
                nums.len() / 3
            } else {
                2
            }
        });
    let (h, r, t) = embedding_hrt(&nums, dim);
    invoke_dual(
        document,
        label,
        "MachineLearning.distmult_score",
        format!("DistMult sketch: dim={} (connect for live score)", h.len()),
        json!({ "h": h, "r": r, "t": t }),
    );
}

/// `MachineLearning.complex_score` — ComplEx score (real/imag length 2k).
pub(super) fn run_complex_score(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let k = numeric_attr(container.as_ref(), "data-k")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(2);
    let dim = (k as usize) * 2;
    let (h, r, t) = embedding_hrt(&nums, dim);
    invoke_dual(
        document,
        label,
        "MachineLearning.complex_score",
        format!("ComplEx sketch: k={k} vec_len={} (connect for live)", h.len()),
        json!({ "h": h, "r": r, "t": t, "k": k }),
    );
}

/// `MachineLearning.rotate_score` — RotatE score.
pub(super) fn run_rotate_score(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let k = numeric_attr(container.as_ref(), "data-k")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(2);
    let dim = (k as usize) * 2;
    let (h, r, t) = embedding_hrt(&nums, dim);
    invoke_dual(
        document,
        label,
        "MachineLearning.rotate_score",
        format!("RotatE sketch: k={k} vec_len={} (connect for live)", h.len()),
        json!({ "h": h, "r": r, "t": t, "k": k }),
    );
}

/// `MachineLearning.ridge_fit` — ridge regression from x / y halves.
pub(super) fn run_ridge_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let lambda = numeric_attr(container.as_ref(), "data-lambda").unwrap_or(1.0);
    let (x, y) = regression_xy_rows(&nums);
    let n = x.len().min(y.len());
    invoke_dual(
        document,
        label,
        "MachineLearning.ridge_fit",
        format!("Ridge sketch: n={n} λ={lambda} (connect for live fit)"),
        json!({ "x": x, "y": y, "lambda": lambda }),
    );
}

/// `MachineLearning.lasso_fit` — lasso regression from x / y halves.
pub(super) fn run_lasso_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let lambda = numeric_attr(container.as_ref(), "data-lambda").unwrap_or(1.0);
    let max_iter = numeric_attr(container.as_ref(), "data-max-iter")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(100);
    let (x, y) = regression_xy_rows(&nums);
    let n = x.len().min(y.len());
    invoke_dual(
        document,
        label,
        "MachineLearning.lasso_fit",
        format!("Lasso sketch: n={n} λ={lambda} max_iter={max_iter}"),
        json!({
            "x": x,
            "y": y,
            "lambda": lambda,
            "max_iter": max_iter,
            "tol": 1e-6,
        }),
    );
}

/// `MachineLearning.pls_fit` — PLS1 regression from x / y halves.
pub(super) fn run_pls_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let n_components = numeric_attr(container.as_ref(), "data-components")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(1);
    let (x, y) = regression_xy_rows(&nums);
    let n = x.len().min(y.len());
    invoke_dual(
        document,
        label,
        "MachineLearning.pls_fit",
        format!("PLS sketch: n={n} components={n_components} (connect for live)"),
        json!({
            "x": x,
            "y": y,
            "n_components": n_components,
        }),
    );
}

/// `MachineLearning.standard_scaler_fit_transform` — z-score rows from surface.
pub(super) fn run_standard_scaler_fit_transform(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or(2);
    let x = matrix_rows_from_flat(&nums, p);
    let n = x.len();
    let p = x.first().map(|r| r.len()).unwrap_or(p);
    invoke_dual(
        document,
        label,
        "MachineLearning.standard_scaler_fit_transform",
        format!("StandardScaler sketch: n={n} p={p} (connect for live z-scores)"),
        json!({ "x": x }),
    );
}

/// 1-feature design with integer class labels from x / y halves, or a demo set.
fn classified_xy_rows(nums: &[f64]) -> (Vec<Vec<f64>>, Vec<u64>) {
    if nums.len() >= 4 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        let xs = &nums[..half];
        let ys = &nums[half..];
        let rows: Vec<Vec<f64>> = xs.iter().copied().map(|x| vec![x]).collect();
        let labels: Vec<u64> = ys.iter().map(|v| v.round().max(0.0) as u64).collect();
        (rows, labels)
    } else {
        (
            vec![vec![0.0], vec![0.2], vec![5.0], vec![5.2]],
            vec![0, 0, 1, 1],
        )
    }
}

/// Binary 0/1 response for logistic from x / y halves, or a demo line.
fn logistic_xy_rows(nums: &[f64]) -> (Vec<Vec<f64>>, Vec<f64>) {
    if nums.len() >= 4 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        let xs = &nums[..half];
        let ys = &nums[half..];
        let rows: Vec<Vec<f64>> = xs.iter().copied().map(|x| vec![x]).collect();
        let labels: Vec<f64> = ys.iter().map(|v| if *v > 0.5 { 1.0 } else { 0.0 }).collect();
        (rows, labels)
    } else {
        (
            vec![vec![1.0], vec![2.0], vec![3.0], vec![4.0]],
            vec![0.0, 0.0, 1.0, 1.0],
        )
    }
}

/// Non-negative count response for Poisson from x / y halves, or a demo.
fn poisson_xy_rows(nums: &[f64]) -> (Vec<Vec<f64>>, Vec<f64>) {
    if nums.len() >= 4 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        let xs = &nums[..half];
        let ys = &nums[half..];
        let rows: Vec<Vec<f64>> = xs.iter().copied().map(|x| vec![x]).collect();
        let counts: Vec<f64> = ys.iter().map(|v| v.round().max(0.0)).collect();
        (rows, counts)
    } else {
        (
            vec![vec![1.0], vec![2.0], vec![3.0], vec![4.0]],
            vec![1.0, 2.0, 4.0, 8.0],
        )
    }
}

/// `MachineLearning.kmeans_fit` — k-means++ fit on row-major points.
pub(super) fn run_kmeans_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or(2);
    let x = matrix_rows_from_flat(&nums, p);
    let n = x.len();
    let p = x.first().map(|r| r.len()).unwrap_or(p);
    let k = numeric_attr(container.as_ref(), "data-k")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(2)
        .min(n as u64)
        .max(1);
    let max_iter = numeric_attr(container.as_ref(), "data-max-iter")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(100);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    invoke_dual(
        document,
        label,
        "MachineLearning.kmeans_fit",
        format!("k-means fit sketch: n={n} p={p} k={k} (connect for live)"),
        json!({
            "x": x,
            "k": k,
            "max_iter": max_iter,
            "seed": seed,
        }),
    );
}

/// `MachineLearning.gmm_fit` — diagonal GMM EM on row-major points.
pub(super) fn run_gmm_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or(2);
    let x = matrix_rows_from_flat(&nums, p);
    let n = x.len();
    let p = x.first().map(|r| r.len()).unwrap_or(p);
    let k = numeric_attr(container.as_ref(), "data-k")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(2)
        .min(n as u64)
        .max(1);
    let max_iter = numeric_attr(container.as_ref(), "data-max-iter")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(100);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    invoke_dual(
        document,
        label,
        "MachineLearning.gmm_fit",
        format!("GMM fit sketch: n={n} p={p} k={k} (connect for live)"),
        json!({
            "x": x,
            "k": k,
            "max_iter": max_iter,
            "tol": 1e-6,
            "seed": seed,
        }),
    );
}

/// `MachineLearning.logistic_fit` — Bernoulli GLM from x / y halves.
pub(super) fn run_logistic_fit(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = logistic_xy_rows(&nums);
    let n = x.len().min(y.len());
    invoke_dual(
        document,
        label,
        "MachineLearning.logistic_fit",
        format!("Logistic sketch: n={n} (connect for live fit)"),
        json!({ "x": x, "y": y, "intercept": true }),
    );
}

/// `MachineLearning.poisson_fit` — Poisson GLM from x / count halves.
pub(super) fn run_poisson_fit(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = poisson_xy_rows(&nums);
    let n = x.len().min(y.len());
    invoke_dual(
        document,
        label,
        "MachineLearning.poisson_fit",
        format!("Poisson sketch: n={n} (connect for live fit)"),
        json!({ "x": x, "y": y, "intercept": true }),
    );
}

/// `MachineLearning.naive_bayes_fit` — Gaussian NB from x / label halves.
pub(super) fn run_naive_bayes_fit(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = classified_xy_rows(&nums);
    let n = x.len().min(y.len());
    invoke_dual(
        document,
        label,
        "MachineLearning.naive_bayes_fit",
        format!("Naive Bayes sketch: n={n} (connect for live fit)"),
        json!({ "x": x, "y": y }),
    );
}

/// `MachineLearning.knn_fit` — k-NN classifier from x / label halves.
pub(super) fn run_knn_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = classified_xy_rows(&nums);
    let n = x.len().min(y.len());
    let k = numeric_attr(container.as_ref(), "data-k")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(3)
        .min(n as u64)
        .max(1);
    invoke_dual(
        document,
        label,
        "MachineLearning.knn_fit",
        format!("k-NN sketch: n={n} k={k} (connect for live fit)"),
        json!({ "x": x, "y": y, "k": k }),
    );
}

/// `MachineLearning.lda_fit` — linear discriminant from x / label halves.
pub(super) fn run_lda_fit(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = classified_xy_rows(&nums);
    let n = x.len().min(y.len());
    invoke_dual(
        document,
        label,
        "MachineLearning.lda_fit",
        format!("LDA sketch: n={n} (connect for live fit)"),
        json!({ "x": x, "y": y }),
    );
}

/// `MachineLearning.pcr_fit` — principal component regression from x / y halves.
pub(super) fn run_pcr_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let n_components = numeric_attr(container.as_ref(), "data-components")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(1);
    let (x, y) = regression_xy_rows(&nums);
    let n = x.len().min(y.len());
    invoke_dual(
        document,
        label,
        "MachineLearning.pcr_fit",
        format!("PCR sketch: n={n} components={n_components} (connect for live)"),
        json!({
            "x": x,
            "y": y,
            "n_components": n_components,
        }),
    );
}

/// Binary bool labels from x / y halves for SVM, or a demo set.
fn svm_xy_rows(nums: &[f64]) -> (Vec<Vec<f64>>, Vec<bool>) {
    if nums.len() >= 4 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        let xs = &nums[..half];
        let ys = &nums[half..];
        let rows: Vec<Vec<f64>> = xs.iter().copied().map(|x| vec![x]).collect();
        let labels: Vec<bool> = ys.iter().map(|v| *v > 0.5).collect();
        (rows, labels)
    } else {
        (
            vec![vec![1.0], vec![0.0], vec![1.0], vec![2.0]],
            vec![true, false, true, false],
        )
    }
}

/// Survival times + event flags from paired halves, or a demo series.
fn survival_times_events(nums: &[f64]) -> (Vec<f64>, Vec<bool>) {
    if nums.len() >= 4 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        let times = nums[..half].to_vec();
        let event: Vec<bool> = nums[half..].iter().map(|v| *v > 0.5).collect();
        (times, event)
    } else {
        (
            vec![5.0, 3.0, 8.0, 2.0, 10.0],
            vec![true, true, false, true, true],
        )
    }
}

/// `MachineLearning.qda_fit` — quadratic discriminant from x / label halves.
pub(super) fn run_qda_fit(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = classified_xy_rows(&nums);
    let n = x.len().min(y.len());
    invoke_dual(
        document,
        label,
        "MachineLearning.qda_fit",
        format!("QDA sketch: n={n} (connect for live fit)"),
        json!({ "x": x, "y": y }),
    );
}

/// `MachineLearning.multinomial_logistic_fit` — softmax from x / label halves.
pub(super) fn run_multinomial_logistic_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = classified_xy_rows(&nums);
    let n = x.len().min(y.len());
    let lr = numeric_attr(container.as_ref(), "data-lr").unwrap_or(0.1);
    let l2 = numeric_attr(container.as_ref(), "data-l2").unwrap_or(0.01);
    let max_iter = numeric_attr(container.as_ref(), "data-max-iter")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(200);
    invoke_dual(
        document,
        label,
        "MachineLearning.multinomial_logistic_fit",
        format!("Multinomial logistic sketch: n={n} lr={lr} (connect for live)"),
        json!({
            "x": x,
            "y": y,
            "intercept": true,
            "lr": lr,
            "l2": l2,
            "max_iter": max_iter,
        }),
    );
}

/// `MachineLearning.hierarchical_fit` — agglomerative clustering on row-major points.
pub(super) fn run_hierarchical_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or(2);
    let x = matrix_rows_from_flat(&nums, p);
    let n = x.len();
    let linkage = container
        .as_ref()
        .and_then(|e| e.get_attribute("data-linkage"))
        .unwrap_or_else(|| "complete".to_string());
    invoke_dual(
        document,
        label,
        "MachineLearning.hierarchical_fit",
        format!("Hierarchical fit sketch: n={n} linkage={linkage}"),
        json!({ "x": x, "linkage": linkage }),
    );
}

/// `MachineLearning.hierarchical_labels` — cut dendrogram into k clusters.
pub(super) fn run_hierarchical_labels(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or(2);
    let x = matrix_rows_from_flat(&nums, p);
    let n = x.len();
    let k = numeric_attr(container.as_ref(), "data-k")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(2)
        .min(n as u64)
        .max(1);
    let linkage = container
        .as_ref()
        .and_then(|e| e.get_attribute("data-linkage"))
        .unwrap_or_else(|| "complete".to_string());
    invoke_dual(
        document,
        label,
        "MachineLearning.hierarchical_labels",
        format!("Hierarchical labels sketch: n={n} k={k} linkage={linkage}"),
        json!({ "x": x, "linkage": linkage, "k": k }),
    );
}

/// `MachineLearning.bayesian_linear_fit` — Bayesian linear from x / y halves.
pub(super) fn run_bayesian_linear_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = regression_xy_rows(&nums);
    let n = x.len().min(y.len());
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(1.0);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(100.0);
    invoke_dual(
        document,
        label,
        "MachineLearning.bayesian_linear_fit",
        format!("Bayesian linear sketch: n={n} α={alpha} β={beta}"),
        json!({
            "x": x,
            "y": y,
            "alpha": alpha,
            "beta": beta,
            "intercept": true,
        }),
    );
}

/// `MachineLearning.decision_tree_fit_regressor` — regression tree from x / y halves.
pub(super) fn run_decision_tree_fit_regressor(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = regression_xy_rows(&nums);
    let n = x.len().min(y.len());
    let max_depth = numeric_attr(container.as_ref(), "data-max-depth")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(3);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    invoke_dual(
        document,
        label,
        "MachineLearning.decision_tree_fit_regressor",
        format!("Decision-tree regressor sketch: n={n} max_depth={max_depth}"),
        json!({
            "x": x,
            "y": y,
            "max_depth": max_depth,
            "min_samples_split": 2,
            "min_samples_leaf": 1,
            "seed": seed,
        }),
    );
}

/// `MachineLearning.decision_tree_fit_classifier` — classification tree from x / labels.
pub(super) fn run_decision_tree_fit_classifier(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = classified_xy_rows(&nums);
    let n = x.len().min(y.len());
    let max_depth = numeric_attr(container.as_ref(), "data-max-depth")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(3);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    invoke_dual(
        document,
        label,
        "MachineLearning.decision_tree_fit_classifier",
        format!("Decision-tree classifier sketch: n={n} max_depth={max_depth}"),
        json!({
            "x": x,
            "y": y,
            "max_depth": max_depth,
            "min_samples_split": 2,
            "min_samples_leaf": 1,
            "seed": seed,
        }),
    );
}

/// `MachineLearning.gp_fit` — Gaussian process regression from x / y halves.
pub(super) fn run_gp_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = regression_xy_rows(&nums);
    let n = x.len().min(y.len());
    let length_scale = numeric_attr(container.as_ref(), "data-length-scale").unwrap_or(1.0);
    let signal_var = numeric_attr(container.as_ref(), "data-signal-var").unwrap_or(1.0);
    let noise_var = numeric_attr(container.as_ref(), "data-noise-var").unwrap_or(1e-6);
    invoke_dual(
        document,
        label,
        "MachineLearning.gp_fit",
        format!("GP sketch: n={n} length_scale={length_scale} (connect for live)"),
        json!({
            "x": x,
            "y": y,
            "length_scale": length_scale,
            "signal_var": signal_var,
            "noise_var": noise_var,
        }),
    );
}

/// `MachineLearning.svm_fit` — soft-margin SVM from x / bool-label halves.
pub(super) fn run_svm_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = svm_xy_rows(&nums);
    let n = x.len().min(y.len());
    let c = numeric_attr(container.as_ref(), "data-c").unwrap_or(1.0);
    let max_passes = numeric_attr(container.as_ref(), "data-max-passes")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(5);
    invoke_dual(
        document,
        label,
        "MachineLearning.svm_fit",
        format!("SVM sketch: n={n} C={c} kernel=linear (connect for live)"),
        json!({
            "x": x,
            "y": y,
            "c": c,
            "kernel": "linear",
            "max_passes": max_passes,
            "tol": 0.001,
        }),
    );
}

/// `MachineLearning.kaplan_meier_fit` — KM curve from time / event halves.
pub(super) fn run_kaplan_meier_fit(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (times, event) = survival_times_events(&nums);
    let n = times.len().min(event.len());
    let events = event.iter().filter(|&&e| e).count();
    invoke_dual(
        document,
        label,
        "MachineLearning.kaplan_meier_fit",
        format!("Kaplan–Meier sketch: n={n} events={events} (connect for live)"),
        json!({ "times": times, "event": event }),
    );
}

/// 1-feature Cox design: thirds → x / times / event, or a demo cohort.
fn cox_xy_times_events(nums: &[f64]) -> (Vec<Vec<f64>>, Vec<f64>, Vec<bool>) {
    if nums.len() >= 9 && nums.len() % 3 == 0 {
        let third = nums.len() / 3;
        let xs = &nums[..third];
        let times = nums[third..third * 2].to_vec();
        let event: Vec<bool> = nums[third * 2..]
            .iter()
            .map(|v| *v > 0.5)
            .collect();
        let rows: Vec<Vec<f64>> = xs.iter().copied().map(|x| vec![x]).collect();
        (rows, times, event)
    } else {
        (
            vec![vec![0.0], vec![1.0], vec![0.0], vec![1.0], vec![0.5]],
            vec![5.0, 3.0, 8.0, 2.0, 10.0],
            vec![true, true, false, true, true],
        )
    }
}

/// `MachineLearning.cox_fit` — Cox PH from x / times / event thirds.
pub(super) fn run_cox_fit(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, times, event) = cox_xy_times_events(&nums);
    let n = x.len().min(times.len()).min(event.len());
    let events = event.iter().filter(|&&e| e).count();
    invoke_dual(
        document,
        label,
        "MachineLearning.cox_fit",
        format!("Cox PH sketch: n={n} events={events} (connect for live)"),
        json!({ "x": x, "times": times, "event": event }),
    );
}

/// `MachineLearning.hmm_baum_welch` — Baum–Welch on discrete obs symbols.
pub(super) fn run_hmm_baum_welch(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let obs: Vec<u64> = if nums.len() >= 4 {
        nums.iter()
            .map(|v| v.round().max(0.0) as u64)
            .take(64)
            .collect()
    } else {
        vec![0, 1, 0, 1, 0, 1, 0, 1]
    };
    let m_guess = obs.iter().copied().max().unwrap_or(0) + 1;
    let k = numeric_attr(container.as_ref(), "data-k")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(2);
    let m = numeric_attr(container.as_ref(), "data-m")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(m_guess.max(2));
    let max_iter = numeric_attr(container.as_ref(), "data-max-iter")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(50);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    let n = obs.len();
    invoke_dual(
        document,
        label,
        "MachineLearning.hmm_baum_welch",
        format!("HMM Baum–Welch sketch: n={n} k={k} m={m} (connect for live)"),
        json!({
            "obs": obs,
            "k": k,
            "m": m,
            "max_iter": max_iter,
            "tol": 1e-6,
            "seed": seed,
        }),
    );
}

/// `MachineLearning.variational_gaussian_fit` — mean-field VI on a univariate sample.
pub(super) fn run_variational_gaussian_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let data = if nums.len() >= 2 {
        nums
    } else {
        vec![0.0, 0.5, 1.0, 1.5, 2.0, 1.2, 0.8]
    };
    let n = data.len();
    let mu0 = numeric_attr(container.as_ref(), "data-mu0").unwrap_or(0.0);
    let lambda0 = numeric_attr(container.as_ref(), "data-lambda0").unwrap_or(1.0);
    let a0 = numeric_attr(container.as_ref(), "data-a0").unwrap_or(1.0);
    let b0 = numeric_attr(container.as_ref(), "data-b0").unwrap_or(1.0);
    let max_iter = numeric_attr(container.as_ref(), "data-max-iter")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(100);
    invoke_dual(
        document,
        label,
        "MachineLearning.variational_gaussian_fit",
        format!("Variational Gaussian sketch: n={n} (connect for live)"),
        json!({
            "data": data,
            "mu0": mu0,
            "lambda0": lambda0,
            "a0": a0,
            "b0": b0,
            "max_iter": max_iter,
            "tol": 1e-6,
        }),
    );
}

/// `MachineLearning.mcmc_metropolis` — random-walk MH (standard normal target).
pub(super) fn run_mcmc_metropolis(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let initial = if !nums.is_empty() {
        nums.into_iter().take(4).collect::<Vec<_>>()
    } else {
        vec![0.0]
    };
    let proposal_std = numeric_attr(container.as_ref(), "data-proposal-std").unwrap_or(1.0);
    let n_samples = numeric_attr(container.as_ref(), "data-n-samples")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(100);
    let burn_in = numeric_attr(container.as_ref(), "data-burn-in")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(10);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    let dim = initial.len();
    invoke_dual(
        document,
        label,
        "MachineLearning.mcmc_metropolis",
        format!("MCMC Metropolis sketch: dim={dim} n={n_samples} (connect for live)"),
        json!({
            "target": "standard_normal",
            "initial": initial,
            "proposal_std": proposal_std,
            "n_samples": n_samples,
            "burn_in": burn_in,
            "seed": seed,
        }),
    );
}

/// `MachineLearning.svm_multiclass_fit` — one-vs-rest multiclass SVM.
pub(super) fn run_svm_multiclass_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = classified_xy_rows(&nums);
    let n = x.len().min(y.len());
    let c = numeric_attr(container.as_ref(), "data-c").unwrap_or(1.0);
    let max_passes = numeric_attr(container.as_ref(), "data-max-passes")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(5);
    invoke_dual(
        document,
        label,
        "MachineLearning.svm_multiclass_fit",
        format!("Multiclass SVM sketch: n={n} C={c} (connect for live)"),
        json!({
            "x": x,
            "y": y,
            "c": c,
            "kernel": "linear",
            "max_passes": max_passes,
            "tol": 0.001,
        }),
    );
}

/// `MachineLearning.som_train` — self-organizing map on row-major points.
pub(super) fn run_som_train(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or(2);
    let data = matrix_rows_from_flat(&nums, p);
    let n = data.len();
    let dim = data.first().map(|r| r.len()).unwrap_or(p);
    let grid_w = numeric_attr(container.as_ref(), "data-grid-w")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(3);
    let grid_h = numeric_attr(container.as_ref(), "data-grid-h")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(3);
    let epochs = numeric_attr(container.as_ref(), "data-epochs")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(20);
    let lr0 = numeric_attr(container.as_ref(), "data-lr0").unwrap_or(0.1);
    let sigma0 = numeric_attr(container.as_ref(), "data-sigma0").unwrap_or(1.0);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    invoke_dual(
        document,
        label,
        "MachineLearning.som_train",
        format!("SOM sketch: n={n} dim={dim} grid={grid_w}×{grid_h} (connect for live)"),
        json!({
            "data": data,
            "grid_w": grid_w,
            "grid_h": grid_h,
            "epochs": epochs,
            "lr0": lr0,
            "sigma0": sigma0,
            "seed": seed,
        }),
    );
}

/// `MachineLearning.random_forest_fit_regressor` — RF regressor from x / y halves.
pub(super) fn run_random_forest_fit_regressor(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = regression_xy_rows(&nums);
    let n = x.len().min(y.len());
    let n_trees = numeric_attr(container.as_ref(), "data-n-trees")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(5);
    let max_depth = numeric_attr(container.as_ref(), "data-max-depth")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(3);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    invoke_dual(
        document,
        label,
        "MachineLearning.random_forest_fit_regressor",
        format!("RF regressor sketch: n={n} trees={n_trees} (connect for live)"),
        json!({
            "x": x,
            "y": y,
            "n_trees": n_trees,
            "max_depth": max_depth,
            "min_samples_split": 2,
            "min_samples_leaf": 1,
            "seed": seed,
        }),
    );
}

/// `MachineLearning.random_forest_fit_classifier` — RF classifier from x / labels.
pub(super) fn run_random_forest_fit_classifier(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = classified_xy_rows(&nums);
    let n = x.len().min(y.len());
    let n_trees = numeric_attr(container.as_ref(), "data-n-trees")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(5);
    let max_depth = numeric_attr(container.as_ref(), "data-max-depth")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(3);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    invoke_dual(
        document,
        label,
        "MachineLearning.random_forest_fit_classifier",
        format!("RF classifier sketch: n={n} trees={n_trees} (connect for live)"),
        json!({
            "x": x,
            "y": y,
            "n_trees": n_trees,
            "max_depth": max_depth,
            "min_samples_split": 2,
            "min_samples_leaf": 1,
            "seed": seed,
        }),
    );
}

/// `MachineLearning.gradient_boosting_fit_regressor` — stage-wise tree boosting.
pub(super) fn run_gradient_boosting_fit_regressor(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = regression_xy_rows(&nums);
    let n = x.len().min(y.len());
    let n_estimators = numeric_attr(container.as_ref(), "data-n-estimators")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(10);
    let learning_rate = numeric_attr(container.as_ref(), "data-learning-rate").unwrap_or(0.1);
    let max_depth = numeric_attr(container.as_ref(), "data-max-depth")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(2);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    invoke_dual(
        document,
        label,
        "MachineLearning.gradient_boosting_fit_regressor",
        format!("GBM sketch: n={n} estimators={n_estimators} (connect for live)"),
        json!({
            "x": x,
            "y": y,
            "n_estimators": n_estimators,
            "learning_rate": learning_rate,
            "max_depth": max_depth,
            "min_samples_split": 2,
            "min_samples_leaf": 1,
            "seed": seed,
        }),
    );
}

/// `MachineLearning.bart_fit` — Bayesian additive regression trees.
pub(super) fn run_bart_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = regression_xy_rows(&nums);
    let n = x.len().min(y.len());
    let m = numeric_attr(container.as_ref(), "data-m")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(10);
    let n_iter = numeric_attr(container.as_ref(), "data-n-iter")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(20);
    let burn_in = numeric_attr(container.as_ref(), "data-burn-in")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(5);
    let k = numeric_attr(container.as_ref(), "data-k").unwrap_or(2.0);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(42);
    invoke_dual(
        document,
        label,
        "MachineLearning.bart_fit",
        format!("BART sketch: n={n} m={m} n_iter={n_iter} (connect for live)"),
        json!({
            "x": x,
            "y": y,
            "m": m,
            "n_iter": n_iter,
            "burn_in": burn_in,
            "k": k,
            "seed": seed,
        }),
    );
}

/// Compact TransE demo table: entity0 + rel0 ≈ entity1 (rank-2, 3 entities).
fn demo_kg_embedding_table() -> serde_json::Value {
    json!({
        "model": "TransE",
        "p": 2,
        "rank": 2,
        "n_entities": 3,
        "n_relations": 1,
        "entities": [0.0, 0.0, 1.0, 0.0, 5.0, 5.0],
        "relations": [1.0, 0.0],
    })
}

fn demo_kg_triples_and_candidates() -> (Vec<Vec<f64>>, Vec<u64>) {
    (vec![vec![0.0, 0.0, 1.0]], vec![0, 1, 2])
}

/// `MachineLearning.kg_mean_rank` — mean rank of test triples under an embedding table.
pub(super) fn run_kg_mean_rank(document: &Document, label: &str) {
    let (triples, candidates) = demo_kg_triples_and_candidates();
    let table = demo_kg_embedding_table();
    invoke_dual(
        document,
        label,
        "MachineLearning.kg_mean_rank",
        format!(
            "KG mean-rank sketch: {} triple(s), {} candidates (TransE demo)",
            triples.len(),
            candidates.len()
        ),
        json!({
            "table": table,
            "triples": triples,
            "candidates": candidates,
        }),
    );
}

/// `MachineLearning.kg_mean_reciprocal_rank` — MRR of test triples.
pub(super) fn run_kg_mean_reciprocal_rank(document: &Document, label: &str) {
    let (triples, candidates) = demo_kg_triples_and_candidates();
    let table = demo_kg_embedding_table();
    invoke_dual(
        document,
        label,
        "MachineLearning.kg_mean_reciprocal_rank",
        format!(
            "KG MRR sketch: {} triple(s), {} candidates (TransE demo)",
            triples.len(),
            candidates.len()
        ),
        json!({
            "table": table,
            "triples": triples,
            "candidates": candidates,
        }),
    );
}

/// `MachineLearning.kg_hits_at_k` — Hits@k of test triples.
pub(super) fn run_kg_hits_at_k(document: &Document, label: &str) {
    let container = selected_container(document);
    let (triples, candidates) = demo_kg_triples_and_candidates();
    let table = demo_kg_embedding_table();
    let k = numeric_attr(container.as_ref(), "data-k")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(1);
    invoke_dual(
        document,
        label,
        "MachineLearning.kg_hits_at_k",
        format!(
            "KG Hits@{k} sketch: {} triple(s), {} candidates (TransE demo)",
            triples.len(),
            candidates.len()
        ),
        json!({
            "table": table,
            "triples": triples,
            "candidates": candidates,
            "k": k,
        }),
    );
}

/// `MachineLearning.kalman_new` — 1-D Kalman predict/update from surface or demo.
pub(super) fn run_kalman_new(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let z = if !nums.is_empty() {
        nums.into_iter().take(1).collect::<Vec<_>>()
    } else {
        vec![5.0]
    };
    let f = vec![1.0];
    let h = vec![1.0];
    let q = vec![1e-4];
    let r = vec![1.0];
    let x0 = vec![0.0];
    let p0 = vec![10.0];
    let nx = 1u64;
    let nz = 1u64;
    invoke_dual(
        document,
        label,
        "MachineLearning.kalman_new",
        format!("Kalman sketch: nx={nx} nz={nz} z≈{:.3} (connect for live)", z[0]),
        json!({
            "f": f,
            "h": h,
            "q": q,
            "r": r,
            "x0": x0,
            "p0": p0,
            "nx": nx,
            "nz": nz,
            "z": z,
        }),
    );
}

/// `MachineLearning.factor_graph_marginals` — sum-product BP on a tiny factor graph.
pub(super) fn run_factor_graph_marginals(document: &Document, label: &str) {
    let container = selected_container(document);
    let max_iter = numeric_attr(container.as_ref(), "data-max-iter")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(50);
    let tol = numeric_attr(container.as_ref(), "data-tol").unwrap_or(1e-6);
    // Compact X0—X1—X2 chain demo (matches Host factor-graph tests).
    let cardinalities = vec![2u64, 2, 2];
    let factors = vec![
        json!({ "vars": [0], "table": [0.7, 0.3] }),
        json!({ "vars": [0, 1], "table": [0.8, 0.2, 0.3, 0.7] }),
        json!({ "vars": [1, 2], "table": [0.6, 0.4, 0.1, 0.9] }),
    ];
    invoke_dual(
        document,
        label,
        "MachineLearning.factor_graph_marginals",
        format!(
            "Factor-graph sketch: {} vars, {} factors (connect for live marginals)",
            cardinalities.len(),
            factors.len()
        ),
        json!({
            "cardinalities": cardinalities,
            "factors": factors,
            "max_iter": max_iter,
            "tol": tol,
        }),
    );
}

/// Softmax-style probability row from surface numbers, or a demo simplex.
fn al_prob_row(nums: &[f64]) -> Vec<f64> {
    let raw: Vec<f64> = if nums.len() >= 2 {
        nums.iter().copied().take(8).map(|v| v.abs()).collect()
    } else {
        vec![0.2, 0.5, 0.3]
    };
    let sum: f64 = raw.iter().sum();
    if sum > 0.0 {
        raw.iter().map(|v| v / sum).collect()
    } else {
        vec![1.0 / raw.len() as f64; raw.len()]
    }
}

fn al_strategy(container: Option<&Element>) -> String {
    container
        .and_then(|e| e.get_attribute("data-strategy"))
        .unwrap_or_else(|| "entropy".to_string())
}

/// `MachineLearning.al_row_score` — uncertainty score for one probability row.
pub(super) fn run_al_row_score(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let row = al_prob_row(&nums);
    let strategy = al_strategy(container.as_ref());
    invoke_dual(
        document,
        label,
        "MachineLearning.al_row_score",
        format!(
            "AL row-score sketch: {} classes strategy={strategy} (connect for live)",
            row.len()
        ),
        json!({ "row": row, "strategy": strategy }),
    );
}

/// `MachineLearning.al_score` — uncertainty scores for a pool of probability rows.
pub(super) fn run_al_score(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(2.0) as usize)
        .unwrap_or(3);
    let probs = if nums.len() >= p * 2 {
        matrix_rows_from_flat(&nums, p)
            .into_iter()
            .map(|row| al_prob_row(&row))
            .collect::<Vec<_>>()
    } else {
        vec![
            al_prob_row(&[0.2, 0.5, 0.3]),
            al_prob_row(&[0.8, 0.1, 0.1]),
            al_prob_row(&[0.33, 0.33, 0.34]),
        ]
    };
    let strategy = al_strategy(container.as_ref());
    let n = probs.len();
    invoke_dual(
        document,
        label,
        "MachineLearning.al_score",
        format!("AL score sketch: pool n={n} strategy={strategy} (connect for live)"),
        json!({ "probs": probs, "strategy": strategy }),
    );
}

/// `MachineLearning.al_cosine_similarity` — cosine of two vectors from halves.
pub(super) fn run_al_cosine_similarity(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (a, b) = match split_pair(&nums) {
        Some(pair) if !pair.0.is_empty() && pair.0.len() == pair.1.len() => pair,
        _ => (vec![1.0, 0.0, 0.0], vec![0.8, 0.2, 0.0]),
    };
    let dim = a.len().min(b.len());
    invoke_dual(
        document,
        label,
        "MachineLearning.al_cosine_similarity",
        format!("AL cosine sketch: dim={dim} (connect for live)"),
        json!({ "a": a, "b": b }),
    );
}

fn al_demo_probs() -> Vec<Vec<f64>> {
    vec![
        al_prob_row(&[0.2, 0.5, 0.3]),
        al_prob_row(&[0.8, 0.1, 0.1]),
        al_prob_row(&[0.33, 0.33, 0.34]),
    ]
}

fn al_probs_from_surface(nums: &[f64], p: usize) -> Vec<Vec<f64>> {
    if nums.len() >= p * 2 {
        matrix_rows_from_flat(nums, p)
            .into_iter()
            .map(|row| al_prob_row(&row))
            .collect()
    } else {
        al_demo_probs()
    }
}

fn al_feature_rows(nums: &[f64], p: usize) -> Vec<Vec<f64>> {
    if nums.len() >= p * 2 {
        matrix_rows_from_flat(nums, p)
    } else {
        vec![
            vec![1.0, 0.0],
            vec![0.9, 0.1],
            vec![0.0, 1.0],
            vec![0.1, 0.9],
        ]
    }
}

fn al_uncertainty_and_features(
    nums: &[f64],
    p: usize,
) -> (Vec<f64>, Vec<Vec<f64>>) {
    let features = al_feature_rows(nums, p);
    let n = features.len();
    let uncertainty = if nums.len() >= n {
        nums.iter().copied().take(n).map(|v| v.abs()).collect()
    } else {
        vec![0.9, 0.4, 0.2, 0.55].into_iter().take(n).collect()
    };
    (uncertainty, features)
}

/// `MachineLearning.al_rank_informative` — rank pool by uncertainty (most first).
pub(super) fn run_al_rank_informative(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(2.0) as usize)
        .unwrap_or(3);
    let probs = al_probs_from_surface(&nums, p);
    let strategy = al_strategy(container.as_ref());
    let n = probs.len();
    invoke_dual(
        document,
        label,
        "MachineLearning.al_rank_informative",
        format!("AL rank-informative sketch: pool n={n} strategy={strategy} (connect for live)"),
        json!({ "probs": probs, "strategy": strategy }),
    );
}

/// `MachineLearning.al_most_informative` — index of the most informative sample.
pub(super) fn run_al_most_informative(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(2.0) as usize)
        .unwrap_or(3);
    let probs = al_probs_from_surface(&nums, p);
    let strategy = al_strategy(container.as_ref());
    let n = probs.len();
    invoke_dual(
        document,
        label,
        "MachineLearning.al_most_informative",
        format!("AL most-informative sketch: pool n={n} strategy={strategy} (connect for live)"),
        json!({ "probs": probs, "strategy": strategy }),
    );
}

/// `MachineLearning.al_representativeness` — mean representativeness per point.
pub(super) fn run_al_representativeness(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or(2);
    let features = al_feature_rows(&nums, p);
    let n = features.len();
    let dim = features.first().map(|r| r.len()).unwrap_or(p);
    invoke_dual(
        document,
        label,
        "MachineLearning.al_representativeness",
        format!("AL representativeness sketch: n={n} dim={dim} (connect for live)"),
        json!({ "features": features }),
    );
}

/// `MachineLearning.al_information_density` — uncertainty × representativeness^β.
pub(super) fn run_al_information_density(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or(2);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(1.0);
    let (uncertainty, features) = al_uncertainty_and_features(&nums, p);
    let n = uncertainty.len();
    invoke_dual(
        document,
        label,
        "MachineLearning.al_information_density",
        format!("AL information-density sketch: n={n} β={beta} (connect for live)"),
        json!({
            "uncertainty": uncertainty,
            "features": features,
            "beta": beta,
        }),
    );
}

/// `MachineLearning.al_rank_by_density` — rank pool by information density.
pub(super) fn run_al_rank_by_density(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or(2);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(1.0);
    let (uncertainty, features) = al_uncertainty_and_features(&nums, p);
    let n = uncertainty.len();
    invoke_dual(
        document,
        label,
        "MachineLearning.al_rank_by_density",
        format!("AL rank-by-density sketch: n={n} β={beta} (connect for live)"),
        json!({
            "uncertainty": uncertainty,
            "features": features,
            "beta": beta,
        }),
    );
}

/// `MachineLearning.al_vote_entropy` — vote entropy of a committee.
pub(super) fn run_al_vote_entropy(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let votes: Vec<u64> = if nums.len() >= 2 {
        nums.iter()
            .map(|v| v.round().max(0.0) as u64)
            .take(64)
            .collect()
    } else {
        vec![0, 1, 0, 2, 1]
    };
    let max_vote = votes.iter().copied().max().unwrap_or(0);
    let n_classes = numeric_attr(container.as_ref(), "data-n-classes")
        .map(|v| v.round().max(1.0) as u64)
        .unwrap_or(max_vote + 1)
        .max(max_vote + 1);
    let n = votes.len();
    invoke_dual(
        document,
        label,
        "MachineLearning.al_vote_entropy",
        format!("AL vote-entropy sketch: votes={n} classes={n_classes} (connect for live)"),
        json!({ "votes": votes, "n_classes": n_classes }),
    );
}

/// `MachineLearning.al_consensus` — consensus distribution of a committee.
pub(super) fn run_al_consensus(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(2.0) as usize)
        .unwrap_or(3);
    let members = al_probs_from_surface(&nums, p);
    let n = members.len();
    let classes = members.first().map(|r| r.len()).unwrap_or(p);
    invoke_dual(
        document,
        label,
        "MachineLearning.al_consensus",
        format!("AL consensus sketch: members={n} classes={classes} (connect for live)"),
        json!({ "members": members }),
    );
}

/// `MachineLearning.al_consensus_entropy` — entropy of the consensus distribution.
pub(super) fn run_al_consensus_entropy(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(2.0) as usize)
        .unwrap_or(3);
    let members = al_probs_from_surface(&nums, p);
    let n = members.len();
    invoke_dual(
        document,
        label,
        "MachineLearning.al_consensus_entropy",
        format!("AL consensus-entropy sketch: members={n} (connect for live)"),
        json!({ "members": members }),
    );
}

/// `MachineLearning.al_average_kl_disagreement` — mean KL(member || consensus).
pub(super) fn run_al_average_kl_disagreement(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let p = numeric_attr(container.as_ref(), "data-p")
        .map(|v| v.round().max(2.0) as usize)
        .unwrap_or(3);
    let members = al_probs_from_surface(&nums, p);
    let n = members.len();
    invoke_dual(
        document,
        label,
        "MachineLearning.al_average_kl_disagreement",
        format!("AL KL-disagreement sketch: members={n} (connect for live)"),
        json!({ "members": members }),
    );
}

/// `MachineLearning.al_rank_by_disagreement` — rank pool by KL disagreement.
pub(super) fn run_al_rank_by_disagreement(document: &Document, label: &str) {
    // Demo: 3 samples × 2 committee members × 3 classes (Host [[[f64]]] shape).
    let pool = vec![
        vec![
            al_prob_row(&[0.7, 0.2, 0.1]),
            al_prob_row(&[0.1, 0.2, 0.7]),
        ],
        vec![
            al_prob_row(&[0.4, 0.4, 0.2]),
            al_prob_row(&[0.35, 0.35, 0.3]),
        ],
        vec![
            al_prob_row(&[0.9, 0.05, 0.05]),
            al_prob_row(&[0.85, 0.1, 0.05]),
        ],
    ];
    let n = pool.len();
    let members = pool.first().map(|m| m.len()).unwrap_or(0);
    invoke_dual(
        document,
        label,
        "MachineLearning.al_rank_by_disagreement",
        format!("AL rank-by-disagreement sketch: samples={n} members={members} (connect for live)"),
        json!({ "pool": pool }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_holm_scales_smallest_first() {
        let adj = local_holm(&[0.01, 0.04, 0.03, 0.20]);
        assert_eq!(adj.len(), 4);
        assert!(adj[0] <= adj[2]);
        assert!(adj.iter().all(|v| *v <= 1.0));
    }

    #[test]
    fn local_bh_fdr_is_defined() {
        let adj = local_benjamini_hochberg(&[0.01, 0.04, 0.03, 0.20]);
        assert_eq!(adj.len(), 4);
        assert!(adj.iter().all(|v| v.is_finite() && *v <= 1.0));
    }

    #[test]
    fn local_n_rejected_counts_at_alpha() {
        assert_eq!(local_n_rejected(&[0.01, 0.04, 0.12], 0.05), 2);
        assert_eq!(local_n_rejected(&[], 0.05), 0);
    }

    #[test]
    fn local_mean_basic() {
        assert_eq!(local_mean(&[1.0, 2.0, 3.0]), Some(2.0));
        assert_eq!(local_mean(&[]), None);
    }
}
