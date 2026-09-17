//! Project-grounded local LLM console.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlInputElement, HtmlTextAreaElement};

use crate::browser::cop_records::{build_family_panel, CopField};
use crate::browser::native_daemon::{
    close_llm_job_stream, daemon_library_query, daemon_llm_cancel, daemon_llm_start,
    is_daemon_connected, open_llm_job_stream, NativeLibraryQueryRequest, NativeLlmJobEvent,
    NativeLlmRequest,
};

const AGENT_FIELDS: &[CopField] = &[
    CopField {
        key: "kind",
        placeholder: "profile",
    },
    CopField {
        key: "agent_did",
        placeholder: "Agent DID",
    },
    CopField {
        key: "owner_did",
        placeholder: "Controlling principal DID",
    },
    CopField {
        key: "purpose",
        placeholder: "Purpose",
    },
    CopField {
        key: "model_path",
        placeholder: "Local .gguf/.p64 path",
    },
    CopField {
        key: "scope",
        placeholder: "none | all | project:<tag>",
    },
    CopField {
        key: "capabilities",
        placeholder: "local-inference,semantic-library-read",
    },
    CopField {
        key: "max_tokens",
        placeholder: "Maximum tokens per run (1-256)",
    },
];

pub fn build_agent_console_view(document: &Document) -> Element {
    let root = document.create_element("section").unwrap();
    root.set_attribute("data-agent-console", "local-grounded")
        .ok();
    style(
        &root,
        "display:flex;flex-direction:column;gap:9px;padding:10px;overflow:auto;",
    );

    let intro = document.create_element("div").unwrap();
    intro.set_inner_html(
        "<h3 style=\"margin:0 0 4px\">Local model workspace</h3>\
         <p style=\"margin:0;color:var(--text-muted);font-size:10px\">\
         Ask a real local GGUF/P64 model. Optional Semantic Library retrieval is supplied as \
         grounding context. Responses remain model assertions requiring human verification.</p>",
    );
    root.append_child(&intro).unwrap();

    let form = document.create_element("div").unwrap();
    form.set_attribute("data-agent-form", "").ok();
    style(&form, "display:grid;grid-template-columns:1fr 1fr;gap:6px;");
    form.append_child(&input(
        document,
        "model-path",
        "C:\\models\\model.gguf or .p64",
    ))
    .unwrap();
    form.append_child(&input(
        document,
        "agent-did",
        "did:qualia:project-specialist",
    ))
    .unwrap();
    form.append_child(&input(
        document,
        "principal-did",
        "Principal DID (optional)",
    ))
    .unwrap();
    let conversation = input(document, "conversation-id", "Conversation id");
    conversation
        .clone()
        .dyn_into::<HtmlInputElement>()
        .unwrap()
        .set_value("general");
    form.append_child(&conversation).unwrap();
    let token_budget = input(document, "max-tokens", "Max tokens (1-256)");
    token_budget
        .clone()
        .dyn_into::<HtmlInputElement>()
        .unwrap()
        .set_value("256");
    form.append_child(&token_budget).unwrap();
    form.append_child(&input(
        document,
        "library-projects",
        "Semantic Library project tags (comma-separated)",
    ))
    .unwrap();
    let grounding = input(document, "grounding", "");
    grounding.set_attribute("type", "checkbox").ok();
    grounding
        .clone()
        .dyn_into::<HtmlInputElement>()
        .unwrap()
        .set_checked(true);
    let grounding_label = document.create_element("label").unwrap();
    grounding_label.set_text_content(Some(" Retrieve grounding from Semantic Library"));
    grounding_label.prepend_with_node_1(&grounding).ok();
    form.append_child(&grounding_label).unwrap();
    let prompt = document.create_element("textarea").unwrap();
    prompt.set_attribute("data-agent-input", "prompt").ok();
    prompt
        .set_attribute(
            "placeholder",
            "Ask about this project, its records, evidence, or next action…",
        )
        .ok();
    style(&prompt, "grid-column:1/-1;min-height:100px;");
    form.append_child(&prompt).unwrap();
    root.append_child(&form).unwrap();
    root.append_child(&super::agent_session_browser::build_agent_session_browser(
        document, &root,
    ))
    .unwrap();
    root.append_child(&super::agent_run_history::build_agent_run_history(document))
        .unwrap();

    let actions = document.create_element("div").unwrap();
    let ask = button(document, "Ask local model");
    let cancel = button(document, "Cancel run");
    set_button_enabled(&cancel, false);
    actions.append_child(&ask).unwrap();
    actions.append_child(&cancel).unwrap();
    root.append_child(&actions).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    status.set_text_content(Some("Ready. No prompt has been sent."));
    style(
        &status,
        "font:10px var(--font-mono);color:var(--text-muted);",
    );
    root.append_child(&status).unwrap();

    let answer = document.create_element("article").unwrap();
    answer.set_attribute("data-agent-answer", "").ok();
    answer.set_attribute("aria-live", "polite").ok();
    style(
        &answer,
        "white-space:pre-wrap;border:1px solid var(--border-medium);border-radius:6px;padding:10px;min-height:80px;",
    );
    answer.set_text_content(Some("Model output will appear here."));
    root.append_child(&answer).unwrap();

    let evidence = document.create_element("details").unwrap();
    let evidence_title = document.create_element("summary").unwrap();
    evidence_title.set_text_content(Some("Grounding and provenance evidence"));
    evidence.append_child(&evidence_title).unwrap();
    let evidence_body = document.create_element("pre").unwrap();
    evidence_body.set_attribute("data-agent-evidence", "").ok();
    style(
        &evidence_body,
        "white-space:pre-wrap;font-size:9px;max-height:220px;overflow:auto;",
    );
    evidence.append_child(&evidence_body).unwrap();
    root.append_child(&evidence).unwrap();

    root.append_child(&super::agent_review::build_agent_review_queue(document))
        .unwrap();

    let form_click = form.clone();
    let status_click = status.clone();
    let answer_click = answer.clone();
    let evidence_click = evidence_body.clone();
    let root_click = root.clone();
    let ask_click = ask.clone();
    let cancel_click = cancel.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        if !is_daemon_connected() {
            status_click.set_text_content(Some("Unavailable: connect the local QualiaDB daemon."));
            return;
        }
        let model_path = field(&form_click, "model-path");
        let prompt = textarea(&form_click, "prompt");
        if model_path.trim().is_empty() || prompt.trim().is_empty() {
            status_click.set_text_content(Some("Choose a local model and enter a question."));
            return;
        }
        let agent_did = defaulted(
            &field(&form_click, "agent-did"),
            "did:qualia:project-specialist",
        );
        let principal_did = field(&form_click, "principal-did");
        let conversation_id = defaulted(&field(&form_click, "conversation-id"), "general");
        let max_tokens = field(&form_click, "max-tokens")
            .parse::<u32>()
            .ok()
            .filter(|value| (1..=256).contains(value));
        let Some(max_tokens) = max_tokens else {
            status_click.set_text_content(Some("Token budget must be between 1 and 256."));
            return;
        };
        let use_grounding = form_click
            .query_selector("[data-agent-input=\"grounding\"]")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
            .is_some_and(|input| input.checked());
        let library_projects = field(&form_click, "library-projects")
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        set_running(&ask_click, &cancel_click, true);
        status_click.set_text_content(Some("Retrieving context before starting the local model…"));
        answer_click.set_text_content(Some(""));
        evidence_click.set_text_content(Some(""));
        let status = status_click.clone();
        let answer = answer_click.clone();
        let evidence = evidence_click.clone();
        let root = root_click.clone();
        let ask = ask_click.clone();
        let cancel = cancel_click.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let history = super::agent_conversation::load_conversation_context(&conversation_id)
                .await
                .unwrap_or_default();
            let library = if use_grounding {
                match daemon_library_query(NativeLibraryQueryRequest {
                    query: prompt.clone(),
                    projects: library_projects.clone(),
                    ..NativeLibraryQueryRequest::default()
                })
                .await
                {
                    Ok(response) if response.ok => truncate(
                        &serde_json::to_string_pretty(&response.data).unwrap_or_default(),
                        64 * 1024,
                    ),
                    Ok(response) => {
                        status.set_text_content(Some(response.diagnostic.as_deref().unwrap_or(
                            "Semantic Library grounding failed; no model request was sent.",
                        )));
                        answer.set_text_content(Some("No output."));
                        set_running(&ask, &cancel, false);
                        return;
                    }
                    Err(error) => {
                        status.set_text_content(Some(&error));
                        answer.set_text_content(Some("No output."));
                        set_running(&ask, &cancel, false);
                        return;
                    }
                }
            } else {
                String::new()
            };
            let context = truncate(
                &format!(
                    "Conversation: {conversation_id}\n\n{history}\n\nSemantic Library grounding:\n{library}"
                ),
                64 * 1024,
            );
            let start = daemon_llm_start(NativeLlmRequest {
                model_path: model_path.clone(),
                prompt: prompt.clone(),
                graph_context: context.clone(),
                agent_did: agent_did.clone(),
                principal_did,
                max_tokens,
                library_projects,
                library_context_supplied: use_grounding && !library.is_empty(),
            })
            .await;
            let start = match start {
                Ok(response) if response.ok && !response.job_id.is_empty() => response,
                Ok(response) => {
                    status.set_text_content(Some(
                        response
                            .diagnostic
                            .as_deref()
                            .unwrap_or("Local model job did not start."),
                    ));
                    answer.set_text_content(Some("No output."));
                    set_running(&ask, &cancel, false);
                    return;
                }
                Err(error) => {
                    status.set_text_content(Some(&error));
                    answer.set_text_content(Some("No output."));
                    set_running(&ask, &cancel, false);
                    return;
                }
            };
            root.set_attribute("data-active-llm-job", &start.job_id)
                .ok();
            status.set_text_content(Some("Local model started; output will stream here."));
            let event_root = root.clone();
            let event_status = status.clone();
            let event_answer = answer.clone();
            let event_evidence = evidence.clone();
            let event_ask = ask.clone();
            let event_cancel = cancel.clone();
            if let Err(error) = open_llm_job_stream(
                &start.job_id,
                move |event: NativeLlmJobEvent| match event.kind.as_str() {
                    "started" => {
                        event_status.set_text_content(Some("Local model is decoding…"));
                    }
                    "token" => {
                        if let Some(delta) =
                            event.data.get("delta").and_then(|value| value.as_str())
                        {
                            let mut streamed = event_answer.text_content().unwrap_or_default();
                            streamed.push_str(delta);
                            event_answer.set_text_content(Some(&streamed));
                        }
                    }
                    "cancelling" => event_status.set_text_content(Some(
                        "Cancellation requested; waiting for the decoder to stop safely…",
                    )),
                    "cancelled" => {
                        let tokens = event_u32(&event, "tokens_generated");
                        if let Some(partial) = event
                            .data
                            .get("partial_text")
                            .and_then(|value| value.as_str())
                        {
                            event_answer.set_text_content(Some(partial));
                        }
                        event_status.set_text_content(Some(&format!(
                            "Run cancelled after {tokens} generated tokens. Partial output was not saved as a completed turn."
                        )));
                        finish_stream_ui(&event_root, &event_ask, &event_cancel);
                        super::agent_run_history::refresh_all_agent_runs();
                    }
                    "done" => {
                        let text = event_string(&event, "text");
                        let tokens = event_u32(&event, "tokens_generated");
                        let duration = event_u64(&event, "inference_duration_ms");
                        let context_hash = event_u64(&event, "context_hash");
                        let assertion = event_string(&event, "assertion_status");
                        event_answer.set_text_content(Some(&text));
                        event_status.set_text_content(Some(&format!(
                            "Local model completed {tokens} tokens in {duration} ms · {assertion}"
                        )));
                        event_evidence.set_text_content(Some(&format!(
                            "context_hash: {context_hash}\nprovenance_hashes: {}\nrepaired: {}\nchecks: {}\n\ncontext preview:\n{}",
                            event.data.get("provenance_hashes").cloned().unwrap_or_default(),
                            event.data.get("repaired").and_then(|value| value.as_bool()).unwrap_or(false),
                            serde_json::to_string_pretty(event.data.get("checks").unwrap_or(&serde_json::Value::Null)).unwrap_or_default(),
                            truncate(&context, 8 * 1024)
                        )));
                        super::agent_conversation::persist_turn(
                            &conversation_id,
                            &prompt,
                            &text,
                            &agent_did,
                            &model_path,
                            context_hash,
                            tokens,
                        );
                        finish_stream_ui(&event_root, &event_ask, &event_cancel);
                        super::agent_run_history::refresh_all_agent_runs();
                    }
                    "error" => {
                        event_status.set_text_content(Some(
                            event
                                .data
                                .get("diagnostic")
                                .and_then(|value| value.as_str())
                                .unwrap_or("The local model job failed."),
                        ));
                        finish_stream_ui(&event_root, &event_ask, &event_cancel);
                        super::agent_run_history::refresh_all_agent_runs();
                    }
                    _ => {}
                },
            ) {
                status.set_text_content(Some(&error));
                finish_stream_ui(&root, &ask, &cancel);
            }
        });
    }) as Box<dyn FnMut(_)>);
    ask.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    let cancel_root = root.clone();
    let cancel_status = status.clone();
    let cancel_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let Some(job_id) = cancel_root.get_attribute("data-active-llm-job") else {
            return;
        };
        cancel_status.set_text_content(Some("Requesting cooperative cancellation…"));
        let status = cancel_status.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(error) = daemon_llm_cancel(&job_id).await {
                status.set_text_content(Some(&format!("Cancellation request failed: {error}")));
            }
        });
    }) as Box<dyn FnMut(_)>);
    cancel
        .add_event_listener_with_callback("click", cancel_closure.as_ref().unchecked_ref())
        .unwrap();
    cancel_closure.forget();

    root.append_child(&build_family_panel(
        document,
        "project_agent",
        "Define project specialists. Runtime turns are local, read-only and separately provenance-labelled.",
        AGENT_FIELDS,
    ))
    .unwrap();
    root
}

fn input(document: &Document, key: &str, placeholder: &str) -> Element {
    let input = document.create_element("input").unwrap();
    input.set_attribute("data-agent-input", key).ok();
    if !placeholder.is_empty() {
        input.set_attribute("placeholder", placeholder).ok();
    }
    input
}

fn field(root: &Element, key: &str) -> String {
    root.query_selector(&format!("[data-agent-input=\"{key}\"]"))
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}

fn textarea(root: &Element, key: &str) -> String {
    root.query_selector(&format!("[data-agent-input=\"{key}\"]"))
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}

fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

fn defaulted(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.into()
    } else {
        value.trim().into()
    }
}

fn button(document: &Document, label: &str) -> Element {
    let button = document.create_element("button").unwrap();
    button.set_attribute("type", "button").ok();
    button.set_text_content(Some(label));
    button
}

fn set_button_enabled(button: &Element, enabled: bool) {
    if enabled {
        button.remove_attribute("disabled").ok();
        button.set_attribute("aria-disabled", "false").ok();
    } else {
        button.set_attribute("disabled", "").ok();
        button.set_attribute("aria-disabled", "true").ok();
    }
}

fn set_running(ask: &Element, cancel: &Element, running: bool) {
    set_button_enabled(ask, !running);
    set_button_enabled(cancel, running);
}

fn finish_stream_ui(root: &Element, ask: &Element, cancel: &Element) {
    root.remove_attribute("data-active-llm-job").ok();
    set_running(ask, cancel, false);
    close_llm_job_stream();
}

fn event_string(event: &NativeLlmJobEvent, key: &str) -> String {
    event
        .data
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

fn event_u64(event: &NativeLlmJobEvent, key: &str) -> u64 {
    event
        .data
        .get(key)
        .and_then(|value| value.as_u64())
        .unwrap_or(0)
}

fn event_u32(event: &NativeLlmJobEvent, key: &str) -> u32 {
    event_u64(event, key).min(u32::MAX as u64) as u32
}

fn style(element: &Element, css: &str) {
    element
        .clone()
        .dyn_into::<HtmlElement>()
        .unwrap()
        .style()
        .set_css_text(css);
}
