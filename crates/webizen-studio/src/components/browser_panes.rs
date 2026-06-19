use dioxus::prelude::*;
use serde_json::json;
use uuid::Uuid;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(
        cmd: &str,
        args: js_sys::Object,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

async fn invoke_tauri(cmd: &str, args: serde_json::Value) -> Result<String, String> {
    let js_args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    match tauri_invoke(cmd, js_args.into()).await {
        Ok(val) => {
            if val.is_string() {
                Ok(val.as_string().unwrap_or_default())
            } else {
                Ok(serde_wasm_bindgen::from_value::<String>(val).unwrap_or_default())
            }
        }
        Err(e) => Err(format!("{:?}", e)),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserTab {
    pub id: String,
    pub title: String,
    pub url: String,
}

#[component]
pub fn WebBrowserPane() -> Element {
    if !crate::endpoints::supports_browser_pane() {
        return rsx! { crate::components::browser_unavailable::BrowserUnavailable {} };
    }

    let mut tabs = use_signal(|| {
        vec![BrowserTab {
            id: Uuid::new_v4().to_string(),
            title: "New Tab".to_string(),
            url: "https://duckduckgo.com/".to_string(),
        }]
    });

    let mut active_tab_id = use_signal(|| tabs.read()[0].id.clone());
    let mut omnibox_input = use_signal(String::new);

    // Sync omnibox when active tab changes
    use_effect(move || {
        let current_id = active_tab_id.read().clone();
        if let Some(tab) = tabs.read().iter().find(|t| t.id == current_id) {
            omnibox_input.set(tab.url.clone());
        }
    });

    let submit_query = move |query: String| {
        spawn(async move {
            let res = invoke_tauri("submit_omnibox_query", json!({ "query": query })).await;
            if let Ok(new_url) = res {
                let current_id = active_tab_id.read().clone();
                let mut t = tabs.write();
                if let Some(tab) = t.iter_mut().find(|t| t.id == current_id) {
                    tab.url = new_url.clone();
                }
                omnibox_input.set(new_url);
            }
        });
    };

    let save_qlink = move || {
        let current_id = active_tab_id.read().clone();
        let active_url = tabs
            .read()
            .iter()
            .find(|t| t.id == current_id)
            .map(|t| t.url.clone())
            .unwrap_or_default();
        let title = tabs
            .read()
            .iter()
            .find(|t| t.id == current_id)
            .map(|t| t.title.clone())
            .unwrap_or_default();
        spawn(async move {
            let _ = invoke_tauri(
                "save_qlink",
                json!({ "url": active_url, "title": title, "context_assertions": null }),
            )
            .await;
        });
    };

    rsx! {
        div {
            class: "flex flex-col w-full h-full bg-surface text-text-main overflow-hidden",

            // Tab Strip
            div {
                class: "flex flex-row overflow-x-auto bg-black/50 p-1 gap-1 border-b border-border/50 min-h-[36px]",
                for tab in tabs.read().iter() {
                    div {
                        class: if *active_tab_id.read() == tab.id {
                            "flex items-center px-3 py-1.5 rounded-t-lg cursor-pointer min-w-[120px] max-w-[200px] text-sm transition-colors bg-surface"
                        } else {
                            "flex items-center px-3 py-1.5 rounded-t-lg cursor-pointer min-w-[120px] max-w-[200px] text-sm transition-colors bg-surface-hover hover:bg-surface-active"
                        },
                        onclick: {
                            let id = tab.id.clone();
                            move |_| active_tab_id.set(id.clone())
                        },
                        span { class: "flex-1 whitespace-nowrap overflow-hidden text-ellipsis", "{tab.title}" }
                        sl-icon { "name": "x", class: "ml-2 cursor-pointer text-text-muted hover:text-primary", onclick: move |e| { e.stop_propagation(); /* remove tab logic */ } }
                    }
                }
                button {
                    class: "px-3 cursor-pointer text-text-muted hover:text-primary bg-transparent border-none text-xl font-bold",
                    onclick: move |_| {
                        let new_id = Uuid::new_v4().to_string();
                        tabs.write().push(BrowserTab { id: new_id.clone(), title: "New Tab".into(), url: "https://duckduckgo.com/".into() });
                        active_tab_id.set(new_id);
                    },
                    "+"
                }
            }

            // Navigation & Omnibox
            div {
                class: "flex flex-row p-2 items-center gap-3 border-b border-border/50 bg-surface",
                form {
                    class: "flex-1 flex flex-row items-center px-4 py-1.5 bg-black/20 rounded-full border border-border/50 focus-within:border-primary focus-within:ring-1 focus-within:ring-primary/50 transition-all shadow-inner",
                    onsubmit: move |e| {
                        e.prevent_default();
                        submit_query(omnibox_input.read().clone());
                    },
                    div { class: "w-2 h-2 rounded-full bg-primary mr-3 shadow-[0_0_8px_var(--color-primary)] animate-pulse" }
                    input {
                        class: "flex-1 bg-transparent border-none outline-none text-text-main placeholder:text-text-muted/70",
                        value: "{omnibox_input}",
                        oninput: move |e| omnibox_input.set(e.value()),
                        placeholder: "Search the graph or type a URL...",
                    }
                    button {
                        r#type: "button",
                        class: "bg-transparent border-none cursor-pointer hover:scale-110 hover:text-primary transition-all text-text-muted ml-2",
                        onclick: move |_| save_qlink(),
                        title: "Save QLink (Semantic Bookmark)",
                        "🔖"
                    }
                }
            }

            // Iframe viewport
            div {
                class: "flex-1 relative bg-white overflow-hidden",
                for tab in tabs.read().iter() {
                    iframe {
                        src: "{tab.url}",
                        class: "w-full h-full border-none absolute top-0 left-0",
                        style: if *active_tab_id.read() == tab.id { "display: block;" } else { "display: none;" },
                        "sandbox": "allow-scripts allow-same-origin allow-forms allow-popups allow-downloads allow-popups-to-escape-sandbox",

                    }
                }
            }
        }
    }
}

// ── Dialectical Sidebar Pane ──────────────────────────────────────────────────
#[component]
pub fn DialecticalSidebarPane() -> Element {
    rsx! {
        div {
            class: "w-full h-full bg-surface border-border/50 flex flex-col backdrop-blur-xl",
            div { class: "p-4 border-b border-border/50", h2 { class: "text-lg font-bold text-primary", "Dialectical Synthesis" } }
            div { class: "flex-1 p-4 text-text-muted text-sm", "Chat & synthesis context goes here..." }
        }
    }
}

// ── Cognitive Monitor Pane ────────────────────────────────────────────────────
#[component]
pub fn CognitiveMonitorPane() -> Element {
    rsx! {
        div {
            class: "w-full h-full bg-surface border-border/50 flex flex-col backdrop-blur-xl",
            div { class: "p-4 border-b border-border/50", h2 { class: "text-lg font-bold text-primary", "Cognitive Monitor" } }
            div { class: "flex-1 p-4 text-text-muted text-sm", "System telemetries and thermal metrics..." }
        }
    }
}
