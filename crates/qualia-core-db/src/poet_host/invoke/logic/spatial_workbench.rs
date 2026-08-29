//! Bounded adapters for Allen/RCC8, Minkowski, heat, and manifold-logic panels.

use super::super::args;
use crate::modalities::interval_reasoning::{AllenRelation, TemporalAlgebra, TemporalInterval};
use crate::modalities::manifold_logic::{
    continuous_to_fact, integrate_abs, wave_eval, WaveCoord, MAX_MANIFOLD_POINTS,
};
use crate::modalities::spatio_temporal::{
    causally_connectable, evaluate_rcc8, evaluate_rcc8_points, heat_equation_step,
    minkowski_interval, spatial_index_query, Aabb, SpatialRegion,
};
use vibe::{Diagnostic, Span, Value};

const MAX_ITEMS: usize = 64;

pub fn compute(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    match args::rec_str(args_v, "mode") {
        Some("allen") => allen(args_v, span),
        Some("rcc8") => rcc8(args_v, span, false),
        Some("rcc8_points") => rcc8(args_v, span, true),
        Some("spatial_index") => spatial_index(args_v, span),
        Some("minkowski") => minkowski(args_v, span, false),
        Some("causally_connectable") => minkowski(args_v, span, true),
        Some("heat_equation") => heat(args_v, span),
        Some("manifold") => manifold(args_v, span),
        _ => Err(args::bad(
            span,
            "SpatialLogic.compute needs a supported `mode`",
        )),
    }
}

fn allen(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = interval(args_v, "a", span)?;
    let b = interval(args_v, "b", span)?;
    let relation = TemporalAlgebra::determine_relation(&a, &b);
    Ok(args::record([
        ("relation", Value::String(format!("{relation:?}"))),
        ("equals", Value::Bool(relation == AllenRelation::Equal)),
        ("overlaps", Value::Bool(a.overlaps(&b))),
        (
            "intersection_duration",
            match a.intersection(&b) {
                Some(overlap) => Value::I64(overlap.duration),
                None => Value::I64(0),
            },
        ),
    ]))
}

fn rcc8(args_v: &Value, span: Span, points_kernel: bool) -> Result<Value, Diagnostic> {
    let a_id = args::rec_u64(args_v, "a_id").unwrap_or(1);
    let b_id = args::rec_u64(args_v, "b_id").unwrap_or(2);
    let a = points(args_v, "a_points", span)?;
    let b = points(args_v, "b_points", span)?;
    let relation = if points_kernel {
        format!("{:?}", evaluate_rcc8_points(a_id, &a, b_id, &b))
    } else {
        format!(
            "{:?}",
            evaluate_rcc8(&SpatialRegion::new(a_id, a), &SpatialRegion::new(b_id, b),)
        )
    };
    Ok(args::record([
        ("relation", Value::String(relation)),
        (
            "kernel",
            Value::String(if points_kernel { "points" } else { "region" }.into()),
        ),
    ]))
}

fn spatial_index(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let query = aabb(args_v, "query", span)?;
    let boxes = args::rec_f64_list(args_v, "boxes")
        .ok_or_else(|| args::bad(span, "spatial index needs `boxes`"))?;
    if boxes.len() % 4 != 0 || boxes.len() / 4 > MAX_ITEMS {
        return Err(args::bad(
            span,
            "boxes must be 1..=64 flattened AABBs of four numbers",
        ));
    }
    let region_boxes: Vec<Aabb> = boxes
        .chunks_exact(4)
        .map(|chunk| (chunk[0], chunk[1], chunk[2], chunk[3]))
        .collect();
    let mut hits = [0usize; MAX_ITEMS];
    let n = spatial_index_query(query, &region_boxes, &mut hits);
    Ok(args::record([
        ("hits", Value::U64(n as u64)),
        (
            "indices",
            Value::List(
                hits[..n]
                    .iter()
                    .map(|index| Value::U64(*index as u64))
                    .collect(),
            ),
        ),
    ]))
}

fn minkowski(args_v: &Value, span: Span, causal: bool) -> Result<Value, Diagnostic> {
    let dt = need_f64(args_v, "dt", span)?;
    let dx = args::rec_f64(args_v, "dx").unwrap_or(0.0);
    let dy = args::rec_f64(args_v, "dy").unwrap_or(0.0);
    let dz = args::rec_f64(args_v, "dz").unwrap_or(0.0);
    let c = args::rec_f64(args_v, "c").unwrap_or(1.0);
    if ![dt, dx, dy, dz, c].iter().all(|value| value.is_finite()) || c <= 0.0 {
        return Err(args::bad(
            span,
            "Minkowski values must be finite with c > 0",
        ));
    }
    let interval = minkowski_interval(dt, dx, dy, dz, c);
    Ok(args::record([
        ("interval", Value::F64(interval)),
        (
            "class",
            Value::String(
                if interval < 0.0 {
                    "timelike"
                } else if interval == 0.0 {
                    "lightlike"
                } else {
                    "spacelike"
                }
                .into(),
            ),
        ),
        (
            "causally_connectable",
            Value::Bool(causally_connectable(dt, dx, dy, dz, c)),
        ),
        ("causal_query", Value::Bool(causal)),
    ]))
}

fn heat(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let u = args::rec_f64_list(args_v, "u")
        .ok_or_else(|| args::bad(span, "heat equation needs `u`"))?;
    if u.len() < 2 || u.len() > MAX_ITEMS || !u.iter().all(|value| value.is_finite()) {
        return Err(args::bad(span, "u must contain 2..=64 finite samples"));
    }
    let alpha = need_f64(args_v, "alpha", span)?;
    let dt = need_f64(args_v, "dt", span)?;
    let dx = need_f64(args_v, "dx", span)?;
    let mut out = vec![0.0; u.len()];
    if !heat_equation_step(&u, alpha, dt, dx, &mut out) {
        return Err(args::bad(span, "heat-equation step was rejected"));
    }
    Ok(args::record([
        ("next", args::f64_list_value(out)),
        ("r", Value::F64(alpha * dt / (dx * dx))),
        ("stable", Value::Bool(alpha * dt / (dx * dx) <= 0.5)),
    ]))
}

fn manifold(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    match args::rec_str(args_v, "operation").unwrap_or("continuous_to_fact") {
        "wave_eval" | "wave_val" => {
            let coord = WaveCoord {
                x: args::rec_f64(args_v, "x").unwrap_or(0.0),
                y: args::rec_f64(args_v, "y").unwrap_or(0.0),
                z: args::rec_f64(args_v, "z").unwrap_or(0.0),
                t: args::rec_f64(args_v, "t").unwrap_or(0.0),
                f: args::rec_f64(args_v, "f").unwrap_or(1.0),
                a: args::rec_f64(args_v, "a").unwrap_or(1.0),
                phi: args::rec_f64(args_v, "phi").unwrap_or(0.0),
            };
            if ![
                coord.x, coord.y, coord.z, coord.t, coord.f, coord.a, coord.phi,
            ]
            .iter()
            .all(|value| value.is_finite())
            {
                return Err(args::bad(span, "wave coordinates must be finite"));
            }
            Ok(args::record([("value", Value::F64(wave_eval(&coord)))]))
        }
        "integrate_abs" => {
            let samples = samples(args_v, span)?;
            Ok(args::record([(
                "integral",
                Value::F64(integrate_abs(&samples)),
            )]))
        }
        "continuous_to_fact" => {
            let samples = samples(args_v, span)?;
            let threshold = need_f64(args_v, "threshold", span)?;
            let fact_id = args::rec_u64(args_v, "fact_id").unwrap_or(1);
            Ok(args::record([
                ("integral", Value::F64(integrate_abs(&samples))),
                (
                    "fact_id",
                    match continuous_to_fact(&samples, threshold, fact_id) {
                        Some(id) => Value::U64(id),
                        None => Value::String("none".into()),
                    },
                ),
            ]))
        }
        other => Err(args::bad(
            span,
            format!("unknown manifold operation `{other}`"),
        )),
    }
}

fn interval(args_v: &Value, key: &str, span: Span) -> Result<TemporalInterval, Diagnostic> {
    let values = args::rec(args_v, key)
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, format!("allen mode needs `{key}`: [start,end]")))?;
    let start = values
        .first()
        .and_then(args::as_i64)
        .ok_or_else(|| args::bad(span, format!("`{key}` needs integer bounds")))?;
    let end = values
        .get(1)
        .and_then(args::as_i64)
        .ok_or_else(|| args::bad(span, format!("`{key}` needs integer bounds")))?;
    if values.len() != 2 || start > end {
        return Err(args::bad(
            span,
            format!("`{key}` must be [start,end] with start <= end"),
        ));
    }
    Ok(TemporalInterval::new(0, start, end))
}

fn points(args_v: &Value, key: &str, span: Span) -> Result<Vec<(f64, f64)>, Diagnostic> {
    let values = args::rec_f64_list(args_v, key)
        .ok_or_else(|| args::bad(span, format!("RCC8 needs `{key}`")))?;
    if values.len() < 6 || values.len() % 2 != 0 || values.len() / 2 > MAX_MANIFOLD_POINTS {
        return Err(args::bad(
            span,
            format!("`{key}` must contain 3..=64 finite coordinate pairs"),
        ));
    }
    if !values.iter().all(|value| value.is_finite()) {
        return Err(args::bad(span, format!("`{key}` values must be finite")));
    }
    Ok(values
        .chunks_exact(2)
        .map(|chunk| (chunk[0], chunk[1]))
        .collect())
}

fn aabb(args_v: &Value, key: &str, span: Span) -> Result<Aabb, Diagnostic> {
    let values = args::rec_f64_list(args_v, key)
        .ok_or_else(|| args::bad(span, format!("spatial index needs `{key}`")))?;
    if values.len() != 4 || !values.iter().all(|value| value.is_finite()) {
        return Err(args::bad(
            span,
            format!("`{key}` must be [min_x, min_y, max_x, max_y]"),
        ));
    }
    Ok((values[0], values[1], values[2], values[3]))
}

fn samples(args_v: &Value, span: Span) -> Result<Vec<f64>, Diagnostic> {
    let values = args::rec_f64_list(args_v, "samples")
        .ok_or_else(|| args::bad(span, "manifold evaluation needs `samples`"))?;
    if values.is_empty()
        || values.len() > MAX_ITEMS
        || !values.iter().all(|value| value.is_finite())
    {
        return Err(args::bad(span, "samples must contain 1..=64 finite values"));
    }
    Ok(values)
}

fn need_f64(args_v: &Value, key: &str, span: Span) -> Result<f64, Diagnostic> {
    let value =
        args::rec_f64(args_v, key).ok_or_else(|| args::bad(span, format!("needs `{key}`")))?;
    value
        .is_finite()
        .then_some(value)
        .ok_or_else(|| args::bad(span, format!("`{key}` must be finite")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
        args::record(pairs)
    }

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn allen_and_rcc8_use_native_kernels() {
        let relation = compute(
            &rec([
                ("mode", Value::String("allen".into())),
                ("a", Value::List(vec![Value::I64(0), Value::I64(10)])),
                ("b", Value::List(vec![Value::I64(5), Value::I64(15)])),
            ]),
            span(),
        )
        .unwrap();
        let Value::Record(result) = relation else {
            panic!("expected record")
        };
        assert_eq!(
            result.get("relation"),
            Some(&Value::String("Overlaps".into()))
        );

        let rcc8 = compute(
            &rec([
                ("mode", Value::String("rcc8_points".into())),
                (
                    "a_points",
                    args::f64_list_value([0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]),
                ),
                (
                    "b_points",
                    args::f64_list_value([3.0, 3.0, 4.0, 3.0, 4.0, 4.0, 3.0, 4.0]),
                ),
            ]),
            span(),
        )
        .unwrap();
        let Value::Record(result) = rcc8 else {
            panic!("expected record")
        };
        assert_eq!(
            result.get("relation"),
            Some(&Value::String("Disconnected".into()))
        );
    }

    #[test]
    fn manifold_continuous_to_fact_and_heat_are_native() {
        let fact = compute(
            &rec([
                ("mode", Value::String("manifold".into())),
                ("operation", Value::String("continuous_to_fact".into())),
                ("samples", args::f64_list_value([0.4, 0.8, 1.2])),
                ("threshold", Value::F64(0.5)),
                ("fact_id", Value::U64(7)),
            ]),
            span(),
        )
        .unwrap();
        let Value::Record(result) = fact else {
            panic!("expected record")
        };
        assert_eq!(result.get("fact_id"), Some(&Value::U64(7)));

        let step = compute(
            &rec([
                ("mode", Value::String("heat_equation".into())),
                ("u", args::f64_list_value([0.0, 1.0, 0.0])),
                ("alpha", Value::F64(0.1)),
                ("dt", Value::F64(0.1)),
                ("dx", Value::F64(1.0)),
            ]),
            span(),
        )
        .unwrap();
        let Value::Record(result) = step else {
            panic!("expected record")
        };
        assert_eq!(result.get("stable"), Some(&Value::Bool(true)));
    }

    #[test]
    fn unknown_mode_fails_closed() {
        assert!(compute(&rec([("mode", Value::String("mock".into()))]), span()).is_err());
    }
}
