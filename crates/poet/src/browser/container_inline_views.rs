//! Inline container view builders extracted from containers.rs.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement};

/// Social chat graph with messages and agent avatars.
pub fn build_social_chat_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();

    let messages: &[(&str, &str, &str, &str, &str, bool, &str)] = &[
        (
            "TH",
            "Timothy Charles Holborn",
            "14:38",
            "North Spring looks clearer after last night's rain; the field notes should capture that.",
            "subjective",
            false,
            "\u{1F9E0}",
        ),
        (
            "SP",
            "Sentinel Planner AI",
            "14:39",
            "Telemetry sensor confirms flow at 142.5 L/m with 221.5 Hz acoustic resonance.",
            "objective",
            true,
            "\u{1F50C}",
        ),
        (
            "FR",
            "Fiduciary Rights AI",
            "14:40",
            "Asserted legal quad: <<[ site:NorthSpring hydro:status \"Monitored\" ]>>.",
            "normative",
            true,
            "\u{2696}",
        ),
    ];

    for (avatar, sender, time, text, epistemic, is_ai, avatar_icon) in messages {
        let msg = document.create_element("div").unwrap();
        msg.set_class_name(&format!(
            "chat-message {}",
            if *is_ai { "ai-msg" } else { "human-msg" }
        ));

        let av = document.create_element("div").unwrap();
        av.set_class_name(&format!(
            "chat-avatar {}",
            if *is_ai { "ai-avatar" } else { "human-avatar" }
        ));
        av.set_text_content(Some(avatar_icon));
        msg.append_child(&av).unwrap();

        let content = document.create_element("div").unwrap();
        content.set_class_name("chat-content");

        let sender_row = document.create_element("div").unwrap();
        sender_row.set_class_name("chat-sender");

        let name = document.create_element("span").unwrap();
        name.set_text_content(Some(sender));
        sender_row.append_child(&name).unwrap();

        let modality = document.create_element("span").unwrap();
        modality.set_class_name(&format!("modality-badge modality-{}", epistemic));
        modality.set_text_content(Some(avatar));
        sender_row.append_child(&modality).unwrap();

        let time_el = document.create_element("span").unwrap();
        time_el.set_class_name("chat-time");
        time_el.set_text_content(Some(time));
        sender_row.append_child(&time_el).unwrap();

        content.append_child(&sender_row).unwrap();

        let text_el = document.create_element("div").unwrap();
        text_el.set_class_name("chat-text");
        text_el.set_text_content(Some(text));
        content.append_child(&text_el).unwrap();

        msg.append_child(&content).unwrap();
        wrapper.append_child(&msg).unwrap();
    }

    wrapper
}

/// Connection requests view with risk indicators.
pub fn build_connection_requests_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();

    let card = document.create_element("div").unwrap();
    card.set_class_name("cr-card");

    let header = document.create_element("div").unwrap();
    header.set_class_name("cr-header");

    let status = document.create_element("span").unwrap();
    status.set_class_name("cr-status cr-status-verifying");
    status.set_text_content(Some("verifying"));
    header.append_child(&status).unwrap();

    let sender = document.create_element("span").unwrap();
    sender.set_class_name("cr-sender");
    sender.set_text_content(Some(" Alice"));
    header.append_child(&sender).unwrap();

    let did = document.create_element("span").unwrap();
    did.set_class_name("cr-did");
    did.set_text_content(Some("did:qualia:alice"));
    header.append_child(&did).unwrap();

    card.append_child(&header).unwrap();

    let meta = document.create_element("div").unwrap();
    meta.set_class_name("cr-meta");
    meta.set_text_content(Some(
        "Requested: soc:friendship \u{00B7} ZKP: age proof, identity uniqueness",
    ));
    card.append_child(&meta).unwrap();

    let risk = document.create_element("div").unwrap();
    risk.set_class_name("cr-meta");
    let ri1 = document.create_element("span").unwrap();
    ri1.set_class_name("risk-indicator risk-low");
    ri1.set_text_content(Some("new-account"));
    let ri2 = document.create_element("span").unwrap();
    ri2.set_class_name("risk-indicator risk-moderate");
    ri2.set_text_content(Some("no-shared-contacts"));
    risk.append_child(&ri1).unwrap();
    risk.append_child(&ri2).unwrap();
    risk.append_child(&document.create_text_node(" Risk: low"))
        .unwrap();
    card.append_child(&risk).unwrap();

    let actions = document.create_element("div").unwrap();
    actions.set_class_name("cr-actions");
    let accept = document.create_element("button").unwrap();
    accept.set_class_name("cr-btn");
    accept.set_text_content(Some("Accept"));
    let decline = document.create_element("button").unwrap();
    decline.set_class_name("cr-btn danger");
    decline.set_text_content(Some("Decline"));
    actions.append_child(&accept).unwrap();
    actions.append_child(&decline).unwrap();
    card.append_child(&actions).unwrap();

    wrapper.append_child(&card).unwrap();

    let card2 = document.create_element("div").unwrap();
    card2.set_class_name("cr-card");
    let h2 = document.create_element("div").unwrap();
    h2.set_class_name("cr-header");
    let s2 = document.create_element("span").unwrap();
    s2.set_class_name("cr-status cr-status-blocked");
    s2.set_text_content(Some("blocked"));
    h2.append_child(&s2).unwrap();
    let n2 = document.create_element("span").unwrap();
    n2.set_class_name("cr-sender");
    n2.set_text_content(Some(" Unknown"));
    h2.append_child(&n2).unwrap();
    card2.append_child(&h2).unwrap();

    let m2 = document.create_element("div").unwrap();
    m2.set_class_name("cr-meta");
    m2.set_text_content(Some(
        "Requested: soc:friendship \u{00B7} Auto-blocked: critical risk",
    ));
    card2.append_child(&m2).unwrap();

    let r2 = document.create_element("div").unwrap();
    r2.set_class_name("cr-meta");
    let ri3 = document.create_element("span").unwrap();
    ri3.set_class_name("risk-indicator risk-critical");
    ri3.set_text_content(Some("grooming-pattern"));
    let ri4 = document.create_element("span").unwrap();
    ri4.set_class_name("risk-indicator risk-high");
    ri4.set_text_content(Some("identity-mismatch"));
    r2.append_child(&ri3).unwrap();
    r2.append_child(&ri4).unwrap();
    r2.append_child(&document.create_text_node(" Risk: critical"))
        .unwrap();
    card2.append_child(&r2).unwrap();

    wrapper.append_child(&card2).unwrap();
    wrapper
}

/// Protection policies view.
pub fn build_protection_policies_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();

    let card = document.create_element("div").unwrap();
    card.set_class_name("pp-card");
    let cat = document.create_element("div").unwrap();
    cat.set_class_name("pp-category");
    cat.set_text_content(Some("Minor Protection"));
    let mand = document.create_element("span").unwrap();
    mand.set_class_name("pp-mandatory");
    mand.set_text_content(Some("mandatory"));
    cat.append_child(&mand).unwrap();
    card.append_child(&cat).unwrap();

    for (k, v) in &[
        ("Approval", "always-required"),
        ("Max disclosure", "acquaintance"),
        ("Monitoring", "active"),
        ("Alerts", "grooming, bullying, adult-contact, isolation"),
    ] {
        let row = document.create_element("div").unwrap();
        row.set_class_name("pp-row");
        let key = document.create_element("span").unwrap();
        key.set_class_name("pp-key");
        key.set_text_content(Some(k));
        let val = document.create_element("span").unwrap();
        val.set_class_name("pp-val");
        val.set_text_content(Some(v));
        row.append_child(&key).unwrap();
        row.append_child(&val).unwrap();
        card.append_child(&row).unwrap();
    }
    wrapper.append_child(&card).unwrap();

    let card2 = document.create_element("div").unwrap();
    card2.set_class_name("pp-card");
    let cat2 = document.create_element("div").unwrap();
    cat2.set_class_name("pp-category");
    cat2.set_text_content(Some("DV Survivor Protection"));
    let opt = document.create_element("span").unwrap();
    opt.set_class_name("pp-optin");
    opt.set_text_content(Some("opt-in"));
    cat2.append_child(&opt).unwrap();
    card2.append_child(&cat2).unwrap();

    for (k, v) in &[
        ("Approval", "network-based"),
        ("Max disclosure", "personal"),
        ("Monitoring", "passive"),
        ("Alerts", "coercive-control, location-sharing, isolation"),
    ] {
        let row = document.create_element("div").unwrap();
        row.set_class_name("pp-row");
        let key = document.create_element("span").unwrap();
        key.set_class_name("pp-key");
        key.set_text_content(Some(k));
        let val = document.create_element("span").unwrap();
        val.set_class_name("pp-val");
        val.set_text_content(Some(v));
        row.append_child(&key).unwrap();
        row.append_child(&val).unwrap();
        card2.append_child(&row).unwrap();
    }
    wrapper.append_child(&card2).unwrap();
    wrapper
}

/// Conversations view (compact message list).
pub fn build_conversations_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();

    let messages: &[&str; 3] = &[
        "Alice: Hey, did you see the catchment report?",
        "Bob: Yes, reviewing the telemetry now.",
        "Sentinel AI: Flow rate stable at 142.5 L/m.",
    ];

    for msg_text in messages {
        let msg = document.create_element("div").unwrap();
        msg.set_class_name("chat-message human-msg");
        let av = document.create_element("div").unwrap();
        av.set_class_name("chat-avatar human-avatar");
        av.set_text_content(Some("\u{1F464}"));
        msg.append_child(&av).unwrap();
        let content = document.create_element("div").unwrap();
        content.set_class_name("chat-content");
        let text_el = document.create_element("div").unwrap();
        text_el.set_class_name("chat-text");
        text_el.set_text_content(Some(msg_text));
        content.append_child(&text_el).unwrap();
        msg.append_child(&content).unwrap();
        wrapper.append_child(&msg).unwrap();
    }
    wrapper
}

/// VibeScript console with syntax-highlighted editor and output panel.
pub fn build_vibescript_console(document: &Document) -> Element {
    let console = document.create_element("div").unwrap();
    console.set_class_name("vibe-console");
    super::surface_aspects::mark(&console, "entrance");

    // Peer tabs: Script | Catalog · Lexicon (same bind as Zone D IDE Catalog).
    let tabs = document.create_element("div").unwrap();
    tabs.set_class_name("studio-bay-tabs vibe-console-tabs");
    tabs.set_attribute("role", "tablist").ok();
    tabs.set_attribute("aria-label", "Script and Catalog · Lexicon").ok();

    let script_tab = document.create_element("button").unwrap();
    script_tab.set_class_name("studio-bay-tab is-active");
    script_tab.set_attribute("type", "button").ok();
    script_tab.set_attribute("data-bay-tab", "script").ok();
    script_tab.set_attribute("aria-selected", "true").ok();
    script_tab.set_text_content(Some("Script"));
    tabs.append_child(&script_tab).unwrap();

    let catalog_tab = document.create_element("button").unwrap();
    catalog_tab.set_class_name("studio-bay-tab");
    catalog_tab.set_attribute("type", "button").ok();
    catalog_tab.set_attribute("data-bay-tab", "catalog").ok();
    catalog_tab.set_attribute("aria-selected", "false").ok();
    catalog_tab.set_text_content(Some("Catalog · Lexicon"));
    tabs.append_child(&catalog_tab).unwrap();
    console.append_child(&tabs).unwrap();

    let script_pane = document.create_element("div").unwrap();
    script_pane.set_attribute("data-bay-pane", "script").ok();

    let toolbar = document.create_element("div").unwrap();
    toolbar.set_class_name("vibe-toolbar");
    let run_btn = document.create_element("button").unwrap();
    run_btn.set_class_name("vibe-run-btn");
    run_btn
        .set_attribute("data-instrument-action", "code:run")
        .unwrap();
    run_btn.set_text_content(Some("\u{25B6} Play"));
    toolbar.append_child(&run_btn).unwrap();
    let diag_btn = document.create_element("button").unwrap();
    diag_btn.set_class_name("vibe-run-btn");
    diag_btn.set_text_content(Some("\u{1F50D} Diagnose"));
    toolbar.append_child(&diag_btn).unwrap();
    script_pane.append_child(&toolbar).unwrap();

    let editor = document.create_element("div").unwrap();
    editor.set_class_name("vibe-editor");
    editor.set_attribute("contenteditable", "true").unwrap();
    editor
        .set_attribute("data-state-key", "vibescript-source")
        .unwrap();
    editor
        .set_attribute("aria-label", "VibeScript source")
        .unwrap();
    editor.set_text_content(Some(
        "// Author in *your* construct (POET shell). Not a shipped world.\n\
         capability.invoke(\"Poet.manifold_create\", { label: \"Cellular structure\", nest: true })\n\
         capability.invoke(\"Poet.container_place\", { container_type: \"doc\", title: \"Field notes\" })\n\
         capability.invoke(\"Poet.nested_link\", { to: \"anatomy\", title: \"Anatomy lens\" })\n\
         capability.invoke(\"Poet.subject_declare\", { label: \"North Spring catchment\" })\n\
         capability.invoke(\"Poet.manifold_create\", { label: \"Camping sites\", social: true })\n\
         capability.invoke(\"Poet.participant_invite\", { did: \"did:qualia:alice\", role: \"member\" })\n",
    ));
    script_pane.append_child(&editor).unwrap();

    let output = document.create_element("div").unwrap();
    output.set_class_name("vibe-output");
    output.set_text_content(Some("No VibeScript has been executed in this container."));
    script_pane.append_child(&output).unwrap();

    wire_vibescript_action(&run_btn, &editor, &output, false);
    wire_diagnose_action(&diag_btn, &editor, &output);
    console.append_child(&script_pane).unwrap();

    let catalog_peer = super::lexicon_bay::build_lexicon_bay(document);
    catalog_peer.set_attribute("data-bay-pane", "catalog").ok();
    catalog_peer.set_attribute("hidden", "").ok();
    console.append_child(&catalog_peer).unwrap();

    super::lexicon_bay::wire_bay_tabs(
        &tabs,
        &[("script", &script_pane), ("catalog", &catalog_peer)],
    );

    console
}

fn wire_diagnose_action(button: &Element, editor: &Element, output: &Element) {
    let editor = editor.clone();
    let output = output.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        let source = editor.text_content().unwrap_or_default();
        if source.trim().is_empty() {
            output.set_attribute("data-honesty", "error").ok();
            output.set_text_content(Some("Enter VibeScript source before diagnosing."));
            return;
        }
        let report = crate::vibe_host::diagnose(&source);
        super::diag_glow::paint_source_element(&editor, &source, &report);
        output.set_class_name("vibe-output diag-report");
        output
            .set_attribute("data-honesty", if report.valid { "live" } else { "error" })
            .ok();
        output
            .set_attribute("data-beat", if report.valid { "dwell" } else { "entrance" })
            .ok();
        output.set_text_content(Some(&super::diag_glow::format_human_report(&report)));
        if super::diag_glow::source_mentions_inference(&source) {
            output.set_attribute("data-inference-trail", "1").ok();
            let honesty = if report.valid { "unavailable" } else { "error" };
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                let trail = super::diag_glow::build_inference_trail(&doc, honesty, "Inference.*");
                let _ = output.append_child(&trail);
            }
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    button
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

fn wire_vibescript_action(button: &Element, editor: &Element, output: &Element, as_cell: bool) {
    let editor = editor.clone();
    let output = output.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        let source = editor.text_content().unwrap_or_default();
        if source.trim().is_empty() {
            output.set_attribute("data-honesty", "error").ok();
            output.set_text_content(Some("Enter VibeScript source before running."));
            return;
        }
        let authored = super::manifold_authoring::parse_authoring_ops(&source);
        if !authored.is_empty() {
            let lines = super::manifold_authoring::apply_authoring_ops(&authored);
            let mut log = String::from("POET authoring (local shell, not a canned world):\n");
            for line in &lines {
                log.push_str("  ");
                log.push_str(line);
                log.push('\n');
            }
            output.set_attribute("data-honesty", "live").ok();
            output.set_text_content(Some(&log));
            if super::diag_glow::source_mentions_inference(&source) {
                output.set_attribute("data-inference-trail", "1").ok();
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    let trail =
                        super::diag_glow::build_inference_trail(&doc, "unavailable", "Inference.*");
                    let _ = output.append_child(&trail);
                }
            }
            let other_invokes = source.matches("capability.invoke").count() > authored.len();
            if !other_invokes {
                return;
            }
        }
        if !super::native_daemon::is_daemon_connected() {
            output.set_attribute("data-honesty", "unavailable").ok();
            output.set_text_content(Some(
                "Unavailable: start the local QualiaDB daemon to execute VibeScript.",
            ));
            return;
        }
        output.set_attribute("data-honesty", "running").ok();
        output.set_text_content(Some("Executing VibeScript on the native daemon…"));
        let output = output.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match super::native_daemon::daemon_eval(&source, as_cell, None).await {
                Ok(response) if response.ok => {
                    output.set_attribute("data-honesty", "live").ok();
                    output.set_text_content(Some(&response.value));
                }
                Ok(response) => {
                    output.set_attribute("data-honesty", "error").ok();
                    output.set_text_content(Some(
                        response
                            .diagnostic
                            .as_deref()
                            .unwrap_or("VibeScript evaluation failed."),
                    ));
                }
                Err(error) => {
                    output.set_attribute("data-honesty", "error").ok();
                    output.set_text_content(Some(&error));
                }
            }
        });
    }) as Box<dyn FnMut(web_sys::Event)>);
    button
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

/// GIS map view — G-COORD realm chrome (Earth / Cosmos / Fiction) on live remaps.
pub fn build_gis_map_view(document: &Document) -> Element {
    super::g_coord::build_map_view(document)
}

/// Native-rendered 3D kinematics preview when a renderer daemon is available.
pub fn build_media_3d_view(document: &Document) -> Element {
    let viewport = document.create_element("div").unwrap();
    viewport.set_class_name("media-3d-viewport");
    viewport.set_attribute("data-media-surface", "3d").ok();
    viewport.set_attribute("data-shape", "container").ok();
    viewport.set_attribute("data-beat", "entrance").ok();

    let inner = document.create_element("div").unwrap();
    inner.set_class_name("media-3d-placeholder");

    let cube = document.create_element("div").unwrap();
    cube.set_class_name("media-3d-cube");
    inner.append_child(&cube).unwrap();

    let label = document.create_element("div").unwrap();
    label.set_text_content(Some(
        "3D Kinematics Viewport · request a genuine offscreen frame from the connected native renderer.",
    ));
    label.set_attribute("role", "status").unwrap();
    label.set_attribute("data-honesty", "present").unwrap();
    inner.append_child(&label).unwrap();

    inner
        .append_child(&super::render_preview::build(document, "media", 800, 480))
        .unwrap();

    viewport.append_child(&inner).unwrap();
    viewport
}

// ---------------------------------------------------------------------------
// Rich Interactive Panels (Reputation, Capabilities, Settings, Channels, Presence)
// ---------------------------------------------------------------------------

/// Reputation & Trustworthiness governance panel.
pub fn build_reputation_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 10px; gap: 8px; \
         background: var(--surface-glass); color: var(--text-primary); font-family: var(--font-mono); overflow-y: auto;"
    );

    // Summary Header
    let header = document.create_element("div").unwrap();
    header.set_class_name("vibe-toolbar");
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el
        .style()
        .set_css_text("justify-content: space-between; padding: 4px 8px;");

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("\u{1F91D} Fiduciary Reputation Index"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; color: var(--accent-cyan); font-size: 11px;");
    header.append_child(&title).unwrap();

    let score = document.create_element("span").unwrap();
    score.set_text_content(Some("Score: 99.3% \u{00B7} Tier: AAA+"));
    let score_el: HtmlElement = score.clone().dyn_into().unwrap();
    score_el
        .style()
        .set_css_text("font-size: 10px; color: var(--accent-emerald); font-weight: 600;");
    header.append_child(&score).unwrap();
    wrapper.append_child(&header).unwrap();

    // 4 Pillar Meters
    let pillars = [
        (
            "Trustworthiness",
            99.4,
            "var(--accent-cyan)",
            "Ed25519 signature consensus verified",
        ),
        (
            "Competence",
            98.1,
            "var(--accent-emerald)",
            "SHACL & formal proof verification: 100% pass",
        ),
        (
            "Integrity",
            100.0,
            "var(--accent-violet)",
            "42MB Sentinel budget adhered; 0 heap leaks",
        ),
        (
            "Conduct",
            99.8,
            "var(--accent-amber)",
            "Cooperative integrity permanent audit log clean",
        ),
    ];

    for (name, pct, color, desc) in pillars {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text(
            "background: var(--surface-panel); border: 1px solid var(--border-subtle); \
             border-radius: var(--radius-xs); padding: 6px 8px; display: flex; flex-direction: column; gap: 3px;"
        );

        let row = document.create_element("div").unwrap();
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el
            .style()
            .set_css_text("display: flex; justify-content: space-between; font-size: 10px;");

        let name_el = document.create_element("span").unwrap();
        name_el.set_text_content(Some(name));
        name_el.set_attribute("style", "font-weight: 600;").unwrap();
        row.append_child(&name_el).unwrap();

        let val_el = document.create_element("span").unwrap();
        val_el.set_text_content(Some(&format!("{:.1}%", pct)));
        val_el
            .set_attribute("style", &format!("color: {}; font-weight: 700;", color))
            .unwrap();
        row.append_child(&val_el).unwrap();
        card.append_child(&row).unwrap();

        // Progress track
        let track = document.create_element("div").unwrap();
        let track_el: HtmlElement = track.clone().dyn_into().unwrap();
        track_el.style().set_css_text(
            "height: 4px; background: rgba(255,255,255,0.08); border-radius: 2px; overflow: hidden;"
        );
        let bar = document.create_element("div").unwrap();
        let bar_el: HtmlElement = bar.clone().dyn_into().unwrap();
        bar_el.style().set_css_text(&format!(
            "height: 100%; width: {}%; background: {}; border-radius: 2px;",
            pct, color
        ));
        track.append_child(&bar).unwrap();
        card.append_child(&track).unwrap();

        let desc_el = document.create_element("span").unwrap();
        desc_el.set_text_content(Some(desc));
        desc_el
            .set_attribute("style", "font-size: 9px; color: var(--text-muted);")
            .unwrap();
        card.append_child(&desc_el).unwrap();

        wrapper.append_child(&card).unwrap();
    }

    wrapper
}

/// Capability badge registry & permission grants panel.
pub fn build_capabilities_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 10px; gap: 8px; \
         background: var(--surface-glass); color: var(--text-primary); font-family: var(--font-mono); overflow-y: auto;"
    );

    let header = document.create_element("div").unwrap();
    header.set_class_name("vibe-toolbar");
    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("\u{1F511} Active Capability Manifests"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; color: var(--accent-cyan); font-size: 11px;");
    header.append_child(&title).unwrap();
    wrapper.append_child(&header).unwrap();

    let caps = [
        (
            "graph:read_triple",
            "Active",
            "var(--accent-emerald)",
            "Query quads within author scope",
        ),
        (
            "graph:write_quin",
            "Active",
            "var(--accent-emerald)",
            "Emit 48-byte Super-Quins into 42MB arena",
        ),
        (
            "spatial:matrix_transform",
            "Active",
            "var(--accent-emerald)",
            "10D affine manifold coordinate projection",
        ),
        (
            "webrtc:p2p_channel",
            "Active",
            "var(--accent-emerald)",
            "Swarm DataChannel state synchronization",
        ),
        (
            "qpu:vqe_scheduler",
            "Scoped",
            "var(--accent-amber)",
            "Quantum chemistry job queue execution",
        ),
        (
            "daemon:native_exec",
            "Restricted",
            "var(--accent-rose)",
            "Loopback IPC command dispatch",
        ),
    ];

    for (name, status, status_color, detail) in caps {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text(
            "background: var(--surface-panel); border: 1px solid var(--border-subtle); \
             border-radius: var(--radius-xs); padding: 6px 8px; display: flex; align-items: center; justify-content: space-between;"
        );

        let left = document.create_element("div").unwrap();
        let left_el: HtmlElement = left.clone().dyn_into().unwrap();
        left_el
            .style()
            .set_css_text("display: flex; flex-direction: column; gap: 2px;");

        let name_el = document.create_element("span").unwrap();
        name_el.set_text_content(Some(name));
        name_el
            .set_attribute("style", "font-weight: 600; font-size: 11px;")
            .unwrap();
        left.append_child(&name_el).unwrap();

        let detail_el = document.create_element("span").unwrap();
        detail_el.set_text_content(Some(detail));
        detail_el
            .set_attribute("style", "font-size: 9px; color: var(--text-muted);")
            .unwrap();
        left.append_child(&detail_el).unwrap();
        card.append_child(&left).unwrap();

        let badge = document.create_element("span").unwrap();
        badge.set_text_content(Some(status));
        let badge_el: HtmlElement = badge.clone().dyn_into().unwrap();
        badge_el.style().set_css_text(&format!(
            "padding: 2px 6px; border-radius: 4px; font-size: 9px; font-weight: 700; \
             background: rgba(255,255,255,0.06); color: {}; border: 1px solid {};",
            status_color, status_color
        ));
        card.append_child(&badge).unwrap();

        wrapper.append_child(&card).unwrap();
    }

    wrapper
}

/// Preferences & Environment Settings panel.
pub fn build_settings_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 10px; gap: 8px; \
         background: var(--surface-glass); color: var(--text-primary); font-family: var(--font-mono); overflow-y: auto;"
    );

    let header = document.create_element("div").unwrap();
    header.set_class_name("vibe-toolbar");
    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("\u{2699}\u{FE0F} Poet System Preferences"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; color: var(--accent-cyan); font-size: 11px;");
    header.append_child(&title).unwrap();
    wrapper.append_child(&header).unwrap();

    let settings = [
        ("Theme Palette", "Cyber Dark (Default)"),
        ("42MB Prolog Sentinel", "Active \u{00B7} Zero-Leak Enforced"),
        ("DirectML / WebGPU Backend", "wgpu 30 Shared Context"),
        ("Daemon SSE Transport", "Connected \u{00B7} 127.0.0.1:3001"),
        ("Quantum Chemistry Engine", "Pure Rust Autodiff DFT Enabled"),
        (
            "Auto-Save & WAL Journals",
            "Continuous Real-Time Checkpointing",
        ),
    ];

    for (label, val) in settings {
        let row = document.create_element("div").unwrap();
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el.style().set_css_text(
            "background: var(--surface-panel); border: 1px solid var(--border-subtle); \
             border-radius: var(--radius-xs); padding: 8px; display: flex; justify-content: space-between; align-items: center; font-size: 10px;"
        );

        let label_el = document.create_element("span").unwrap();
        label_el.set_text_content(Some(label));
        label_el
            .set_attribute("style", "color: var(--text-secondary);")
            .unwrap();
        row.append_child(&label_el).unwrap();

        let val_el = document.create_element("span").unwrap();
        val_el.set_text_content(Some(val));
        val_el
            .set_attribute("style", "color: var(--accent-cyan); font-weight: 600;")
            .unwrap();
        row.append_child(&val_el).unwrap();

        wrapper.append_child(&row).unwrap();
    }

    wrapper
}

/// Swarm Communications & Broadcast Channels panel.
pub fn build_channels_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 10px; gap: 8px; \
         background: var(--surface-glass); color: var(--text-primary); font-family: var(--font-mono); overflow-y: auto;"
    );

    let header = document.create_element("div").unwrap();
    header.set_class_name("vibe-toolbar");
    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("\u{1F4E1} Swarm Federation Channels"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; color: var(--accent-cyan); font-size: 11px;");
    header.append_child(&title).unwrap();
    wrapper.append_child(&header).unwrap();

    let channels = [
        (
            "#direct-fiduciary",
            "1:1 Consent Pipeline",
            "Online",
            "2 peers",
        ),
        (
            "#topic:catchment:water-quality",
            "Hydrology Sensor Telemetry",
            "Active",
            "12 nodes",
        ),
        (
            "#topic:chemistry:vqe-grid",
            "Distributed Q-Forge Scheduler",
            "Syncing",
            "6 nodes",
        ),
        (
            "#federation:mesh-commons",
            "Cross-Domain Interoperability",
            "Online",
            "34 peers",
        ),
    ];

    for (name, desc, status, peers) in channels {
        let row = document.create_element("div").unwrap();
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el.style().set_css_text(
            "background: var(--surface-panel); border: 1px solid var(--border-subtle); \
             border-radius: var(--radius-xs); padding: 8px; display: flex; justify-content: space-between; align-items: center;"
        );

        let left = document.create_element("div").unwrap();
        let left_el: HtmlElement = left.clone().dyn_into().unwrap();
        left_el
            .style()
            .set_css_text("display: flex; flex-direction: column; gap: 2px;");

        let name_el = document.create_element("span").unwrap();
        name_el.set_text_content(Some(name));
        name_el
            .set_attribute(
                "style",
                "font-weight: 600; font-size: 11px; color: var(--accent-cyan);",
            )
            .unwrap();
        left.append_child(&name_el).unwrap();

        let desc_el = document.create_element("span").unwrap();
        desc_el.set_text_content(Some(desc));
        desc_el
            .set_attribute("style", "font-size: 9px; color: var(--text-muted);")
            .unwrap();
        left.append_child(&desc_el).unwrap();
        row.append_child(&left).unwrap();

        let right = document.create_element("div").unwrap();
        let right_el: HtmlElement = right.clone().dyn_into().unwrap();
        right_el
            .style()
            .set_css_text("text-align: right; font-size: 9px;");

        let status_el = document.create_element("span").unwrap();
        status_el.set_text_content(Some(&format!("\u{25CF} {}", status)));
        status_el
            .set_attribute(
                "style",
                "color: var(--accent-emerald); font-weight: 600; display: block;",
            )
            .unwrap();
        right.append_child(&status_el).unwrap();

        let peers_el = document.create_element("span").unwrap();
        peers_el.set_text_content(Some(peers));
        peers_el
            .set_attribute("style", "color: var(--text-muted);")
            .unwrap();
        right.append_child(&peers_el).unwrap();
        row.append_child(&right).unwrap();

        wrapper.append_child(&row).unwrap();
    }

    wrapper
}

/// Agent & Peer Presence roster panel.
pub fn build_presence_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 10px; gap: 8px; \
         background: var(--surface-glass); color: var(--text-primary); font-family: var(--font-mono); overflow-y: auto;"
    );

    let header = document.create_element("div").unwrap();
    header.set_class_name("vibe-toolbar");
    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("\u{1F465} Swarm Node Presence"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; color: var(--accent-cyan); font-size: 11px;");
    header.append_child(&title).unwrap();
    wrapper.append_child(&header).unwrap();

    let peers = [
        (
            "did:qualia:timothy_charles_holborn",
            "Human Principal",
            "\u{1F468}\u{200D}\u{1F4BB}",
            "Online \u{2014} Active",
            "var(--accent-emerald)",
        ),
        (
            "did:qualia:agent-sentinel-42",
            "42MB Sentinel Daemon",
            "\u{1F6E1}\u{FE0F}",
            "Online \u{2014} Monitoring",
            "var(--accent-emerald)",
        ),
        (
            "did:qualia:agent-qforge-gpu",
            "Q-Forge Accelerator",
            "\u{26A1}",
            "Active \u{2014} Compute Loop",
            "var(--accent-amber)",
        ),
        (
            "did:qualia:peer-melbourne-node",
            "Edge Swarm Node",
            "\u{1F310}",
            "Idle \u{2014} Ping 12ms",
            "var(--text-secondary)",
        ),
    ];

    for (did, role, icon, status, color) in peers {
        let row = document.create_element("div").unwrap();
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el.style().set_css_text(
            "background: var(--surface-panel); border: 1px solid var(--border-subtle); \
             border-radius: var(--radius-xs); padding: 8px; display: flex; align-items: center; gap: 8px;"
        );

        let icon_el = document.create_element("span").unwrap();
        icon_el.set_text_content(Some(icon));
        icon_el.set_attribute("style", "font-size: 16px;").unwrap();
        row.append_child(&icon_el).unwrap();

        let mid = document.create_element("div").unwrap();
        let mid_el: HtmlElement = mid.clone().dyn_into().unwrap();
        mid_el
            .style()
            .set_css_text("flex: 1; display: flex; flex-direction: column; gap: 1px;");

        let did_el = document.create_element("span").unwrap();
        did_el.set_text_content(Some(did));
        did_el
            .set_attribute(
                "style",
                "font-weight: 600; font-size: 10px; color: var(--text-primary);",
            )
            .unwrap();
        mid.append_child(&did_el).unwrap();

        let role_el = document.create_element("span").unwrap();
        role_el.set_text_content(Some(role));
        role_el
            .set_attribute("style", "font-size: 9px; color: var(--text-muted);")
            .unwrap();
        mid.append_child(&role_el).unwrap();
        row.append_child(&mid).unwrap();

        let status_el = document.create_element("span").unwrap();
        status_el.set_text_content(Some(status));
        let s_el: HtmlElement = status_el.clone().dyn_into().unwrap();
        s_el.style().set_css_text(&format!(
            "font-size: 9px; color: {}; font-weight: 600;",
            color
        ));
        row.append_child(&status_el).unwrap();

        wrapper.append_child(&row).unwrap();
    }

    wrapper
}
