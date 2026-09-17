//! Local model catalogue and persisted conversation history for Agent Console.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlInputElement, HtmlTextAreaElement};

use crate::browser::native_daemon::{
    daemon_llm_activate, daemon_llm_evict, daemon_llm_models, daemon_records_query, NativeLlmModel,
    NativeRecordQueryRequest,
};

pub fn build_agent_session_browser(document: &Document, workspace: &Element) -> Element {
    let root = document.create_element("section").unwrap();
    root.set_attribute("data-agent-session-browser", "").ok();
    style(
        &root,
        "border:1px solid var(--border-medium);border-radius:6px;padding:8px;display:grid;grid-template-columns:1fr 1fr;gap:8px;",
    );

    let header = document.create_element("div").unwrap();
    style(
        &header,
        "grid-column:1/-1;display:flex;align-items:center;justify-content:space-between;gap:8px;",
    );
    let title = document.create_element("strong").unwrap();
    title.set_text_content(Some("Models and conversations"));
    header.append_child(&title).unwrap();
    let refresh = button(document, "Refresh");
    header.append_child(&refresh).unwrap();
    let evict = button(document, "Evict resident model");
    header.append_child(&evict).unwrap();
    root.append_child(&header).unwrap();

    let models = document.create_element("div").unwrap();
    models.set_attribute("data-agent-models", "").ok();
    models.set_text_content(Some("No model catalogue loaded."));
    style(
        &models,
        "display:flex;flex-direction:column;gap:4px;max-height:180px;overflow:auto;",
    );
    root.append_child(&models).unwrap();

    let history = document.create_element("div").unwrap();
    history.set_attribute("data-agent-history", "").ok();
    history.set_text_content(Some("No conversation history loaded."));
    style(
        &history,
        "display:flex;flex-direction:column;gap:4px;max-height:180px;overflow:auto;",
    );
    root.append_child(&history).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    style(
        &status,
        "grid-column:1/-1;font:9px var(--font-mono);color:var(--text-muted);",
    );
    root.append_child(&status).unwrap();

    let refresh_root = root.clone();
    let refresh_workspace = workspace.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        refresh_catalogues(&refresh_root, &refresh_workspace);
    }) as Box<dyn FnMut(_)>);
    refresh
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    let evict_root = root.clone();
    let evict_workspace = workspace.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        set_status(&evict_root, "Evicting the resident model…");
        let root = evict_root.clone();
        let workspace = evict_workspace.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match daemon_llm_evict().await {
                Ok(response) if response.ok => {
                    set_status(
                        &root,
                        "Resident model evicted; mapped model memory was released.",
                    );
                    refresh_catalogues(&root, &workspace);
                }
                Ok(response) => set_status(
                    &root,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Model eviction was rejected."),
                ),
                Err(error) => set_status(&root, &error),
            }
        });
    }) as Box<dyn FnMut(_)>);
    evict
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    refresh_catalogues(&root, workspace);
    root
}

fn refresh_catalogues(root: &Element, workspace: &Element) {
    let root = root.clone();
    let workspace = workspace.clone();
    set_status(&root, "Loading local models and persisted turns…");
    wasm_bindgen_futures::spawn_local(async move {
        let models = daemon_llm_models().await;
        let history = daemon_records_query(NativeRecordQueryRequest {
            family: "project_agent".into(),
            query: String::new(),
            kind: "turn".into(),
        })
        .await;

        match models {
            Ok(response) if response.ok => render_models(&root, &workspace, &response.data.models),
            Ok(response) => render_error(
                &root,
                "[data-agent-models]",
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Model discovery failed."),
            ),
            Err(error) => render_error(&root, "[data-agent-models]", &error),
        }
        match history {
            Ok(response) if response.ok => render_history(&root, &workspace, &response.data),
            Ok(response) => render_error(
                &root,
                "[data-agent-history]",
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("History query failed."),
            ),
            Err(error) => render_error(&root, "[data-agent-history]", &error),
        }
        set_status(&root, "Catalogue and conversation history refreshed.");
    });
}

fn render_models(root: &Element, workspace: &Element, models: &[NativeLlmModel]) {
    let Some(list) = root.query_selector("[data-agent-models]").ok().flatten() else {
        return;
    };
    list.set_inner_html("");
    let document = root.owner_document().unwrap();
    let heading = document.create_element("small").unwrap();
    heading.set_text_content(Some("Available local models"));
    list.append_child(&heading).unwrap();
    for model in models {
        let label = format!(
            "{} · {} · {:.1} MiB{}",
            model.name,
            model.format,
            model.bytes as f64 / 1_048_576.0,
            if model.resident { " · resident" } else { "" }
        );
        let row = document.create_element("div").unwrap();
        let select = button(&document, &label);
        select.set_attribute("title", &model.path).ok();
        let path = model.path.clone();
        let select_workspace = workspace.clone();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            set_input(&select_workspace, "model-path", &path);
        }) as Box<dyn FnMut(_)>);
        select
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
        row.append_child(&select).unwrap();
        if !model.resident {
            let activate = button(&document, "Activate");
            let path = model.path.clone();
            let root = root.clone();
            let workspace = workspace.clone();
            let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                set_status(&root, "Validating and memory-mapping the selected model…");
                let root = root.clone();
                let workspace = workspace.clone();
                let path = path.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match daemon_llm_activate(&path).await {
                        Ok(response) if response.ok => {
                            set_input(&workspace, "model-path", &path);
                            set_status(&root, "Model activated as the resident local model.");
                            refresh_catalogues(&root, &workspace);
                        }
                        Ok(response) => set_status(
                            &root,
                            response
                                .diagnostic
                                .as_deref()
                                .unwrap_or("Model activation was rejected."),
                        ),
                        Err(error) => set_status(&root, &error),
                    }
                });
            }) as Box<dyn FnMut(_)>);
            activate
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
            row.append_child(&activate).unwrap();
        }
        list.append_child(&row).unwrap();
    }
    if models.is_empty() {
        let empty = document.create_element("span").unwrap();
        empty.set_text_content(Some(
            "No configured model found. Enter a path or set QUALIA_MODEL_PATHS on the daemon.",
        ));
        list.append_child(&empty).unwrap();
    }
}

fn render_history(root: &Element, workspace: &Element, data: &serde_json::Value) {
    let Some(list) = root.query_selector("[data-agent-history]").ok().flatten() else {
        return;
    };
    list.set_inner_html("");
    let document = root.owner_document().unwrap();
    let heading = document.create_element("small").unwrap();
    heading.set_text_content(Some("Previous local-model turns"));
    list.append_child(&heading).unwrap();
    let records = data
        .get("records")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for record in records.iter().rev().take(32) {
        let fields = record.get("fields").and_then(serde_json::Value::as_object);
        let prompt = string_field(fields, "prompt");
        let model = string_field(fields, "model_path");
        let agent = string_field(fields, "agent_did");
        let conversation = string_field(fields, "conversation");
        let select = button(
            &document,
            &format!("{} · {}", short(agent, 32), short(prompt, 70)),
        );
        select
            .set_attribute("title", "Restore this turn's model, agent and prompt")
            .ok();
        let workspace = workspace.clone();
        let prompt = prompt.to_string();
        let model = model.to_string();
        let agent = agent.to_string();
        let conversation = conversation.to_string();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            set_input(&workspace, "model-path", &model);
            set_input(&workspace, "agent-did", &agent);
            set_input(
                &workspace,
                "conversation-id",
                if conversation.is_empty() {
                    "general"
                } else {
                    &conversation
                },
            );
            if let Some(input) = workspace
                .query_selector("[data-agent-input=\"prompt\"]")
                .ok()
                .flatten()
                .and_then(|element| element.dyn_into::<HtmlTextAreaElement>().ok())
            {
                input.set_value(&prompt);
            }
        }) as Box<dyn FnMut(_)>);
        select
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
        list.append_child(&select).unwrap();
    }
    if records.is_empty() {
        let empty = document.create_element("span").unwrap();
        empty.set_text_content(Some("No local-model turns have been persisted yet."));
        list.append_child(&empty).unwrap();
    }
}

fn string_field<'a>(
    fields: Option<&'a serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> &'a str {
    fields
        .and_then(|fields| fields.get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}

fn set_input(root: &Element, key: &str, value: &str) {
    if let Some(input) = root
        .query_selector(&format!("[data-agent-input=\"{key}\"]"))
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
    {
        input.set_value(value);
    }
}

fn render_error(root: &Element, selector: &str, message: &str) {
    if let Some(element) = root.query_selector(selector).ok().flatten() {
        element.set_text_content(Some(message));
    }
}

fn set_status(root: &Element, message: &str) {
    if let Some(element) = root.query_selector("[role=\"status\"]").ok().flatten() {
        element.set_text_content(Some(message));
    }
}

fn short(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn button(document: &Document, label: &str) -> Element {
    let button = document.create_element("button").unwrap();
    button.set_attribute("type", "button").ok();
    button.set_text_content(Some(label));
    button
}

fn style(element: &Element, css: &str) {
    element
        .clone()
        .dyn_into::<HtmlElement>()
        .unwrap()
        .style()
        .set_css_text(css);
}
