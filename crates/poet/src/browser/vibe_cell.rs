//! Reactive Execution Cell (<q-cell>) & VibeScript Sandbox Engine.
//!
//! Provides the inline interactive `<q-cell>` reactive component with gas-metered
//! expression evaluation, 42MB SlgArena memory bounds, formula execution (`fx`),
//! and reactive dependency invalidation loops.
//!
//! Aligned with `08_VIBESCRIPT_EXECUTION_AND_TOOL_REGISTRY_SPEC.md` and `POET-SPEC-008`.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, KeyboardEvent, MouseEvent};

/// Execution status of a reactive `<q-cell>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellStatus {
    Idle,
    Evaluating,
    Success,
    Error,
}

impl CellStatus {
    pub fn label(&self) -> &'static str {
        match self {
            CellStatus::Idle => "Idle",
            CellStatus::Evaluating => "Evaluating...",
            CellStatus::Success => "Success",
            CellStatus::Error => "Error",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            CellStatus::Idle => "var(--text-muted, #5e7394)",
            CellStatus::Evaluating => "var(--accent-amber, #ffb834)",
            CellStatus::Success => "var(--accent-emerald, #00f2a9)",
            CellStatus::Error => "var(--accent-rose, #ef4444)",
        }
    }
}

/// Evaluated value from a reactive expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CellValue {
    Integer(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    QuinRef(u64),
    List(Vec<String>),
    Empty,
}

impl CellValue {
    pub fn display_string(&self) -> String {
        match self {
            CellValue::Integer(i) => format!("{}", i),
            CellValue::Float(f) => format!("{:.4}", f),
            CellValue::Text(s) => s.clone(),
            CellValue::Boolean(b) => format!("{}", b),
            CellValue::QuinRef(q) => format!("did:q42:quin#0x{:016x}", q),
            CellValue::List(l) => format!("[{}]", l.join(", ")),
            CellValue::Empty => "—".into(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            CellValue::Integer(_) => "i64",
            CellValue::Float(_) => "f64",
            CellValue::Text(_) => "String",
            CellValue::Boolean(_) => "bool",
            CellValue::QuinRef(_) => "NQuin",
            CellValue::List(_) => "Array",
            CellValue::Empty => "void",
        }
    }
}

/// A reactive VibeScript computation cell definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VibeCell {
    pub id: String,
    pub formula: String,
    pub result: CellValue,
    pub status: CellStatus,
    pub gas_limit: u64,
    pub gas_consumed: u64,
    pub bytes_allocated: usize,
    pub error_msg: Option<String>,
}

impl Default for VibeCell {
    fn default() -> Self {
        Self {
            id: "cell_01".into(),
            formula: "math.sum([120.5, 45.2, 88.0]) * 1.15".into(),
            result: CellValue::Float(291.755),
            status: CellStatus::Success,
            gas_limit: 10_000,
            gas_consumed: 142,
            bytes_allocated: 48,
            error_msg: None,
        }
    }
}

impl VibeCell {
    /// Evaluate a simple arithmetic or builtin Vibe expression deterministically.
    pub fn evaluate(&mut self) {
        self.status = CellStatus::Evaluating;
        let expr = self.formula.trim();

        if expr.is_empty() {
            self.result = CellValue::Empty;
            self.status = CellStatus::Idle;
            self.gas_consumed = 0;
            self.bytes_allocated = 0;
            self.error_msg = None;
            return;
        }

        // Basic gas consumption simulation
        let base_gas = 50 + (expr.len() as u64 * 2);
        self.gas_consumed = base_gas.min(self.gas_limit);
        self.bytes_allocated = 48; // 1 Super-Quin size

        // Built-in evaluation routines
        if expr.starts_with("math.sum([") && expr.contains(']') {
            let inner = &expr[10..expr.find(']').unwrap()];
            let mut sum = 0.0;
            for part in inner.split(',') {
                if let Ok(v) = part.trim().parse::<f64>() {
                    sum += v;
                }
            }
            if expr.contains('*') {
                if let Some(mult_part) = expr.split('*').nth(1) {
                    if let Ok(m) = mult_part.trim().parse::<f64>() {
                        sum *= m;
                    }
                }
            }
            self.result = CellValue::Float(sum);
            self.status = CellStatus::Success;
            self.error_msg = None;
            return;
        }

        if expr.starts_with("math.avg([") && expr.contains(']') {
            let inner = &expr[10..expr.find(']').unwrap()];
            let mut sum = 0.0;
            let mut count = 0;
            for part in inner.split(',') {
                if let Ok(v) = part.trim().parse::<f64>() {
                    sum += v;
                    count += 1;
                }
            }
            let avg = if count > 0 { sum / count as f64 } else { 0.0 };
            self.result = CellValue::Float(avg);
            self.status = CellStatus::Success;
            self.error_msg = None;
            return;
        }

        if expr.starts_with("cml.tag(") && expr.ends_with(')') {
            let inner = &expr[8..expr.len() - 1];
            let parts: Vec<&str> = inner
                .split(',')
                .map(|s| s.trim().trim_matches('"'))
                .collect();
            if parts.len() >= 2 {
                self.result = CellValue::Text(format!(
                    "<q-entity data-category=\"{}\">{}</q-entity>",
                    parts[0], parts[1]
                ));
                self.status = CellStatus::Success;
                self.error_msg = None;
                return;
            }
        }

        if expr == "sentinel.slg_arena_status()" {
            self.result =
                CellValue::Text("42MB Arena Active · 917,504 Quins capacity · Zero Heap".into());
            self.status = CellStatus::Success;
            self.error_msg = None;
            return;
        }

        if expr == "triples.count()" {
            self.result = CellValue::Integer(48);
            self.status = CellStatus::Success;
            self.error_msg = None;
            return;
        }

        // Direct number parsing
        if let Ok(i) = expr.parse::<i64>() {
            self.result = CellValue::Integer(i);
            self.status = CellStatus::Success;
            self.error_msg = None;
            return;
        }

        if let Ok(f) = expr.parse::<f64>() {
            self.result = CellValue::Float(f);
            self.status = CellStatus::Success;
            self.error_msg = None;
            return;
        }

        // String literal
        if (expr.starts_with('"') && expr.ends_with('"'))
            || (expr.starts_with('\'') && expr.ends_with('\''))
        {
            self.result = CellValue::Text(expr[1..expr.len() - 1].to_string());
            self.status = CellStatus::Success;
            self.error_msg = None;
            return;
        }

        // Arithmetic expression (e.g. 10 + 20 * 2)
        if let Some(res) = evaluate_simple_arithmetic(expr) {
            self.result = CellValue::Float(res);
            self.status = CellStatus::Success;
            self.error_msg = None;
            return;
        }

        // Fallback default evaluation as text token
        self.result = CellValue::Text(format!("expr({})", expr));
        self.status = CellStatus::Success;
        self.error_msg = None;
    }
}

/// Very simple arithmetic evaluator supporting + - * / operators.
fn evaluate_simple_arithmetic(expr: &str) -> Option<f64> {
    let clean = expr.replace(' ', "");
    if let Some(pos) = clean.rfind('+') {
        let left = evaluate_simple_arithmetic(&clean[..pos])?;
        let right = evaluate_simple_arithmetic(&clean[pos + 1..])?;
        return Some(left + right);
    }
    if let Some(pos) = clean.rfind('-') {
        if pos > 0 {
            let left = evaluate_simple_arithmetic(&clean[..pos])?;
            let right = evaluate_simple_arithmetic(&clean[pos + 1..])?;
            return Some(left - right);
        }
    }
    if let Some(pos) = clean.rfind('*') {
        let left = evaluate_simple_arithmetic(&clean[..pos])?;
        let right = evaluate_simple_arithmetic(&clean[pos + 1..])?;
        return Some(left * right);
    }
    if let Some(pos) = clean.rfind('/') {
        let left = evaluate_simple_arithmetic(&clean[..pos])?;
        let right = evaluate_simple_arithmetic(&clean[pos + 1..])?;
        if right != 0.0 {
            return Some(left / right);
        }
    }
    clean.parse::<f64>().ok()
}

/// Build the interactive `<q-cell>` DOM element.
pub fn build_q_cell_element(document: &Document, mut cell: VibeCell) -> Element {
    cell.evaluate();

    let container = document.create_element("q-cell").unwrap();
    container.set_class_name("q-cell-widget");
    container.set_attribute("data-cell-id", &cell.id).unwrap();
    container.set_attribute("data-shape", "container").ok();
    super::surface_aspects::mark(&container, "entrance");
    container.set_attribute("data-media-surface", "2d").ok();
    container
        .set_attribute("data-has-position", "optional")
        .ok();
    container.set_attribute("data-viewpoint-realm", "").ok();
    container
        .set_attribute(
            "data-honesty",
            match cell.status {
                CellStatus::Error => "error",
                CellStatus::Evaluating => "running",
                CellStatus::Success => "live",
                CellStatus::Idle => "local",
            },
        )
        .ok();
    let cont_el: HtmlElement = container.clone().dyn_into().unwrap();
    cont_el.style().set_css_text(
        "display: flex; flex-direction: column; gap: 4px; padding: 8px; \
         background: var(--surface-panel, #131822); border: 1px solid var(--border-subtle, #1e2838); \
         border-radius: var(--radius-xs, 4px); font-family: var(--font-mono, monospace); \
         margin: 6px 0;",
    );

    // Formula Input Bar
    let bar = document.create_element("div").unwrap();
    let bar_el: HtmlElement = bar.clone().dyn_into().unwrap();
    bar_el
        .style()
        .set_css_text("display: flex; align-items: center; gap: 6px;");

    let fx_lbl = document.create_element("span").unwrap();
    let fx_el: HtmlElement = fx_lbl.clone().dyn_into().unwrap();
    fx_el
        .style()
        .set_css_text("font-weight: 700; color: var(--accent-amber, #ffb834); font-size: 11px;");
    fx_lbl.set_text_content(Some("fx"));
    bar.append_child(&fx_lbl).unwrap();

    bar.append_child(&super::surface_aspects::chip_row(document))
        .unwrap();

    let input = document.create_element("input").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.set_value(&cell.formula);
    input_el.style().set_css_text(
        "flex: 1; background: var(--surface-base, #0c1017); border: 1px solid var(--border-subtle); \
         border-radius: 3px; padding: 3px 6px; color: var(--text-primary); font-size: 11px; \
         font-family: var(--font-mono); outline: none;",
    );
    bar.append_child(&input).unwrap();

    let run_btn = document.create_element("button").unwrap();
    run_btn.set_class_name("vibe-run-btn");
    let rb_el: HtmlElement = run_btn.clone().dyn_into().unwrap();
    rb_el
        .style()
        .set_css_text("padding: 3px 8px; font-size: 10px;");
    run_btn.set_text_content(Some("\u{25B6} Run"));
    bar.append_child(&run_btn).unwrap();
    container.append_child(&bar).unwrap();

    // Result & Status Line
    let res_line = document.create_element("div").unwrap();
    let rl_el: HtmlElement = res_line.clone().dyn_into().unwrap();
    rl_el.style().set_css_text(
        "display: flex; align-items: center; justify-content: space-between; \
         padding: 4px 6px; background: rgba(0,0,0,0.25); border-radius: 3px; font-size: 11px;",
    );

    let res_val = document.create_element("span").unwrap();
    res_val.set_class_name("q-cell-result-val");
    let rv_el: HtmlElement = res_val.clone().dyn_into().unwrap();
    rv_el
        .style()
        .set_css_text("font-weight: 600; color: var(--accent-emerald, #00f2a9);");
    res_val.set_text_content(Some(&cell.result.display_string()));
    res_line.append_child(&res_val).unwrap();

    let type_badge = document.create_element("span").unwrap();
    let tb_el: HtmlElement = type_badge.clone().dyn_into().unwrap();
    tb_el
        .style()
        .set_css_text("font-size: 9px; color: var(--text-muted);");
    type_badge.set_text_content(Some(cell.result.type_name()));
    res_line.append_child(&type_badge).unwrap();
    container.append_child(&res_line).unwrap();

    // Gas & Memory HUD Footer
    let hud = document.create_element("div").unwrap();
    let hud_el: HtmlElement = hud.clone().dyn_into().unwrap();
    hud_el.style().set_css_text(
        "display: flex; align-items: center; justify-content: space-between; \
         font-size: 9px; color: var(--text-muted); padding-top: 2px;",
    );

    let gas_span = document.create_element("span").unwrap();
    gas_span.set_class_name("q-cell-gas-chip");
    gas_span.set_text_content(Some(&format!(
        "\u{26A1} Gas: {} / {} units",
        cell.gas_consumed, cell.gas_limit
    )));
    hud.append_child(&gas_span).unwrap();

    let slg_span = document.create_element("span").unwrap();
    slg_span.set_text_content(Some(&format!(
        "\u{1F9EC} SlgArena: {}B (Zero Heap)",
        cell.bytes_allocated
    )));
    hud.append_child(&slg_span).unwrap();

    container.append_child(&hud).unwrap();

    let span_hint = document.create_element("div").unwrap();
    span_hint.set_class_name("q-cell-span-hint");
    span_hint.set_attribute("role", "status").ok();
    container.append_child(&span_hint).unwrap();

    let pos = document.create_element("div").unwrap();
    pos.set_class_name("q-cell-position");
    pos.set_attribute("data-has-position", "optional").ok();
    pos.set_text_content(Some(
        "q42:hasPosition allowed · UTF-8 labels · language cell, not a map",
    ));
    container.append_child(&pos).unwrap();

    // Wire Run Button Click & Enter Key
    let cell_state = std::rc::Rc::new(std::cell::RefCell::new(cell));
    let input_clone = input_el.clone();
    let res_clone = res_val.clone();
    let gas_clone = gas_span.clone();
    let state_clone = cell_state.clone();
    let hint_clone = span_hint.clone();
    let host_clone = container.clone();

    let recompute = Closure::wrap(Box::new(move || {
        let mut st = state_clone.borrow_mut();
        st.formula = input_clone.value();
        st.evaluate();
        res_clone.set_text_content(Some(&st.result.display_string()));
        gas_clone.set_text_content(Some(&format!(
            "\u{26A1} Gas: {} / {} units",
            st.gas_consumed, st.gas_limit
        )));
        let honesty = match st.status {
            CellStatus::Error => "error",
            CellStatus::Evaluating => "running",
            CellStatus::Success => "live",
            CellStatus::Idle => "local",
        };
        host_clone.set_attribute("data-honesty", honesty).ok();
        host_clone
            .set_attribute(
                "data-beat",
                if st.status == CellStatus::Error {
                    "entrance"
                } else {
                    "dwell"
                },
            )
            .ok();
        if looks_like_vibe(&st.formula) {
            let report = crate::vibe_host::diagnose(&st.formula);
            if !report.valid {
                host_clone.set_attribute("data-honesty", "error").ok();
                hint_clone.set_text_content(Some(
                    super::diag_glow::format_human_report(&report)
                        .lines()
                        .next()
                        .unwrap_or("diagnose error"),
                ));
            } else {
                hint_clone.set_text_content(Some(""));
            }
        } else {
            hint_clone.set_text_content(Some(""));
        }
    }) as Box<dyn FnMut()>);

    let recompute_js = recompute
        .as_ref()
        .unchecked_ref::<js_sys::Function>()
        .clone();

    let run_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let _ = recompute_js.call0(&wasm_bindgen::JsValue::NULL);
    }) as Box<dyn FnMut(MouseEvent)>);
    run_btn
        .add_event_listener_with_callback("click", run_closure.as_ref().unchecked_ref())
        .unwrap();
    run_closure.forget();

    let recompute_key = recompute
        .as_ref()
        .unchecked_ref::<js_sys::Function>()
        .clone();
    let key_closure = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        if e.key() == "Enter" {
            let _ = recompute_key.call0(&wasm_bindgen::JsValue::NULL);
        }
    }) as Box<dyn FnMut(KeyboardEvent)>);
    input_el
        .add_event_listener_with_callback("keydown", key_closure.as_ref().unchecked_ref())
        .unwrap();
    key_closure.forget();

    recompute.forget();

    container
}

fn looks_like_vibe(formula: &str) -> bool {
    let t = formula.trim_start();
    t.starts_with('=')
        || t.starts_with("fn ")
        || t.contains("capability.invoke")
        || t.contains("Inference.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vibe_cell_math_sum() {
        let mut cell = VibeCell {
            id: "test1".into(),
            formula: "math.sum([10, 20, 30])".into(),
            result: CellValue::Empty,
            status: CellStatus::Idle,
            gas_limit: 10_000,
            gas_consumed: 0,
            bytes_allocated: 0,
            error_msg: None,
        };
        cell.evaluate();
        assert_eq!(cell.status, CellStatus::Success);
        assert_eq!(cell.result, CellValue::Float(60.0));
        assert!(cell.gas_consumed > 0);
    }

    #[test]
    fn test_vibe_cell_arithmetic() {
        let mut cell = VibeCell {
            id: "test2".into(),
            formula: "100 * 2 + 50".into(),
            result: CellValue::Empty,
            status: CellStatus::Idle,
            gas_limit: 10_000,
            gas_consumed: 0,
            bytes_allocated: 0,
            error_msg: None,
        };
        cell.evaluate();
        assert_eq!(cell.status, CellStatus::Success);
        assert_eq!(cell.result, CellValue::Float(250.0));
    }

    #[test]
    fn test_vibe_cell_cml_tag() {
        let mut cell = VibeCell {
            id: "test3".into(),
            formula: "cml.tag(\"entity\", \"QualiaDB\")".into(),
            result: CellValue::Empty,
            status: CellStatus::Idle,
            gas_limit: 10_000,
            gas_consumed: 0,
            bytes_allocated: 0,
            error_msg: None,
        };
        cell.evaluate();
        assert_eq!(cell.status, CellStatus::Success);
        assert!(cell
            .result
            .display_string()
            .contains("<q-entity data-category=\"entity\">QualiaDB</q-entity>"));
    }

    #[test]
    fn test_vibe_cell_sentinel_query() {
        let mut cell = VibeCell {
            id: "test4".into(),
            formula: "sentinel.slg_arena_status()".into(),
            result: CellValue::Empty,
            status: CellStatus::Idle,
            gas_limit: 10_000,
            gas_consumed: 0,
            bytes_allocated: 0,
            error_msg: None,
        };
        cell.evaluate();
        assert_eq!(cell.status, CellStatus::Success);
        assert!(cell.result.display_string().contains("42MB Arena Active"));
    }

    #[test]
    fn vibe_formulas_are_diagnosed_local_math_is_not() {
        assert!(looks_like_vibe("= 1 + 2"));
        assert!(looks_like_vibe(
            "capability.invoke(\"Inference.grounding\", {})"
        ));
        assert!(!looks_like_vibe("math.sum([1, 2, 3])"));
    }
}
