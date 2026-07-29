//! Dialectical sidebar pane component.

use super::shared::*;

// ── Dialectical Sidebar Pane (Web Annotations & Semantic Manifold) ──
#[component]
pub fn DialecticalSidebarPane(active_url: String) -> Element {
    let mut message = use_signal(String::new);
    let mut permission = use_signal(|| "permissive".to_string());
    let mut status = use_signal(String::new);
    let mut target_fragment = use_signal(String::new);
    let mut auth_uri = use_signal(String::new);
    let mut show_cml = use_signal(|| false);

    let mut thread_target = use_signal(|| active_url.clone());
    let mut annotations = use_signal(|| Vec::<serde_json::Value>::new());

    use_effect({
        let active_url = active_url.clone();
        move || {
            thread_target.set(active_url.clone());
        }
    });

    let add_annotation = move |_| {
        let body = message().trim().to_string();
        if body.is_empty() {
            status.set("Write a note before adding it to the semantic thread.".to_string());
            return;
        }
        let fragment = target_fragment().trim().to_string();
        let target = if fragment.is_empty() {
            thread_target()
        } else {
            format!("{}#{}", thread_target().trim_end_matches('#'), fragment)
        };
        annotations.with_mut(|items| {
            items.push(serde_json::json!({
                "target": target,
                "body": body,
                "permission": permission(),
                "authority": auth_uri().trim(),
                "cml": show_cml(),
            }));
        });
        message.set(String::new());
        target_fragment.set(String::new());
        status.set("Annotation added to this local semantic thread.".to_string());
    };

    rsx! {
        div {
            class: "w-80 border-l border-border/50 bg-black/40 flex flex-col overflow-hidden",
            div {
                class: "p-3 border-b border-border/40 text-sm font-semibold text-text-main",
                "Dialectical sidebar"
            }
            div {
                class: "p-3 text-xs text-text-muted leading-relaxed flex-1 overflow-y-auto",
                p { "Attach locally scoped semantic annotations to the active URL." }
                p { class: "mt-2 font-mono break-all text-text-main/80", "{active_url}" }
                label {
                    class: "mt-3 block",
                    "Permission lane"
                    select {
                        class: "mt-1 w-full bg-black/30 border border-border/40 rounded px-2 py-1 text-sm text-text-main",
                        value: "{permission}",
                        oninput: move |event| permission.set(event.value()),
                        option { value: "private", "Private" }
                        option { value: "bilateral", "Bilateral" }
                        option { value: "permissive", "Permissive commons" }
                    }
                }
                label {
                    class: "mt-3 block",
                    "Target fragment"
                    input {
                        class: "mt-1 w-full bg-black/30 border border-border/40 rounded px-2 py-1 text-sm text-text-main",
                        placeholder: "section-or-entity",
                        value: "{target_fragment}",
                        oninput: move |event| target_fragment.set(event.value()),
                    }
                }
                label {
                    class: "mt-3 block",
                    "Authority / ontology URI"
                    input {
                        class: "mt-1 w-full bg-black/30 border border-border/40 rounded px-2 py-1 text-sm text-text-main",
                        placeholder: "https://example.org/ontology",
                        value: "{auth_uri}",
                        oninput: move |event| auth_uri.set(event.value()),
                    }
                }
                label {
                    class: "mt-3 flex items-center gap-2",
                    input {
                        r#type: "checkbox",
                        checked: show_cml(),
                        onchange: move |event| show_cml.set(event.checked()),
                    }
                    "Include in the contextual markup layer"
                }
                if !status().is_empty() {
                    p { class: "mt-3 text-text-main/80", role: "status", "{status}" }
                }
                if !annotations().is_empty() {
                    div {
                        class: "mt-4",
                        h3 { class: "font-semibold text-text-main", "Thread annotations ({annotations().len()})" }
                        for (index, annotation) in annotations().iter().enumerate() {
                            article {
                                key: "{index}",
                                class: "mt-2 rounded border border-border/40 p-2 bg-black/20",
                                p { class: "text-text-main", "{annotation.get(\"body\").and_then(|value| value.as_str()).unwrap_or_default()}" }
                                p { class: "mt-1 font-mono break-all", "{annotation.get(\"target\").and_then(|value| value.as_str()).unwrap_or_default()}" }
                            }
                        }
                    }
                }
            }
            div { class: "p-2 border-t border-border/40 flex gap-1",
                input {
                    class: "flex-1 bg-black/30 border border-border/40 rounded px-2 py-1 text-sm text-text-main",
                    placeholder: "Note…",
                    value: "{message}",
                    oninput: move |e| message.set(e.value()),
                }
                button {
                    class: "border border-border/40 rounded px-2 py-1 text-sm text-text-main",
                    onclick: add_annotation,
                    "Add"
                }
            }
        }
    }
}
