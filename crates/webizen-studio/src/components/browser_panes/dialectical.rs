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
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (
        &mut message,
        &mut permission,
        &mut status,
        &mut target_fragment,
        &mut auth_uri,
        &mut show_cml,
        &mut thread_target,
        &mut annotations,
    );

    use_effect({
        let active_url = active_url.clone();
        move || {
            thread_target.set(active_url.clone());
        }
    });

    rsx! {
        div {
            class: "w-80 border-l border-border/50 bg-black/40 flex flex-col overflow-hidden",
            div {
                class: "p-3 border-b border-border/40 text-sm font-semibold text-text-main",
                "Dialectical sidebar"
            }
            div {
                class: "p-3 text-xs text-text-muted leading-relaxed flex-1 overflow-y-auto",
                p { "Semantic annotations for the active URL (later: full chat-graph + CML)." }
                p { class: "mt-2 font-mono break-all text-text-main/80", "{active_url}" }
                p { class: "mt-3", "Permission lane: {permission}" }
            }
            div { class: "p-2 border-t border-border/40 flex gap-1",
                input {
                    class: "flex-1 bg-black/30 border border-border/40 rounded px-2 py-1 text-sm text-text-main",
                    placeholder: "Note…",
                    value: "{message}",
                    oninput: move |e| message.set(e.value()),
                }
            }
        }
    }
}
