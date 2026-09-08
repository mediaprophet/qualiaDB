//! Wave-10/11 Host binds: statistical-manifold pure helpers.
//!
//! Wraps `specialized_libs::computational_geometry::statistical_manifold`
//! `pub fn`s that had no exact `Statistics.*` Host twin.
//! Wave-10: validate_probability / simplex_project / fisher_distance / neg_entropy.
//! Wave-11: remaining free `pub fn`s from the same module.

use super::super::args;
use crate::specialized_libs::computational_geometry::statistical_manifold as sm;
use vibe::{Diagnostic, Span, Value};

fn f32_list(args_v: &Value, key: &str, span: Span) -> Result<Vec<f32>, Diagnostic> {
    let xs = args::rec_f64_list(args_v, key)
        .ok_or_else(|| args::bad(span, format!("Statistics needs {key}: number list")))?;
    Ok(xs.into_iter().map(|v| v as f32).collect())
}

fn f32_list_value(xs: &[f32]) -> Value {
    args::f64_list_value(xs.iter().map(|&v| v as f64))
}

fn f64_list_value(xs: &[f64]) -> Value {
    args::f64_list_value(xs.iter().copied())
}

/// `Statistics.validate_probability` — non-negative, sums to 1.
/// Args: `{ p }`. Out: `{ ok: true }` or error.
pub fn validate_probability(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = f32_list(args_v, "p", span)?;
    sm::validate_probability(&p).map_err(|e| args::bad(span, format!("validate_probability: {e}")))?;
    Ok(args::record([("ok", Value::Bool(true))]))
}

/// `Statistics.simplex_project` — Euclidean projection onto the probability simplex.
/// Args: `{ p }`. Out: `{ q }` (projected).
pub fn simplex_project(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = f32_list(args_v, "p", span)?;
    let mut out = vec![0.0f32; p.len()];
    sm::simplex_project(&p, &mut out)
        .map_err(|e| args::bad(span, format!("simplex_project: {e}")))?;
    Ok(args::record([("q", f32_list_value(&out))]))
}

/// `Statistics.fisher_distance` — Fisher–Rao geodesic distance on the simplex.
/// Args: `{ p, q }`. Out: `{ value }`.
pub fn fisher_distance(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = f32_list(args_v, "p", span)?;
    let q = f32_list(args_v, "q", span)?;
    let d = sm::fisher_distance(&p, &q)
        .map_err(|e| args::bad(span, format!("fisher_distance: {e}")))?;
    Ok(args::record([("value", Value::F64(d))]))
}

/// `Statistics.neg_entropy` — `Σ pᵢ log(pᵢ)` (KL Bregman generator).
/// Args: `{ p }`. Out: `{ value }`.
pub fn neg_entropy(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = f32_list(args_v, "p", span)?;
    Ok(args::record([("value", Value::F64(sm::neg_entropy(&p)))]))
}

/// `Statistics.simplex_project_idempotent` — project(project(p)) == project(p).
/// Args: `{ p }`. Out: `{ ok }`.
pub fn simplex_project_idempotent(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = f32_list(args_v, "p", span)?;
    Ok(args::record([(
        "ok",
        Value::Bool(sm::simplex_project_idempotent(&p)),
    )]))
}

/// `Statistics.fisher_inner_product` — ⟨u, v⟩_p = Σ uᵢ vᵢ / pᵢ.
/// Args: `{ p, u, v }`. Out: `{ value }`.
pub fn fisher_inner_product(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = f32_list(args_v, "p", span)?;
    let u = f32_list(args_v, "u", span)?;
    let v = f32_list(args_v, "v", span)?;
    let ip = sm::fisher_inner_product(&p, &u, &v)
        .map_err(|e| args::bad(span, format!("fisher_inner_product: {e}")))?;
    Ok(args::record([("value", Value::F64(ip))]))
}

/// `Statistics.neg_entropy_grad` — ∇ψ(p)_i = log(pᵢ) + 1.
/// Args: `{ p }`. Out: `{ grad }`.
pub fn neg_entropy_grad(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = f32_list(args_v, "p", span)?;
    let mut out = vec![0.0f64; p.len()];
    sm::neg_entropy_grad(&p, &mut out);
    Ok(args::record([("grad", f64_list_value(&out))]))
}

/// `Statistics.kl_bregman_form` — KL via Bregman generator ψ = neg-entropy.
/// Args: `{ p, q }`. Out: `{ value }`.
pub fn kl_bregman_form(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = f32_list(args_v, "p", span)?;
    let q = f32_list(args_v, "q", span)?;
    let kl = sm::kl_bregman_form(&p, &q)
        .map_err(|e| args::bad(span, format!("kl_bregman_form: {e}")))?;
    Ok(args::record([("value", Value::F64(kl))]))
}

/// `Statistics.bregman_pythagorean_test` — returns (KL(p‖q), KL(p‖q*), KL(q*‖q)).
/// Args: `{ p, q_star, q }`. Out: `{ kl_pq, kl_pqstar, kl_qstar_q }`.
pub fn bregman_pythagorean_test(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = f32_list(args_v, "p", span)?;
    let q_star = f32_list(args_v, "q_star", span)?;
    let q = f32_list(args_v, "q", span)?;
    let (kl_pq, kl_pqstar, kl_qstar_q) = sm::bregman_pythagorean_test(&p, &q_star, &q)
        .map_err(|e| args::bad(span, format!("bregman_pythagorean_test: {e}")))?;
    Ok(args::record([
        ("kl_pq", Value::F64(kl_pq)),
        ("kl_pqstar", Value::F64(kl_pqstar)),
        ("kl_qstar_q", Value::F64(kl_qstar_q)),
    ]))
}

/// `Statistics.probability_hash` — FNV-1a over f32 bit patterns.
/// Args: `{ p }`. Out: `{ hash }`.
pub fn probability_hash(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = f32_list(args_v, "p", span)?;
    Ok(args::record([(
        "hash",
        Value::U64(sm::probability_hash(&p)),
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave10_validate_probability_ok() {
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value(vec![0.5, 0.5]));
        let out = validate_probability(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_bool(&out, "ok"), Some(true));
    }

    #[test]
    fn wave10_validate_probability_rejects() {
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value(vec![0.5, 0.6]));
        assert!(validate_probability(&Value::Record(m), span()).is_err());
    }

    #[test]
    fn wave10_simplex_project_normalises() {
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value(vec![1.0, 1.0, 0.0]));
        let out = simplex_project(&Value::Record(m), span()).unwrap();
        let q = args::rec(&out, "q").and_then(args::f64s).unwrap();
        assert_eq!(q.len(), 3);
        let sum: f64 = q.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        assert!((q[0] - 0.5).abs() < 1e-6);
        assert!((q[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn wave10_fisher_distance_self_zero() {
        // Use the same simplex point as the specialized_libs unit test.
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value(vec![0.3, 0.3, 0.4]));
        m.insert("q".into(), args::f64_list_value(vec![0.3, 0.3, 0.4]));
        let out = fisher_distance(&Value::Record(m), span()).unwrap();
        let d = args::rec_f64(&out, "value").unwrap();
        assert!(d.abs() < 1e-6, "Fisher distance to self must be ~0, got {d}");
    }

    #[test]
    fn wave10_neg_entropy_uniform_two() {
        // 0.5 ln 0.5 + 0.5 ln 0.5 = ln 0.5
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value(vec![0.5, 0.5]));
        let out = neg_entropy(&Value::Record(m), span()).unwrap();
        let v = args::rec_f64(&out, "value").unwrap();
        assert!((v - 0.5f64.ln()).abs() < 1e-9);
    }

    #[test]
    fn wave11_simplex_project_idempotent_true() {
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value(vec![1.0, 1.0, 0.0]));
        let out = simplex_project_idempotent(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_bool(&out, "ok"), Some(true));
    }

    #[test]
    fn wave11_fisher_inner_product_unit() {
        // At p=(0.5,0.5), u=v=(1,-1): Σ u_i^2 / p_i = 1/0.5 + 1/0.5 = 4
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value(vec![0.5, 0.5]));
        m.insert("u".into(), args::f64_list_value(vec![1.0, -1.0]));
        m.insert("v".into(), args::f64_list_value(vec![1.0, -1.0]));
        let out = fisher_inner_product(&Value::Record(m), span()).unwrap();
        let ip = args::rec_f64(&out, "value").unwrap();
        assert!((ip - 4.0).abs() < 1e-9, "got {ip}");
    }

    #[test]
    fn wave11_neg_entropy_grad_uniform() {
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value(vec![0.5, 0.5]));
        let out = neg_entropy_grad(&Value::Record(m), span()).unwrap();
        let g = args::rec(&out, "grad").and_then(args::f64s).unwrap();
        assert_eq!(g.len(), 2);
        let expect = 0.5f64.ln() + 1.0;
        assert!((g[0] - expect).abs() < 1e-9);
        assert!((g[1] - expect).abs() < 1e-9);
    }

    #[test]
    fn wave11_kl_bregman_matches_self_zero() {
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value(vec![0.2, 0.5, 0.3]));
        m.insert("q".into(), args::f64_list_value(vec![0.2, 0.5, 0.3]));
        let out = kl_bregman_form(&Value::Record(m), span()).unwrap();
        let v = args::rec_f64(&out, "value").unwrap();
        assert!(v.abs() < 1e-10, "KL Bregman self must be ~0, got {v}");
    }

    #[test]
    fn wave11_bregman_pythagorean_parts() {
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value(vec![0.5, 0.5]));
        m.insert("q_star".into(), args::f64_list_value(vec![0.5, 0.5]));
        m.insert("q".into(), args::f64_list_value(vec![0.25, 0.75]));
        let out = bregman_pythagorean_test(&Value::Record(m), span()).unwrap();
        let kl_pq = args::rec_f64(&out, "kl_pq").unwrap();
        let kl_pqstar = args::rec_f64(&out, "kl_pqstar").unwrap();
        let kl_qstar_q = args::rec_f64(&out, "kl_qstar_q").unwrap();
        assert!(kl_pqstar.abs() < 1e-12);
        assert!((kl_pq - (kl_pqstar + kl_qstar_q)).abs() < 1e-9);
    }

    #[test]
    fn wave11_probability_hash_stable() {
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value(vec![0.25, 0.75]));
        let out = probability_hash(&Value::Record(m.clone()), span()).unwrap();
        let h1 = match args::rec(&out, "hash") {
            Some(Value::U64(v)) => *v,
            other => panic!("expected U64 hash, got {other:?}"),
        };
        let out2 = probability_hash(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec(&out2, "hash"), Some(&Value::U64(h1)));
        assert_ne!(h1, 0);
    }
}
