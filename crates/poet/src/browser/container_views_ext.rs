//! Additional container body views: library, aura, latex, health, anatomy,
//! webview, webrtc, finance, vision, listen, triad, portal, slide, 3d, subcanvas.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Library / Lived Memory (§3.8b — P0 critical gap)
// ---------------------------------------------------------------------------

/// Hypermedia library browser — 8 sections, search, facet filters, stats.
pub fn build_library_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    // Stats bar
    let stats = document.create_element("div").unwrap();
    stats.set_class_name("vibe-toolbar");
    let stats_el: HtmlElement = stats.clone().dyn_into().unwrap();
    stats_el.style().set_css_text(
        "gap: 12px; font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);",
    );
    for (label, value) in &[
        ("Documents", "142"),
        ("Ontologies", "36"),
        ("Models", "8"),
        ("Graph facts", "12.4k"),
    ] {
        let stat = document.create_element("span").unwrap();
        stat.set_text_content(Some(&format!("{}: {}", label, value)));
        stats.append_child(&stat).unwrap();
    }
    wrapper.append_child(&stats).unwrap();

    // Section tabs
    let tabs = document.create_element("div").unwrap();
    tabs.set_class_name("vibe-toolbar");
    let tabs_el: HtmlElement = tabs.clone().dyn_into().unwrap();
    tabs_el.style().set_css_text("gap: 2px; flex-wrap: wrap;");
    for section in &[
        "All", "Secret", "Wellfair", "Personal", "Work", "Tools", "Software", "Commons",
    ] {
        let tab = document.create_element("button").unwrap();
        tab.set_class_name("vibe-run-btn");
        let tab_el: HtmlElement = tab.clone().dyn_into().unwrap();
        tab_el
            .style()
            .set_css_text("font-size: 10px; padding: 2px 8px;");
        tab.set_text_content(Some(section));
        tabs.append_child(&tab).unwrap();
    }
    wrapper.append_child(&tabs).unwrap();

    // Search bar
    let search = document.create_element("div").unwrap();
    search.set_class_name("vibe-toolbar");
    let input = document.create_element("input").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.set_placeholder("Search library\u{2026}");
    input.set_attribute("style", "flex: 1; background: var(--canvas-bg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 4px 8px; color: var(--text-primary); font-size: 11px; font-family: var(--font-mono);").unwrap();
    search.append_child(&input).unwrap();
    let facet_btn = document.create_element("button").unwrap();
    facet_btn.set_class_name("vibe-run-btn");
    facet_btn.set_text_content(Some("Facets"));
    search.append_child(&facet_btn).unwrap();
    let ingest_btn = document.create_element("button").unwrap();
    ingest_btn.set_class_name("vibe-run-btn");
    ingest_btn.set_text_content(Some("+ Ingest"));
    search.append_child(&ingest_btn).unwrap();
    wrapper.append_child(&search).unwrap();

    // Entry list
    let list = document.create_element("div").unwrap();
    list.set_class_name("vibe-output");
    let entries = &[
        (
            "doc",
            "Research notes — Q42 embedding manifold",
            "wellfair",
            "present",
        ),
        (
            "ontology",
            "epistemics.n3 — epistemic modalities",
            "software",
            "live",
        ),
        (
            "pdf",
            "Framingham risk assessment — clinical",
            "wellfair",
            "partial",
        ),
        (
            "image",
            "Satellite imagery — catchment hull",
            "work",
            "present",
        ),
        ("model", "Llama-3.2-1B — quantised GGUF", "tools", "live"),
        (
            "text",
            "Agreement draft — fiduciary obligation",
            "personal",
            "present",
        ),
        (
            "ontology",
            "hypermedia.n3 — HCF container spec",
            "software",
            "live",
        ),
        (
            "text",
            "Legislation — Health Records Act 2024",
            "commons",
            "present",
        ),
    ];
    for (kind, title, section, honesty) in entries {
        let row = document.create_element("div").unwrap();
        row.set_class_name("vibe-out-line");
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el
            .style()
            .set_css_text("display: flex; align-items: center; gap: 8px;");

        let kind_badge = document.create_element("span").unwrap();
        kind_badge.set_class_name(&format!("honesty-badge honesty-{}", honesty));
        kind_badge.set_text_content(Some(kind));
        row.append_child(&kind_badge).unwrap();

        let title_el = document.create_element("span").unwrap();
        title_el.set_text_content(Some(title));
        row.append_child(&title_el).unwrap();

        let section_el = document.create_element("span").unwrap();
        let s_el: HtmlElement = section_el.clone().dyn_into().unwrap();
        s_el.style()
            .set_css_text("margin-left: auto; color: var(--text-muted); font-size: 9px;");
        section_el.set_text_content(Some(section));
        row.append_child(&section_el).unwrap();

        list.append_child(&row).unwrap();
    }
    wrapper.append_child(&list).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Aura (SHACL inspector)
// ---------------------------------------------------------------------------

/// SHACL validation inspector — shape list, conformance results.
pub fn build_aura_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    // Toolbar
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    let run_btn = document.create_element("button").unwrap();
    run_btn.set_class_name("vibe-run-btn");
    run_btn.set_text_content(Some("\u{25B6} Validate"));
    bar.append_child(&run_btn).unwrap();
    let label = document.create_element("span").unwrap();
    label.set_text_content(Some("Shapes:"));
    let l_el: HtmlElement = label.clone().dyn_into().unwrap();
    l_el.style().set_css_text("color: var(--text-muted); font-size: 10px; font-family: var(--font-mono); margin-left: 8px;");
    bar.append_child(&label).unwrap();
    wrapper.append_child(&bar).unwrap();

    // Shape results
    let results = document.create_element("div").unwrap();
    results.set_class_name("vibe-output");
    let shapes = &[
        ("soc:PeerShape", "conformant", "42 nodes validated"),
        ("soc:AgreementShape", "conformant", "8 nodes validated"),
        (
            "health:RecordShape",
            "violation",
            "2 nodes: missing `health:hasConsent`",
        ),
        ("rights:FiduciaryShape", "conformant", "3 nodes validated"),
        ("vibe:IntentShape", "conformant", "156 nodes validated"),
    ];
    for (shape, status, detail) in shapes {
        let row = document.create_element("div").unwrap();
        row.set_class_name("vibe-out-line");
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el
            .style()
            .set_css_text("display: flex; align-items: center; gap: 8px;");

        let badge = document.create_element("span").unwrap();
        let badge_class = if *status == "violation" {
            "honesty-badge honesty-missing"
        } else {
            "honesty-badge honesty-live"
        };
        badge.set_class_name(badge_class);
        badge.set_text_content(Some(status));
        row.append_child(&badge).unwrap();

        let shape_el = document.create_element("span").unwrap();
        let sh_el: HtmlElement = shape_el.clone().dyn_into().unwrap();
        sh_el
            .style()
            .set_css_text("color: var(--accent-cyan); font-family: var(--font-mono);");
        shape_el.set_text_content(Some(shape));
        row.append_child(&shape_el).unwrap();

        let detail_el = document.create_element("span").unwrap();
        let d_el: HtmlElement = detail_el.clone().dyn_into().unwrap();
        d_el.style()
            .set_css_text("color: var(--text-muted); font-size: 10px; margin-left: auto;");
        detail_el.set_text_content(Some(detail));
        row.append_child(&detail_el).unwrap();

        results.append_child(&row).unwrap();
    }
    wrapper.append_child(&results).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Latex (CAS invoke)
// ---------------------------------------------------------------------------

/// LaTeX editor — snippet bar, CAS invoke, symbolic algebra.
pub fn build_latex_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 4px;");

    // Snippet bar
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    for label in &[
        "\\frac", "\\sum", "\\int", "\\sqrt", "\\alpha", "\\nabla", "CAS",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        let btn_el: HtmlElement = btn.clone().dyn_into().unwrap();
        btn_el
            .style()
            .set_css_text("font-size: 10px; padding: 2px 6px; font-family: var(--font-mono);");
        btn.set_text_content(Some(label));
        bar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&bar).unwrap();

    // Editor
    let editor = document.create_element("div").unwrap();
    editor.set_class_name("vibe-editor");
    editor.set_text_content(Some(
        "\\documentclass{article}\n\
         \\begin{document}\n\
         \\section{Quantum DFT Ground State}\n\
         $$E_0 = \\min_{\\psi} \\langle \\psi | \\hat{H} | \\psi \\rangle$$\n\
         \\end{document}",
    ));
    wrapper.append_child(&editor).unwrap();

    // CAS output
    let output = document.create_element("div").unwrap();
    output.set_class_name("vibe-output");
    let line = document.create_element("div").unwrap();
    line.set_class_name("vibe-out-line");
    line.set_text_content(Some(
        "\u{2139}\u{FE0F} CAS: awaiting SymbolicAlgebra engine wiring (native_bindings)",
    ));
    output.append_child(&line).unwrap();
    wrapper.append_child(&output).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Health (Framingham, clinical, consent-gated)
// ---------------------------------------------------------------------------

/// Health container — Framingham risk, medication, consent gate.
pub fn build_health_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px;");

    // Consent gate notice
    let gate = document.create_element("div").unwrap();
    gate.set_class_name("cr-card");
    let gate_el: HtmlElement = gate.clone().dyn_into().unwrap();
    gate_el
        .style()
        .set_css_text("border-color: var(--accent-amber); border-width: 1px; border-style: solid;");
    gate.set_text_content(Some(
        "\u{26A0}\u{FE0F} Health data is consent-gated. \
         Wellfair commands require `wellfair_*_consent_creds` \
         before reads or writes.",
    ));
    wrapper.append_child(&gate).unwrap();

    // Framingham risk card
    let risk = document.create_element("div").unwrap();
    risk.set_class_name("cr-card");
    let h = document.create_element("div").unwrap();
    h.set_class_name("cr-header");
    let title = document.create_element("span").unwrap();
    title.set_class_name("cr-name");
    title.set_text_content(Some("Framingham 10-Year Risk"));
    h.append_child(&title).unwrap();
    let badge = document.create_element("span").unwrap();
    badge.set_class_name("honesty-badge honesty-partial");
    badge.set_text_content(Some("partial"));
    h.append_child(&badge).unwrap();
    risk.append_child(&h).unwrap();
    let meta = document.create_element("div").unwrap();
    meta.set_class_name("cr-meta");
    meta.set_text_content(Some(
        "Score: 12% (moderate) \u{00B7} Age: 45 \u{00B7} SBP: 130 \u{00B7} TC: 5.2",
    ));
    risk.append_child(&meta).unwrap();
    wrapper.append_child(&risk).unwrap();

    // Medication list
    let meds = document.create_element("div").unwrap();
    meds.set_class_name("vibe-output");
    for med in &[
        "Metformin 500mg \u{00B7} twice daily",
        "Atorvastatin 20mg \u{00B7} once daily",
    ] {
        let line = document.create_element("div").unwrap();
        line.set_class_name("vibe-out-line");
        line.set_text_content(Some(med));
        meds.append_child(&line).unwrap();
    }
    wrapper.append_child(&meds).unwrap();

    // Actions
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &["Calculate Risk", "Medications", "Records", "Consent"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Anatomy (10D, organ percepts)
// ---------------------------------------------------------------------------

/// Anatomy container — 10D body view, organ percepts, comorbidity.
pub fn build_anatomy_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    let placeholder = document.create_element("div").unwrap();
    placeholder.set_class_name("container-placeholder");
    placeholder.set_text_content(Some(
        "10D anatomy viewport \u{2014} awaiting GPU surface bridge. \
         Organ percepts, comorbidity scorecard, and FMA/SNOMED-CT \
         extraction will render here via `mount_gpu_surface` + \
         `browse_10d_containers`.",
    ));
    wrapper.append_child(&placeholder).unwrap();

    // Organ list
    let organs = document.create_element("div").unwrap();
    organs.set_class_name("vibe-output");
    for organ in &[
        "Heart \u{00B7} FMA:7088",
        "Lungs \u{00B7} FMA:7195",
        "Liver \u{00B7} FMA:7203",
        "Brain \u{00B7} FMA:50801",
    ] {
        let line = document.create_element("div").unwrap();
        line.set_class_name("vibe-out-line");
        line.set_text_content(Some(organ));
        organs.append_child(&line).unwrap();
    }
    wrapper.append_child(&organs).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Webview (browser pane — desktop only)
// ---------------------------------------------------------------------------

/// Webview container — capability-gated browser pane.
pub fn build_webview_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 4px;");

    // URL bar
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    let input = document.create_element("input").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.set_placeholder("https://\u{2026}");
    input.set_attribute("style", "flex: 1; background: var(--canvas-bg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 4px 8px; color: var(--text-primary); font-size: 11px; font-family: var(--font-mono);").unwrap();
    bar.append_child(&input).unwrap();
    let go_btn = document.create_element("button").unwrap();
    go_btn.set_class_name("vibe-run-btn");
    go_btn.set_text_content(Some("\u{2192}"));
    bar.append_child(&go_btn).unwrap();
    wrapper.append_child(&bar).unwrap();

    // Capability gate notice
    let notice = document.create_element("div").unwrap();
    notice.set_class_name("container-placeholder");
    notice.set_text_content(Some(
        "Browser pane requires desktop webview (`supports_browser_pane()`). \
         Trust store, cert override, and cookie observation are \
         capability-gated. Public web: unavailable.",
    ));
    wrapper.append_child(&notice).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// WebRTC (present — no fake stream until consent gate)
// ---------------------------------------------------------------------------

/// WebRTC container — present but no stream until consent gate designed.
pub fn build_webrtc_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    let notice = document.create_element("div").unwrap();
    notice.set_class_name("cr-card");
    let n_el: HtmlElement = notice.clone().dyn_into().unwrap();
    n_el.style()
        .set_css_text("border-color: var(--accent-amber);");
    notice.set_text_content(Some(
        "WebRTC is `present` \u{2014} no stream is rendered until \
         the consent gate is designed. ICE servers are hardcoded \
         in the desktop host. Peer connection manager awaits \
         consent UI.",
    ));
    wrapper.append_child(&notice).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Finance (Black-Scholes, portfolio)
// ---------------------------------------------------------------------------

/// Finance container — portfolio, Black-Scholes, ledger entries.
pub fn build_finance_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    // Portfolio summary
    let summary = document.create_element("div").unwrap();
    summary.set_class_name("cr-card");
    let h = document.create_element("div").unwrap();
    h.set_class_name("cr-header");
    let title = document.create_element("span").unwrap();
    title.set_class_name("cr-name");
    title.set_text_content(Some("Portfolio"));
    h.append_child(&title).unwrap();
    let badge = document.create_element("span").unwrap();
    badge.set_class_name("honesty-badge honesty-partial");
    badge.set_text_content(Some("partial"));
    h.append_child(&badge).unwrap();
    summary.append_child(&h).unwrap();
    let meta = document.create_element("div").unwrap();
    meta.set_class_name("cr-meta");
    meta.set_text_content(Some(
        "Total: $4,250.00 \u{00B7} XEC: 1,250 \u{00B7} USDC: 340 \u{00B7} Q42: 8,000",
    ));
    summary.append_child(&meta).unwrap();
    wrapper.append_child(&summary).unwrap();

    // Ledger
    let ledger = document.create_element("div").unwrap();
    ledger.set_class_name("vibe-output");
    for entry in &[
        "2026-08-17 \u{00B7} +250.00 XEC \u{00B7} vault handshake",
        "2026-08-16 \u{00B7} -40.00 USDC \u{00B7} tax payment",
        "2026-08-15 \u{00B7} +1,000 Q42 \u{00B7} semantic token mint",
    ] {
        let line = document.create_element("div").unwrap();
        line.set_class_name("vibe-out-line");
        line.set_text_content(Some(entry));
        ledger.append_child(&line).unwrap();
    }
    wrapper.append_child(&ledger).unwrap();

    // Actions
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &["Black-Scholes", "Tax Suite", "Send XEC", "Export"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Vision (ComputerVision.ahash, qualia-vision)
// ---------------------------------------------------------------------------

/// Vision container — ahash, detection, super-res.
pub fn build_vision_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    let notice = document.create_element("div").unwrap();
    notice.set_class_name("container-placeholder");
    notice.set_text_content(Some(
        "Vision workbench \u{2014} awaiting `qualia-vision` engine wiring. \
         `vision_run_synthetic_demo`, `vision_detect_image_file`, \
         `vision_super_resolve` reachable via IntentBus when native.",
    ));
    wrapper.append_child(&notice).unwrap();

    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &["Detect", "Classify", "Super-Resolve", "Weights"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Listen (qualia-audio, EnCodec)
// ---------------------------------------------------------------------------

/// Listen container — audio capture, AED, speech, sonify.
pub fn build_listen_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    let notice = document.create_element("div").unwrap();
    notice.set_class_name("container-placeholder");
    notice.set_text_content(Some(
        "Listen workbench \u{2014} awaiting `qualia-audio` engine wiring. \
         `audio_mic_start`, `audio_live_aed`, `audio_speech_demo`, \
         `audio_sonify_hear` reachable via IntentBus when native.",
    ));
    wrapper.append_child(&notice).unwrap();

    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &["Mic Start", "Mic Stop", "AED", "Speech", "Sonify"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Triad (q42+p64+d10, qualia-audio articulatory)
// ---------------------------------------------------------------------------

/// Triad container — q42+p64+d10 articulatory inspector.
pub fn build_triad_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    let notice = document.create_element("div").unwrap();
    notice.set_class_name("container-placeholder");
    notice.set_text_content(Some(
        "Triad (q42 + p64 + d10) \u{2014} articulatory inspector. \
         `qualia-audio` cross-modal demo, shared clock, mixer bounce. \
         Awaiting engine wiring.",
    ));
    wrapper.append_child(&notice).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Portal (QApp dispatch, wormhole IRI)
// ---------------------------------------------------------------------------

/// Portal container — QApp dispatch, wormhole IRI.
pub fn build_portal_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    let notice = document.create_element("div").unwrap();
    notice.set_class_name("container-placeholder");
    notice.set_text_content(Some(
        "Portal \u{2014} QApp dispatch and wormhole IRI. \
         `launch_installed_qapp`, `verify_and_install_qapp`, \
         `export_qapp_as_wasm_package`. Awaiting engine wiring.",
    ));
    wrapper.append_child(&notice).unwrap();

    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &["Launch QApp", "Install", "Export WASM", "Catalog"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Slide (office presentation)
// ---------------------------------------------------------------------------

/// Slide container — office presentation.
pub fn build_slide_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 4px;");

    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    for label in &["+ Slide", "Layout", "Transition", "Present"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        bar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&bar).unwrap();

    let slide_area = document.create_element("div").unwrap();
    let sa_el: HtmlElement = slide_area.clone().dyn_into().unwrap();
    sa_el.style().set_css_text(
        "flex: 1; background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); display: flex; align-items: center; \
         justify-content: center; color: var(--text-muted); font-size: 12px;",
    );
    slide_area.set_text_content(Some("Slide 1 \u{2014} click to add title"));
    wrapper.append_child(&slide_area).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// 3D (GPU viewport, mesh, 10D)
// ---------------------------------------------------------------------------

/// 3D container — GPU viewport, mesh, 10D asset loading.
pub fn build_3d_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    let notice = document.create_element("div").unwrap();
    notice.set_class_name("container-placeholder");
    notice.set_text_content(Some(
        "3D viewport \u{2014} awaiting GPU surface bridge. \
         `mount_gpu_surface`, `upload_gpu_mesh`, `upload_gpu_10d_mesh`, \
         `load_gpu_10d_asset` reachable via IntentBus when native.",
    ));
    wrapper.append_child(&notice).unwrap();

    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &["Object", "Modelling", "Materials", "Render"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Subcanvas (switch manifold, enter-zoom)
// ---------------------------------------------------------------------------

/// Subcanvas container — switch manifold, enter-zoom.
pub fn build_subcanvas_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    let notice = document.create_element("div").unwrap();
    notice.set_class_name("container-placeholder");
    notice.set_text_content(Some(
        "Subcanvas \u{2014} switch manifold or enter-zoom. \
         Double-click to enter; Esc to exit. \
         Awaiting manifold nesting implementation.",
    ));
    wrapper.append_child(&notice).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Multi-Modal Document Embedding Chips (Subsystem 1.7)
// ---------------------------------------------------------------------------

/// Interactive audio waveform chip with play/pause and spectrogram bars.
pub fn build_multimodal_chip_audio(document: &Document, title: &str, duration: &str) -> Element {
    let chip = document.create_element("span").unwrap();
    chip.set_class_name("cml-chip cml-chip-audio");
    let chip_el: HtmlElement = chip.clone().dyn_into().unwrap();
    chip_el.style().set_css_text(
        "display: inline-flex; align-items: center; gap: 6px; padding: 2px 8px; \
         background: rgba(139, 92, 246, 0.12); border: 1px solid rgba(139, 92, 246, 0.35); \
         border-radius: 12px; font-size: 11px; color: #a78bfa; margin: 0 4px; vertical-align: middle; \
         cursor: pointer; user-select: none;"
    );

    let play_icon = document.create_element("span").unwrap();
    play_icon.set_text_content(Some("\u{25B6}")); // ▶
    chip.append_child(&play_icon).unwrap();

    let label = document.create_element("span").unwrap();
    label.set_text_content(Some(title));
    let label_el: HtmlElement = label.clone().dyn_into().unwrap();
    label_el.style().set_css_text("font-weight: 500;");
    chip.append_child(&label).unwrap();

    // Mini SVG waveform indicator
    let waves = document.create_element("span").unwrap();
    waves.set_text_content(Some("\u{2582}\u{2585}\u{2588}\u{2586}\u{2583}\u{2587}\u{2584}"));
    let waves_el: HtmlElement = waves.clone().dyn_into().unwrap();
    waves_el.style().set_css_text("font-family: var(--font-mono); letter-spacing: -1px; opacity: 0.8;");
    chip.append_child(&waves).unwrap();

    let dur = document.create_element("span").unwrap();
    dur.set_text_content(Some(duration));
    let dur_el: HtmlElement = dur.clone().dyn_into().unwrap();
    dur_el.style().set_css_text("font-size: 9px; opacity: 0.6;");
    chip.append_child(&dur).unwrap();

    chip
}

/// 3D manifold / wireframe thumbnail chip.
pub fn build_multimodal_chip_3d(document: &Document, name: &str, poly_count: u32) -> Element {
    let chip = document.create_element("span").unwrap();
    chip.set_class_name("cml-chip cml-chip-3d");
    let chip_el: HtmlElement = chip.clone().dyn_into().unwrap();
    chip_el.style().set_css_text(
        "display: inline-flex; align-items: center; gap: 5px; padding: 2px 8px; \
         background: rgba(59, 130, 246, 0.12); border: 1px solid rgba(59, 130, 246, 0.35); \
         border-radius: 12px; font-size: 11px; color: #60a5fa; margin: 0 4px; vertical-align: middle; \
         cursor: pointer; user-select: none;"
    );

    let icon = document.create_element("span").unwrap();
    icon.set_text_content(Some("\u{25C7}")); // ◇
    chip.append_child(&icon).unwrap();

    let label = document.create_element("span").unwrap();
    label.set_text_content(Some(name));
    let label_el: HtmlElement = label.clone().dyn_into().unwrap();
    label_el.style().set_css_text("font-weight: 500;");
    chip.append_child(&label).unwrap();

    let count = document.create_element("span").unwrap();
    count.set_text_content(Some(&format!("{}p", poly_count)));
    let count_el: HtmlElement = count.clone().dyn_into().unwrap();
    count_el.style().set_css_text("font-size: 9px; opacity: 0.6;");
    chip.append_child(&count).unwrap();

    chip
}

/// Embedded mini-spreadsheet chip with live preview cell.
pub fn build_multimodal_chip_sheet(document: &Document, sheet_ref: &str, summary_val: &str) -> Element {
    let chip = document.create_element("span").unwrap();
    chip.set_class_name("cml-chip cml-chip-sheet");
    let chip_el: HtmlElement = chip.clone().dyn_into().unwrap();
    chip_el.style().set_css_text(
        "display: inline-flex; align-items: center; gap: 5px; padding: 2px 8px; \
         background: rgba(16, 185, 129, 0.12); border: 1px solid rgba(16, 185, 129, 0.35); \
         border-radius: 12px; font-size: 11px; color: #34d399; margin: 0 4px; vertical-align: middle; \
         cursor: pointer; user-select: none;"
    );

    let icon = document.create_element("span").unwrap();
    icon.set_text_content(Some("\u{229E}")); // ⊞
    chip.append_child(&icon).unwrap();

    let label = document.create_element("span").unwrap();
    label.set_text_content(Some(sheet_ref));
    let label_el: HtmlElement = label.clone().dyn_into().unwrap();
    label_el.style().set_css_text("font-weight: 500;");
    chip.append_child(&label).unwrap();

    let val = document.create_element("span").unwrap();
    val.set_text_content(Some(&format!("= {}", summary_val)));
    let val_el: HtmlElement = val.clone().dyn_into().unwrap();
    val_el.style().set_css_text("font-family: var(--font-mono); font-size: 10px; font-weight: 600;");
    chip.append_child(&val).unwrap();

    chip
}

/// Semantic provenance citation badge chip linking to W3C Verifiable Credentials / DIDs.
pub fn build_multimodal_chip_citation(document: &Document, did_short: &str, certainty_pct: u8) -> Element {
    let chip = document.create_element("span").unwrap();
    chip.set_class_name("cml-chip cml-chip-citation");
    let chip_el: HtmlElement = chip.clone().dyn_into().unwrap();
    chip_el.style().set_css_text(
        "display: inline-flex; align-items: center; gap: 4px; padding: 1px 6px; \
         background: rgba(245, 158, 11, 0.12); border: 1px solid rgba(245, 158, 11, 0.35); \
         border-radius: 8px; font-size: 10px; color: #fbbf24; margin: 0 3px; vertical-align: middle; \
         cursor: pointer; user-select: none;"
    );

    let icon = document.create_element("span").unwrap();
    icon.set_text_content(Some("\u{2713}")); // ✓
    chip.append_child(&icon).unwrap();

    let label = document.create_element("span").unwrap();
    label.set_text_content(Some(did_short));
    let label_el: HtmlElement = label.clone().dyn_into().unwrap();
    label_el.style().set_css_text("font-family: var(--font-mono);");
    chip.append_child(&label).unwrap();

    let cert = document.create_element("span").unwrap();
    cert.set_text_content(Some(&format!("{}%", certainty_pct)));
    let cert_el: HtmlElement = cert.clone().dyn_into().unwrap();
    cert_el.style().set_css_text("font-size: 9px; opacity: 0.75;");
    chip.append_child(&cert).unwrap();

    chip
}
