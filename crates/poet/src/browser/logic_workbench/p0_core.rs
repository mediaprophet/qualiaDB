//! P0 core panels: deontic editor, N3 logic studio, SHACL validator.

use super::helpers::{
    make_button, make_results_area, make_section_label, make_select, make_text_input,
    make_textarea, make_tool_panel, show_logic_notification, show_mock_results,
};
use super::{DEONTIC_OPERATORS, SHACL_CONSTRAINTS};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, Element, HtmlElement, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement,
    MouseEvent,
};

pub(super) fn build_deontic_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "deontic", true);

    panel
        .append_child(&make_section_label(
            document,
            "Deontic Rule Editor \u{2014} OBLIGATE / PERMIT / FORBID / WAIVE",
        ))
        .unwrap();

    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");

    row.append_child(&make_select(
        document,
        "deontic-operator",
        DEONTIC_OPERATORS,
    ))
    .unwrap();

    let target = make_text_input(
        document,
        "deontic-target",
        "Rule target (e.g. agent:payTax)",
    );
    let t_el: HtmlElement = target.clone().dyn_into().unwrap();
    t_el.style().set_css_text("flex: 1; padding: 6px 10px; background: var(--canvas-bg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); font-family: var(--font-mono); font-size: 11px; color: var(--text-primary);");
    row.append_child(&target).unwrap();

    panel.append_child(&row).unwrap();

    panel
        .append_child(&make_section_label(document, "Rule body (N3 conditions):"))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "deontic-body",
            "# Example: obligate agent to pay tax on income\n{\n  ?agent a agency:Agent .\n  ?agent agency:hasIncome ?income .\n  ?income agency:amount ?amount .\n  FILTER(?amount > 0)\n}\n=>\n{\n  ?agent ont:OBLIGATED_TO ont:payTax .\n}.",
            "120px",
        ))
        .unwrap();

    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "deontic-evaluate",
            "\u{2696} Evaluate Norms",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "deontic-compile",
            "\u{1F4BE} Compile to Contract",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();

    panel
        .append_child(&make_results_area(
            document,
            "deontic-results",
            "Click \"Evaluate Norms\" to check rules against the live graph (mock).",
        ))
        .unwrap();

    panel
}

pub(super) fn wire_deontic_editor(document: &Document) {
    if let Some(btn) = document.get_element_by_id("deontic-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "deontic-results", "deontic");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("deontic-compile") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_logic_notification(
                &doc,
                "Deontic contract compiled (mock \u{2014} engine wiring pending)",
            );
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_n3_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "n3", false);

    panel
        .append_child(&make_section_label(
            document,
            "N3 Logic Studio \u{2014} Notation3 rule authoring + evaluation",
        ))
        .unwrap();

    panel
        .append_child(&make_textarea(
            document,
            "n3-editor",
            "# Notation3 rules example\n@prefix log: <http://www.w3.org/2000/10/swap/log#>.\n@prefix var: <http://www.w3.org/2000/10/swap/var#>.\n\n# Rule: if someone is a contributor and has effort, they have obligation\n{\n  var:person a ont:Contributor .\n  var:person ont:hasEffort var:effort .\n}\n=>\n{\n  var:person ont:hasObligation var:effort .\n}.",
            "200px",
        ))
        .unwrap();

    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "n3-evaluate",
            "\u{1F9E9} Evaluate Rules",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "n3-parse",
            "\u{2699} Parse Only",
            false,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "n3-save",
            "\u{1F4BE} Save Ruleset",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();

    panel
        .append_child(&make_results_area(
            document,
            "n3-results",
            "Click \"Evaluate Rules\" to run N3 inference (mock).",
        ))
        .unwrap();

    panel
}

pub(super) fn wire_n3_editor(document: &Document) {
    if let Some(btn) = document.get_element_by_id("n3-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "n3-results", "N3");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("n3-parse") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_logic_notification(
                &doc,
                "N3 parsed successfully (mock \u{2014} no syntax errors)",
            );
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("n3-save") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_logic_notification(&doc, "N3 ruleset saved to localStorage (mock)");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_shacl_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "shacl", false);

    panel
        .append_child(&make_section_label(
            document,
            "SHACL Validator \u{2014} shape constraint authoring + validation",
        ))
        .unwrap();

    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center;");
    row.append_child(&{
        let lbl = document.create_element("span").unwrap();
        let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        lbl.set_text_content(Some("Target class:"));
        lbl.into()
    })
    .unwrap();
    row.append_child(&make_text_input(
        document,
        "shacl-target-class",
        "e.g. ont:Project",
    ))
    .unwrap();
    panel.append_child(&row).unwrap();

    let row2 = document.create_element("div").unwrap();
    let r2_el: HtmlElement = row2.clone().dyn_into().unwrap();
    r2_el
        .style()
        .set_css_text("display: flex; gap: 8px; align-items: center;");
    row2.append_child(&make_select(
        document,
        "shacl-constraint-type",
        SHACL_CONSTRAINTS,
    ))
    .unwrap();
    row2.append_child(&make_text_input(
        document,
        "shacl-constraint-value",
        "Constraint value",
    ))
    .unwrap();
    let add_btn = make_button(document, "shacl-add-constraint", "+ Add", false);
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text("padding: 6px 12px; background: var(--surface-panel); border: 1px dashed var(--border-subtle); border-radius: var(--radius-xs); color: var(--accent-emerald); font-family: var(--font-mono); font-size: 10px; cursor: pointer;");
    row2.append_child(&add_btn).unwrap();
    panel.append_child(&row2).unwrap();

    let list = document.create_element("div").unwrap();
    list.set_id("shacl-constraint-list");
    let l_el: HtmlElement = list.clone().dyn_into().unwrap();
    l_el.style().set_css_text(
        "border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         padding: 8px; min-height: 60px; background: var(--surface-panel); \
         display: flex; flex-direction: column; gap: 4px; \
         font-family: var(--font-mono); font-size: 10px;",
    );
    list.set_text_content(Some("No constraints added yet."));
    panel.append_child(&list).unwrap();

    panel
        .append_child(&make_section_label(
            document,
            "Generated SHACL shape (Turtle):",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "shacl-preview",
            "@prefix sh: <http://www.w3.org/ns/shacl#>.\n@prefix ont: <http://qualia.org/ontology#>.\n\nont:ProjectShape\n  a sh:NodeShape ;\n  sh:targetClass ont:Project ;\n  # Add constraints above to generate shape\n  .",
            "120px",
        ))
        .unwrap();

    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "shacl-validate",
            "\u{2705} Validate Graph",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "shacl-generate",
            "\u{2699} Regenerate Shape",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();

    panel
        .append_child(&make_results_area(
            document,
            "shacl-results",
            "Click \"Validate Graph\" to run SHACL validation (mock).",
        ))
        .unwrap();

    panel
}

pub(super) fn wire_shacl_validator(document: &Document) {
    if let Some(add_btn) = document.get_element_by_id("shacl-add-constraint") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let ctype = doc
                .get_element_by_id("shacl-constraint-type")
                .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
                .map(|s| s.value())
                .unwrap_or_default();
            let cval = doc
                .get_element_by_id("shacl-constraint-value")
                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                .map(|i| i.value())
                .unwrap_or_default();
            if cval.trim().is_empty() {
                return;
            }
            let list = match doc.get_element_by_id("shacl-constraint-list") {
                Some(l) => l,
                None => return,
            };
            if list
                .text_content()
                .unwrap_or_default()
                .contains("No constraints")
            {
                list.set_text_content(Some(""));
            }
            let item = doc.create_element("div").unwrap();
            let i_el: HtmlElement = item.clone().dyn_into().unwrap();
            i_el.style().set_css_text(
                "display: flex; gap: 8px; align-items: center; padding: 4px 6px; \
                 background: var(--surface-panel-elevated); border-radius: var(--radius-xs);",
            );
            item.set_text_content(Some(&format!("{} \u{2192} {}", ctype, cval)));
            list.append_child(&item).unwrap();
        }) as Box<dyn FnMut(MouseEvent)>);
        add_btn
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    if let Some(btn) = document.get_element_by_id("shacl-validate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "shacl-results", "SHACL");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("shacl-generate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let list = match doc.get_element_by_id("shacl-constraint-list") {
                Some(l) => l,
                None => return,
            };
            let target = doc
                .get_element_by_id("shacl-target-class")
                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                .map(|i| i.value())
                .unwrap_or_else(|| "ont:Project".to_string());

            let mut shape = format!(
                "@prefix sh: <http://www.w3.org/ns/shacl#>.\n@prefix ont: <http://qualia.org/ontology#>.\n\n{}Shape\n  a sh:NodeShape ;\n  sh:targetClass {} ;\n",
                target.replace(':', ""),
                target
            );

            let items = list.query_selector_all("div").unwrap();
            for i in 0..items.length() {
                let item = items.get(i).unwrap();
                let text = item.text_content().unwrap_or_default();
                if let Some((ctype, cval)) = text.split_once(" \u{2192} ") {
                    shape.push_str(&format!("  {} {} ;\n", ctype, cval));
                }
            }
            shape.push_str("  .");

            if let Some(preview) = doc.get_element_by_id("shacl-preview") {
                let ta: HtmlTextAreaElement = preview.dyn_into().unwrap();
                ta.set_value(&shape);
            }
            show_logic_notification(&doc, "SHACL shape regenerated");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
