//! Layout, Viewport Virtualization, Splitters, Collision-Aware Anchoring & Hit-Testing
//! (`layout.*` and `ui.*`) for VibeScript v0.02.

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

use super::ui_controls::{
    extract_f64, extract_i64, extract_list, extract_str, opt_f64, opt_i64, opt_str,
};

/// Dispatch a `layout.*` or layout-related `ui.*` function call.
pub fn call_layout_virtual(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "layout.virtual_slice" => {
            let total_items = extract_i64(args, 0, "layout.virtual_slice", span)?.max(0);
            let row_height = extract_f64(args, 1, "layout.virtual_slice", span)?;
            if row_height <= 0.0 {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "layout.virtual_slice: row_height must be > 0",
                ));
            }
            let scroll_top = extract_f64(args, 2, "layout.virtual_slice", span)?.max(0.0);
            let viewport_h = extract_f64(args, 3, "layout.virtual_slice", span)?.max(0.0);
            let overscan = opt_i64(args, 4, 2).max(0);

            let total_height = (total_items as f64) * row_height;
            let (start_idx, end_idx, visible_count, offset_y) = if total_items == 0 {
                (0, 0, 0, 0.0)
            } else {
                let first_vis = ((scroll_top / row_height).floor() as i64).clamp(0, total_items);
                let vis_rows = ((viewport_h / row_height).ceil() as i64).max(0);
                let s = (first_vis - overscan).clamp(0, total_items);
                let e = (first_vis + vis_rows + overscan).clamp(s, total_items);
                (s, e, e - s, (s as f64) * row_height)
            };

            let mut rec = BTreeMap::new();
            rec.insert("start_idx".into(), Value::I64(start_idx));
            rec.insert("end_idx".into(), Value::I64(end_idx));
            rec.insert("visible_count".into(), Value::I64(visible_count));
            rec.insert("offset_y".into(), Value::F64(offset_y));
            rec.insert("total_height".into(), Value::F64(total_height));
            Ok(Some(Value::Record(rec)))
        }
        "layout.solve_flex" => {
            let items = extract_list(args, 0, "layout.solve_flex", span)?;
            let viewport_w = extract_f64(args, 1, "layout.solve_flex", span)?.max(0.0);
            let viewport_h = extract_f64(args, 2, "layout.solve_flex", span)?.max(0.0);
            let direction = opt_str(args, 3, "row");
            let gap = opt_f64(args, 4, 0.0).max(0.0);
            let is_col = matches!(direction.as_str(), "column" | "col" | "vertical");
            if !is_col && !matches!(direction.as_str(), "row" | "horizontal") {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!("layout.solve_flex: unsupported direction '{direction}'"),
                ));
            }

            let n = items.len();
            let total_gap = if n > 1 { gap * (n - 1) as f64 } else { 0.0 };
            let main_cap = if is_col { viewport_h } else { viewport_w };
            let cross_cap = if is_col { viewport_w } else { viewport_h };

            let mut fixed_sum = 0.0;
            let mut flex_sum = 0.0;
            for item in &items {
                if let Value::Record(m) = item {
                    let main_sz = item_dim(m, if is_col { "h" } else { "w" });
                    if main_sz > 0.0 {
                        fixed_sum += main_sz;
                    } else {
                        let fl = m
                            .get("flex")
                            .and_then(Value::as_f64)
                            .unwrap_or(1.0)
                            .max(0.0);
                        flex_sum += if fl > 0.0 { fl } else { 1.0 };
                    }
                }
            }
            let remaining = (main_cap - fixed_sum - total_gap).max(0.0);
            let mut cursor = 0.0;
            let mut out = Vec::with_capacity(n);
            for item in items {
                if let Value::Record(mut m) = item {
                    let raw_w = item_dim(&m, "w");
                    let raw_h = item_dim(&m, "h");
                    let fl = m
                        .get("flex")
                        .and_then(Value::as_f64)
                        .unwrap_or(1.0)
                        .max(1.0);
                    let (x, y, w, h) = if is_col {
                        let h = if raw_h > 0.0 {
                            raw_h
                        } else if flex_sum > 0.0 {
                            remaining * (fl / flex_sum)
                        } else {
                            0.0
                        };
                        let w = if raw_w > 0.0 { raw_w } else { cross_cap };
                        let y = cursor;
                        cursor += h + gap;
                        (0.0, y, w, h)
                    } else {
                        let w = if raw_w > 0.0 {
                            raw_w
                        } else if flex_sum > 0.0 {
                            remaining * (fl / flex_sum)
                        } else {
                            0.0
                        };
                        let h = if raw_h > 0.0 { raw_h } else { cross_cap };
                        let x = cursor;
                        cursor += w + gap;
                        (x, 0.0, w, h)
                    };
                    m.insert("x".into(), Value::F64(x));
                    m.insert("y".into(), Value::F64(y));
                    m.insert("w".into(), Value::F64(w));
                    m.insert("h".into(), Value::F64(h));
                    out.push(Value::Record(m));
                }
            }
            Ok(Some(Value::List(out)))
        }
        "ui.splitter" => {
            let id = extract_str(args, 0, "ui.splitter", span)?;
            let orientation = extract_str(args, 1, "ui.splitter", span)?;
            if !matches!(
                orientation.as_str(),
                "horizontal" | "vertical" | "row" | "column"
            ) {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!("ui.splitter: invalid orientation '{orientation}'"),
                ));
            }
            let ratio = extract_f64(args, 2, "ui.splitter", span)?.clamp(0.0, 1.0);
            let min_px = extract_f64(args, 3, "ui.splitter", span)?.max(0.0);
            let max_px = extract_f64(args, 4, "ui.splitter", span)?.max(min_px);
            let total_px = opt_f64(args, 5, 1000.0_f64.max(max_px)).max(min_px);

            let primary_px = (ratio * total_px).clamp(min_px, max_px.min(total_px).max(min_px));
            let secondary_px = (total_px - primary_px).max(0.0);
            let clamped_ratio = if total_px > 0.0 {
                primary_px / total_px
            } else {
                0.0
            };

            let mut rec = BTreeMap::new();
            rec.insert("id".into(), Value::String(id));
            rec.insert("kind".into(), Value::String("splitter".into()));
            rec.insert("orientation".into(), Value::String(orientation));
            rec.insert("ratio".into(), Value::F64(clamped_ratio));
            rec.insert("primary_px".into(), Value::F64(primary_px));
            rec.insert("secondary_px".into(), Value::F64(secondary_px));
            Ok(Some(Value::Record(rec)))
        }
        "ui.anchor_position" => eval_anchor_position(args, span),
        "ui.hit_test" => {
            let items = match args.first() {
                Some(Value::List(xs)) => xs.clone(),
                Some(Value::Record(doc)) => match doc.get("items") {
                    Some(Value::List(xs)) => xs.clone(),
                    _ => Vec::new(),
                },
                _ => return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "ui.hit_test(items, x, y): argument #0 must be a List or document Record",
                )),
            };
            let x = extract_f64(args, 1, "ui.hit_test", span)?;
            let y = extract_f64(args, 2, "ui.hit_test", span)?;

            for item in items.iter().rev() {
                if let Value::Record(m) = item {
                    let disabled = m.get("disabled") == Some(&Value::Bool(true))
                        || m.get("hidden") == Some(&Value::Bool(true))
                        || m.get("inert") == Some(&Value::Bool(true));
                    if disabled {
                        continue;
                    }
                    let ix = m.get("x").and_then(Value::as_f64).unwrap_or(0.0);
                    let iy = m.get("y").and_then(Value::as_f64).unwrap_or(0.0);
                    let iw = item_dim(m, "w");
                    let ih = item_dim(m, "h");
                    if iw > 0.0 && ih > 0.0 && x >= ix && x <= ix + iw && y >= iy && y <= iy + ih {
                        if let Some(Value::String(id)) = m.get("id") {
                            return Ok(Some(Value::String(id.clone())));
                        }
                    }
                }
            }
            Ok(Some(Value::String(String::new())))
        }
        _ => Ok(None),
    }
}

fn eval_anchor_position(args: &[Value], span: Span) -> Result<Option<Value>, Diagnostic> {
    let (ax, ay, aw, ah, ow, oh, vw, vh, preferred_side) =
        if let Some(Value::Record(r)) = args.first() {
            let ax = r.get("x").and_then(Value::as_f64).unwrap_or(0.0);
            let ay = r.get("y").and_then(Value::as_f64).unwrap_or(0.0);
            let aw = item_dim(r, "w");
            let ah = item_dim(r, "h");
            let ow = extract_f64(args, 1, "ui.anchor_position", span)?.max(0.0);
            let oh = extract_f64(args, 2, "ui.anchor_position", span)?.max(0.0);
            let vw = extract_f64(args, 3, "ui.anchor_position", span)?.max(0.0);
            let vh = extract_f64(args, 4, "ui.anchor_position", span)?.max(0.0);
            let side = opt_str(args, 5, "bottom");
            (ax, ay, aw, ah, ow, oh, vw, vh, side)
        } else {
            let ax = extract_f64(args, 0, "ui.anchor_position", span)?;
            let ay = extract_f64(args, 1, "ui.anchor_position", span)?;
            let aw = extract_f64(args, 2, "ui.anchor_position", span)?.max(0.0);
            let ah = extract_f64(args, 3, "ui.anchor_position", span)?.max(0.0);
            let ow = extract_f64(args, 4, "ui.anchor_position", span)?.max(0.0);
            let oh = extract_f64(args, 5, "ui.anchor_position", span)?.max(0.0);
            let vw = extract_f64(args, 6, "ui.anchor_position", span)?.max(0.0);
            let vh = extract_f64(args, 7, "ui.anchor_position", span)?.max(0.0);
            let side = opt_str(args, 8, "bottom");
            (ax, ay, aw, ah, ow, oh, vw, vh, side)
        };

    let (mut x, mut y, resolved_side, flipped) = match preferred_side.as_str() {
        "bottom" => {
            if ay + ah + oh > vh && ay - oh >= 0.0 {
                (ax, ay - oh, "top", true)
            } else {
                (ax, ay + ah, "bottom", false)
            }
        }
        "top" => {
            if ay - oh < 0.0 && ay + ah + oh <= vh {
                (ax, ay + ah, "bottom", true)
            } else {
                (ax, ay - oh, "top", false)
            }
        }
        "right" => {
            if ax + aw + ow > vw && ax - ow >= 0.0 {
                (ax - ow, ay, "left", true)
            } else {
                (ax + aw, ay, "right", false)
            }
        }
        "left" => {
            if ax - ow < 0.0 && ax + aw + ow <= vw {
                (ax + aw, ay, "right", true)
            } else {
                (ax - ow, ay, "left", false)
            }
        }
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("ui.anchor_position: invalid preferred_side '{preferred_side}'"),
            ))
        }
    };

    x = x.clamp(0.0, (vw - ow).max(0.0));
    y = y.clamp(0.0, (vh - oh).max(0.0));

    let mut rec = BTreeMap::new();
    rec.insert("x".into(), Value::F64(x));
    rec.insert("y".into(), Value::F64(y));
    rec.insert("resolved_side".into(), Value::String(resolved_side.into()));
    rec.insert("flipped".into(), Value::Bool(flipped));
    Ok(Some(Value::Record(rec)))
}

fn item_dim(m: &BTreeMap<String, Value>, key: &str) -> f64 {
    let alt = if key == "w" { "width" } else { "height" };
    m.get(key)
        .or_else(|| m.get(alt))
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}
