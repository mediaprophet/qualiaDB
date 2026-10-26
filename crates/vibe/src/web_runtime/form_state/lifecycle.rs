//! Form State Lifecycle, Dirty Tracking, Diffing, Reset, and Submission (`form.*`).

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

use super::{
    extract_record, extract_schema_base, extract_str, get_sub_list, get_sub_record, push_bounded,
};

/// Handle form lifecycle operations.
pub(crate) fn call_lifecycle(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "form.create" => {
            let form_id =
                extract_str(args, 0, "form.create(form_id, schema_iri, initial_fields)", span)?;
            let schema_iri =
                extract_str(args, 1, "form.create(form_id, schema_iri, initial_fields)", span)?;
            let initial_fields =
                extract_record(args, 2, "form.create(form_id, schema_iri, initial_fields)", span)?;

            Ok(Some(build_form_record(form_id, schema_iri, initial_fields)))
        }
        "form.get_value" => {
            let form = extract_record(args, 0, "form.get_value(form, field_id)", span)?;
            let field_id = extract_str(args, 1, "form.get_value(form, field_id)", span)?;
            let values = get_sub_record(&form, "values");
            let schema_base = extract_schema_base(&form);
            let scoped_key = format!("{schema_base}{field_id}");
            let val = values
                .get(&field_id)
                .or_else(|| values.get(&scoped_key))
                .or_else(|| form.get(&field_id))
                .cloned()
                .unwrap_or(Value::Null);
            Ok(Some(val))
        }
        "form.set_value" => {
            let mut form =
                extract_record(args, 0, "form.set_value(form, field_id, new_value)", span)?;
            let field_id =
                extract_str(args, 1, "form.set_value(form, field_id, new_value)", span)?;
            let new_value = args.get(2).cloned().ok_or_else(|| {
                Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "form.set_value(form, field_id, new_value): missing new_value",
                )
            })?;

            let mut values = get_sub_record(&form, "values");
            let mut history = get_sub_list(&form, "history");
            push_bounded(&mut history, Value::Record(values.clone()));

            values.insert(field_id.clone(), new_value);

            let mut clocks = get_sub_record(&form, "clocks");
            let next_clk = clocks
                .get(&field_id)
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .saturating_add(1);
            clocks.insert(field_id, Value::I64(next_clk));

            form.insert("values".to_string(), Value::Record(values));
            form.insert("clocks".to_string(), Value::Record(clocks));
            form.insert("history".to_string(), Value::List(history));
            form.insert("redo_stack".to_string(), Value::List(Vec::new()));
            Ok(Some(Value::Record(form)))
        }
        "form.get_group_value" => {
            let form = extract_record(args, 0, "form.get_group_value(form, group_name)", span)?;
            let group_name = extract_str(args, 1, "form.get_group_value(form, group_name)", span)?;
            let values = get_sub_record(&form, "values");

            if let Some(direct) = values.get(&group_name) {
                match direct {
                    Value::Record(sub) => {
                        if sub.get("checked") == Some(&Value::Bool(true)) {
                            return Ok(Some(
                                sub.get("value").cloned().unwrap_or(Value::Bool(true)),
                            ));
                        }
                    }
                    Value::List(items) => {
                        for item in items {
                            if let Value::Record(sub) = item {
                                if sub.get("checked") == Some(&Value::Bool(true)) {
                                    return Ok(Some(
                                        sub.get("value").cloned().unwrap_or(Value::Bool(true)),
                                    ));
                                }
                            }
                        }
                        return Ok(Some(Value::Null));
                    }
                    other => return Ok(Some(other.clone())),
                }
            }

            for (key, val) in &values {
                if let Value::Record(sub) = val {
                    let group_matches = sub
                        .get("group")
                        .or_else(|| sub.get("name"))
                        .and_then(|v| match v {
                            Value::String(s) => Some(s.as_str() == group_name),
                            _ => None,
                        })
                        .unwrap_or(false);
                    let checked = sub.get("checked") == Some(&Value::Bool(true));
                    if group_matches && checked {
                        let out = sub
                            .get("value")
                            .cloned()
                            .unwrap_or_else(|| Value::String(key.clone()));
                        return Ok(Some(out));
                    }
                }
            }
            Ok(Some(Value::Null))
        }
        "form.reset" => {
            let mut form = extract_record(args, 0, "form.reset(form)", span)?;
            let values = get_sub_record(&form, "values");
            let defaults = get_sub_record(&form, "default_values");

            let all_keys: BTreeSet<&String> = values.keys().chain(defaults.keys()).collect();
            let mut patches = Vec::new();
            for k in all_keys {
                let cur = values.get(k);
                let def = defaults.get(k);
                if cur != def {
                    let mut patch = BTreeMap::new();
                    patch.insert("op".to_string(), Value::String("ui.patch".to_string()));
                    patch.insert("target_id".to_string(), Value::String(k.clone()));
                    patch.insert("property".to_string(), Value::String("value".to_string()));
                    patch.insert("value".to_string(), def.cloned().unwrap_or(Value::Null));
                    patches.push(Value::Record(patch));
                }
            }

            form.insert("values".to_string(), Value::Record(defaults));
            form.insert("redo_stack".to_string(), Value::List(Vec::new()));
            form.insert("patches".to_string(), Value::List(patches));
            Ok(Some(Value::Record(form)))
        }
        "form.is_dirty" => {
            let form = extract_record(args, 0, "form.is_dirty(form)", span)?;
            let values = get_sub_record(&form, "values");
            let defaults = get_sub_record(&form, "default_values");
            Ok(Some(Value::Bool(values != defaults)))
        }
        "form.field_is_dirty" => {
            let form = extract_record(args, 0, "form.field_is_dirty(form, field_id)", span)?;
            let field_id = extract_str(args, 1, "form.field_is_dirty(form, field_id)", span)?;
            let values = get_sub_record(&form, "values");
            let defaults = get_sub_record(&form, "default_values");
            Ok(Some(Value::Bool(
                values.get(&field_id) != defaults.get(&field_id),
            )))
        }
        "form.diff" => {
            let form = extract_record(args, 0, "form.diff(form)", span)?;
            let values = get_sub_record(&form, "values");
            let defaults = get_sub_record(&form, "default_values");
            let all_keys: BTreeSet<&String> = values.keys().chain(defaults.keys()).collect();

            let mut diff = BTreeMap::new();
            for k in all_keys {
                let cur = values.get(k);
                let def = defaults.get(k);
                if cur != def {
                    let mut entry = BTreeMap::new();
                    entry.insert("initial".to_string(), def.cloned().unwrap_or(Value::Null));
                    entry.insert("current".to_string(), cur.cloned().unwrap_or(Value::Null));
                    diff.insert(k.clone(), Value::Record(entry));
                }
            }
            Ok(Some(Value::Record(diff)))
        }
        "form.commit_defaults" => {
            let mut form = extract_record(args, 0, "form.commit_defaults(form)", span)?;
            let values = get_sub_record(&form, "values");
            form.insert("default_values".to_string(), Value::Record(values));
            Ok(Some(Value::Record(form)))
        }
        "form.request_submit" => {
            let form = extract_record(args, 0, "form.request_submit(form, submitter_id)", span)?;
            let submitter_id = args
                .get(1)
                .and_then(|v| match v {
                    Value::String(s) | Value::Iri(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "default".to_string());

            let form_id = form
                .get("form_id")
                .cloned()
                .unwrap_or_else(|| Value::String(String::new()));
            let values = get_sub_record(&form, "values");
            let defaults = get_sub_record(&form, "default_values");
            let dirty = values != defaults;
            let field_count = values.len() as i64;

            let mut env = BTreeMap::new();
            env.insert("submitted".to_string(), Value::Bool(true));
            env.insert("form_id".to_string(), form_id);
            env.insert("submitter_id".to_string(), Value::String(submitter_id));
            env.insert("values".to_string(), Value::Record(values));
            env.insert("dirty".to_string(), Value::Bool(dirty));
            env.insert("field_count".to_string(), Value::I64(field_count));
            Ok(Some(Value::Record(env)))
        }
        _ => Ok(None),
    }
}

pub(crate) fn build_form_record(
    form_id: String,
    schema_iri: String,
    initial_fields: BTreeMap<String, Value>,
) -> Value {
    let mut clocks = BTreeMap::new();
    for k in initial_fields.keys() {
        clocks.insert(k.clone(), Value::I64(1));
    }

    let mut form = BTreeMap::new();
    form.insert("form_id".to_string(), Value::String(form_id));
    form.insert("schema_iri".to_string(), Value::Iri(schema_iri));
    form.insert(
        "default_values".to_string(),
        Value::Record(initial_fields.clone()),
    );
    form.insert("values".to_string(), Value::Record(initial_fields));
    form.insert("clocks".to_string(), Value::Record(clocks));
    form.insert("history".to_string(), Value::List(Vec::new()));
    form.insert("redo_stack".to_string(), Value::List(Vec::new()));
    form.insert("checkpoints".to_string(), Value::Record(BTreeMap::new()));
    form.insert("step_index".to_string(), Value::I64(0));
    Value::Record(form)
}
