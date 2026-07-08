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

    let mut show_sidebar = use_signal(|| false);

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

    let current_url = omnibox_input.read().clone();
    let (scheme_icon, scheme_color) = if current_url.starts_with("qualia://") {
        ("box", "text-purple-400")
    } else if current_url.starts_with("webizen://") {
        ("globe", "text-cyan-400")
    } else {
        ("globe-americas", "text-gray-400")
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
                    div { 
                        class: "mr-3 flex items-center justify-center {scheme_color}", 
                        title: "Protocol Indicator",
                        sl-icon { "name": "{scheme_icon}", style: "font-size: 1.1rem;" } 
                    }
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
                    button {
                        r#type: "button",
                        class: "bg-transparent border-none cursor-pointer hover:scale-110 hover:text-primary transition-all text-text-muted ml-2",
                        onclick: move |_| show_sidebar.set(!show_sidebar()),
                        title: "Semantic Conversations (Dialectical Sidebar)",
                        sl-icon { "name": "chat-right-text", style: "font-size:1.1rem;" }
                    }
                }
            }

            // Iframe viewport & Sidebar
            div { class: "flex-1 flex flex-row overflow-hidden",
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
                if show_sidebar() {
                    DialecticalSidebarPane { active_url: current_url }
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

    // Chat-Graph tracking
    let mut thread_target = use_signal(|| active_url.clone());
    let mut annotations = use_signal(|| Vec::<serde_json::Value>::new());

    // Sync if active_url changes from outside
    use_effect({
        let active_url = active_url.clone();
        move || {
            thread_target.set(active_url.clone());
        }
    });

    // Fetch annotations whenever the thread target changes
    use_effect(move || {
        let target = thread_target().clone();
        if target.is_empty() { return; }
        spawn(async move {
            if let Ok(res) = crate::components::wellfair::host_client::invoke_tauri("wellfair_search_library", serde_json::json!({ "facet": "target", "value": target })).await {
                if let Some(s) = res.as_str() {
                    if let Ok(data) = serde_json::from_str::<Vec<serde_json::Value>>(s) {
                        annotations.set(data);
                    }
                }
            }
        });
    });

    let submit = move |_| {
        let text = message().clone();
        let perm = permission().clone();
        let target_uri = thread_target().clone();
        let fragment = target_fragment().clone();
        let auth = auth_uri().clone();
        
        if text.trim().is_empty() && auth.trim().is_empty() { return; }

        spawn(async move {
            use crate::components::wellfair::host_client::IngestFacets;
            let manual = IngestFacets {
                occurred_at: Some(chrono::Utc::now().timestamp()),
                place_label: None, lat: None, lon: None,
                project: None,
                purpose: Some(format!("web-annotation ({})", perm)),
            };
            
            // Build the CML/Web Annotation payload
            let mut payload = serde_json::json!({
                "@context": "http://www.w3.org/ns/anno.jsonld",
                "type": "Annotation",
                "body": {
                    "type": "TextualBody",
                    "value": text
                },
                "target": target_uri
            });

            if !fragment.trim().is_empty() {
                payload["target"] = serde_json::json!({
                    "source": target_uri,
                    "selector": {
                        "type": "TextQuoteSelector",
                        "exact": fragment
                    }
                });
            }

            if !auth.trim().is_empty() {
                payload["cml:contextAssertions"] = serde_json::json!([{
                    "assertionType": "AuthoritativeClarification",
                    "authoritativeSource": auth
                }]);
            }

            let payload_str = serde_json::to_string_pretty(&payload).unwrap_or_default();
            
            let annotation_uri = format!("urn:q42:cml:{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));

            // Treat the message as a web-annotation JSON-LD document
            match crate::components::wellfair::host_client::ingest_document(
                &annotation_uri, "application/ld+json", &payload_str, None, &manual, &perm
            ).await {
                Ok(_) => {
                    message.set(String::new());
                    target_fragment.set(String::new());
                    auth_uri.set(String::new());
                    status.set("CML Annotation ingested into semantic manifold.".to_string());
                    // Force refresh by toggling thread_target
                    let t = thread_target().clone();
                    thread_target.set(String::new());
                    thread_target.set(t);
                }
                Err(e) => status.set(format!("Error: {}", e)),
            }
        });
    };

    rsx! {
        div {
            class: "w-80 h-full bg-surface border-l border-border/50 flex flex-col backdrop-blur-xl shadow-lg z-10",
            div { class: "p-4 border-b border-border/50 flex flex-col gap-2 relative", 
                h2 { class: "text-base font-bold text-primary m-0", "Semantic Conversations" }
                if *thread_target.read() != active_url {
                    div { class: "flex items-center gap-2",
                        button {
                            class: "px-2 py-1 bg-black/20 hover:bg-primary/20 text-text-muted hover:text-primary rounded text-xs cursor-pointer border border-border/50 transition-colors",
                            onclick: {
                                let root = active_url.clone();
                                move |_| thread_target.set(root.clone())
                            },
                            "← Back to Page Root"
                        }
                    }
                }
                span { class: "text-xs text-text-muted truncate", "{thread_target}" }
            }
            div { class: "flex-1 p-4 overflow-y-auto text-sm text-text-muted flex flex-col gap-3",
                if annotations.read().is_empty() {
                    p { class: "m-0 italic", "No annotations found for this URI. Be the first to build the chat-graph." }
                } else {
                    for ann in annotations.read().iter() {
                        div { class: "flex flex-col gap-1 p-3 bg-black/20 rounded border border-border/30",
                            if let Some(uri) = ann.get("uri").and_then(|u| u.as_str()) {
                                div { class: "flex justify-between items-center",
                                    span { class: "text-xs font-mono text-primary truncate w-4/5", "{uri}" }
                                    button {
                                        class: "text-xs bg-transparent border-none text-accent hover:text-primary cursor-pointer",
                                        onclick: {
                                            let u = uri.to_string();
                                            move |_| thread_target.set(u.clone())
                                        },
                                        "Reply"
                                    }
                                }
                            }
                            if let Some(text) = ann.get("text_excerpt").and_then(|t| t.as_str()) {
                                p { class: "text-text-main m-0 text-sm whitespace-pre-wrap", "{text}" }
                            }
                            // Show topics as tags
                            if let Some(topics) = ann.get("topics").and_then(|t| t.as_array()) {
                                div { class: "flex flex-wrap gap-1 mt-1",
                                    for t in topics.iter().filter_map(|t| t.as_str()) {
                                        span { class: "text-[10px] px-1.5 py-0.5 rounded bg-primary/20 text-primary", "#{t}" }
                                    }
                                }
                            }
                        }
                    }
                }
                if !status().is_empty() {
                    div { class: "p-2 bg-black/20 rounded text-accent text-xs", "{status()}" }
                }
            }
            div { class: "p-4 border-t border-border/50 flex flex-col gap-2",
                div { class: "flex justify-between items-center w-full",
                    select {
                        class: "p-2 bg-black/20 border border-border/50 rounded text-sm text-text-main flex-1",
                        value: "{permission}",
                        onchange: move |e| permission.set(e.value()),
                        option { value: "private", "Private (Only You)" }
                        option { value: "permissive", "Permissive (Trusted Fabric)" }
                        option { value: "public", "Public (Global Manifold)" }
                    }
                    button {
                        class: "ml-2 p-2 bg-transparent text-text-muted hover:text-primary border-none cursor-pointer flex items-center",
                        onclick: move |_| show_cml.set(!show_cml()),
                        title: "Toggle Advanced CML Assertions",
                        sl-icon { "name": "braces", style: "font-size: 1.2rem;" }
                    }
                }
                
                if show_cml() {
                    div { class: "flex flex-col gap-2 p-2 bg-black/10 rounded border border-border/30",
                        input {
                            class: "w-full p-2 bg-black/20 border border-border/50 rounded text-xs text-text-main outline-none focus:border-primary",
                            placeholder: "Target Fragment (highlighted claim)...",
                            value: "{target_fragment}",
                            oninput: move |e| target_fragment.set(e.value())
                        }
                        input {
                            class: "w-full p-2 bg-black/20 border border-border/50 rounded text-xs text-text-main outline-none focus:border-primary",
                            placeholder: "Authoritative Source URI...",
                            value: "{auth_uri}",
                            oninput: move |e| auth_uri.set(e.value())
                        }
                    }
                }

                textarea {
                    class: "w-full p-2 bg-black/20 border border-border/50 rounded text-sm text-text-main resize-none outline-none focus:border-primary",
                    rows: "3",
                    placeholder: "Add web-annotation...",
                    value: "{message}",
                    oninput: move |e| message.set(e.value())
                }
                button {
                    class: "w-full p-2 bg-primary text-white rounded font-bold cursor-pointer hover:opacity-90 transition-opacity",
                    onclick: submit,
                    "Share to Manifold"
                }
            }
        }
    }
}

// ── Cognitive Manifold Visualizer (Temporal Integrity Dashboard) ──
#[component]
pub fn CognitiveMonitorPane() -> Element {
    let mut topic_density = use_signal(|| std::collections::HashMap::<String, usize>::new());
    let mut total_annotations = use_signal(|| 0);
    let mut max_density = use_signal(|| 0);

    use_effect(move || {
        spawn(async move {
            let end = chrono::Utc::now().timestamp();
            let start = end - (30 * 24 * 60 * 60); // Last 30 days
            
            if let Ok(res) = crate::components::wellfair::host_client::invoke_tauri(
                "wellfair_search_library_time", 
                serde_json::json!({ "start": start, "end": end })
            ).await {
                if let Some(s) = res.as_str() {
                    if let Ok(data) = serde_json::from_str::<Vec<serde_json::Value>>(s) {
                        let mut counts = std::collections::HashMap::new();
                        let mut total = 0;
                        for entry in data {
                            if let Some(topics) = entry.get("topics").and_then(|t| t.as_array()) {
                                for topic in topics.iter().filter_map(|t| t.as_str()) {
                                    *counts.entry(topic.to_string()).or_insert(0) += 1;
                                    total += 1;
                                }
                            }
                        }
                        let max = counts.values().copied().max().unwrap_or(0);
                        topic_density.set(counts);
                        total_annotations.set(total);
                        max_density.set(max);
                    }
                }
            }
        });
    });

    rsx! {
        div {
            class: "w-full h-full bg-surface border-border/50 flex flex-col backdrop-blur-xl text-text-main",
            div { class: "p-4 border-b border-border/50", h2 { class: "text-lg font-bold text-primary m-0", "Cognitive Manifold Visualizer" } }
            div { class: "flex-1 p-4 overflow-y-auto flex flex-col gap-6",
                
                div { class: "flex justify-between items-center bg-black/20 p-4 rounded-lg border border-border/50 shadow-inner",
                    div { class: "flex flex-col gap-1",
                        span { class: "text-sm text-text-muted", "Temporal Window" }
                        span { class: "text-lg font-bold text-primary", "Last 30 Days" }
                    }
                    div { class: "flex flex-col gap-1 items-end",
                        span { class: "text-sm text-text-muted", "Total Assertions" }
                        span { class: "text-lg font-bold text-accent", "{total_annotations}" }
                    }
                }

                div { class: "flex flex-col gap-3",
                    h3 { class: "text-sm font-bold text-text-muted uppercase tracking-wider", "Topic Density Clusters" }
                    if topic_density.read().is_empty() {
                        p { class: "text-sm italic text-text-muted", "Insufficient data to render manifold. Begin annotating the web." }
                    } else {
                        // Sort topics by density descending
                        {
                            let mut sorted: Vec<_> = topic_density.read().iter().map(|(k, v)| (k.clone(), *v)).collect();
                            sorted.sort_by(|a, b| b.1.cmp(&a.1));
                            
                            rsx! {
                                div { class: "flex flex-col gap-2",
                                    for (topic, count) in sorted {
                                        div { class: "flex items-center gap-3 w-full",
                                            span { class: "text-xs w-32 truncate text-right", "{topic}" }
                                            div { class: "flex-1 bg-black/20 h-4 rounded overflow-hidden flex",
                                                div {
                                                    class: "h-full bg-primary transition-all duration-1000",
                                                    style: "width: {if *max_density.read() > 0 { (count as f32 / *max_density.read() as f32) * 100.0 } else { 0.0 }}%; opacity: {if count > 10 { 1.0 } else { 0.6 }};",
                                                }
                                            }
                                            span { class: "text-xs w-8 text-accent text-right font-mono", "{count}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                div { class: "mt-auto p-4 bg-black/20 rounded-lg border border-border/30 text-xs text-text-muted",
                    p { class: "m-0 mb-2 font-bold text-primary", "Integrity Incentive Mechanics" }
                    p { class: "m-0 opacity-80", "This manifold maps the structural density of authoritative CML annotations against transient noise. High-density topic clusters (opaque bars) represent mathematical consensus, directly rewarding rigorous provenance over engagement bait." }
                }
            }
        }
    }
}
