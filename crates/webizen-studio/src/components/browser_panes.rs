//! **Reach / Webizen Browser chrome** (studio pane).
//!
//! Pages load in a **native top-level WebView** window (`webizen-browser` via
//! `browser_navigate` / `open_web_url`) — not an iframe (X-Frame-Options) and not
//! the missing `spawn_native_webview` child-pane path.
//!
//! This pane owns: tabs, omnibox, back/forward/reload (app history + engine), focus,
//! and the dialectical sidebar. See `docs/plans/webizen-browser-and-trust.md` P0/P0.1.

use dioxus::prelude::*;
use serde_json::json;
use uuid::Uuid;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::components::qapp_engine::invoke_json;

async fn invoke_tauri(cmd: &str, args: serde_json::Value) -> Result<String, String> {
    let res = invoke_json(cmd, args).await?;
    if let Some(s) = res.as_str() {
        Ok(s.to_string())
    } else {
        // Commands may return JSON objects (browser_status) or bare strings.
        Ok(res.to_string())
    }
}

fn is_web_or_app_url(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    u.starts_with("http://")
        || u.starts_with("https://")
        || u.starts_with("qualia://")
        || u.starts_with("webizen://")
}

fn display_title_for(url: &str) -> String {
    let u = url.trim();
    if u.is_empty() {
        return "New Tab".into();
    }
    // Host-ish label without depending on the `url` crate.
    let rest = u
        .strip_prefix("https://")
        .or_else(|| u.strip_prefix("http://"))
        .or_else(|| u.strip_prefix("qualia://"))
        .or_else(|| u.strip_prefix("webizen://"))
        .unwrap_or(u);
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    if host.is_empty() {
        u.chars().take(40).collect()
    } else {
        host.chars().take(48).collect()
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
        let title = display_title_for(&url);
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            url: url.clone(),
            history: vec![url],
            history_index: 0,
        }
    }

    fn can_go_back(&self) -> bool {
        self.history_index > 0
    }

    fn can_go_forward(&self) -> bool {
        self.history_index + 1 < self.history.len()
    }

    fn go_back(&mut self) -> Option<String> {
        if self.can_go_back() {
            self.history_index -= 1;
            self.url = self.history[self.history_index].clone();
            self.title = display_title_for(&self.url);
            Some(self.url.clone())
        } else {
            None
        }
    }

    fn go_forward(&mut self) -> Option<String> {
        if self.can_go_forward() {
            self.history_index += 1;
            self.url = self.history[self.history_index].clone();
            self.title = display_title_for(&self.url);
            Some(self.url.clone())
        } else {
            None
        }
    }

    fn navigate(&mut self, url: String) {
        self.history.truncate(self.history_index + 1);
        self.history.push(url.clone());
        self.history_index = self.history.len() - 1;
        self.url = url;
        self.title = display_title_for(&self.url);
    }
}

/// Open URL in the native Webizen Browser window (top-level WebView).
async fn navigate_native(url: &str) -> Result<String, String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("empty URL".into());
    }
    // Prefer browser_navigate (returns URL string); fall back to open_web_url.
    match invoke_tauri("browser_navigate", json!({ "url": url })).await {
        Ok(s) => {
            // May be JSON-quoted string
            if let Ok(v) = serde_json::from_str::<String>(&s) {
                Ok(v)
            } else if s.starts_with('"') {
                serde_json::from_str(&s).map_err(|e| e.to_string())
            } else {
                Ok(if s.is_empty() { url.to_string() } else { s })
            }
        }
        Err(_) => {
            invoke_tauri("open_web_url", json!({ "url": url })).await?;
            Ok(url.to_string())
        }
    }
}

#[component]
pub fn WebBrowserPane() -> Element {
    let mut tabs = use_signal(|| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(win) = web_sys::window() {
                if let Ok(Some(storage)) = win.session_storage() {
                    if let Ok(Some(url)) = storage.get_item("webizen_browser_url") {
                        let _ = storage.remove_item("webizen_browser_url");
                        if !url.trim().is_empty() {
                            let tab = BrowserTab::new(url);
                            return vec![tab];
                        }
                    }
                }
            }
        }
        vec![BrowserTab::new("https://duckduckgo.com/".to_string())]
    });

    let mut active_tab_id = use_signal(|| tabs.read()[0].id.clone());
    let mut omnibox_input = use_signal(|| tabs.read()[0].url.clone());
    let mut status = use_signal(String::new);
    let mut status_err = use_signal(|| false);
    let mut show_sidebar = use_signal(|| false);
    let mut show_trust = use_signal(|| false);
    let mut show_bookmarks = use_signal(|| false);
    let mut trust_status = use_signal(String::new);
    let mut trust_list_text = use_signal(String::new);
    let mut bookmark_list = use_signal(Vec::<serde_json::Value>::new);
    let mut browser_open = use_signal(|| false);
    let mut bootstrapped = use_signal(|| false);

    // Sync omnibox when active tab changes.
    use_effect(move || {
        let current_id = active_tab_id();
        if let Some(tab) = tabs.read().iter().find(|t| t.id == current_id) {
            omnibox_input.set(tab.url.clone());
        }
    });

    // On first mount: open the start URL in the native window + poll status.
    use_effect(move || {
        if bootstrapped() {
            return;
        }
        bootstrapped.set(true);
        let start = tabs
            .read()
            .iter()
            .find(|t| t.id == active_tab_id())
            .map(|t| t.url.clone())
            .unwrap_or_else(|| "https://duckduckgo.com/".into());
        spawn(async move {
            match navigate_native(&start).await {
                Ok(_) => {
                    status_err.set(false);
                    status.set(format!("Opened in Webizen Browser: {start}"));
                    browser_open.set(true);
                }
                Err(e) => {
                    status_err.set(true);
                    status.set(format!(
                        "{e} — open the desktop app (Tauri host). Public wasm demo has no native browser."
                    ));
                }
            }
            if let Ok(raw) = invoke_tauri("browser_status", json!({})).await {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                    browser_open.set(v.get("open").and_then(|x| x.as_bool()).unwrap_or(false));
                }
            }
        });
    });

    let navigate_active = move |url: String| {
        spawn(async move {
            // Resolve search / shorthand through the host.
            let resolved = match invoke_tauri("submit_omnibox_query", json!({ "query": url.clone() }))
                .await
            {
                Ok(s) => {
                    // submit returns a bare string from Tauri JSON
                    let s = s.trim().trim_matches('"').to_string();
                    if s.is_empty() {
                        url
                    } else {
                        s
                    }
                }
                Err(_) => url,
            };

            if !is_web_or_app_url(&resolved) {
                status_err.set(true);
                status.set(format!("Not a navigable URL: {resolved}"));
                return;
            }

            let current_id = active_tab_id();
            {
                let mut t = tabs.write();
                if let Some(tab) = t.iter_mut().find(|t| t.id == current_id) {
                    tab.navigate(resolved.clone());
                }
            }
            omnibox_input.set(resolved.clone());

            match navigate_native(&resolved).await {
                Ok(_) => {
                    status_err.set(false);
                    status.set(format!("Navigated → {resolved}"));
                    browser_open.set(true);
                }
                Err(e) => {
                    status_err.set(true);
                    status.set(e);
                }
            }
        });
    };

    let go_back = move |_| {
        let current_id = active_tab_id();
        let next = {
            let mut t = tabs.write();
            t.iter_mut()
                .find(|t| t.id == current_id)
                .and_then(|tab| tab.go_back())
        };
        if let Some(url) = next {
            omnibox_input.set(url.clone());
            spawn(async move {
                // Drive both app history URL and engine history.
                let _ = invoke_tauri("browser_go_back", json!({})).await;
                let _ = navigate_native(&url).await;
                status.set(format!("Back → {url}"));
            });
        }
    };

    let go_forward = move |_| {
        let current_id = active_tab_id();
        let next = {
            let mut t = tabs.write();
            t.iter_mut()
                .find(|t| t.id == current_id)
                .and_then(|tab| tab.go_forward())
        };
        if let Some(url) = next {
            omnibox_input.set(url.clone());
            spawn(async move {
                let _ = invoke_tauri("browser_go_forward", json!({})).await;
                let _ = navigate_native(&url).await;
                status.set(format!("Forward → {url}"));
            });
        }
    };

    let reload = move |_| {
        let current_id = active_tab_id();
        let url = tabs
            .read()
            .iter()
            .find(|t| t.id == current_id)
            .map(|t| t.url.clone())
            .unwrap_or_default();
        omnibox_input.set(url.clone());
        spawn(async move {
            match invoke_tauri("browser_reload", json!({})).await {
                Ok(_) => {
                    status_err.set(false);
                    status.set("Reloaded".into());
                }
                Err(_) => {
                    // Window closed — re-open.
                    match navigate_native(&url).await {
                        Ok(_) => {
                            status_err.set(false);
                            status.set("Reopened browser window".into());
                            browser_open.set(true);
                        }
                        Err(e) => {
                            status_err.set(true);
                            status.set(e);
                        }
                    }
                }
            }
        });
    };

    let focus_browser = move |_| {
        spawn(async move {
            match invoke_tauri("browser_focus", json!({})).await {
                Ok(_) => {
                    status_err.set(false);
                    status.set("Focused Webizen Browser window".into());
                }
                Err(e) => {
                    status_err.set(true);
                    status.set(e);
                }
            }
        });
    };

    let mut select_tab = move |id: String| {
        let url = {
            active_tab_id.set(id.clone());
            tabs.read()
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.url.clone())
        };
        if let Some(url) = url {
            omnibox_input.set(url.clone());
            spawn(async move {
                let _ = navigate_native(&url).await;
            });
        }
    };

    let mut close_tab = move |id: String| {
        let mut t = tabs.write();
        if t.len() <= 1 {
            return;
        }
        t.retain(|tab| tab.id != id);
        if active_tab_id() == id {
            if let Some(first) = t.first() {
                let nid = first.id.clone();
                let url = first.url.clone();
                drop(t);
                active_tab_id.set(nid);
                omnibox_input.set(url.clone());
                spawn(async move {
                    let _ = navigate_native(&url).await;
                });
            }
        }
    };

    let save_qlink = move || {
        let current_id = active_tab_id();
        let (active_url, title) = {
            let t = tabs.read();
            let tab = t.iter().find(|t| t.id == current_id);
            (
                tab.map(|t| t.url.clone()).unwrap_or_default(),
                tab.map(|t| t.title.clone()).unwrap_or_default(),
            )
        };
        spawn(async move {
            match invoke_tauri(
                "save_qlink",
                json!({ "url": active_url, "title": title, "context_assertions": null }),
            )
            .await
            {
                Ok(msg) => {
                    status_err.set(false);
                    status.set(if msg.is_empty() {
                        "Bookmark saved (qlinks + Library when vault unlocked)".into()
                    } else {
                        msg
                    });
                }
                Err(e) => {
                    status_err.set(true);
                    status.set(format!("Bookmark failed: {e}"));
                }
            }
        });
    };

    let refresh_trust = move || {
        spawn(async move {
            match invoke_tauri("browser_trust_list", json!({})).await {
                Ok(raw) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                        let anchors = v
                            .get("anchors")
                            .and_then(|a| a.as_array())
                            .cloned()
                            .unwrap_or_default();
                        let lines: Vec<String> = anchors
                            .iter()
                            .map(|a| {
                                format!(
                                    "{} {} [{}]",
                                    if a.get("enabled").and_then(|x| x.as_bool()).unwrap_or(false) {
                                        "✓"
                                    } else {
                                        "·"
                                    },
                                    a.get("label").and_then(|x| x.as_str()).unwrap_or("?"),
                                    a.get("kind").and_then(|x| x.as_str()).unwrap_or("?")
                                )
                            })
                            .collect();
                        trust_list_text.set(if lines.is_empty() {
                            "(empty — add DID/PEM below)\nCustom PEM/DID govern agent fetch + policy badge; OS validates WebView TLS unless override is active.".into()
                        } else {
                            lines.join("\n")
                        });
                    } else {
                        trust_list_text.set(raw);
                    }
                    trust_status.set(String::new());
                }
                Err(e) => trust_status.set(e),
            }
        });
    };

    let refresh_bookmarks = move || {
        spawn(async move {
            match invoke_tauri("list_qlinks", json!({})).await {
                Ok(raw) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                        let list = v
                            .get("bookmarks")
                            .and_then(|a| a.as_array())
                            .cloned()
                            .unwrap_or_default();
                        bookmark_list.set(list);
                    }
                }
                Err(e) => {
                    status_err.set(true);
                    status.set(format!("Bookmarks: {e}"));
                }
            }
        });
    };

    let (can_back, can_fwd) = {
        let current_id = active_tab_id();
        let t = tabs.read();
        let tab = t.iter().find(|t| t.id == current_id);
        (
            tab.map(|t| t.can_go_back()).unwrap_or(false),
            tab.map(|t| t.can_go_forward()).unwrap_or(false),
        )
    };

    let current_url = omnibox_input();
    let (scheme_icon, scheme_color) = if current_url.starts_with("qualia://") {
        ("box", "text-purple-400")
    } else if current_url.starts_with("webizen://") {
        ("globe", "text-cyan-400")
    } else {
        ("globe-americas", "text-gray-400")
    };

    let active_url_for_sidebar = tabs
        .read()
        .iter()
        .find(|t| t.id == active_tab_id())
        .map(|t| t.url.clone())
        .unwrap_or_else(|| current_url.clone());

    rsx! {
        div {
            class: "flex flex-col w-full h-full bg-surface text-text-main overflow-hidden",

            // Tab strip
            div {
                class: "flex flex-row overflow-x-auto bg-black/50 p-1 gap-1 border-b border-border/50 min-h-[36px]",
                for tab in tabs.read().iter() {
                    {
                        let id = tab.id.clone();
                        let id_close = tab.id.clone();
                        let title = tab.title.clone();
                        let on = active_tab_id() == tab.id;
                        rsx! {
                            div {
                                class: if on {
                                    "flex items-center px-3 py-1.5 rounded-t-lg cursor-pointer min-w-[120px] max-w-[200px] text-sm transition-colors bg-surface"
                                } else {
                                    "flex items-center px-3 py-1.5 rounded-t-lg cursor-pointer min-w-[120px] max-w-[200px] text-sm transition-colors bg-surface-hover hover:bg-surface-active"
                                },
                                onclick: move |_| select_tab(id.clone()),
                                span { class: "flex-1 whitespace-nowrap overflow-hidden text-ellipsis", "{title}" }
                                button {
                                    r#type: "button",
                                    class: "ml-2 bg-transparent border-none cursor-pointer text-text-muted hover:text-primary p-0",
                                    onclick: move |e| {
                                        e.stop_propagation();
                                        close_tab(id_close.clone());
                                    },
                                    title: "Close tab",
                                    "×"
                                }
                            }
                        }
                    }
                }
                button {
                    r#type: "button",
                    class: "px-3 cursor-pointer text-text-muted hover:text-primary bg-transparent border-none text-xl font-bold",
                    title: "New tab",
                    onclick: move |_| {
                        let tab = BrowserTab::new("https://duckduckgo.com/".to_string());
                        let id = tab.id.clone();
                        let url = tab.url.clone();
                        tabs.write().push(tab);
                        active_tab_id.set(id);
                        omnibox_input.set(url.clone());
                        spawn(async move {
                            let _ = navigate_native(&url).await;
                        });
                    },
                    "+"
                }
            }

            // Navigation & omnibox
            div {
                class: "flex flex-row p-2 items-center gap-2 border-b border-border/50 bg-surface",
                button {
                    r#type: "button",
                    class: "bg-transparent border-none cursor-pointer p-1.5 rounded text-text-muted hover:text-primary transition-all",
                    disabled: !can_back,
                    onclick: go_back,
                    title: "Back",
                    style: if !can_back { "opacity:0.4;cursor:default;" } else { "" },
                    sl-icon { "name": "arrow-left", style: "font-size:1rem;" }
                }
                button {
                    r#type: "button",
                    class: "bg-transparent border-none cursor-pointer p-1.5 rounded text-text-muted hover:text-primary transition-all",
                    disabled: !can_fwd,
                    onclick: go_forward,
                    title: "Forward",
                    style: if !can_fwd { "opacity:0.4;cursor:default;" } else { "" },
                    sl-icon { "name": "arrow-right", style: "font-size:1rem;" }
                }
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
                        navigate_active(omnibox_input());
                    },
                    div {
                        class: "mr-3 flex items-center justify-center {scheme_color}",
                        title: "Protocol",
                        sl-icon { "name": "{scheme_icon}", style: "font-size: 1.1rem;" }
                    }
                    input {
                        class: "flex-1 bg-transparent border-none outline-none text-text-main placeholder:text-text-muted/70",
                        value: "{omnibox_input}",
                        oninput: move |e| omnibox_input.set(e.value()),
                        placeholder: "Search or type a URL (https://…, qualia://…, webizen://…)",
                    }
                    button {
                        r#type: "button",
                        class: "bg-transparent border-none cursor-pointer hover:scale-110 hover:text-primary transition-all text-text-muted ml-2",
                        onclick: move |_| save_qlink(),
                        title: "Save QLink",
                        "🔖"
                    }
                    button {
                        r#type: "button",
                        class: "bg-transparent border-none cursor-pointer hover:scale-110 hover:text-primary transition-all text-text-muted ml-2",
                        onclick: move |_| show_sidebar.set(!show_sidebar()),
                        title: "Dialectical sidebar",
                        sl-icon { "name": "chat-right-text", style: "font-size:1.1rem;" }
                    }
                }
                button {
                    r#type: "button",
                    class: "px-3 py-1.5 rounded-lg border border-border/50 bg-black/20 text-sm font-semibold cursor-pointer hover:border-primary text-text-main",
                    onclick: focus_browser,
                    title: "Focus the native browser window",
                    "Focus window"
                }
            }

            // Status
            if !status().is_empty() {
                div {
                    class: if status_err() {
                        "px-3 py-1.5 text-sm border-b border-red-500/40 bg-red-950/40 text-red-200"
                    } else {
                        "px-3 py-1.5 text-sm border-b border-border/40 bg-black/30 text-text-muted"
                    },
                    "{status}"
                }
            }

            // Main: honest host panel (pages open in native window) + optional sidebar
            div { class: "flex-1 flex flex-row overflow-hidden min-h-0",
                div {
                    class: "flex-1 relative overflow-auto p-6",
                    style: "background: linear-gradient(180deg, #0f172a 0%, #0b1220 100%); color: #e2e8f0;",
                    div {
                        style: "max-width: 36rem; margin: 0 auto;",
                        h2 {
                            style: "margin: 0 0 0.5rem; font-size: 1.25rem; font-weight: 700; color: #e9d5ff;",
                            "Webizen Browser"
                        }
                        p {
                            style: "margin: 0 0 1rem; font-size: 0.9rem; line-height: 1.5; color: #94a3b8;",
                            "Pages open in the native Webizen Browser window (in-window chrome + OS WebView content — not an iframe). This Reach pane mirrors tabs, omnibox, bookmarks, trust, and focus."
                        }
                        div {
                            style: "padding: 1rem 1.1rem; border-radius: 14px; border: 1px solid #334155; background: #111827; margin-bottom: 1rem;",
                            div { style: "font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.04em; color: #64748b; margin-bottom: 0.35rem;", "Active URL" }
                            div { style: "font-family: ui-monospace, monospace; font-size: 0.85rem; word-break: break-all; color: #c4b5fd;", "{active_url_for_sidebar}" }
                            div {
                                style: "margin-top: 0.65rem; font-size: 0.78rem; color: #94a3b8;",
                                if browser_open() {
                                    "Native window: open · substrate: OS WebView (WebView2 / WKWebView)"
                                } else {
                                    "Native window: not open yet — submit a URL or wait for startup navigation"
                                }
                            }
                        }
                        div { style: "display: flex; flex-wrap: wrap; gap: 0.5rem;",
                            button {
                                r#type: "button",
                                style: "padding: 0.5rem 0.9rem; border-radius: 9px; border: none; background: #8b5cf6; color: #fff; font-weight: 600; cursor: pointer; font-size: 0.85rem;",
                                onclick: move |_| navigate_active(omnibox_input()),
                                "Open / navigate"
                            }
                            button {
                                r#type: "button",
                                style: "padding: 0.5rem 0.9rem; border-radius: 9px; border: 1px solid #334155; background: #1e293b; color: #e2e8f0; font-weight: 600; cursor: pointer; font-size: 0.85rem;",
                                onclick: focus_browser,
                                "Focus browser window"
                            }
                            button {
                                r#type: "button",
                                style: "padding: 0.5rem 0.9rem; border-radius: 9px; border: 1px solid #334155; background: #1e293b; color: #e2e8f0; font-weight: 600; cursor: pointer; font-size: 0.85rem;",
                                onclick: move |_| navigate_active("https://duckduckgo.com/".into()),
                                "DuckDuckGo"
                            }
                            button {
                                r#type: "button",
                                style: "padding: 0.5rem 0.9rem; border-radius: 9px; border: 1px solid #334155; background: #1e293b; color: #e2e8f0; font-weight: 600; cursor: pointer; font-size: 0.85rem;",
                                onclick: move |_| {
                                    show_trust.set(!show_trust());
                                    if show_trust() {
                                        refresh_trust();
                                    }
                                },
                                "Trust store"
                            }
                            button {
                                r#type: "button",
                                style: "padding: 0.5rem 0.9rem; border-radius: 9px; border: 1px solid #334155; background: #1e293b; color: #e2e8f0; font-weight: 600; cursor: pointer; font-size: 0.85rem;",
                                onclick: move |_| {
                                    show_bookmarks.set(!show_bookmarks());
                                    if show_bookmarks() {
                                        refresh_bookmarks();
                                    }
                                },
                                "Bookmarks"
                            }
                        }
                        if show_trust() {
                            div {
                                style: "margin-top: 1rem; padding: 1rem; border-radius: 12px; border: 1px solid #334155; background: #0f172a;",
                                div { style: "font-size: 0.72rem; text-transform: uppercase; color: #c4b5fd; margin-bottom: 0.5rem;", "Your trust store" }
                                pre { style: "margin: 0 0 0.75rem; white-space: pre-wrap; font-size: 0.78rem; color: #94a3b8;", "{trust_list_text}" }
                                if !trust_status().is_empty() {
                                    div { style: "color: #fca5a5; font-size: 0.78rem; margin-bottom: 0.5rem;", "{trust_status}" }
                                }
                                form {
                                    style: "display: flex; flex-direction: column; gap: 0.4rem;",
                                    onsubmit: move |e| {
                                        e.prevent_default();
                                    },
                                    input {
                                        id: "reach-trust-did",
                                        style: "padding: 0.45rem 0.6rem; border-radius: 8px; border: 1px solid #334155; background: #0b1220; color: #e2e8f0; font-size: 0.8rem;",
                                        placeholder: "did:web:… to add",
                                    }
                                    button {
                                        r#type: "button",
                                        style: "padding: 0.45rem 0.75rem; border-radius: 8px; border: none; background: #8b5cf6; color: #fff; font-weight: 600; cursor: pointer; font-size: 0.8rem; align-self: flex-start;",
                                        onclick: move |_| {
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                if let Some(win) = web_sys::window() {
                                                    if let Some(doc) = win.document() {
                                                        if let Some(el) = doc.get_element_by_id("reach-trust-did") {
                                                            if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
                                                                let did = input.value();
                                                                if did.trim().is_empty() {
                                                                    trust_status.set("Enter a DID".into());
                                                                    return;
                                                                }
                                                                spawn(async move {
                                                                    match invoke_tauri(
                                                                        "browser_trust_add_did",
                                                                        json!({ "label": "", "did": did, "notes": null }),
                                                                    )
                                                                    .await
                                                                    {
                                                                        Ok(_) => {
                                                                            trust_status.set("DID added".into());
                                                                            refresh_trust();
                                                                        }
                                                                        Err(e) => trust_status.set(e),
                                                                    }
                                                                });
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            #[cfg(not(target_arch = "wasm32"))]
                                            {
                                                trust_status.set("Add DID from desktop Reach (wasm UI)".into());
                                            }
                                        },
                                        "Add DID"
                                    }
                                    p {
                                        style: "margin: 0.35rem 0 0; font-size: 0.72rem; color: #64748b; line-height: 1.4;",
                                        "Path: {{storage}}/webizen/trust_store.json · PEM roots apply to agent HTTPS; WebView still uses OS TLS."
                                    }
                                }
                            }
                        }
                        if show_bookmarks() {
                            div {
                                style: "margin-top: 1rem; padding: 1rem; border-radius: 12px; border: 1px solid #334155; background: #0f172a;",
                                div { style: "font-size: 0.72rem; text-transform: uppercase; color: #c4b5fd; margin-bottom: 0.5rem;", "Bookmarks" }
                                if bookmark_list().is_empty() {
                                    p { style: "margin: 0; font-size: 0.8rem; color: #64748b;", "No bookmarks yet — use 🔖 in the omnibox or browser chrome." }
                                } else {
                                    for b in bookmark_list().iter() {
                                        {
                                            let url = b.get("url").and_then(|x| x.as_str()).unwrap_or("").to_string();
                                            let name = b.get("name").and_then(|x| x.as_str()).unwrap_or(&url).to_string();
                                            let url_nav = url.clone();
                                            rsx! {
                                                button {
                                                    r#type: "button",
                                                    style: "display: block; width: 100%; text-align: left; margin-bottom: 0.35rem; padding: 0.45rem 0.55rem; border-radius: 8px; border: 1px solid #1e293b; background: #111827; color: #e2e8f0; cursor: pointer; font-size: 0.8rem;",
                                                    onclick: move |_| navigate_active(url_nav.clone()),
                                                    title: "{url}",
                                                    "{name}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        p {
                            style: "margin: 1.25rem 0 0; font-size: 0.78rem; line-height: 1.45; color: #64748b;",
                            "Shipped: in-window chrome, trust store CRUD, browser agent, bookmarks, cookie graph v0. Servo remains deferred."
                        }
                    }
                }
                if show_sidebar() {
                    DialecticalSidebarPane { active_url: active_url_for_sidebar.clone() }
                }
            }
        }
    }
}

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
