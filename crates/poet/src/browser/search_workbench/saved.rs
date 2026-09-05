//! Saved-query list UI, load/delete actions, and save-from-current-mode.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlInputElement, HtmlTextAreaElement, MouseEvent};

pub(super) fn build_saved_queries_panel(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("search-mode-panel");
    panel.set_attribute("data-mode", "saved").unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("display: none; flex-direction: column; gap: 8px;");

    let label = document.create_element("div").unwrap();
    label.set_text_content(Some("Saved Queries (persisted in localStorage):"));
    let l_el: HtmlElement = label.clone().dyn_into().unwrap();
    l_el.style().set_css_text("font-size: 10px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase;");
    panel.append_child(&label).unwrap();

    let list = document.create_element("div").unwrap();
    list.set_id("saved-queries-list");
    let l_el: HtmlElement = list.clone().dyn_into().unwrap();
    l_el.style().set_css_text(
        "flex: 1; overflow-y: auto; border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; min-height: 200px; \
         display: flex; flex-direction: column; gap: 4px;",
    );
    panel.append_child(&list).unwrap();

    let refresh_btn = document.create_element("button").unwrap();
    refresh_btn.set_id("saved-refresh");
    refresh_btn.set_text_content(Some("\u{1F504} Refresh"));
    let r_el: HtmlElement = refresh_btn.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "padding: 6px 12px; background: var(--surface-panel); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         color: var(--text-secondary); font-family: var(--font-mono); font-size: 10px; \
         cursor: pointer; align-self: flex-start;",
    );
    panel.append_child(&refresh_btn).unwrap();

    panel
}

pub(super) fn wire_saved_queries(document: &Document) {
    if let Some(refresh_btn) = document.get_element_by_id("saved-refresh") {
        let r_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            render_saved_queries(&doc);
        }) as Box<dyn FnMut(MouseEvent)>);
        refresh_btn
            .add_event_listener_with_callback("click", r_closure.as_ref().unchecked_ref())
            .unwrap();
        r_closure.forget();
    }
}

pub(super) fn render_saved_queries(document: &Document) {
    let list = match document.get_element_by_id("saved-queries-list") {
        Some(l) => l,
        None => return,
    };
    list.set_inner_html("");

    let queries = super::persist::load_saved_queries();

    if queries.is_empty() {
        let empty = document.create_element("div").unwrap();
        let e_el: HtmlElement = empty.clone().dyn_into().unwrap();
        e_el.style().set_css_text(
            "font-size: 11px; color: var(--text-muted); padding: 20px; text-align: center;",
        );
        empty.set_text_content(Some(
            "No saved queries yet. Use \u{1F4BE} Save Query in any mode to save a query.",
        ));
        list.append_child(&empty).unwrap();
        return;
    }

    for q in &queries {
        let item = document.create_element("div").unwrap();
        let i_el: HtmlElement = item.clone().dyn_into().unwrap();
        i_el.style().set_css_text(
            "display: flex; align-items: center; gap: 8px; padding: 8px 10px; \
             background: var(--surface-panel); border-radius: var(--radius-xs); \
             border-left: 3px solid var(--accent-cyan);",
        );

        // Name + metadata
        let info = document.create_element("div").unwrap();
        let inf_el: HtmlElement = info.clone().dyn_into().unwrap();
        inf_el
            .style()
            .set_css_text("flex: 1; display: flex; flex-direction: column; gap: 2px;");

        let name = document.create_element("span").unwrap();
        let n_el: HtmlElement = name.clone().dyn_into().unwrap();
        n_el.style().set_css_text("font-size: 11px; font-weight: 600; color: var(--text-primary); font-family: var(--font-mono);");
        name.set_text_content(Some(&q.name));
        info.append_child(&name).unwrap();

        let meta = document.create_element("span").unwrap();
        let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
        m_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        meta.set_text_content(Some(&format!(
            "[{}] {} chars \u{2014} {}",
            q.mode,
            q.query.len(),
            q.timestamp
        )));
        info.append_child(&meta).unwrap();

        item.append_child(&info).unwrap();

        // Load button
        let load_btn = document.create_element("button").unwrap();
        load_btn.set_text_content(Some("Load"));
        let lb_el: HtmlElement = load_btn.clone().dyn_into().unwrap();
        lb_el.style().set_css_text(
            "padding: 4px 10px; background: var(--surface-panel-elevated); \
             border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
             color: var(--accent-cyan); font-family: var(--font-mono); font-size: 9px; cursor: pointer;"
        );
        let q_clone = q.clone();
        let load_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            load_query_into_editor(&doc, &q_clone);
        }) as Box<dyn FnMut(MouseEvent)>);
        load_btn
            .add_event_listener_with_callback("click", load_closure.as_ref().unchecked_ref())
            .unwrap();
        load_closure.forget();
        item.append_child(&load_btn).unwrap();

        // Place container button
        let place_btn = document.create_element("button").unwrap();
        place_btn.set_text_content(Some("Place"));
        let pb_el: HtmlElement = place_btn.clone().dyn_into().unwrap();
        pb_el.style().set_css_text(
            "padding: 4px 10px; background: var(--surface-panel-elevated); \
             border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
             color: var(--accent-violet); font-family: var(--font-mono); font-size: 9px; cursor: pointer;"
        );
        let q_clone2 = q.clone();
        let place_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            super::placement::place_named_query_container(&doc, &q_clone2);
        }) as Box<dyn FnMut(MouseEvent)>);
        place_btn
            .add_event_listener_with_callback("click", place_closure.as_ref().unchecked_ref())
            .unwrap();
        place_closure.forget();
        item.append_child(&place_btn).unwrap();

        // Delete button
        let del_btn = document.create_element("button").unwrap();
        del_btn.set_text_content(Some("\u{2715}"));
        let db_el: HtmlElement = del_btn.clone().dyn_into().unwrap();
        db_el.style().set_css_text(
            "padding: 4px 8px; background: transparent; \
             border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
             color: var(--accent-red); font-family: var(--font-mono); font-size: 9px; cursor: pointer;"
        );
        let q_id = q.id.clone();
        let del_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            delete_saved_query(&doc, &q_id);
        }) as Box<dyn FnMut(MouseEvent)>);
        del_btn
            .add_event_listener_with_callback("click", del_closure.as_ref().unchecked_ref())
            .unwrap();
        del_closure.forget();
        item.append_child(&del_btn).unwrap();

        list.append_child(&item).unwrap();
    }
}

pub(super) fn save_current_query(document: &Document, mode: &str) {
    // Get query text based on mode
    let (query_text, name) = match mode {
        "faceted" => {
            let facets = super::faceted::get_active_facets(document);
            let facet_desc: Vec<String> = facets
                .iter()
                .map(|(f, v)| format!("{}={}", f, v.join(",")))
                .collect();
            let q = format!(
                "# Faceted search\n# Facets: {}\nSELECT * WHERE {{ ?s ?p ?o . }} LIMIT 100",
                facet_desc.join("; ")
            );
            let name = format!("faceted-{}", js_sys::Date::now() as u64 % 10000);
            (q, name)
        }
        "builder" => {
            let preview = document
                .get_element_by_id("builder-preview")
                .and_then(|p| p.dyn_into::<HtmlTextAreaElement>().ok())
                .map(|ta| ta.value())
                .unwrap_or_default();
            let name = format!("builder-{}", js_sys::Date::now() as u64 % 10000);
            (preview, name)
        }
        "sparql" => {
            let editor = document
                .get_element_by_id("sparql-editor")
                .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok())
                .map(|ta| ta.value())
                .unwrap_or_default();
            let name = document
                .get_element_by_id("sparql-query-name")
                .and_then(|n| n.dyn_into::<HtmlInputElement>().ok())
                .map(|i| i.value())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("sparql-{}", js_sys::Date::now() as u64 % 10000));
            (editor, name)
        }
        _ => return,
    };

    if query_text.trim().is_empty() {
        super::shell::show_search_notification(
            document,
            "Query is empty \u{2014} nothing to save.",
        );
        return;
    }

    let now = js_sys::Date::new_0();
    let timestamp = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        now.get_full_year(),
        now.get_month() + 1,
        now.get_date(),
        now.get_hours(),
        now.get_minutes(),
        now.get_seconds()
    );

    let saved = super::persist::SavedQuery {
        id: format!("q-{}", js_sys::Date::now() as u64),
        name,
        mode: mode.to_string(),
        query: query_text,
        timestamp,
    };

    super::persist::save_query_to_storage(&saved);
    super::shell::show_search_notification(
        document,
        &format!("Saved query \u{201C}{}\u{201D}", saved.name),
    );
    render_saved_queries(document);
}

fn load_query_into_editor(document: &Document, query: &super::persist::SavedQuery) {
    // Load into the SPARQL editor
    if let Some(editor) = document.get_element_by_id("sparql-editor") {
        let ta: HtmlTextAreaElement = editor.dyn_into().unwrap();
        ta.set_value(&query.query);
    }
    if let Some(name_input) = document.get_element_by_id("sparql-query-name") {
        let input: HtmlInputElement = name_input.dyn_into().unwrap();
        input.set_value(&query.name);
    }

    // Switch to SPARQL mode
    super::shell::show_mode_panel(document, "sparql");
    // Update tab active states
    let tabs = document.query_selector_all(".search-mode-tab").unwrap();
    for i in 0..tabs.length() {
        let t = tabs.get(i).unwrap();
        let t_el: Element = t.dyn_into().unwrap();
        let t_mode = t_el.get_attribute("data-mode").unwrap_or_default();
        if t_mode == "sparql" {
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

    super::shell::show_search_notification(
        document,
        &format!("Loaded \u{201C}{}\u{201D} into SPARQL editor", query.name),
    );
}

fn delete_saved_query(document: &Document, id: &str) {
    super::persist::delete_saved_query_from_storage(id);
    render_saved_queries(document);
    super::shell::show_search_notification(document, "Query deleted");
}
