//! Stepping (`input.step`), IME-safe declarative masking (`input.apply_mask`),
//! and text selection range operations (`selection.set_range`, `selection.replace_range`).

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::{Instant, Quantity, Value};

use super::{extract_i64, extract_opt_str, extract_str};

/// `input.step(current, step_delta, min_opt, max_opt, step_size_opt)`
pub fn eval_input_step(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let current = args.first().ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "input.step: missing current value argument",
        )
    })?;
    let step_delta = extract_i64(args, 1, "input.step", span)?;
    let min_opt = args.get(2);
    let max_opt = args.get(3);
    let step_size_opt = args.get(4);

    match current {
        Value::I64(cur) => {
            let step_size = step_size_opt
                .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
                .unwrap_or(1);
            let mut next = cur.saturating_add(step_delta.saturating_mul(step_size));
            if let Some(lo) =
                min_opt.and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
            {
                next = next.max(lo);
            }
            if let Some(hi) =
                max_opt.and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
            {
                next = next.min(hi);
            }
            Ok(Value::I64(next))
        }
        Value::U64(cur) => {
            let step_size = step_size_opt
                .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
                .unwrap_or(1);
            let mut next = (*cur as i64).saturating_add(step_delta.saturating_mul(step_size));
            if let Some(lo) =
                min_opt.and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
            {
                next = next.max(lo);
            }
            if let Some(hi) =
                max_opt.and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
            {
                next = next.min(hi);
            }
            Ok(Value::I64(next))
        }
        Value::F64(cur) => {
            let step_size = step_size_opt.and_then(Value::as_f64).unwrap_or(1.0);
            let mut next = *cur + (step_delta as f64) * step_size;
            if let Some(lo) = min_opt.and_then(Value::as_f64) {
                next = next.max(lo);
            }
            if let Some(hi) = max_opt.and_then(Value::as_f64) {
                next = next.min(hi);
            }
            Ok(Value::F64(next))
        }
        Value::Quantity(q) => {
            let step_size = step_size_opt.and_then(Value::as_f64).unwrap_or(1.0);
            let mut next = q.value + (step_delta as f64) * step_size;
            if let Some(lo) = min_opt.and_then(Value::as_f64) {
                next = next.max(lo);
            }
            if let Some(hi) = max_opt.and_then(Value::as_f64) {
                next = next.min(hi);
            }
            Ok(Value::Quantity(Quantity {
                value: next,
                unit: q.unit.clone(),
            }))
        }
        Value::Instant(inst) => {
            let step_size = match step_size_opt {
                Some(Value::Duration(d)) => d.secs,
                Some(other) => other
                    .as_i64()
                    .or_else(|| other.as_f64().map(|f| f as i64))
                    .unwrap_or(1),
                None => 1,
            };
            let mut next_secs = inst
                .secs
                .saturating_add(step_delta.saturating_mul(step_size));
            let min_secs = match min_opt {
                Some(Value::Instant(i)) => Some(i.secs),
                Some(other) => other.as_i64().or_else(|| other.as_f64().map(|f| f as i64)),
                None => None,
            };
            let max_secs = match max_opt {
                Some(Value::Instant(i)) => Some(i.secs),
                Some(other) => other.as_i64().or_else(|| other.as_f64().map(|f| f as i64)),
                None => None,
            };
            if let Some(lo) = min_secs {
                next_secs = next_secs.max(lo);
            }
            if let Some(hi) = max_secs {
                next_secs = next_secs.min(hi);
            }
            Ok(Value::Instant(Instant {
                secs: next_secs,
                ..inst.clone()
            }))
        }
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "input.step: current value must be I64, F64, Quantity, or Instant",
        )),
    }
}

fn is_mask_token(m: char) -> bool {
    matches!(m, '9' | 'a' | '*' | 'h' | '#')
}

fn matches_mask_token(c: char, token: char) -> bool {
    match token {
        '9' => c.is_ascii_digit(),
        'a' => c.is_ascii_alphabetic(),
        '*' => c.is_ascii_alphanumeric(),
        'h' => c.is_ascii_hexdigit(),
        '#' => c.is_ascii_digit() || c == '+' || c == '-',
        _ => false,
    }
}

/// `input.apply_mask(raw, mask)`
pub fn eval_apply_mask(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let raw = extract_str(args, 0, "input.apply_mask(raw, mask)", span)?;
    let mask = extract_str(args, 1, "input.apply_mask(raw, mask)", span)?;

    let raw_chars: Vec<char> = raw.chars().collect();
    let mask_chars: Vec<char> = mask.chars().collect();

    let total_tokens = mask_chars.iter().filter(|&&m| is_mask_token(m)).count();
    let mut filled_tokens = 0usize;
    let mut raw_idx = 0usize;
    let mut formatted = String::with_capacity(mask.len());
    let mut raw_clean = String::with_capacity(total_tokens);

    for (m_idx, &m) in mask_chars.iter().enumerate() {
        if is_mask_token(m) {
            let mut matched = None;
            while raw_idx < raw_chars.len() {
                let candidate = raw_chars[raw_idx];
                raw_idx += 1;
                if matches_mask_token(candidate, m) {
                    matched = Some(candidate);
                    break;
                }
            }
            if let Some(ch) = matched {
                formatted.push(ch);
                raw_clean.push(ch);
                filled_tokens += 1;
            } else {
                break;
            }
        } else {
            // Fixed literal in mask
            if raw_idx < raw_chars.len() && raw_chars[raw_idx] == m {
                raw_idx += 1;
                formatted.push(m);
            } else {
                // Insert literal automatically if there are remaining raw characters that can match
                // upcoming mask tokens, or if all tokens have already been filled (trailing literal).
                let next_token = mask_chars[m_idx + 1..]
                    .iter()
                    .copied()
                    .find(|&tk| is_mask_token(tk));
                let has_upcoming_match = match next_token {
                    Some(tk) => raw_chars[raw_idx..]
                        .iter()
                        .any(|&c| matches_mask_token(c, tk)),
                    None => filled_tokens == total_tokens && total_tokens > 0,
                };
                if has_upcoming_match {
                    formatted.push(m);
                } else if next_token.is_some() {
                    break;
                }
            }
        }
    }

    let complete = filled_tokens == total_tokens;
    let valid = complete;
    let cursor_pos = formatted.chars().count() as i64;

    let mut rec = BTreeMap::new();
    rec.insert("formatted".to_string(), Value::String(formatted));
    rec.insert("raw_clean".to_string(), Value::String(raw_clean));
    rec.insert("cursor_pos".to_string(), Value::I64(cursor_pos));
    rec.insert("complete".to_string(), Value::Bool(complete));
    rec.insert("valid".to_string(), Value::Bool(valid));
    Ok(Value::Record(rec))
}

/// `selection.set_range(target_id, start, end, direction_opt)`
pub fn eval_selection_set_range(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let target_id = extract_str(args, 0, "selection.set_range", span)?;
    let start_raw = extract_i64(args, 1, "selection.set_range", span)?;
    let end_raw = extract_i64(args, 2, "selection.set_range", span)?;
    let dir = extract_opt_str(args, 3).unwrap_or_else(|| "none".to_string());

    if !matches!(dir.as_str(), "forward" | "backward" | "none") {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!(
                "selection.set_range: direction must be 'forward', 'backward', or 'none', got '{dir}'"
            ),
        ));
    }

    let start = start_raw.max(0);
    let end = end_raw.max(start);

    let mut rec = BTreeMap::new();
    rec.insert(
        "op".to_string(),
        Value::String("selection.set_range".to_string()),
    );
    rec.insert("target_id".to_string(), Value::String(target_id));
    rec.insert("selection_start".to_string(), Value::I64(start));
    rec.insert("selection_end".to_string(), Value::I64(end));
    rec.insert("selection_direction".to_string(), Value::String(dir));
    Ok(Value::Record(rec))
}

/// `selection.replace_range(text, replacement, start, end, select_mode_opt)`
pub fn eval_selection_replace_range(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let text = extract_str(args, 0, "selection.replace_range", span)?;
    let replacement = extract_str(args, 1, "selection.replace_range", span)?;
    let start_raw = extract_i64(args, 2, "selection.replace_range", span)?;
    let end_raw = extract_i64(args, 3, "selection.replace_range", span)?;
    let mode = extract_opt_str(args, 4).unwrap_or_else(|| "preserve".to_string());

    let chars: Vec<char> = text.chars().collect();
    let len = chars.len() as i64;
    let start = start_raw.clamp(0, len);
    let end = end_raw.clamp(start, len);
    let rep_len = replacement.chars().count() as i64;

    let (sel_start, sel_end) = match mode.as_str() {
        "select" | "preserve" => (start, start + rep_len),
        "start" => (start, start),
        "end" => (start + rep_len, start + rep_len),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!(
                    "selection.replace_range: select_mode must be 'select', 'start', 'end', or 'preserve', got '{mode}'"
                ),
            ))
        }
    };

    let mut new_text = String::with_capacity(text.len() + replacement.len());
    for &c in &chars[..start as usize] {
        new_text.push(c);
    }
    new_text.push_str(&replacement);
    for &c in &chars[end as usize..] {
        new_text.push(c);
    }

    let mut rec = BTreeMap::new();
    rec.insert("text".to_string(), Value::String(new_text));
    rec.insert("selection_start".to_string(), Value::I64(sel_start));
    rec.insert("selection_end".to_string(), Value::I64(sel_end));
    rec.insert("select_mode".to_string(), Value::String(mode));
    Ok(Value::Record(rec))
}
