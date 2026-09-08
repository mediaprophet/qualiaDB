//! Wave 40 Linear Algebra app/REPL Live tools (`dot`/`norm`/`trace`/`identity`/`inverse`)
//! plus surface-aware `LinearAlgebra.gemm`.

use serde_json::json;
use web_sys::Document;

fn local_dot(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }
    let v: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    v.is_finite().then_some(v)
}

fn local_norm(a: &[f64]) -> Option<f64> {
    if a.is_empty() {
        return None;
    }
    let v = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    v.is_finite().then_some(v)
}

fn local_trace(n: usize, a: &[f64]) -> Option<f64> {
    if n == 0 || a.len() < n * n {
        return None;
    }
    let v: f64 = (0..n).map(|i| a[i * n + i]).sum();
    v.is_finite().then_some(v)
}

fn identity_data(n: usize) -> Vec<f64> {
    let mut d = vec![0.0; n * n];
    for i in 0..n {
        d[i * n + i] = 1.0;
    }
    d
}

fn split_equal(nums: &[f64]) -> Option<(Vec<f64>, Vec<f64>)> {
    if nums.len() >= 2 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        Some((nums[..half].to_vec(), nums[half..].to_vec()))
    } else {
        None
    }
}

/// `LinearAlgebra.gemm` — two surface matrices, else A·I, else 2×2 demo.
pub(super) fn run_gemm(document: &Document, label: &str) {
    let container = super::linalg_chain_actions::selected_container(document);
    let nums = super::linalg_chain_actions::parse_numbers(
        &super::linalg_chain_actions::selected_source(document).unwrap_or_default(),
    );
    let (a, b) = if let Some((left, right)) = split_equal(&nums) {
        match (
            super::linalg_chain_actions::resolve_matrix(&left, container.as_ref()),
            super::linalg_chain_actions::resolve_matrix(&right, container.as_ref()),
        ) {
            (Some(a), Some(b)) if a.1 == b.0 => (a, b),
            _ => default_gemm_pair(),
        }
    } else if let Some((r, c, data)) =
        super::linalg_chain_actions::resolve_matrix(&nums, container.as_ref())
    {
        ((r, c, data), (c, c, identity_data(c)))
    } else {
        default_gemm_pair()
    };
    if a.1 != b.0 {
        return;
    }
    let sketch = format!(
        "gemm sketch {}×{} · {}×{}",
        a.0, a.1, b.0, b.1
    );
    super::linalg_chain_actions::invoke_dual(
        document,
        label,
        "LinearAlgebra.gemm",
        sketch,
        json!({
            "a": super::linalg_chain_actions::mat_json(a.0, a.1, &a.2),
            "b": super::linalg_chain_actions::mat_json(b.0, b.1, &b.2),
            "alpha": 1.0,
            "beta": 0.0
        }),
    );
}

fn default_gemm_pair() -> ((usize, usize, Vec<f64>), (usize, usize, Vec<f64>)) {
    (
        (2, 2, vec![1.0, 2.0, 3.0, 4.0]),
        (2, 2, vec![1.0, 0.0, 0.0, 1.0]),
    )
}

/// `LinearAlgebra.dot` — equal halves of the surface, else a demo pair.
pub(super) fn run_dot(document: &Document, label: &str) {
    let nums = super::linalg_chain_actions::parse_numbers(
        &super::linalg_chain_actions::selected_source(document).unwrap_or_default(),
    );
    let (a, b) = split_equal(&nums).unwrap_or_else(|| (vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]));
    let Some(value) = local_dot(&a, &b) else {
        return;
    };
    super::linalg_chain_actions::invoke_dual(
        document,
        label,
        "LinearAlgebra.dot",
        format!("dot sketch over {} → {value}", a.len()),
        json!({ "a": a, "b": b }),
    );
}

/// `LinearAlgebra.norm` — L2 of surface numbers, else `[3, 4]`.
pub(super) fn run_norm(document: &Document, label: &str) {
    let mut nums = super::linalg_chain_actions::parse_numbers(
        &super::linalg_chain_actions::selected_source(document).unwrap_or_default(),
    );
    if nums.is_empty() {
        nums = vec![3.0, 4.0];
    }
    let Some(value) = local_norm(&nums) else {
        return;
    };
    super::linalg_chain_actions::invoke_dual(
        document,
        label,
        "LinearAlgebra.norm",
        format!("norm sketch over {} → {value}", nums.len()),
        json!({ "a": nums }),
    );
}

/// `LinearAlgebra.trace` — square from surface / attrs, else 2×2 demo.
pub(super) fn run_trace(document: &Document, label: &str) {
    let container = super::linalg_chain_actions::selected_container(document);
    let nums = super::linalg_chain_actions::parse_numbers(
        &super::linalg_chain_actions::selected_source(document).unwrap_or_default(),
    );
    let (n, _c, data) = super::linalg_chain_actions::resolve_matrix(&nums, container.as_ref())
        .filter(|(r, c, _)| *r == *c)
        .unwrap_or_else(|| super::linalg_chain_actions::default_square2());
    let Some(value) = local_trace(n, &data) else {
        return;
    };
    super::linalg_chain_actions::invoke_dual(
        document,
        label,
        "LinearAlgebra.trace",
        format!("trace sketch {n}×{n} → {value}"),
        json!({ "a": super::linalg_chain_actions::mat_json(n, n, &data) }),
    );
}

/// `LinearAlgebra.identity` — `data-n` or inferred side, else n=3.
pub(super) fn run_identity(document: &Document, label: &str) {
    let container = super::linalg_chain_actions::selected_container(document);
    let n = super::linalg_chain_actions::usize_attr(container.as_ref(), "data-n")
        .filter(|n| *n >= 1 && *n <= 256)
        .unwrap_or(3);
    super::linalg_chain_actions::invoke_dual(
        document,
        label,
        "LinearAlgebra.identity",
        format!("identity sketch n={n}"),
        json!({ "n": n as u64 }),
    );
}

/// `LinearAlgebra.inverse` — square from surface, else invertible 2×2 demo.
pub(super) fn run_inverse(document: &Document, label: &str) {
    let container = super::linalg_chain_actions::selected_container(document);
    let nums = super::linalg_chain_actions::parse_numbers(
        &super::linalg_chain_actions::selected_source(document).unwrap_or_default(),
    );
    let (n, _c, data) = super::linalg_chain_actions::resolve_matrix(&nums, container.as_ref())
        .filter(|(r, c, _)| *r == *c && *r <= 64)
        .unwrap_or((2, 2, vec![1.0, 2.0, 3.0, 4.0]));
    super::linalg_chain_actions::invoke_dual(
        document,
        label,
        "LinearAlgebra.inverse",
        format!("inverse sketch {n}×{n}"),
        json!({ "a": super::linalg_chain_actions::mat_json(n, n, &data) }),
    );
}
