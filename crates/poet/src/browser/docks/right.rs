//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Right dock: Aura, Pulse, jobs, studio preview, and Vibe UI host.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, Event, HtmlElement};

/// Build the right dock (aura tray + pulse stream + job center).
pub fn build_right_dock(document: &Document) -> Element {
    let dock = document.create_element("div").unwrap();
    dock.set_class_name("right-dock");
    dock.set_id("right-dock");
    crate::browser::surface_aspects::mark(&dock, "entrance");

    // Collapse toggle button (shown when dock is collapsed)
    let expand_btn = document.create_element("button").unwrap();
    expand_btn.set_class_name("right-dock-expand-btn");
    expand_btn.set_id("right-dock-expand-btn");
    let eb_el: HtmlElement = expand_btn.clone().dyn_into().unwrap();
    eb_el.style().set_css_text(
        "display: none; position: absolute; right: 0; top: 50%; \
         transform: translateY(-50%); width: 20px; height: 60px; \
         background: var(--surface-panel); border: 1px solid var(--border-subtle); \
         border-right: none; border-radius: var(--radius-xs) 0 0 var(--radius-xs); \
         color: var(--text-muted); cursor: pointer; font-size: 14px; \
         z-index: 100; writing-mode: vertical-rl; padding: 4px;",
    );
    expand_btn.set_text_content(Some("\u{25C0} Dock"));
    dock.append_child(&expand_btn).unwrap();

    // Dock content wrapper (hidden when collapsed)
    let content = document.create_element("div").unwrap();
    content.set_class_name("right-dock-content");
    content.set_id("right-dock-content");

    // Collapse button (shown when dock is expanded)
    let collapse_btn = document.create_element("button").unwrap();
    collapse_btn.set_class_name("right-dock-collapse-btn");
    let cb_el: HtmlElement = collapse_btn.clone().dyn_into().unwrap();
    cb_el.style().set_css_text(
        "position: absolute; right: 4px; top: 4px; width: 18px; height: 18px; \
         background: transparent; border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); color: var(--text-muted); \
         cursor: pointer; font-size: 10px; z-index: 10; \
         display: flex; align-items: center; justify-content: center;",
    );
    collapse_btn.set_text_content(Some("\u{25B6}"));
    content.append_child(&collapse_btn).unwrap();

    // 1. Aura Tray — wired to diagnostics module with collapsible sub-trays
    let shacl_results = crate::browser::diagnostics::default_shacl_results();
    let passed = shacl_results.iter().filter(|r| r.conformant).count();
    let aura_badge = if shacl_results.is_empty() {
        "unavailable".to_string()
    } else {
        format!("{}/{} valid", passed, shacl_results.len())
    };
    let aura_body = crate::browser::diagnostics::render_aura_tray(document, &shacl_results);
    let aura_panel = super::panel::create_collapsible_dock_panel(
        document,
        "Aura Tray",
        Some(&aura_badge),
        aura_body,
        true,  // initially expanded
        false, // flex_grow
    );
    content.append_child(&aura_panel).unwrap();

    // 2. Pulse Stream — wired to diagnostics module
    let pulse_events = crate::browser::diagnostics::default_pulse_events();
    let pulse_badge = if pulse_events.is_empty() {
        "unavailable".to_string()
    } else {
        format!("{} events", pulse_events.len())
    };
    let pulse_body = crate::browser::diagnostics::render_pulse_stream(document, &pulse_events);
    let pulse_panel = super::panel::create_collapsible_dock_panel(
        document,
        "Pulse Stream",
        Some(&pulse_badge),
        pulse_body,
        true, // initially expanded
        true, // flex_grow (occupies remaining height)
    );
    content.append_child(&pulse_panel).unwrap();

    // 3. Job Center — background job queue
    let jobs = crate::browser::diagnostics::default_jobs();
    let active_jobs = jobs
        .iter()
        .filter(|j| j.status == crate::browser::diagnostics::JobStatus::Running)
        .count();
    let jobs_badge = if jobs.is_empty() {
        "unavailable".to_string()
    } else {
        format!("{} running", active_jobs)
    };
    let job_body = crate::browser::diagnostics::render_job_body(document, &jobs);
    let job_panel = super::panel::create_collapsible_dock_panel(
        document,
        "Job Center",
        Some(&jobs_badge),
        job_body,
        true,  // initially expanded
        false, // flex_grow
    );
    content.append_child(&job_panel).unwrap();

    // 4. Studio preview — still / clip / scene handle kinds on live Render preview.
    let preview_body = crate::browser::render_preview::build_studio_dock(document);
    let preview_panel = super::panel::create_collapsible_dock_panel(
        document,
        "Studio Preview",
        Some("still · clip · scene"),
        preview_body,
        true,
        false,
    );
    content.append_child(&preview_panel).unwrap();

    // 5. VibeScript UI Host: do not present synthetic runtime metrics as live.
    let vibe_ui_host = document.create_element("div").unwrap();
    vibe_ui_host.set_class_name("container-placeholder");
    vibe_ui_host
        .set_attribute("data-honesty", "unavailable")
        .ok();
    vibe_ui_host.set_text_content(Some(
        "Unavailable: the live VibeScript UI runtime is not connected.",
    ));
    vibe_ui_host.set_attribute("data-vibe-ui-host", "1").ok();
    let vibe_ui_panel = super::panel::create_collapsible_dock_panel(
        document,
        "Vibe UI Live Engine",
        Some("unavailable"),
        vibe_ui_host,
        false, // collapsed by default
        false, // flex_grow
    );
    vibe_ui_panel.set_attribute("data-vibe-ui-panel", "1").ok();
    content.append_child(&vibe_ui_panel).unwrap();

    dock.append_child(&content).unwrap();

    // Wire collapse/expand
    let content_clone = content.clone();
    let dock_clone = dock.clone();
    let expand_btn_clone1 = expand_btn.clone();
    let expand_btn_clone2 = expand_btn.clone();

    let collapse_closure = Closure::wrap(Box::new(move |_e: Event| {
        let content_el: HtmlElement = content_clone.clone().dyn_into().unwrap();
        content_el.style().set_property("display", "none").unwrap();
        let eb: HtmlElement = expand_btn_clone1.clone().dyn_into().unwrap();
        eb.style().set_property("display", "flex").unwrap();
        let d_el: HtmlElement = dock_clone.clone().dyn_into().unwrap();
        d_el.style().set_property("width", "20px").unwrap();
        d_el.style().set_property("min-width", "20px").unwrap();
    }) as Box<dyn FnMut(Event)>);
    collapse_btn
        .add_event_listener_with_callback("click", collapse_closure.as_ref().unchecked_ref())
        .unwrap();
    collapse_closure.forget();

    let content_clone2 = content.clone();
    let dock_clone2 = dock.clone();
    let expand_closure = Closure::wrap(Box::new(move |_e: Event| {
        let content_el: HtmlElement = content_clone2.clone().dyn_into().unwrap();
        content_el.style().set_property("display", "").unwrap();
        let eb: HtmlElement = expand_btn_clone2.clone().dyn_into().unwrap();
        eb.style().set_property("display", "none").unwrap();
        let d_el: HtmlElement = dock_clone2.clone().dyn_into().unwrap();
        d_el.style().set_property("width", "").unwrap();
        d_el.style().set_property("min-width", "").unwrap();
    }) as Box<dyn FnMut(Event)>);
    expand_btn
        .add_event_listener_with_callback("click", expand_closure.as_ref().unchecked_ref())
        .unwrap();
    expand_closure.forget();

    dock
}
