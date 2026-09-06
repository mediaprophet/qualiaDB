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

pub mod accessibility;
pub mod agreement_views;
pub mod app_launcher;
pub mod canvas_extent;
pub mod canvas_state;
pub mod capabilities;
mod chain_actions;
pub mod checkpoint_panel;
pub mod chora_canvas;
pub mod clipboard;
pub mod cml_document;
pub mod command_palette;
pub mod construct_shelf;
pub mod container_chrome;
pub mod container_inline_views;
pub mod container_transfer;
pub mod container_views;
pub mod container_views_ext;
pub mod containers;
pub mod contextual_popover;
pub mod cooperative_economics;
pub mod cop_records;
pub mod css;
pub mod dataset_views;
pub mod device_views;
mod diag_glow;
pub mod diagnostics;
pub mod docks;
pub mod dom_bindings;
pub mod domain_presence;
pub mod g_coord;
pub mod git_forge;
pub mod governance_views;
pub mod governance_workflow;
pub mod health_views;
pub mod history;
pub mod hypermedia_bookmarks;
pub mod icon_graph;
pub mod icon_registry;
pub mod icon_session;
pub mod ide;
pub mod instrument_panel;
pub mod intent_bus;
pub mod interactions;
pub mod job_queue;
pub mod lexicon_bay;
pub mod live_invoke;
pub mod lived_memory_archive;
mod local_container_views;
pub mod logic_workbench;
pub mod mail_composer;
pub mod manifest;
pub mod manifold_authoring;
pub mod manifold_social;
pub mod media_codecs;
pub mod native_daemon;
pub mod ontology_views;
pub mod project_views;
pub mod projections;
pub mod publication_panel;
pub mod pulse_stream;
pub mod radial_menu;
pub mod registration;
mod render_preview;
pub mod rights_views;
pub mod search_workbench;
mod semantic_library_render;
mod semantic_library_view;
pub mod shader_pipelines;
mod shapes_actions;
pub mod sheet;
pub mod social_inbox;
pub mod social_lifecycle;
pub mod social_moderation;
pub mod social_notifications;
pub mod social_presence;
pub mod social_workspace;
pub mod solid_interop;
mod spec_tools;
pub mod specialist_persist;
pub mod studio_views;
pub mod submanifold_nav;
pub mod surface_aspects;
mod surface_honesty;
pub(crate) mod surface_states;
pub mod theme;
pub mod tool_actions;
mod tool_copy;
mod tool_dual_path;
mod logic_chain_actions;
mod tool_proficiency;
pub mod tool_widgets;
pub mod topbar;
pub mod vibe_cell;
pub mod vibe_ui;
pub mod view_state;
pub mod vision_10d_scrubber;
pub mod webrtc_sync;
pub mod wire_inspector;
pub mod workflow_panels;
pub mod workspace_pivot;

use std::sync::atomic::{AtomicBool, Ordering};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::tool_chest::core::registry::ManifoldSeed;
use crate::tool_chest::core::{ManifoldParticipant, ManifoldSociality, SubjectSeed};

// ---------------------------------------------------------------------------
// Thread-local state
// ---------------------------------------------------------------------------

thread_local! {
    static CURRENT_SEEDS: std::cell::RefCell<Vec<ManifoldSeed>> = std::cell::RefCell::new(Vec::new());
    static CURRENT_CONSTRUCT: std::cell::RefCell<String> = std::cell::RefCell::new("poet".into());
    static CONSTRUCT_EXTRAS: std::cell::RefCell<std::collections::BTreeMap<String, Vec<String>>> =
        std::cell::RefCell::new(std::collections::BTreeMap::new());
    static CONSTRUCT_NAV: std::cell::RefCell<submanifold_nav::SubmanifoldNavigator> =
        std::cell::RefCell::new(submanifold_nav::SubmanifoldNavigator::new("poet", "POET"));
    static SUBJECTS: std::cell::RefCell<Vec<SubjectSeed>> = std::cell::RefCell::new(Vec::new());
    static OBSERVER_DID: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

const CONSTRUCT_STORAGE_KEY: &str = "qualia-ui:construct";
const CONSTRUCT_EXTRAS_KEY: &str = "qualia-ui:construct-extras";
const SUBJECTS_KEY: &str = "qualia-ui:subjects";

fn storage_get(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok())
        .flatten()
        .and_then(|storage| storage.get_item(key).ok())
        .flatten()
}

fn storage_set(key: &str, value: &str) {
    if let Some(storage) = web_sys::window()
        .and_then(|window| window.local_storage().ok())
        .flatten()
    {
        let _ = storage.set_item(key, value);
    }
}

fn load_stored_construct_id() -> String {
    web_sys::window()
        .and_then(|window| window.local_storage().ok())
        .flatten()
        .and_then(|storage| storage.get_item(CONSTRUCT_STORAGE_KEY).ok())
        .flatten()
        .filter(|id| crate::tool_chest::constructs::construct_by_id(id).is_some())
        .unwrap_or_else(|| "poet".into())
}

fn persist_construct_id(id: &str) {
    storage_set(CONSTRUCT_STORAGE_KEY, id);
}

pub(crate) fn persist_construct_extras() {
    CONSTRUCT_EXTRAS.with(|slot| {
        if let Ok(json) = serde_json::to_string(&*slot.borrow()) {
            storage_set(CONSTRUCT_EXTRAS_KEY, &json);
        }
    });
}

fn load_construct_extras() {
    if let Some(json) = storage_get(CONSTRUCT_EXTRAS_KEY) {
        if let Ok(map) = serde_json::from_str(&json) {
            CONSTRUCT_EXTRAS.with(|slot| *slot.borrow_mut() = map);
        }
    }
}

pub(crate) fn persist_subjects() {
    SUBJECTS.with(|slot| {
        if let Ok(json) = serde_json::to_string(&*slot.borrow()) {
            storage_set(SUBJECTS_KEY, &json);
        }
    });
}

fn load_subjects() {
    if let Some(json) = storage_get(SUBJECTS_KEY) {
        if let Ok(list) = serde_json::from_str(&json) {
            SUBJECTS.with(|slot| *slot.borrow_mut() = list);
        }
    }
}

pub fn declared_subjects() -> Vec<SubjectSeed> {
    SUBJECTS.with(|slot| slot.borrow().clone())
}

pub(crate) fn register_subject(seed: SubjectSeed) {
    SUBJECTS.with(|slot| {
        let mut list = slot.borrow_mut();
        if let Some(existing) = list.iter_mut().find(|candidate| candidate.id == seed.id) {
            *existing = seed;
        } else {
            list.push(seed);
        }
    });
    persist_subjects();
}

pub fn current_observer_did() -> String {
    OBSERVER_DID.with(|slot| slot.borrow().clone())
}

pub fn set_observer_did(did: String) {
    OBSERVER_DID.with(|slot| *slot.borrow_mut() = did);
}

pub fn current_manifold_is_social() -> bool {
    let id = current_manifold_id();
    get_current_seeds()
        .iter()
        .find(|seed| seed.id == id)
        .map(ManifoldSeed::is_social)
        .unwrap_or(false)
}

pub fn current_participants() -> Vec<ManifoldParticipant> {
    let id = current_manifold_id();
    get_current_seeds()
        .into_iter()
        .find(|seed| seed.id == id)
        .map(|seed| seed.participants)
        .unwrap_or_default()
}

pub(crate) fn add_participant_to_current(person: ManifoldParticipant) {
    let manifold_id = current_manifold_id();
    CURRENT_SEEDS.with(|slot| {
        let mut seeds = slot.borrow_mut();
        if let Some(seed) = seeds.iter_mut().find(|seed| seed.id == manifold_id) {
            seed.sociality = ManifoldSociality::Social;
            if !seed.participants.iter().any(|p| p.did == person.did) {
                seed.participants.push(person);
            }
        }
    });
}

pub fn bind_observer_from_daemon() {
    if !native_daemon::is_daemon_connected() {
        return;
    }
    wasm_bindgen_futures::spawn_local(async {
        match native_daemon::daemon_invoke("Identity.current_user", serde_json::json!({})).await {
            Ok(response) if response.ok => {
                if let Some(did) = parse_did_from_invoke(&response.value) {
                    set_observer_did(did);
                }
            }
            _ => {}
        }
    });
}

fn parse_did_from_invoke(value: &str) -> Option<String> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(value) {
        if let Some(did) = json.get("did").and_then(|v| v.as_str()) {
            if !did.is_empty() {
                return Some(did.to_string());
            }
        }
    }
    value
        .split('"')
        .find(|part| part.starts_with("did:"))
        .map(str::to_string)
}

fn upgrade_bundled_sociality(seeds: &mut [ManifoldSeed]) {
    for seed in seeds.iter_mut() {
        if crate::tool_chest::core::sociality::bundled_social_manifold(&seed.id) {
            seed.sociality = ManifoldSociality::Social;
        }
    }
}

pub fn current_manifold_id() -> String {
    CONSTRUCT_NAV.with(|slot| slot.borrow().current_manifold_id().to_string())
}

/// Manifolds visible in the open construct (poet shows every seeded lens).
pub fn visible_seeds() -> Vec<ManifoldSeed> {
    let all = get_current_seeds();
    let construct_id = current_construct_id();
    let Some(construct) = crate::tool_chest::constructs::construct_by_id(&construct_id) else {
        return all;
    };
    if construct.id == "poet" {
        return all;
    }
    let extras = CONSTRUCT_EXTRAS.with(|slot| {
        slot.borrow()
            .get(&construct.id)
            .cloned()
            .unwrap_or_default()
    });
    all.into_iter()
        .filter(|seed| {
            construct.contains_manifold(&seed.id) || extras.iter().any(|id| id == &seed.id)
        })
        .collect()
}

pub fn register_construct_manifold(manifold_id: &str) {
    let construct_id = current_construct_id();
    if construct_id == "poet" {
        return;
    }
    CONSTRUCT_EXTRAS.with(|slot| {
        let mut map = slot.borrow_mut();
        let extras = map.entry(construct_id).or_default();
        if !extras.iter().any(|id| id == manifold_id) {
            extras.push(manifold_id.to_string());
        }
    });
    persist_construct_extras();
}

/// Get a copy of the current manifold seeds (for persistence).
pub fn get_current_seeds() -> Vec<ManifoldSeed> {
    CURRENT_SEEDS.with(|s| s.borrow().clone())
}

/// Replace the live canvas state with a verified checkpoint snapshot.
pub fn restore_manifold_checkpoint(mut seeds: Vec<ManifoldSeed>) -> Result<(), String> {
    if seeds.is_empty() {
        return Err("checkpoint contains no manifolds".into());
    }
    canvas_state::normalise_seed_ids(&mut seeds);
    store_current_seeds(&seeds);
    let current = current_manifold_id();
    let visible = visible_seeds();
    let target = visible
        .iter()
        .find(|seed| seed.id == current)
        .or_else(|| visible.first())
        .or_else(|| seeds.first())
        .ok_or("checkpoint contains no visible manifold")?
        .id
        .clone();
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        topbar::rebuild_pager(&document, &visible, &target);
    }
    switch_manifold(&target, &seeds);
    manifest::save_all_manifolds()?;
    Ok(())
}

/// Store the current manifold seeds in the thread-local.
fn store_current_seeds(seeds: &[ManifoldSeed]) {
    CURRENT_SEEDS.with(|s| {
        *s.borrow_mut() = seeds.to_vec();
    });
}

/// Replace one manifold in the persistence store with its latest canvas state.
pub fn replace_current_seed(seed: &ManifoldSeed) {
    CURRENT_SEEDS.with(|slot| {
        let mut seeds = slot.borrow_mut();
        if let Some(existing) = seeds.iter_mut().find(|candidate| candidate.id == seed.id) {
            *existing = seed.clone();
        } else {
            seeds.push(seed.clone());
        }
    });
}

pub fn rename_current_seed(id: &str, label: &str) {
    CURRENT_SEEDS.with(|slot| {
        if let Some(seed) = slot.borrow_mut().iter_mut().find(|seed| seed.id == id) {
            seed.label = label.to_string();
        }
    });
}

/// Activate a manifold using the latest persisted in-memory seed. Dynamic
/// tabs call this instead of retaining the empty seed captured at creation.
fn manifold_title(manifold_id: &str) -> String {
    get_current_seeds()
        .into_iter()
        .find(|seed| seed.id == manifold_id)
        .map(|seed| seed.label)
        .unwrap_or_else(|| manifold_id.to_string())
}

/// Open a nested manifold lens inside the current construct and record breadcrumb.
pub fn dive_nested_manifold(manifold_id: &str) {
    let title = manifold_title(manifold_id);
    CONSTRUCT_NAV.with(|slot| {
        let _ = slot.borrow_mut().dive_into_subcanvas(manifold_id, &title);
    });
    activate_manifold(manifold_id);
}

/// Pager / sibling switch — replace the breadcrumb stack with this lens.
pub fn switch_to_sibling_manifold(manifold_id: &str) {
    let title = manifold_title(manifold_id);
    CONSTRUCT_NAV.with(|slot| {
        *slot.borrow_mut() = submanifold_nav::SubmanifoldNavigator::new(manifold_id, &title);
    });
    activate_manifold(manifold_id);
}

/// Pop one nested-manifold level. Root of the open construct is a no-op.
pub fn pop_nested_manifold() {
    let parent = CONSTRUCT_NAV.with(|slot| {
        let mut nav = slot.borrow_mut();
        if !nav.pop_one_level() {
            return None;
        }
        Some(nav.current_manifold_id().to_string())
    });
    match parent {
        Some(id) => activate_manifold(&id),
        None => {
            if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                interactions::show_tool_status(
                    &document,
                    "Construct",
                    "Already at the root lens of this construct.",
                    "info",
                );
            }
        }
    }
}

/// Jump to a specific breadcrumb depth (0 = root lens of this construct).
pub fn pop_nested_to_depth(depth: usize) {
    let id = CONSTRUCT_NAV.with(|slot| {
        let mut nav = slot.borrow_mut();
        if !nav.pop_to_depth(depth) {
            return None;
        }
        Some(nav.current_manifold_id().to_string())
    });
    if let Some(id) = id {
        activate_manifold(&id);
    }
}

pub fn construct_nav_crumbs() -> Vec<(String, String)> {
    CONSTRUCT_NAV.with(|slot| {
        slot.borrow()
            .breadcrumb_stack
            .iter()
            .map(|crumb| (crumb.id.clone(), crumb.title.clone()))
            .collect()
    })
}

pub fn activate_manifold(manifold_id: &str) {
    let seeds = get_current_seeds();
    switch_manifold(manifold_id, &seeds);
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        topbar::refresh_construct_chrome(&document, &current_construct_id(), manifold_id);
    }
}

pub fn current_construct_id() -> String {
    CURRENT_CONSTRUCT.with(|slot| slot.borrow().clone())
}

/// Open a construct: record it, then switch to its default (or requested) manifold.
pub fn open_construct(construct_id: &str, manifold_id: Option<&str>) {
    let Some(seed) = crate::tool_chest::constructs::construct_by_id(construct_id) else {
        let document = web_sys::window().and_then(|window| window.document());
        if let Some(document) = document {
            interactions::show_tool_status(
                &document,
                "Construct",
                &format!("Unknown construct `{construct_id}`."),
                "error",
            );
        }
        return;
    };
    if seed.default_manifold.is_empty() {
        let document = web_sys::window().and_then(|window| window.document());
        if let Some(document) = document {
            interactions::show_tool_status(
                &document,
                &seed.label,
                "Unavailable: Library Software stub with no manifold seed.",
                "unavailable",
            );
        }
        return;
    }
    CURRENT_CONSTRUCT.with(|slot| *slot.borrow_mut() = seed.id.clone());
    persist_construct_id(&seed.id);
    let extras =
        CONSTRUCT_EXTRAS.with(|slot| slot.borrow().get(&seed.id).cloned().unwrap_or_default());
    let target = manifold_id
        .filter(|id| {
            seed.contains_manifold(id)
                || seed.id == "poet"
                || extras.iter().any(|extra| extra == *id)
        })
        .unwrap_or(seed.default_manifold.as_str());
    let target_label = manifold_title(target);
    CONSTRUCT_NAV.with(|slot| {
        *slot.borrow_mut() = submanifold_nav::SubmanifoldNavigator::new(target, &target_label);
    });
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        let visible = visible_seeds();
        topbar::rebuild_pager(&document, &visible, target);
        topbar::refresh_construct_chrome(&document, &seed.id, target);
    }
    activate_manifold(target);
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        interactions::show_tool_status(
            &document,
            &seed.label,
            &format!(
                "Opened your construct `{id}` on manifold `{target}`.",
                id = seed.id
            ),
            "success",
        );
    }
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
    accessibility::restore(document);
    tool_proficiency::restore(document);

    // Build the app
    let app = build_app(document);

    // Append to body
    document
        .body()
        .ok_or("body() returned None")?
        .append_child(&app)
        .map_err(|e| format!("append_child(app): {:?}", e))?;
    tool_proficiency::restore(document);

    // DOM-query based wiring must happen after the detached app tree is mounted.
    // Wiring before this point silently finds zero controls on a cold start.
    let seeds = get_current_seeds();
    wire_app(document, &seeds);

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
    surface_aspects::mark(&app, "entrance");

    // Build the populated registry — all 15 Dioxus-parity toolboxes + manifold seeds.
    let registry = registration::build_registry();
    let defaults: Vec<ManifoldSeed> = registry.manifolds().to_vec();
    let mut seeds = manifest::load_saved_seeds()
        .ok()
        .flatten()
        .filter(|saved| !saved.is_empty())
        .unwrap_or_else(|| defaults.clone());
    for default_seed in defaults {
        if !seeds.iter().any(|seed| seed.id == default_seed.id) {
            seeds.push(default_seed);
        }
    }
    canvas_state::normalise_seed_ids(&mut seeds);
    upgrade_bundled_sociality(&mut seeds);
    store_current_seeds(&seeds);
    load_construct_extras();
    load_subjects();
    CURRENT_CONSTRUCT.with(|slot| *slot.borrow_mut() = load_stored_construct_id());
    let visible = visible_seeds();
    let opening = visible.first().cloned().unwrap_or_else(|| seeds[0].clone());
    CONSTRUCT_NAV.with(|slot| {
        *slot.borrow_mut() =
            submanifold_nav::SubmanifoldNavigator::new(&opening.id, &opening.label);
    });

    // Extract cloneable toolbox views for the flyout panel and store
    // them in a thread-local accessible from click handlers.
    let toolbox_views = docks::extract_toolbox_views(registry.toolboxes());
    docks::store_toolbox_views(toolbox_views);

    // Initialise canvas undo/redo history with the first visible manifold seed.
    history::init_history(opening.clone());

    // Top menubar
    let menubar = topbar::build_top_menubar(document);
    app.append_child(&menubar).unwrap();

    // Canvas control bar (manifold pager + strata + epistemic + dimension)
    let control_bar = topbar::build_canvas_control_bar(document, &visible);
    app.append_child(&control_bar).unwrap();
    topbar::refresh_construct_chrome(document, &current_construct_id(), &opening.id);

    // Main workspace
    let workspace = document.create_element("div").unwrap();
    workspace.set_class_name("main-workspace");
    surface_aspects::mark(&workspace, "entrance");

    // Toolbox dock (left sidebar) — driven by the registry
    let toolbox = docks::build_toolbox_dock(document, registry.toolboxes());
    workspace.append_child(&toolbox).unwrap();

    // Canvas viewport
    let canvas = build_canvas(document, &opening);
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

    app.dyn_into::<HtmlElement>().unwrap()
}

/// Attach behaviour to the mounted app tree.
fn wire_app(document: &Document, seeds: &[ManifoldSeed]) {
    wire_manifold_tabs(document, seeds);
    topbar::wire_pods(document);
    topbar::wire_title_rename(document, seeds);

    interactions::wire_container_selection(document);
    interactions::wire_container_dragging(document);
    interactions::wire_container_resize(document);
    interactions::wire_container_deletion(document);
    container_chrome::wire_container_chrome(document);
    interactions::wire_delete_key(document);
    interactions::wire_container_duplication(document);
    interactions::wire_port_dragging(document);
    interactions::wire_canvas_pan_zoom(document);
    interactions::wire_toolbox_dock(document);
    interactions::wire_selector_buttons(document);

    command_palette::wire_command_palette(document);
    history::wire_undo_redo(document);
    history::wire_editable_history(document);
    wire_alt_shortcuts(document, seeds);
    wire_canvas_click_hide_instrument_panel(document);
    topbar::wire_menu_dropdowns(document);
    wire_inspector::wire_wire_inspector(document);
    contextual_popover::wire_contextual_popover(document);
    search_workbench::wire_search_workbench_shortcut(document);
    logic_workbench::wire_logic_workbench_shortcut(document);
    radial_menu::wire_radial_menu(document);
    clipboard::wire_clipboard_shortcut(document);

    let saved_position = web_sys::window()
        .and_then(|window| window.local_storage().ok())
        .flatten()
        .and_then(|storage| storage.get_item("qualia_dock_pos").ok().flatten())
        .unwrap_or_else(|| "left".into());
    interactions::apply_toolbox_position(document, &saved_position);
    canvas_extent::ensure_manifold_extent(document);
}

// ---------------------------------------------------------------------------
// Canvas
// ---------------------------------------------------------------------------

fn build_canvas(document: &Document, seed: &ManifoldSeed) -> Element {
    let canvas = document.create_element("div").unwrap();
    canvas.set_class_name("canvas-viewport-container");
    surface_aspects::mark(&canvas, "entrance");
    canvas.set_attribute("data-zoom", "1.0").unwrap();
    canvas.set_attribute("data-pan-x", "0").unwrap();
    canvas.set_attribute("data-pan-y", "0").unwrap();

    // Content layer — world surface. Pan/zoom is a transform; the grid
    // lives here so it extends with the manifold rather than the viewport.
    let content_layer = document.create_element("div").unwrap();
    content_layer.set_class_name("canvas-content-layer");
    let content_el: web_sys::HtmlElement = content_layer.clone().dyn_into().unwrap();
    content_el.style().set_css_text(
        "position: absolute; top: 0; left: 0; overflow: visible; \
         transform-origin: 0 0; transform: translate(0px, 0px) scale(1.0);",
    );

    let grid = document.create_element("div").unwrap();
    grid.set_class_name("canvas-grid-svg");
    content_layer.append_child(&grid).unwrap();

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

fn wire_manifold_tabs(document: &Document, _seeds: &[ManifoldSeed]) {
    let tabs = document.query_selector_all(".desktop-tab-btn").unwrap();
    for i in 0..tabs.length() {
        let tab = tabs.get(i).unwrap();
        let tab_el: Element = tab.dyn_into().unwrap();
        let manifold_id = match tab_el.get_attribute("data-manifold") {
            Some(id) => id,
            None => continue,
        };

        let closure = Closure::wrap(Box::new(move || {
            switch_to_sibling_manifold(&manifold_id);
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
    let current_seeds = get_current_seeds();
    if let Some(seed) = current_seeds
        .iter()
        .find(|seed| seed.id == manifold_id)
        .or_else(|| seeds.iter().find(|seed| seed.id == manifold_id))
    {
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

    if let Ok(tabs) = document.query_selector_all(".desktop-tab-btn") {
        for index in 0..tabs.length() {
            let Some(node) = tabs.get(index) else {
                continue;
            };
            let Ok(tab) = node.dyn_into::<Element>() else {
                continue;
            };
            let active = tab.get_attribute("data-manifold").as_deref() == Some(&seed.id);
            let _ = tab.class_list().toggle_with_force("active", active);
        }
    }

    // Update title and graph badge
    if let Some(title) = document.query_selector(".canvas-title-input").unwrap() {
        if let Ok(input) = title.dyn_into::<web_sys::HtmlInputElement>() {
            input.set_value(&seed.label);
        }
    }
    if let Some(badge) = document.query_selector(".graph-address-badge").unwrap() {
        badge.set_text_content(Some(&format!("graph:manifold:{}", seed.id)));
    }
    if let Some(selector) = document.get_element_by_id("manifold-selector") {
        if let Ok(selector) = selector.dyn_into::<web_sys::HtmlSelectElement>() {
            selector.set_value(&seed.id);
        }
    }

    // Replace canvas content
    if let Some(canvas) = document.get_element_by_id("manifold-canvas") {
        canvas.set_inner_html("");
        canvas.set_attribute("data-zoom", "1.0").unwrap();
        canvas.set_attribute("data-pan-x", "0").unwrap();
        canvas.set_attribute("data-pan-y", "0").unwrap();

        let content_layer = document.create_element("div").unwrap();
        content_layer.set_class_name("canvas-content-layer");
        let content_el: HtmlElement = content_layer.clone().dyn_into().unwrap();
        content_el.style().set_css_text(
            "position: absolute; top: 0; left: 0; overflow: visible; \
             transform-origin: 0 0; transform: translate(0px, 0px) scale(1.0);",
        );
        let grid = document.create_element("div").unwrap();
        grid.set_class_name("canvas-grid-svg");
        content_layer.append_child(&grid).unwrap();

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
        history::wire_editable_history(&document);
        canvas_extent::ensure_manifold_extent(&document);
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
