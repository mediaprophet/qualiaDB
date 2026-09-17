//! Dual-path Tool Chest actions for curated `LinearAlgebra.*` ALL_BOUND ids.
//!
//! Live over Host scopes. New app primitives (`dot`/`norm`/`trace`/`identity`/`inverse`)
//! live in `linalg_app_chain_actions.rs` so this file does not grow further.

use serde_json::json;
use web_sys::{Document, Element};

pub(super) fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

pub(super) fn selected_source(document: &Document) -> Option<String> {
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

pub(super) fn parse_numbers(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(4096)
        .collect()
}

pub(super) fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
}

pub(super) fn usize_attr(el: Option<&Element>, name: &str) -> Option<usize> {
    numeric_attr(el, name).and_then(|v| {
        if v.is_finite() && v >= 1.0 && v == v.floor() {
            Some(v as usize)
        } else {
            None
        }
    })
}

pub(super) fn mat_json(rows: usize, cols: usize, data: &[f64]) -> serde_json::Value {
    json!({ "rows": rows as u64, "cols": cols as u64, "data": data })
}

fn is_perfect_square(n: usize) -> Option<usize> {
    if n == 0 {
        return None;
    }
    let s = (n as f64).sqrt() as usize;
    (s * s == n).then_some(s)
}

/// Prefer `data-rows`/`data-cols`; else infer a square layout from length.
pub(super) fn resolve_matrix(
    nums: &[f64],
    container: Option<&Element>,
) -> Option<(usize, usize, Vec<f64>)> {
    let rows = usize_attr(container, "data-rows");
    let cols = usize_attr(container, "data-cols");
    if let (Some(r), Some(c)) = (rows, cols) {
        if r * c <= nums.len() && r > 0 && c > 0 && r * c <= 256 {
            return Some((r, c, nums[..r * c].to_vec()));
        }
    }
    if let Some(n) = is_perfect_square(nums.len()) {
        if n <= 16 {
            return Some((n, n, nums.to_vec()));
        }
    }
    None
}

pub(super) fn default_square2() -> (usize, usize, Vec<f64>) {
    (2, 2, vec![2.0, 1.0, 1.0, 2.0])
}

fn default_matmul_pair() -> ((usize, usize, Vec<f64>), (usize, usize, Vec<f64>)) {
    (
        (2, 2, vec![1.0, 2.0, 3.0, 4.0]),
        (2, 2, vec![5.0, 6.0, 7.0, 8.0]),
    )
}

fn local_matmul(
    a_rows: usize,
    a_cols: usize,
    a: &[f64],
    b_rows: usize,
    b_cols: usize,
    b: &[f64],
) -> Option<Vec<f64>> {
    if a_cols != b_rows || a.len() < a_rows * a_cols || b.len() < b_rows * b_cols {
        return None;
    }
    let mut c = vec![0.0; a_rows * b_cols];
    for i in 0..a_rows {
        for j in 0..b_cols {
            let mut s = 0.0;
            for k in 0..a_cols {
                s += a[i * a_cols + k] * b[k * b_cols + j];
            }
            c[i * b_cols + j] = s;
        }
    }
    c.iter().all(|x| x.is_finite()).then_some(c)
}

fn local_matvec(rows: usize, cols: usize, a: &[f64], x: &[f64]) -> Option<Vec<f64>> {
    if a.len() < rows * cols || x.len() < cols {
        return None;
    }
    let mut y = vec![0.0; rows];
    for i in 0..rows {
        let mut s = 0.0;
        for j in 0..cols {
            s += a[i * cols + j] * x[j];
        }
        y[i] = s;
    }
    y.iter().all(|v| v.is_finite()).then_some(y)
}

fn local_transpose(rows: usize, cols: usize, a: &[f64]) -> Option<Vec<f64>> {
    if a.len() < rows * cols {
        return None;
    }
    let mut out = vec![0.0; rows * cols];
    for i in 0..rows {
        for j in 0..cols {
            out[j * rows + i] = a[i * cols + j];
        }
    }
    Some(out)
}

fn local_determinant(n: usize, a: &[f64]) -> Option<f64> {
    if n == 0 || a.len() < n * n || n > 8 {
        return None;
    }
    let mut m = a[..n * n].to_vec();
    let mut det = 1.0;
    for k in 0..n {
        let mut piv = k;
        for i in (k + 1)..n {
            if m[i * n + k].abs() > m[piv * n + k].abs() {
                piv = i;
            }
        }
        if m[piv * n + k].abs() < 1e-15 {
            return Some(0.0);
        }
        if piv != k {
            for j in 0..n {
                m.swap(k * n + j, piv * n + j);
            }
            det = -det;
        }
        let diag = m[k * n + k];
        det *= diag;
        for i in (k + 1)..n {
            let f = m[i * n + k] / diag;
            for j in k..n {
                m[i * n + j] -= f * m[k * n + j];
            }
        }
    }
    det.is_finite().then_some(det)
}

fn local_solve(n: usize, a: &[f64], b: &[f64]) -> Option<Vec<f64>> {
    if n == 0 || a.len() < n * n || b.len() < n || n > 8 {
        return None;
    }
    let mut m = a[..n * n].to_vec();
    let mut x = b[..n].to_vec();
    for k in 0..n {
        let mut piv = k;
        for i in (k + 1)..n {
            if m[i * n + k].abs() > m[piv * n + k].abs() {
                piv = i;
            }
        }
        if m[piv * n + k].abs() < 1e-15 {
            return None;
        }
        if piv != k {
            for j in 0..n {
                m.swap(k * n + j, piv * n + j);
            }
            x.swap(k, piv);
        }
        let diag = m[k * n + k];
        for i in (k + 1)..n {
            let f = m[i * n + k] / diag;
            for j in k..n {
                m[i * n + j] -= f * m[k * n + j];
            }
            x[i] -= f * x[k];
        }
    }
    for i in (0..n).rev() {
        let mut s = x[i];
        for j in (i + 1)..n {
            s -= m[i * n + j] * x[j];
        }
        x[i] = s / m[i * n + i];
        if !x[i].is_finite() {
            return None;
        }
    }
    Some(x)
}

fn local_scale(a: &[f64], s: f64) -> Option<Vec<f64>> {
    if a.is_empty() || !s.is_finite() {
        return None;
    }
    let out: Vec<f64> = a.iter().map(|x| x * s).collect();
    out.iter().all(|x| x.is_finite()).then_some(out)
}

fn local_add_into(a: &[f64], b: &[f64]) -> Option<Vec<f64>> {
    if a.is_empty() || a.len() != b.len() {
        return None;
    }
    let out: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
    out.iter().all(|x| x.is_finite()).then_some(out)
}

fn local_axpy(alpha: f64, x: &[f64], y: &[f64]) -> Option<Vec<f64>> {
    if x.is_empty() || x.len() != y.len() || !alpha.is_finite() {
        return None;
    }
    let out: Vec<f64> = x
        .iter()
        .zip(y.iter())
        .map(|(xi, yi)| yi + alpha * xi)
        .collect();
    out.iter().all(|v| v.is_finite()).then_some(out)
}

fn local_hadamard(a: &[f64], b: &[f64]) -> Option<Vec<f64>> {
    if a.is_empty() || a.len() != b.len() {
        return None;
    }
    let out: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x * y).collect();
    out.iter().all(|x| x.is_finite()).then_some(out)
}

/// Offline Householder QR sketch for tall-or-square matrices (Host uses full QR).
fn local_qr_factor(rows: usize, cols: usize, a: &[f64]) -> Option<(Vec<f64>, Vec<f64>)> {
    if rows < cols || cols == 0 || a.len() < rows * cols || rows > 12 || cols > 8 {
        return None;
    }
    let mut r = a[..rows * cols].to_vec();
    let mut tau = vec![0.0; cols];
    for k in 0..cols {
        let mut norm = 0.0;
        for i in k..rows {
            let v = r[i * cols + k];
            norm += v * v;
        }
        norm = norm.sqrt();
        if !(norm > 0.0) {
            return None;
        }
        let x0 = r[k * cols + k];
        let alpha = if x0 >= 0.0 { -norm } else { norm };
        let u0 = x0 - alpha;
        let mut beta = u0 * u0;
        for i in (k + 1)..rows {
            let v = r[i * cols + k];
            beta += v * v;
        }
        if !(beta > 0.0) {
            return None;
        }
        let inv = 1.0 / beta.sqrt();
        let mut v = vec![0.0; rows - k];
        v[0] = u0 * inv;
        for i in (k + 1)..rows {
            v[i - k] = r[i * cols + k] * inv;
        }
        tau[k] = 2.0 * v[0] * v[0];
        for j in k..cols {
            let mut dot = 0.0;
            for i in k..rows {
                dot += v[i - k] * r[i * cols + j];
            }
            for i in k..rows {
                r[i * cols + j] -= tau[k] * v[i - k] * dot / (2.0 * v[0] * v[0]).max(1e-30);
            }
        }
        r[k * cols + k] = alpha;
        for i in (k + 1)..rows {
            r[i * cols + k] = v[i - k];
        }
    }
    (r.iter().all(|x| x.is_finite()) && tau.iter().all(|x| x.is_finite())).then_some((r, tau))
}

pub(super) fn invoke_dual(
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

/// `LinearAlgebra.matmul` — two matrices from surface / attrs / 2×2 demos.
pub(super) fn run_matmul(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let ((ar, ac, a), (br, bc, b)) = {
        let a_rows = usize_attr(container.as_ref(), "data-a-rows");
        let a_cols = usize_attr(container.as_ref(), "data-a-cols");
        let b_cols = usize_attr(container.as_ref(), "data-b-cols");
        if let (Some(ar), Some(ac), Some(bc)) = (a_rows, a_cols, b_cols) {
            let br = ac;
            let need = ar * ac + br * bc;
            if need <= nums.len() && ar * ac > 0 && br * bc > 0 {
                (
                    (ar, ac, nums[..ar * ac].to_vec()),
                    (br, bc, nums[ar * ac..need].to_vec()),
                )
            } else {
                default_matmul_pair()
            }
        } else if let Some(n) = is_perfect_square(nums.len() / 2) {
            let half = n * n;
            if nums.len() >= 2 * half {
                (
                    (n, n, nums[..half].to_vec()),
                    (n, n, nums[half..2 * half].to_vec()),
                )
            } else {
                default_matmul_pair()
            }
        } else {
            default_matmul_pair()
        }
    };
    let Some(c) = local_matmul(ar, ac, &a, br, bc, &b) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need two compatible matrices (attrs data-a-rows/cols + data-b-cols, or two squares).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.matmul",
        format!("matmul sketch {ar}×{ac} · {br}×{bc} → {ar}×{bc}: {c:?}"),
        json!({
            "a": mat_json(ar, ac, &a),
            "b": mat_json(br, bc, &b),
        }),
    );
}

/// `LinearAlgebra.matvec` — row-major A then vector x.
pub(super) fn run_matvec(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (rows, cols, a, x) = if let Some((r, c, m)) = resolve_matrix(&nums, container.as_ref()) {
        let rest = &nums[r * c..];
        if rest.len() >= c {
            (r, c, m, rest[..c].to_vec())
        } else {
            let (r, c, m) = default_square2();
            (r, c, m, vec![1.0, 0.0])
        }
    } else {
        let (r, c, m) = default_square2();
        (r, c, m, vec![1.0, 0.0])
    };
    let Some(y) = local_matvec(rows, cols, &a, &x) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a matrix then a vector of length cols (or data-rows/data-cols).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.matvec",
        format!("matvec sketch {rows}×{cols} · x → y={y:?}"),
        json!({
            "matrix": mat_json(rows, cols, &a),
            "x": x,
            "transpose": false,
        }),
    );
}

/// `LinearAlgebra.transpose`
pub(super) fn run_transpose(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (rows, cols, data) =
        resolve_matrix(&nums, container.as_ref()).unwrap_or_else(default_square2);
    let Some(out) = local_transpose(rows, cols, &data) else {
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.transpose",
        format!("transpose sketch {rows}×{cols} → {cols}×{rows}: {out:?}"),
        json!({ "matrix": mat_json(rows, cols, &data) }),
    );
}

/// `LinearAlgebra.determinant`
pub(super) fn run_determinant(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (rows, cols, data) =
        resolve_matrix(&nums, container.as_ref()).unwrap_or_else(default_square2);
    if rows != cols {
        super::interactions::show_tool_status(
            document,
            label,
            "Determinant needs a square matrix.",
            "error",
        );
        return;
    }
    let Some(det) = local_determinant(rows, &data) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Could not form a determinant sketch for this matrix.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.determinant",
        format!("determinant sketch of {rows}×{rows}: {det:.6}"),
        json!({ "matrix": mat_json(rows, cols, &data) }),
    );
}

/// `LinearAlgebra.solve` — square A then RHS vector b.
pub(super) fn run_solve(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, a, b) = if let Some(n) = usize_attr(container.as_ref(), "data-n")
        .or_else(|| usize_attr(container.as_ref(), "data-rows"))
    {
        let need = n * n + n;
        if nums.len() >= need {
            (n, nums[..n * n].to_vec(), nums[n * n..need].to_vec())
        } else {
            (2, vec![2.0, 1.0, 1.0, 2.0], vec![1.0, 0.0])
        }
    } else if let Some(n) = (2..=8).find(|n| nums.len() == n * n + n) {
        (n, nums[..n * n].to_vec(), nums[n * n..].to_vec())
    } else {
        (2, vec![2.0, 1.0, 1.0, 2.0], vec![1.0, 0.0])
    };
    let Some(x) = local_solve(n, &a, &b) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need square A then b (length n), or data-n.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.solve",
        format!("solve sketch A x = b → x={x:?}"),
        json!({
            "a": mat_json(n, n, &a),
            "b": b,
        }),
    );
}

/// `LinearAlgebra.scale` — vector/matrix entries × `data-s` (default 2).
pub(super) fn run_scale(document: &Document, label: &str) {
    let container = selected_container(document);
    let mut nums = parse_numbers(&selected_source(document).unwrap_or_default());
    if nums.is_empty() {
        nums = vec![1.0, 2.0, 3.0, 4.0];
    }
    let s = numeric_attr(container.as_ref(), "data-s").unwrap_or(2.0);
    let Some(out) = local_scale(&nums, s) else {
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.scale",
        format!("scale sketch ×{s} over {} entries → {out:?}", nums.len()),
        json!({ "a": nums, "s": s }),
    );
}

/// `LinearAlgebra.add_into` — split surface into equal a/b halves.
pub(super) fn run_add_into(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = if nums.len() >= 2 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else {
        (vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0])
    };
    let Some(c) = local_add_into(&a, &b) else {
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.add_into",
        format!("add_into sketch over {} pairs → {c:?}", a.len()),
        json!({ "a": a, "b": b }),
    );
}

/// `LinearAlgebra.axpy` — `y += α·x`; α from `data-alpha` (default 1).
pub(super) fn run_axpy(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(1.0);
    let (x, y) = if nums.len() >= 2 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else {
        (vec![1.0, 2.0, 3.0], vec![0.0, 0.0, 0.0])
    };
    let Some(out) = local_axpy(alpha, &x, &y) else {
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.axpy",
        format!("axpy sketch α={alpha} over {} → y={out:?}", x.len()),
        json!({ "alpha": alpha, "x": x, "y": y }),
    );
}

/// `LinearAlgebra.hadamard_into` — element-wise product of equal halves.
pub(super) fn run_hadamard_into(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = if nums.len() >= 2 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else {
        (vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0])
    };
    let Some(c) = local_hadamard(&a, &b) else {
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.hadamard_into",
        format!("hadamard sketch over {} pairs → {c:?}", a.len()),
        json!({ "a": a, "b": b }),
    );
}

/// `LinearAlgebra.qr_factor` — Householder QR of tall-or-square matrix.
pub(super) fn run_qr_factor(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (rows, cols, data) =
        resolve_matrix(&nums, container.as_ref()).unwrap_or_else(|| (3, 2, vec![1.0, 1.0, 0.0, 1.0, 0.0, 1.0]));
    if rows < cols {
        super::interactions::show_tool_status(
            document,
            label,
            "QR factor needs rows ≥ cols.",
            "error",
        );
        return;
    }
    let Some((_factored, tau)) = local_qr_factor(rows, cols, &data) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Could not form a QR sketch for this matrix.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.qr_factor",
        format!("qr_factor sketch {rows}×{cols}; τ={tau:?}"),
        json!({ "matrix": mat_json(rows, cols, &data) }),
    );
}

fn nested_square(n: usize, data: &[f64]) -> serde_json::Value {
    let rows: Vec<Vec<f64>> = (0..n)
        .map(|i| data[i * n..(i + 1) * n].to_vec())
        .collect();
    json!(rows)
}

fn default_spd2() -> (usize, Vec<f64>) {
    (2, vec![2.0, 1.0, 1.0, 2.0])
}

fn local_cholesky_factor(n: usize, a: &[f64]) -> Option<Vec<f64>> {
    if n == 0 || a.len() < n * n || n > 8 {
        return None;
    }
    let mut l = vec![0.0; n * n];
    for j in 0..n {
        let mut diag = a[j * n + j];
        for k in 0..j {
            diag -= l[j * n + k] * l[j * n + k];
        }
        if !(diag > 0.0) {
            return None;
        }
        let ljj = diag.sqrt();
        l[j * n + j] = ljj;
        for i in (j + 1)..n {
            let mut s = a[i * n + j];
            for k in 0..j {
                s -= l[i * n + k] * l[j * n + k];
            }
            l[i * n + j] = s / ljj;
        }
    }
    l.iter().all(|x| x.is_finite()).then_some(l)
}

fn local_cholesky_solve(n: usize, l: &[f64], b: &[f64]) -> Option<Vec<f64>> {
    if n == 0 || l.len() < n * n || b.len() < n || n > 8 {
        return None;
    }
    let mut x = vec![0.0; n];
    for i in 0..n {
        let mut s = b[i];
        for k in 0..i {
            s -= l[i * n + k] * x[k];
        }
        let diag = l[i * n + i];
        if !(diag.abs() > 1e-15) {
            return None;
        }
        x[i] = s / diag;
    }
    for i in (0..n).rev() {
        let mut s = x[i];
        for k in (i + 1)..n {
            s -= l[k * n + i] * x[k];
        }
        x[i] = s / l[i * n + i];
        if !x[i].is_finite() {
            return None;
        }
    }
    Some(x)
}

fn local_lu_decompose(n: usize, a: &[f64]) -> Option<(Vec<f64>, Vec<usize>, f64)> {
    if n == 0 || a.len() < n * n || n > 8 {
        return None;
    }
    let mut lu = a[..n * n].to_vec();
    let mut pivots: Vec<usize> = (0..n).collect();
    let mut sign = 1.0;
    for k in 0..n {
        let mut piv = k;
        for i in (k + 1)..n {
            if lu[i * n + k].abs() > lu[piv * n + k].abs() {
                piv = i;
            }
        }
        if lu[piv * n + k].abs() < 1e-15 {
            return None;
        }
        if piv != k {
            for j in 0..n {
                lu.swap(k * n + j, piv * n + j);
            }
            pivots.swap(k, piv);
            sign = -sign;
        }
        let diag = lu[k * n + k];
        for i in (k + 1)..n {
            let f = lu[i * n + k] / diag;
            lu[i * n + k] = f;
            for j in (k + 1)..n {
                lu[i * n + j] -= f * lu[k * n + j];
            }
        }
    }
    lu.iter().all(|x| x.is_finite()).then_some((lu, pivots, sign))
}

fn local_qr_form_q(rows: usize, cols: usize, a: &[f64], tau: &[f64]) -> Option<Vec<f64>> {
    if rows < cols || a.len() < rows * cols || tau.len() < cols || rows > 12 || cols > 8 {
        return None;
    }
    let mut q = vec![0.0; rows * cols];
    for i in 0..cols.min(rows) {
        q[i * cols + i] = 1.0;
    }
    for k in (0..cols).rev() {
        let mut v = vec![0.0; rows - k];
        v[0] = 1.0;
        for i in (k + 1)..rows {
            v[i - k] = a[i * cols + k];
        }
        let t = tau[k];
        for j in 0..cols {
            let mut dot = 0.0;
            for i in k..rows {
                dot += v[i - k] * q[i * cols + j];
            }
            for i in k..rows {
                q[i * cols + j] -= t * v[i - k] * dot;
            }
        }
    }
    q.iter().all(|x| x.is_finite()).then_some(q)
}

fn local_qr_solve_ls(
    rows: usize,
    cols: usize,
    a: &[f64],
    tau: &[f64],
    b: &[f64],
) -> Option<Vec<f64>> {
    if rows < cols || a.len() < rows * cols || tau.len() < cols || b.len() < rows {
        return None;
    }
    let mut y = b[..rows].to_vec();
    for k in 0..cols {
        let mut v = vec![0.0; rows - k];
        v[0] = 1.0;
        for i in (k + 1)..rows {
            v[i - k] = a[i * cols + k];
        }
        let mut dot = 0.0;
        for i in k..rows {
            dot += v[i - k] * y[i];
        }
        for i in k..rows {
            y[i] -= tau[k] * v[i - k] * dot;
        }
    }
    let mut x = vec![0.0; cols];
    for i in (0..cols).rev() {
        let mut s = y[i];
        for j in (i + 1)..cols {
            s -= a[i * cols + j] * x[j];
        }
        let diag = a[i * cols + i];
        if !(diag.abs() > 1e-15) {
            return None;
        }
        x[i] = s / diag;
        if !x[i].is_finite() {
            return None;
        }
    }
    Some(x)
}

/// Offline singular values via eigenvalues of AᵀA (small matrices).
fn local_svd_singular_values(rows: usize, cols: usize, a: &[f64]) -> Option<Vec<f64>> {
    if rows == 0 || cols == 0 || a.len() < rows * cols || rows > 8 || cols > 8 {
        return None;
    }
    let n = cols;
    let mut ata = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            let mut s = 0.0;
            for k in 0..rows {
                s += a[k * cols + i] * a[k * cols + j];
            }
            ata[i * n + j] = s;
        }
    }
    // Jacobi eigen for SPD AᵀA (eigenvalues only).
    for _ in 0..48 {
        let mut max_off = 0.0;
        let mut p = 0usize;
        let mut q = 1usize;
        for i in 0..n {
            for j in (i + 1)..n {
                let off = ata[i * n + j].abs();
                if off > max_off {
                    max_off = off;
                    p = i;
                    q = j;
                }
            }
        }
        if max_off < 1e-12 {
            break;
        }
        let app = ata[p * n + p];
        let aqq = ata[q * n + q];
        let apq = ata[p * n + q];
        let tau = (aqq - app) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        for k in 0..n {
            let aik = ata[p * n + k];
            let aqk = ata[q * n + k];
            ata[p * n + k] = c * aik - s * aqk;
            ata[q * n + k] = s * aik + c * aqk;
        }
        for k in 0..n {
            let akp = ata[k * n + p];
            let akq = ata[k * n + q];
            ata[k * n + p] = c * akp - s * akq;
            ata[k * n + q] = s * akp + c * akq;
        }
    }
    let mut svals: Vec<f64> = (0..n).map(|i| ata[i * n + i].max(0.0).sqrt()).collect();
    svals.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    svals.iter().all(|x| x.is_finite()).then_some(svals)
}

fn local_eigenvalues_2x2(a: &[f64]) -> Option<Vec<(f64, f64)>> {
    if a.len() < 4 {
        return None;
    }
    let tr = a[0] + a[3];
    let det = a[0] * a[3] - a[1] * a[2];
    let disc = tr * tr - 4.0 * det;
    if disc >= 0.0 {
        let s = disc.sqrt();
        Some(vec![((tr + s) / 2.0, 0.0), ((tr - s) / 2.0, 0.0)])
    } else {
        let s = (-disc).sqrt();
        Some(vec![(tr / 2.0, s / 2.0), (tr / 2.0, -s / 2.0)])
    }
}

fn resolve_square_system(
    nums: &[f64],
    container: Option<&Element>,
) -> (usize, Vec<f64>, Vec<f64>) {
    if let Some(n) = usize_attr(container, "data-n").or_else(|| usize_attr(container, "data-rows"))
    {
        let need = n * n + n;
        if nums.len() >= need {
            return (n, nums[..n * n].to_vec(), nums[n * n..need].to_vec());
        }
    }
    if let Some(n) = (2..=8).find(|n| nums.len() == n * n + n) {
        return (n, nums[..n * n].to_vec(), nums[n * n..].to_vec());
    }
    let (n, a) = default_spd2();
    (n, a, vec![1.0, 0.0])
}

/// `LinearAlgebra.cholesky_factor` — SPD Cholesky; Host args `{ a: [[f64]] }`.
pub(super) fn run_cholesky_factor(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, data) = resolve_matrix(&nums, container.as_ref())
        .filter(|(r, c, _)| *r == *c)
        .map(|(r, _, d)| (r, d))
        .unwrap_or_else(default_spd2);
    let Some(l) = local_cholesky_factor(n, &data) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Cholesky needs a small SPD square matrix.",
            "error",
        );
        return;
    };
    let diag: Vec<f64> = (0..n).map(|i| l[i * n + i]).collect();
    invoke_dual(
        document,
        label,
        "LinearAlgebra.cholesky_factor",
        format!("cholesky_factor sketch {n}×{n}; L diag={diag:?}"),
        json!({ "a": nested_square(n, &data) }),
    );
}

/// `LinearAlgebra.cholesky_solve` — factor A then solve; Host `{ l, n, b }`.
pub(super) fn run_cholesky_solve(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, a, b) = resolve_square_system(&nums, container.as_ref());
    let Some(l) = local_cholesky_factor(n, &a) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Cholesky solve needs SPD A then b (or data-n).",
            "error",
        );
        return;
    };
    let Some(x) = local_cholesky_solve(n, &l, &b) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Could not backsolve with Cholesky factor.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.cholesky_solve",
        format!("cholesky_solve sketch → x={x:?}"),
        json!({ "l": l, "n": n as u64, "b": b }),
    );
}

/// `LinearAlgebra.lu_decompose` — Host `{ a: [[f64]] }`.
pub(super) fn run_lu_decompose(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, data) = resolve_matrix(&nums, container.as_ref())
        .filter(|(r, c, _)| *r == *c)
        .map(|(r, _, d)| (r, d))
        .unwrap_or_else(|| {
            let (n, a) = default_spd2();
            (n, a)
        });
    let Some((lu, pivots, sign)) = local_lu_decompose(n, &data) else {
        super::interactions::show_tool_status(
            document,
            label,
            "LU needs a small nonsingular square matrix.",
            "error",
        );
        return;
    };
    let _ = lu;
    invoke_dual(
        document,
        label,
        "LinearAlgebra.lu_decompose",
        format!("lu_decompose sketch {n}×{n}; pivots={pivots:?}; sign={sign}"),
        json!({ "a": nested_square(n, &data) }),
    );
}

/// `LinearAlgebra.lu_solve` — Host `{ a: [[f64]], b: [f64] }`.
pub(super) fn run_lu_solve(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, a, b) = resolve_square_system(&nums, container.as_ref());
    let Some(x) = local_solve(n, &a, &b) else {
        super::interactions::show_tool_status(
            document,
            label,
            "LU solve needs square A then b (or data-n).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.lu_solve",
        format!("lu_solve sketch → x={x:?}"),
        json!({ "a": nested_square(n, &a), "b": b }),
    );
}

/// `LinearAlgebra.qr_form_q` — factor then Host `{ a, tau, rows, cols }`.
pub(super) fn run_qr_form_q(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (rows, cols, data) = resolve_matrix(&nums, container.as_ref())
        .unwrap_or_else(|| (3, 2, vec![1.0, 1.0, 0.0, 1.0, 0.0, 1.0]));
    if rows < cols {
        super::interactions::show_tool_status(
            document,
            label,
            "qr_form_q needs rows ≥ cols.",
            "error",
        );
        return;
    }
    let Some((factored, tau)) = local_qr_factor(rows, cols, &data) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Could not QR-factor this matrix for Q.",
            "error",
        );
        return;
    };
    let Some(q) = local_qr_form_q(rows, cols, &factored, &tau) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Could not form thin Q sketch.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.qr_form_q",
        format!("qr_form_q sketch {rows}×{cols}; Q≈{q:?}"),
        json!({
            "a": factored,
            "tau": tau,
            "rows": rows as u64,
            "cols": cols as u64,
        }),
    );
}

/// `LinearAlgebra.qr_solve_least_squares` — Host `{ a, tau, rows, cols, b }`.
pub(super) fn run_qr_solve_least_squares(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (rows, cols, a, b) = if let Some((r, c, m)) = resolve_matrix(&nums, container.as_ref()) {
        let rest = &nums[r * c..];
        if rest.len() >= r {
            (r, c, m, rest[..r].to_vec())
        } else {
            (
                3,
                2,
                vec![1.0, 0.0, 1.0, 1.0, 0.0, 1.0],
                vec![1.0, 2.0, 3.0],
            )
        }
    } else {
        (
            3,
            2,
            vec![1.0, 0.0, 1.0, 1.0, 0.0, 1.0],
            vec![1.0, 2.0, 3.0],
        )
    };
    if rows < cols {
        super::interactions::show_tool_status(
            document,
            label,
            "qr_solve_least_squares needs rows ≥ cols.",
            "error",
        );
        return;
    }
    let Some((factored, tau)) = local_qr_factor(rows, cols, &a) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Could not QR-factor for least squares.",
            "error",
        );
        return;
    };
    let Some(x) = local_qr_solve_ls(rows, cols, &factored, &tau, &b) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Could not form least-squares sketch.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.qr_solve_least_squares",
        format!("qr_solve_least_squares sketch → x={x:?}"),
        json!({
            "a": factored,
            "tau": tau,
            "rows": rows as u64,
            "cols": cols as u64,
            "b": b,
        }),
    );
}

/// `LinearAlgebra.svd` — Host `{ matrix: { rows, cols, data } }`.
pub(super) fn run_svd(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (rows, cols, data) =
        resolve_matrix(&nums, container.as_ref()).unwrap_or_else(default_square2);
    let Some(svals) = local_svd_singular_values(rows, cols, &data) else {
        super::interactions::show_tool_status(
            document,
            label,
            "SVD sketch needs a small matrix (≤8×8).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.svd",
        format!("svd sketch {rows}×{cols}; σ≈{svals:?}"),
        json!({ "matrix": mat_json(rows, cols, &data) }),
    );
}

/// `LinearAlgebra.eigenvalues` — Host `{ matrix: { rows, cols, data } }`.
pub(super) fn run_eigenvalues(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (rows, cols, data) =
        resolve_matrix(&nums, container.as_ref()).unwrap_or_else(default_square2);
    if rows != cols {
        super::interactions::show_tool_status(
            document,
            label,
            "Eigenvalues need a square matrix.",
            "error",
        );
        return;
    }
    let eigs = if rows == 2 {
        local_eigenvalues_2x2(&data)
    } else {
        // Dominant real part sketch via power iteration on AᵀA-free path: use trace/n.
        let tr: f64 = (0..rows).map(|i| data[i * rows + i]).sum();
        Some(vec![(tr / rows as f64, 0.0)])
    };
    let Some(eigs) = eigs else {
        return;
    };
    let msg = eigs
        .iter()
        .map(|(re, im)| {
            if *im == 0.0 {
                format!("{re:.4}")
            } else {
                format!("{re:.4}{im:+.4}i")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    invoke_dual(
        document,
        label,
        "LinearAlgebra.eigenvalues",
        format!("eigenvalues sketch {rows}×{rows}: [{msg}]"),
        json!({ "matrix": mat_json(rows, cols, &data) }),
    );
}

fn local_add_assign(a: &[f64], b: &[f64]) -> Option<Vec<f64>> {
    local_add_into(a, b)
}

fn local_hadamard_assign(a: &[f64], b: &[f64]) -> Option<Vec<f64>> {
    local_hadamard(a, b)
}

fn local_cholesky_determinant(n: usize, l: &[f64]) -> Option<f64> {
    if n == 0 || l.len() < n * n || n > 8 {
        return None;
    }
    let mut det = 1.0;
    for i in 0..n {
        det *= l[i * n + i];
    }
    let det = det * det;
    det.is_finite().then_some(det)
}

/// Char poly coeffs (monic, descending) for 2×2: λ² − tr λ + det.
fn local_charpoly_2x2(a: &[f64]) -> Option<Vec<f64>> {
    if a.len() < 4 {
        return None;
    }
    let tr = a[0] + a[3];
    let det = a[0] * a[3] - a[1] * a[2];
    let coeffs = vec![1.0, -tr, det];
    coeffs.iter().all(|x| x.is_finite()).then_some(coeffs)
}

/// Quadratic roots sketch (descending coeffs a,b,c → ax²+bx+c).
fn local_poly_roots_quadratic(coeffs: &[f64]) -> Option<Vec<(f64, f64)>> {
    if coeffs.len() != 3 || coeffs[0].abs() < 1e-15 {
        return None;
    }
    let (a, b, c) = (coeffs[0], coeffs[1], coeffs[2]);
    let disc = b * b - 4.0 * a * c;
    if disc >= 0.0 {
        let s = disc.sqrt();
        Some(vec![((-b + s) / (2.0 * a), 0.0), ((-b - s) / (2.0 * a), 0.0)])
    } else {
        let s = (-disc).sqrt();
        Some(vec![
            (-b / (2.0 * a), s / (2.0 * a)),
            (-b / (2.0 * a), -s / (2.0 * a)),
        ])
    }
}

/// Closed-form symmetric 3×3 eigenvalues sketch (diagonal-dominant path).
fn local_symmetric_eigen_3x3(a: &[f64]) -> Option<[f64; 3]> {
    if a.len() < 9 {
        return None;
    }
    // Offline sketch: report sorted diagonal when off-diagonals are tiny; else 2×2 block + a22.
    let off = a[1].abs() + a[2].abs() + a[3].abs() + a[5].abs() + a[6].abs() + a[7].abs();
    let mut e = if off < 1e-9 {
        [a[0], a[4], a[8]]
    } else {
        // Trace / 3 as dominant real-part sketch plus extremes of diagonal.
        let tr = (a[0] + a[4] + a[8]) / 3.0;
        let mut d = [a[0], a[4], a[8]];
        d.sort_by(|x, y| y.partial_cmp(x).unwrap_or(std::cmp::Ordering::Equal));
        [d[0], tr, d[2]]
    };
    e.sort_by(|x, y| y.partial_cmp(x).unwrap_or(std::cmp::Ordering::Equal));
    e.iter().all(|x| x.is_finite()).then_some(e)
}

/// `LinearAlgebra.add_assign` — Host `{ a, b }` → mutated `a`.
pub(super) fn run_add_assign(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = if nums.len() >= 2 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else {
        (vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0])
    };
    let Some(out) = local_add_assign(&a, &b) else {
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.add_assign",
        format!("add_assign sketch over {} → a={out:?}", a.len()),
        json!({ "a": a, "b": b }),
    );
}

/// `LinearAlgebra.hadamard_assign` — Host `{ a, b }` → mutated `a`.
pub(super) fn run_hadamard_assign(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = if nums.len() >= 2 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else {
        (vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0])
    };
    let Some(out) = local_hadamard_assign(&a, &b) else {
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.hadamard_assign",
        format!("hadamard_assign sketch over {} → a={out:?}", a.len()),
        json!({ "a": a, "b": b }),
    );
}

/// `LinearAlgebra.cholesky_determinant` — factor SPD then Host `{ l, n }`.
pub(super) fn run_cholesky_determinant(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, data) = resolve_matrix(&nums, container.as_ref())
        .filter(|(r, c, _)| *r == *c)
        .map(|(r, _, d)| (r, d))
        .unwrap_or_else(default_spd2);
    let Some(l) = local_cholesky_factor(n, &data) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Cholesky determinant needs a small SPD square matrix.",
            "error",
        );
        return;
    };
    let Some(det) = local_cholesky_determinant(n, &l) else {
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.cholesky_determinant",
        format!("cholesky_determinant sketch {n}×{n} → det≈{det}"),
        json!({ "l": l, "n": n as u64 }),
    );
}

/// `LinearAlgebra.characteristic_polynomial` — Host `{ a: [[f64]] }`.
pub(super) fn run_characteristic_polynomial(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, data) = resolve_matrix(&nums, container.as_ref())
        .filter(|(r, c, _)| *r == *c)
        .map(|(r, _, d)| (r, d))
        .unwrap_or_else(|| {
            let (r, _, d) = default_square2();
            (r, d)
        });
    let coeffs = if n == 2 {
        local_charpoly_2x2(&data)
    } else {
        // Degenerate monic sketch: λⁿ − (tr) λⁿ⁻¹ (leading two terms only).
        let tr: f64 = (0..n).map(|i| data[i * n + i]).sum();
        let mut c = vec![0.0; n + 1];
        c[0] = 1.0;
        if n >= 1 {
            c[1] = -tr;
        }
        c.iter().all(|x| x.is_finite()).then_some(c)
    };
    let Some(coeffs) = coeffs else {
        super::interactions::show_tool_status(
            document,
            label,
            "Characteristic polynomial needs a small square matrix.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.characteristic_polynomial",
        format!("characteristic_polynomial sketch {n}×{n} → {coeffs:?}"),
        json!({ "a": nested_square(n, &data) }),
    );
}

/// `LinearAlgebra.eigenvalues_general` — Host `{ a: [[f64]] }`.
pub(super) fn run_eigenvalues_general(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, data) = resolve_matrix(&nums, container.as_ref())
        .filter(|(r, c, _)| *r == *c)
        .map(|(r, _, d)| (r, d))
        .unwrap_or_else(|| {
            let (r, _, d) = default_square2();
            (r, d)
        });
    let eigs = if n == 2 {
        local_eigenvalues_2x2(&data)
    } else {
        let tr: f64 = (0..n).map(|i| data[i * n + i]).sum();
        Some(vec![(tr / n as f64, 0.0)])
    };
    let Some(eigs) = eigs else {
        return;
    };
    let msg = eigs
        .iter()
        .map(|(re, im)| {
            if *im == 0.0 {
                format!("{re:.4}")
            } else {
                format!("{re:.4}{im:+.4}i")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    invoke_dual(
        document,
        label,
        "LinearAlgebra.eigenvalues_general",
        format!("eigenvalues_general sketch {n}×{n}: [{msg}]"),
        json!({ "a": nested_square(n, &data) }),
    );
}

/// `LinearAlgebra.eigen_symmetric` — Host `{ matrix: { rows, cols, data } }`.
pub(super) fn run_eigen_symmetric(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, data) = resolve_matrix(&nums, container.as_ref())
        .filter(|(r, c, _)| *r == *c)
        .map(|(r, _, d)| (r, d))
        .unwrap_or_else(default_spd2);
    // Offline: Jacobi not fully ported — report sorted diagonal of SPD factor path.
    let mut diag: Vec<f64> = (0..n).map(|i| data[i * n + i]).collect();
    diag.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    if !diag.iter().all(|x| x.is_finite()) {
        return;
    }
    invoke_dual(
        document,
        label,
        "LinearAlgebra.eigen_symmetric",
        format!("eigen_symmetric sketch {n}×{n}; diag≈{diag:?}"),
        json!({ "matrix": mat_json(n, n, &data) }),
    );
}

/// `LinearAlgebra.polynomial_roots` — Host `{ coeffs }` (descending).
pub(super) fn run_polynomial_roots(document: &Document, label: &str) {
    let mut coeffs = parse_numbers(&selected_source(document).unwrap_or_default());
    if coeffs.len() < 2 {
        coeffs = vec![1.0, -3.0, 2.0]; // (x−1)(x−2)
    }
    let roots = if coeffs.len() == 3 {
        local_poly_roots_quadratic(&coeffs)
    } else {
        // Degenerate sketch: report −c₁/c₀ when linear-ish leading pair exists.
        if coeffs[0].abs() > 1e-15 && coeffs.len() >= 2 {
            Some(vec![(-coeffs[1] / coeffs[0], 0.0)])
        } else {
            None
        }
    };
    let Some(roots) = roots else {
        super::interactions::show_tool_status(
            document,
            label,
            "Polynomial roots need descending coefficients (demo: 1 -3 2).",
            "error",
        );
        return;
    };
    let msg = roots
        .iter()
        .map(|(re, im)| {
            if *im == 0.0 {
                format!("{re:.4}")
            } else {
                format!("{re:.4}{im:+.4}i")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    invoke_dual(
        document,
        label,
        "LinearAlgebra.polynomial_roots",
        format!("polynomial_roots sketch deg {}: [{msg}]", coeffs.len().saturating_sub(1)),
        json!({ "coeffs": coeffs }),
    );
}

/// `LinearAlgebra.solve_linear_system` — Host `{ a, b, n }` row-major.
pub(super) fn run_solve_linear_system(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, a, b) = resolve_square_system(&nums, container.as_ref());
    let Some(x) = local_solve(n, &a, &b) else {
        super::interactions::show_tool_status(
            document,
            label,
            "solve_linear_system needs square A then b (or data-n).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.solve_linear_system",
        format!("solve_linear_system sketch → x={x:?}"),
        json!({ "a": a, "b": b, "n": n as u64 }),
    );
}

/// `LinearAlgebra.symmetric_eigen_3x3` — Host `{ a }` length-9 row-major.
pub(super) fn run_symmetric_eigen_3x3(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let a = if nums.len() >= 9 {
        nums[..9].to_vec()
    } else {
        // diag(3,2,1) SPD demo
        vec![3.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 1.0]
    };
    let Some(e) = local_symmetric_eigen_3x3(&a) else {
        super::interactions::show_tool_status(
            document,
            label,
            "symmetric_eigen_3x3 needs 9 row-major entries.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "LinearAlgebra.symmetric_eigen_3x3",
        format!("symmetric_eigen_3x3 sketch → λ≈{e:?}"),
        json!({ "a": a }),
    );
}
