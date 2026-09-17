//! Visual SPARQL query builder: prefixes, triple patterns, and generated preview.

use super::catalog::COMMON_PREDICATES;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlInputElement, HtmlTextAreaElement, MouseEvent};

pub(super) fn build_query_builder_panel(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("search-mode-panel");
    panel.set_attribute("data-mode", "builder").unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("display: none; flex-direction: column; gap: 12px;");

    // Prefix declarations
    let prefix_section = document.create_element("div").unwrap();
    let ps_el: HtmlElement = prefix_section.clone().dyn_into().unwrap();
    ps_el
        .style()
        .set_css_text("display: flex; flex-direction: column; gap: 4px;");

    let ps_label = document.create_element("div").unwrap();
    ps_label.set_text_content(Some("PREFIX declarations:"));
    let psl_el: HtmlElement = ps_label.clone().dyn_into().unwrap();
    psl_el.style().set_css_text("font-size: 10px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase;");
    prefix_section.append_child(&ps_label).unwrap();

    let prefix_text = document.create_element("textarea").unwrap();
    prefix_text.set_id("builder-prefixes");
    let pt_el: HtmlTextAreaElement = prefix_text.clone().dyn_into().unwrap();
    pt_el.set_value(
        "PREFIX ont: <http://qualia.org/ontology#>\n\
         PREFIX doc: <http://qualia.org/document#>\n\
         PREFIX prov: <http://qualia.org/provenance#>\n\
         PREFIX agency: <http://qualia.org/agency#>\n\
         PREFIX epi: <http://qualia.org/epistemics#>\n\
         PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n\
         PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>",
    );
    let pt_html: HtmlElement = prefix_text.clone().dyn_into().unwrap();
    pt_html.style().set_css_text(
        "width: 100%; box-sizing: border-box; height: 80px; \
         background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-secondary); \
         resize: vertical;",
    );
    prefix_section.append_child(&prefix_text).unwrap();
    panel.append_child(&prefix_section).unwrap();

    // Triple patterns
    let patterns_label = document.create_element("div").unwrap();
    patterns_label.set_text_content(Some("Triple Patterns:"));
    let pl_el: HtmlElement = patterns_label.clone().dyn_into().unwrap();
    pl_el.style().set_css_text("font-size: 10px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase;");
    panel.append_child(&patterns_label).unwrap();

    let patterns_container = document.create_element("div").unwrap();
    patterns_container.set_id("builder-patterns");
    let pc_el: HtmlElement = patterns_container.clone().dyn_into().unwrap();
    pc_el.style().set_css_text(
        "display: flex; flex-direction: column; gap: 6px; \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         padding: 8px; min-height: 60px; background: var(--surface-panel);",
    );

    // Add one default pattern row
    patterns_container
        .append_child(&build_pattern_row(document, 0))
        .unwrap();

    panel.append_child(&patterns_container).unwrap();

    // Add pattern button
    let add_row_btn = document.create_element("button").unwrap();
    add_row_btn.set_id("builder-add-row");
    add_row_btn.set_text_content(Some("+ Add Pattern"));
    let ar_el: HtmlElement = add_row_btn.clone().dyn_into().unwrap();
    ar_el.style().set_css_text(
        "padding: 6px 12px; background: var(--surface-panel); \
         border: 1px dashed var(--border-subtle); border-radius: var(--radius-xs); \
         color: var(--accent-emerald); font-family: var(--font-mono); font-size: 10px; \
         cursor: pointer; align-self: flex-start;",
    );
    panel.append_child(&add_row_btn).unwrap();

    // Generated query preview
    let preview_label = document.create_element("div").unwrap();
    preview_label.set_text_content(Some("Generated SPARQL:"));
    let pv_el: HtmlElement = preview_label.clone().dyn_into().unwrap();
    pv_el.style().set_css_text("font-size: 10px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase;");
    panel.append_child(&preview_label).unwrap();

    let preview = document.create_element("textarea").unwrap();
    preview.set_id("builder-preview");
    preview.set_attribute("readonly", "true").unwrap();
    let pv_html: HtmlElement = preview.clone().dyn_into().unwrap();
    pv_html.style().set_css_text(
        "width: 100%; box-sizing: border-box; height: 120px; \
         background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; \
         font-family: var(--font-mono); font-size: 10px; color: var(--accent-cyan); \
         resize: vertical;",
    );
    panel.append_child(&preview).unwrap();

    // Actions
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");

    let gen_btn = document.create_element("button").unwrap();
    gen_btn.set_id("builder-generate");
    gen_btn.set_text_content(Some("\u{2699} Generate Query"));
    let gb_el: HtmlElement = gen_btn.clone().dyn_into().unwrap();
    gb_el.style().set_css_text(
        "padding: 8px 16px; background: var(--accent-cyan); color: var(--bg-deep); \
         border: 1px solid var(--accent-cyan); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; font-weight: 700; cursor: pointer;",
    );
    actions.append_child(&gen_btn).unwrap();

    let save_btn = document.create_element("button").unwrap();
    save_btn.set_id("builder-save");
    save_btn.set_text_content(Some("\u{1F4BE} Save Query"));
    let sb_el: HtmlElement = save_btn.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "padding: 8px 16px; background: var(--surface-panel); color: var(--text-secondary); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; cursor: pointer;",
    );
    actions.append_child(&save_btn).unwrap();

    let run_btn = document.create_element("button").unwrap();
    run_btn.set_id("builder-run");
    run_btn.set_text_content(Some("\u{25B6} Run Query"));
    let rb_el: HtmlElement = run_btn.clone().dyn_into().unwrap();
    rb_el.style().set_css_text(
        "padding: 8px 16px; background: var(--surface-panel); color: var(--text-secondary); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; cursor: pointer;",
    );
    actions.append_child(&run_btn).unwrap();

    panel.append_child(&actions).unwrap();

    // Results
    let results = document.create_element("div").unwrap();
    results.set_id("builder-results");
    let r_el: HtmlElement = results.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "flex: 1; overflow-y: auto; border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; min-height: 80px; \
         font-family: var(--font-mono); font-size: 11px; color: var(--text-muted);",
    );
    results.set_text_content(Some(
        "Generate a query, then run it against the connected QualiaDB daemon.",
    ));
    panel.append_child(&results).unwrap();

    panel
}

fn build_pattern_row(document: &Document, idx: usize) -> Element {
    let row = document.create_element("div").unwrap();
    row.set_class_name("builder-pattern-row");
    row.set_attribute("data-row-idx", &idx.to_string()).unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 6px; align-items: center; flex-wrap: wrap;");

    // Subject
    let subj = document.create_element("input").unwrap();
    subj.set_class_name("pattern-subject");
    subj.set_attribute("type", "text").unwrap();
    subj.set_attribute("placeholder", "?subject").unwrap();
    subj.set_attribute("value", &format!("?s{}", idx)).unwrap();
    let s_el: HtmlElement = subj.clone().dyn_into().unwrap();
    s_el.style().set_css_text(
        "width: 100px; padding: 4px 6px; background: var(--canvas-bg); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-primary);",
    );
    row.append_child(&subj).unwrap();

    // Predicate dropdown
    let pred = document.create_element("select").unwrap();
    pred.set_class_name("pattern-predicate");
    let p_el: HtmlElement = pred.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "padding: 4px 6px; background: var(--canvas-bg); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-primary);",
    );
    for (key, display) in COMMON_PREDICATES {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", key).unwrap();
        opt.set_text_content(Some(display));
        pred.append_child(&opt).unwrap();
    }
    // Add a "custom" option
    let custom_opt = document.create_element("option").unwrap();
    custom_opt.set_attribute("value", "custom").unwrap();
    custom_opt.set_text_content(Some("custom\u{2026}"));
    pred.append_child(&custom_opt).unwrap();
    row.append_child(&pred).unwrap();

    // Object
    let obj = document.create_element("input").unwrap();
    obj.set_class_name("pattern-object");
    obj.set_attribute("type", "text").unwrap();
    obj.set_attribute("placeholder", "?object").unwrap();
    obj.set_attribute("value", &format!("?o{}", idx)).unwrap();
    let o_el: HtmlElement = obj.clone().dyn_into().unwrap();
    o_el.style().set_css_text(
        "width: 100px; padding: 4px 6px; background: var(--canvas-bg); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-primary);",
    );
    row.append_child(&obj).unwrap();

    // Remove button
    let remove_btn = document.create_element("button").unwrap();
    remove_btn.set_class_name("pattern-remove");
    remove_btn.set_text_content(Some("\u{2715}"));
    let rm_el: HtmlElement = remove_btn.clone().dyn_into().unwrap();
    rm_el.style().set_css_text(
        "padding: 4px 8px; background: transparent; border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); color: var(--accent-red); cursor: pointer; font-size: 10px;"
    );
    row.append_child(&remove_btn).unwrap();

    // Wire remove button
    let row_clone = row.clone();
    let rm_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        row_clone.remove();
    }) as Box<dyn FnMut(MouseEvent)>);
    remove_btn
        .add_event_listener_with_callback("click", rm_closure.as_ref().unchecked_ref())
        .unwrap();
    rm_closure.forget();

    row
}

pub(super) fn wire_query_builder(document: &Document) {
    // Wire add row button
    if let Some(add_btn) = document.get_element_by_id("builder-add-row") {
        let patterns_container = document.get_element_by_id("builder-patterns");
        let add_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            if let Some(container) = patterns_container.as_ref() {
                let count = container
                    .query_selector_all(".builder-pattern-row")
                    .unwrap()
                    .length();
                let row = build_pattern_row(&doc, count as usize);
                container.append_child(&row).unwrap();
            }
        }) as Box<dyn FnMut(MouseEvent)>);
        add_btn
            .add_event_listener_with_callback("click", add_closure.as_ref().unchecked_ref())
            .unwrap();
        add_closure.forget();
    }

    // Wire generate button
    if let Some(gen_btn) = document.get_element_by_id("builder-generate") {
        let gen_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            generate_builder_query(&doc);
        }) as Box<dyn FnMut(MouseEvent)>);
        gen_btn
            .add_event_listener_with_callback("click", gen_closure.as_ref().unchecked_ref())
            .unwrap();
        gen_closure.forget();
    }

    // Wire save button
    if let Some(save_btn) = document.get_element_by_id("builder-save") {
        let svb_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            super::saved::save_current_query(&doc, "builder");
        }) as Box<dyn FnMut(MouseEvent)>);
        save_btn
            .add_event_listener_with_callback("click", svb_closure.as_ref().unchecked_ref())
            .unwrap();
        svb_closure.forget();
    }

    // Wire run button
    if let Some(run_btn) = document.get_element_by_id("builder-run") {
        let rb_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            super::sparql::run_query(&doc, "builder-results");
        }) as Box<dyn FnMut(MouseEvent)>);
        run_btn
            .add_event_listener_with_callback("click", rb_closure.as_ref().unchecked_ref())
            .unwrap();
        rb_closure.forget();
    }
}

fn generate_builder_query(document: &Document) {
    // Get prefixes
    let prefixes = match document.get_element_by_id("builder-prefixes") {
        Some(p) => {
            let ta: HtmlTextAreaElement = p.dyn_into().unwrap();
            ta.value()
        }
        None => return,
    };

    // Get patterns
    let patterns = match document.get_element_by_id("builder-patterns") {
        Some(p) => p,
        None => return,
    };

    let rows = patterns.query_selector_all(".builder-pattern-row").unwrap();
    let mut pattern_lines = Vec::new();
    let mut vars = Vec::new();

    for i in 0..rows.length() {
        let row = rows.get(i).unwrap();
        let row_el: Element = row.dyn_into().unwrap();

        let subj = row_el.query_selector(".pattern-subject").unwrap().unwrap();
        let subj_input: HtmlInputElement = subj.dyn_into().unwrap();
        let s = subj_input.value();

        let pred = row_el
            .query_selector(".pattern-predicate")
            .unwrap()
            .unwrap();
        let pred_select: web_sys::HtmlSelectElement = pred.dyn_into().unwrap();
        let p = pred_select.value();

        let obj = row_el.query_selector(".pattern-object").unwrap().unwrap();
        let obj_input: HtmlInputElement = obj.dyn_into().unwrap();
        let o = obj_input.value();

        pattern_lines.push(format!("  {} {} {} .", s, p, o));

        // Collect variables for SELECT
        if s.starts_with('?') && !vars.contains(&s) {
            vars.push(s.clone());
        }
        if o.starts_with('?') && !vars.contains(&o) {
            vars.push(o.clone());
        }
    }

    if pattern_lines.is_empty() {
        pattern_lines.push("  ?s ?p ?o .".to_string());
        vars = vec!["?s".to_string(), "?p".to_string(), "?o".to_string()];
    }

    let select_vars = if vars.is_empty() {
        "*".to_string()
    } else {
        vars.join(" ")
    };

    let query = format!(
        "{}\nSELECT {} WHERE {{\n{}\n}}\nLIMIT 100",
        prefixes,
        select_vars,
        pattern_lines.join("\n")
    );

    // Set preview
    if let Some(preview) = document.get_element_by_id("builder-preview") {
        let ta: HtmlTextAreaElement = preview.dyn_into().unwrap();
        ta.set_value(&query);
    }
}
