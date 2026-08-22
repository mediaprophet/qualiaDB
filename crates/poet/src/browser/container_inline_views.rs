//! Inline container view builders extracted from containers.rs.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// Social chat graph with messages and agent avatars.
pub fn build_social_chat_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();

    let messages: &[(&str, &str, &str, &str, &str, bool, &str)] = &[
        ("TH", "Timothy Charles Holborn", "14:38",
         "North Spring looks clearer after last night's rain; the field notes should capture that.",
         "subjective", false, "\u{1F9E0}"),
        ("SP", "Sentinel Planner AI", "14:39",
         "Telemetry sensor confirms flow at 142.5 L/m with 221.5 Hz acoustic resonance.",
         "objective", true, "\u{1F50C}"),
        ("FR", "Fiduciary Rights AI", "14:40",
         "Asserted legal quad: <<[ site:NorthSpring hydro:status \"Monitored\" ]>>.",
         "normative", true, "\u{2696}"),
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

    let toolbar = document.create_element("div").unwrap();
    toolbar.set_class_name("vibe-toolbar");
    let run_btn = document.create_element("button").unwrap();
    run_btn.set_class_name("vibe-run-btn");
    run_btn.set_text_content(Some("\u{25B6} Run"));
    toolbar.append_child(&run_btn).unwrap();
    let diag_btn = document.create_element("button").unwrap();
    diag_btn.set_class_name("vibe-run-btn");
    diag_btn.set_text_content(Some("\u{1F50D} Diagnose"));
    toolbar.append_child(&diag_btn).unwrap();
    console.append_child(&toolbar).unwrap();

    let editor = document.create_element("div").unwrap();
    editor.set_class_name("vibe-editor");
    editor.set_text_content(Some(
        "// VibeScript \u{2014} human door into Qualia\n\
         @intent social:connection_request {\n\
         \x20\x20target: did:qualia:alice\n\
         \x20\x20predicate: soc:friendship\n\
         \x20\x20proof: zkp:age_over_18\n\
         }\n\
         \n\
         @intent social:assess_risk {\n\
         \x20\x20target: did:qualia:alice\n\
         \x20\x20indicators: [new-account, no-shared-contacts]\n\
         }",
    ));
    console.append_child(&editor).unwrap();

    let output = document.create_element("div").unwrap();
    output.set_class_name("vibe-output");
    let line1 = document.create_element("div").unwrap();
    line1.set_class_name("vibe-out-line");
    line1.set_text_content(Some(
        "\u{2705} Intent dispatched: social:connection_request",
    ));
    let line2 = document.create_element("div").unwrap();
    line2.set_class_name("vibe-out-line");
    line2.set_text_content(Some("\u{2705} Risk assessment: low (2 indicators)"));
    let line3 = document.create_element("div").unwrap();
    line3.set_class_name("vibe-out-line");
    line3.set_text_content(Some(
        "\u{2139}\u{FE0F} Receipt: 0x8f...a42 \u{00B7} CBOR-LD: 184 bytes",
    ));
    output.append_child(&line1).unwrap();
    output.append_child(&line2).unwrap();
    output.append_child(&line3).unwrap();
    console.append_child(&output).unwrap();

    console
}

/// GIS map view with layer controls and agent pins.
pub fn build_gis_map_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1;");

    let layer_bar = document.create_element("div").unwrap();
    layer_bar.set_class_name("gis-layer-bar");
    for (label, active) in &[
        ("Terrain", true),
        ("Water", true),
        ("Agents", true),
        ("Legal", false),
        ("Risk", false),
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("gis-layer-btn");
        if *active {
            btn.class_list().add_1("active").unwrap();
        }
        btn.set_text_content(Some(label));
        layer_bar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&layer_bar).unwrap();

    let svg = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "svg")
        .unwrap();
    svg.set_attribute("class", "gis-map-svg").unwrap();
    svg.set_attribute("viewBox", "0 0 400 300").unwrap();
    svg.set_attribute("preserveAspectRatio", "xMidYMid slice")
        .unwrap();

    let river = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "path")
        .unwrap();
    river
        .set_attribute("d", "M 50 50 Q 120 80 180 120 T 350 250")
        .unwrap();
    river.set_attribute("fill", "none").unwrap();
    river
        .set_attribute("stroke", "rgba(56, 189, 248, 0.5)")
        .unwrap();
    river.set_attribute("stroke-width", "3").unwrap();
    svg.append_child(&river).unwrap();

    let spring = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "circle")
        .unwrap();
    spring.set_attribute("cx", "180").unwrap();
    spring.set_attribute("cy", "120").unwrap();
    spring.set_attribute("r", "6").unwrap();
    spring
        .set_attribute("fill", "var(--accent-emerald)")
        .unwrap();
    svg.append_child(&spring).unwrap();

    let spring_label = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "text")
        .unwrap();
    spring_label.set_attribute("x", "190").unwrap();
    spring_label.set_attribute("y", "115").unwrap();
    spring_label
        .set_attribute("fill", "var(--accent-emerald)")
        .unwrap();
    spring_label.set_attribute("font-size", "9").unwrap();
    spring_label
        .set_attribute("font-family", "var(--font-mono)")
        .unwrap();
    spring_label.set_text_content(Some("North Spring"));
    svg.append_child(&spring_label).unwrap();

    let pin1 = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "g")
        .unwrap();
    pin1.set_attribute("class", "agent-pin-marker").unwrap();
    let pin1_circle = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "circle")
        .unwrap();
    pin1_circle.set_attribute("cx", "80").unwrap();
    pin1_circle.set_attribute("cy", "60").unwrap();
    pin1_circle.set_attribute("r", "6").unwrap();
    pin1_circle
        .set_attribute("fill", "var(--accent-amber)")
        .unwrap();
    pin1.append_child(&pin1_circle).unwrap();
    svg.append_child(&pin1).unwrap();

    let pin2 = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "g")
        .unwrap();
    pin2.set_attribute("class", "agent-pin-marker").unwrap();
    let pin2_circle = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "circle")
        .unwrap();
    pin2_circle.set_attribute("cx", "280").unwrap();
    pin2_circle.set_attribute("cy", "180").unwrap();
    pin2_circle.set_attribute("r", "6").unwrap();
    pin2_circle
        .set_attribute("fill", "var(--color-ai)")
        .unwrap();
    pin2.append_child(&pin2_circle).unwrap();
    svg.append_child(&pin2).unwrap();

    wrapper.append_child(&svg).unwrap();
    wrapper
}

/// 3D media viewport placeholder with spinning cube.
pub fn build_media_3d_view(document: &Document) -> Element {
    let viewport = document.create_element("div").unwrap();
    viewport.set_class_name("media-3d-viewport");

    let inner = document.create_element("div").unwrap();
    inner.set_class_name("media-3d-placeholder");

    let cube = document.create_element("div").unwrap();
    cube.set_class_name("media-3d-cube");
    inner.append_child(&cube).unwrap();

    let label = document.create_element("div").unwrap();
    label.set_text_content(Some(
        "3D Kinematics Viewport \u{2014} awaiting wgpu integration",
    ));
    inner.append_child(&label).unwrap();

    viewport.append_child(&inner).unwrap();
    viewport
}
