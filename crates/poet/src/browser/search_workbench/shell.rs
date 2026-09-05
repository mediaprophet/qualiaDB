//! Search workbench overlay shell: build, mode tabs, toggle, shortcut, notices.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, KeyboardEvent, MouseEvent};

pub fn build_search_workbench(document: &Document) -> Element {
    let overlay = document.create_element("div").unwrap();
    overlay.set_id("search-workbench");
    let ov_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    ov_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.7); z-index: 10001; display: none; \
         align-items: flex-start; justify-content: center; padding-top: 60px;",
    );

    let panel = document.create_element("div").unwrap();
    panel.set_class_name("search-workbench-panel");
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "width: 880px; max-height: 680px; background: var(--surface-glass-heavy); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         backdrop-filter: blur(20px); -webkit-backdrop-filter: blur(20px); \
         box-shadow: 0 12px 48px rgba(0,0,0,0.5); overflow: hidden; \
         display: flex; flex-direction: column;",
    );

    // Header bar with title + close
    let header = document.create_element("div").unwrap();
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "display: flex; align-items: center; justify-content: space-between; \
         padding: 10px 16px; border-bottom: 1px solid var(--border-subtle);",
    );

    let title = document.create_element("span").unwrap();
    let t_el: HtmlElement = title.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "font-size: 13px; font-weight: 700; color: var(--accent-cyan); \
         text-transform: uppercase; letter-spacing: 0.5px; font-family: var(--font-mono);",
    );
    title.set_text_content(Some("\u{1F50D} Search Workbench"));
    header.append_child(&title).unwrap();

    let close_btn = document.create_element("button").unwrap();
    close_btn.set_text_content(Some("\u{2715}"));
    let cb_el: HtmlElement = close_btn.clone().dyn_into().unwrap();
    cb_el.style().set_css_text(
        "background: transparent; border: none; color: var(--text-muted); \
         cursor: pointer; font-size: 16px; padding: 4px;",
    );
    header.append_child(&close_btn).unwrap();
    panel.append_child(&header).unwrap();

    // Mode tabs
    let tabs = document.create_element("div").unwrap();
    tabs.set_class_name("search-mode-tabs");
    let tabs_el: HtmlElement = tabs.clone().dyn_into().unwrap();
    tabs_el
        .style()
        .set_css_text("display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle);");

    for (i, (mode_id, label, icon)) in [
        ("faceted", "Faceted Search", "\u{1F3AF}"),
        ("builder", "Query Builder", "\u{1F9F9}"),
        ("sparql", "Manual SPARQL", "\u{270F}\u{FE0F}"),
        ("saved", "Saved Queries", "\u{1F4BE}"),
    ]
    .iter()
    .enumerate()
    {
        let tab = document.create_element("button").unwrap();
        tab.set_class_name("search-mode-tab");
        tab.set_attribute("data-mode", mode_id).unwrap();
        if i == 0 {
            tab.class_list().add_1("active").unwrap();
        }
        let tab_el: HtmlElement = tab.clone().dyn_into().unwrap();
        tab_el.style().set_css_text(&format!(
            "padding: 8px 16px; background: transparent; border: none; \
             border-bottom: 2px solid {}; \
             color: {}; font-size: 11px; font-family: var(--font-mono); \
             cursor: pointer; display: flex; align-items: center; gap: 6px; \
             transition: var(--trans-fast);",
            if i == 0 {
                "var(--accent-cyan)"
            } else {
                "transparent"
            },
            if i == 0 {
                "var(--text-primary)"
            } else {
                "var(--text-muted)"
            },
        ));
        tab.set_text_content(Some(&format!("{} {}", icon, label)));
        tabs.append_child(&tab).unwrap();
    }
    panel.append_child(&tabs).unwrap();

    // Content area — holds the active mode panel
    let content = document.create_element("div").unwrap();
    content.set_id("search-workbench-content");
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style().set_css_text(
        "flex: 1; overflow-y: auto; padding: 16px; display: flex; flex-direction: column; gap: 12px;"
    );

    // Build all mode panels (only the active one is visible)
    content
        .append_child(&super::faceted::build_faceted_panel(document))
        .unwrap();
    content
        .append_child(&super::builder::build_query_builder_panel(document))
        .unwrap();
    content
        .append_child(&super::sparql::build_sparql_panel(document))
        .unwrap();
    content
        .append_child(&super::saved::build_saved_queries_panel(document))
        .unwrap();

    // Show only the faceted panel initially
    show_mode_panel(document, "faceted");

    panel.append_child(&content).unwrap();

    // Honesty footer
    let footer = document.create_element("div").unwrap();
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "padding: 6px 16px; border-top: 1px solid var(--border-subtle); \
         font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);",
    );
    footer.set_text_content(Some(
        "\u{1F4A1} Canvas facets, query construction, and saving are local. \
         SPARQL executes only against a connected QualiaDB daemon.",
    ));
    panel.append_child(&footer).unwrap();

    overlay.append_child(&panel).unwrap();

    // Wire close button
    let ov_clone = overlay.clone();
    let close_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let ov: HtmlElement = ov_clone.clone().dyn_into().unwrap();
        ov.style().set_property("display", "none").unwrap();
    }) as Box<dyn FnMut(MouseEvent)>);
    close_btn
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .unwrap();
    close_closure.forget();

    // Wire mode tabs
    wire_mode_tabs(document);

    // Wire faceted search
    super::faceted::wire_faceted_search(document);

    // Wire query builder
    super::builder::wire_query_builder(document);

    // Wire SPARQL editor
    super::sparql::wire_sparql_editor(document);

    // Wire saved queries
    super::saved::wire_saved_queries(document);

    overlay
}

pub(super) fn show_mode_panel(document: &Document, mode: &str) {
    let panels = document.query_selector_all(".search-mode-panel").unwrap();
    for i in 0..panels.length() {
        let p = panels.get(i).unwrap();
        let p_el: Element = p.dyn_into().unwrap();
        let p_mode = p_el.get_attribute("data-mode").unwrap_or_default();
        let html_el: HtmlElement = p_el.clone().dyn_into().unwrap();
        if p_mode == mode {
            html_el.style().set_property("display", "").unwrap();
        } else {
            html_el.style().set_property("display", "none").unwrap();
        }
    }
}

fn wire_mode_tabs(document: &Document) {
    let tabs = document.query_selector_all(".search-mode-tab").unwrap();
    for i in 0..tabs.length() {
        let tab = tabs.get(i).unwrap();
        let tab_el: Element = tab.dyn_into().unwrap();
        let mode = tab_el.get_attribute("data-mode").unwrap_or_default();
        let tab_clone = tab_el.clone();

        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            // Update tab active states
            let all_tabs = doc.query_selector_all(".search-mode-tab").unwrap();
            for j in 0..all_tabs.length() {
                let t = all_tabs.get(j).unwrap();
                let t_el: Element = t.dyn_into().unwrap();
                t_el.class_list().remove_1("active").unwrap();
                let t_html: HtmlElement = t_el.clone().dyn_into().unwrap();
                t_html
                    .style()
                    .set_property("border-bottom", "2px solid transparent")
                    .unwrap();
                t_html
                    .style()
                    .set_property("color", "var(--text-muted)")
                    .unwrap();
            }
            tab_clone.class_list().add_1("active").unwrap();
            let tab_html: HtmlElement = tab_clone.clone().dyn_into().unwrap();
            tab_html
                .style()
                .set_property("border-bottom", "2px solid var(--accent-cyan)")
                .unwrap();
            tab_html
                .style()
                .set_property("color", "var(--text-primary)")
                .unwrap();

            show_mode_panel(&doc, &mode);
        }) as Box<dyn FnMut(MouseEvent)>);

        tab_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn show_search_notification(document: &Document, message: &str) {
    let notif = document.create_element("div").unwrap();
    let n_el: HtmlElement = notif.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "position: fixed; bottom: 40px; right: 16px; \
         background: var(--surface-panel-elevated); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         padding: 10px 14px; font-size: 12px; color: var(--text-primary); \
         box-shadow: var(--shadow-lg); z-index: 10002; max-width: 320px;",
    );
    notif.set_text_content(Some(&format!("\u{1F50D} {}", message)));
    if let Some(body) = document.body() {
        body.append_child(&notif).unwrap();
    }
    let notif_clone = notif.clone();
    let timeout = Closure::wrap(Box::new(move || {
        notif_clone.remove();
    }) as Box<dyn FnMut()>);
    super::super::interactions::set_timeout(timeout.as_ref().unchecked_ref(), 2500);
    timeout.forget();
}

/// Toggle the search workbench visibility.
pub fn toggle_search_workbench(document: &Document) {
    if let Some(wb) = document.get_element_by_id("search-workbench") {
        let wb_el: HtmlElement = wb.dyn_into().unwrap();
        let display = wb_el
            .style()
            .get_property_value("display")
            .unwrap_or_default();
        if display == "none" {
            wb_el.style().set_property("display", "flex").unwrap();
            // Refresh saved queries
            super::saved::render_saved_queries(document);
        } else {
            wb_el.style().set_property("display", "none").unwrap();
        }
    }
}

/// Open the search workbench to a specific mode.
pub fn open_to_mode(document: &Document, mode: &str) {
    if let Some(wb) = document.get_element_by_id("search-workbench") {
        let wb_el: HtmlElement = wb.dyn_into().unwrap();
        wb_el.style().set_property("display", "flex").unwrap();
    }
    // Update tab active states
    let tabs = document.query_selector_all(".search-mode-tab").unwrap();
    for i in 0..tabs.length() {
        let t = tabs.get(i).unwrap();
        let t_el: Element = t.dyn_into().unwrap();
        let t_mode = t_el.get_attribute("data-mode").unwrap_or_default();
        if t_mode == mode {
            t_el.class_list().add_1("active").unwrap();
            let t_html: HtmlElement = t_el.clone().dyn_into().unwrap();
            t_html
                .style()
                .set_property("border-bottom", "2px solid var(--accent-cyan)")
                .unwrap();
            t_html
                .style()
                .set_property("color", "var(--text-primary)")
                .unwrap();
        } else {
            t_el.class_list().remove_1("active").unwrap();
            let t_html: HtmlElement = t_el.clone().dyn_into().unwrap();
            t_html
                .style()
                .set_property("border-bottom", "2px solid transparent")
                .unwrap();
            t_html
                .style()
                .set_property("color", "var(--text-muted)")
                .unwrap();
        }
    }
    show_mode_panel(document, mode);
    if mode == "saved" {
        super::saved::render_saved_queries(document);
    }
}

/// Wire Ctrl+Shift+F to toggle the search workbench.
pub fn wire_search_workbench_shortcut(document: &Document) {
    let closure = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        if e.key() == "F" && e.shift_key() && (e.ctrl_key() || e.meta_key()) {
            e.prevent_default();
            let doc = web_sys::window().unwrap().document().unwrap();
            toggle_search_workbench(&doc);
        }
    }) as Box<dyn FnMut(KeyboardEvent)>);

    document
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}
