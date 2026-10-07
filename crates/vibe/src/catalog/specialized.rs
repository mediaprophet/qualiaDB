//! Specialized LocalHost kernels (UE-040…043): econ extras, geometry, clinical/chem honesty.

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

/// Attempt a specialized LocalHost invoke. `None` when not handled here.
pub fn try_invoke(id: &str, args: &Value, span: Span) -> Option<Result<Value, Diagnostic>> {
    match id {
        "Econ.rawlsian_welfare" => Some(rawlsian(args, span)),
        "Econ.nash_welfare" => Some(nash(args, span)),
        "Econ.headcount_poverty" => Some(headcount(args, span)),
        "Econ.npv" => Some(npv(args, span)),
        "ComputationalGeometry.orientation_2" => Some(orientation_2(args, span)),
        "ComputationalGeometry.distance_2d" => Some(distance_2d(args, span)),
        "ComputationalGeometry.convex_hull_2" => Some(convex_hull_2(args, span)),
        "ClinicalRisk.framingham"
        | "OrganicChemistry.compute"
        | "OrganicChemistry.validate_smiles" => Some(engine_required(id, span)),
        _ => None,
    }
}

fn engine_required(id: &str, span: Span) -> Result<Value, Diagnostic> {
    Err(Diagnostic::new(
        DiagCode::E300,
        span,
        format!(
            "{id} on LocalHost requires Qualia engine host (Poet) or WASM \
             (compute_framingham_risk_wasm / evaluate_lipinski_wasm / …)"
        ),
    ))
}

fn rawlsian(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let u = f64_list(args, "utilities")
        .or_else(|| f64_list(args, "incomes"))
        .ok_or_else(|| {
            Diagnostic::new(
                DiagCode::E100,
                span,
                "Econ.rawlsian_welfare needs utilities: [f64]",
            )
        })?;
    if u.is_empty() {
        return Err(Diagnostic::new(DiagCode::E100, span, "rawlsian: empty"));
    }
    let min = u.iter().copied().fold(f64::INFINITY, f64::min);
    Ok(receipt(
        "Econ.rawlsian_welfare",
        [("welfare", Value::F64(min))],
    ))
}

fn nash(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let u = f64_list(args, "utilities").ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.nash_welfare needs utilities: [f64]",
        )
    })?;
    if u.is_empty() || u.iter().any(|x| *x <= 0.0) {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "nash: need strictly positive utilities",
        ));
    }
    let product: f64 = u.iter().product();
    Ok(receipt(
        "Econ.nash_welfare",
        [("welfare", Value::F64(product))],
    ))
}

fn headcount(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let incomes = f64_list(args, "incomes").ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.headcount_poverty needs incomes: [f64]",
        )
    })?;
    let line = f64_field(args, "poverty_line").ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.headcount_poverty needs poverty_line: f64",
        )
    })?;
    if line <= 0.0 {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "poverty_line must be > 0",
        ));
    }
    let poor = incomes.iter().filter(|&&x| x < line).count();
    let ratio = poor as f64 / incomes.len().max(1) as f64;
    Ok(receipt(
        "Econ.headcount_poverty",
        [
            ("count", Value::U64(poor as u64)),
            ("ratio", Value::F64(ratio)),
            ("n", Value::U64(incomes.len() as u64)),
        ],
    ))
}

fn npv(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let benefits = f64_list(args, "benefits")
        .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "Econ.npv needs benefits: [f64]"))?;
    let costs = f64_list(args, "costs").unwrap_or_else(|| vec![0.0; benefits.len()]);
    let rate = f64_field(args, "rate").unwrap_or(0.0);
    if benefits.len() != costs.len() || benefits.is_empty() {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.npv: benefits/costs length mismatch or empty",
        ));
    }
    if !rate.is_finite() || rate <= -1.0 {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.npv: invalid discount rate",
        ));
    }
    let mut npv = 0.0;
    let mut discount = 1.0;
    let one_plus = 1.0 + rate;
    for t in 0..benefits.len() {
        npv += (benefits[t] - costs[t]) * discount;
        discount /= one_plus;
    }
    Ok(receipt("Econ.npv", [("npv", Value::F64(npv))]))
}

fn orientation_2(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let ax = f64_field(args, "ax").or_else(|| point_coord(args, "a", 0));
    let ay = f64_field(args, "ay").or_else(|| point_coord(args, "a", 1));
    let bx = f64_field(args, "bx").or_else(|| point_coord(args, "b", 0));
    let by = f64_field(args, "by").or_else(|| point_coord(args, "b", 1));
    let cx = f64_field(args, "cx").or_else(|| point_coord(args, "c", 0));
    let cy = f64_field(args, "cy").or_else(|| point_coord(args, "c", 1));
    let (ax, ay, bx, by, cx, cy) = match (ax, ay, bx, by, cx, cy) {
        (Some(ax), Some(ay), Some(bx), Some(by), Some(cx), Some(cy)) => (ax, ay, bx, by, cx, cy),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "orientation_2 needs ax,ay,bx,by,cx,cy",
            ))
        }
    };
    let cross = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
    let (sign, label) = if cross > 0.0 {
        (1i64, "counter_clockwise")
    } else if cross < 0.0 {
        (-1, "clockwise")
    } else {
        (0, "collinear")
    };
    Ok(receipt(
        "ComputationalGeometry.orientation_2",
        [
            ("sign", Value::I64(sign)),
            ("orientation", Value::String(label.into())),
        ],
    ))
}

fn distance_2d(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let ax = f64_field(args, "ax").or_else(|| point_coord(args, "a", 0));
    let ay = f64_field(args, "ay").or_else(|| point_coord(args, "a", 1));
    let bx = f64_field(args, "bx").or_else(|| point_coord(args, "b", 0));
    let by = f64_field(args, "by").or_else(|| point_coord(args, "b", 1));
    let (ax, ay, bx, by) = match (ax, ay, bx, by) {
        (Some(ax), Some(ay), Some(bx), Some(by)) => (ax, ay, bx, by),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "distance_2d needs ax,ay,bx,by",
            ))
        }
    };
    let d = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
    Ok(receipt(
        "ComputationalGeometry.distance_2d",
        [("distance", Value::F64(d))],
    ))
}

fn convex_hull_2(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let pts = points_xy(args).ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "convex_hull_2 needs points: [[x,y],…] or flat [x0,y0,…]",
        )
    })?;
    if pts.len() < 3 {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "convex_hull_2 needs ≥ 3 points",
        ));
    }
    let hull = monotone_chain(&pts);
    let indices: Vec<Value> = hull.iter().map(|&i| Value::U64(i as u64)).collect();
    Ok(receipt(
        "ComputationalGeometry.convex_hull_2",
        [
            ("indices", Value::List(indices)),
            ("vertex_count", Value::U64(hull.len() as u64)),
        ],
    ))
}

fn monotone_chain(pts: &[(f64, f64)]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..pts.len()).collect();
    order.sort_by(|&i, &j| {
        pts[i]
            .0
            .partial_cmp(&pts[j].0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                pts[i]
                    .1
                    .partial_cmp(&pts[j].1)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });
    let cross = |o: usize, a: usize, b: usize| {
        let (ox, oy) = pts[o];
        let (ax, ay) = pts[a];
        let (bx, by) = pts[b];
        (ax - ox) * (by - oy) - (ay - oy) * (bx - ox)
    };
    let mut lower = Vec::new();
    for &i in &order {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], i) <= 0.0 {
            lower.pop();
        }
        lower.push(i);
    }
    let mut upper = Vec::new();
    for &i in order.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], i) <= 0.0 {
            upper.pop();
        }
        upper.push(i);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn receipt<const N: usize>(id: &str, fields: [(&str, Value); N]) -> Value {
    let mut m = BTreeMap::new();
    m.insert("id".into(), Value::String(id.into()));
    m.insert("honesty".into(), Value::String("local".into()));
    m.insert("evaluated".into(), Value::Bool(true));
    for (k, v) in fields {
        m.insert(k.into(), v);
    }
    Value::Record(m)
}

fn point_coord(args: &Value, key: &str, idx: usize) -> Option<f64> {
    let Value::Record(map) = args else {
        return None;
    };
    match map.get(key)? {
        Value::List(xs) if xs.len() > idx => match &xs[idx] {
            Value::F64(n) => Some(*n),
            Value::I64(n) => Some(*n as f64),
            Value::U64(n) => Some(*n as f64),
            _ => None,
        },
        _ => None,
    }
}

fn points_xy(args: &Value) -> Option<Vec<(f64, f64)>> {
    let Value::Record(map) = args else {
        return None;
    };
    let Value::List(xs) = map.get("points")? else {
        return None;
    };
    if xs.is_empty() {
        return None;
    }
    // Nested [[x,y],…]
    if matches!(xs[0], Value::List(_)) {
        let mut out = Vec::new();
        for p in xs {
            let Value::List(xy) = p else {
                return None;
            };
            if xy.len() < 2 {
                return None;
            }
            let x = match &xy[0] {
                Value::F64(n) => *n,
                Value::I64(n) => *n as f64,
                Value::U64(n) => *n as f64,
                _ => return None,
            };
            let y = match &xy[1] {
                Value::F64(n) => *n,
                Value::I64(n) => *n as f64,
                Value::U64(n) => *n as f64,
                _ => return None,
            };
            out.push((x, y));
        }
        return Some(out);
    }
    // Flat [x0,y0,…]
    if xs.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < xs.len() {
        let x = match &xs[i] {
            Value::F64(n) => *n,
            Value::I64(n) => *n as f64,
            Value::U64(n) => *n as f64,
            _ => return None,
        };
        let y = match &xs[i + 1] {
            Value::F64(n) => *n,
            Value::I64(n) => *n as f64,
            Value::U64(n) => *n as f64,
            _ => return None,
        };
        out.push((x, y));
        i += 2;
    }
    Some(out)
}

fn f64_field(args: &Value, key: &str) -> Option<f64> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::F64(n)) => Some(*n),
            Some(Value::I64(n)) => Some(*n as f64),
            Some(Value::U64(n)) => Some(*n as f64),
            _ => None,
        },
        _ => None,
    }
}

fn f64_list(args: &Value, key: &str) -> Option<Vec<f64>> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::List(items)) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        Value::F64(n) => out.push(*n),
                        Value::I64(n) => out.push(*n as f64),
                        Value::U64(n) => out.push(*n as f64),
                        _ => return None,
                    }
                }
                Some(out)
            }
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_and_hull() {
        let mut rec = BTreeMap::new();
        rec.insert("ax".into(), Value::F64(0.0));
        rec.insert("ay".into(), Value::F64(0.0));
        rec.insert("bx".into(), Value::F64(1.0));
        rec.insert("by".into(), Value::F64(0.0));
        rec.insert("cx".into(), Value::F64(0.0));
        rec.insert("cy".into(), Value::F64(1.0));
        let v = orientation_2(&Value::Record(rec), Span { start: 0, end: 0 }).unwrap();
        let Value::Record(m) = v else { panic!() };
        assert_eq!(m.get("sign"), Some(&Value::I64(1)));

        let mut h = BTreeMap::new();
        h.insert(
            "points".into(),
            Value::List(vec![
                Value::List(vec![Value::F64(0.0), Value::F64(0.0)]),
                Value::List(vec![Value::F64(1.0), Value::F64(0.0)]),
                Value::List(vec![Value::F64(0.5), Value::F64(0.5)]),
                Value::List(vec![Value::F64(1.0), Value::F64(1.0)]),
                Value::List(vec![Value::F64(0.0), Value::F64(1.0)]),
            ]),
        );
        let hv = convex_hull_2(&Value::Record(h), Span { start: 0, end: 0 }).unwrap();
        let Value::Record(hm) = hv else { panic!() };
        match hm.get("vertex_count") {
            Some(Value::U64(n)) => assert!(*n >= 4),
            _ => panic!(),
        }
    }

    #[test]
    fn framingham_is_e300() {
        let err = try_invoke(
            "ClinicalRisk.framingham",
            &Value::Null,
            Span { start: 0, end: 0 },
        )
        .unwrap()
        .unwrap_err();
        assert_eq!(err.code, DiagCode::E300);
        assert_eq!(err.civics_code(), "E0403");
    }
}
