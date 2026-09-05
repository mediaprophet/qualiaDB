//! Technical sidebar, accessibility notice, and pod tray shell.

use super::*;

pub fn toggle_tech_sidebar(document: &Document) {
    // If sidebar exists, remove it
    if let Some(existing) = document.get_element_by_id("tech-sidebar") {
        existing.remove();
        return;
    }

    let sidebar = document.create_element("div").unwrap();
    sidebar.set_id("tech-sidebar");
    sidebar.set_class_name("tech-sidebar");
    let s_el: HtmlElement = sidebar.clone().dyn_into().unwrap();
    s_el.style().set_css_text(
        "position: fixed; top: 80px; right: 16px; width: 300px; max-height: 500px; \
         background: var(--surface-glass-heavy); backdrop-filter: blur(20px); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         box-shadow: var(--shadow-lg); z-index: 550; display: flex; flex-direction: column; \
         overflow: hidden;",
    );

    // Header
    let header = document.create_element("div").unwrap();
    header.set_class_name("tech-sidebar-header");
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "height: 36px; padding: 0 12px; display: flex; align-items: center; justify-content: space-between; \
         background: var(--surface-panel); border-bottom: 1px solid var(--border-subtle); \
         font-size: 10px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-muted); font-weight: 700;"
    );
    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("\u{2699}\u{FE0F} Telemetry & DAG"));
    header.append_child(&title).unwrap();
    let close = document.create_element("button").unwrap();
    close.set_text_content(Some("\u{2715}"));
    let close_el: HtmlElement = close.clone().dyn_into().unwrap();
    close_el.style().set_css_text("background: transparent; border: none; color: var(--text-muted); cursor: pointer; font-size: 14px;");
    header.append_child(&close).unwrap();
    sidebar.append_child(&header).unwrap();

    // Body
    let body = document.create_element("div").unwrap();
    let b_el: HtmlElement = body.clone().dyn_into().unwrap();
    b_el.style().set_css_text("flex: 1; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 12px;");

    // Merkle-CRDT DAG section
    let dag_section = document.create_element("div").unwrap();
    let dag_title = document.create_element("div").unwrap();
    dag_title.set_attribute("style", "font-size: 10px; font-weight: 700; color: var(--accent-cyan); text-transform: uppercase; margin-bottom: 6px;").unwrap();
    dag_title.set_text_content(Some("Merkle-CRDT DAG"));
    dag_section.append_child(&dag_title).unwrap();

    let dag_viz = document.create_element("div").unwrap();
    dag_viz.set_class_name("dag-viz");
    let dv_el: HtmlElement = dag_viz.clone().dyn_into().unwrap();
    dv_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 10px; color: var(--text-muted); \
         background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; line-height: 1.6;",
    );
    dag_viz.set_text_content(Some(
        "Unavailable: live Merkle-CRDT DAG data is not connected.",
    ));
    dag_section.append_child(&dag_viz).unwrap();
    body.append_child(&dag_section).unwrap();

    // Container quads section
    let quads_section = document.create_element("div").unwrap();
    let quads_title = document.create_element("div").unwrap();
    quads_title.set_attribute("style", "font-size: 10px; font-weight: 700; color: var(--accent-cyan); text-transform: uppercase; margin-bottom: 6px;").unwrap();
    quads_title.set_text_content(Some("Container Quads"));
    quads_section.append_child(&quads_title).unwrap();

    let quads_list = document.create_element("div").unwrap();
    let ql_el: HtmlElement = quads_list.clone().dyn_into().unwrap();
    ql_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 9px; color: var(--text-muted); \
         background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; line-height: 1.6;",
    );
    quads_list.set_text_content(Some(
        "Unavailable: live container quad data is not connected.",
    ));
    quads_section.append_child(&quads_list).unwrap();
    body.append_child(&quads_section).unwrap();

    // Connection ontology section
    let conn_section = document.create_element("div").unwrap();
    let conn_title = document.create_element("div").unwrap();
    conn_title.set_attribute("style", "font-size: 10px; font-weight: 700; color: var(--accent-cyan); text-transform: uppercase; margin-bottom: 6px;").unwrap();
    conn_title.set_text_content(Some("Connection Ontology"));
    conn_section.append_child(&conn_title).unwrap();

    let conn_info = document.create_element("div").unwrap();
    let ci_el: HtmlElement = conn_info.clone().dyn_into().unwrap();
    ci_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 9px; color: var(--text-muted); \
         background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; line-height: 1.6;",
    );
    conn_info.set_text_content(Some(
        "Unavailable: live connection ontology data is not connected.",
    ));
    conn_section.append_child(&conn_info).unwrap();
    body.append_child(&conn_section).unwrap();

    // Honesty note
    let note = document.create_element("div").unwrap();
    note.set_attribute("style", "font-size: 9px; color: var(--text-muted); padding-top: 4px; border-top: 1px solid var(--border-subtle);").unwrap();
    note.set_text_content(Some(
        "unavailable \u{00B7} telemetry and DAG data require a live backend connection",
    ));
    body.append_child(&note).unwrap();

    sidebar.append_child(&body).unwrap();

    if let Some(doc_body) = document.body() {
        doc_body.append_child(&sidebar).unwrap();
    }

    // Wire close button
    let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        let doc = web_sys::window().unwrap().document().unwrap();
        if let Some(sb) = doc.get_element_by_id("tech-sidebar") {
            sb.remove();
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    close
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

pub(super) fn show_a11y_notification(document: &Document) {
    super::super::accessibility::open_dialog(document);
}

pub(super) fn toggle_pod_tray(document: &Document, pod_id: &str) {
    let tray = match document.get_element_by_id("top-pod-drop-tray") {
        Some(t) => t,
        None => return,
    };
    let t_el: HtmlElement = tray.clone().dyn_into().unwrap();
    let display = t_el
        .style()
        .get_property_value("display")
        .unwrap_or_default();

    if display != "none" && tray.get_attribute("data-active-pod").as_deref() == Some(pod_id) {
        // Same pod — close
        t_el.style().set_property("display", "none").unwrap();
        return;
    }

    tray.set_inner_html("");
    tray.set_attribute("data-active-pod", pod_id).unwrap();

    match pod_id {
        "strata" => populate_strata_tray(document, &tray),
        "epistemic" => populate_epistemic_tray(document, &tray),
        "time-dim" => populate_dim_tray(document, &tray),
        _ => {}
    }

    t_el.style().set_property("display", "flex").unwrap();
}
