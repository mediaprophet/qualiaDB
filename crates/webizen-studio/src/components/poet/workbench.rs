//! HyperCanvas shell — structure from `Canvas_Workbench/index.html` and `POET-SPEC-001..023`.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::chest::ToolChest;
use super::chrome::{ControlBar, Expose, StatusBar, TopMenubar};
#[cfg(target_arch = "wasm32")]
use super::host;
use super::instrument_bay::InstrumentBay;
use super::lexicon_bay::LexiconBay;
use super::radial_menu::{RadialActionRing, RadialState};
use super::stage::CanvasStage;
use super::store::Workbench;
use super::styles::HyperCanvasStyles;
use dioxus::prelude::*;

#[component]
fn CatalogStudioBay() -> Element {
    let mut tab = use_signal(|| "catalog");
    rsx! {
        div { class: "catalog-studio-bay-head", role: "tablist", "aria-label": "Catalog peers",
            button {
                r#type: "button",
                class: if tab() == "catalog" { "catalog-studio-bay-tab is-active" } else { "catalog-studio-bay-tab" },
                "aria-selected": "{tab() == \"catalog\"}",
                onclick: move |_| tab.set("catalog"),
                "Catalog · Lexicon"
            }
            button {
                r#type: "button",
                class: if tab() == "instruments" { "catalog-studio-bay-tab is-active" } else { "catalog-studio-bay-tab" },
                "aria-selected": "{tab() == \"instruments\"}",
                onclick: move |_| tab.set("instruments"),
                "Catalog · Instruments"
            }
        }
        if tab() == "catalog" {
            LexiconBay {}
        } else {
            InstrumentBay {}
        }
    }
}

/// Document-level key handling — Delete/Backspace removes the selected
/// wire or container, Escape unwinds menus/armed-wire/selection, and
/// Ctrl+S/O/D drive checkpoint save, .hcf clipboard-open, and duplicate.
/// The bubbling `onkeydown` on #app-root only fires while a focusable
/// child holds focus, so canvas selection needs a real document listener.
#[cfg(target_arch = "wasm32")]
fn install_global_keys(mut wb: Signal<Workbench>, mut radial: Signal<RadialState>) {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    let handler = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        let editing = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
            .map(|el| {
                matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                    || el.get_attribute("contenteditable").as_deref() == Some("true")
            })
            .unwrap_or(false);

        if event.ctrl_key() || event.meta_key() {
            match event.key().as_str() {
                "s" | "S" => {
                    event.prevent_default();
                    host::save_checkpoint(wb);
                }
                "o" | "O" => {
                    event.prevent_default();
                    host::open_hcf_from_clipboard(wb);
                }
                "d" | "D" if !editing => {
                    event.prevent_default();
                    let mut s = wb();
                    match s.duplicate_selected() {
                        Some(id) => s.note(format!("Duplicated → {id}")),
                        None => s.note("Duplicate — select a container first"),
                    }
                    wb.set(s);
                }
                _ => {}
            }
            return;
        }

        match event.key().as_str() {
            "Delete" | "Backspace" if !editing => {
                event.prevent_default();
                let mut s = wb();
                if s.delete_selected() {
                    s.note("Deleted selection");
                }
                wb.set(s);
            }
            "Escape" => {
                let mut s = wb();
                s.menu = None;
                s.wire_source = None;
                s.clear_selection();
                s.expose = false;
                wb.set(s);
                let mut rd = radial();
                rd.visible = false;
                radial.set(rd);
            }
            _ => {}
        }
    }) as Box<dyn FnMut(_)>);
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        let _ = document.add_event_listener_with_callback(
            "keydown",
            handler.as_ref().unchecked_ref(),
        );
        handler.forget();
    }
}

#[component]
pub fn PoetWorkbench() -> Element {
    let nav = use_navigator();
    let mut wb = use_signal(Workbench::new);
    let mut radial = use_signal(RadialState::default);
    #[allow(unused_mut)] // set() only happens under wasm32
    let mut key_listener_started = use_signal(|| false);
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            if !key_listener_started() {
                key_listener_started.set(true);
                install_global_keys(wb, radial);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key_listener_started;
        }
    });

    rsx! {
        HyperCanvasStyles {}
        div {
            id: "app-root",
            onclick: move |_| {
                let mut s = wb();
                if s.menu.is_some() {
                    s.menu = None;
                    wb.set(s);
                }
                let mut rd = radial();
                if rd.visible {
                    rd.visible = false;
                    radial.set(rd);
                }
            },
            oncontextmenu: move |e| {
                e.prevent_default();
                let coords = e.data().client_coordinates();
                radial.set(RadialState {
                    visible: true,
                    x: coords.x,
                    y: coords.y,
                });
            },
            onkeydown: move |e| {
                let key = e.data().key().to_string();
                let alt = e.data().modifiers().alt();
                if alt {
                    if key.eq_ignore_ascii_case("o") {
                        let mut s = wb();
                        s.expose = !s.expose;
                        wb.set(s);
                    } else if key.eq_ignore_ascii_case("a") {
                        let mut s = wb();
                        s.auto_arrange();
                        wb.set(s);
                    } else if key.eq_ignore_ascii_case("u") {
                        // Alt+U — pivot to Classic Settings (Tools→Settings / Ctrl+, target).
                        crate::components::shell_kind::persist_shell_kind(
                            crate::components::shell_kind::ShellKind::Classic,
                        );
                        let _ = nav.push(crate::Route::SettingsRoute {});
                    } else if key.eq_ignore_ascii_case("b") || key.eq_ignore_ascii_case("h") {
                        // Alt+B or Alt+H — return to Webizen Studio Talk/Home.
                        crate::components::shell_kind::persist_shell_kind(
                            crate::components::shell_kind::ShellKind::Classic,
                        );
                        let _ = nav.push(crate::Route::TalkRoute {});
                    } else if let Ok(digit) = key.parse::<usize>() {
                        let idx = if digit == 0 { 9 } else { digit - 1 };
                        if let Some(id) = super::kinds::ManifoldId::ALL.get(idx) {
                            let mut s = wb();
                            s.switch(*id);
                            wb.set(s);
                        }
                    }
                }
            },
            TopMenubar { wb }
            ControlBar { wb }
            div { class: "main-workspace",
                div { class: "workspace-row",
                    ToolChest { wb }
                    CanvasStage { wb }
                    aside {
                        class: if wb().sidebar { "tech-sidebar open" } else { "tech-sidebar" },
                        super::inspector::InspectorPanel { wb }
                    }
                }
                section {
                    class: "catalog-studio-bay",
                    "data-catalog-studio-bay": "1",
                    "aria-label": "Catalog · Lexicon and Instruments",
                    CatalogStudioBay {}
                }
            }
            StatusBar { wb }
            Expose { wb }
            RadialActionRing { wb, state: radial }
        }
    }
}
