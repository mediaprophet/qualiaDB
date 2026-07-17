//! Command palette (U6-A) — Ctrl/Cmd+K or Ctrl/Cmd+P to jump to product destinations.
//!
//! Mirrors the desktop shell palette (`shell_html.rs`). Studio path uses the Dioxus
//! router; shell path uses `navigate(qappId)` / `createTab`.

#![allow(non_snake_case)]
use dioxus::prelude::*;

use crate::Route;

/// One palette destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaletteDestination {
    pub id: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub keywords: &'static str,
}

/// Built-in destinations (≥5 required by U6-A).
pub const PALETTE_DESTINATIONS: &[PaletteDestination] = &[
    PaletteDestination {
        id: "talk",
        label: "Talk",
        hint: "Home · chat & people",
        keywords: "talk chat agent home",
    },
    PaletteDestination {
        id: "browser",
        label: "Browser (Reach)",
        hint: "Web browser",
        keywords: "browser reach web",
    },
    PaletteDestination {
        id: "10d-browser",
        label: "10D / Infosphere",
        hint: "Anatomy & vision .10d",
        keywords: "10d ten-d infosphere anatomy vision",
    },
    PaletteDestination {
        id: "settings",
        label: "Settings",
        hint: "Backend & preferences",
        keywords: "settings prefs config",
    },
    PaletteDestination {
        id: "library",
        label: "Library",
        hint: "Hypermedia shelf",
        keywords: "library hypermedia models",
    },
    PaletteDestination {
        id: "qapps",
        label: "QApps",
        hint: "QApp catalog",
        keywords: "qapps apps catalog",
    },
    PaletteDestination {
        id: "keep",
        label: "Keep",
        hint: "Vault & places hub",
        keywords: "keep vault",
    },
    PaletteDestination {
        id: "logs",
        label: "Desktop logs",
        hint: "Host log stream",
        keywords: "logs log",
    },
];

/// Map a palette id to a studio [`Route`].
pub fn route_for_palette_id(id: &str) -> Route {
    match id {
        "talk" | "chat" | "home" => Route::TalkRoute {},
        "browser" | "reach" | "web" => Route::BrowserRoute {},
        "10d-browser" | "10d" | "infosphere" => Route::TenDBrowserRoute {},
        "settings" | "prefs" => Route::SettingsRoute {},
        "library" => Route::LibraryRoute {},
        "qapps" | "apps" => Route::QAppsRoute {},
        "keep" | "vault" => Route::KeepRoute {},
        "logs" => Route::LogsRoute {},
        _ => Route::TalkRoute {},
    }
}

/// Filter destinations by free-text query (label, hint, keywords, id).
pub fn filter_destinations(query: &str) -> Vec<&'static PaletteDestination> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return PALETTE_DESTINATIONS.iter().collect();
    }
    PALETTE_DESTINATIONS
        .iter()
        .filter(|d| {
            let hay = format!("{} {} {} {}", d.id, d.label, d.hint, d.keywords).to_lowercase();
            hay.contains(&needle)
        })
        .collect()
}

/// Modal command palette overlay. Listens for Ctrl/Cmd+K and Ctrl/Cmd+P on wasm.
#[component]
pub fn CommandPalette() -> Element {
    let mut open = use_signal(|| false);
    let mut query = use_signal(String::new);
    let mut active = use_signal(|| 0usize);
    let navigator = use_navigator();
    let key_listener_started = use_signal(|| false);

    // Document-level hotkey (wasm only — studio product surface).
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            if key_listener_started() {
                return;
            }
            let mut key_listener_started = key_listener_started;
            key_listener_started.set(true);

            use wasm_bindgen::closure::Closure;
            use wasm_bindgen::JsCast;

            let mut open_sig = open;
            let mut query_sig = query;
            let mut active_sig = active;
            let handler = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
                move |event: web_sys::KeyboardEvent| {
                    let key = event.key().to_lowercase();
                    let mod_key = event.ctrl_key() || event.meta_key();
                    if mod_key && (key == "k" || key == "p") {
                        event.prevent_default();
                        let next = !open_sig();
                        open_sig.set(next);
                        if next {
                            query_sig.set(String::new());
                            active_sig.set(0);
                        }
                    }
                },
            );
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    let _ = document.add_event_listener_with_callback(
                        "keydown",
                        handler.as_ref().unchecked_ref(),
                    );
                    handler.forget();
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key_listener_started;
            let _ = open;
            let _ = query;
            let _ = active;
        }
    });

    let filtered = filter_destinations(&query());
    let n = filtered.len();
    let active_idx = if n == 0 {
        0
    } else {
        active().min(n - 1)
    };

    rsx! {
        // Toolbar affordance (always visible; keyboard is primary).
        button {
            r#type: "button",
            title: "Command palette (Ctrl+K)",
            style: "position: fixed; bottom: 1rem; right: 1rem; z-index: 900; \
                    border: 1px solid var(--qualia-border); background: rgba(10,15,30,0.85); \
                    color: var(--qualia-text-muted); border-radius: 999px; padding: 0.4rem 0.75rem; \
                    font-size: 0.72rem; font-weight: 600; cursor: pointer; backdrop-filter: blur(12px);",
            onclick: move |_| {
                open.set(true);
                query.set(String::new());
                active.set(0);
            },
            "Ctrl+K"
        }

        if open() {
            div {
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "Command palette",
                style: "position: fixed; inset: 0; z-index: 1000; background: rgba(0,0,0,0.55); \
                        display: flex; align-items: flex-start; justify-content: center; \
                        padding-top: 12vh; backdrop-filter: blur(4px);",
                onclick: move |_| {
                    open.set(false);
                    query.set(String::new());
                },
                div {
                    style: "width: min(560px, 92vw); background: var(--qualia-surface, #16213e); \
                            border: 1px solid var(--qualia-border); border-radius: 12px; \
                            box-shadow: 0 16px 48px rgba(0,0,0,0.45); overflow: hidden;",
                    // Stop backdrop close when clicking panel.
                    onclick: move |e| e.stop_propagation(),
                    input {
                        r#type: "text",
                        value: "{query}",
                        autofocus: true,
                        placeholder: "Go to Talk, Browser, 10D, Settings…",
                        style: "width: 100%; border: none; border-bottom: 1px solid var(--qualia-border); \
                                background: rgba(0,0,0,0.22); color: var(--qualia-text); \
                                padding: 14px 16px; font-size: 14px; outline: none; box-sizing: border-box;",
                        oninput: move |e| {
                            query.set(e.value());
                            active.set(0);
                        },
                        onkeydown: move |e| {
                            let items = filter_destinations(&query());
                            let count = items.len();
                            let idx = if count == 0 {
                                0
                            } else {
                                active().min(count - 1)
                            };
                            match e.key() {
                                Key::Escape => {
                                    open.set(false);
                                    query.set(String::new());
                                }
                                Key::ArrowDown => {
                                    e.prevent_default();
                                    if count > 0 {
                                        active.set((idx + 1) % count);
                                    }
                                }
                                Key::ArrowUp => {
                                    e.prevent_default();
                                    if count > 0 {
                                        active.set((idx + count - 1) % count);
                                    }
                                }
                                Key::Enter => {
                                    e.prevent_default();
                                    if let Some(dest) = items.get(idx) {
                                        let route = route_for_palette_id(dest.id);
                                        open.set(false);
                                        query.set(String::new());
                                        active.set(0);
                                        let _ = navigator.push(route);
                                    }
                                }
                                _ => {}
                            }
                        },
                    }
                    div {
                        role: "listbox",
                        style: "max-height: 320px; overflow-y: auto; padding: 6px;",
                        if filtered.is_empty() {
                            div {
                                style: "padding: 10px 12px; color: var(--qualia-text-muted); font-size: 13px;",
                                "No matching destination"
                            }
                        } else {
                            for (i, dest) in filtered.iter().enumerate() {
                                {
                                    let dest_id = dest.id;
                                    let dest_label = dest.label;
                                    let dest_hint = dest.hint;
                                    let is_active = i == active_idx;
                                    let bg = if is_active {
                                        "rgba(224,122,95,0.18)"
                                    } else {
                                        "transparent"
                                    };
                                    rsx! {
                                        div {
                                            key: "{dest_id}",
                                            role: "option",
                                            "aria-selected": "{is_active}",
                                            style: "display: flex; align-items: center; gap: 10px; \
                                                    padding: 10px 12px; border-radius: 8px; cursor: pointer; \
                                                    background: {bg}; color: var(--qualia-text);",
                                            onmouseenter: move |_| active.set(i),
                                            onclick: move |_| {
                                                let route = route_for_palette_id(dest_id);
                                                open.set(false);
                                                query.set(String::new());
                                                active.set(0);
                                                let _ = navigator.push(route);
                                            },
                                            span {
                                                style: "flex: 1; font-weight: 600; font-size: 13px;",
                                                "{dest_label}"
                                            }
                                            span {
                                                style: "font-size: 11px; color: var(--qualia-text-muted);",
                                                "{dest_hint}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div {
                        style: "padding: 8px 14px; border-top: 1px solid var(--qualia-border); \
                                font-size: 11px; color: var(--qualia-text-muted); display: flex; gap: 12px;",
                        span { "↑↓ navigate" }
                        span { "Enter open" }
                        span { "Esc close" }
                        span { style: "margin-left: auto;", "Ctrl+K · Ctrl+P" }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_least_five_destinations() {
        assert!(PALETTE_DESTINATIONS.len() >= 5);
    }

    #[test]
    fn filter_talk_and_browser() {
        let talk = filter_destinations("talk");
        assert!(talk.iter().any(|d| d.id == "talk"));
        let browser = filter_destinations("reach");
        assert!(browser.iter().any(|d| d.id == "browser"));
        let ten = filter_destinations("infosphere");
        assert!(ten.iter().any(|d| d.id == "10d-browser"));
    }

    #[test]
    fn empty_query_returns_all() {
        assert_eq!(filter_destinations("").len(), PALETTE_DESTINATIONS.len());
    }

    #[test]
    fn route_map_covers_core_five() {
        assert!(matches!(route_for_palette_id("talk"), Route::TalkRoute {}));
        assert!(matches!(
            route_for_palette_id("browser"),
            Route::BrowserRoute {}
        ));
        assert!(matches!(
            route_for_palette_id("10d-browser"),
            Route::TenDBrowserRoute {}
        ));
        assert!(matches!(
            route_for_palette_id("settings"),
            Route::SettingsRoute {}
        ));
        assert!(matches!(
            route_for_palette_id("library"),
            Route::LibraryRoute {}
        ));
    }
}
