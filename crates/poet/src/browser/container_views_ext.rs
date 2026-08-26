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
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Header Stratum Selector
    let stratum_bar = document.create_element("div").unwrap();
    stratum_bar.set_class_name("vibe-toolbar");
    for (idx, stratum) in ["10D Visceral", "Neural Connectome", "Musculoskeletal", "Vascular"].iter().enumerate() {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        let btn_el: HtmlElement = btn.clone().dyn_into().unwrap();
        if idx == 0 {
            btn_el.style().set_css_text("background: var(--accent-rose); color: var(--text-inverse); font-weight: 700;");
        }
        btn.set_text_content(Some(stratum));
        stratum_bar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&stratum_bar).unwrap();

    // 10D Anatomical Stratum Grid
    let organ_grid = document.create_element("div").unwrap();
    let og_el: HtmlElement = organ_grid.clone().dyn_into().unwrap();
    og_el.style().set_css_text("display: grid; grid-template-columns: repeat(2, 1fr); gap: 6px;");

    let organs = [
        ("Heart \u{00B7} Cor", "FMA:7088", "98.4% Normal", "var(--accent-emerald)", "HRV: 68ms \u{00B7} EF: 62%"),
        ("Lungs \u{00B7} Pulmo", "FMA:7195", "99.1% Clear", "var(--accent-emerald)", "SpO2: 99% \u{00B7} FEV1: 3.8L"),
        ("Liver \u{00B7} Hepar", "FMA:7203", "97.5% Normal", "var(--accent-emerald)", "ALT: 22 U/L \u{00B7} AST: 20"),
        ("Brain \u{00B7} Encephalon", "FMA:50801", "99.8% Coherent", "var(--accent-violet)", "Gamma: 40Hz \u{00B7} Alpha: 10Hz"),
    ];

    for (name, fma, status, col, telemetry) in organs {
        let card = document.create_element("div").unwrap();
        card.set_class_name("cr-card");
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text("padding: 8px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); display: flex; flex-direction: column; gap: 3px; font-size: 10px;");

        let h_row = document.create_element("div").unwrap();
        let h_row_el: HtmlElement = h_row.clone().dyn_into().unwrap();
        h_row_el.style().set_css_text("display: flex; justify-content: space-between; align-items: center;");
        let t_el = document.create_element("span").unwrap();
        t_el.set_text_content(Some(name));
        t_el.set_attribute("style", "font-weight: 700; color: var(--text-primary);").unwrap();
        h_row.append_child(&t_el).unwrap();

        let s_el = document.create_element("span").unwrap();
        s_el.set_text_content(Some(status));
        s_el.set_attribute("style", &format!("font-size: 9px; color: {}; font-weight: 600;", col)).unwrap();
        h_row.append_child(&s_el).unwrap();
        card.append_child(&h_row).unwrap();

        let sub = document.create_element("div").unwrap();
        sub.set_attribute("style", "display: flex; justify-content: space-between; color: var(--text-muted); font-size: 9px;").unwrap();
        sub.set_inner_html(&format!("<span>{}</span><span>{}</span>", fma, telemetry));
        card.append_child(&sub).unwrap();

        organ_grid.append_child(&card).unwrap();
    }
    wrapper.append_child(&organ_grid).unwrap();

    // Comorbidity Scorecard
    let comorb = document.create_element("div").unwrap();
    comorb.set_class_name("cr-card");
    let comorb_el: HtmlElement = comorb.clone().dyn_into().unwrap();
    comorb_el.style().set_css_text("padding: 8px 10px; background: rgba(0,0,0,0.25); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); font-size: 10px; display: flex; justify-content: space-between; align-items: center;");
    comorb.set_inner_html(
        "<div><span style='color: var(--accent-cyan); font-weight: 700;'>Charlson Index:</span> <span style='color: var(--text-primary);'>0 (10-Yr Survival: 99%)</span></div>\
         <div style='color: var(--accent-emerald); font-weight: 600;'>\u{2713} Low Comorbidity</div>"
    );
    wrapper.append_child(&comorb).unwrap();

    // Action Toolbar
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &["Mount 10D Manifold", "Comorbidity Risk", "Extract SNOMED", "Export FHIR"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

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
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px; font-family: var(--font-mono); color: var(--text-primary);");

    // Browser Tabs
    let tab_strip = document.create_element("div").unwrap();
    let ts_el: HtmlElement = tab_strip.clone().dyn_into().unwrap();
    ts_el.style().set_css_text("display: flex; gap: 4px; align-items: center; border-bottom: 1px solid var(--border-subtle); padding-bottom: 4px; font-size: 10px;");

    let tab1 = document.create_element("span").unwrap();
    tab1.set_text_content(Some("\u{1F310} Qualia Network Explorer \u{00D7}"));
    tab1.set_attribute("style", "padding: 3px 8px; background: var(--surface-panel-elevated); border: 1px solid var(--accent-cyan); border-radius: 4px; color: var(--text-primary); font-weight: 600;").unwrap();
    tab_strip.append_child(&tab1).unwrap();

    let tab_add = document.create_element("button").unwrap();
    tab_add.set_class_name("vibe-run-btn");
    tab_add.set_text_content(Some("+"));
    tab_strip.append_child(&tab_add).unwrap();
    wrapper.append_child(&tab_strip).unwrap();

    // URL & Navigation bar
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    let nav_back = document.create_element("button").unwrap();
    nav_back.set_class_name("vibe-run-btn");
    nav_back.set_text_content(Some("\u{25C0}"));
    bar.append_child(&nav_back).unwrap();

    let nav_fwd = document.create_element("button").unwrap();
    nav_fwd.set_class_name("vibe-run-btn");
    nav_fwd.set_text_content(Some("\u{25B6}"));
    bar.append_child(&nav_fwd).unwrap();

    let lock_icon = document.create_element("span").unwrap();
    lock_icon.set_text_content(Some("\u{1F512}"));
    lock_icon.set_attribute("style", "font-size: 11px; margin-left: 2px; color: var(--accent-emerald);").unwrap();
    bar.append_child(&lock_icon).unwrap();

    let input = document.create_element("input").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.set_value("https://qualia.network/explorer/habitat");
    input.set_attribute("style", "flex: 1; background: var(--canvas-bg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 4px 8px; color: var(--text-primary); font-size: 11px; font-family: var(--font-mono); outline: none;").unwrap();
    bar.append_child(&input).unwrap();

    let go_btn = document.create_element("button").unwrap();
    go_btn.set_class_name("vibe-run-btn");
    go_btn.set_text_content(Some("\u{21BB}"));
    bar.append_child(&go_btn).unwrap();
    wrapper.append_child(&bar).unwrap();

    // Rendered Viewport Sandbox Frame
    let viewport = document.create_element("div").unwrap();
    let vp_el: HtmlElement = viewport.clone().dyn_into().unwrap();
    vp_el.style().set_css_text("flex: 1; background: rgba(0,0,0,0.4); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 12px; display: flex; flex-direction: column; gap: 8px; font-size: 11px; overflow-y: auto;");

    let cert_badge = document.create_element("div").unwrap();
    cert_badge.set_attribute("style", "display: flex; align-items: center; justify-content: space-between; padding: 4px 8px; background: rgba(0, 242, 169, 0.08); border: 1px solid rgba(0, 242, 169, 0.25); border-radius: 4px; font-size: 9px; color: var(--accent-emerald);").unwrap();
    cert_badge.set_inner_html("<span>\u{2713} Origin Trust: Level 4 (Verifiable DID Signed)</span><span>TLS 1.3 \u{00B7} Sandbox Strict</span>");
    viewport.append_child(&cert_badge).unwrap();

    let page_content = document.create_element("div").unwrap();
    page_content.set_inner_html(
        "<h3 style='margin: 0 0 6px 0; color: var(--accent-cyan); font-size: 13px;'>Qualia Network Habitat Explorer</h3>\
         <p style='margin: 0 0 8px 0; color: var(--text-secondary); line-height: 1.4;'>\
         Connected to local cluster gateway at <code>http://127.0.0.1:4242</code>. \
         All hypermedia documents are cryptographically resolved through zero-copy Super-Quins.\
         </p>\
         <div style='background: var(--surface-panel); padding: 8px; border-radius: 4px; border: 1px solid var(--border-subtle);'>\
         <strong>Active Habitat Node:</strong> did:qualia:timothy_charles_holborn<br/>\
         <strong>Routing Lane:</strong> Bilateral Micro-Commons (48-byte Packed)\
         </div>"
    );
    viewport.append_child(&page_content).unwrap();
    wrapper.append_child(&viewport).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// WebRTC (P2P mesh, DataChannel sync)
// ---------------------------------------------------------------------------

/// WebRTC container — P2P DataChannel mesh & Super-Quin synchronization.
pub fn build_webrtc_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Header Status Card
    let status_card = document.create_element("div").unwrap();
    status_card.set_class_name("cr-card");
    let sc_el: HtmlElement = status_card.clone().dyn_into().unwrap();
    sc_el.style().set_css_text("padding: 8px 10px; background: rgba(0, 210, 255, 0.08); border: 1px solid var(--accent-cyan); border-radius: var(--radius-xs); display: flex; justify-content: space-between; align-items: center; font-size: 10px;");
    status_card.set_inner_html(
        "<div><span style='color: var(--accent-cyan); font-weight: 700;'>P2P Mesh:</span> <span style='color: var(--accent-emerald); font-weight: 600;'>\u{25CF} Connected (3 Peers)</span></div>\
         <div style='color: var(--text-muted);'>ICE: Direct Host-to-Host</div>"
    );
    wrapper.append_child(&status_card).unwrap();

    // Active Swarm Peer List
    let peer_list = document.create_element("div").unwrap();
    let pl_el: HtmlElement = peer_list.clone().dyn_into().unwrap();
    pl_el.style().set_css_text("display: flex; flex-direction: column; gap: 4px;");

    let peers = [
        ("did:qualia:edge:node-7f2a", "14ms", "182 KB/s", "CRDT Sync: Active"),
        ("did:qualia:edge:node-3b91", "28ms", "94 KB/s", "CRDT Sync: Active"),
        ("did:qualia:edge:node-c044", "19ms", "220 KB/s", "CRDT Sync: Active"),
    ];

    for (peer_did, latency, throughput, sync_mode) in peers {
        let row = document.create_element("div").unwrap();
        row.set_class_name("vibe-output");
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text("padding: 6px 8px; display: flex; justify-content: space-between; align-items: center; font-size: 9px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs);");

        let left = document.create_element("div").unwrap();
        left.set_attribute("style", "display: flex; align-items: center; gap: 6px;").unwrap();
        left.set_inner_html(&format!("<span style='color: var(--accent-amber);'>\u{29BF}</span><strong>{}</strong>", peer_did));
        row.append_child(&left).unwrap();

        let right = document.create_element("div").unwrap();
        right.set_attribute("style", "display: flex; gap: 10px; color: var(--text-muted);").unwrap();
        right.set_inner_html(&format!("<span>{}</span><span>{}</span><span style='color: var(--accent-emerald);'>{}</span>", latency, throughput, sync_mode));
        row.append_child(&right).unwrap();

        peer_list.append_child(&row).unwrap();
    }
    wrapper.append_child(&peer_list).unwrap();

    // Action Toolbar
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &["Broadcast Super-Quin", "Ping Swarm", "ICE Renegotiate", "Inspect SDP"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

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
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Portfolio summary
    let summary = document.create_element("div").unwrap();
    summary.set_class_name("cr-card");
    let s_el: HtmlElement = summary.clone().dyn_into().unwrap();
    s_el.style().set_css_text("padding: 8px 10px; background: rgba(0, 210, 255, 0.08); border: 1px solid var(--accent-cyan); border-radius: var(--radius-xs); display: flex; justify-content: space-between; align-items: center; font-size: 10px;");
    summary.set_inner_html(
        "<div><span style='color: var(--accent-cyan); font-weight: 700;'>Portfolio Total:</span> $4,250.00 USD</div>\
         <div style='color: var(--accent-emerald); font-weight: 600;'>+8.4% (24h)</div>"
    );
    wrapper.append_child(&summary).unwrap();

    // Asset balances grid
    let asset_grid = document.create_element("div").unwrap();
    let ag_el: HtmlElement = asset_grid.clone().dyn_into().unwrap();
    ag_el.style().set_css_text("display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px;");

    let assets = [
        ("XEC Vault", "1,250 XEC", "$2,100.00", "var(--accent-amber)"),
        ("USDC Collateral", "340 USDC", "$340.00", "var(--accent-cyan)"),
        ("Q42 Commons", "8,000 Q42", "$1,810.00", "var(--accent-violet)"),
    ];

    for (name, bal, val, col) in assets {
        let card = document.create_element("div").unwrap();
        card.set_class_name("cr-card");
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text("padding: 6px 8px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); font-size: 9px; display: flex; flex-direction: column; gap: 2px;");

        let n_el = document.create_element("span").unwrap();
        n_el.set_text_content(Some(name));
        n_el.set_attribute("style", &format!("font-weight: 700; color: {};", col)).unwrap();
        card.append_child(&n_el).unwrap();

        let b_el = document.create_element("span").unwrap();
        b_el.set_text_content(Some(bal));
        b_el.set_attribute("style", "color: var(--text-primary); font-weight: 600;").unwrap();
        card.append_child(&b_el).unwrap();

        let v_el = document.create_element("span").unwrap();
        v_el.set_text_content(Some(val));
        v_el.set_attribute("style", "color: var(--text-muted); font-size: 8px;").unwrap();
        card.append_child(&v_el).unwrap();

        asset_grid.append_child(&card).unwrap();
    }
    wrapper.append_child(&asset_grid).unwrap();

    // Ledger
    let ledger = document.create_element("div").unwrap();
    ledger.set_class_name("vibe-output");
    let l_el: HtmlElement = ledger.clone().dyn_into().unwrap();
    l_el.style().set_css_text("display: flex; flex-direction: column; gap: 3px; font-size: 9px;");
    for entry in &[
        "2026-08-17 \u{00B7} +250.00 XEC \u{00B7} vault handshake (verified)",
        "2026-08-16 \u{00B7} -40.00 USDC \u{00B7} zero-knowledge tax batch",
        "2026-08-15 \u{00B7} +1,000 Q42 \u{00B7} semantic token minting",
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
    for label in &["Black-Scholes", "Tax Suite", "Send XEC", "Export Ledger"] {
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
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Header Toolbar
    let toolbar = document.create_element("div").unwrap();
    toolbar.set_class_name("vibe-toolbar");
    for label in &["Capture Frame", "Detect", "Super-Resolve (2x)", "ahash Compute"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        toolbar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&toolbar).unwrap();

    // Visual Detection Canvas Frame
    let canvas_frame = document.create_element("div").unwrap();
    let cf_el: HtmlElement = canvas_frame.clone().dyn_into().unwrap();
    cf_el.style().set_css_text("flex: 1; background: rgba(0,0,0,0.5); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); position: relative; display: flex; align-items: center; justify-content: center; min-height: 120px; overflow: hidden;");

    // Render an interactive SVG representing live vision detection
    let svg = document.create_element_ns(Some("http://www.w3.org/2000/svg"), "svg").unwrap();
    svg.set_attribute("width", "100%").unwrap();
    svg.set_attribute("height", "100%").unwrap();
    svg.set_attribute("viewBox", "0 0 320 120").unwrap();

    svg.set_inner_html(
        "<rect x='20' y='15' width='110' height='90' fill='none' stroke='#00d2ff' stroke-width='1.5' stroke-dasharray='4,2'/>\
         <text x='25' y='30' fill='#00d2ff' font-size='9' font-family='monospace'>Node Principal [0.99]</text>\
         <rect x='160' y='25' width='140' height='75' fill='none' stroke='#00f2a9' stroke-width='1.5'/>\
         <text x='165' y='40' fill='#00f2a9' font-size='9' font-family='monospace'>Display Surface [0.96]</text>\
         <circle cx='75' cy='60' r='18' fill='none' stroke='#a855f7' stroke-width='1'/>"
    );
    canvas_frame.append_child(&svg).unwrap();
    wrapper.append_child(&canvas_frame).unwrap();

    // Telemetry & aHash Perceptual Status
    let meta_card = document.create_element("div").unwrap();
    meta_card.set_class_name("cr-card");
    let mc_el: HtmlElement = meta_card.clone().dyn_into().unwrap();
    mc_el.style().set_css_text("padding: 6px 10px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); font-size: 9px; display: flex; justify-content: space-between;");
    meta_card.set_inner_html(
        "<div><span style='color: var(--accent-amber); font-weight: 700;'>ahash:</span> <code>0xa4f8_910b_e2d3_441c</code></div>\
         <div style='color: var(--text-muted);'>Resolution: 1920x1080 @ 60 FPS</div>"
    );
    wrapper.append_child(&meta_card).unwrap();

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
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Header Toolbar
    let toolbar = document.create_element("div").unwrap();
    toolbar.set_class_name("vibe-toolbar");
    for label in &["Mic (Live)", "AED Spectrum", "EnCodec Tokenize", "Formant Filter"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        toolbar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&toolbar).unwrap();

    // Animated Frequency Spectrum Display
    let spectrum_box = document.create_element("div").unwrap();
    let sb_el: HtmlElement = spectrum_box.clone().dyn_into().unwrap();
    sb_el.style().set_css_text("flex: 1; background: rgba(0,0,0,0.5); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 10px; display: flex; align-items: flex-end; justify-content: space-around; min-height: 80px; gap: 3px;");

    let bar_heights = [30, 45, 70, 85, 95, 60, 40, 75, 90, 65, 50, 35, 80, 60, 40, 20];
    for (_idx, h) in bar_heights.iter().enumerate() {
        let bar = document.create_element("div").unwrap();
        let b_el: HtmlElement = bar.clone().dyn_into().unwrap();
        let color = if *h > 80 {
            "var(--accent-rose)"
        } else if *h > 50 {
            "var(--accent-amber)"
        } else {
            "var(--accent-cyan)"
        };
        b_el.style().set_css_text(&format!(
            "flex: 1; height: {}%; background: {}; border-radius: 2px; transition: height 0.15s ease;",
            h, color
        ));
        spectrum_box.append_child(&bar).unwrap();
    }
    wrapper.append_child(&spectrum_box).unwrap();

    // Acoustic Event Detection (AED) Classification Feed
    let aed_card = document.create_element("div").unwrap();
    aed_card.set_class_name("cr-card");
    let ac_el: HtmlElement = aed_card.clone().dyn_into().unwrap();
    ac_el.style().set_css_text("padding: 6px 10px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); font-size: 9px; display: flex; justify-content: space-between; align-items: center;");
    aed_card.set_inner_html(
        "<div><span style='color: var(--accent-emerald); font-weight: 700;'>AED:</span> Speech Formants Active (99.4%)</div>\
         <div style='color: var(--text-muted);'>RMS: -14.2 dB \u{00B7} F0: 124 Hz</div>"
    );
    wrapper.append_child(&aed_card).unwrap();

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
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Triad Cores Grid
    let grid = document.create_element("div").unwrap();
    let g_el: HtmlElement = grid.clone().dyn_into().unwrap();
    g_el.style().set_css_text("display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px;");

    let cores = [
        ("Core 0: Reasoning", "42MB Sentinel", "12% Load", "4.2 MB / 42 MB", "var(--accent-cyan)"),
        ("Core 1: QTensor", "DirectML / GGUF", "48% Load", "1.2 GB VRAM", "var(--accent-amber)"),
        ("Core 2: Volumetric", "WebGPU 60 FPS", "24% Load", "16.6ms Render", "var(--accent-emerald)"),
    ];

    for (name, role, load, mem, col) in cores {
        let card = document.create_element("div").unwrap();
        card.set_class_name("cr-card");
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text("padding: 8px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); display: flex; flex-direction: column; gap: 3px; font-size: 9px;");

        let h = document.create_element("div").unwrap();
        h.set_attribute("style", &format!("font-weight: 700; color: {};", col)).unwrap();
        h.set_text_content(Some(name));
        card.append_child(&h).unwrap();

        let r = document.create_element("div").unwrap();
        r.set_attribute("style", "color: var(--text-secondary); font-size: 9px;").unwrap();
        r.set_text_content(Some(role));
        card.append_child(&r).unwrap();

        let m = document.create_element("div").unwrap();
        m.set_attribute("style", "display: flex; justify-content: space-between; color: var(--text-muted); font-size: 8px; margin-top: 4px;").unwrap();
        m.set_inner_html(&format!("<span>{}</span><span>{}</span>", load, mem));
        card.append_child(&m).unwrap();

        grid.append_child(&card).unwrap();
    }
    wrapper.append_child(&grid).unwrap();

    // Thermal & Governor Status
    let gov_card = document.create_element("div").unwrap();
    gov_card.set_class_name("cr-card");
    let gc_el: HtmlElement = gov_card.clone().dyn_into().unwrap();
    gc_el.style().set_css_text("padding: 6px 10px; background: rgba(0,0,0,0.3); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); font-size: 9px; display: flex; justify-content: space-between; align-items: center;");
    gov_card.set_inner_html(
        "<div><span style='color: var(--accent-emerald); font-weight: 700;'>Governor:</span> Cool (46\u{00B0}C) \u{00B7} Zero Throttling</div>\
         <div style='color: var(--text-muted);'>Power Budget: 25W (Balanced)</div>"
    );
    wrapper.append_child(&gov_card).unwrap();

    // Action Toolbar
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &["Benchmark Triad", "Reset Arena", "Governor Mode", "Export Telemetry"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

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
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Target Manifold Resolver Bar
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    let input = document.create_element("input").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.set_value("did:qualia:manifold:spatial-neuro-catchment#v1");
    input.set_attribute("style", "flex: 1; background: var(--canvas-bg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 4px 8px; color: var(--text-primary); font-size: 11px; font-family: var(--font-mono); outline: none;").unwrap();
    bar.append_child(&input).unwrap();

    let teleport_btn = document.create_element("button").unwrap();
    teleport_btn.set_class_name("vibe-run-btn");
    teleport_btn.set_text_content(Some("\u{2728} Teleport"));
    bar.append_child(&teleport_btn).unwrap();
    wrapper.append_child(&bar).unwrap();

    // Portal Destination Matrix
    let port_list = document.create_element("div").unwrap();
    let pl_el: HtmlElement = port_list.clone().dyn_into().unwrap();
    pl_el.style().set_css_text("display: flex; flex-direction: column; gap: 4px;");

    let portals = [
        ("Catchment Studio", "Local Workspace", "SHA-256 Verified", "var(--accent-emerald)"),
        ("Neuro-Anatomy 10D", "P2P Swarm Relay", "Signed DID Token", "var(--accent-cyan)"),
        ("Quantum Chemistry Lab", "Federated Cluster", "Integrity Checked", "var(--accent-violet)"),
    ];

    for (name, domain, auth, col) in portals {
        let row = document.create_element("div").unwrap();
        row.set_class_name("vibe-output");
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text("padding: 6px 8px; display: flex; justify-content: space-between; align-items: center; font-size: 9px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); cursor: pointer;");

        let left = document.create_element("div").unwrap();
        left.set_inner_html(&format!("<strong style='color: {};'>{}</strong> \u{00B7} <span style='color: var(--text-muted);'>{}</span>", col, name, domain));
        row.append_child(&left).unwrap();

        let right = document.create_element("div").unwrap();
        right.set_attribute("style", "color: var(--accent-emerald); font-size: 8px;").unwrap();
        right.set_text_content(Some(auth));
        row.append_child(&right).unwrap();

        port_list.append_child(&row).unwrap();
    }
    wrapper.append_child(&port_list).unwrap();

    // Actions
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &["Verify Wormhole", "Sandbox Config", "Export Link"] {
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
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Header Toolbar
    let toolbar = document.create_element("div").unwrap();
    toolbar.set_class_name("vibe-toolbar");
    for label in &["Orbit Camera", "Wireframe", "WGSL Shading", "Subdivide", "Export Mesh"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        toolbar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&toolbar).unwrap();

    // Interactive 3D SVG Projection Viewport
    let viewport = document.create_element("div").unwrap();
    let vp_el: HtmlElement = viewport.clone().dyn_into().unwrap();
    vp_el.style().set_css_text("flex: 1; background: rgba(0,0,0,0.55); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); position: relative; display: flex; align-items: center; justify-content: center; min-height: 120px; overflow: hidden;");

    let svg = document.create_element_ns(Some("http://www.w3.org/2000/svg"), "svg").unwrap();
    svg.set_attribute("width", "100%").unwrap();
    svg.set_attribute("height", "100%").unwrap();
    svg.set_attribute("viewBox", "0 0 320 120").unwrap();

    svg.set_inner_html(
        "<polygon points='160,20 220,50 160,100 100,50' fill='rgba(0, 210, 255, 0.15)' stroke='#00d2ff' stroke-width='1.5'/>\
         <polygon points='160,20 220,50 200,90 160,100' fill='rgba(168, 85, 247, 0.2)' stroke='#a855f7' stroke-width='1.5'/>\
         <line x1='160' y1='20' x2='160' y2='100' stroke='#38bdf8' stroke-width='1' stroke-dasharray='2,2'/>\
         <circle cx='160' cy='20' r='3.5' fill='#00f2a9'/>\
         <circle cx='220' cy='50' r='3.5' fill='#00f2a9'/>\
         <circle cx='160' cy='100' r='3.5' fill='#00f2a9'/>\
         <circle cx='100' cy='50' r='3.5' fill='#00f2a9'/>\
         <text x='15' y='25' fill='#94a3b8' font-size='9' font-family='monospace'>Vertices: 1,024 \u{00B7} Faces: 2,048</text>\
         <text x='15' y='110' fill='#00f2a9' font-size='9' font-family='monospace'>Pitch: 22\u{00B0} \u{00B7} Yaw: 45\u{00B7} FOV: 60\u{00B0}</text>"
    );
    viewport.append_child(&svg).unwrap();
    wrapper.append_child(&viewport).unwrap();

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
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Breadcrumb Navigation Header
    let breadcrumb = document.create_element("div").unwrap();
    let bc_el: HtmlElement = breadcrumb.clone().dyn_into().unwrap();
    bc_el.style().set_css_text("padding: 4px 8px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); font-size: 10px; color: var(--accent-cyan); display: flex; align-items: center; gap: 6px;");
    breadcrumb.set_inner_html(
        "<span>Workspace</span> <span>\u{203A}</span> <span>Manifold Alpha</span> <span>\u{203A}</span> <strong style='color: var(--text-primary);'>Nested Subcanvas 1</strong>"
    );
    wrapper.append_child(&breadcrumb).unwrap();

    // Nested Subcanvas Viewport Preview
    let preview = document.create_element("div").unwrap();
    let p_el: HtmlElement = preview.clone().dyn_into().unwrap();
    p_el.style().set_css_text("flex: 1; background: rgba(0,0,0,0.4); border: 1px dashed var(--accent-cyan); border-radius: var(--radius-xs); display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; min-height: 100px;");

    let icon = document.create_element("div").unwrap();
    icon.set_text_content(Some("\u{1F50D}"));
    icon.set_attribute("style", "font-size: 24px;").unwrap();
    preview.append_child(&icon).unwrap();

    let label = document.create_element("div").unwrap();
    label.set_attribute("style", "font-size: 11px; color: var(--text-secondary); text-align: center;").unwrap();
    label.set_text_content(Some("Subcanvas Isolation Sandbox \u{00B7} LOD Depth: 2"));
    preview.append_child(&label).unwrap();
    wrapper.append_child(&preview).unwrap();

    // Action Toolbar
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &["Enter Subcanvas (Zoom)", "Pop to Parent", "Clone Subtree", "Merge to Root"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

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
