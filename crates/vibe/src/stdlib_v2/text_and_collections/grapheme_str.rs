//! Unicode UAX #29 Extended Grapheme Cluster String Operations (`str.*`).
//!
//! Eliminates JavaScript's UTF-16 surrogate-pair corruption (`"🚀".length === 2`,
//! `"👨‍👩‍👧‍👦".length === 11`, and `$&`/`$'` replacement traps) by operating on extended
//! grapheme clusters, explicit scalar code points, and UTF-8 bytes.

use crate::error::Diagnostic;
use crate::span::Span;
use crate::value::Value;

use super::{
    extract_i64, extract_list, extract_opt_i64, extract_str, resolve_element_index,
    resolve_slice_bound,
};

/// Segment a UTF-8 string into Unicode UAX #29 Extended Grapheme Clusters.
///
/// Keeps together:
/// - CRLF (`\r\n`)
/// - Base characters + combining marks, variation selectors, and enclosing marks
/// - Emoji skin-tone modifiers (`\u{1F3FB}..=\u{1F3FF}`)
/// - Zero-Width Joiner (`\u{200D}`) emoji sequences (e.g. `"👨‍👩‍👧‍👦"`, `"👩‍🚀"`)
/// - Regional Indicator flag pairs (`\u{1F1E6}..=\u{1F1FF}` in pairs, e.g. `"🇦🇺"`)
pub fn split_graphemes(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut clusters = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let first = chars[i];
        let mut cluster = String::new();
        cluster.push(first);
        i += 1;

        if first == '\r' {
            if i < chars.len() && chars[i] == '\n' {
                cluster.push('\n');
                i += 1;
            }
            clusters.push(cluster);
            continue;
        }
        if first == '\n' {
            clusters.push(cluster);
            continue;
        }

        let mut ri_count = usize::from(is_regional_indicator(first));

        while i < chars.len() {
            let c = chars[i];
            if c == '\r' || c == '\n' {
                break;
            }
            if is_grapheme_extend_or_modifier(c) {
                cluster.push(c);
                i += 1;
                continue;
            }
            if c == '\u{200D}' {
                cluster.push(c);
                i += 1;
                if i < chars.len() && chars[i] != '\r' && chars[i] != '\n' {
                    cluster.push(chars[i]);
                    i += 1;
                }
                continue;
            }
            if ri_count == 1 && is_regional_indicator(c) {
                cluster.push(c);
                ri_count += 1;
                i += 1;
                continue;
            }
            break;
        }

        clusters.push(cluster);
    }

    clusters
}

fn is_regional_indicator(c: char) -> bool {
    matches!(c as u32, 0x1F1E6..=0x1F1FF)
}

fn is_grapheme_extend_or_modifier(c: char) -> bool {
    let cp = c as u32;
    matches!(
        cp,
        0x0300..=0x036F
            | 0x0483..=0x0489
            | 0x0591..=0x05BD
            | 0x0610..=0x061A
            | 0x064B..=0x065F
            | 0x0670
            | 0x06D6..=0x06ED
            | 0x0900..=0x0903
            | 0x093A..=0x094F
            | 0x0E31
            | 0x0E34..=0x0E3A
            | 0x0E47..=0x0E4E
            | 0x1AB0..=0x1AFF
            | 0x1DC0..=0x1DFF
            | 0x200C
            | 0x20D0..=0x20FF
            | 0xFE00..=0xFE0F
            | 0xFE20..=0xFE2F
            | 0x1F3FB..=0x1F3FF
            | 0xE0020..=0xE007F
            | 0xE0100..=0xE01EF
    )
}

/// Dispatch `str.*` bindings.
pub(crate) fn call_grapheme_str(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "str.graphemes" => {
            let s = extract_str(args, 0, "str.graphemes(s)", span)?;
            let list = split_graphemes(&s)
                .into_iter()
                .map(Value::String)
                .collect();
            Ok(Some(Value::List(list)))
        }
        "str.grapheme_len" => {
            let s = extract_str(args, 0, "str.grapheme_len(s)", span)?;
            let count = split_graphemes(&s).len() as i64;
            Ok(Some(Value::I64(count)))
        }
        "str.grapheme_at" => {
            let s = extract_str(args, 0, "str.grapheme_at(s, idx)", span)?;
            let idx = extract_i64(args, 1, "str.grapheme_at(s, idx)", span)?;
            let graphemes = split_graphemes(&s);
            let out = resolve_element_index(idx, graphemes.len())
                .map(|i| Value::String(graphemes[i].clone()))
                .unwrap_or(Value::Null);
            Ok(Some(out))
        }
        "str.grapheme_slice" => {
            let s = extract_str(args, 0, "str.grapheme_slice(s, start, end_opt)", span)?;
            let start = extract_i64(args, 1, "str.grapheme_slice(s, start, end_opt)", span)?;
            let graphemes = split_graphemes(&s);
            let len = graphemes.len();
            let s_idx = resolve_slice_bound(start, len);
            let e_idx = extract_opt_i64(args, 2)
                .map(|e| resolve_slice_bound(e, len))
                .unwrap_or(len);
            let sliced = if s_idx < e_idx {
                graphemes[s_idx..e_idx].concat()
            } else {
                String::new()
            };
            Ok(Some(Value::String(sliced)))
        }
        "str.code_points" => {
            let s = extract_str(args, 0, "str.code_points(s)", span)?;
            let cps = s.chars().map(|c| Value::I64(c as u32 as i64)).collect();
            Ok(Some(Value::List(cps)))
        }
        "str.bytes" => {
            let s = extract_str(args, 0, "str.bytes(s)", span)?;
            let bs = s.as_bytes().iter().map(|&b| Value::I64(b as i64)).collect();
            Ok(Some(Value::List(bs)))
        }
        "str.trim" => {
            let s = extract_str(args, 0, "str.trim(s)", span)?;
            Ok(Some(Value::String(s.trim().to_string())))
        }
        "str.trim_start" => {
            let s = extract_str(args, 0, "str.trim_start(s)", span)?;
            Ok(Some(Value::String(s.trim_start().to_string())))
        }
        "str.trim_end" => {
            let s = extract_str(args, 0, "str.trim_end(s)", span)?;
            Ok(Some(Value::String(s.trim_end().to_string())))
        }
        "str.pad_start" => {
            let s = extract_str(args, 0, "str.pad_start(s, target_len, fill)", span)?;
            let target_len =
                extract_i64(args, 1, "str.pad_start(s, target_len, fill)", span)?.max(0) as usize;
            let fill = args
                .get(2)
                .and_then(|v| match v {
                    Value::String(f) if !f.is_empty() => Some(f.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| " ".to_string());
            Ok(Some(Value::String(pad_graphemes(&s, target_len, &fill, true))))
        }
        "str.pad_end" => {
            let s = extract_str(args, 0, "str.pad_end(s, target_len, fill)", span)?;
            let target_len =
                extract_i64(args, 1, "str.pad_end(s, target_len, fill)", span)?.max(0) as usize;
            let fill = args
                .get(2)
                .and_then(|v| match v {
                    Value::String(f) if !f.is_empty() => Some(f.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| " ".to_string());
            Ok(Some(Value::String(pad_graphemes(&s, target_len, &fill, false))))
        }
        "str.split" => {
            let s = extract_str(args, 0, "str.split(s, sep)", span)?;
            let sep = extract_str(args, 1, "str.split(s, sep)", span)?;
            let parts: Vec<Value> = if sep.is_empty() {
                split_graphemes(&s).into_iter().map(Value::String).collect()
            } else {
                s.split(&sep).map(|p| Value::String(p.to_string())).collect()
            };
            Ok(Some(Value::List(parts)))
        }
        "str.join" => {
            let list = extract_list(args, 0, "str.join(list, sep)", span)?;
            let sep = extract_str(args, 1, "str.join(list, sep)", span)?;
            let mut strs = Vec::with_capacity(list.len());
            for item in &list {
                match item {
                    Value::String(s) | Value::Iri(s) => strs.push(s.clone()),
                    other => strs.push(format!("{other}")),
                }
            }
            Ok(Some(Value::String(strs.join(&sep))))
        }
        "str.replace_literal" => {
            let s = extract_str(args, 0, "str.replace_literal(s, pat, rep)", span)?;
            let pat = extract_str(args, 1, "str.replace_literal(s, pat, rep)", span)?;
            let rep = extract_str(args, 2, "str.replace_literal(s, pat, rep)", span)?;
            Ok(Some(Value::String(s.replace(&pat, &rep))))
        }
        "str.starts_with" => {
            let s = extract_str(args, 0, "str.starts_with(s, prefix)", span)?;
            let prefix = extract_str(args, 1, "str.starts_with(s, prefix)", span)?;
            Ok(Some(Value::Bool(s.starts_with(&prefix))))
        }
        "str.ends_with" => {
            let s = extract_str(args, 0, "str.ends_with(s, suffix)", span)?;
            let suffix = extract_str(args, 1, "str.ends_with(s, suffix)", span)?;
            Ok(Some(Value::Bool(s.ends_with(&suffix))))
        }
        "str.contains" => {
            let s = extract_str(args, 0, "str.contains(s, sub)", span)?;
            let sub = extract_str(args, 1, "str.contains(s, sub)", span)?;
            Ok(Some(Value::Bool(s.contains(&sub))))
        }
        "str.to_upper" => {
            let s = extract_str(args, 0, "str.to_upper(s)", span)?;
            Ok(Some(Value::String(s.to_uppercase())))
        }
        "str.to_lower" => {
            let s = extract_str(args, 0, "str.to_lower(s)", span)?;
            Ok(Some(Value::String(s.to_lowercase())))
        }
        _ => Ok(None),
    }
}

fn pad_graphemes(s: &str, target_len: usize, fill: &str, pad_left: bool) -> String {
    let gs = split_graphemes(s);
    if gs.len() >= target_len {
        return s.to_string();
    }
    let fill_gs = split_graphemes(fill);
    if fill_gs.is_empty() {
        return s.to_string();
    }
    let needed = target_len - gs.len();
    let mut pad_str = String::new();
    for i in 0..needed {
        pad_str.push_str(&fill_gs[i % fill_gs.len()]);
    }
    if pad_left {
        format!("{pad_str}{s}")
    } else {
        format!("{s}{pad_str}")
    }
}
