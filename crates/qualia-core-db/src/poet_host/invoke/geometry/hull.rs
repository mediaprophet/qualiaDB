//! 2-D convex hull — `specialized_libs::computational_geometry`.

use super::super::args;
use crate::specialized_libs::computational_geometry::{convex_hull_2, Point2};
use vibe::{Diagnostic, Span, Value};

pub fn hull2(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let rows = args::list(args_v)
        .or_else(|| args::rec(args_v, "points").and_then(args::list))
        .ok_or_else(|| args::bad(span, "convex_hull_2 needs [[x,y], ...]"))?;
    let mut points = Vec::with_capacity(rows.len());
    for row in rows {
        let xy = args::f64s(row).ok_or_else(|| args::bad(span, "point must be [x, y]"))?;
        if xy.len() < 2 {
            return Err(args::bad(span, "point must be [x, y]"));
        }
        points.push(Point2::new(xy[0], xy[1]));
    }
    if points.is_empty() {
        return Ok(Value::List(Vec::new()));
    }
    let mut scratch = vec![0u32; points.len().saturating_mul(3)];
    let mut out = vec![Point2::new(0.0, 0.0); points.len()];
    let n = convex_hull_2(&points, &mut scratch, &mut out)
        .map_err(|e| args::bad(span, format!("hull: {e:?}")))?;
    Ok(Value::List(
        out[..n]
            .iter()
            .map(|p| args::f64_list_value([p.x, p.y]))
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_corners() {
        let pts = Value::List(vec![
            args::f64_list_value(vec![0.0, 0.0]),
            args::f64_list_value(vec![1.0, 0.0]),
            args::f64_list_value(vec![1.0, 1.0]),
            args::f64_list_value(vec![0.0, 1.0]),
            args::f64_list_value(vec![0.5, 0.5]),
        ]);
        match hull2(&pts, Span { start: 0, end: 0 }).unwrap() {
            Value::List(xs) => assert_eq!(xs.len(), 4),
            other => panic!("{other:?}"),
        }
    }
}
