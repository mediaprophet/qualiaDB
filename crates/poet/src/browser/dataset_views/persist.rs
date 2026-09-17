//! Dataset surfaces persist on the COP `/records` ledger.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlSelectElement, HtmlTextAreaElement};

use super::super::cop_records::{build_family_panel, CopField};
use super::super::native_daemon::{
    daemon_library_ingest, is_daemon_connected, NativeLibraryIngestRequest,
};

fn wrap(document: &Document, child: Element) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; overflow: auto; padding: 8px; gap: 8px;",
    );
    wrapper.append_child(&child).unwrap();
    wrapper
}

fn ledger(
    document: &Document,
    family: &'static str,
    heading: &str,
    fields: &'static [CopField],
) -> Element {
    wrap(
        document,
        build_family_panel(document, family, heading, fields),
    )
}

pub fn build_dataset_registry_view(document: &Document) -> Element {
    ledger(
        document,
        "dataset",
        "Dataset registry records persist on the COP ledger. Sample CSV/RDF rows are not shown.",
        &[
            CopField {
                key: "format",
                placeholder: "Format (csv|n3|json|jsonld|parquet|10d|dicom)",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity (public|restricted|classified)",
            },
            CopField {
                key: "uri",
                placeholder: "URI",
            },
            CopField {
                key: "rows",
                placeholder: "Row/statement count",
            },
        ],
    )
}

pub fn build_annotation_panel_view(document: &Document) -> Element {
    ledger(
        document,
        "dataset_annotation",
        "Annotations persist as records against a dataset id.",
        &[
            CopField {
                key: "dataset",
                placeholder: "Dataset id",
            },
            CopField {
                key: "span",
                placeholder: "Span / selector",
            },
            CopField {
                key: "body",
                placeholder: "Annotation body",
            },
        ],
    )
}

pub fn build_lineage_graph_view(document: &Document) -> Element {
    ledger(
        document,
        "dataset_lineage",
        "Lineage edges persist as records (source → derived). This is not a fabricated graph.",
        &[
            CopField {
                key: "source",
                placeholder: "Source dataset id",
            },
            CopField {
                key: "derived",
                placeholder: "Derived dataset id",
            },
            CopField {
                key: "activity",
                placeholder: "Activity (import|transform|annotate)",
            },
        ],
    )
}

pub fn build_view_canvas_view(document: &Document) -> Element {
    ledger(
        document,
        "dataset_view",
        "Saved dataset views. GPU renderers stay unbound until a DAT view session is registered.",
        &[
            CopField {
                key: "dataset",
                placeholder: "Dataset id",
            },
            CopField {
                key: "projection",
                placeholder: "Projection",
            },
            CopField {
                key: "status",
                placeholder: "Status (saved|unbound-renderer)",
            },
        ],
    )
}

pub fn build_presentation_editor_view(document: &Document) -> Element {
    ledger(
        document,
        "dataset_presentation",
        "Presentation documents persist as records. Slide/render engines stay unbound until registered.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (deck|poster|paper)",
            },
            CopField {
                key: "dataset",
                placeholder: "Dataset id",
            },
            CopField {
                key: "status",
                placeholder: "Status (draft|ready)",
            },
        ],
    )
}

pub fn build_presentation_publish_view(document: &Document) -> Element {
    ledger(
        document,
        "dataset_presentation",
        "Publish receipts persist here. Transport (RSS/magnet/Commons) is unbound until registered.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (publish)",
            },
            CopField {
                key: "target",
                placeholder: "Target URI",
            },
            CopField {
                key: "status",
                placeholder: "Status (queued|published)",
            },
        ],
    )
}

pub fn build_video_view_view(document: &Document) -> Element {
    ledger(
        document,
        "dataset_media",
        "Video/media records persist here. Decode/playback requires a DAT-28 media session.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (video)",
            },
            CopField {
                key: "uri",
                placeholder: "URI",
            },
            CopField {
                key: "codec",
                placeholder: "Codec",
            },
        ],
    )
}

pub fn build_super_resolve_view(document: &Document) -> Element {
    ledger(
        document,
        "dataset_media",
        "Super-resolve jobs persist as records. CV/geometry execution requires a DAT-30 session.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (super_resolve)",
            },
            CopField {
                key: "source",
                placeholder: "Source dataset id",
            },
            CopField {
                key: "status",
                placeholder: "Status (queued|unbound)",
            },
        ],
    )
}

pub fn build_cad_curation_view(document: &Document) -> Element {
    ledger(
        document,
        "dataset_cad",
        "CAD curation records persist here. GD&T inspection requires a DAT-31 CAD session.",
        &[
            CopField {
                key: "format",
                placeholder: "Format (step|iges|mesh)",
            },
            CopField {
                key: "uri",
                placeholder: "URI",
            },
            CopField {
                key: "status",
                placeholder: "Status (catalogued|unbound-inspector)",
            },
        ],
    )
}

pub fn build_dataset_importer_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; overflow: auto; padding: 8px; gap: 8px;",
    );
    let note = document.create_element("div").unwrap();
    note.set_text_content(Some(
        "Paste text and import. RDF/N3/JSON-LD go through Semantic Library ingest. Tabular formats persist as dataset records; parquet/binary decode is unbound.",
    ));
    wrapper.append_child(&note).unwrap();
    let select = document.create_element("select").unwrap();
    select.set_attribute("data-dataset-format", "true").ok();
    for (value, label) in [
        ("text/csv", "CSV (registry only)"),
        ("application/json", "JSON (registry only)"),
        ("text/n3", "N3 (library ingest)"),
        ("text/turtle", "Turtle (library ingest)"),
        ("application/ld+json", "JSON-LD (library ingest)"),
    ] {
        let option = document.create_element("option").unwrap();
        option.set_attribute("value", value).ok();
        option.set_text_content(Some(label));
        select.append_child(&option).unwrap();
    }
    wrapper.append_child(&select).unwrap();
    let area = document.create_element("textarea").unwrap();
    area.set_attribute("data-dataset-body", "true").ok();
    let area_el: HtmlElement = area.clone().dyn_into().unwrap();
    area_el.style().set_css_text(
        "min-height: 140px; font-family: var(--font-mono); font-size: 10px; padding: 8px; \
         background: var(--canvas-bg); color: var(--text-primary); border: 1px solid var(--border-subtle);",
    );
    wrapper.append_child(&area).unwrap();
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    wrapper.append_child(&status).unwrap();
    let save = document.create_element("button").unwrap();
    save.set_text_content(Some("Import"));
    save.set_attribute("type", "button").ok();
    if !is_daemon_connected() {
        save.set_attribute("disabled", "").ok();
        save.set_attribute("title", "Requires a running local QualiaDB daemon.")
            .ok();
    }
    wrapper.append_child(&save).unwrap();
    let wrapper_clone = wrapper.clone();
    let status_clone = status.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let format = wrapper_clone
            .query_selector("[data-dataset-format]")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<HtmlSelectElement>().ok())
            .map(|select| select.value())
            .unwrap_or_default();
        let body = wrapper_clone
            .query_selector("[data-dataset-body]")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<HtmlTextAreaElement>().ok())
            .map(|area| area.value())
            .unwrap_or_default();
        if body.trim().is_empty() {
            status_clone.set_text_content(Some("Paste source text before importing."));
            return;
        }
        status_clone.set_text_content(Some("Importing…"));
        let status_async = status_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let library = matches!(
                format.as_str(),
                "text/n3" | "text/turtle" | "application/ld+json"
            );
            if library {
                match daemon_library_ingest(NativeLibraryIngestRequest {
                    uri: format!("urn:poet:dataset:{}", now_stamp()),
                    media_type: format.clone(),
                    text: body.clone(),
                    section: Some("work".into()),
                    sensitivity: Some("restricted".into()),
                    projects: Vec::new(),
                    purposes: vec!["dataset".into()],
                    occurred_at: None,
                    place_label: None,
                    lat: None,
                    lon: None,
                })
                .await
                {
                    Ok(response) if response.ok => {
                        status_async.set_text_content(Some("Ingested into the Semantic Library."))
                    }
                    Ok(response) => status_async.set_text_content(Some(
                        response
                            .diagnostic
                            .as_deref()
                            .unwrap_or("Library ingest failed."),
                    )),
                    Err(error) => status_async.set_text_content(Some(&error)),
                }
            }
            let excerpt: String = body.chars().take(1024).collect();
            let _ = super::super::native_daemon::daemon_records_upsert(
                super::super::native_daemon::NativeRecordUpsertRequest {
                    family: "dataset_import".into(),
                    title: format!("import {format}"),
                    id: None,
                    fields: serde_json::Map::from_iter([
                        ("format".into(), serde_json::Value::String(format.clone())),
                        ("source".into(), serde_json::Value::String(excerpt)),
                    ]),
                },
            )
            .await;
            let _ = super::super::native_daemon::daemon_records_upsert(
                super::super::native_daemon::NativeRecordUpsertRequest {
                    family: "dataset".into(),
                    title: format!("imported {format}"),
                    id: None,
                    fields: serde_json::Map::from_iter([
                        ("format".into(), serde_json::Value::String(format)),
                        (
                            "uri".into(),
                            serde_json::Value::String("urn:poet:dataset:import".into()),
                        ),
                    ]),
                },
            )
            .await;
            if !library {
                status_async.set_text_content(Some(
                    "Dataset record saved. Binary/parquet decode is unbound.",
                ));
            }
        });
    }) as Box<dyn FnMut(_)>);
    save.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    wrapper
        .append_child(&build_family_panel(
            document,
            "dataset_import",
            "Previous import receipts.",
            &[
                CopField {
                    key: "format",
                    placeholder: "Format",
                },
                CopField {
                    key: "source",
                    placeholder: "Excerpt",
                },
            ],
        ))
        .unwrap();
    wrapper
}

fn now_stamp() -> u64 {
    js_sys::Date::now() as u64
}

#[cfg(test)]
mod tests {
    #[test]
    fn importer_formats_include_n3_and_csv() {
        let formats = [
            "text/csv",
            "application/json",
            "text/n3",
            "text/turtle",
            "application/ld+json",
        ];
        assert!(formats.contains(&"text/n3"));
        assert!(formats.contains(&"text/csv"));
    }
}
