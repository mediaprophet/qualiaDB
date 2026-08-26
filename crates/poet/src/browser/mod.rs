//! Browser UI shell — renders the tool-chest in a browser for UX testing.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)
//!
//! This module renders the manifold pager, container layout, and
//! tool-chest sidebar using DOM manipulation via web-sys. It is
//! intended for UX testing prior to full QualiaDB integration.
//!
//! Design system adapted from Canvas_Workbench — cyber-semantic
//! glassmorphism with strata, social, health & epistemic modalities.

pub mod agreement_views;
pub mod app_launcher;
pub mod capabilities;
pub mod clipboard;
pub mod cml_document;
pub mod command_palette;
pub mod cooperative_economics;
pub mod chora_canvas;
pub mod container_inline_views;
pub mod container_views;
pub mod container_views_ext;
pub mod containers;
pub mod contextual_popover;
pub mod css;
pub mod dataset_views;
pub mod device_views;
pub mod diagnostics;
pub mod docks;
pub mod domain_presence;
pub mod git_forge;
pub mod governance_views;
pub mod health_views;
pub mod history;
pub mod hypermedia_bookmarks;
pub mod icon_graph;
pub mod icon_registry;
pub mod ide;
pub mod instrument_panel;
pub mod intent_bus;
pub mod interactions;
pub mod job_queue;
pub mod lived_memory_archive;
pub mod logic_workbench;
pub mod manifest;
pub mod mail_composer;
pub mod media_codecs;
pub mod native_daemon;
pub mod ontology_views;
pub mod project_views;
pub mod projections;
pub mod radial_menu;
pub mod registration;
pub mod rights_views;
pub mod search_workbench;
pub mod shader_pipelines;
pub mod solid_interop;
pub mod studio_views;
pub mod submanifold_nav;
pub mod theme;
pub mod tool_widgets;
pub mod topbar;
pub mod vibe_cell;
pub mod vibe_ui;
pub mod vision_10d_scrubber;
pub mod wire_inspector;
pub mod webrtc_sync;
pub mod workflow_panels;
pub mod workspace_pivot;

use std::sync::atomic::{AtomicBool, Ordering};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::tool_chest::core::registry::ManifoldSeed;

// ---------------------------------------------------------------------------
// Thread-local state
// ---------------------------------------------------------------------------

thread_local! {
    static CURRENT_SEEDS: std::cell::RefCell<Vec<ManifoldSeed>> = std::cell::RefCell::new(Vec::new());
}

/// Get a copy of the current manifold seeds (for persistence).
pub fn get_current_seeds() -> Vec<ManifoldSeed> {
    CURRENT_SEEDS.with(|s| s.borrow().clone())
}

/// Store the current manifold seeds in the thread-local.
fn store_current_seeds(seeds: &[ManifoldSeed]) {
    CURRENT_SEEDS.with(|s| {
        *s.borrow_mut() = seeds.to_vec();
    });
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Guard against double-initialisation. `main()` calls `start()` and the
/// HTML `TrunkApplicationStarted` listener also calls `start()` as a
/// fallback; the guard makes the second call a no-op.
static START_CALLED: AtomicBool = AtomicBool::new(false);

/// WASM entry point — called from main() in the binary target.
pub fn start() {
    if START_CALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    console_error_panic_hook::set_once();

    let window = web_sys::window().expect("no window");
    let document = window.document().expect("no document");

    if let Err(msg) = try_start(&document) {
        web_sys::console::error_1(&format!("[qualia-ui] FATAL: {}", msg).into());
        show_fatal_error(&document, &msg);
    }
}

/// Show a visible error message in the DOM (replaces loading indicator).
fn show_fatal_error(document: &Document, msg: &str) {
    if let Some(loading) = document.get_element_by_id("loading") {
        loading.set_inner_html(&format!(
            r#"<div style="color:#ff4444;font-family:monospace;font-size:12px;padding:20px;max-width:600px;">
            <h3 style="margin:0 0 10px;">Webizen Poet — Initialization Error</h3>
            <pre style="white-space:pre-wrap;word-break:break-all;">{}</pre>
            </div>"#,
            msg.replace('<', "&lt;").replace('>', "&gt;")
        ));
    }
}

fn try_start(document: &Document) -> Result<(), String> {
    // Inject CSS
    let style = document
        .create_element("style")
        .map_err(|e| format!("create_element(style): {:?}", e))?;
    style.set_text_content(Some(css::CSS));
    document
        .head()
        .ok_or("head() returned None")?
        .append_child(&style)
        .map_err(|e| format!("append_child(style): {:?}", e))?;

    // Apply default theme
    let theme_state = theme::ThemeState::default();
    theme_state.apply(document);

    // Build the app
    let app = build_app(document);

    // Append to body
    document
        .body()
        .ok_or("body() returned None")?
        .append_child(&app)
        .map_err(|e| format!("append_child(app): {:?}", e))?;

    // Remove loading indicator
    if let Some(loading) = document.get_element_by_id("loading") {
        loading.remove();
    }

    // Probe local Webizen daemon
    native_daemon::spawn_daemon_probe();

    Ok(())
}

// ---------------------------------------------------------------------------
// App builder
// ---------------------------------------------------------------------------

fn build_app(document: &Document) -> HtmlElement {
    let app = document.create_element("div").unwrap();
    app.set_class_name("app");

    // Build the populated registry — all 10 toolboxes + 10 manifold seeds.
    let registry = registration::build_registry();
    let seeds: Vec<ManifoldSeed> = registry.manifolds().to_vec();
    store_current_seeds(&seeds);

    // Extract cloneable toolbox views for the flyout panel and store
    // them in a thread-local accessible from click handlers.
    let toolbox_views = docks::extract_toolbox_views(registry.toolboxes());
    docks::store_toolbox_views(toolbox_views);

    // Initialise canvas undo/redo history with the first manifold seed.
    history::init_history(seeds[0].clone());

    // Top menubar
    let menubar = topbar::build_top_menubar(document);
    app.append_child(&menubar).unwrap();

    // Canvas control bar (manifold pager + strata + epistemic + dimension)
    let control_bar = topbar::build_canvas_control_bar(document, &seeds);
    app.append_child(&control_bar).unwrap();

    // Main workspace
    let workspace = document.create_element("div").unwrap();
    workspace.set_class_name("main-workspace");

    // Toolbox dock (left sidebar) — driven by the registry
    let toolbox = docks::build_toolbox_dock(document, registry.toolboxes());
    workspace.append_child(&toolbox).unwrap();

    // Canvas viewport
    let canvas = build_canvas(document, &seeds[0]);
    canvas.set_id("manifold-canvas");
    workspace.append_child(&canvas).unwrap();

    // Right dock (aura tray + pulse stream)
    let right_dock = docks::build_right_dock(document);
    workspace.append_child(&right_dock).unwrap();

    app.append_child(&workspace).unwrap();

    // Bottom status bar
    let statusbar = docks::build_bottom_statusbar(document);
    app.append_child(&statusbar).unwrap();

    // Command palette overlay
    let palette = command_palette::build_command_palette(document);
    app.append_child(&palette).unwrap();

    // Search workbench overlay
    let search_wb = search_workbench::build_search_workbench(document);
    app.append_child(&search_wb).unwrap();

    // Logic workbench overlay
    let logic_wb = logic_workbench::build_logic_workbench(document);
    app.append_child(&logic_wb).unwrap();

    // Wire up manifold switching
    wire_manifold_tabs(document, &seeds);

    // Wire up top control bar pods (Strata, Epistemic Lens, Dim/Time) & title rename
    topbar::wire_pods(document);
    topbar::wire_title_rename(document, &seeds);

    // Wire up canvas interactions
    interactions::wire_container_selection(document);
    interactions::wire_container_dragging(document);
    interactions::wire_container_resize(document);
    interactions::wire_container_deletion(document);
    interactions::wire_delete_key(document);
    interactions::wire_container_duplication(document);
    interactions::wire_port_dragging(document);
    interactions::wire_canvas_pan_zoom(document);
    interactions::wire_toolbox_dock(document);
    interactions::wire_selector_buttons(document);

    // Wire up command palette (Ctrl+K)
    command_palette::wire_command_palette(document);

    // Wire up undo/redo keyboard shortcuts (Ctrl+Z / Ctrl+Y)
    history::wire_undo_redo(document);

    // Wire up Alt+number manifold shortcuts + Alt+O Exposé
    wire_alt_shortcuts(document, &seeds);

    // Wire canvas click to hide instrument panel
    wire_canvas_click_hide_instrument_panel(document);

    // Wire menu dropdowns
    topbar::wire_menu_dropdowns(document);

    // Wire wire inspector (click wires to see details)
    wire_inspector::wire_wire_inspector(document);

    // Wire contextual RDF popover (select text in doc → annotate)
    contextual_popover::wire_contextual_popover(document);

    // Wire search workbench shortcut (Ctrl+Shift+F)
    search_workbench::wire_search_workbench_shortcut(document);

    // Wire logic workbench shortcut (Ctrl+Shift+L)
    logic_workbench::wire_logic_workbench_shortcut(document);

    // Wire 8-sector radial action ring (right-click / stylus context gesture)
    radial_menu::wire_radial_menu(document);

    // Restore saved dock position if present
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok()).flatten() {
        if let Ok(Some(saved_pos)) = storage.get_item("qualia_dock_pos") {
            if let Ok(Some(dock_el)) = document.query_selector(".toolbox-dock") {
                dock_el.set_class_name(&format!("toolbox-dock dock-pos-{}", saved_pos));
            }
        }
    }

    // Open default tool-chest drawer on startup (Word Processor & CML)
    docks::show_flyout(document, "office");
    interactions::wire_flyout_tools(document);

    app.dyn_into::<HtmlElement>().unwrap()
}

// ---------------------------------------------------------------------------
// Canvas
// ---------------------------------------------------------------------------

fn build_canvas(document: &Document, seed: &ManifoldSeed) -> Element {
    let canvas = document.create_element("div").unwrap();
    canvas.set_class_name("canvas-viewport-container");
    canvas.set_attribute("data-zoom", "1.0").unwrap();

    // Grid background
    let grid = document.create_element("div").unwrap();
    grid.set_class_name("canvas-grid-svg");
    canvas.append_child(&grid).unwrap();

    // Content layer — holds containers and wires, scaled by zoom
    let content_layer = document.create_element("div").unwrap();
    content_layer.set_class_name("canvas-content-layer");
    let content_el: web_sys::HtmlElement = content_layer.clone().dyn_into().unwrap();
    content_el.style().set_css_text(
        "position: absolute; top: 0; left: 0; width: 100%; height: 100%; \
         transform-origin: 0 0; transform: scale(1.0);",
    );

    // Containers
    for container in &seed.containers {
        let el = containers::build_container(document, container);
        content_layer.append_child(&el).unwrap();
    }

    canvas.append_child(&content_layer).unwrap();

    // Connection wires (rendered into the content layer so they scale too)
    interactions::render_wires(document, &content_layer, seed);

    // Zoom indicator (bottom-right corner)
    let zoom_ind = document.create_element("div").unwrap();
    zoom_ind.set_class_name("canvas-zoom-indicator");
    let zi_el: web_sys::HtmlElement = zoom_ind.clone().dyn_into().unwrap();
    zi_el.style().set_css_text(
        "position: absolute; bottom: 8px; right: 12px; \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-muted); \
         background: var(--surface-glass); padding: 2px 8px; border-radius: var(--radius-xs); \
         border: 1px solid var(--border-subtle); pointer-events: none; z-index: 50;",
    );
    zoom_ind.set_text_content(Some("100%"));
    canvas.append_child(&zoom_ind).unwrap();

    canvas
}

// ---------------------------------------------------------------------------
// Manifold switching
// ---------------------------------------------------------------------------

fn wire_manifold_tabs(document: &Document, seeds: &[ManifoldSeed]) {
    let tabs = document.query_selector_all(".desktop-tab-btn").unwrap();
    for i in 0..tabs.length() {
        let tab = tabs.get(i).unwrap();
        let tab_el: Element = tab.dyn_into().unwrap();
        let manifold_id = match tab_el.get_attribute("data-manifold") {
            Some(id) => id,
            None => continue,
        };

        let seeds_clone: Vec<ManifoldSeed> = seeds.to_vec();
        let closure = Closure::wrap(Box::new(move || {
            switch_manifold(&manifold_id, &seeds_clone);
        }) as Box<dyn FnMut()>);

        tab_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

fn switch_manifold(manifold_id: &str, seeds: &[ManifoldSeed]) {
    let document = web_sys::window().unwrap().document().unwrap();

    // Update tab active states
    let tabs = document.query_selector_all(".desktop-tab-btn").unwrap();
    for i in 0..tabs.length() {
        let tab = tabs.get(i).unwrap();
        let tab_el: Element = tab.dyn_into().unwrap();
        if tab_el.get_attribute("data-manifold").as_deref() == Some(manifold_id) {
            tab_el.class_list().add_1("active").unwrap();
        } else {
            tab_el.class_list().remove_1("active").unwrap();
        }
    }

    // Switch to the new manifold
    if let Some(seed) = seeds.iter().find(|s| s.id == manifold_id) {
        // Push the current state to history (synced from DOM) then set new seed.
        history::switch_to_manifold(seed.clone());
        rerender_canvas(seed);
    }
}

/// Re-render the canvas from a manifold seed.
/// Replaces canvas content, re-renders wires, re-wires interactions,
/// and updates the title + graph badge.
pub fn rerender_canvas(seed: &ManifoldSeed) {
    let document = web_sys::window().unwrap().document().unwrap();

    // Update title and graph badge
    if let Some(title) = document.query_selector(".canvas-title-input").unwrap() {
        if let Ok(input) = title.dyn_into::<web_sys::HtmlInputElement>() {
            input.set_value(&seed.label);
        }
    }
    if let Some(badge) = document.query_selector(".graph-address-badge").unwrap() {
        badge.set_text_content(Some(&format!("graph:manifold:{}", seed.id)));
    }

    // Replace canvas content
    if let Some(canvas) = document.get_element_by_id("manifold-canvas") {
        canvas.set_inner_html("");
        canvas.set_attribute("data-zoom", "1.0").unwrap();
        let grid = document.create_element("div").unwrap();
        grid.set_class_name("canvas-grid-svg");
        canvas.append_child(&grid).unwrap();

        // Content layer — holds containers and wires, scaled by zoom
        let content_layer = document.create_element("div").unwrap();
        content_layer.set_class_name("canvas-content-layer");
        let content_el: HtmlElement = content_layer.clone().dyn_into().unwrap();
        content_el.style().set_css_text(
            "position: absolute; top: 0; left: 0; width: 100%; height: 100%; \
             transform-origin: 0 0; transform: scale(1.0);",
        );

        for container in &seed.containers {
            let el = containers::build_container(&document, container);
            content_layer.append_child(&el).unwrap();
        }

        // Empty state if no containers
        if seed.containers.is_empty() {
            let empty = document.create_element("div").unwrap();
            empty.set_class_name("canvas-empty-state");
            let e_el: HtmlElement = empty.clone().dyn_into().unwrap();
            e_el.style().set_css_text(
                "position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); \
                 text-align: center; color: var(--text-muted); font-size: 14px; \
                 display: flex; flex-direction: column; gap: 12px; align-items: center;",
            );
            let icon = document.create_element("div").unwrap();
            icon.set_attribute("style", "font-size: 48px; opacity: 0.3;")
                .unwrap();
            icon.set_text_content(Some("\u{1F30C}"));
            empty.append_child(&icon).unwrap();

            let msg = document.create_element("div").unwrap();
            msg.set_text_content(Some(&format!("\"{}\" is empty", seed.label)));
            empty.append_child(&msg).unwrap();

            let hint = document.create_element("div").unwrap();
            hint.set_attribute("style", "font-size: 11px; color: var(--text-muted);")
                .unwrap();
            hint.set_text_content(Some(
                "Click a toolbox on the left and place a container to get started.",
            ));
            empty.append_child(&hint).unwrap();

            content_layer.append_child(&empty).unwrap();
        }

        canvas.append_child(&content_layer).unwrap();

        // Re-render wires into the content layer
        interactions::render_wires(&document, &content_layer, seed);

        // Zoom indicator
        let zoom_ind = document.create_element("div").unwrap();
        zoom_ind.set_class_name("canvas-zoom-indicator");
        let zi_el: HtmlElement = zoom_ind.clone().dyn_into().unwrap();
        zi_el.style().set_css_text(
            "position: absolute; bottom: 8px; right: 12px; \
             font-family: var(--font-mono); font-size: 10px; color: var(--text-muted); \
             background: var(--surface-glass); padding: 2px 8px; border-radius: var(--radius-xs); \
             border: 1px solid var(--border-subtle); pointer-events: none; z-index: 50;",
        );
        zoom_ind.set_text_content(Some("100%"));
        canvas.append_child(&zoom_ind).unwrap();

        // Re-wire interactions for new containers
        interactions::wire_container_selection(&document);
        interactions::wire_container_dragging(&document);
        interactions::wire_container_resize(&document);
        interactions::wire_container_deletion(&document);
        interactions::wire_port_dragging(&document);

        // Re-wire wire inspector for new wires
        wire_inspector::wire_wire_inspector(&document);

        // Re-wire contextual RDF popover for new doc editors
        contextual_popover::wire_contextual_popover(&document);
    }
}

// ---------------------------------------------------------------------------
// Alt+number shortcuts + Exposé overview
// ---------------------------------------------------------------------------

/// Wire Alt+1..Alt+0 for manifold switching and Alt+O for Exposé overview.
fn wire_alt_shortcuts(document: &Document, seeds: &[ManifoldSeed]) {
    let seeds_clone = seeds.to_vec();
    let doc_clone = document.clone();
    let closure = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
        if !e.alt_key() {
            return;
        }
        let key = e.key();
        e.prevent_default();

        // Alt+1 through Alt+9 → switch to manifold N
        if let Ok(n) = key.parse::<usize>() {
            if n >= 1 && n <= seeds_clone.len() {
                let id = &seeds_clone[n - 1].id;
                // Click the corresponding tab
                let doc = web_sys::window().unwrap().document().unwrap();
                let tabs = doc.query_selector_all(".desktop-tab-btn").unwrap();
                for i in 0..tabs.length() {
                    let tab = tabs.get(i).unwrap();
                    let tab_el: Element = tab.dyn_into().unwrap();
                    if tab_el.get_attribute("data-manifold").as_deref() == Some(id) {
                        let html_el: HtmlElement = tab_el.dyn_into().unwrap();
                        html_el.click();
                        break;
                    }
                }
            }
        }

        // Alt+O → toggle Exposé
        if key == "o" || key == "O" {
            toggle_expose(&doc_clone, &seeds_clone);
        }

        // Alt+A → auto-arrange manifold containers (Tidy)
        if key == "a" || key == "A" {
            interactions::auto_arrange_manifold(&doc_clone);
        }
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

    document
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

/// Toggle the Exposé 2x2 grid overview of all manifolds.
fn toggle_expose(document: &Document, seeds: &[ManifoldSeed]) {
    // If Exposé is already open, close it
    if let Some(existing) = document.get_element_by_id("expose-overlay") {
        existing.remove();
        return;
    }

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("expose-overlay");
    let o_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    o_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.85); z-index: 9000; display: flex; \
         align-items: center; justify-content: center; padding: 40px; \
         box-sizing: border-box;",
    );

    let grid = document.create_element("div").unwrap();
    let g_el: HtmlElement = grid.clone().dyn_into().unwrap();
    let cols = if seeds.len() <= 4 {
        2
    } else {
        if seeds.len() <= 6 {
            3
        } else {
            4
        }
    };
    g_el.style().set_css_text(&format!(
        "display: grid; grid-template-columns: repeat({}, 1fr); \
         gap: 16px; max-width: 1200px; width: 100%;",
        cols
    ));

    for (idx, seed) in seeds.iter().enumerate() {
        let card = document.create_element("div").unwrap();
        card.set_class_name("expose-card");
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "background: var(--surface-glass-heavy); border: 1px solid var(--border-medium); \
             border-radius: var(--radius-md); padding: 16px; cursor: pointer; \
             min-height: 180px; display: flex; flex-direction: column; gap: 8px; \
             transition: var(--trans-fast);",
        );
        card.set_attribute("data-manifold", &seed.id).unwrap();

        let header = document.create_element("div").unwrap();
        header
            .set_attribute("style", "display: flex; align-items: center; gap: 8px;")
            .unwrap();
        let num = document.create_element("span").unwrap();
        num.set_attribute(
            "style",
            "font-size: 24px; font-weight: 700; color: var(--accent-cyan);",
        )
        .unwrap();
        num.set_text_content(Some(&format!("{}", idx + 1)));
        header.append_child(&num).unwrap();

        let title = document.create_element("span").unwrap();
        title
            .set_attribute(
                "style",
                "font-size: 14px; font-weight: 600; color: var(--text-primary);",
            )
            .unwrap();
        title.set_text_content(Some(&seed.label));
        header.append_child(&title).unwrap();
        card.append_child(&header).unwrap();

        let desc = document.create_element("div").unwrap();
        desc.set_attribute("style", "font-size: 11px; color: var(--text-muted);")
            .unwrap();
        desc.set_text_content(Some(&seed.description));
        card.append_child(&desc).unwrap();

        let count = document.create_element("div").unwrap();
        count
            .set_attribute(
                "style",
                "font-size: 10px; color: var(--text-muted); margin-top: auto;",
            )
            .unwrap();
        count.set_text_content(Some(&format!("{} containers", seed.containers.len())));
        card.append_child(&count).unwrap();

        // Click to switch to this manifold
        let manifold_id = seed.id.clone();
        let seeds_for_click = seeds.to_vec();
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            // Close Exposé
            if let Some(ov) = doc.get_element_by_id("expose-overlay") {
                ov.remove();
            }
            // Switch to manifold
            switch_manifold(&manifold_id, &seeds_for_click);
        }) as Box<dyn FnMut(web_sys::Event)>);
        card.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();

        grid.append_child(&card).unwrap();
    }

    overlay.append_child(&grid).unwrap();

    // Click on overlay background closes Exposé
    let overlay_clone = overlay.clone();
    let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
        let target: Element = e.target().unwrap().dyn_into().unwrap();
        if target.id() == "expose-overlay" {
            overlay_clone.remove();
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    overlay
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    if let Some(body) = document.body() {
        body.append_child(&overlay).unwrap();
    }
}

/// Wire canvas click to hide the contextual instrument panel.
fn wire_canvas_click_hide_instrument_panel(document: &Document) {
    if let Some(canvas) = document.get_element_by_id("manifold-canvas") {
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            instrument_panel::hide(&doc);
        }) as Box<dyn FnMut(web_sys::Event)>);
        canvas
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
