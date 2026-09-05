//! Person-controlled clinical calculator form. Empty until the person enters values.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, Event, HtmlInputElement, HtmlSelectElement};

use super::model::{CalculatorDraft, CalculatorKind, NOT_DIAGNOSIS};
use crate::browser::native_daemon::{daemon_invoke, is_daemon_connected};
use crate::browser::surface_states;

pub fn build(document: &Document) -> Element {
    surface_states::install(document);
    let root = document.create_element("section").unwrap();
    root.set_class_name("health-home health-calculators");
    root.set_attribute("data-health-calculators", "").ok();
    root.set_attribute("data-honesty", "running").ok();
    root.set_inner_html(r#"
      <header class="health-hero">
        <div>
          <div class="health-eyebrow">Clinical calculators</div>
          <h2>Risk estimates from entered values</h2>
          <p>Every required input and unit must be entered. Empty fields are not patient values. The result names the algorithm and is not a diagnosis.</p>
        </div>
        <div class="health-hero-actions">
          <span class="health-privacy-chip">Not a diagnosis</span>
        </div>
      </header>

      <section class="health-card" aria-labelledby="calc-entry-title">
        <div class="health-card-heading">
          <div>
            <span class="health-card-kicker">Native ClinicalRisk</span>
            <h3 id="calc-entry-title">Enter the values this algorithm needs</h3>
          </div>
          <span class="health-unit-badge">years · mmol/L · mmHg</span>
        </div>
        <div class="health-form-grid">
          <label class="health-field health-field-wide">
            <span>Algorithm</span>
            <select data-calc-field="kind">
              <option value="">Choose…</option>
              <option value="framingham">Framingham 10-year CVD (Wilson 1998)</option>
              <option value="cha2ds2_vasc">CHA₂DS₂-VASc stroke risk (Lip 2010)</option>
              <option value="score2">SCORE2 10-year CVD (ESC 2021)</option>
            </select>
          </label>
          <p class="health-field-wide" data-calc-applicability>Choose an algorithm to see its age band and required units.</p>
          <label class="health-field">
            <span>Age <small>years</small></span>
            <input type="number" inputmode="numeric" min="18" max="120" step="1" placeholder="" data-calc-field="age">
          </label>
          <label class="health-field">
            <span>Sex</span>
            <select data-calc-field="sex">
              <option value="">Choose…</option>
              <option value="true">Male</option>
              <option value="false">Female</option>
            </select>
          </label>
          <label class="health-field" data-calc-show="lipids">
            <span>Total cholesterol <small>mmol/L</small></span>
            <input type="number" inputmode="decimal" min="0.1" max="20" step="0.1" placeholder="" data-calc-field="total_cholesterol_mmol">
          </label>
          <label class="health-field" data-calc-show="lipids">
            <span>HDL cholesterol <small>mmol/L</small></span>
            <input type="number" inputmode="decimal" min="0.1" max="5" step="0.1" placeholder="" data-calc-field="hdl_cholesterol_mmol">
          </label>
          <label class="health-field" data-calc-show="bp">
            <span>Systolic blood pressure <small>mmHg</small></span>
            <input type="number" inputmode="numeric" min="70" max="260" step="1" placeholder="" data-calc-field="systolic_bp">
          </label>
          <label class="health-field" data-calc-show="framingham">
            <span>Blood pressure treated?</span>
            <select data-calc-field="bp_treated">
              <option value="">Choose…</option>
              <option value="true">Yes</option>
              <option value="false">No</option>
            </select>
          </label>
          <label class="health-field" data-calc-show="smoke">
            <span>Current smoker?</span>
            <select data-calc-field="current_smoker">
              <option value="">Choose…</option>
              <option value="true">Yes</option>
              <option value="false">No</option>
            </select>
          </label>
          <label class="health-field" data-calc-show="diabetes">
            <span>Diabetes?</span>
            <select data-calc-field="diabetic">
              <option value="">Choose…</option>
              <option value="true">Yes</option>
              <option value="false">No</option>
            </select>
          </label>
          <label class="health-field" data-calc-show="af">
            <span>Atrial fibrillation?</span>
            <select data-calc-field="atrial_fibrillation">
              <option value="">Choose…</option>
              <option value="true">Yes — non-valvular AF</option>
              <option value="false">No</option>
            </select>
          </label>
          <label class="health-field" data-calc-show="cha2ds2">
            <span>Heart failure?</span>
            <select data-calc-field="congestive_heart_failure">
              <option value="">Choose…</option>
              <option value="true">Yes</option>
              <option value="false">No</option>
            </select>
          </label>
          <label class="health-field" data-calc-show="cha2ds2">
            <span>Hypertension?</span>
            <select data-calc-field="hypertension">
              <option value="">Choose…</option>
              <option value="true">Yes</option>
              <option value="false">No</option>
            </select>
          </label>
          <label class="health-field" data-calc-show="cha2ds2">
            <span>Prior stroke or TIA?</span>
            <select data-calc-field="stroke_tia_history">
              <option value="">Choose…</option>
              <option value="true">Yes</option>
              <option value="false">No</option>
            </select>
          </label>
          <label class="health-field" data-calc-show="cha2ds2">
            <span>Vascular disease?</span>
            <select data-calc-field="vascular_disease">
              <option value="">Choose…</option>
              <option value="true">Yes</option>
              <option value="false">No</option>
            </select>
          </label>
          <label class="health-field" data-calc-show="score2">
            <span>European risk region</span>
            <select data-calc-field="risk_region">
              <option value="">Choose…</option>
              <option value="low">Low</option>
              <option value="moderate">Moderate</option>
              <option value="high">High</option>
              <option value="very_high">Very high</option>
            </select>
          </label>
        </div>
        <div class="health-form-footer">
          <p data-calc-gate>Calculate stays off until the form is complete. Offline, the native engine is held — no score is invented.</p>
          <button class="health-primary-button" type="button" data-calc-run disabled>Calculate</button>
        </div>
        <div class="health-status" role="status" aria-live="polite" data-calc-status></div>
        <pre class="health-calc-result" data-calc-result hidden></pre>
      </section>
    "#);

    wire(document, &root);
    refresh(&root);
    root
}

fn wire(document: &Document, root: &Element) {
    if let Ok(fields) = root.query_selector_all("[data-calc-field]") {
        for index in 0..fields.length() {
            let Some(node) = fields.get(index) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<Element>() else {
                continue;
            };
            let root_for_input = root.clone();
            let closure = Closure::wrap(Box::new(move |_event: Event| {
                refresh(&root_for_input);
            }) as Box<dyn FnMut(_)>);
            element
                .add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())
                .ok();
            element
                .add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
                .ok();
            closure.forget();
        }
    }

    if let Some(run) = root.query_selector("[data-calc-run]").ok().flatten() {
        let root_for_run = root.clone();
        let document = document.clone();
        let closure = Closure::wrap(Box::new(move |_event: Event| {
            run_calculator(&document, &root_for_run);
        }) as Box<dyn FnMut(_)>);
        run.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }
}

fn field_value(root: &Element, key: &str) -> String {
    let selector = format!("[data-calc-field=\"{key}\"]");
    let Some(element) = root.query_selector(&selector).ok().flatten() else {
        return String::new();
    };
    if let Ok(input) = element.clone().dyn_into::<HtmlInputElement>() {
        return input.value();
    }
    if let Ok(select) = element.dyn_into::<HtmlSelectElement>() {
        return select.value();
    }
    String::new()
}

fn read_draft(root: &Element) -> CalculatorDraft {
    CalculatorDraft {
        kind: CalculatorKind::parse(&field_value(root, "kind")),
        age: CalculatorDraft::parse_number(&field_value(root, "age")).and_then(|value| {
            if value.fract() == 0.0 && (0.0..=120.0).contains(&value) {
                Some(value as u8)
            } else {
                None
            }
        }),
        sex_male: CalculatorDraft::parse_bool(&field_value(root, "sex")),
        total_cholesterol_mmol: CalculatorDraft::parse_number(&field_value(
            root,
            "total_cholesterol_mmol",
        )),
        hdl_cholesterol_mmol: CalculatorDraft::parse_number(&field_value(
            root,
            "hdl_cholesterol_mmol",
        )),
        systolic_bp: CalculatorDraft::parse_number(&field_value(root, "systolic_bp")),
        bp_treated: CalculatorDraft::parse_bool(&field_value(root, "bp_treated")),
        current_smoker: CalculatorDraft::parse_bool(&field_value(root, "current_smoker")),
        diabetic: CalculatorDraft::parse_bool(&field_value(root, "diabetic")),
        congestive_heart_failure: CalculatorDraft::parse_bool(&field_value(
            root,
            "congestive_heart_failure",
        )),
        hypertension: CalculatorDraft::parse_bool(&field_value(root, "hypertension")),
        stroke_tia_history: CalculatorDraft::parse_bool(&field_value(root, "stroke_tia_history")),
        vascular_disease: CalculatorDraft::parse_bool(&field_value(root, "vascular_disease")),
        atrial_fibrillation: CalculatorDraft::parse_bool(&field_value(root, "atrial_fibrillation")),
        risk_region: {
            let value = field_value(root, "risk_region");
            if value.trim().is_empty() {
                None
            } else {
                Some(value)
            }
        },
    }
}

fn set_visible(root: &Element, token: &str, visible: bool) {
    if let Ok(nodes) = root.query_selector_all(&format!("[data-calc-show=\"{token}\"]")) {
        for index in 0..nodes.length() {
            if let Some(node) = nodes.get(index) {
                if let Ok(element) = node.dyn_into::<Element>() {
                    let _ =
                        element.set_attribute("style", if visible { "" } else { "display:none;" });
                }
            }
        }
    }
}

fn refresh(root: &Element) {
    let draft = read_draft(root);
    let kind = draft.kind;
    set_visible(
        root,
        "lipids",
        matches!(
            kind,
            Some(CalculatorKind::Framingham | CalculatorKind::Score2)
        ),
    );
    set_visible(
        root,
        "bp",
        matches!(
            kind,
            Some(CalculatorKind::Framingham | CalculatorKind::Score2)
        ),
    );
    set_visible(root, "framingham", kind == Some(CalculatorKind::Framingham));
    set_visible(
        root,
        "smoke",
        matches!(
            kind,
            Some(CalculatorKind::Framingham | CalculatorKind::Score2)
        ),
    );
    set_visible(
        root,
        "diabetes",
        matches!(
            kind,
            Some(CalculatorKind::Framingham | CalculatorKind::Cha2ds2Vasc)
        ),
    );
    set_visible(root, "af", kind == Some(CalculatorKind::Cha2ds2Vasc));
    set_visible(root, "cha2ds2", kind == Some(CalculatorKind::Cha2ds2Vasc));
    set_visible(root, "score2", kind == Some(CalculatorKind::Score2));

    if let Some(applicability) = root
        .query_selector("[data-calc-applicability]")
        .ok()
        .flatten()
    {
        applicability.set_text_content(Some(
            kind.map(CalculatorKind::applicability)
                .unwrap_or("Choose an algorithm to see its age band and required units."),
        ));
    }

    let complete = draft.incomplete_reason().is_none();
    let daemon = is_daemon_connected();
    if let Some(run) = root.query_selector("[data-calc-run]").ok().flatten() {
        if complete && daemon {
            run.remove_attribute("disabled").ok();
            run.set_attribute("aria-disabled", "false").ok();
            run.set_attribute("title", "Run the named native algorithm.")
                .ok();
        } else {
            run.set_attribute("disabled", "").ok();
            run.set_attribute("aria-disabled", "true").ok();
            let title = if !daemon {
                "Start the local QualiaDB daemon. Offline, no score is invented."
            } else {
                draft
                    .incomplete_reason()
                    .unwrap_or("Incomplete input cannot calculate.")
            };
            run.set_attribute("title", title).ok();
        }
    }

    if let Some(gate) = root.query_selector("[data-calc-gate]").ok().flatten() {
        let text = if !daemon {
            "The native engine is held until the local QualiaDB daemon is running. No score is invented offline."
        } else if let Some(reason) = draft.incomplete_reason() {
            reason
        } else {
            "Ready. The result will name the algorithm and is not a diagnosis."
        };
        gate.set_text_content(Some(text));
    }

    if let Some(status) = root.query_selector("[data-calc-status]").ok().flatten() {
        if !daemon {
            root.set_attribute("data-honesty", "unavailable").ok();
            status.set_text_content(Some(
                "Unavailable: start the local QualiaDB daemon. Incomplete or offline input cannot calculate.",
            ));
        } else {
            root.set_attribute("data-honesty", "running").ok();
            status.set_text_content(Some(""));
        }
    }
}

fn run_calculator(_document: &Document, root: &Element) {
    let draft = read_draft(root);
    let Ok((kind, args)) = draft.invoke_args() else {
        refresh(root);
        return;
    };
    if !is_daemon_connected() {
        refresh(root);
        return;
    }
    let status = root.query_selector("[data-calc-status]").ok().flatten();
    let result_el = root.query_selector("[data-calc-result]").ok().flatten();
    if let Some(status) = status.as_ref() {
        status.set_text_content(Some(&format!("Running {}…", kind.capability())));
    }
    if let Some(result_el) = result_el.as_ref() {
        result_el.set_attribute("hidden", "").ok();
        result_el.set_text_content(Some(""));
    }
    let root = root.clone();
    let capability = kind.capability();
    let version = kind.version();
    let label = kind.label();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_invoke(capability, args).await {
            Ok(response) if response.ok => {
                root.set_attribute("data-honesty", "live").ok();
                if let Some(status) = root.query_selector("[data-calc-status]").ok().flatten() {
                    status
                        .set_text_content(Some(&format!("{label} · {version} · {NOT_DIAGNOSIS}")));
                }
                if let Some(result_el) = root.query_selector("[data-calc-result]").ok().flatten() {
                    result_el.remove_attribute("hidden").ok();
                    result_el.set_text_content(Some(&response.value));
                }
            }
            Ok(response) => {
                root.set_attribute("data-honesty", "error").ok();
                if let Some(status) = root.query_selector("[data-calc-status]").ok().flatten() {
                    status.set_text_content(Some(
                        response
                            .diagnostic
                            .as_deref()
                            .unwrap_or("Native invoke failed. No score is shown."),
                    ));
                }
            }
            Err(error) => {
                root.set_attribute("data-honesty", "error").ok();
                if let Some(status) = root.query_selector("[data-calc-status]").ok().flatten() {
                    status.set_text_content(Some(&error));
                }
            }
        }
    });
}
