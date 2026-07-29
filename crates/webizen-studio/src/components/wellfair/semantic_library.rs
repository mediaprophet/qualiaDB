//! Naturalised Semantic Library.
//!
//! This is the primary, human-facing surface for the HypermediaStore. The
//! existing `library_panel` remains the Advanced Technical workbench.

use super::host_client::{ingest_document, library_stats, query_library_faceted, IngestFacets};
use crate::components::qapp_engine::invoke_json;
use crate::Route;
use dioxus::prelude::*;

mod views;
use views::{EmptyCollection, ItemInspector, LibraryOverview, SemanticPipeline};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Collection {
    Overview,
    All,
    Documents,
    Ontologies,
    Models,
    Health,
    Finance,
    QApps,
    Media,
}

impl Collection {
    const ALL: [Self; 9] = [
        Self::Overview,
        Self::All,
        Self::Documents,
        Self::Ontologies,
        Self::Models,
        Self::Health,
        Self::Finance,
        Self::QApps,
        Self::Media,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::All => "All items",
            Self::Documents => "Documents",
            Self::Ontologies => "Ontologies",
            Self::Models => "AI models",
            Self::Health => "Health records",
            Self::Finance => "Finance",
            Self::QApps => "QApps",
            Self::Media => "Images & audio",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Overview => "grid-1x2",
            Self::All => "collection",
            Self::Documents => "file-earmark-text",
            Self::Ontologies => "diagram-3",
            Self::Models => "cpu",
            Self::Health => "heart-pulse",
            Self::Finance => "receipt",
            Self::QApps => "boxes",
            Self::Media => "images",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ImportKind {
    Ontology,
    Pdf,
    Image,
    Text,
    Tabular,
    Model,
    Unsupported,
}

fn classify_path(path: &str) -> ImportKind {
    let ext = path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "ttl" | "rdf" | "owl" | "n3" | "nt" | "jsonld" | "trig" => ImportKind::Ontology,
        "pdf" => ImportKind::Pdf,
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "tif" | "tiff" => ImportKind::Image,
        "txt" | "md" | "html" | "htm" | "epub" | "doc" | "docx" | "odt" => ImportKind::Text,
        "csv" | "tsv" | "xls" | "xlsx" | "ods" => ImportKind::Tabular,
        "gguf" | "p64" => ImportKind::Model,
        _ => ImportKind::Unsupported,
    }
}

fn text_field(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|field| field.as_str())
        .unwrap_or_default()
        .to_string()
}

fn string_list(value: &serde_json::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|field| field.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn searchable_text(row: &serde_json::Value) -> String {
    let mut parts = vec![
        text_field(row, "asset_uri"),
        text_field(row, "media_type"),
        text_field(row, "excerpt"),
        text_field(row, "section"),
    ];
    for key in ["topics", "projects", "purposes", "depicts"] {
        parts.extend(string_list(row, key));
    }
    parts.join(" ").to_ascii_lowercase()
}

fn belongs_to(row: &serde_json::Value, collection: Collection) -> bool {
    if matches!(collection, Collection::Overview | Collection::All) {
        return true;
    }
    let hay = searchable_text(row);
    match collection {
        Collection::Documents => {
            hay.contains("text/")
                || hay.contains(".pdf")
                || hay.contains(".doc")
                || hay.contains("document")
        }
        Collection::Ontologies => {
            hay.contains("ontology")
                || hay.contains(".ttl")
                || hay.contains(".owl")
                || hay.contains("application/rdf")
        }
        Collection::Models => {
            hay.contains("model://") || hay.contains(".gguf") || hay.contains(".p64")
        }
        Collection::Health => {
            hay.contains("health")
                || hay.contains("pathology")
                || hay.contains("fhir")
                || hay.contains("loinc")
                || hay.contains("icd")
                || hay.contains("anatom")
                || hay.contains("wellfair")
        }
        Collection::Finance => {
            hay.contains("invoice")
                || hay.contains("receipt")
                || hay.contains("finance")
                || hay.contains("expense")
        }
        Collection::QApps => hay.contains("qapp") || hay.contains("software"),
        Collection::Media => hay.contains("image/") || hay.contains("audio/"),
        Collection::Overview | Collection::All => true,
    }
}

fn display_name(row: &serde_json::Value) -> String {
    let uri = text_field(row, "asset_uri");
    uri.rsplit(['/', '\\', ':'])
        .find(|part| !part.trim().is_empty())
        .unwrap_or("Untitled item")
        .to_string()
}

fn graph_count(stats: &serde_json::Value) -> u64 {
    stats
        .get("quins")
        .and_then(|value| value.as_u64())
        .unwrap_or_default()
}

async fn register_import_receipt(
    path: &str,
    kind: ImportKind,
    processor_result: &serde_json::Value,
) -> Result<(), String> {
    let label = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let purpose = match kind {
        ImportKind::Ontology => "ontology",
        ImportKind::Model => "model",
        ImportKind::Pdf | ImportKind::Text => "document",
        ImportKind::Image => "media",
        ImportKind::Tabular => "tabular-data",
        ImportKind::Unsupported => "file",
    };
    let text = format!(
        "# Imported source\n\nFile: {label}\nSource: {path}\nSemantic role: {purpose}\nProcessor receipt: {processor_result}"
    );
    let facets = IngestFacets {
        purpose: Some(purpose.to_string()),
        section: Some("personal".to_string()),
        ..IngestFacets::default()
    };
    ingest_document(
        &format!("file://{path}"),
        "text/markdown",
        &text,
        None,
        &facets,
        "private",
    )
    .await
    .map(|_| ())
}

async fn process_selected_file(path: &str) -> Result<String, String> {
    let kind = classify_path(path);
    if kind == ImportKind::Model {
        return Ok(
            "Model recognised. Open AI models to map, validate, and activate it without copying the weights."
                .to_string(),
        );
    }
    if kind == ImportKind::Tabular {
        return Ok(
            "Tabular file recognised. Choose its domain mapping (for example finance or health) before graph conversion."
                .to_string(),
        );
    }
    if kind == ImportKind::Unsupported {
        return Err("This file type has no registered semantic processor yet.".to_string());
    }

    let (command, args) = match kind {
        ImportKind::Ontology => ("ingest_ontology", serde_json::json!({ "fileName": path })),
        ImportKind::Pdf => ("ingest_pdf", serde_json::json!({ "fileName": path })),
        ImportKind::Image => ("ingest_image", serde_json::json!({ "filePath": path })),
        ImportKind::Text => ("ingest_literature", serde_json::json!({ "filePath": path })),
        _ => unreachable!(),
    };
    let receipt = invoke_json(command, args).await?;
    register_import_receipt(path, kind, &receipt).await?;
    Ok(format!(
        "Processed by {command}; its graph document and provenance receipt are now in the library."
    ))
}

#[component]
pub fn SemanticLibrary() -> Element {
    let mut collection = use_signal(|| Collection::Overview);
    let mut rows = use_signal(Vec::<serde_json::Value>::new);
    let mut stats = use_signal(|| serde_json::json!({}));
    let mut search = use_signal(String::new);
    let mut status = use_signal(String::new);
    let mut status_error = use_signal(|| false);
    let mut selected = use_signal(|| None::<serde_json::Value>);
    let mut importing = use_signal(|| false);

    let refresh = move || {
        spawn(async move {
            match query_library_faceted(&serde_json::json!({}), "newest").await {
                Ok(value) => {
                    rows.set(
                        value
                            .get("entries")
                            .and_then(|entries| entries.as_array())
                            .cloned()
                            .unwrap_or_default(),
                    );
                    status_error.set(false);
                }
                Err(error) => {
                    if crate::endpoints::is_native_host() {
                        status_error.set(true);
                        status.set(format!(
                            "The desktop library service is unavailable: {error}"
                        ));
                    } else {
                        // Static previews demonstrate the information architecture;
                        // native commands are verified separately by desktop contracts.
                        status_error.set(false);
                        status.set(String::new());
                    }
                }
            }
            if let Ok(value) = library_stats().await {
                stats.set(value);
            }
        });
    };

    let mut loaded = use_signal(|| false);
    use_effect(move || {
        if !loaded() {
            loaded.set(true);
            refresh();
        }
    });

    let import_file = move |_| {
        importing.set(true);
        status.set("Choose a source file…".to_string());
        spawn(async move {
            let picked = invoke_json("wellfair_pick_file_path", serde_json::json!({})).await;
            match picked {
                Ok(serde_json::Value::String(path)) => match process_selected_file(&path).await {
                    Ok(message) => {
                        status_error.set(false);
                        status.set(message);
                        refresh();
                    }
                    Err(error) => {
                        status_error.set(true);
                        status.set(format!("Import needs attention: {error}"));
                    }
                },
                Ok(serde_json::Value::Null) => status.set("Import cancelled.".to_string()),
                Ok(other) => {
                    status_error.set(true);
                    status.set(format!("Unexpected file-picker response: {other}"));
                }
                Err(error) => {
                    status_error.set(true);
                    status.set(format!("Could not open the desktop file picker: {error}"));
                }
            }
            importing.set(false);
        });
    };

    let query = search().trim().to_ascii_lowercase();
    let active = collection();
    let visible: Vec<_> = rows()
        .into_iter()
        .filter(|row| belongs_to(row, active))
        .filter(|row| query.is_empty() || searchable_text(row).contains(&query))
        .collect();
    let all_rows = rows();
    let count = |kind| all_rows.iter().filter(|row| belongs_to(row, kind)).count();

    rsx! {
        div {
            style: "height:100%;min-height:0;display:grid;grid-template-columns:240px minmax(0,1fr) 310px;background:var(--qualia-bg);color:var(--qualia-text);",
            aside {
                style: "min-height:0;overflow:auto;border-right:1px solid var(--qualia-border);padding:20px 14px;background:color-mix(in srgb,var(--qualia-surface) 86%,transparent);",
                div { style: "padding:0 10px 18px;",
                    div { style: "font-size:.68rem;text-transform:uppercase;letter-spacing:.12em;color:var(--qualia-accent);font-weight:800;", "Hypermedia" }
                    h1 { style: "font-size:1.28rem;margin:5px 0 6px;letter-spacing:-.03em;", "Semantic Library" }
                    p { style: "font-size:.75rem;line-height:1.45;color:var(--qualia-text-muted);margin:0;", "Files become connected graph documents—not isolated uploads." }
                }
                nav { aria_label: "Library collections", style: "display:flex;flex-direction:column;gap:4px;",
                    for item in Collection::ALL {
                        button {
                            key: "{item.label()}",
                            r#type: "button",
                            onclick: move |_| {
                                collection.set(item);
                                selected.set(None);
                            },
                            style: if active == item {
                                "display:flex;align-items:center;gap:10px;width:100%;padding:10px 11px;border:1px solid color-mix(in srgb,var(--qualia-accent) 42%,transparent);border-radius:10px;background:var(--qualia-accent-glow);color:var(--qualia-text);font:inherit;font-size:.8rem;font-weight:750;cursor:pointer;text-align:left;"
                            } else {
                                "display:flex;align-items:center;gap:10px;width:100%;padding:10px 11px;border:1px solid transparent;border-radius:10px;background:transparent;color:var(--qualia-text-muted);font:inherit;font-size:.8rem;font-weight:650;cursor:pointer;text-align:left;"
                            },
                            sl-icon { "name": item.icon() }
                            span { style: "flex:1;", "{item.label()}" }
                            if item != Collection::Overview {
                                span { style: "font-size:.66rem;opacity:.65;", "{count(item)}" }
                            }
                        }
                    }
                }
                div { style: "margin:20px 8px 0;padding:12px;border:1px solid var(--qualia-border);border-radius:12px;background:rgba(127,127,127,.05);",
                    div { style: "font-size:.73rem;font-weight:750;margin-bottom:5px;", "Private by default" }
                    div { style: "font-size:.68rem;line-height:1.45;color:var(--qualia-text-muted);", "Every import keeps its source, graph provenance, sensitivity, and sharing state distinct." }
                }
            }
            main { style: "min-width:0;min-height:0;overflow:auto;padding:24px 26px 40px;",
                div { style: "display:flex;align-items:flex-start;justify-content:space-between;gap:18px;margin-bottom:20px;",
                    div {
                        h2 { style: "font-size:1.45rem;margin:0 0 6px;letter-spacing:-.035em;", "{active.label()}" }
                        p { style: "margin:0;color:var(--qualia-text-muted);font-size:.8rem;", "Search the source, its meaning, or the ontology concepts it contains." }
                    }
                    div { style: "display:flex;gap:8px;flex-wrap:wrap;justify-content:flex-end;",
                        button {
                            r#type: "button",
                            disabled: importing(),
                            onclick: import_file,
                            style: "border:0;border-radius:10px;background:var(--qualia-accent);color:white;padding:10px 14px;font:inherit;font-size:.78rem;font-weight:800;cursor:pointer;",
                            sl-icon { "name": "plus-lg", style: "margin-right:6px;" }
                            if importing() { "Processing…" } else { "Import files" }
                        }
                        Link {
                            to: Route::QAppsRoute {},
                            style: "border:1px solid var(--qualia-border);border-radius:10px;color:var(--qualia-text);padding:9px 13px;font-size:.78rem;font-weight:750;text-decoration:none;",
                            "Run a QApp"
                        }
                    }
                }
                div { style: "position:relative;margin-bottom:16px;",
                    sl-icon { "name": "search", style: "position:absolute;left:13px;top:11px;color:var(--qualia-text-muted);" }
                    input {
                        r#type: "search",
                        aria_label: "Search semantic library",
                        placeholder: "Search files, concepts, projects, people, dates…",
                        value: "{search}",
                        oninput: move |event| search.set(event.value()),
                        style: "width:100%;box-sizing:border-box;border:1px solid var(--qualia-border);border-radius:12px;background:var(--qualia-surface);color:var(--qualia-text);padding:11px 14px 11px 39px;font:inherit;font-size:.82rem;outline:none;",
                    }
                }
                if !status().is_empty() {
                    div {
                        role: "status",
                        style: if status_error() {
                            "margin-bottom:16px;padding:11px 13px;border:1px solid #ef4444;border-radius:10px;background:rgba(239,68,68,.08);color:#fca5a5;font-size:.75rem;"
                        } else {
                            "margin-bottom:16px;padding:11px 13px;border:1px solid #22c55e;border-radius:10px;background:rgba(34,197,94,.08);color:#86efac;font-size:.75rem;"
                        },
                        "{status}"
                    }
                }
                if active == Collection::Overview {
                    LibraryOverview {
                        documents: count(Collection::Documents),
                        ontologies: count(Collection::Ontologies),
                        models: count(Collection::Models),
                        facts: graph_count(&stats()),
                    }
                } else if visible.is_empty() {
                    EmptyCollection { collection: active.label().to_string() }
                } else {
                    div { style: "display:grid;grid-template-columns:repeat(auto-fill,minmax(260px,1fr));gap:10px;",
                        for row in visible {
                            {
                                let row_for_click = row.clone();
                                let topics = string_list(&row, "topics");
                                let media = text_field(&row, "media_type");
                                let uri = text_field(&row, "asset_uri");
                                rsx! {
                                    button {
                                        key: "{uri}",
                                        r#type: "button",
                                        onclick: move |_| selected.set(Some(row_for_click.clone())),
                                        style: "text-align:left;border:1px solid var(--qualia-border);border-radius:13px;background:var(--qualia-surface);color:var(--qualia-text);padding:14px;font:inherit;cursor:pointer;min-width:0;",
                                        div { style: "display:flex;align-items:flex-start;gap:10px;",
                                            div { style: "width:34px;height:34px;border-radius:9px;background:var(--qualia-accent-glow);display:grid;place-items:center;color:var(--qualia-accent);flex:0 0 auto;",
                                                sl-icon { "name": "file-earmark-richtext" }
                                            }
                                            div { style: "min-width:0;",
                                                div { style: "font-size:.82rem;font-weight:780;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;", "{display_name(&row)}" }
                                                div { style: "font-size:.67rem;color:var(--qualia-text-muted);margin-top:3px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;", "{media}" }
                                            }
                                        }
                                        p { style: "font-size:.71rem;color:var(--qualia-text-muted);line-height:1.45;margin:11px 0 8px;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;", {text_field(&row, "excerpt")} }
                                        div { style: "display:flex;gap:5px;flex-wrap:wrap;",
                                            for topic in topics.into_iter().take(3) {
                                                span { style: "font-size:.62rem;padding:3px 6px;border-radius:999px;background:rgba(127,127,127,.08);color:var(--qualia-text-muted);", "{topic}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            aside { style: "min-height:0;overflow:auto;border-left:1px solid var(--qualia-border);padding:22px 18px;background:color-mix(in srgb,var(--qualia-surface) 76%,transparent);",
                if let Some(item) = selected() {
                    ItemInspector { item }
                } else {
                    SemanticPipeline {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_classifier_routes_semantic_processors() {
        assert_eq!(classify_path("lab/pathology.PDF"), ImportKind::Pdf);
        assert_eq!(classify_path("schema/FHIR.ttl"), ImportKind::Ontology);
        assert_eq!(classify_path("models/local.gguf"), ImportKind::Model);
        assert_eq!(classify_path("accounts/invoices.csv"), ImportKind::Tabular);
    }

    #[test]
    fn semantic_collections_use_meaning_not_only_media_type() {
        let row = serde_json::json!({
            "asset_uri": "file://results.pdf",
            "media_type": "text/markdown",
            "topics": ["LOINC", "pathology"],
            "section": "wellfair"
        });
        assert!(belongs_to(&row, Collection::Health));
        assert!(!belongs_to(&row, Collection::Finance));
    }
}
