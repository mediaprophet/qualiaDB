//! Standard HTML Form Controls, Patching, Focus, Dialog, Popover & Disclosure (`ui.*`).

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

use super::{extract_list, extract_str, opt_bool, opt_str, prop_bool};

/// All 22 standard HTML `<input>` types plus 13 standard form/interactive container controls.
pub const VALID_CONTROL_KINDS: &[&str] = &[
    // 22 HTML <input> types
    "button",
    "checkbox",
    "color",
    "date",
    "datetime-local",
    "email",
    "file",
    "hidden",
    "image",
    "month",
    "number",
    "password",
    "radio",
    "range",
    "reset",
    "search",
    "submit",
    "tel",
    "text",
    "time",
    "url",
    "week",
    // 13 form & interactive elements
    "select",
    "option",
    "optgroup",
    "textarea",
    "output",
    "datalist",
    "fieldset",
    "legend",
    "label",
    "progress",
    "meter",
    "dialog",
    "details",
];

/// Dispatch a standard `ui.*` control or interactive container call.
pub fn call_standard_control(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "ui.patch" => {
            let target_id = extract_str(args, 0, "ui.patch(target_id, property, value)", span)?;
            let property = extract_str(args, 1, "ui.patch(target_id, property, value)", span)?;
            let value = args.get(2).cloned().unwrap_or(Value::Null);
            let mut rec = BTreeMap::new();
            rec.insert("op".into(), Value::String("ui.patch".into()));
            rec.insert("target_id".into(), Value::String(target_id));
            rec.insert("property".into(), Value::String(property));
            rec.insert("value".into(), value);
            Ok(Some(Value::Record(rec)))
        }
        "ui.patch_batch" => {
            let patches = extract_list(args, 0, "ui.patch_batch(patches)", span)?;
            let count = patches.len() as i64;
            let mut rec = BTreeMap::new();
            rec.insert("op".into(), Value::String("ui.patch_batch".into()));
            rec.insert("count".into(), Value::I64(count));
            rec.insert("patches".into(), Value::List(patches));
            Ok(Some(Value::Record(rec)))
        }
        "ui.select_persona" => {
            let persona = extract_str(args, 0, "ui.select_persona(persona, page)", span)?;
            let page = extract_str(args, 1, "ui.select_persona(persona, page)", span)?;
            let badge = match persona.as_str() {
                "novice" => "Active View: 1. Novice (Guided)",
                "geek" => "Active View: 2. Tech Geek (Sysadmin/Net)",
                "dev" => "Active View: 3. Developer (SPARQL/ABI)",
                _ => "Active View: All 3 Personas Stacked",
            };
            let mut rec = BTreeMap::new();
            rec.insert("persona".into(), Value::String(persona));
            rec.insert("page".into(), Value::String(page));
            rec.insert("badge".into(), Value::String(badge.into()));
            rec.insert("runtime".into(), Value::String("vibescript-0.2".into()));
            rec.insert(
                "wire_format".into(),
                Value::String("application/cbor-ld".into()),
            );
            Ok(Some(Value::Record(rec)))
        }
        "ui.control" => {
            let id = extract_str(args, 0, "ui.control(id, kind, props)", span)?;
            let kind = extract_str(args, 1, "ui.control(id, kind, props)", span)?;
            if !VALID_CONTROL_KINDS.contains(&kind.as_str()) {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!("ui.control: unsupported control kind '{kind}'"),
                ));
            }
            let props = match args.get(2) {
                Some(Value::Record(m)) => m.clone(),
                None | Some(Value::Null) => BTreeMap::new(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "ui.control(id, kind, props): argument #2 must be a Record",
                    ))
                }
            };
            let disabled = prop_bool(&props, "disabled", false);
            let read_only =
                prop_bool(&props, "read_only", prop_bool(&props, "readOnly", false));
            let required = prop_bool(&props, "required", false);
            let value = props
                .get("value")
                .cloned()
                .unwrap_or_else(|| Value::String(String::new()));

            let mut rec = BTreeMap::new();
            rec.insert("id".into(), Value::String(id));
            rec.insert("kind".into(), Value::String(kind));
            rec.insert("disabled".into(), Value::Bool(disabled));
            rec.insert("read_only".into(), Value::Bool(read_only));
            rec.insert("required".into(), Value::Bool(required));
            rec.insert("value".into(), value);
            rec.insert("props".into(), Value::Record(props));
            Ok(Some(Value::Record(rec)))
        }
        "ui.set_tri_state" => {
            let field_id = extract_str(args, 0, "ui.set_tri_state(field_id, state)", span)?;
            let state = extract_str(args, 1, "ui.set_tri_state(field_id, state)", span)?;
            let (checked, indeterminate, contradiction) = match state.as_str() {
                "checked" => (true, false, false),
                "unchecked" => (false, false, false),
                "indeterminate" => (false, true, false),
                "contradiction" => (false, false, true),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("ui.set_tri_state: invalid state '{state}'"),
                    ))
                }
            };
            let mut rec = BTreeMap::new();
            rec.insert("op".into(), Value::String("ui.set_tri_state".into()));
            rec.insert("target_id".into(), Value::String(field_id));
            rec.insert("checked".into(), Value::Bool(checked));
            rec.insert("indeterminate".into(), Value::Bool(indeterminate));
            rec.insert("contradiction".into(), Value::Bool(contradiction));
            rec.insert("state".into(), Value::String(state));
            Ok(Some(Value::Record(rec)))
        }
        "ui.select_set_options" => {
            let id = extract_str(args, 0, "ui.select_set_options(id, options)", span)?;
            let options = extract_list(args, 1, "ui.select_set_options(id, options)", span)?;
            let count = options.len() as i64;
            let mut rec = BTreeMap::new();
            rec.insert("op".into(), Value::String("ui.select_set_options".into()));
            rec.insert("target_id".into(), Value::String(id));
            rec.insert("count".into(), Value::I64(count));
            rec.insert("options".into(), Value::List(options));
            Ok(Some(Value::Record(rec)))
        }
        "ui.select_get_selected" => {
            let options = extract_list(args, 0, "ui.select_get_selected(options)", span)?;
            let mut selected = Vec::new();
            for opt in options {
                if let Value::Record(map) = opt {
                    if map.get("selected") == Some(&Value::Bool(true)) {
                        selected.push(
                            map.get("value")
                                .cloned()
                                .unwrap_or_else(|| Value::String(String::new())),
                        );
                    }
                }
            }
            Ok(Some(Value::List(selected)))
        }
        "ui.focus" => {
            let target_id = extract_str(args, 0, "ui.focus(target_id, ...)", span)?;
            let prevent_scroll = opt_bool(args, 1, false);
            let focus_visible = opt_bool(args, 2, true);
            let mut rec = BTreeMap::new();
            rec.insert("op".into(), Value::String("ui.focus".into()));
            rec.insert("target_id".into(), Value::String(target_id));
            rec.insert("prevent_scroll".into(), Value::Bool(prevent_scroll));
            rec.insert("focus_visible".into(), Value::Bool(focus_visible));
            Ok(Some(Value::Record(rec)))
        }
        "ui.blur" => {
            let target_id = extract_str(args, 0, "ui.blur(target_id)", span)?;
            let mut rec = BTreeMap::new();
            rec.insert("op".into(), Value::String("ui.blur".into()));
            rec.insert("target_id".into(), Value::String(target_id));
            Ok(Some(Value::Record(rec)))
        }
        "ui.focus_next" => {
            let items = extract_list(args, 0, "ui.focus_next(items, current_id, reverse)", span)?;
            let current_id =
                extract_str(args, 1, "ui.focus_next(items, current_id, reverse)", span)?;
            let reverse = opt_bool(args, 2, false);
            let mut enabled = Vec::new();
            for item in &items {
                match item {
                    Value::Record(m) => {
                        let disabled = m.get("disabled") == Some(&Value::Bool(true))
                            || m.get("inert") == Some(&Value::Bool(true));
                        if !disabled {
                            if let Some(Value::String(id)) = m.get("id") {
                                if !id.is_empty() {
                                    enabled.push(id.clone());
                                }
                            }
                        }
                    }
                    Value::String(id) if !id.is_empty() => enabled.push(id.clone()),
                    _ => {}
                }
            }
            if enabled.is_empty() {
                return Ok(Some(Value::String(String::new())));
            }
            let pos = enabled.iter().position(|id| id == &current_id);
            let next_idx = match (pos, reverse) {
                (Some(i), false) => (i + 1) % enabled.len(),
                (Some(i), true) => (i + enabled.len() - 1) % enabled.len(),
                (None, false) => 0,
                (None, true) => enabled.len() - 1,
            };
            Ok(Some(Value::String(enabled[next_idx].clone())))
        }
        "ui.dialog_open" => {
            let id = extract_str(args, 0, "ui.dialog_open(id, modal)", span)?;
            let modal = opt_bool(args, 1, true);
            let mut rec = BTreeMap::new();
            rec.insert("op".into(), Value::String("ui.dialog_open".into()));
            rec.insert("target_id".into(), Value::String(id));
            rec.insert("open".into(), Value::Bool(true));
            rec.insert("modal".into(), Value::Bool(modal));
            rec.insert("top_layer".into(), Value::Bool(true));
            rec.insert("background_inert".into(), Value::Bool(modal));
            Ok(Some(Value::Record(rec)))
        }
        "ui.dialog_close" => {
            let id = extract_str(args, 0, "ui.dialog_close(id, return_value)", span)?;
            let return_value = args
                .get(1)
                .cloned()
                .unwrap_or_else(|| Value::String(String::new()));
            let mut rec = BTreeMap::new();
            rec.insert("op".into(), Value::String("ui.dialog_close".into()));
            rec.insert("target_id".into(), Value::String(id));
            rec.insert("open".into(), Value::Bool(false));
            rec.insert("return_value".into(), return_value);
            Ok(Some(Value::Record(rec)))
        }
        "ui.popover_toggle" => {
            let id = extract_str(args, 0, "ui.popover_toggle(id, action)", span)?;
            let action = opt_str(args, 1, "toggle");
            let open = match action.as_str() {
                "show" | "open" | "toggle" => true,
                "hide" | "close" => false,
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("ui.popover_toggle: invalid action '{action}'"),
                    ))
                }
            };
            let mut rec = BTreeMap::new();
            rec.insert("op".into(), Value::String("ui.popover_toggle".into()));
            rec.insert("target_id".into(), Value::String(id));
            rec.insert("action".into(), Value::String(action));
            rec.insert("open".into(), Value::Bool(open));
            rec.insert("top_layer".into(), Value::Bool(open));
            Ok(Some(Value::Record(rec)))
        }
        "ui.accordion_toggle" => {
            let group_name =
                extract_str(args, 0, "ui.accordion_toggle(group, panel, open)", span)?;
            let panel_id = extract_str(args, 1, "ui.accordion_toggle(group, panel, open)", span)?;
            let open = opt_bool(args, 2, true);
            let mut rec = BTreeMap::new();
            rec.insert("op".into(), Value::String("ui.accordion_toggle".into()));
            rec.insert("group_name".into(), Value::String(group_name));
            rec.insert("active_panel_id".into(), Value::String(panel_id));
            rec.insert("open".into(), Value::Bool(open));
            rec.insert("close_siblings".into(), Value::Bool(open));
            Ok(Some(Value::Record(rec)))
        }
        "ui.show_picker" => {
            let target_id = extract_str(args, 0, "ui.show_picker(target_id)", span)?;
            let mut rec = BTreeMap::new();
            rec.insert("op".into(), Value::String("ui.show_picker".into()));
            rec.insert("target_id".into(), Value::String(target_id));
            Ok(Some(Value::Record(rec)))
        }
        _ => Ok(None),
    }
}
