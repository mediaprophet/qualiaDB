//! Shared browser adapter for COP-backed daemon records.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlInputElement};

use super::native_daemon::{
    daemon_records_delete, daemon_records_query, daemon_records_upsert, is_daemon_connected,
    NativeRecordDeleteRequest, NativeRecordQueryRequest, NativeRecordUpsertRequest,
};

pub struct CopField {
    pub key: &'static str,
    pub placeholder: &'static str,
}

pub struct CopPanel<'a> {
    pub family: &'static str,
    pub heading: &'a str,
    pub fields: &'static [CopField],
    pub kind: Option<&'static str>,
    pub group_by: Option<&'static str>,
    pub columns: Option<&'static [(&'static str, &'static str)]>,
}

pub fn build_family_panel(
    document: &Document,
    family: &'static str,
    heading: &str,
    fields: &'static [CopField],
) -> Element {
    build_cop_panel(
        document,
        &CopPanel {
            family,
            heading,
            fields,
            kind: None,
            group_by: None,
            columns: None,
        },
    )
}

pub fn build_cop_panel(document: &Document, spec: &CopPanel<'_>) -> Element {
    let family = spec.family;
    let root = document.create_element("div").unwrap();
    root.set_attribute("data-cop-family", family).ok();
    root.set_attribute("data-honesty", "running").ok();
    if let Some(kind) = spec.kind {
        root.set_attribute("data-cop-kind", kind).ok();
    }
    if let Some(group_by) = spec.group_by {
        root.set_attribute("data-cop-group-by", group_by).ok();
    }
    if let Some(columns) = spec.columns {
        let encoded = columns
            .iter()
            .map(|(id, label)| format!("{id}:{label}"))
            .collect::<Vec<_>>()
            .join(",");
        root.set_attribute("data-cop-columns", &encoded).ok();
    }

    let status = document.create_element("div").unwrap();
    status.set_attribute("data-cop-status", family).ok();
    status.set_attribute("role", "status").ok();
    let status_el: HtmlElement = status.clone().dyn_into().unwrap();
    status_el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); margin-bottom: 6px;",
    );
    status.set_text_content(Some(spec.heading));
    root.append_child(&status).unwrap();

    let list = document.create_element("div").unwrap();
    list.set_attribute("data-cop-list", family).ok();
    root.append_child(&list).unwrap();

    let form = document.create_element("div").unwrap();
    let form_el: HtmlElement = form.clone().dyn_into().unwrap();
    form_el
        .style()
        .set_css_text("display: flex; flex-wrap: wrap; gap: 6px; margin-top: 8px;");
    let title = document.create_element("input").unwrap();
    title.set_attribute("data-cop-title", family).ok();
    title.set_attribute("placeholder", "Title").ok();
    style_input(&title);
    form.append_child(&title).unwrap();
    for field in spec.fields {
        let input = document.create_element("input").unwrap();
        input.set_attribute("data-cop-field", field.key).ok();
        input.set_attribute("placeholder", field.placeholder).ok();
        style_input(&input);
        form.append_child(&input).unwrap();
    }
    let save = document.create_element("button").unwrap();
    save.set_text_content(Some("Save"));
    save.set_attribute("type", "button").ok();
    gate_daemon(&save, "Persist this COP record on the local daemon.");
    let save_el: HtmlElement = save.clone().dyn_into().unwrap();
    save_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); background: transparent; \
         color: var(--text-secondary); border-radius: 3px; cursor: pointer; font-size: 10px;",
    );
    form.append_child(&save).unwrap();
    root.append_child(&form).unwrap();

    let root_clone = root.clone();
    let status_clone = status.clone();
    let form_clone = form.clone();
    let family_owned = family.to_string();
    let kind_owned = spec.kind.unwrap_or("").to_string();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let title = input_value(&form_clone, &format!("[data-cop-title=\"{family_owned}\"]"));
        if title.trim().is_empty() {
            status_clone.set_text_content(Some("Enter a title before saving."));
            return;
        }
        let mut fields = serde_json::Map::new();
        if let Ok(inputs) = form_clone.query_selector_all("[data-cop-field]") {
            for index in 0..inputs.length() {
                let Some(node) = inputs.get(index) else {
                    continue;
                };
                let Ok(input) = node.dyn_into::<HtmlInputElement>() else {
                    continue;
                };
                if let Some(key) = input.get_attribute("data-cop-field") {
                    fields.insert(key, serde_json::Value::String(input.value()));
                }
            }
        }
        if !kind_owned.is_empty() {
            fields.insert(
                "kind".to_string(),
                serde_json::Value::String(kind_owned.clone()),
            );
        }
        status_clone.set_text_content(Some("Saving COP record…"));
        let root_async = root_clone.clone();
        let status_async = status_clone.clone();
        let family_async = family_owned.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let response = daemon_records_upsert(NativeRecordUpsertRequest {
                family: family_async.clone(),
                title,
                id: None,
                fields,
            })
            .await;
            match response {
                Ok(response) if response.ok => {
                    status_async.set_text_content(Some("Record saved."));
                    refresh(&root_async, &family_async, &status_async);
                }
                Ok(response) => status_async.set_text_content(Some(
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("The daemon rejected the record."),
                )),
                Err(error) => status_async.set_text_content(Some(&error)),
            }
        });
    }) as Box<dyn FnMut(_)>);
    save.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    refresh(&root, family, &status);
    root
}

pub fn build_count_panel(
    document: &Document,
    heading: &str,
    families: &'static [(&'static str, &'static str)],
) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_attribute("data-cop-counts", "project").ok();
    root.set_attribute("data-honesty", "running").ok();

    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    let status_el: HtmlElement = status.clone().dyn_into().unwrap();
    status_el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); margin-bottom: 6px;",
    );
    status.set_text_content(Some(heading));
    root.append_child(&status).unwrap();

    let grid = document.create_element("div").unwrap();
    let grid_el: HtmlElement = grid.clone().dyn_into().unwrap();
    grid_el
        .style()
        .set_css_text("display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px;");
    for (family, label) in families {
        let card = document.create_element("div").unwrap();
        card.set_attribute("data-count-family", family).ok();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text(
            "border: 1px solid var(--border-medium); border-radius: 6px; padding: 8px; \
             background: var(--surface-panel);",
        );
        let lbl = document.create_element("div").unwrap();
        lbl.set_text_content(Some(label));
        let lbl_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        lbl_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono); margin-bottom: 4px;",
        );
        card.append_child(&lbl).unwrap();
        let val = document.create_element("div").unwrap();
        val.set_attribute("data-count-value", family).ok();
        val.set_text_content(Some("—"));
        let val_el: HtmlElement = val.clone().dyn_into().unwrap();
        val_el.style().set_css_text(
            "font-size: 16px; font-weight: 700; color: var(--accent-cyan); font-family: var(--font-mono);",
        );
        card.append_child(&val).unwrap();
        let det = document.create_element("div").unwrap();
        det.set_attribute("data-count-detail", family).ok();
        det.set_text_content(Some("Waiting for daemon…"));
        let det_el: HtmlElement = det.clone().dyn_into().unwrap();
        det_el
            .style()
            .set_css_text("font-size: 8px; color: var(--text-muted); margin-top: 2px;");
        card.append_child(&det).unwrap();
        grid.append_child(&card).unwrap();
    }
    root.append_child(&grid).unwrap();
    refresh_counts(&root, &status, families);
    root
}

/// Refresh an already-mounted COP panel after an external event (for example
/// the live Pulse SSE stream) changes the underlying ledger.
pub fn refresh_family_panel(root: &Element) {
    let family = root.get_attribute("data-cop-family").unwrap_or_default();
    if family.is_empty() {
        return;
    }
    if let Some(status) = root
        .query_selector(&format!("[data-cop-status=\"{family}\"]"))
        .ok()
        .flatten()
    {
        refresh(root, &family, &status);
    }
}

fn refresh(root: &Element, family: &str, status: &Element) {
    if !is_daemon_connected() {
        root.set_attribute("data-honesty", "unavailable").ok();
        status.set_text_content(Some(
            "Unavailable: start the local QualiaDB daemon to persist COP records.",
        ));
        return;
    }
    root.set_attribute("data-honesty", "running").ok();
    status.set_text_content(Some("Loading COP records…"));
    let root_async = root.clone();
    let status_async = status.clone();
    let family_async = family.to_string();
    let kind_async = root.get_attribute("data-cop-kind").unwrap_or_default();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_query(NativeRecordQueryRequest {
            family: family_async.clone(),
            query: String::new(),
            kind: kind_async,
        })
        .await
        {
            Ok(response) if response.ok => {
                root_async.set_attribute("data-honesty", "live").ok();
                render_records(&root_async, &family_async, &response.data, &status_async);
            }
            Ok(response) => {
                root_async.set_attribute("data-honesty", "error").ok();
                status_async.set_text_content(Some(
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("COP record query failed."),
                ));
            }
            Err(error) => {
                root_async.set_attribute("data-honesty", "error").ok();
                status_async.set_text_content(Some(&error));
            }
        }
    });
}

fn refresh_counts(
    root: &Element,
    status: &Element,
    families: &'static [(&'static str, &'static str)],
) {
    if !is_daemon_connected() {
        root.set_attribute("data-honesty", "unavailable").ok();
        status.set_text_content(Some(
            "Unavailable: start the local QualiaDB daemon to aggregate project records.",
        ));
        return;
    }
    let root_async = root.clone();
    let status_async = status.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let mut loaded = 0usize;
        let mut failed = 0usize;
        for (family, _) in families {
            match daemon_records_query(NativeRecordQueryRequest {
                family: (*family).to_string(),
                query: String::new(),
                kind: String::new(),
            })
            .await
            {
                Ok(response) if response.ok => {
                    loaded += 1;
                    let count = response
                        .data
                        .get("count")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    if let Some(value) = root_async
                        .query_selector(&format!("[data-count-value=\"{family}\"]"))
                        .ok()
                        .flatten()
                    {
                        value.set_text_content(Some(&count.to_string()));
                    }
                    if let Some(detail) = root_async
                        .query_selector(&format!("[data-count-detail=\"{family}\"]"))
                        .ok()
                        .flatten()
                    {
                        detail.set_text_content(Some("Live COP ledger count"));
                    }
                }
                Ok(_) | Err(_) => {
                    failed += 1;
                    if let Some(detail) = root_async
                        .query_selector(&format!("[data-count-detail=\"{family}\"]"))
                        .ok()
                        .flatten()
                    {
                        detail.set_text_content(Some("Query failed"));
                    }
                }
            }
        }
        root_async
            .set_attribute("data-honesty", if failed == 0 { "live" } else { "error" })
            .ok();
        status_async.set_text_content(Some(&format!(
            "Live counts from {loaded} project families ({} failed).",
            failed
        )));
    });
}

fn render_records(root: &Element, family: &str, data: &serde_json::Value, status: &Element) {
    let Some(list) = root
        .query_selector(&format!("[data-cop-list=\"{family}\"]"))
        .ok()
        .flatten()
    else {
        return;
    };
    while let Some(child) = list.first_element_child() {
        child.remove();
    }
    let records = data
        .get("records")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    if records.is_empty() {
        status.set_text_content(Some("No COP records in this family yet."));
        return;
    }
    status.set_text_content(Some(&format!("{} live COP record(s).", records.len())));
    if root.get_attribute("data-cop-group-by").is_some() {
        render_board(root, &list, &records);
        return;
    }
    let document = root.owner_document().unwrap();
    for record in records {
        list.append_child(&record_card(&document, root, family, status, &record))
            .unwrap();
    }
}

fn render_board(root: &Element, list: &Element, records: &[serde_json::Value]) {
    let document = root.owner_document().unwrap();
    let group_by = root
        .get_attribute("data-cop-group-by")
        .unwrap_or_else(|| "status".to_string());
    let columns = parse_columns(root.get_attribute("data-cop-columns").as_deref());
    let family = root.get_attribute("data-cop-family").unwrap_or_default();
    let status = root
        .query_selector(&format!("[data-cop-status=\"{family}\"]"))
        .ok()
        .flatten();
    let board = document.create_element("div").unwrap();
    let board_el: HtmlElement = board.clone().dyn_into().unwrap();
    board_el
        .style()
        .set_css_text("display: flex; gap: 8px; overflow-x: auto; align-items: flex-start;");
    for (id, label) in &columns {
        let column = document.create_element("div").unwrap();
        let column_el: HtmlElement = column.clone().dyn_into().unwrap();
        column_el.style().set_css_text(
            "min-width: 180px; flex: 1; background: var(--surface-panel); border: 1px solid var(--border-subtle); \
             border-radius: 4px; padding: 6px;",
        );
        let header = document.create_element("div").unwrap();
        let matching = records
            .iter()
            .filter(|record| field_str(record, &group_by) == *id)
            .count();
        header.set_text_content(Some(&format!("{label} ({matching})")));
        let header_el: HtmlElement = header.clone().dyn_into().unwrap();
        header_el.style().set_css_text(
            "font-size: 10px; font-weight: 600; color: var(--text-muted); margin-bottom: 6px; \
             font-family: var(--font-mono);",
        );
        column.append_child(&header).unwrap();
        for record in records
            .iter()
            .filter(|record| field_str(record, &group_by) == *id)
        {
            if let Some(status) = status.as_ref() {
                column
                    .append_child(&record_card(&document, root, &family, status, record))
                    .unwrap();
            }
        }
        board.append_child(&column).unwrap();
    }
    list.append_child(&board).unwrap();
}

fn record_card(
    document: &Document,
    root: &Element,
    family: &str,
    status: &Element,
    record: &serde_json::Value,
) -> Element {
    let card = document.create_element("div").unwrap();
    let card_el: HtmlElement = card.clone().dyn_into().unwrap();
    card_el.style().set_css_text(
        "background: var(--surface-panel); border: 1px solid var(--border-subtle); \
         border-radius: 4px; padding: 8px 10px; margin-bottom: 6px;",
    );
    let title = record
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("(untitled)");
    let id = record
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string();
    let header = document.create_element("div").unwrap();
    header.set_text_content(Some(title));
    card.append_child(&header).unwrap();
    if let Some(fields) = record.get("fields").and_then(serde_json::Value::as_object) {
        let meta = document.create_element("div").unwrap();
        let summary = fields
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(" · ");
        meta.set_text_content(Some(&summary));
        let meta_el: HtmlElement = meta.clone().dyn_into().unwrap();
        meta_el.style().set_css_text(
            "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        card.append_child(&meta).unwrap();
    }
    let delete = document.create_element("button").unwrap();
    delete.set_text_content(Some("Delete"));
    delete.set_attribute("type", "button").ok();
    gate_daemon(&delete, "Delete this COP record.");
    let root_clone = root.clone();
    let status_clone = status.clone();
    let family_owned = family.to_string();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let root_async = root_clone.clone();
        let status_async = status_clone.clone();
        let family_async = family_owned.clone();
        let id_async = id.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let _ = daemon_records_delete(NativeRecordDeleteRequest {
                family: family_async.clone(),
                id: id_async,
            })
            .await;
            refresh(&root_async, &family_async, &status_async);
        });
    }) as Box<dyn FnMut(_)>);
    delete
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    card.append_child(&delete).unwrap();
    card
}

fn parse_columns(encoded: Option<&str>) -> Vec<(String, String)> {
    encoded
        .unwrap_or("")
        .split(',')
        .filter_map(|part| {
            let (id, label) = part.split_once(':')?;
            if id.is_empty() {
                None
            } else {
                Some((id.to_string(), label.to_string()))
            }
        })
        .collect()
}

fn field_str(record: &serde_json::Value, key: &str) -> String {
    record
        .get("fields")
        .and_then(serde_json::Value::as_object)
        .and_then(|fields| fields.get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn style_input(input: &Element) {
    let el: HtmlElement = input.clone().dyn_into().unwrap();
    el.style().set_css_text(
        "flex: 1; min-width: 120px; padding: 4px 6px; background: var(--canvas-bg); \
         border: 1px solid var(--border-subtle); border-radius: 3px; font-size: 10px; \
         font-family: var(--font-mono); color: var(--text-primary);",
    );
}

fn gate_daemon(button: &Element, title: &str) {
    button.set_attribute("data-requires-daemon", "true").ok();
    button.set_attribute("data-enabled-title", title).ok();
    if !is_daemon_connected() {
        button.set_attribute("disabled", "").ok();
        button.set_attribute("aria-disabled", "true").ok();
        button
            .set_attribute("title", "Requires a running local QualiaDB daemon.")
            .ok();
    }
}

fn input_value(root: &Element, selector: &str) -> String {
    root.query_selector(selector)
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}
