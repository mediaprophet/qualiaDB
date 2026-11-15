//! Form Transactional Undo/Redo Stack, Named Checkpoints, and Lamport LWW CRDT Merge (`form.*`).

use std::collections::{BTreeMap, BTreeSet};

use crate::error::Diagnostic;
use crate::span::Span;
use crate::value::Value;

use super::{extract_record, extract_str, get_sub_list, get_sub_record, push_bounded};

/// Handle undo/redo, checkpointing, and CRDT merge operations.
pub(crate) fn call_history_crdt(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "form.checkpoint" => {
            let mut form = extract_record(args, 0, "form.checkpoint(form, label)", span)?;
            let label = extract_str(args, 1, "form.checkpoint(form, label)", span)?;
            let values = get_sub_record(&form, "values");
            let mut checkpoints = get_sub_record(&form, "checkpoints");
            checkpoints.insert(label, Value::Record(values));
            form.insert("checkpoints".to_string(), Value::Record(checkpoints));
            Ok(Some(Value::Record(form)))
        }
        "form.undo" => {
            let mut form = extract_record(args, 0, "form.undo(form)", span)?;
            let mut history = get_sub_list(&form, "history");
            if let Some(prev) = history.pop() {
                let cur_values = get_sub_record(&form, "values");
                let mut redo = get_sub_list(&form, "redo_stack");
                push_bounded(&mut redo, Value::Record(cur_values));
                form.insert("values".to_string(), prev);
                form.insert("history".to_string(), Value::List(history));
                form.insert("redo_stack".to_string(), Value::List(redo));
            }
            Ok(Some(Value::Record(form)))
        }
        "form.redo" => {
            let mut form = extract_record(args, 0, "form.redo(form)", span)?;
            let mut redo = get_sub_list(&form, "redo_stack");
            if let Some(next) = redo.pop() {
                let cur_values = get_sub_record(&form, "values");
                let mut history = get_sub_list(&form, "history");
                push_bounded(&mut history, Value::Record(cur_values));
                form.insert("values".to_string(), next);
                form.insert("history".to_string(), Value::List(history));
                form.insert("redo_stack".to_string(), Value::List(redo));
            }
            Ok(Some(Value::Record(form)))
        }
        "form.can_undo" => {
            let form = extract_record(args, 0, "form.can_undo(form)", span)?;
            let history = get_sub_list(&form, "history");
            Ok(Some(Value::Bool(!history.is_empty())))
        }
        "form.can_redo" => {
            let form = extract_record(args, 0, "form.can_redo(form)", span)?;
            let redo = get_sub_list(&form, "redo_stack");
            Ok(Some(Value::Bool(!redo.is_empty())))
        }
        "form.rollback_to" => {
            let mut form = extract_record(args, 0, "form.rollback_to(form, label)", span)?;
            let label = extract_str(args, 1, "form.rollback_to(form, label)", span)?;
            let checkpoints = get_sub_record(&form, "checkpoints");
            if let Some(Value::Record(cp_vals)) = checkpoints.get(&label) {
                let cur_values = get_sub_record(&form, "values");
                let mut history = get_sub_list(&form, "history");
                push_bounded(&mut history, Value::Record(cur_values));
                form.insert("values".to_string(), Value::Record(cp_vals.clone()));
                form.insert("history".to_string(), Value::List(history));
                form.insert("redo_stack".to_string(), Value::List(Vec::new()));
            }
            Ok(Some(Value::Record(form)))
        }
        "form.crdt_merge" => {
            let mut local_form =
                extract_record(args, 0, "form.crdt_merge(local_form, remote_form)", span)?;
            let remote_form =
                extract_record(args, 1, "form.crdt_merge(local_form, remote_form)", span)?;

            let l_vals = get_sub_record(&local_form, "values");
            let r_vals = get_sub_record(&remote_form, "values");
            let l_clks = get_sub_record(&local_form, "clocks");
            let r_clks = get_sub_record(&remote_form, "clocks");

            let all_keys: BTreeSet<&String> = l_vals.keys().chain(r_vals.keys()).collect();
            let mut merged_vals = BTreeMap::new();
            let mut merged_clks = BTreeMap::new();

            for k in all_keys {
                let l_clk = l_clks.get(k).and_then(Value::as_i64).unwrap_or(0);
                let r_clk = r_clks.get(k).and_then(Value::as_i64).unwrap_or(0);
                match (l_vals.get(k), r_vals.get(k)) {
                    (Some(lv), Some(rv)) => {
                        let remote_wins = if r_clk != l_clk {
                            r_clk > l_clk
                        } else {
                            format!("{rv}") > format!("{lv}")
                        };
                        if remote_wins {
                            merged_vals.insert(k.clone(), rv.clone());
                            merged_clks.insert(k.clone(), Value::I64(r_clk));
                        } else {
                            merged_vals.insert(k.clone(), lv.clone());
                            merged_clks.insert(k.clone(), Value::I64(l_clk));
                        }
                    }
                    (None, Some(rv)) => {
                        merged_vals.insert(k.clone(), rv.clone());
                        merged_clks.insert(k.clone(), Value::I64(r_clk.max(1)));
                    }
                    (Some(lv), None) => {
                        merged_vals.insert(k.clone(), lv.clone());
                        merged_clks.insert(k.clone(), Value::I64(l_clk.max(1)));
                    }
                    (None, None) => {}
                }
            }

            local_form.insert("values".to_string(), Value::Record(merged_vals));
            local_form.insert("clocks".to_string(), Value::Record(merged_clks));
            Ok(Some(Value::Record(local_form)))
        }
        _ => Ok(None),
    }
}
