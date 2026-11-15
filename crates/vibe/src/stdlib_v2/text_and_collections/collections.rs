//! Immutable-by-Default Collections, Type-Aware Natural Sorting, Seeded Fisher-Yates Shuffle,
//! Double-Ended Queue (`deque.*`), Priority Queue (`pqueue.*`), and Structural Sets (`set.*`).

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

use super::{
    extract_i64, extract_list, extract_opt_i64, extract_str, resolve_element_index,
    resolve_slice_bound,
};

/// Maximum elements produced by `list.range` inside a single pure call.
const MAX_RANGE_LEN: usize = 65_536;

/// Dispatch `list.*`, `deque.*`, `pqueue.*`, and `set.*` calls.
pub(crate) fn call_collections(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "list.range" => {
            let start = extract_i64(args, 0, "list.range(start, end, step_opt)", span)?;
            let end = extract_i64(args, 1, "list.range(start, end, step_opt)", span)?;
            let default_step = if start <= end { 1 } else { -1 };
            let step = extract_opt_i64(args, 2).unwrap_or(default_step);
            if step == 0 {
                return Err(Diagnostic::new(DiagCode::E100, span, "list.range: step cannot be 0"));
            }
            let mut out = Vec::new();
            let mut cur = start;
            while (step > 0 && cur < end) || (step < 0 && cur > end) {
                if out.len() >= MAX_RANGE_LEN {
                    return Err(Diagnostic::new(
                        DiagCode::E400,
                        span,
                        "list.range: range exceeds maximum bounded length",
                    ));
                }
                out.push(Value::I64(cur));
                cur = cur.saturating_add(step);
            }
            Ok(Some(Value::List(out)))
        }
        "list.at" => {
            let xs = extract_list(args, 0, "list.at(xs, idx)", span)?;
            let idx = extract_i64(args, 1, "list.at(xs, idx)", span)?;
            let val = resolve_element_index(idx, xs.len())
                .map(|i| xs[i].clone())
                .unwrap_or(Value::Null);
            Ok(Some(val))
        }
        "list.slice" => {
            let xs = extract_list(args, 0, "list.slice(xs, start, end_opt, step_opt)", span)?;
            let len = xs.len();
            let start_raw = extract_opt_i64(args, 1).unwrap_or(0);
            let step = extract_opt_i64(args, 3).unwrap_or(1);
            if step == 0 {
                return Err(Diagnostic::new(DiagCode::E100, span, "list.slice: step cannot be 0"));
            }
            let s_idx = resolve_slice_bound(start_raw, len);
            let e_idx = extract_opt_i64(args, 2)
                .map(|e| resolve_slice_bound(e, len))
                .unwrap_or(len);

            let mut out = Vec::new();
            if step > 0 {
                let stride = step as usize;
                let mut i = s_idx;
                while i < e_idx && i < len {
                    out.push(xs[i].clone());
                    i += stride;
                }
            } else {
                let stride = (-step) as usize;
                if s_idx <= e_idx {
                    let sub = &xs[s_idx..e_idx.min(len)];
                    let mut i = sub.len();
                    while i > 0 {
                        out.push(sub[i - 1].clone());
                        if i <= stride {
                            break;
                        }
                        i -= stride;
                    }
                } else {
                    let mut i = s_idx.min(len.saturating_sub(1));
                    while i > e_idx {
                        out.push(xs[i].clone());
                        if i < e_idx + stride {
                            break;
                        }
                        i -= stride;
                    }
                }
            }
            Ok(Some(Value::List(out)))
        }
        "list.sort_natural" => {
            let mut xs = extract_list(args, 0, "list.sort_natural(xs)", span)?;
            xs.sort_by(compare_values_natural);
            Ok(Some(Value::List(xs)))
        }
        "list.chunk" => {
            let xs = extract_list(args, 0, "list.chunk(xs, size)", span)?;
            let size = extract_i64(args, 1, "list.chunk(xs, size)", span)?;
            if size <= 0 {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "list.chunk: size must be positive",
                ));
            }
            let chunks = xs.chunks(size as usize).map(|c| Value::List(c.to_vec())).collect();
            Ok(Some(Value::List(chunks)))
        }
        "list.windows" => {
            let xs = extract_list(args, 0, "list.windows(xs, size)", span)?;
            let size = extract_i64(args, 1, "list.windows(xs, size)", span)?;
            if size <= 0 {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "list.windows: size must be positive",
                ));
            }
            let w = size as usize;
            let windows = if w > xs.len() {
                Vec::new()
            } else {
                xs.windows(w).map(|win| Value::List(win.to_vec())).collect()
            };
            Ok(Some(Value::List(windows)))
        }
        "list.zip" => {
            let a = extract_list(args, 0, "list.zip(a, b)", span)?;
            let b = extract_list(args, 1, "list.zip(a, b)", span)?;
            let zipped = a.into_iter().zip(b).map(|(x, y)| Value::List(vec![x, y])).collect();
            Ok(Some(Value::List(zipped)))
        }
        "list.unzip" => {
            let pairs = extract_list(args, 0, "list.unzip(pairs)", span)?;
            let mut left = Vec::with_capacity(pairs.len());
            let mut right = Vec::with_capacity(pairs.len());
            for item in pairs {
                match item {
                    Value::List(mut p) if p.len() >= 2 => {
                        let r = p.remove(1);
                        let l = p.remove(0);
                        left.push(l);
                        right.push(r);
                    }
                    Value::Record(rec) => {
                        left.push(
                            rec.get("first")
                                .or_else(|| rec.get("index"))
                                .cloned()
                                .unwrap_or(Value::Null),
                        );
                        right.push(
                            rec.get("second")
                                .or_else(|| rec.get("value"))
                                .cloned()
                                .unwrap_or(Value::Null),
                        );
                    }
                    _ => {}
                }
            }
            Ok(Some(Value::List(vec![Value::List(left), Value::List(right)])))
        }
        "list.enumerate" => {
            let xs = extract_list(args, 0, "list.enumerate(xs)", span)?;
            let out = xs
                .into_iter()
                .enumerate()
                .map(|(idx, val)| {
                    let mut rec = BTreeMap::new();
                    rec.insert("index".to_string(), Value::I64(idx as i64));
                    rec.insert("value".to_string(), val);
                    Value::Record(rec)
                })
                .collect();
            Ok(Some(Value::List(out)))
        }
        "list.partition" => {
            let xs = extract_list(args, 0, "list.partition(xs, pivot_or_field)", span)?;
            let crit = args.get(1).cloned().unwrap_or(Value::Null);
            let mut matches = Vec::new();
            let mut rest = Vec::new();
            for item in xs {
                let hit = match (&item, &crit) {
                    (Value::Record(rec), Value::String(field)) => {
                        rec.get(field).map(Value::is_truthy).unwrap_or(false)
                    }
                    (a, b) if a.as_f64().is_some() && b.as_f64().is_some() => {
                        a.as_f64().unwrap_or(0.0) < b.as_f64().unwrap_or(0.0)
                    }
                    (a, b) => a == b,
                };
                if hit { matches.push(item) } else { rest.push(item) }
            }
            let mut rec = BTreeMap::new();
            rec.insert("matches".to_string(), Value::List(matches.clone()));
            rec.insert("rest".to_string(), Value::List(rest.clone()));
            rec.insert("left".to_string(), Value::List(matches));
            rec.insert("right".to_string(), Value::List(rest));
            Ok(Some(Value::Record(rec)))
        }
        "list.uniq" => {
            let xs = extract_list(args, 0, "list.uniq(xs)", span)?;
            Ok(Some(Value::List(dedup_values(xs))))
        }
        "list.uniq_by" => {
            let xs = extract_list(args, 0, "list.uniq_by(xs, key_field)", span)?;
            let key_field = extract_str(args, 1, "list.uniq_by(xs, key_field)", span)?;
            let mut seen = Vec::new();
            let mut out = Vec::new();
            for item in xs {
                let proj = match &item {
                    Value::Record(rec) => rec.get(&key_field).cloned().unwrap_or(Value::Null),
                    other => other.clone(),
                };
                if !seen.contains(&proj) {
                    seen.push(proj);
                    out.push(item);
                }
            }
            Ok(Some(Value::List(out)))
        }
        "list.intersperse" => {
            let xs = extract_list(args, 0, "list.intersperse(xs, sep)", span)?;
            let sep = args.get(1).cloned().unwrap_or(Value::Null);
            let mut out = Vec::with_capacity(xs.len().saturating_mul(2));
            for (i, item) in xs.into_iter().enumerate() {
                if i > 0 {
                    out.push(sep.clone());
                }
                out.push(item);
            }
            Ok(Some(Value::List(out)))
        }
        "list.flatten" => {
            let xs = extract_list(args, 0, "list.flatten(xs)", span)?;
            let mut out = Vec::new();
            for item in xs {
                match item {
                    Value::List(sub) => out.extend(sub),
                    other => out.push(other),
                }
            }
            Ok(Some(Value::List(out)))
        }
        "list.shuffle_seeded" => {
            let mut xs = extract_list(args, 0, "list.shuffle_seeded(xs, seed)", span)?;
            let mut state = extract_i64(args, 1, "list.shuffle_seeded(xs, seed)", span)? as u64;
            for i in (1..xs.len()).rev() {
                state = splitmix64_next(state);
                xs.swap(i, (state as usize) % (i + 1));
            }
            Ok(Some(Value::List(xs)))
        }
        "deque.new" => {
            let items = match args.first() {
                Some(Value::List(xs)) => xs.clone(),
                _ => Vec::new(),
            };
            Ok(Some(wrap_collection("__deque", items)))
        }
        "deque.push_front" => {
            let mut items = extract_collection_items(args, 0, "deque.push_front(d, val)", span)?;
            items.insert(0, args.get(1).cloned().unwrap_or(Value::Null));
            Ok(Some(wrap_collection("__deque", items)))
        }
        "deque.push_back" => {
            let mut items = extract_collection_items(args, 0, "deque.push_back(d, val)", span)?;
            items.push(args.get(1).cloned().unwrap_or(Value::Null));
            Ok(Some(wrap_collection("__deque", items)))
        }
        "deque.pop_front" => {
            let mut items = extract_collection_items(args, 0, "deque.pop_front(d)", span)?;
            let val = if items.is_empty() { Value::Null } else { items.remove(0) };
            let mut rec = BTreeMap::new();
            rec.insert("value".to_string(), val);
            rec.insert("deque".to_string(), wrap_collection("__deque", items));
            Ok(Some(Value::Record(rec)))
        }
        "deque.pop_back" => {
            let mut items = extract_collection_items(args, 0, "deque.pop_back(d)", span)?;
            let val = items.pop().unwrap_or(Value::Null);
            let mut rec = BTreeMap::new();
            rec.insert("value".to_string(), val);
            rec.insert("deque".to_string(), wrap_collection("__deque", items));
            Ok(Some(Value::Record(rec)))
        }
        "pqueue.new" => {
            let mut items = match args.first() {
                Some(Value::List(xs)) => xs.clone(),
                _ => Vec::new(),
            };
            items.sort_by(compare_pqueue_entries);
            Ok(Some(wrap_collection("__pqueue", items)))
        }
        "pqueue.push" => {
            let mut items =
                extract_collection_items(args, 0, "pqueue.push(pq, priority, value)", span)?;
            let mut entry = BTreeMap::new();
            entry.insert("priority".to_string(), args.get(1).cloned().unwrap_or(Value::I64(0)));
            entry.insert("value".to_string(), args.get(2).cloned().unwrap_or(Value::Null));
            items.push(Value::Record(entry));
            items.sort_by(compare_pqueue_entries);
            Ok(Some(wrap_collection("__pqueue", items)))
        }
        "pqueue.pop_min" | "pqueue.pop_max" => {
            let mut items = extract_collection_items(args, 0, path, span)?;
            items.sort_by(compare_pqueue_entries);
            let popped = if items.is_empty() {
                None
            } else if path == "pqueue.pop_min" {
                Some(items.remove(0))
            } else {
                items.pop()
            };
            let (prio, val) = popped.map(unpack_pqueue_entry).unwrap_or((Value::Null, Value::Null));
            let mut rec = BTreeMap::new();
            rec.insert("priority".to_string(), prio);
            rec.insert("value".to_string(), val);
            rec.insert("pqueue".to_string(), wrap_collection("__pqueue", items));
            Ok(Some(Value::Record(rec)))
        }
        "set.from_list" => {
            let xs = extract_collection_items(args, 0, "set.from_list(xs)", span)?;
            Ok(Some(Value::List(dedup_values(xs))))
        }
        "set.union" => {
            let mut a = extract_collection_items(args, 0, "set.union(a, b)", span)?;
            a.extend(extract_collection_items(args, 1, "set.union(a, b)", span)?);
            Ok(Some(Value::List(dedup_values(a))))
        }
        "set.intersection" => {
            let a = dedup_values(extract_collection_items(args, 0, "set.intersection(a, b)", span)?);
            let b = extract_collection_items(args, 1, "set.intersection(a, b)", span)?;
            Ok(Some(Value::List(a.into_iter().filter(|x| b.contains(x)).collect())))
        }
        "set.difference" => {
            let a = dedup_values(extract_collection_items(args, 0, "set.difference(a, b)", span)?);
            let b = extract_collection_items(args, 1, "set.difference(a, b)", span)?;
            Ok(Some(Value::List(a.into_iter().filter(|x| !b.contains(x)).collect())))
        }
        "set.symmetric_difference" => {
            let a = dedup_values(extract_collection_items(args, 0, path, span)?);
            let b = dedup_values(extract_collection_items(args, 1, path, span)?);
            let mut out: Vec<Value> = a.iter().filter(|x| !b.contains(x)).cloned().collect();
            out.extend(b.into_iter().filter(|x| !a.contains(x)));
            Ok(Some(Value::List(out)))
        }
        "set.is_subset" => {
            let a = extract_collection_items(args, 0, "set.is_subset(a, b)", span)?;
            let b = extract_collection_items(args, 1, "set.is_subset(a, b)", span)?;
            Ok(Some(Value::Bool(a.iter().all(|x| b.contains(x)))))
        }
        "set.is_disjoint" => {
            let a = extract_collection_items(args, 0, "set.is_disjoint(a, b)", span)?;
            let b = extract_collection_items(args, 1, "set.is_disjoint(a, b)", span)?;
            Ok(Some(Value::Bool(a.iter().all(|x| !b.contains(x)))))
        }
        _ => Ok(None),
    }
}

fn compare_values_natural(a: &Value, b: &Value) -> Ordering {
    match (a.as_f64(), b.as_f64()) {
        (Some(fa), Some(fb)) => fa.partial_cmp(&fb).unwrap_or(Ordering::Equal),
        _ => match (a, b) {
            (Value::String(sa), Value::String(sb)) | (Value::Iri(sa), Value::Iri(sb)) => sa.cmp(sb),
            (Value::Bool(ba), Value::Bool(bb)) => ba.cmp(bb),
            _ => format!("{a}").cmp(&format!("{b}")),
        },
    }
}

fn dedup_values(xs: Vec<Value>) -> Vec<Value> {
    let mut out = Vec::new();
    for x in xs {
        if !out.contains(&x) {
            out.push(x);
        }
    }
    out
}

fn splitmix64_next(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

fn wrap_collection(tag: &str, items: Vec<Value>) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert(tag.to_string(), Value::Bool(true));
    rec.insert("len".to_string(), Value::I64(items.len() as i64));
    rec.insert("items".to_string(), Value::List(items));
    Value::Record(rec)
}

fn extract_collection_items(
    args: &[Value],
    idx: usize,
    ctx: &str,
    span: Span,
) -> Result<Vec<Value>, Diagnostic> {
    match args.get(idx) {
        Some(Value::List(xs)) => Ok(xs.clone()),
        Some(Value::Record(rec)) => match rec.get("items") {
            Some(Value::List(xs)) => Ok(xs.clone()),
            _ => Ok(Vec::new()),
        },
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be a collection or list"),
        )),
    }
}

fn compare_pqueue_entries(a: &Value, b: &Value) -> Ordering {
    let (pa, va) = borrow_pqueue_entry(a);
    let (pb, vb) = borrow_pqueue_entry(b);
    compare_values_natural(pa, pb).then_with(|| compare_values_natural(va, vb))
}

fn borrow_pqueue_entry(v: &Value) -> (&Value, &Value) {
    if let Value::Record(rec) = v {
        (
            rec.get("priority").unwrap_or(&Value::Null),
            rec.get("value").unwrap_or(&Value::Null),
        )
    } else {
        (v, &Value::Null)
    }
}

fn unpack_pqueue_entry(v: Value) -> (Value, Value) {
    if let Value::Record(mut rec) = v {
        (
            rec.remove("priority").unwrap_or(Value::Null),
            rec.remove("value").unwrap_or(Value::Null),
        )
    } else {
        (Value::I64(0), v)
    }
}
