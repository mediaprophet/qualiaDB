use dioxus::prelude::*;
use serde_json::json;
use uuid::Uuid;

use crate::components::qapp_engine::invoke_json;
async fn invoke_tauri(cmd: &str, args: serde_json::Value) -> Result<String, String> {
    let res = invoke_json(cmd, args).await?;
    if let Some(s) = res.as_str() {
        Ok(s.to_string())
    } else {
        serde_json::from_value::<String>(res).map_err(|e| e.to_string())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserTab {
    pub id: String,
    pub title: String,
    pub url: String,
    pub history: Vec<String>,
    pub history_index: usize,
}

impl BrowserTab {
    fn new(url: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: "New Tab".to_string(),
            url: url.clone(),
            history: vec![url],
            history_index: 0,
        }
    }

    fn can_go_back(&self) -> bool {
        self.history_index > 0
    }

    fn can_go_forward(&self) -> bool {
        self.history_index < self.history.len() - 1
    }

    fn go_back(&mut self) -> bool {
        if self.can_go_back() {
            self.history_index -= 1;
            self.url = self.history[self.history_index].clone();
            true
        } else {
            false
        }
    }

    fn go_forward(&mut self) -> bool {
        if self.can_go_forward() {
            self.history_index += 1;
            self.url = self.history[self.history_index].clone();
            true
        } else {
            false
        }
    }

    fn navigate(&mut self, url: String) {
        // Truncate forward history
        self.history.truncate(self.history_index + 1);
        self.history.push(url.clone());
        self.history_index = self.history.len() - 1;
        self.url = url;
    }
}

#[component]
pub fn WebBrowserPane() -> Element {
    if !crate::endpoints::supports_browser_pane() {
        return rsx! { crate::components::browser_unavailable::BrowserUnavailable {} };
    }

    let mut tabs = use_signal(|| {
        vec![BrowserTab::new("https://duckduckgo.com/".to_string())]
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
                    tab.navigate(new_url.clone());
                }
                omnibox_input.set(new_url);
            }
        });
    };

    let go_back = move |_| {
        let current_id = active_tab_id.read().clone();
        let mut t = tabs.write();
        if let Some(tab) = t.iter_mut().find(|t| t.id == current_id) {
            if tab.go_back() {
                let url = tab.url.clone();
                omnibox_input.set(url);
            }
        }
    };

    let go_forward = move |_| {
        let current_id = active_tab_id.read().clone();
        let mut t = tabs.write();
        if let Some(tab) = t.iter_mut().find(|t| t.id == current_id) {
            if tab.go_forward() {
                let url = tab.url.clone();
                omnibox_input.set(url);
            }
        }
    };

    let reload = move |_| {
        // Force iframe reload by toggling a key — Dioxus will re-render
        let current_id = active_tab_id.read().clone();
        let url = tabs.read().iter()
            .find(|t| t.id == current_id)
            .map(|t| t.url.clone())
            .unwrap_or_default();
        omnibox_input.set(url.clone());
        // Re-navigate to the same URL
        let mut t = tabs.write();
        if let Some(tab) = t.iter_mut().find(|t| t.id == current_id) {
            let current = tab.url.clone();
            tab.url = String::new();
            tab.url = current;
        }
    };

    // Get current tab's navigation state
    let (can_back, can_fwd) = {
        let current_id = active_tab_id.read().clone();
        let t = tabs.read();
        let tab = t.iter().find(|t| t.id == current_id);
        (tab.map(|t| t.can_go_back()).unwrap_or(false), tab.map(|t| t.can_go_forward()).unwrap_or(false))
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
                        tabs.write().push(BrowserTab::new("https://duckduckgo.com/".to_string()));
                        active_tab_id.set(new_id);
                    },
                    "+"
                }
            }

            // Navigation & Omnibox
            div {
                class: "flex flex-row p-2 items-center gap-2 border-b border-border/50 bg-surface",

                // Back button
                button {
                    r#type: "button",
                    class: "bg-transparent border-none cursor-pointer p-1.5 rounded text-text-muted hover:text-primary transition-all",
                    disabled: !can_back,
                    onclick: go_back,
                    title: "Back",
                    style: if !can_back { "opacity:0.4;cursor:default;" } else { "" },
                    sl-icon { "name": "arrow-left", style: "font-size:1rem;" }
                }

                // Forward button
                button {
                    r#type: "button",
                    class: "bg-transparent border-none cursor-pointer p-1.5 rounded text-text-muted hover:text-primary transition-all",
                    disabled: !can_fwd,
                    onclick: go_forward,
                    title: "Forward",
                    style: if !can_fwd { "opacity:0.4;cursor:default;" } else { "" },
                    sl-icon { "name": "arrow-right", style: "font-size:1rem;" }
                }

                // Reload button
                button {
                    r#type: "button",
                    class: "bg-transparent border-none cursor-pointer p-1.5 rounded text-text-muted hover:text-primary transition-all",
                    onclick: reload,
                    title: "Reload",
                    sl-icon { "name": "arrow-clockwise", style: "font-size:1rem;" }
                }

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
