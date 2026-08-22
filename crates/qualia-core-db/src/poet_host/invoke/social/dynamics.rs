//! Social dynamics and forensic economics invoke seam.
//!
//! Exposes welfare metrics (Gini, Lorenz), network centrality, and forensic
//! economics functions from specialized_libs::computational_economics.

use super::super::args;
use vibe::{DiagCode, Diagnostic, Span, Value};

/// `Social.gini` — compute the Gini coefficient of a distribution.
/// Takes `incomes` (list of f64). Returns the Gini coefficient (0 = equal, 1 = unequal).
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub fn gini(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::welfare;
    let incomes = args::rec_f64_list(args, "incomes")
        .ok_or_else(|| args::bad(span, "Social.gini needs incomes"))?;
    if incomes.is_empty() {
        return Err(args::bad(span, "Social.gini needs non-empty incomes"));
    }
    match welfare::gini_coefficient(&incomes) {
        Ok(g) => Ok(args::record([("gini", Value::F64(g))])),
        Err(e) => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("Social.gini: {e:?}"),
        )),
    }
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn gini(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::need_scientific(span, "Social.gini"))
}

/// `Social.lorenz` — compute Lorenz curve points.
/// Takes `incomes` (list of f64). Returns list of cumulative share points.
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub fn lorenz(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::welfare;
    let incomes = args::rec_f64_list(args, "incomes")
        .ok_or_else(|| args::bad(span, "Social.lorenz needs incomes"))?;
    if incomes.is_empty() {
        return Err(args::bad(span, "Social.lorenz needs non-empty incomes"));
    }
    // lorenz_curve_into writes up to 2*n points (cumulative population + income shares).
    let mut out = vec![0.0f64; incomes.len() * 2];
    match welfare::lorenz_curve_into(&incomes, &mut out) {
        Ok(count) => Ok(args::f64_list_value(out[..count].iter().copied())),
        Err(e) => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("Social.lorenz: {e:?}"),
        )),
    }
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn lorenz(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::need_scientific(span, "Social.lorenz"))
}

/// `Social.degree_centrality` — compute degree centrality for a network.
/// Takes `adjacency` (flat row-major n×n matrix as list of f64) and `n` (node count).
/// Returns list of centrality scores.
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub fn degree_centrality(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::network_economics;
    let adjacency = args::rec_f64_list(args, "adjacency")
        .ok_or_else(|| args::bad(span, "Social.degree_centrality needs adjacency"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Social.degree_centrality needs n"))? as usize;
    if adjacency.len() < n * n {
        return Err(args::bad(
            span,
            "Social.degree_centrality adjacency must be n*n",
        ));
    }
    let mut out = vec![0.0f64; n];
    match network_economics::degree_centrality_into(&adjacency, n, &mut out) {
        Ok(count) => Ok(args::f64_list_value(out[..count].iter().copied())),
        Err(e) => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("Social.degree_centrality: {e:?}"),
        )),
    }
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn degree_centrality(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::need_scientific(span, "Social.degree_centrality"))
}

/// `Forensic.malfeasance_delta` — compute the malfeasance delta from allocated
/// capital and delivered utility. Takes `capital_allocated`, `delivered_utility`,
/// and `inverted` (bool). Returns the delta record.
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub fn malfeasance_delta(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::forensic_economics;
    let capital = args::rec_f64(args, "capital_allocated")
        .ok_or_else(|| args::bad(span, "Forensic.malfeasance_delta needs capital_allocated"))?;
    let delivered = args::rec_f64(args, "delivered_utility")
        .ok_or_else(|| args::bad(span, "Forensic.malfeasance_delta needs delivered_utility"))?;
    let inverted = args::rec_bool(args, "inverted").unwrap_or(false);
    match forensic_economics::compute_malfeasance_delta(capital, delivered, inverted) {
        Ok(delta) => Ok(args::record([
            ("capital_allocated", Value::F64(delta.capital_allocated)),
            ("delivered_utility", Value::F64(delta.delivered_utility)),
            ("delta", Value::F64(delta.delta)),
            ("inverted", Value::Bool(delta.governance_yield_inverted)),
        ])),
        Err(e) => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("Forensic.malfeasance_delta: {e:?}"),
        )),
    }
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn malfeasance_delta(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::need_scientific(span, "Forensic.malfeasance_delta"))
}

/// `Forensic.narrative_divergence` — compute divergence between factual and
/// fantasy nquin traces. Takes `factual` and `fantasy` (lists of records with
/// 6 f64 fields). Returns divergence record.
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub fn narrative_divergence(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::forensic_economics;

    let factual_val = args::rec(args, "factual")
        .ok_or_else(|| args::bad(span, "Forensic.narrative_divergence needs factual"))?;
    let fantasy_val = args::rec(args, "fantasy")
        .ok_or_else(|| args::bad(span, "Forensic.narrative_divergence needs fantasy"))?;

    let factual_list = args::list(factual_val)
        .ok_or_else(|| args::bad(span, "Forensic.narrative_divergence factual must be a list"))?;
    let fantasy_list = args::list(fantasy_val)
        .ok_or_else(|| args::bad(span, "Forensic.narrative_divergence fantasy must be a list"))?;

    let parse_nquin = |v: &Value| -> Option<forensic_economics::NquinVector> {
        let fields = args::list(v)?;
        let dims: Vec<f64> = fields.iter().filter_map(|x| args::as_f64(x)).collect();
        if dims.len() >= forensic_economics::NQUIN_DIMS {
            Some(forensic_economics::NquinVector::from_array([
                dims[0], dims[1], dims[2], dims[3], dims[4],
            ]))
        } else {
            None
        }
    };

    let factual: Vec<forensic_economics::NquinVector> =
        factual_list.iter().filter_map(parse_nquin).collect();
    let fantasy: Vec<forensic_economics::NquinVector> =
        fantasy_list.iter().filter_map(parse_nquin).collect();

    if factual.is_empty() || fantasy.is_empty() {
        return Err(args::bad(
            span,
            "Forensic.narrative_divergence needs non-empty traces",
        ));
    }

    match forensic_economics::compute_narrative_divergence(&factual, &fantasy) {
        Ok(div) => Ok(args::record([
            ("propagated_cost", Value::F64(div.propagated_cost)),
            ("maintenance_active", Value::Bool(div.maintenance_active)),
        ])),
        Err(e) => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("Forensic.narrative_divergence: {e:?}"),
        )),
    }
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn narrative_divergence(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::need_scientific(span, "Forensic.narrative_divergence"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn gini_equal_distribution() {
        let mut m = BTreeMap::new();
        m.insert("incomes".into(), Value::List(vec![Value::F64(100.0); 5]));
        let result = gini(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => match rec.get("gini") {
                Some(Value::F64(g)) => assert!(*g < 0.01, "equal distribution should have ~0 gini"),
                _ => panic!("expected f64"),
            },
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn gini_unequal_distribution() {
        let mut m = BTreeMap::new();
        m.insert(
            "incomes".into(),
            Value::List(vec![
                Value::F64(0.0),
                Value::F64(0.0),
                Value::F64(0.0),
                Value::F64(0.0),
                Value::F64(100.0),
            ]),
        );
        let result = gini(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => match rec.get("gini") {
                Some(Value::F64(g)) => {
                    assert!(*g > 0.5, "unequal distribution should have high gini")
                }
                _ => panic!("expected f64"),
            },
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn lorenz_returns_curve() {
        let mut m = BTreeMap::new();
        m.insert(
            "incomes".into(),
            Value::List(vec![Value::F64(10.0), Value::F64(20.0), Value::F64(30.0)]),
        );
        let result = lorenz(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::List(pts) => assert!(!pts.is_empty()),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn degree_centrality_simple() {
        // 3x3 adjacency matrix: 0→1, 1→2
        let mut m = BTreeMap::new();
        m.insert(
            "adjacency".into(),
            Value::List(vec![
                Value::F64(0.0),
                Value::F64(1.0),
                Value::F64(0.0),
                Value::F64(0.0),
                Value::F64(0.0),
                Value::F64(1.0),
                Value::F64(0.0),
                Value::F64(0.0),
                Value::F64(0.0),
            ]),
        );
        m.insert("n".into(), Value::U64(3));
        let result = degree_centrality(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn malfeasance_delta_basic() {
        let mut m = BTreeMap::new();
        m.insert("capital_allocated".into(), Value::F64(100.0));
        m.insert("delivered_utility".into(), Value::F64(30.0));
        m.insert("inverted".into(), Value::Bool(false));
        let result = malfeasance_delta(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => match rec.get("delta") {
                Some(Value::F64(d)) => assert!(*d > 0.0, "deficit should be positive"),
                _ => panic!("expected f64 delta"),
            },
            _ => panic!("expected record"),
        }
    }
}
