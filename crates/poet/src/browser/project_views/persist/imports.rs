use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlSelectElement, HtmlTextAreaElement};

use super::super::super::cop_records::{
    build_family_panel, CopField,
};
use super::super::super::native_daemon::{
    daemon_records_query, daemon_records_upsert, is_daemon_connected, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};
use super::IMPORT_FAMILIES;

pub fn build_bulk_import_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; overflow: auto; padding: 8px; gap: 8px;",
    );

    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    let status_el: HtmlElement = status.clone().dyn_into().unwrap();
    status_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);");
    status.set_text_content(Some(
        "Paste JSON Lines ({title, fields}) and import into a project family. Invalid lines fail closed.",
    ));
    wrapper.append_child(&status).unwrap();

    let select = document.create_element("select").unwrap();
    select.set_attribute("data-import-family", "true").ok();
    for (family, label) in IMPORT_FAMILIES {
        let option = document.create_element("option").unwrap();
        option.set_attribute("value", family).ok();
        option.set_text_content(Some(&format!("{label} ({family})")));
        select.append_child(&option).unwrap();
    }
    wrapper.append_child(&select).unwrap();

    let area = document.create_element("textarea").unwrap();
    area.set_attribute("data-import-body", "true").ok();
    area.set_attribute(
        "placeholder",
        "{\"title\":\"Review ontology\",\"fields\":{\"status\":\"open\",\"assignee\":\"did:…\"}}",
    )
    .ok();
    let area_el: HtmlElement = area.clone().dyn_into().unwrap();
    area_el.style().set_css_text(
        "min-height: 160px; font-family: var(--font-mono); font-size: 10px; padding: 8px; \
         background: var(--canvas-bg); color: var(--text-primary); border: 1px solid var(--border-subtle);",
    );
    wrapper.append_child(&area).unwrap();

    let save = document.create_element("button").unwrap();
    save.set_text_content(Some("Import JSON Lines"));
    save.set_attribute("type", "button").ok();
    save.set_attribute("data-requires-daemon", "true").ok();
    let export = document.create_element("button").unwrap();
    export.set_text_content(Some("Export selected family as JSON"));
    export.set_attribute("type", "button").ok();
    export.set_attribute("data-requires-daemon", "true").ok();
    if !is_daemon_connected() {
        save.set_attribute("disabled", "").ok();
        save.set_attribute("title", "Requires a running local QualiaDB daemon.")
            .ok();
        export.set_attribute("disabled", "").ok();
        export
            .set_attribute("title", "Requires a running local QualiaDB daemon.")
            .ok();
    }
    wrapper.append_child(&save).unwrap();
    wrapper.append_child(&export).unwrap();

    let wrapper_clone = wrapper.clone();
    let status_clone = status.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let family = wrapper_clone
            .query_selector("[data-import-family]")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<HtmlSelectElement>().ok())
            .map(|select| select.value())
            .unwrap_or_default();
        let body = wrapper_clone
            .query_selector("[data-import-body]")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<HtmlTextAreaElement>().ok())
            .map(|area| area.value())
            .unwrap_or_default();
        if family.is_empty() || body.trim().is_empty() {
            status_clone
                .set_text_content(Some("Select a family and paste at least one JSON line."));
            return;
        }
        status_clone.set_text_content(Some("Importing…"));
        let status_async = status_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let mut saved = 0usize;
            let mut failed = 0usize;
            for (index, line) in body.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let parsed: serde_json::Value = match serde_json::from_str(line) {
                    Ok(value) => value,
                    Err(error) => {
                        failed += 1;
                        status_async.set_text_content(Some(&format!(
                            "Line {} is not JSON: {error}",
                            index + 1
                        )));
                        continue;
                    }
                };
                let title = parsed
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if title.trim().is_empty() {
                    failed += 1;
                    continue;
                }
                let fields = parsed
                    .get("fields")
                    .and_then(serde_json::Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                match daemon_records_upsert(NativeRecordUpsertRequest {
                    family: family.clone(),
                    title,
                    id: None,
                    fields,
                })
                .await
                {
                    Ok(response) if response.ok => saved += 1,
                    _ => failed += 1,
                }
            }
            let _ = daemon_records_upsert(NativeRecordUpsertRequest {
                family: "project_import".to_string(),
                title: format!("import into {family}"),
                id: None,
                fields: serde_json::Map::from_iter([
                    (
                        "target".to_string(),
                        serde_json::Value::String(family.clone()),
                    ),
                    (
                        "saved".to_string(),
                        serde_json::Value::String(saved.to_string()),
                    ),
                    (
                        "failed".to_string(),
                        serde_json::Value::String(failed.to_string()),
                    ),
                ]),
            })
            .await;
            status_async.set_text_content(Some(&format!(
                "Imported {saved} record(s); {failed} line(s) rejected."
            )));
        });
    }) as Box<dyn FnMut(_)>);
    save.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    let wrapper_export = wrapper.clone();
    let status_export = status.clone();
    let export_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let family = wrapper_export
            .query_selector("[data-import-family]")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<HtmlSelectElement>().ok())
            .map(|select| select.value())
            .unwrap_or_default();
        if family.is_empty() {
            status_export.set_text_content(Some("Select a family to export."));
            return;
        }
        status_export.set_text_content(Some("Exporting…"));
        let status_async = status_export.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match daemon_records_query(NativeRecordQueryRequest {
                family: family.clone(),
                query: String::new(),
                kind: String::new(),
            })
            .await
            {
                Ok(response) if response.ok => {
                    let encoded = js_sys::encode_uri_component(
                        &serde_json::to_string_pretty(&response.data).unwrap_or_default(),
                    );
                    let Some(document) = web_sys::window().and_then(|window| window.document())
                    else {
                        status_async.set_text_content(Some("Window unavailable for export."));
                        return;
                    };
                    match document
                        .create_element("a")
                        .ok()
                        .and_then(|element| element.dyn_into::<web_sys::HtmlAnchorElement>().ok())
                    {
                        Some(anchor) => {
                            anchor.set_href(&format!(
                                "data:application/json;charset=utf-8,{encoded}"
                            ));
                            anchor.set_download(&format!("{family}.json"));
                            anchor.click();
                            status_async.set_text_content(Some(&format!(
                                "Exported live `{family}` records as JSON."
                            )));
                        }
                        None => {
                            status_async
                                .set_text_content(Some("Export download could not be created."));
                        }
                    }
                }
                Ok(response) => status_async.set_text_content(Some(
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Export query failed."),
                )),
                Err(error) => status_async.set_text_content(Some(&error)),
            }
        });
    }) as Box<dyn FnMut(_)>);
    export
        .add_event_listener_with_callback("click", export_closure.as_ref().unchecked_ref())
        .unwrap();
    export_closure.forget();

    wrapper
        .append_child(&build_family_panel(
            document,
            "project_import",
            "Import receipts from previous JSON Line runs.",
            &[
                CopField {
                    key: "target",
                    placeholder: "Target family",
                },
                CopField {
                    key: "saved",
                    placeholder: "Saved count",
                },
                CopField {
                    key: "failed",
                    placeholder: "Failed count",
                },
            ],
        ))
        .unwrap();
    wrapper
}
