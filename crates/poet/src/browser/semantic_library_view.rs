//! Live standalone Semantic Library backed by the native HypermediaStore.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

use super::native_daemon::{
    daemon_library_ingest, daemon_library_query, daemon_library_stats, NativeLibraryIngestRequest,
    NativeLibraryQueryRequest,
};
use super::semantic_library_render::{render_facets, render_results, show_error};

pub fn build(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("poet-semantic-library");
    root.set_attribute("data-library-section", "all").ok();

    let status = document.create_element("div").unwrap();
    status.set_class_name("vibe-toolbar poet-library-status");
    status.set_attribute("role", "status").ok();
    status.set_text_content(Some("Semantic Library · waiting for the native daemon"));
    root.append_child(&status).unwrap();

    let sections = document.create_element("div").unwrap();
    sections.set_class_name("vibe-toolbar");
    for section in [
        "All", "Secret", "Wellfair", "Personal", "Work", "Tools", "Software", "Commons",
    ] {
        let button = document.create_element("button").unwrap();
        button.set_class_name("vibe-run-btn");
        button.set_text_content(Some(section));
        button.set_attribute("type", "button").ok();
        button
            .set_attribute("data-library-section-tab", &section.to_ascii_lowercase())
            .ok();
        button
            .set_attribute(
                "aria-pressed",
                if section == "All" { "true" } else { "false" },
            )
            .ok();
        let root_clone = root.clone();
        let status_clone = status.clone();
        let section_value = section.to_ascii_lowercase();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            root_clone
                .set_attribute("data-library-section", &section_value)
                .ok();
            update_section_buttons(&root_clone, &section_value);
            spawn_refresh(root_clone.clone(), status_clone.clone());
        }) as Box<dyn FnMut(_)>);
        button
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
        sections.append_child(&button).unwrap();
    }
    root.append_child(&sections).unwrap();

    let query_bar = document.create_element("div").unwrap();
    query_bar.set_class_name("vibe-toolbar");
    let query = document.create_element("input").unwrap();
    query.set_attribute("data-library-query", "").ok();
    query
        .set_attribute("data-state-key", "semantic-library-query")
        .ok();
    query
        .set_attribute(
            "placeholder",
            "Search meaning, topics, projects, places, or media…",
        )
        .ok();
    query.set_attribute("style", "flex:1").ok();
    query_bar.append_child(&query).unwrap();
    let sort = select(
        document,
        "data-library-sort",
        &[
            ("newest", "Newest"),
            ("oldest", "Oldest"),
            ("title_asc", "Title A–Z"),
            ("title_desc", "Title Z–A"),
            ("media_type", "Media type"),
        ],
    );
    query_bar.append_child(&sort).unwrap();
    let search = button(document, "Search");
    daemon_gate(&search, "Query the persistent Semantic Library.");
    query_bar.append_child(&search).unwrap();
    let facets_toggle = button(document, "Facets");
    query_bar.append_child(&facets_toggle).unwrap();
    let ingest_toggle = button(document, "+ Ingest text");
    query_bar.append_child(&ingest_toggle).unwrap();
    root.append_child(&query_bar).unwrap();

    let facets = document.create_element("div").unwrap();
    facets.set_class_name("poet-library-facets");
    facets.set_attribute("hidden", "").ok();
    root.append_child(&facets).unwrap();

    let ingest = build_ingest_form(document, &root, &status);
    root.append_child(&ingest).unwrap();

    let results = document.create_element("div").unwrap();
    results.set_class_name("vibe-output poet-library-results");
    results.set_attribute("aria-live", "polite").ok();
    root.append_child(&results).unwrap();

    wire_query_action(&search, &root, &status);
    let root_clone = root.clone();
    let status_clone = status.clone();
    let key = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        if event.key() == "Enter" {
            spawn_refresh(root_clone.clone(), status_clone.clone());
        }
    }) as Box<dyn FnMut(_)>);
    query
        .add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref())
        .unwrap();
    key.forget();

    let facets_clone = facets.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        if facets_clone.has_attribute("hidden") {
            facets_clone.remove_attribute("hidden").ok();
        } else {
            facets_clone.set_attribute("hidden", "").ok();
        }
    }) as Box<dyn FnMut(_)>);
    facets_toggle
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    let ingest_clone = ingest.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        if ingest_clone.has_attribute("hidden") {
            ingest_clone.remove_attribute("hidden").ok();
        } else {
            ingest_clone.set_attribute("hidden", "").ok();
        }
    }) as Box<dyn FnMut(_)>);
    ingest_toggle
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    spawn_refresh(root.clone(), status);
    root
}

fn build_ingest_form(document: &Document, root: &Element, status: &Element) -> Element {
    let form = document.create_element("div").unwrap();
    form.set_class_name("poet-library-ingest");
    form.set_attribute("hidden", "").ok();
    let uri = input(
        document,
        "data-ingest-uri",
        "Asset URI (for example urn:poet:note:water-plan)",
    );
    let title = input(document, "data-ingest-project", "Project facet (optional)");
    let purpose = input(document, "data-ingest-purpose", "Purpose facet (optional)");
    let section = select(
        document,
        "data-ingest-section",
        &[
            ("personal", "Personal"),
            ("work", "Work"),
            ("wellfair", "Wellfair"),
            ("tools", "Tools"),
            ("software", "Software"),
            ("commons", "Commons"),
        ],
    );
    let sensitivity = select(
        document,
        "data-ingest-sensitivity",
        &[
            ("public", "Public"),
            ("restricted", "Restricted"),
            ("classified", "Classified"),
        ],
    );
    let text = document.create_element("textarea").unwrap();
    text.set_attribute("data-ingest-text", "").ok();
    text.set_attribute(
        "placeholder",
        "Paste Markdown or plain text to derive topics, CML, COF, and searchable semantic Quins…",
    )
    .ok();
    text.set_attribute("maxlength", "1048576").ok();
    form.append_child(&uri).unwrap();
    form.append_child(&section).unwrap();
    form.append_child(&sensitivity).unwrap();
    form.append_child(&title).unwrap();
    form.append_child(&purpose).unwrap();
    form.append_child(&text).unwrap();
    let save = button(document, "Ingest into Semantic Library");
    daemon_gate(&save, "Derive semantic descriptors and persist this text.");
    form.append_child(&save).unwrap();

    let form_clone = form.clone();
    let root_clone = root.clone();
    let status_clone = status.clone();
    let save_clone = save.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let Some(uri) = value::<HtmlInputElement>(&form_clone, "[data-ingest-uri]") else {
            return;
        };
        let Some(text) = value::<HtmlTextAreaElement>(&form_clone, "[data-ingest-text]") else {
            return;
        };
        if uri.trim().is_empty() || text.trim().is_empty() {
            status_clone.set_text_content(Some(
                "Ingestion requires both an asset URI and document text.",
            ));
            return;
        }
        save_clone.set_attribute("disabled", "").ok();
        status_clone.set_text_content(Some(
            "Deriving semantic descriptors and persisting document…",
        ));
        let request = NativeLibraryIngestRequest {
            uri,
            media_type: "text/markdown".into(),
            text,
            section: value::<HtmlSelectElement>(&form_clone, "[data-ingest-section]"),
            sensitivity: value::<HtmlSelectElement>(&form_clone, "[data-ingest-sensitivity]"),
            projects: optional_vec(value::<HtmlInputElement>(
                &form_clone,
                "[data-ingest-project]",
            )),
            purposes: optional_vec(value::<HtmlInputElement>(
                &form_clone,
                "[data-ingest-purpose]",
            )),
            occurred_at: None,
            place_label: None,
            lat: None,
            lon: None,
        };
        let root_async = root_clone.clone();
        let status_async = status_clone.clone();
        let form_async = form_clone.clone();
        let save_async = save_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match daemon_library_ingest(request).await {
                Ok(response) if response.ok => {
                    status_async.set_text_content(Some(
                        "Document ingested; refreshing persistent semantic results…",
                    ));
                    if let Ok(Some(node)) = form_async.query_selector("[data-ingest-text]") {
                        if let Ok(area) = node.dyn_into::<HtmlTextAreaElement>() {
                            area.set_value("");
                        }
                    }
                    spawn_refresh(root_async, status_async);
                }
                Ok(response) => status_async.set_text_content(Some(
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Semantic ingestion failed."),
                )),
                Err(error) => status_async
                    .set_text_content(Some(&format!("Semantic ingestion failed: {error}"))),
            }
            save_async.remove_attribute("disabled").ok();
        });
    }) as Box<dyn FnMut(_)>);
    save.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    form
}

pub(super) fn spawn_refresh(root: Element, status: Element) {
    if !super::native_daemon::is_daemon_connected() {
        status.set_text_content(Some(
            "Semantic Library · probing for the local QualiaDB daemon…",
        ));
        let attempt = root
            .get_attribute("data-library-probe-attempt")
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(0);
        if attempt < 6 {
            root.set_attribute("data-library-probe-attempt", &(attempt + 1).to_string())
                .ok();
            if attempt == 0 {
                super::native_daemon::spawn_daemon_probe();
            }
            let root_retry = root.clone();
            let status_retry = status.clone();
            let closure = Closure::once(move || spawn_refresh(root_retry, status_retry));
            super::interactions::set_timeout(closure.as_ref().unchecked_ref(), 1_000);
            closure.forget();
        }
        return;
    }
    root.remove_attribute("data-library-probe-attempt").ok();
    wasm_bindgen_futures::spawn_local(async move {
        status.set_text_content(Some("Querying the persistent Semantic Library…"));
        let request = query_request(&root);
        let stats = daemon_library_stats().await;
        let query = daemon_library_query(request).await;
        match query {
            Ok(response) if response.ok => {
                render_results(&root, &response.data);
                render_facets(&root, &status, &response.data);
                let total = response
                    .data
                    .get("total")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let stat_data = stats
                    .ok()
                    .filter(|v| v.ok)
                    .map(|v| v.data)
                    .unwrap_or_default();
                let quins = stat_data.get("quins").and_then(|v| v.as_u64()).unwrap_or(0);
                let library_total = stat_data
                    .get("total")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(total);
                status.set_text_content(Some(&format!("{total} shown · {library_total} persistent assets · {quins} semantic Quins · live")));
            }
            Ok(response) => show_error(
                &root,
                &status,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Library query failed."),
            ),
            Err(error) => show_error(&root, &status, &error),
        }
    });
}

fn query_request(root: &Element) -> NativeLibraryQueryRequest {
    let query = value::<HtmlInputElement>(root, "[data-library-query]").unwrap_or_default();
    let selected = |name: &str| {
        root.get_attribute(name)
            .filter(|v| !v.is_empty())
            .into_iter()
            .collect()
    };
    NativeLibraryQueryRequest {
        query,
        section: root.get_attribute("data-library-section"),
        sort: value::<HtmlSelectElement>(root, "[data-library-sort]"),
        topics: selected("data-library-topic"),
        categories: selected("data-library-category"),
        media_types: selected("data-library-media-type"),
        ..Default::default()
    }
}

fn wire_query_action(button: &Element, root: &Element, status: &Element) {
    let root = root.clone();
    let status = status.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        spawn_refresh(root.clone(), status.clone());
    }) as Box<dyn FnMut(_)>);
    button
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

fn update_section_buttons(root: &Element, section: &str) {
    if let Ok(buttons) = root.query_selector_all("[data-library-section-tab]") {
        for index in 0..buttons.length() {
            if let Some(node) = buttons.get(index) {
                if let Ok(button) = node.dyn_into::<Element>() {
                    let active = button.get_attribute("data-library-section-tab").as_deref()
                        == Some(section);
                    button
                        .set_attribute("aria-pressed", if active { "true" } else { "false" })
                        .ok();
                }
            }
        }
    }
}

fn input(document: &Document, attr: &str, placeholder: &str) -> Element {
    let input = document.create_element("input").unwrap();
    input.set_attribute(attr, "").ok();
    input.set_attribute("placeholder", placeholder).ok();
    input
}

fn select(document: &Document, attr: &str, values: &[(&str, &str)]) -> Element {
    let select = document.create_element("select").unwrap();
    select.set_attribute(attr, "").ok();
    for (value, label) in values {
        let option = document.create_element("option").unwrap();
        option.set_attribute("value", value).ok();
        option.set_text_content(Some(label));
        select.append_child(&option).unwrap();
    }
    select
}

fn button(document: &Document, label: &str) -> Element {
    let button = document.create_element("button").unwrap();
    button.set_class_name("vibe-run-btn");
    button.set_attribute("type", "button").ok();
    button.set_text_content(Some(label));
    button
}

fn daemon_gate(button: &Element, title: &str) {
    button.set_attribute("data-requires-daemon", "true").ok();
    button.set_attribute("data-enabled-title", title).ok();
    if !super::native_daemon::is_daemon_connected() {
        button.set_attribute("disabled", "").ok();
        button.set_attribute("aria-disabled", "true").ok();
        button
            .set_attribute("title", "Requires a running local QualiaDB daemon.")
            .ok();
    }
}

fn value<T: JsCast>(root: &Element, selector: &str) -> Option<String>
where
    T: ValueControl,
{
    root.query_selector(selector)
        .ok()
        .flatten()?
        .dyn_into::<T>()
        .ok()
        .map(|v| v.value())
}

trait ValueControl {
    fn value(&self) -> String;
}
impl ValueControl for HtmlInputElement {
    fn value(&self) -> String {
        HtmlInputElement::value(self)
    }
}
impl ValueControl for HtmlSelectElement {
    fn value(&self) -> String {
        HtmlSelectElement::value(self)
    }
}
impl ValueControl for HtmlTextAreaElement {
    fn value(&self) -> String {
        HtmlTextAreaElement::value(self)
    }
}

fn optional_vec(value: Option<String>) -> Vec<String> {
    value
        .into_iter()
        .flat_map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}
