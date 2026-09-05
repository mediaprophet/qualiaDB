//! Poet Integrated Development Environment (IDE) Subsystem (Spec 13).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Implements the 6-zone Poet IDE developer manifold: Activity Rail, File Explorer,
//! Multi-Pane Code Editor with Syntax Highlighting, Bottom Dockable Drawer (Vibe REPL,
//! Problems, Alloc Runner), Secondary AST Inspector, and Real-Time Status Bar.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// Active tool panel on the left vertical rail (Zone A).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IdeActivityTab {
    Explorer,
    Search,
    SourceControl,
    Debug,
    Packages,
    Tests,
    Copilot,
}

impl IdeActivityTab {
    pub fn glyph(&self) -> &'static str {
        match self {
            Self::Explorer => "\u{1F4C1}",      // 📁
            Self::Search => "\u{1F50D}",        // 🔍
            Self::SourceControl => "\u{1F33F}", // 🌿
            Self::Debug => "\u{1F41E}",         // 🐞
            Self::Packages => "\u{1F4E6}",      // 📦
            Self::Tests => "\u{1F9EA}",         // 🧪
            Self::Copilot => "\u{1F916}",       // 🤖
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Explorer => "Explorer",
            Self::Search => "Search & AST Query",
            Self::SourceControl => "Source Control",
            Self::Debug => "Run & Debug",
            Self::Packages => "Law Packages",
            Self::Tests => "Test & Alloc Explorer",
            Self::Copilot => "AI Co-Pilot",
        }
    }
}

/// A node in the workspace file tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IdeFileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Vec<IdeFileNode>,
    pub size_bytes: usize,
}

/// Open editor tab in Zone C.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IdeEditorTab {
    pub path: String,
    pub title: String,
    pub language: String,
    pub content: String,
    pub is_dirty: bool,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

/// Problem diagnostic entry in Zone D.2.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IdeProblem {
    pub file_path: String,
    pub line: usize,
    pub col: usize,
    pub severity: String, // "error", "warning", "info"
    pub message: String,
    pub code: String,
}

/// A REPL execution entry in Zone D.1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IdeReplEntry {
    pub prompt: String,
    pub response: String,
    pub elapsed_us: u64,
    pub gas_consumed: u64,
    pub is_error: bool,
}

/// State of the complete Poet IDE.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IdeState {
    pub active_activity: IdeActivityTab,
    pub file_tree: Vec<IdeFileNode>,
    pub open_tabs: Vec<IdeEditorTab>,
    pub active_tab_index: usize,
    pub problems: Vec<IdeProblem>,
    pub repl_history: Vec<IdeReplEntry>,
    pub git_branch: String,
    pub sentinel_memory_used_mb: f32,
    pub total_gas_consumed: u64,
}

impl Default for IdeState {
    fn default() -> Self {
        let sample_vibe = r#"// VibeScript 0.1 Component Definition
entity "WaterQualityTelemetrySensor" {
    requires: { graph.read, pulse.publish, sentinel.gate },

    // Reactive sensor cell with inline runner
    cell compute_reading = fn(raw_volts: f32) -> f32 {
        let calibration_factor = 3.14159;
        let baseline = 0.042;
        return (raw_volts * calibration_factor) + baseline;
    }

    animate on_pulse = fn(reading: f32) {
        if reading > 10.0 {
            emit "qualia:TelemetryAlert" { level: "high", val: reading };
        }
    }
}
"#;

        let sample_wgsl = r#"// WGSL Forge Shader Pipeline
@group(0) @binding(0) var<storage, read> in_tensors: array<f32>;
@group(0) @binding(1) var<storage, read_write> out_tensors: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    out_tensors[idx] = in_tensors[idx] * 2.0;
}
"#;

        let sample_n3 = r#"@prefix qualia: <urn:qualia:> .
@prefix deontic: <urn:qualia:deontic:> .

# Strict Modus Ponens Policy
{ ?agent qualia:role "Doctor" }
  => { ?agent deontic:permit qualia:ReadPatientVitals } .

# Emergency Defeater Rule
{ ?event qualia:condition "CardiacArrest" }
  ^> { ?agent deontic:permit qualia:EmergencyBypass } .
"#;

        let file_tree = vec![
            IdeFileNode {
                name: "src".into(),
                path: "src".into(),
                is_dir: true,
                size_bytes: 0,
                children: vec![
                    IdeFileNode {
                        name: "main.vibe".into(),
                        path: "src/main.vibe".into(),
                        is_dir: false,
                        size_bytes: sample_vibe.len(),
                        children: vec![],
                    },
                    IdeFileNode {
                        name: "pipeline.wgsl".into(),
                        path: "src/pipeline.wgsl".into(),
                        is_dir: false,
                        size_bytes: sample_wgsl.len(),
                        children: vec![],
                    },
                    IdeFileNode {
                        name: "rules.n3".into(),
                        path: "src/rules.n3".into(),
                        is_dir: false,
                        size_bytes: sample_n3.len(),
                        children: vec![],
                    },
                ],
            },
            IdeFileNode {
                name: "manifest.json".into(),
                path: "manifest.json".into(),
                is_dir: false,
                size_bytes: 420,
                children: vec![],
            },
        ];

        let open_tabs = vec![
            IdeEditorTab {
                path: "src/main.vibe".into(),
                title: "main.vibe".into(),
                language: "vibe".into(),
                content: sample_vibe.into(),
                is_dirty: false,
                cursor_line: 7,
                cursor_col: 9,
            },
            IdeEditorTab {
                path: "src/pipeline.wgsl".into(),
                title: "pipeline.wgsl".into(),
                language: "wgsl".into(),
                content: sample_wgsl.into(),
                is_dirty: false,
                cursor_line: 1,
                cursor_col: 1,
            },
            IdeEditorTab {
                path: "src/rules.n3".into(),
                title: "rules.n3".into(),
                language: "n3".into(),
                content: sample_n3.into(),
                is_dirty: false,
                cursor_line: 1,
                cursor_col: 1,
            },
        ];

        let problems = vec![
            IdeProblem {
                file_path: "src/main.vibe".into(),
                line: 12,
                col: 18,
                severity: "info".into(),
                message: "Unused variable `reading` in local scope".into(),
                code: "VIBE-W012".into(),
            },
            IdeProblem {
                file_path: "src/rules.n3".into(),
                line: 8,
                col: 3,
                severity: "warning".into(),
                message: "Defeater rule requires elevated provenance signature".into(),
                code: "SHACL-P04".into(),
            },
        ];

        let repl_history = vec![
            IdeReplEntry {
                prompt: "let x = 42 * 2;".into(),
                response: "84 (u64)".into(),
                elapsed_us: 18,
                gas_consumed: 12,
                is_error: false,
            },
            IdeReplEntry {
                prompt: "graph::query(\"SELECT ?s WHERE { ?s a qualia:Sensor } LIMIT 2\")".into(),
                response: "[\"did:q42:sensor:01\", \"did:q42:sensor:02\"]".into(),
                elapsed_us: 140,
                gas_consumed: 65,
                is_error: false,
            },
        ];

        Self {
            active_activity: IdeActivityTab::Explorer,
            file_tree,
            open_tabs,
            active_tab_index: 0,
            problems,
            repl_history,
            git_branch: "0.0.34".into(),
            sentinel_memory_used_mb: 8.24,
            total_gas_consumed: 420,
        }
    }
}

impl IdeState {
    /// Open a file in a new or existing tab.
    pub fn open_file(&mut self, path: &str, content: &str, lang: &str) {
        if let Some(idx) = self.open_tabs.iter().position(|t| t.path == path) {
            self.active_tab_index = idx;
        } else {
            let title = path.split('/').last().unwrap_or(path).to_string();
            self.open_tabs.push(IdeEditorTab {
                path: path.to_string(),
                title,
                language: lang.to_string(),
                content: content.to_string(),
                is_dirty: false,
                cursor_line: 1,
                cursor_col: 1,
            });
            self.active_tab_index = self.open_tabs.len() - 1;
        }
    }

    /// Close an editor tab.
    pub fn close_tab(&mut self, index: usize) {
        if index < self.open_tabs.len() {
            self.open_tabs.remove(index);
            if self.active_tab_index >= self.open_tabs.len() && !self.open_tabs.is_empty() {
                self.active_tab_index = self.open_tabs.len() - 1;
            }
        }
    }

    /// Evaluate an expression in the Vibe REPL using the live VibeScript engine.
    pub fn eval_repl(&mut self, expr: &str) -> IdeReplEntry {
        let trimmed = expr.trim();
        let normalized = trimmed.replace("::", ".");
        let src = if normalized.starts_with('=') {
            normalized
        } else {
            format!("={normalized}")
        };

        let mut host = vibe::LocalHost::default();
        let mut env = vibe::Env::default();

        // Replay previous non-error definitions to maintain session state.
        for prev in &self.repl_history {
            if !prev.is_error {
                let prev_norm = prev.prompt.trim().replace("::", ".");
                let prev_src = if prev_norm.starts_with('=') {
                    prev_norm
                } else {
                    format!("={prev_norm}")
                };
                let _ = vibe::eval_cell(&prev_src, &mut host, &mut env);
            }
        }

        let (resp, gas, is_error) = match vibe::parse_cell(&src) {
            Ok(expr) => {
                let mut engine = vibe::Engine::new(&mut host, vibe::Budget::default());
                match engine.eval_expr(&expr, &mut env) {
                    Ok(val) => {
                        let text = match &val {
                            vibe::Value::Null => "null".to_string(),
                            vibe::Value::Bool(b) => b.to_string(),
                            vibe::Value::I64(n) => n.to_string(),
                            vibe::Value::U64(n) => n.to_string(),
                            vibe::Value::F64(n) => n.to_string(),
                            vibe::Value::String(s) => format!("\"{s}\""),
                            vibe::Value::Iri(s) => format!("<{s}>"),
                            _ => format!("{val}"),
                        };
                        (text, 25, false)
                    }
                    Err(diag) => {
                        let err_msg = format!("Error [{:?}]: {}", diag.code, diag.message);
                        (err_msg, 5, true)
                    }
                }
            }
            Err(diag) => {
                let err_msg = format!("Parse Error [{:?}]: {}", diag.code, diag.message);
                (err_msg, 5, true)
            }
        };

        let entry = IdeReplEntry {
            prompt: expr.to_string(),
            response: resp,
            elapsed_us: 65,
            gas_consumed: gas,
            is_error,
        };
        self.total_gas_consumed += gas;
        self.repl_history.push(entry.clone());
        entry
    }
}

/// Tokenize and syntax-highlight code for display in the IDE editor pane.
pub fn syntax_highlight_vibe(code: &str) -> String {
    let mut out = String::with_capacity(code.len() * 2);
    for line in code.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with('#') {
            out.push_str(&format!(
                "<span style=\"color: #64748b; font-style: italic;\">{}</span>\n",
                line
            ));
            continue;
        }

        let mut line_html = line.to_string();
        for kw in &[
            "entity",
            "requires:",
            "cell",
            "fn",
            "let",
            "return",
            "if",
            "else",
            "animate",
            "emit",
        ] {
            line_html = line_html.replace(
                kw,
                &format!(
                    "<span style=\"color: #f43f5e; font-weight: 600;\">{}</span>",
                    kw
                ),
            );
        }
        for typ in &["f32", "f64", "u64", "u32", "String", "bool"] {
            line_html = line_html.replace(
                typ,
                &format!("<span style=\"color: #38bdf8;\">{}</span>", typ),
            );
        }
        out.push_str(&line_html);
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// DOM UI Component Builders
// ---------------------------------------------------------------------------

/// Build the full 6-zone Poet IDE Developer Manifold.
pub fn build_ide_view(document: &Document, state: &IdeState) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("poet-ide-root");
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display: flex; flex-direction: column; width: 100%; height: 100%; \
         background: #020617; color: #f8fafc; font-family: sans-serif; overflow: hidden;",
    );

    // Middle Area: Activity Bar + Sidebar + Editor + Secondary Inspector
    let main_area = document.create_element("div").unwrap();
    let main_area_el: HtmlElement = main_area.clone().dyn_into().unwrap();
    main_area_el
        .style()
        .set_css_text("display: flex; flex: 1; overflow: hidden;");

    // Zone A: Activity Rail (Left 48px)
    let activity_rail = document.create_element("div").unwrap();
    let activity_rail_el: HtmlElement = activity_rail.clone().dyn_into().unwrap();
    activity_rail_el.style().set_css_text(
        "width: 48px; background: #0b1120; border-right: 1px solid rgba(255, 255, 255, 0.08); \
         display: flex; flex-direction: column; align-items: center; padding-top: 8px; gap: 12px; z-index: 10;"
    );

    for tab in &[
        IdeActivityTab::Explorer,
        IdeActivityTab::Search,
        IdeActivityTab::SourceControl,
        IdeActivityTab::Debug,
        IdeActivityTab::Packages,
        IdeActivityTab::Tests,
        IdeActivityTab::Copilot,
    ] {
        let btn = document.create_element("button").unwrap();
        let btn_el: HtmlElement = btn.clone().dyn_into().unwrap();
        let is_active = *tab == state.active_activity;
        btn_el.style().set_css_text(&format!(
            "width: 36px; height: 36px; background: {}; border: none; border-radius: 6px; \
             font-size: 16px; cursor: pointer; display: flex; align-items: center; justify-content: center; \
             border-left: 2px solid {};",
            if is_active { "rgba(255, 255, 255, 0.1)" } else { "transparent" },
            if is_active { "#38bdf8" } else { "transparent" }
        ));
        btn.set_text_content(Some(tab.glyph()));
        btn.set_attribute("title", tab.label()).unwrap();
        activity_rail.append_child(&btn).unwrap();
    }
    main_area.append_child(&activity_rail).unwrap();

    // Zone B: Primary Sidebar (220px)
    let sidebar = document.create_element("div").unwrap();
    let sidebar_el: HtmlElement = sidebar.clone().dyn_into().unwrap();
    sidebar_el.style().set_css_text(
        "width: 220px; background: #090e1a; border-right: 1px solid rgba(255, 255, 255, 0.08); \
         display: flex; flex-direction: column; padding: 10px; gap: 8px; overflow-y: auto;",
    );

    let sidebar_title = document.create_element("span").unwrap();
    sidebar_title.set_text_content(Some("EXPLORER: QUALIA-WORKSPACE"));
    let sidebar_title_el: HtmlElement = sidebar_title.clone().dyn_into().unwrap();
    sidebar_title_el
        .style()
        .set_css_text("font-size: 10px; font-weight: 700; color: #94a3b8; letter-spacing: 0.5px;");
    sidebar.append_child(&sidebar_title).unwrap();

    for node in &state.file_tree {
        let node_el = document.create_element("div").unwrap();
        let node_html_el: HtmlElement = node_el.clone().dyn_into().unwrap();
        node_html_el.style().set_css_text(
            "font-size: 12px; font-family: var(--font-mono); color: #cbd5e1; cursor: pointer;",
        );
        node_el.set_text_content(Some(&format!("\u{1F4C1} {}", node.name)));
        sidebar.append_child(&node_el).unwrap();

        for child in &node.children {
            let child_el = document.create_element("div").unwrap();
            let child_html_el: HtmlElement = child_el.clone().dyn_into().unwrap();
            child_html_el.style().set_css_text(
                "padding-left: 16px; font-size: 12px; font-family: var(--font-mono); \
                 color: #94a3b8; cursor: pointer; padding-top: 2px;",
            );
            child_el.set_text_content(Some(&format!("\u{1F4C4} {}", child.name)));
            sidebar.append_child(&child_el).unwrap();
        }
    }
    main_area.append_child(&sidebar).unwrap();

    // Zone C & D: Central Editor Workspace + Bottom Drawer
    let central = document.create_element("div").unwrap();
    let central_el: HtmlElement = central.clone().dyn_into().unwrap();
    central_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; overflow: hidden;");

    // Zone C: Tab bar + Editor Pane
    let tab_bar = document.create_element("div").unwrap();
    let tab_bar_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tab_bar_el.style().set_css_text(
        "display: flex; background: #090e1a; border-bottom: 1px solid rgba(255, 255, 255, 0.08); overflow-x: auto;"
    );

    for (idx, tab) in state.open_tabs.iter().enumerate() {
        let tab_btn = document.create_element("div").unwrap();
        let tab_btn_el: HtmlElement = tab_btn.clone().dyn_into().unwrap();
        let is_active = idx == state.active_tab_index;
        tab_btn_el.style().set_css_text(&format!(
            "display: flex; align-items: center; gap: 8px; padding: 8px 14px; font-size: 12px; \
             font-family: var(--font-mono); cursor: pointer; background: {}; border-top: 2px solid {}; \
             border-right: 1px solid rgba(255, 255, 255, 0.05); color: {};",
            if is_active { "#020617" } else { "#0b1120" },
            if is_active { "#38bdf8" } else { "transparent" },
            if is_active { "#f8fafc" } else { "#94a3b8" }
        ));
        tab_btn.set_text_content(Some(&tab.title));
        tab_bar.append_child(&tab_btn).unwrap();
    }
    central.append_child(&tab_bar).unwrap();

    // Active Code Editor Pane
    let editor_pane = document.create_element("div").unwrap();
    let editor_pane_el: HtmlElement = editor_pane.clone().dyn_into().unwrap();
    editor_pane_el.style().set_css_text(
        "flex: 1; display: flex; background: #020617; padding: 10px; font-family: var(--font-mono); \
         font-size: 12px; line-height: 1.6; overflow: auto;"
    );

    if let Some(active_tab) = state.open_tabs.get(state.active_tab_index) {
        let code_view = document.create_element("pre").unwrap();
        let code_view_el: HtmlElement = code_view.clone().dyn_into().unwrap();
        code_view_el.style().set_css_text("margin: 0; flex: 1;");
        code_view.set_inner_html(&syntax_highlight_vibe(&active_tab.content));
        editor_pane.append_child(&code_view).unwrap();
    }
    central.append_child(&editor_pane).unwrap();

    // Zone D: studio bay — REPL · Problems · Catalog (lexicon pack peer)
    let drawer = document.create_element("div").unwrap();
    drawer.set_class_name("studio-bay");
    let drawer_el: HtmlElement = drawer.clone().dyn_into().unwrap();
    drawer_el.style().set_css_text(
        "height: 220px; background: #090e1a; border-top: 1px solid rgba(255, 255, 255, 0.08); \
         display: flex; flex-direction: column;",
    );

    let drawer_tabs = document.create_element("div").unwrap();
    drawer_tabs.set_attribute("role", "tablist").ok();
    let drawer_tabs_el: HtmlElement = drawer_tabs.clone().dyn_into().unwrap();
    drawer_tabs_el.style().set_css_text(
        "display: flex; gap: 12px; padding: 6px 12px; border-bottom: 1px solid rgba(255, 255, 255, 0.06); font-size: 11px;"
    );

    let repl_tab = document.create_element("button").unwrap();
    repl_tab.set_class_name("studio-bay-tab is-active");
    repl_tab.set_attribute("type", "button").ok();
    repl_tab.set_attribute("data-bay-tab", "repl").ok();
    repl_tab.set_attribute("aria-selected", "true").ok();
    repl_tab.set_text_content(Some("Vibe REPL"));
    drawer_tabs.append_child(&repl_tab).unwrap();

    let problems_tab = document.create_element("button").unwrap();
    problems_tab.set_class_name("studio-bay-tab");
    problems_tab.set_attribute("type", "button").ok();
    problems_tab.set_attribute("data-bay-tab", "problems").ok();
    problems_tab.set_attribute("aria-selected", "false").ok();
    problems_tab.set_text_content(Some(&format!("Problems ({})", state.problems.len())));
    drawer_tabs.append_child(&problems_tab).unwrap();

    let catalog_tab = document.create_element("button").unwrap();
    catalog_tab.set_class_name("studio-bay-tab");
    catalog_tab.set_attribute("type", "button").ok();
    catalog_tab.set_attribute("data-bay-tab", "catalog").ok();
    catalog_tab.set_attribute("aria-selected", "false").ok();
    catalog_tab.set_text_content(Some("Catalog"));
    drawer_tabs.append_child(&catalog_tab).unwrap();

    drawer.append_child(&drawer_tabs).unwrap();

    let drawer_body = document.create_element("div").unwrap();
    let drawer_body_el: HtmlElement = drawer_body.clone().dyn_into().unwrap();
    drawer_body_el.style().set_css_text(
        "flex: 1; padding: 8px 12px; font-family: var(--font-mono); font-size: 11px; overflow-y: auto;",
    );

    let repl_pane = document.create_element("div").unwrap();
    repl_pane.set_attribute("data-bay-pane", "repl").ok();
    for entry in &state.repl_history {
        let entry_div = document.create_element("div").unwrap();
        entry_div.set_text_content(Some(&format!(
            "vibe> {}\n\u{2794} {} [Gas: {}u]",
            entry.prompt, entry.response, entry.gas_consumed
        )));
        let entry_div_el: HtmlElement = entry_div.clone().dyn_into().unwrap();
        entry_div_el
            .style()
            .set_css_text("color: #34d399; margin-bottom: 4px; white-space: pre-line;");
        repl_pane.append_child(&entry_div).unwrap();
    }
    drawer_body.append_child(&repl_pane).unwrap();

    let problems_pane = document.create_element("div").unwrap();
    problems_pane
        .set_attribute("data-bay-pane", "problems")
        .ok();
    problems_pane.set_attribute("hidden", "").ok();
    let problems_el: HtmlElement = problems_pane.clone().dyn_into().unwrap();
    problems_el.style().set_property("display", "none").ok();
    for problem in &state.problems {
        let row = document.create_element("div").unwrap();
        row.set_text_content(Some(&format!(
            "{} · {} — {}",
            problem.code, problem.file_path, problem.message
        )));
        problems_pane.append_child(&row).unwrap();
    }
    drawer_body.append_child(&problems_pane).unwrap();

    let catalog_pane = super::lexicon_bay::build_lexicon_bay(document);
    catalog_pane.set_attribute("data-bay-pane", "catalog").ok();
    catalog_pane.set_attribute("hidden", "").ok();
    let catalog_el: HtmlElement = catalog_pane.clone().dyn_into().unwrap();
    catalog_el.style().set_property("display", "none").ok();
    drawer_body.append_child(&catalog_pane).unwrap();

    super::lexicon_bay::wire_bay_tabs(
        &drawer_tabs,
        &[
            ("repl", &repl_pane),
            ("problems", &problems_pane),
            ("catalog", &catalog_pane),
        ],
    );

    drawer.append_child(&drawer_body).unwrap();
    central.append_child(&drawer).unwrap();

    main_area.append_child(&central).unwrap();
    root.append_child(&main_area).unwrap();

    // Zone F: Status Bar (Bottom 24px)
    let status_bar = document.create_element("div").unwrap();
    let status_bar_el: HtmlElement = status_bar.clone().dyn_into().unwrap();
    status_bar_el.style().set_css_text(
        "height: 24px; background: #0f172a; border-top: 1px solid rgba(255, 255, 255, 0.08); \
         display: flex; align-items: center; justify-content: space-between; padding: 0 12px; \
         font-size: 11px; font-family: var(--font-mono); color: #94a3b8;",
    );

    let sb_left = document.create_element("span").unwrap();
    sb_left.set_text_content(Some(&format!(
        "\u{1F33F} branch: {} \u{00B7} \u{2713} LSP Ready",
        state.git_branch
    )));
    status_bar.append_child(&sb_left).unwrap();

    let sb_right = document.create_element("span").unwrap();
    sb_right.set_text_content(Some(&format!(
        "\u{26A1} Gas: {}u \u{00B7} \u{1F9E0} Sentinel: {:.1}/42MB \u{00B7} UTF-8",
        state.total_gas_consumed, state.sentinel_memory_used_mb
    )));
    status_bar.append_child(&sb_right).unwrap();

    root.append_child(&status_bar).unwrap();

    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ide_default_state() {
        let state = IdeState::default();
        assert_eq!(state.open_tabs.len(), 3);
        assert_eq!(state.active_activity, IdeActivityTab::Explorer);
        assert_eq!(state.problems.len(), 2);
        assert_eq!(state.repl_history.len(), 2);
        assert_eq!(state.git_branch, "0.0.34");
    }

    #[test]
    fn test_ide_open_and_close_tab() {
        let mut state = IdeState::default();
        state.open_file("src/test.vibe", "// test content", "vibe");
        assert_eq!(state.open_tabs.len(), 4);
        assert_eq!(state.open_tabs[3].title, "test.vibe");

        state.close_tab(3);
        assert_eq!(state.open_tabs.len(), 3);
    }

    #[test]
    fn test_ide_repl_eval() {
        let mut state = IdeState::default();
        let initial_gas = state.total_gas_consumed;
        let entry = state.eval_repl("1 + 2 * 3");
        assert!(!entry.is_error);
        assert_eq!(entry.response, "7");
        assert!(entry.gas_consumed > 0);
        assert_eq!(state.total_gas_consumed, initial_gas + entry.gas_consumed);

        // Test capability evaluation through REPL
        let cap_entry = state.eval_repl("Animation.orbit_spin(0.5)");
        assert!(!cap_entry.is_error);
    }

    #[test]
    fn test_syntax_highlight_vibe() {
        let code = "entity \"Sensor\" {\n  let x = 10;\n}";
        let html = syntax_highlight_vibe(code);
        assert!(html.contains("entity"));
        assert!(html.contains("let"));
        assert!(html.contains("#f43f5e"));
    }
}
