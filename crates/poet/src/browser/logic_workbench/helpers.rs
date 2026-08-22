//! Shared helpers for the logic workbench panels.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlInputElement, HtmlTextAreaElement};

pub(super) fn make_textarea(document: &Document, id: &str, value: &str, height: &str) -> Element {
    let ta = document.create_element("textarea").unwrap();
    ta.set_id(id);
    let ta_el: HtmlTextAreaElement = ta.clone().dyn_into().unwrap();
    ta_el.set_value(value);
    let html: HtmlElement = ta.clone().dyn_into().unwrap();
    html.style().set_css_text(&format!(
        "width: 100%; box-sizing: border-box; height: {}; \
         background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 10px; \
         font-family: var(--font-mono); font-size: 11px; color: var(--accent-cyan); \
         line-height: 1.5; resize: vertical;",
        height
    ));
    ta
}

pub(super) fn make_button(document: &Document, id: &str, label: &str, primary: bool) -> Element {
    let btn = document.create_element("button").unwrap();
    btn.set_id(id);
    btn.set_text_content(Some(label));
    let el: HtmlElement = btn.clone().dyn_into().unwrap();
    if primary {
        el.style().set_css_text(
            "padding: 8px 16px; background: var(--accent-violet); color: #fff; \
             border: 1px solid var(--accent-violet); border-radius: var(--radius-xs); \
             font-family: var(--font-mono); font-size: 11px; font-weight: 700; cursor: pointer;",
        );
    } else {
        el.style().set_css_text(
            "padding: 8px 16px; background: var(--surface-panel); color: var(--text-secondary); \
             border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
             font-family: var(--font-mono); font-size: 11px; cursor: pointer;",
        );
    }
    btn
}

pub(super) fn make_results_area(document: &Document, id: &str, placeholder: &str) -> Element {
    let results = document.create_element("div").unwrap();
    results.set_id(id);
    let r_el: HtmlElement = results.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "flex: 1; overflow-y: auto; border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; min-height: 80px; \
         font-family: var(--font-mono); font-size: 11px; color: var(--text-muted);",
    );
    results.set_text_content(Some(placeholder));
    results
}

pub(super) fn make_section_label(document: &Document, text: &str) -> Element {
    let lbl = document.create_element("div").unwrap();
    let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
    l_el.style().set_css_text(
        "font-size: 10px; font-weight: 700; color: var(--text-secondary); \
         text-transform: uppercase; letter-spacing: 0.3px;",
    );
    lbl.set_text_content(Some(text));
    lbl
}

pub(super) fn make_tool_panel(document: &Document, tool_id: &str, visible: bool) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("logic-tool-panel");
    panel.set_attribute("data-tool", tool_id).unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style().set_css_text(&format!(
        "display: {}; flex-direction: column; gap: 12px;",
        if visible { "flex" } else { "none" }
    ));
    panel
}

pub(super) fn make_select(document: &Document, id: &str, options: &[(&str, &str)]) -> Element {
    let sel = document.create_element("select").unwrap();
    sel.set_id(id);
    let s_el: HtmlElement = sel.clone().dyn_into().unwrap();
    s_el.style().set_css_text(
        "padding: 6px 10px; background: var(--canvas-bg); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; color: var(--text-primary);",
    );
    for (key, display) in options {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", key).unwrap();
        opt.set_text_content(Some(display));
        sel.append_child(&opt).unwrap();
    }
    sel
}

pub(super) fn make_text_input(document: &Document, id: &str, placeholder: &str) -> Element {
    let input = document.create_element("input").unwrap();
    input.set_id(id);
    input.set_attribute("type", "text").unwrap();
    input.set_attribute("placeholder", placeholder).unwrap();
    let el: HtmlInputElement = input.clone().dyn_into().unwrap();
    el.style().set_css_text(
        "flex: 1; padding: 6px 10px; background: var(--canvas-bg); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; color: var(--text-primary);",
    );
    input
}

pub(super) fn show_logic_notification(document: &Document, message: &str) {
    let notif = document.create_element("div").unwrap();
    let n_el: HtmlElement = notif.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "position: fixed; bottom: 40px; right: 16px; \
         background: var(--surface-panel-elevated); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         padding: 10px 14px; font-size: 12px; color: var(--text-primary); \
         box-shadow: var(--shadow-lg); z-index: 10002; max-width: 320px;",
    );
    notif.set_text_content(Some(&format!("\u{1F9E0} {}", message)));
    if let Some(body) = document.body() {
        body.append_child(&notif).unwrap();
    }
    let notif_clone = notif.clone();
    let timeout = Closure::wrap(Box::new(move || {
        notif_clone.remove();
    }) as Box<dyn FnMut()>);
    crate::browser::interactions::set_timeout(timeout.as_ref().unchecked_ref(), 2500);
    timeout.forget();
}

pub(super) fn show_mock_results(document: &Document, results_id: &str, tool_name: &str) {
    let results = match document.get_element_by_id(results_id) {
        Some(r) => r,
        None => return,
    };
    results.set_inner_html("");

    let mut html = String::new();
    html.push_str(&format!(
        "<div style=\"padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
         font-size: 9px; color: var(--text-muted); margin-bottom: 4px;\">\
         Mock {} evaluation \u{2014} engine wiring pending (MCP evaluate_modality)</div>",
        tool_name
    ));

    for i in 0..5 {
        html.push_str(&format!(
            "<div style=\"padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
             display: flex; gap: 8px;\">\
            <span style=\"color: var(--accent-violet); font-size: 10px;\">#{:02}</span>\
            <span style=\"color: var(--text-primary);\">{} derivation step {}</span>\
            <span style=\"color: var(--text-muted); font-size: 9px; margin-left: auto;\">\
             confidence: {:.2}</span></div>",
            i + 1,
            tool_name,
            i + 1,
            1.0 - (i as f64 * 0.15),
        ));
    }

    results.set_inner_html(&html);
}
