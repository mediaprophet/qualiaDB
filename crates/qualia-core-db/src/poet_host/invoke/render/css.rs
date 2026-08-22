//! `Render.*` CSS invoke wrappers — `@keyframes`, color, transform generation.
//!
//! These produce CSS text from structured Vibe values, enabling reactive cells
//! to drive visual output. All functions are deterministic string generation
//! from numeric inputs — no GPU, no side effects.

use super::super::args;
use crate::render::spectral_kernel::{emf_to_linear_rgb, linear_rgb_to_display};
use vibe::{Diagnostic, Span, Value};

/// `Render.css_animation` — generate `@keyframes` CSS from a value curve.
///
/// Args:
/// - `name`: keyframes name (string)
/// - `property`: CSS property name (e.g. "opacity", "transform", "color")
/// - `keyframes`: list of { time: f64, value: f64 } records
/// - `unit`: optional CSS unit suffix (e.g. "px", "%", "deg")
///
/// Returns: `{ css: string, name: string, duration: f64 }`
pub fn css_animation(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let name = args::rec_str(args_v, "name").ok_or_else(|| {
        args::bad(
            span,
            "css_animation needs { name: string, property: string, keyframes: [...] }",
        )
    })?;
    let property = args::rec_str(args_v, "property").unwrap_or("opacity");
    let unit = args::rec_str(args_v, "unit").unwrap_or("");
    let keyframes_v = args::rec(args_v, "keyframes").ok_or_else(|| {
        args::bad(
            span,
            "css_animation needs keyframes: [{ time: f64, value: f64 }, ...]",
        )
    })?;

    let keyframes = args::list(keyframes_v)
        .ok_or_else(|| args::bad(span, "keyframes must be a list of { time, value } records"))?;

    if keyframes.is_empty() {
        return Err(args::bad(span, "keyframes must be non-empty"));
    }

    let mut css = String::with_capacity(256);
    css.push_str("@keyframes ");
    css.push_str(name);
    css.push_str(" {\n");

    let mut max_time = 0.0f64;

    for kf in keyframes {
        let time = args::rec_f64(kf, "time")
            .ok_or_else(|| args::bad(span, "each keyframe needs { time: f64, value: f64 }"))?;
        let value = args::rec_f64(kf, "value")
            .ok_or_else(|| args::bad(span, "each keyframe needs { time: f64, value: f64 }"))?;
        max_time = max_time.max(time);
        let pct = if max_time > 0.0 {
            (time / max_time * 100.0).round() as u64
        } else {
            0
        };
        css.push_str(&format!("  {pct}% {{ {property}: {value}{unit}; }}\n"));
    }
    css.push_str("}\n");

    Ok(args::record([
        ("css", Value::String(css)),
        ("name", Value::String(name.to_string())),
        ("duration", Value::F64(max_time)),
    ]))
}

/// `Render.css_color` — EMF parameters → CSS `rgb(r,g,b)` string.
///
/// Wraps the spectral pipeline: EMF `[α, μ, σ]` → SPD → XYZ → linear sRGB
/// → display sRGB → `rgb(r,g,b)`.
///
/// Args:
/// - `alpha`: EMF amplitude
/// - `mu`: EMF spectral centre (0–1 maps to 380–780 nm)
/// - `sigma`: EMF spectral width
///
/// Returns: `{ css: string, r: u64, g: u64, b: u64 }`
pub fn css_color(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let alpha = args::rec_f64(args_v, "alpha")
        .ok_or_else(|| args::bad(span, "css_color needs { alpha: f64, mu: f64, sigma: f64 }"))?;
    let mu = args::rec_f64(args_v, "mu").unwrap_or(0.0);
    let sigma = args::rec_f64(args_v, "sigma").unwrap_or(0.5);
    let lin = emf_to_linear_rgb(alpha as f32, mu as f32, sigma as f32);
    let (r, g, b) = linear_rgb_to_display(&lin);
    Ok(args::record([
        ("css", Value::String(format!("rgb({r},{g},{b})"))),
        ("r", Value::U64(r as u64)),
        ("g", Value::U64(g as u64)),
        ("b", Value::U64(b as u64)),
    ]))
}

/// `Render.css_transform` — generate a CSS `transform` string from components.
///
/// Args:
/// - `translate`: optional [tx, ty] or [tx, ty, tz]
/// - `rotate`: optional angle in degrees
/// - `scale`: optional [sx, sy] or uniform scale
/// - `skew`: optional [ax, ay] in degrees
///
/// Returns: `{ transform: string }`
pub fn css_transform(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut parts: Vec<String> = Vec::new();

    if let Some(translate) = args::rec_f64_list(args_v, "translate") {
        match translate.len() {
            2 => parts.push(format!("translate({}px, {}px)", translate[0], translate[1])),
            3 => parts.push(format!(
                "translate3d({}px, {}px, {}px)",
                translate[0], translate[1], translate[2]
            )),
            _ => {
                return Err(args::bad(
                    span,
                    "translate must be [tx, ty] or [tx, ty, tz]",
                ))
            }
        }
    }

    if let Some(rotate) = args::rec_f64(args_v, "rotate") {
        parts.push(format!("rotate({rotate}deg)"));
    }

    if let Some(scale) = args::rec_f64_list(args_v, "scale") {
        match scale.len() {
            1 => parts.push(format!("scale({})", scale[0])),
            2 => parts.push(format!("scale({}, {})", scale[0], scale[1])),
            _ => return Err(args::bad(span, "scale must be [s] or [sx, sy]")),
        }
    }

    if let Some(skew) = args::rec_f64_list(args_v, "skew") {
        match skew.len() {
            1 => parts.push(format!("skew({}deg)", skew[0])),
            2 => parts.push(format!("skew({}deg, {}deg)", skew[0], skew[1])),
            _ => return Err(args::bad(span, "skew must be [ax] or [ax, ay]")),
        }
    }

    if parts.is_empty() {
        return Err(args::bad(
            span,
            "css_transform needs at least one of: translate, rotate, scale, skew",
        ));
    }

    let transform = parts.join(" ");
    Ok(args::record([("transform", Value::String(transform))]))
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

    fn kf(time: f64, value: f64) -> Value {
        rec(&[("time", Value::F64(time)), ("value", Value::F64(value))])
    }

    #[test]
    fn css_animation_generates_keyframes() {
        let args = rec(&[
            ("name", Value::String("fade".into())),
            ("property", Value::String("opacity".into())),
            ("keyframes", Value::List(vec![kf(0.0, 0.0), kf(1.0, 1.0)])),
        ]);
        let r = css_animation(&args, vibe::Span::new(0, 0)).unwrap();
        if let Value::Record(m) = r {
            let css = m.get("css").and_then(args::as_str).unwrap();
            assert!(css.contains("@keyframes fade"));
            assert!(css.contains("0%"));
            assert!(css.contains("100%"));
            assert!(css.contains("opacity: 0"));
            assert!(css.contains("opacity: 1"));
        }
    }

    #[test]
    fn css_animation_with_unit() {
        let args = rec(&[
            ("name", Value::String("move".into())),
            ("property", Value::String("transform".into())),
            ("unit", Value::String("px".into())),
            ("keyframes", Value::List(vec![kf(0.0, 0.0), kf(2.0, 100.0)])),
        ]);
        let r = css_animation(&args, vibe::Span::new(0, 0)).unwrap();
        if let Value::Record(m) = r {
            let css = m.get("css").and_then(args::as_str).unwrap();
            assert!(css.contains("transform: 0px"));
            assert!(css.contains("transform: 100px"));
            let dur = m.get("duration").and_then(args::as_f64).unwrap();
            assert_eq!(dur, 2.0);
        }
    }

    #[test]
    fn css_color_returns_rgb_string() {
        let args = rec(&[
            ("alpha", Value::F64(1.0)),
            ("mu", Value::F64(0.0)),
            ("sigma", Value::F64(0.5)),
        ]);
        let r = css_color(&args, vibe::Span::new(0, 0)).unwrap();
        if let Value::Record(m) = r {
            let css = m.get("css").and_then(args::as_str).unwrap();
            assert!(css.starts_with("rgb("));
            assert!(css.ends_with(')'));
            let r_val = m.get("r").and_then(args::as_u64).unwrap();
            let g_val = m.get("g").and_then(args::as_u64).unwrap();
            let b_val = m.get("b").and_then(args::as_u64).unwrap();
            assert!(r_val <= 255 && g_val <= 255 && b_val <= 255);
        }
    }

    #[test]
    fn css_transform_combined() {
        let args = rec(&[
            (
                "translate",
                Value::List(vec![Value::F64(10.0), Value::F64(20.0)]),
            ),
            ("rotate", Value::F64(45.0)),
            ("scale", Value::List(vec![Value::F64(1.5)])),
        ]);
        let r = css_transform(&args, vibe::Span::new(0, 0)).unwrap();
        if let Value::Record(m) = r {
            let t = m.get("transform").and_then(args::as_str).unwrap();
            assert!(t.contains("translate(10px, 20px)"));
            assert!(t.contains("rotate(45deg)"));
            assert!(t.contains("scale(1.5)"));
        }
    }

    #[test]
    fn css_animation_empty_keyframes_errors() {
        let args = rec(&[
            ("name", Value::String("x".into())),
            ("property", Value::String("opacity".into())),
            ("keyframes", Value::List(vec![])),
        ]);
        assert!(css_animation(&args, vibe::Span::new(0, 0)).is_err());
    }

    #[test]
    fn css_transform_no_args_errors() {
        let args = rec(&[]);
        assert!(css_transform(&args, vibe::Span::new(0, 0)).is_err());
    }
}
