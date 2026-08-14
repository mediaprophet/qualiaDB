//! Person-authored measurements / characteristics / attributes. Saves to the vault
//! and asks the 3D view to reload so the body reflects what they declared.

use std::collections::BTreeMap;

use super::anatomy_measurement_fields::{
    fields_in, MeasureField, MeasureGroup, MeasureInput, MEASURE_FIELDS,
};
use super::host_client::{
    get_body_constitution, reset_body_constitution, set_body_constitution,
};
use dioxus::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
struct FormState {
    measures: BTreeMap<String, String>,
    age_years: String,
    karyotype: String,
    ethnicities: String,
    eye_colour: String,
    hair_colour: String,
    skin_tone: String,
    absent: String,
    notes: String,
    status: String,
    honesty: Vec<String>,
    open: bool,
}

fn kebab(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

fn display_value(field: &MeasureField, stored: Option<&serde_json::Value>) -> String {
    let Some(n) = stored.and_then(|x| x.as_u64()) else {
        return String::new();
    };
    match field.input {
        MeasureInput::Kg => format!("{}", n as f64 / 1000.0),
        MeasureInput::Cm => format!("{}", n as f64 / 10.0),
        MeasureInput::Mm => format!("{n}"),
    }
}

fn parse_stored(field: &MeasureField, typed: &str) -> Result<Option<u64>, String> {
    let t = typed.trim();
    if t.is_empty() {
        return Ok(None);
    }
    let n: f64 = t
        .parse()
        .map_err(|_| format!("{} must be a number", field.label))?;
    if n < 0.0 {
        return Err(format!("{} cannot be negative", field.label));
    }
    let stored = match field.input {
        MeasureInput::Kg => (n * 1000.0).round(),
        MeasureInput::Cm => (n * 10.0).round(),
        MeasureInput::Mm => n.round(),
    };
    if stored > u32::MAX as f64 {
        return Err(format!("{} is too large", field.label));
    }
    Ok(Some(stored as u64))
}

fn fill_from_json(v: &serde_json::Value) -> FormState {
    let c = v.get("constitution").cloned().unwrap_or(serde_json::json!({}));
    let m = c.get("measurements").cloned().unwrap_or(serde_json::json!({}));
    let ch = c
        .get("characteristics")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let a = c.get("attributes").cloned().unwrap_or(serde_json::json!({}));
    let karyotype = match ch.get("karyotype").and_then(|k| k.as_str()) {
        Some("Xy") | Some("XY") => "XY".to_string(),
        Some("Xx") | Some("XX") => "XX".to_string(),
        Some(other) => other.to_string(),
        None => String::new(),
    };
    let absent = a
        .get("absent")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| p.get("key").and_then(|k| k.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let honesty = v
        .get("fit")
        .and_then(|f| f.get("honesty_notes"))
        .and_then(|n| n.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let mut measures = BTreeMap::new();
    for field in MEASURE_FIELDS {
        measures.insert(field.id.to_string(), display_value(field, m.get(field.id)));
    }
    FormState {
        measures,
        age_years: ch
            .get("age_months")
            .and_then(|x| x.as_u64())
            .map(|mo| format!("{}", mo / 12))
            .unwrap_or_default(),
        karyotype,
        ethnicities: c
            .get("knowledge")
            .and_then(|k| k.get("ethnicities"))
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e.get("label").and_then(|l| l.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default(),
        eye_colour: a
            .get("eye_colour")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string(),
        hair_colour: a
            .get("hair_colour")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string(),
        skin_tone: a
            .get("skin_tone")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string(),
        absent,
        notes: a
            .get("notes")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string(),
        status: String::new(),
        honesty,
        open: true,
    }
}

fn build_json(form: &FormState) -> Result<String, String> {
    let mut measurements = serde_json::Map::new();
    for field in MEASURE_FIELDS {
        let typed = form.measures.get(field.id).map(String::as_str).unwrap_or("");
        if let Some(n) = parse_stored(field, typed)? {
            measurements.insert(field.id.to_string(), serde_json::json!(n));
        }
    }

    let mut characteristics = serde_json::Map::new();
    match form.karyotype.trim() {
        "XY" | "xy" | "Xy" => {
            characteristics.insert("karyotype".into(), serde_json::json!("Xy"));
        }
        "XX" | "xx" | "Xx" => {
            characteristics.insert("karyotype".into(), serde_json::json!("Xx"));
        }
        "" => {}
        other => return Err(format!("karyotype must be XY or XX, not {other}")),
    }
    if let Ok(years) = form.age_years.trim().parse::<u16>() {
        if years > 0 {
            characteristics.insert("age_months".into(), serde_json::json!(years * 12));
        }
    }

    let absent: Vec<serde_json::Value> = form
        .absent
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|key| {
            serde_json::json!({
                "key": key,
                "reason": "person_declared",
            })
        })
        .collect();
    let mut attributes = serde_json::Map::new();
    if !absent.is_empty() {
        attributes.insert("absent".into(), serde_json::Value::Array(absent));
    }
    if !form.notes.trim().is_empty() {
        attributes.insert("notes".into(), serde_json::json!(form.notes.trim()));
    }
    if !form.eye_colour.trim().is_empty() {
        attributes.insert("eye_colour".into(), serde_json::json!(form.eye_colour.trim()));
    }
    if !form.hair_colour.trim().is_empty() {
        attributes.insert("hair_colour".into(), serde_json::json!(form.hair_colour.trim()));
    }
    if !form.skin_tone.trim().is_empty() {
        attributes.insert("skin_tone".into(), serde_json::json!(form.skin_tone.trim()));
    }

    let ethnicities: Vec<serde_json::Value> = form
        .ethnicities
        .split(',')
        .filter_map(|label| {
            let label = label.trim();
            if label.is_empty() {
                return None;
            }
            Some(serde_json::json!({
                "token": kebab(label),
                "label": label,
            }))
        })
        .collect();
    let mut knowledge = serde_json::Map::new();
    if !ethnicities.is_empty() {
        knowledge.insert("ethnicities".into(), serde_json::Value::Array(ethnicities));
    }

    serde_json::to_string(&serde_json::json!({
        "measurements": measurements,
        "characteristics": characteristics,
        "attributes": attributes,
        "knowledge": knowledge,
    }))
    .map_err(|e| e.to_string())
}

fn reload_body(model: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let model = model.to_string();
        let js = format!(
            r#"(function() {{
                var f = document.getElementById('anatomy-portal-iframe');
                if (f && f.contentWindow) {{
                    f.contentWindow.postMessage({{ type: 'anatomy-load-body', model: '{model}' }}, '*');
                    return true;
                }}
                return false;
            }})()"#
        );
        let _ = js_sys::eval(&js);
    }
    let _ = model;
}

fn declared_count(form: &FormState) -> usize {
    MEASURE_FIELDS
        .iter()
        .filter(|f| {
            form.measures
                .get(f.id)
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
        })
        .count()
}

#[component]
pub fn AnatomyConstitutionForm(model: String) -> Element {
    let mut form = use_signal(FormState::default);
    let mut loaded = use_signal(|| false);
    let model_for_reload = model.clone();

    use_effect(move || {
        if loaded() {
            return;
        }
        loaded.set(true);
        spawn(async move {
            match get_body_constitution().await {
                Ok(v) => {
                    let mut next = fill_from_json(&v);
                    next.open = false;
                    form.set(next);
                }
                Err(e) => form.write().status = e,
            }
        });
    });

    let state = form();
    let filled = declared_count(&state);
    rsx! {
        details {
            open: if state.open { Some(true) } else { None },
            style: "margin-top:0.85rem;padding:0.75rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fff);",
            summary {
                style: "cursor:pointer;font-weight:600;font-size:0.92rem;",
                "Your measurements and attributes"
            }
            p {
                style: "margin:0.5rem 0 0.75rem;font-size:0.78rem;color:var(--qualia-text-muted,#666);",
                "Fill in what you have — {filled} of {MEASURE_FIELDS.len()} declared. The same record is used for the body, clothing, footwear, helmets, gloves, rings, and glasses. Anatomy currently reshapes the reference mesh from height, sitting height, inseam, arm span, shoulders, chest, waist, and hip only. Leave any field blank."
            }
            for group in MeasureGroup::all() {
                MeasureGroupBlock {
                    group,
                    measures: state.measures.clone(),
                    onchange: move |(id, value): (String, String)| {
                        form.write().measures.insert(id, value);
                    }
                }
            }
            div {
                style: "display:grid;grid-template-columns:repeat(auto-fill,minmax(140px,1fr));gap:0.55rem;margin-top:0.75rem;",
                Field { label: "Age (years)".to_string(), value: state.age_years.clone(), hint: None, oninput: move |v| form.write().age_years = v }
            }
            div { style: "margin-top:0.65rem;display:flex;flex-direction:column;gap:0.45rem;",
                label { style: "font-size:0.75rem;",
                    "Chromosomal reference (XY / XX) — selects the Visible Human mesh set, not a gender"
                    input {
                        r#type: "text",
                        value: "{state.karyotype}",
                        placeholder: "XY or XX",
                        oninput: move |e| form.write().karyotype = e.value(),
                        style: "display:block;width:100%;margin-top:0.2rem;padding:0.35rem;",
                    }
                }
                label { style: "font-size:0.75rem;",
                    "Ethnicity (your words, comma-separated, repeatable) — biomedical context only: screening and some drug-metabolism hypotheses. Does not change the mesh, skin, hair, or XY/XX."
                    input {
                        r#type: "text",
                        value: "{state.ethnicities}",
                        placeholder: "e.g. Ashkenazi, Greek",
                        oninput: move |e| form.write().ethnicities = e.value(),
                        style: "display:block;width:100%;margin-top:0.2rem;padding:0.35rem;",
                    }
                }
                label { style: "font-size:0.75rem;",
                    "Eye colour (declared appearance — not inferred from ethnicity)"
                    input {
                        r#type: "text",
                        value: "{state.eye_colour}",
                        placeholder: "brown",
                        oninput: move |e| form.write().eye_colour = e.value(),
                        style: "display:block;width:100%;margin-top:0.2rem;padding:0.35rem;",
                    }
                }
                label { style: "font-size:0.75rem;",
                    "Hair colour (declared appearance)"
                    input {
                        r#type: "text",
                        value: "{state.hair_colour}",
                        placeholder: "black",
                        oninput: move |e| form.write().hair_colour = e.value(),
                        style: "display:block;width:100%;margin-top:0.2rem;padding:0.35rem;",
                    }
                }
                label { style: "font-size:0.75rem;",
                    "Skin tone (declared appearance — never inferred from ethnicity or a photo)"
                    input {
                        r#type: "text",
                        value: "{state.skin_tone}",
                        placeholder: "your words",
                        oninput: move |e| form.write().skin_tone = e.value(),
                        style: "display:block;width:100%;margin-top:0.2rem;padding:0.35rem;",
                    }
                }
                label { style: "font-size:0.75rem;",
                    "Parts not on your body (comma-separated keys, e.g. uterus, prostate)"
                    input {
                        r#type: "text",
                        value: "{state.absent}",
                        placeholder: "uterus",
                        oninput: move |e| form.write().absent = e.value(),
                        style: "display:block;width:100%;margin-top:0.2rem;padding:0.35rem;",
                    }
                }
                label { style: "font-size:0.75rem;",
                    "Notes for yourself (not used as geometry)"
                    textarea {
                        value: "{state.notes}",
                        oninput: move |e| form.write().notes = e.value(),
                        style: "display:block;width:100%;min-height:3rem;margin-top:0.2rem;padding:0.35rem;",
                    }
                }
            }
            div { style: "display:flex;gap:0.5rem;margin-top:0.7rem;flex-wrap:wrap;",
                button {
                    r#type: "button",
                    onclick: move |_| {
                        let model = model_for_reload.clone();
                        spawn(async move {
                            match build_json(&form()) {
                                Ok(json) => match set_body_constitution(&json).await {
                                    Ok(v) => {
                                        let mut next = fill_from_json(&v);
                                        next.status = "Saved — clothing, footwear, and helmets can use these numbers; the body uses the fit subset.".into();
                                        next.open = true;
                                        form.set(next);
                                        reload_body(&model);
                                    }
                                    Err(e) => form.write().status = e,
                                },
                                Err(e) => form.write().status = e,
                            }
                        });
                    },
                    "Save to my body"
                }
                button {
                    r#type: "button",
                    onclick: move |_| {
                        let model = model.clone();
                        spawn(async move {
                            match reset_body_constitution().await {
                                Ok(()) => {
                                    form.set(FormState {
                                        status: "Cleared — showing the public reference body.".into(),
                                        open: true,
                                        ..Default::default()
                                    });
                                    reload_body(&model);
                                }
                                Err(e) => form.write().status = e,
                            }
                        });
                    },
                    "Clear"
                }
            }
            if !state.status.is_empty() {
                p { style: "margin:0.5rem 0 0;font-size:0.78rem;", "{state.status}" }
            }
            if !state.honesty.is_empty() {
                ul { style: "margin:0.45rem 0 0;padding-left:1.1rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                    for note in state.honesty.iter() {
                        li { "{note}" }
                    }
                }
            }
        }
    }
}

#[component]
fn MeasureGroupBlock(
    group: MeasureGroup,
    measures: BTreeMap<String, String>,
    onchange: EventHandler<(String, String)>,
) -> Element {
    rsx! {
        details {
            style: "margin-top:0.65rem;",
            open: if group == MeasureGroup::WholeBody || group == MeasureGroup::Torso { Some(true) } else { None },
            summary {
                style: "cursor:pointer;font-weight:600;font-size:0.82rem;",
                "{group.label()}"
            }
            p {
                style: "margin:0.35rem 0 0.5rem;font-size:0.72rem;color:var(--qualia-text-muted,#666);",
                "{group.hint()}"
            }
            div {
                style: "display:grid;grid-template-columns:repeat(auto-fill,minmax(160px,1fr));gap:0.55rem;",
                for field in fields_in(group) {
                    Field {
                        label: format!("{} ({})", field.label, field.input.suffix()),
                        value: measures.get(field.id).cloned().unwrap_or_default(),
                        hint: if field.hint.is_empty() { None } else { Some(field.hint.to_string()) },
                        oninput: {
                            let id = field.id.to_string();
                            move |v| onchange.call((id.clone(), v))
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Field(
    label: String,
    value: String,
    hint: Option<String>,
    oninput: EventHandler<String>,
) -> Element {
    rsx! {
        label { style: "font-size:0.75rem;display:flex;flex-direction:column;gap:0.2rem;",
            title: hint.clone().unwrap_or_default(),
            "{label}"
            input {
                r#type: "text",
                inputmode: "decimal",
                value: "{value}",
                oninput: move |e| oninput.call(e.value()),
                style: "padding:0.35rem;",
            }
        }
    }
}
