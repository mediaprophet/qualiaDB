//! `Render.*` SVG invoke wrappers — path, shapes, field visualization, bezier curves.
//!
//! These produce SVG text from structured Vibe values. SVG paths can be wired
//! through computational geometry libs for bezier/nurbs curve evaluation.

use super::super::args;
use crate::specialized_libs::computational_geometry::parametric_cad::bezier_eval;
use crate::specialized_libs::computational_geometry::Point3;
use poet_vibe::{Diagnostic, Span, Value};

/// Format an SVG `<path>` element from a list of [x, y] points.
///
/// Args:
/// - `points`: list of [x, y] pairs
/// - `stroke`: stroke color (default "black")
/// - `stroke_width`: stroke width (default 1)
/// - `fill`: fill color (default "none")
/// - `closed`: if true, close the path with Z
///
/// Returns: `{ svg: string, d: string, point_count: u64 }`
pub fn svg_path(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let points_flat = args::rec_f64_list(args_v, "points").ok_or_else(|| {
        args::bad(span, "svg_path needs { points: [[x, y], ...] }")
    })?;
    if points_flat.len() < 4 || points_flat.len() % 2 != 0 {
        return Err(args::bad(span, "points must be a flat list of x,y pairs (≥ 2 points)"));
    }
    let stroke = args::rec_str(args_v, "stroke").unwrap_or("black");
    let stroke_width = args::rec_f64(args_v, "stroke_width").unwrap_or(1.0);
    let fill = args::rec_str(args_v, "fill").unwrap_or("none");
    let closed = args::rec_bool(args_v, "closed").unwrap_or(false);

    let n = points_flat.len() / 2;
    let mut d = format!("M {} {}", fmt_num(points_flat[0]), fmt_num(points_flat[1]));
    for i in 1..n {
        d.push_str(&format!(" L {} {}", fmt_num(points_flat[i * 2]), fmt_num(points_flat[i * 2 + 1])));
    }
    if closed {
        d.push_str(" Z");
    }

    let svg = format!(
        r#"<path d="{d}" stroke="{stroke}" stroke-width="{stroke_width}" fill="{fill}"/>"#
    );

    Ok(args::record([
        ("svg", Value::String(svg)),
        ("d", Value::String(d)),
        ("point_count", Value::U64(n as u64)),
    ]))
}

/// `Render.svg_circle` — generate an SVG `<circle>` element.
///
/// Args: `cx`, `cy`, `r`, `stroke`, `stroke_width`, `fill`
pub fn svg_circle(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let cx = args::rec_f64(args_v, "cx").ok_or_else(|| args::bad(span, "svg_circle needs { cx, cy, r }"))?;
    let cy = args::rec_f64(args_v, "cy").unwrap_or(0.0);
    let r = args::rec_f64(args_v, "r").unwrap_or(1.0);
    let stroke = args::rec_str(args_v, "stroke").unwrap_or("black");
    let stroke_width = args::rec_f64(args_v, "stroke_width").unwrap_or(1.0);
    let fill = args::rec_str(args_v, "fill").unwrap_or("none");
    let svg = format!(
        r#"<circle cx="{}" cy="{}" r="{}" stroke="{stroke}" stroke-width="{stroke_width}" fill="{fill}"/>"#,
        fmt_num(cx), fmt_num(cy), fmt_num(r)
    );
    Ok(args::record([("svg", Value::String(svg))]))
}

/// `Render.svg_rect` — generate an SVG `<rect>` element.
///
/// Args: `x`, `y`, `width`, `height`, `rx`, `stroke`, `stroke_width`, `fill`
pub fn svg_rect(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args_v, "x").ok_or_else(|| args::bad(span, "svg_rect needs { x, y, width, height }"))?;
    let y = args::rec_f64(args_v, "y").unwrap_or(0.0);
    let width = args::rec_f64(args_v, "width").unwrap_or(1.0);
    let height = args::rec_f64(args_v, "height").unwrap_or(1.0);
    let rx = args::rec_f64(args_v, "rx");
    let stroke = args::rec_str(args_v, "stroke").unwrap_or("black");
    let stroke_width = args::rec_f64(args_v, "stroke_width").unwrap_or(1.0);
    let fill = args::rec_str(args_v, "fill").unwrap_or("none");
    let rx_attr = rx.map(|v| format!(r#" rx="{}""#, fmt_num(v))).unwrap_or_default();
    let svg = format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}"{rx_attr} stroke="{stroke}" stroke-width="{stroke_width}" fill="{fill}"/>"#,
        fmt_num(x), fmt_num(y), fmt_num(width), fmt_num(height)
    );
    Ok(args::record([("svg", Value::String(svg))]))
}

/// `Render.svg_line` — generate an SVG `<line>` element.
///
/// Args: `x1`, `y1`, `x2`, `y2`, `stroke`, `stroke_width`
pub fn svg_line(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x1 = args::rec_f64(args_v, "x1").ok_or_else(|| args::bad(span, "svg_line needs { x1, y1, x2, y2 }"))?;
    let y1 = args::rec_f64(args_v, "y1").unwrap_or(0.0);
    let x2 = args::rec_f64(args_v, "x2").unwrap_or(0.0);
    let y2 = args::rec_f64(args_v, "y2").unwrap_or(0.0);
    let stroke = args::rec_str(args_v, "stroke").unwrap_or("black");
    let stroke_width = args::rec_f64(args_v, "stroke_width").unwrap_or(1.0);
    let svg = format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{stroke}" stroke-width="{stroke_width}"/>"#,
        fmt_num(x1), fmt_num(y1), fmt_num(x2), fmt_num(y2)
    );
    Ok(args::record([("svg", Value::String(svg))]))
}

/// `Render.svg_bezier` — evaluate a Bezier curve and generate an SVG path.
///
/// Uses the computational geometry library's `bezier_eval` for accurate
/// de Casteljau evaluation of control points.
///
/// Args:
/// - `control_points`: flat list of [x, y, z, ...] (3D points, z ignored for SVG)
/// - `segments`: number of segments to sample (default 32)
/// - `stroke`, `stroke_width`, `fill`: SVG styling
///
/// Returns: `{ svg: string, d: string, segments: u64 }`
pub fn svg_bezier(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let cp_flat = args::rec_f64_list(args_v, "control_points").ok_or_else(|| {
        args::bad(span, "svg_bezier needs { control_points: [x,y,z, ...] }")
    })?;
    if cp_flat.len() < 6 || cp_flat.len() % 3 != 0 {
        return Err(args::bad(span, "control_points must be a flat list of x,y,z triples (≥ 2 points)"));
    }
    let segments = args::rec_u64(args_v, "segments").unwrap_or(32) as usize;
    let stroke = args::rec_str(args_v, "stroke").unwrap_or("black");
    let stroke_width = args::rec_f64(args_v, "stroke_width").unwrap_or(1.0);
    let fill = args::rec_str(args_v, "fill").unwrap_or("none");

    let n_cp = cp_flat.len() / 3;
    let control: Vec<Point3> = (0..n_cp)
        .map(|i| Point3::new(cp_flat[i * 3], cp_flat[i * 3 + 1], cp_flat[i * 3 + 2]))
        .collect();

    let mut d = String::with_capacity(segments * 16);
    for i in 0..=segments {
        let t = i as f64 / segments as f64;
        let pt = bezier_eval(&control, t).map_err(|e| args::bad(span, format!("bezier_eval: {e:?}")))?;
        if i == 0 {
            d.push_str(&format!("M {} {}", fmt_num(pt.x), fmt_num(pt.y)));
        } else {
            d.push_str(&format!(" L {} {}", fmt_num(pt.x), fmt_num(pt.y)));
        }
    }

    let svg = format!(
        r#"<path d="{d}" stroke="{stroke}" stroke-width="{stroke_width}" fill="{fill}"/>"#
    );

    Ok(args::record([
        ("svg", Value::String(svg)),
        ("d", Value::String(d)),
        ("segments", Value::U64(segments as u64)),
    ]))
}

/// `Render.svg_field` — generate SVG elements from a 2D field grid.
///
/// Takes a flat grid of amplitudes + dimensions and renders circles sized
/// by amplitude, colored by phase (via HSL hue mapping).
///
/// Args:
/// - `amplitudes`: flat list of f64 (nx × ny)
/// - `nx`, `ny`: grid dimensions
/// - `cell_size`: pixel size per grid cell
/// - `max_radius`: maximum circle radius (default = cell_size / 2)
///
/// Returns: `{ svg: string, element_count: u64 }`
pub fn svg_field(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let amplitudes = args::rec_f64_list(args_v, "amplitudes").ok_or_else(|| {
        args::bad(span, "svg_field needs { amplitudes: [f64], nx: u64, ny: u64 }")
    })?;
    let nx = args::rec_u64(args_v, "nx").unwrap_or(1) as usize;
    let ny = args::rec_u64(args_v, "ny").unwrap_or(1) as usize;
    if nx == 0 || ny == 0 {
        return Err(args::bad(span, "nx and ny must be non-zero"));
    }
    if amplitudes.len() < nx * ny {
        return Err(args::bad(span, format!(
            "amplitudes length {} < nx*ny = {}", amplitudes.len(), nx * ny
        )));
    }
    let cell_size = args::rec_f64(args_v, "cell_size").unwrap_or(20.0);
    let max_radius = args::rec_f64(args_v, "max_radius").unwrap_or(cell_size / 2.0);
    let phases = args::rec_f64_list(args_v, "phases").unwrap_or_default();

    let max_amp = amplitudes.iter().cloned().fold(0.0f64, f64::max).max(1e-12);

    let mut svg = String::with_capacity(nx * ny * 80);
    let mut count = 0u64;

    for iy in 0..ny {
        for ix in 0..nx {
            let idx = iy * nx + ix;
            let amp = amplitudes[idx];
            let r = (amp / max_amp * max_radius).max(0.5);
            let cx = (ix as f64 + 0.5) * cell_size;
            let cy = (iy as f64 + 0.5) * cell_size;

            // Color by phase if available, else by amplitude.
            let hue = if idx < phases.len() {
                ((phases[idx] / std::f64::consts::TAU * 360.0) % 360.0 + 360.0) % 360.0
            } else {
                (amp / max_amp * 240.0).min(240.0)
            };
            let color = format!("hsl({hue:.0}, 70%, 50%)");

            svg.push_str(&format!(
                r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="none"/>"#,
                fmt_num(cx), fmt_num(cy), fmt_num(r), color
            ));
            count += 1;
        }
    }

    Ok(args::record([
        ("svg", Value::String(svg)),
        ("element_count", Value::U64(count)),
    ]))
}

/// Format a number for SVG output — integers without decimal, floats with minimal precision.
fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{}", n as i64)
    } else {
        format!("{n:.3}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert((*k).into(), v.clone());
        }
        Value::Record(m)
    }

    fn f64_list(xs: &[f64]) -> Value {
        Value::List(xs.iter().map(|x| Value::F64(*x)).collect())
    }

    #[test]
    fn svg_path_basic() {
        let args = rec(&[
            ("points", f64_list(&[0.0, 0.0, 10.0, 20.0, 30.0, 40.0])),
            ("stroke", Value::String("red".into())),
        ]);
        let r = svg_path(&args, poet_vibe::Span::new(0, 0)).unwrap();
        if let Value::Record(m) = r {
            let svg = m.get("svg").and_then(args::as_str).unwrap();
            assert!(svg.contains("<path"));
            assert!(svg.contains("M 0 0"));
            assert!(svg.contains("L 10 20"));
            assert!(svg.contains("L 30 40"));
            assert!(svg.contains(r#"stroke="red""#));
            let n = m.get("point_count").and_then(args::as_u64).unwrap();
            assert_eq!(n, 3);
        }
    }

    #[test]
    fn svg_path_closed() {
        let args = rec(&[
            ("points", f64_list(&[0.0, 0.0, 10.0, 0.0, 10.0, 10.0])),
            ("closed", Value::Bool(true)),
        ]);
        let r = svg_path(&args, poet_vibe::Span::new(0, 0)).unwrap();
        if let Value::Record(m) = r {
            let d = m.get("d").and_then(args::as_str).unwrap();
            assert!(d.ends_with(" Z"));
        }
    }

    #[test]
    fn svg_circle_basic() {
        let args = rec(&[
            ("cx", Value::F64(50.0)),
            ("cy", Value::F64(50.0)),
            ("r", Value::F64(10.0)),
            ("fill", Value::String("blue".into())),
        ]);
        let r = svg_circle(&args, poet_vibe::Span::new(0, 0)).unwrap();
        if let Value::Record(m) = r {
            let svg = m.get("svg").and_then(args::as_str).unwrap();
            assert!(svg.contains("<circle"));
            assert!(svg.contains(r#"cx="50""#));
            assert!(svg.contains(r#"r="10""#));
            assert!(svg.contains(r#"fill="blue""#));
        }
    }

    #[test]
    fn svg_rect_with_rounded_corners() {
        let args = rec(&[
            ("x", Value::F64(0.0)),
            ("y", Value::F64(0.0)),
            ("width", Value::F64(100.0)),
            ("height", Value::F64(50.0)),
            ("rx", Value::F64(5.0)),
        ]);
        let r = svg_rect(&args, poet_vibe::Span::new(0, 0)).unwrap();
        if let Value::Record(m) = r {
            let svg = m.get("svg").and_then(args::as_str).unwrap();
            assert!(svg.contains("<rect"));
            assert!(svg.contains(r#"rx="5""#));
        }
    }

    #[test]
    fn svg_line_basic() {
        let args = rec(&[
            ("x1", Value::F64(0.0)),
            ("y1", Value::F64(0.0)),
            ("x2", Value::F64(100.0)),
            ("y2", Value::F64(100.0)),
        ]);
        let r = svg_line(&args, poet_vibe::Span::new(0, 0)).unwrap();
        if let Value::Record(m) = r {
            let svg = m.get("svg").and_then(args::as_str).unwrap();
            assert!(svg.contains("<line"));
            assert!(svg.contains(r#"x1="0""#));
            assert!(svg.contains(r#"x2="100""#));
        }
    }

    #[test]
    fn svg_bezier_quadratic() {
        // 3 control points → quadratic bezier
        let args = rec(&[
            ("control_points", f64_list(&[0.0, 0.0, 0.0, 50.0, 100.0, 0.0, 100.0, 100.0, 0.0])),
            ("segments", Value::U64(16)),
        ]);
        let r = svg_bezier(&args, poet_vibe::Span::new(0, 0)).unwrap();
        if let Value::Record(m) = r {
            let d = m.get("d").and_then(args::as_str).unwrap();
            assert!(d.starts_with("M "));
            // Should have 17 points (0..=16)
            let l_count = d.matches(" L ").count();
            assert_eq!(l_count, 16);
            let segs = m.get("segments").and_then(args::as_u64).unwrap();
            assert_eq!(segs, 16);
        }
    }

    #[test]
    fn svg_field_grid() {
        let amps = vec![0.0, 0.5, 1.0, 0.5];
        let args = rec(&[
            ("amplitudes", f64_list(&amps)),
            ("nx", Value::U64(2)),
            ("ny", Value::U64(2)),
            ("cell_size", Value::F64(20.0)),
        ]);
        let r = svg_field(&args, poet_vibe::Span::new(0, 0)).unwrap();
        if let Value::Record(m) = r {
            let svg = m.get("svg").and_then(args::as_str).unwrap();
            assert!(svg.contains("<circle"));
            let count = m.get("element_count").and_then(args::as_u64).unwrap();
            assert_eq!(count, 4);
        }
    }

    #[test]
    fn svg_path_too_few_points_errors() {
        let args = rec(&[
            ("points", f64_list(&[0.0, 0.0])),
        ]);
        assert!(svg_path(&args, poet_vibe::Span::new(0, 0)).is_err());
    }

    #[test]
    fn svg_bezier_too_few_controls_errors() {
        let args = rec(&[
            ("control_points", f64_list(&[0.0, 0.0, 0.0])),
        ]);
        assert!(svg_bezier(&args, poet_vibe::Span::new(0, 0)).is_err());
    }
}
