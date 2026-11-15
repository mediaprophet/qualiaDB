//! Composite & Uplift UI Controls (`ui.*`) for VibeScript v0.02.
//!
//! Implements interval sliders, tag chips, fuzzy comboboxes, tri-state tree
//! selectors, transfer lists, segmented controls, steppers, ratings, virtualized
//! data grids, spectral color pickers, bezier ink pads, and OTP inputs.

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

use super::{
    extract_f64, extract_i64, extract_list, extract_str, opt_bool, opt_f64, opt_i64, opt_str,
};

/// Dispatch a composite uplift `ui.*` widget function call.
pub fn call_composite_control(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "ui.range_interval" => {
            let id = extract_str(args, 0, "ui.range_interval", span)?;
            let min = extract_f64(args, 1, "ui.range_interval", span)?;
            let max = extract_f64(args, 2, "ui.range_interval", span)?.max(min);
            let mut lo = extract_f64(args, 3, "ui.range_interval", span)?.clamp(min, max);
            let mut hi = extract_f64(args, 4, "ui.range_interval", span)?.clamp(min, max);
            if lo > hi {
                std::mem::swap(&mut lo, &mut hi);
            }
            let min_gap = opt_f64(args, 5, 0.0).max(0.0).min(max - min);
            if hi - lo < min_gap {
                hi = (lo + min_gap).min(max);
                lo = (hi - min_gap).max(min);
            }
            let scale = opt_str(args, 6, "linear");
            let ratio_lo = scale_ratio(lo, min, max, &scale, span)?;
            let ratio_hi = scale_ratio(hi, min, max, &scale, span)?;
            let mut rec = BTreeMap::new();
            rec.insert("id".into(), Value::String(id));
            rec.insert("kind".into(), Value::String("range-interval".into()));
            rec.insert("min".into(), Value::F64(min));
            rec.insert("max".into(), Value::F64(max));
            rec.insert("lo".into(), Value::F64(lo));
            rec.insert("hi".into(), Value::F64(hi));
            rec.insert("span".into(), Value::F64(hi - lo));
            rec.insert("ratio_lo".into(), Value::F64(ratio_lo));
            rec.insert("ratio_hi".into(), Value::F64(ratio_hi));
            rec.insert("scale".into(), Value::String(scale));
            Ok(Some(Value::Record(rec)))
        }
        "ui.tag_input" => {
            let id = extract_str(args, 0, "ui.tag_input", span)?;
            let existing = extract_list(args, 1, "ui.tag_input", span)?;
            let raw_entry = extract_str(args, 2, "ui.tag_input", span)?;
            let max_tags = opt_i64(args, 3, 64).max(1) as usize;
            let allow_duplicates = opt_bool(args, 4, false);

            let mut tags: Vec<String> = Vec::new();
            for item in existing {
                if let Value::String(s) = item {
                    let t = s.trim().to_string();
                    if !t.is_empty()
                        && (allow_duplicates || !tags.contains(&t))
                        && tags.len() < max_tags
                    {
                        tags.push(t);
                    }
                }
            }
            for token in raw_entry.split([',', ';', '\n']) {
                let t = token.trim().to_string();
                if !t.is_empty()
                    && (allow_duplicates || !tags.contains(&t))
                    && tags.len() < max_tags
                {
                    tags.push(t);
                }
            }
            let count = tags.len() as i64;
            let quota_reached = tags.len() >= max_tags;
            let mut rec = BTreeMap::new();
            rec.insert("id".into(), Value::String(id));
            rec.insert("kind".into(), Value::String("tag-input".into()));
            rec.insert(
                "tags".into(),
                Value::List(tags.into_iter().map(Value::String).collect()),
            );
            rec.insert("count".into(), Value::I64(count));
            rec.insert("quota_reached".into(), Value::Bool(quota_reached));
            Ok(Some(Value::Record(rec)))
        }
        "ui.combobox" => {
            let id = extract_str(args, 0, "ui.combobox", span)?;
            let query = extract_str(args, 1, "ui.combobox", span)?;
            let options = extract_list(args, 2, "ui.combobox", span)?;
            let max_visible = opt_i64(args, 3, 10).max(1) as usize;
            let q = query.trim().to_ascii_lowercase();

            let mut scored: Vec<(i64, usize, BTreeMap<String, Value>)> = Vec::new();
            for (idx, opt) in options.into_iter().enumerate() {
                if let Value::Record(mut map) = opt {
                    let s = score_option(&map, &q);
                    if s > 0 {
                        map.insert("score".into(), Value::I64(s));
                        scored.push((s, idx, map));
                    }
                }
            }
            scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
            let total_matches = scored.len() as i64;
            let matches: Vec<Value> = scored
                .into_iter()
                .take(max_visible)
                .map(|(_, _, m)| Value::Record(m))
                .collect();

            let mut rec = BTreeMap::new();
            rec.insert("id".into(), Value::String(id));
            rec.insert("kind".into(), Value::String("combobox".into()));
            rec.insert("query".into(), Value::String(query));
            rec.insert("matches".into(), Value::List(matches));
            rec.insert("total_matches".into(), Value::I64(total_matches));
            Ok(Some(Value::Record(rec)))
        }
        "ui.tree_select" => eval_tree_select(args, span),
        "ui.transfer_list" => {
            let id = extract_str(args, 0, "ui.transfer_list", span)?;
            let mut available = extract_list(args, 1, "ui.transfer_list", span)?;
            let mut assigned = extract_list(args, 2, "ui.transfer_list", span)?;
            let move_items = extract_list(args, 3, "ui.transfer_list", span)?;
            let direction = extract_str(args, 4, "ui.transfer_list", span)?;
            match direction.as_str() {
                "to_assigned" => {
                    available.retain(|x| !move_items.contains(x));
                    for item in move_items {
                        if !assigned.contains(&item) {
                            assigned.push(item);
                        }
                    }
                }
                "to_available" => {
                    assigned.retain(|x| !move_items.contains(x));
                    for item in move_items {
                        if !available.contains(&item) {
                            available.push(item);
                        }
                    }
                }
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("ui.transfer_list: invalid direction '{direction}'"),
                    ))
                }
            }
            let mut rec = BTreeMap::new();
            rec.insert("id".into(), Value::String(id));
            rec.insert("available".into(), Value::List(available));
            rec.insert("assigned".into(), Value::List(assigned));
            Ok(Some(Value::Record(rec)))
        }
        "ui.segmented_control" => {
            let id = extract_str(args, 0, "ui.segmented_control", span)?;
            let segments = extract_list(args, 1, "ui.segmented_control", span)?;
            let active = extract_str(args, 2, "ui.segmented_control", span)?;
            let mut rec = BTreeMap::new();
            rec.insert("id".into(), Value::String(id));
            rec.insert("kind".into(), Value::String("segmented-control".into()));
            rec.insert("segments".into(), Value::List(segments));
            rec.insert("active".into(), Value::String(active));
            Ok(Some(Value::Record(rec)))
        }
        "ui.stepper" => {
            let id = extract_str(args, 0, "ui.stepper", span)?;
            let steps = extract_list(args, 1, "ui.stepper", span)?;
            let max_idx = (steps.len() as i64 - 1).max(0);
            let current_index = extract_i64(args, 2, "ui.stepper", span)?.clamp(0, max_idx);
            let progress_pct = if steps.len() <= 1 {
                100.0
            } else {
                (current_index as f64 / max_idx as f64) * 100.0
            };
            let mut rec = BTreeMap::new();
            rec.insert("id".into(), Value::String(id));
            rec.insert("kind".into(), Value::String("stepper".into()));
            rec.insert("steps".into(), Value::List(steps.clone()));
            rec.insert("current_index".into(), Value::I64(current_index));
            rec.insert("progress_pct".into(), Value::F64(progress_pct));
            rec.insert("can_prev".into(), Value::Bool(current_index > 0));
            rec.insert(
                "can_next".into(),
                Value::Bool((current_index as usize) + 1 < steps.len()),
            );
            Ok(Some(Value::Record(rec)))
        }
        "ui.rating" => {
            let id = extract_str(args, 0, "ui.rating", span)?;
            let raw_val = extract_f64(args, 1, "ui.rating", span)?;
            let max_stars = opt_i64(args, 2, 5).max(1);
            let clamped = raw_val.clamp(0.0, max_stars as f64);
            let value = (clamped * 2.0).round() / 2.0;
            let mut rec = BTreeMap::new();
            rec.insert("id".into(), Value::String(id));
            rec.insert("kind".into(), Value::String("rating".into()));
            rec.insert("value".into(), Value::F64(value));
            rec.insert("max_stars".into(), Value::I64(max_stars));
            Ok(Some(Value::Record(rec)))
        }
        "ui.data_grid" => {
            let id = extract_str(args, 0, "ui.data_grid", span)?;
            let columns = extract_list(args, 1, "ui.data_grid", span)?;
            let rows = extract_list(args, 2, "ui.data_grid", span)?;
            let total_rows = rows.len() as i64;
            let start_row = opt_i64(args, 3, 0).max(0).min(total_rows);
            let row_count = opt_i64(args, 4, total_rows).max(0);
            let end = ((start_row + row_count) as usize).min(rows.len());
            let visible_rows = rows[start_row as usize..end].to_vec();
            let mut rec = BTreeMap::new();
            rec.insert("id".into(), Value::String(id));
            rec.insert("kind".into(), Value::String("data-grid".into()));
            rec.insert("column_count".into(), Value::I64(columns.len() as i64));
            rec.insert("total_rows".into(), Value::I64(total_rows));
            rec.insert("visible_rows".into(), Value::List(visible_rows));
            rec.insert("start_row".into(), Value::I64(start_row));
            Ok(Some(Value::Record(rec)))
        }
        "ui.spectral_picker" => eval_spectral_picker(args, span),
        "ui.ink_pad" => eval_ink_pad(args, span),
        "ui.otp_input" => eval_otp_input(args, span),
        _ => Ok(None),
    }
}

fn eval_tree_select(args: &[Value], span: Span) -> Result<Option<Value>, Diagnostic> {
    let id = extract_str(args, 0, "ui.tree_select", span)?;
    let nodes = extract_list(args, 1, "ui.tree_select", span)?;
    let selected_list = extract_list(args, 2, "ui.tree_select", span)?;
    let selected_ids: Vec<String> = selected_list
        .iter()
        .filter_map(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .collect();

    let mut all_ids: Vec<String> = Vec::new();
    let mut children_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in &nodes {
        if let Value::Record(m) = node {
            if let Some(Value::String(nid)) = m.get("id") {
                if !all_ids.contains(nid) {
                    all_ids.push(nid.clone());
                }
                if let Some(Value::String(pid)) = m.get("parent_id") {
                    if !pid.is_empty() {
                        children_map.entry(pid.clone()).or_default().push(nid.clone());
                    }
                }
                if let Some(Value::List(ch)) = m.get("children") {
                    for c in ch {
                        let cid_opt = match c {
                            Value::String(s) => Some(s.clone()),
                            Value::Record(cm) => cm.get("id").and_then(|v| match v {
                                Value::String(s) => Some(s.clone()),
                                _ => None,
                            }),
                            _ => None,
                        };
                        if let Some(cid) = cid_opt {
                            if !all_ids.contains(&cid) {
                                all_ids.push(cid.clone());
                            }
                            children_map.entry(nid.clone()).or_default().push(cid);
                        }
                    }
                }
            }
        }
    }

    let mut node_states = BTreeMap::new();
    for nid in &all_ids {
        let st = resolve_tree_state(nid, &children_map, &selected_ids, 0);
        node_states.insert(nid.clone(), Value::String(st.into()));
    }
    let mut rec = BTreeMap::new();
    rec.insert("id".into(), Value::String(id));
    rec.insert("kind".into(), Value::String("tree-select".into()));
    rec.insert("selected_ids".into(), Value::List(selected_list));
    rec.insert("node_states".into(), Value::Record(node_states));
    Ok(Some(Value::Record(rec)))
}

fn resolve_tree_state<'a>(
    nid: &str,
    children_map: &BTreeMap<String, Vec<String>>,
    selected: &[String],
    depth: usize,
) -> &'a str {
    if depth > 16 {
        return "unchecked";
    }
    if let Some(children) = children_map.get(nid) {
        if !children.is_empty() {
            let mut checked_cnt = 0usize;
            let mut indet_cnt = 0usize;
            for c in children {
                match resolve_tree_state(c, children_map, selected, depth + 1) {
                    "checked" => checked_cnt += 1,
                    "indeterminate" => indet_cnt += 1,
                    _ => {}
                }
            }
            return if checked_cnt == children.len() {
                "checked"
            } else if checked_cnt == 0 && indet_cnt == 0 && !selected.iter().any(|s| s == nid) {
                "unchecked"
            } else {
                "indeterminate"
            };
        }
    }
    if selected.iter().any(|s| s == nid) {
        "checked"
    } else {
        "unchecked"
    }
}

fn eval_spectral_picker(args: &[Value], span: Span) -> Result<Option<Value>, Diagnostic> {
    let id = extract_str(args, 0, "ui.spectral_picker", span)?;
    let alpha = extract_f64(args, 1, "ui.spectral_picker", span)?.clamp(0.0, 1.0);
    let mu_nm = extract_f64(args, 2, "ui.spectral_picker", span)?.clamp(380.0, 780.0);
    let sigma_nm = extract_f64(args, 3, "ui.spectral_picker", span)?.max(1.0);
    let (r, g, b) = wavelength_to_srgb(mu_nm);
    let ri = (r * 255.0).round().clamp(0.0, 255.0) as u8;
    let gi = (g * 255.0).round().clamp(0.0, 255.0) as u8;
    let bi = (b * 255.0).round().clamp(0.0, 255.0) as u8;
    let hex = format!("#{ri:02x}{gi:02x}{bi:02x}");
    let oklch_l = (0.2126 * r + 0.7152 * g + 0.0722 * b).cbrt();
    let oklch_c = ((r - g).powi(2) + (g - b).powi(2) + (b - r).powi(2)).sqrt() * 0.25;
    let oklch_h = ((780.0 - mu_nm) / 400.0 * 300.0).clamp(0.0, 360.0);
    let mut rec = BTreeMap::new();
    rec.insert("id".into(), Value::String(id));
    rec.insert("kind".into(), Value::String("spectral-picker".into()));
    rec.insert("alpha".into(), Value::F64(alpha));
    rec.insert("mu_nm".into(), Value::F64(mu_nm));
    rec.insert("sigma_nm".into(), Value::F64(sigma_nm));
    rec.insert("hex".into(), Value::String(hex));
    rec.insert("r".into(), Value::F64(r));
    rec.insert("g".into(), Value::F64(g));
    rec.insert("b".into(), Value::F64(b));
    rec.insert("oklch_l".into(), Value::F64(oklch_l));
    rec.insert("oklch_c".into(), Value::F64(oklch_c));
    rec.insert("oklch_h".into(), Value::F64(oklch_h));
    Ok(Some(Value::Record(rec)))
}

fn eval_ink_pad(args: &[Value], span: Span) -> Result<Option<Value>, Diagnostic> {
    let id = extract_str(args, 0, "ui.ink_pad", span)?;
    let points = extract_list(args, 1, "ui.ink_pad", span)?;
    let mut coords: Vec<(f64, f64, f64)> = Vec::new();
    for pt in points {
        if let Value::Record(m) = pt {
            let x = m.get("x").and_then(Value::as_f64).unwrap_or(0.0);
            let y = m.get("y").and_then(Value::as_f64).unwrap_or(0.0);
            let p = m.get("pressure").and_then(Value::as_f64).unwrap_or(0.5);
            coords.push((x, y, p));
        }
    }
    let point_count = coords.len() as i64;
    let (svg_path, mean_pressure) = if coords.is_empty() {
        (String::new(), 0.0)
    } else {
        let mut path = format!("M {} {}", coords[0].0, coords[0].1);
        for i in 1..coords.len() {
            let (x0, y0, _) = coords[i - 1];
            let (x1, y1, _) = coords[i];
            let mx = (x0 + x1) * 0.5;
            let my = (y0 + y1) * 0.5;
            path.push_str(&format!(" Q {x0} {y0} {mx} {my}"));
        }
        let sum_p: f64 = coords.iter().map(|c| c.2).sum();
        (path, sum_p / (coords.len() as f64))
    };
    let mut rec = BTreeMap::new();
    rec.insert("id".into(), Value::String(id));
    rec.insert("kind".into(), Value::String("ink-pad".into()));
    rec.insert("point_count".into(), Value::I64(point_count));
    rec.insert("svg_path".into(), Value::String(svg_path));
    rec.insert("mean_pressure".into(), Value::F64(mean_pressure));
    Ok(Some(Value::Record(rec)))
}

fn eval_otp_input(args: &[Value], span: Span) -> Result<Option<Value>, Diagnostic> {
    let id = extract_str(args, 0, "ui.otp_input", span)?;
    let digits = extract_i64(args, 1, "ui.otp_input", span)?.max(1);
    let raw_input = extract_str(args, 2, "ui.otp_input", span)?;
    let masked = opt_bool(args, 3, false);
    let value: String = raw_input
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(digits as usize)
        .collect();
    let display = if masked {
        "*".repeat(value.len())
    } else {
        value.clone()
    };
    let mut cells = Vec::with_capacity(digits as usize);
    let chars: Vec<char> = display.chars().collect();
    for i in 0..(digits as usize) {
        cells.push(Value::String(
            chars.get(i).map(|c| c.to_string()).unwrap_or_default(),
        ));
    }
    let complete = value.len() == digits as usize;
    let next_focus_index = (value.len() as i64).min(digits - 1);
    let mut rec = BTreeMap::new();
    rec.insert("id".into(), Value::String(id));
    rec.insert("kind".into(), Value::String("otp-input".into()));
    rec.insert("digits".into(), Value::I64(digits));
    rec.insert("value".into(), Value::String(value));
    rec.insert("display".into(), Value::String(display));
    rec.insert("cells".into(), Value::List(cells));
    rec.insert("complete".into(), Value::Bool(complete));
    rec.insert("next_focus_index".into(), Value::I64(next_focus_index));
    Ok(Some(Value::Record(rec)))
}

fn scale_ratio(v: f64, min: f64, max: f64, scale: &str, span: Span) -> Result<f64, Diagnostic> {
    let t = if (max - min).abs() < 1e-12 {
        0.0
    } else {
        ((v - min) / (max - min)).clamp(0.0, 1.0)
    };
    match scale {
        "linear" => Ok(t),
        "sqrt" => Ok(t.sqrt()),
        "logarithmic" | "decibel" => Ok((1.0 + 9.0 * t).log10().clamp(0.0, 1.0)),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("ui.range_interval: unsupported scale '{scale}'"),
        )),
    }
}

fn score_option(map: &BTreeMap<String, Value>, q: &str) -> i64 {
    if q.is_empty() {
        return 100;
    }
    let mut best = 0;
    for key in ["label", "value", "keywords"] {
        if let Some(Value::String(s)) = map.get(key) {
            let cand = s.to_ascii_lowercase();
            let s_val = if cand == q {
                100
            } else if cand.starts_with(q) {
                80
            } else if cand.contains(q) {
                60
            } else if is_subsequence(q, &cand) {
                40
            } else {
                0
            };
            if s_val > best {
                best = s_val;
            }
        }
    }
    best
}

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut it = haystack.chars();
    for nc in needle.chars() {
        if !it.any(|hc| hc == nc) {
            return false;
        }
    }
    true
}

fn wavelength_to_srgb(wl: f64) -> (f64, f64, f64) {
    let (r, g, b) = if wl < 440.0 {
        (-(wl - 440.0) / 60.0, 0.0, 1.0)
    } else if wl < 490.0 {
        (0.0, (wl - 440.0) / 50.0, 1.0)
    } else if wl < 510.0 {
        (0.0, 1.0, -(wl - 510.0) / 20.0)
    } else if wl < 580.0 {
        ((wl - 510.0) / 70.0, 1.0, 0.0)
    } else if wl < 645.0 {
        (1.0, -(wl - 645.0) / 65.0, 0.0)
    } else {
        (1.0, 0.0, 0.0)
    };
    (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}
