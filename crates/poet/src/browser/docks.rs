//! Dock rendering: toolbox sidebar, right dock (aura + pulse), bottom status bar.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, Event, HtmlElement};

use crate::browser::tool_widgets::ToolWidget;
use crate::tool_chest::core::tool::ToolKind;
use crate::tool_chest::core::tool_chain::ToolChainMetadata;
use crate::tool_chest::core::toolbox::{Toolbox, ToolboxMetadata};

// ---------------------------------------------------------------------------
// Cloneable view models (the registry holds Box<dyn Tool> which is not Clone)
// ---------------------------------------------------------------------------

/// A cloneable view of a single tool's metadata for UI rendering.
#[derive(Clone, Debug)]
pub struct ToolView {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub kind: ToolKind,
    pub capability_scope: Option<String>,
    pub description: String,
}

/// A cloneable view of a tool-chain with its tools and domain widgets.
#[derive(Clone, Debug)]
pub struct ToolChainView {
    pub metadata: ToolChainMetadata,
    pub tools: Vec<ToolView>,
    pub widgets: Vec<ToolWidget>,
}

/// A cloneable view of a toolbox with its tool-chains.
#[derive(Clone, Debug)]
pub struct ToolboxView {
    pub metadata: ToolboxMetadata,
    pub chains: Vec<ToolChainView>,
}

/// Dock position orientations for 4-way docking architecture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockPosition {
    Left,
    Top,
    Right,
    Bottom,
}

impl DockPosition {
    pub fn as_str(&self) -> &'static str {
        match self {
            DockPosition::Left => "left",
            DockPosition::Top => "top",
            DockPosition::Right => "right",
            DockPosition::Bottom => "bottom",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "top" => DockPosition::Top,
            "right" => DockPosition::Right,
            "bottom" => DockPosition::Bottom,
            _ => DockPosition::Left,
        }
    }
}

/// Metadata for a toolbox family group.
#[derive(Clone, Debug)]
pub struct ToolboxFamily {
    pub id: String,
    pub label: String,
    pub icon: String,
}

/// Get the ordered list of 12 master toolbox families.
pub fn family_order() -> Vec<ToolboxFamily> {
    vec![
        ToolboxFamily {
            id: "epistemic".into(),
            label: "Epistemic Mindware".into(),
            icon: "\u{1F9ED}".into(), // 🧭
        },
        ToolboxFamily {
            id: "authoring".into(),
            label: "Word Processor & CML".into(),
            icon: "\u{1F4DD}".into(), // 📝
        },
        ToolboxFamily {
            id: "sheet".into(),
            label: "Spreadsheets & Tensors".into(),
            icon: "\u{1F4CA}".into(), // 📊
        },
        ToolboxFamily {
            id: "graphics".into(),
            label: "Graphics & Vector".into(),
            icon: "\u{1F3A8}".into(), // 🎨
        },
        ToolboxFamily {
            id: "spatial".into(),
            label: "3D & Geospatial".into(),
            icon: "\u{1F9CA}".into(), // 🧊
        },
        ToolboxFamily {
            id: "audio".into(),
            label: "Triad Formant Audio".into(),
            icon: "\u{1F399}\u{FE0F}".into(), // 🎙️
        },
        ToolboxFamily {
            id: "code".into(),
            label: "Code IDE & Vibe REPL".into(),
            icon: "\u{1F4BB}".into(), // 💻
        },
        ToolboxFamily {
            id: "erp".into(),
            label: "Cooperative ERP & PM".into(),
            icon: "\u{1F4C5}".into(), // 📅
        },
        ToolboxFamily {
            id: "mail".into(),
            label: "Mail & Web Presence".into(),
            icon: "\u{2709}\u{FE0F}".into(), // ✉️
        },
        ToolboxFamily {
            id: "lab".into(),
            label: "Scientific & Clinical".into(),
            icon: "\u{1F52C}".into(), // 🔬
        },
        ToolboxFamily {
            id: "ai".into(),
            label: "AI Co-Pilot & Sentinel".into(),
            icon: "\u{2728}".into(), // ✨
        },
        ToolboxFamily {
            id: "governance".into(),
            label: "Governance & Rights".into(),
            icon: "\u{2696}\u{FE0F}".into(), // ⚖️
        },
        ToolboxFamily {
            id: "sdn".into(),
            label: "SDN & Economics".into(),
            icon: "\u{1F310}".into(), // 🌐
        },
    ]
}

/// Build specialized domain widgets for a tool-chain based on its ID and tools.
pub fn build_toolchain_widgets(chain_id: &str, tools: &[ToolView]) -> Vec<ToolWidget> {
    let mut widgets = Vec::new();

    match chain_id {
        // Typography in Word Processor / Authoring
        id if id.contains("typography") || id.contains("font") => {
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:font_family"),
                label: "Font Family".into(),
                options: vec![
                    ("Inter".into(), "Inter (System Sans)".into()),
                    ("JetBrains Mono".into(), "JetBrains Mono (Code)".into()),
                    ("Outfit".into(), "Outfit (Modern Geometric)".into()),
                    ("Lora".into(), "Lora (Editorial Serif)".into()),
                    ("Cinzel".into(), "Cinzel (Classical Display)".into()),
                ],
                default_val: "Inter".into(),
            });
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:font_size"),
                label: "Font Size".into(),
                options: vec![
                    ("12px".into(), "12px — Caption".into()),
                    ("14px".into(), "14px — Compact".into()),
                    ("16px".into(), "16px — Body".into()),
                    ("18px".into(), "18px — Lead".into()),
                    ("24px".into(), "24px — Heading".into()),
                    ("32px".into(), "32px — Title".into()),
                ],
                default_val: "16px".into(),
            });
            widgets.push(ToolWidget::ToggleGroup {
                id: format!("{id}:style"),
                label: "Text Style".into(),
                options: vec![
                    ("bold".into(), "B".into(), "Bold (Ctrl+B)".into()),
                    ("italic".into(), "I".into(), "Italic (Ctrl+I)".into()),
                    ("underline".into(), "U".into(), "Underline (Ctrl+U)".into()),
                    ("code".into(), "</>".into(), "Inline Code".into()),
                ],
                default_selected: "".into(),
            });
            widgets.push(ToolWidget::ColorPicker {
                id: format!("{id}:color"),
                label: "Text Color".into(),
                default_hex: "#00f2a9".into(),
                presets: vec![
                    "#00f2a9".into(),
                    "#38bdf8".into(),
                    "#ffb834".into(),
                    "#f43f5e".into(),
                    "#ffffff".into(),
                    "#94a3b8".into(),
                ],
            });
        }

        // Paragraph & Headings in Authoring
        id if id.contains("paragraph") || id.contains("layout") => {
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:heading_level"),
                label: "Block Style".into(),
                options: vec![
                    ("p".into(), "Paragraph".into()),
                    ("h1".into(), "H1 — Chapter Title".into()),
                    ("h2".into(), "H2 — Section Header".into()),
                    ("h3".into(), "H3 — Subsection".into()),
                    ("callout".into(), "Callout Box 💡".into()),
                    ("quote".into(), "Blockquote ❝".into()),
                ],
                default_val: "p".into(),
            });
            widgets.push(ToolWidget::ToggleGroup {
                id: format!("{id}:align"),
                label: "Alignment".into(),
                options: vec![
                    ("left".into(), "≡".into(), "Align Left".into()),
                    ("center".into(), "⫼".into(), "Align Center".into()),
                    ("right".into(), "⫹".into(), "Align Right".into()),
                    ("justify".into(), "▤".into(), "Justify".into()),
                ],
                default_selected: "left".into(),
            });
        }

        // Brushes & Stroke in Graphics
        id if id.contains("brush") || id.contains("pen") || id.contains("ink") => {
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:brush_type"),
                label: "Brush Type".into(),
                options: vec![
                    ("round".into(), "Round Pen 🖊️".into()),
                    ("calligraphy".into(), "Calligraphy Nib ✒️".into()),
                    ("airbrush".into(), "Airbrush 💨".into()),
                    ("vector".into(), "Vector Inker ⚡".into()),
                    ("highlighter".into(), "Highlighter 🖍️".into()),
                    ("eraser".into(), "Precision Eraser 🧹".into()),
                ],
                default_val: "round".into(),
            });
            widgets.push(ToolWidget::Slider {
                id: format!("{id}:brush_size"),
                label: "Brush Size".into(),
                min: 1.0,
                max: 64.0,
                step: 1.0,
                default_val: 6.0,
                unit: "px".into(),
            });
            widgets.push(ToolWidget::Slider {
                id: format!("{id}:opacity"),
                label: "Stroke Opacity".into(),
                min: 5.0,
                max: 100.0,
                step: 5.0,
                default_val: 100.0,
                unit: "%".into(),
            });
        }

        // Color & Palette in Graphics
        id if id.contains("palette") || id.contains("color") => {
            widgets.push(ToolWidget::ColorPicker {
                id: format!("{id}:stroke_color"),
                label: "Stroke Color".into(),
                default_hex: "#38bdf8".into(),
                presets: vec![
                    "#00f2a9".into(),
                    "#38bdf8".into(),
                    "#ffb834".into(),
                    "#f43f5e".into(),
                    "#a855f7".into(),
                    "#ffffff".into(),
                    "#030508".into(),
                ],
            });
            widgets.push(ToolWidget::ColorPicker {
                id: format!("{id}:fill_color"),
                label: "Fill Color".into(),
                default_hex: "#ffb834".into(),
                presets: vec![
                    "#ffb834".into(),
                    "#00f2a9".into(),
                    "#38bdf8".into(),
                    "#f43f5e".into(),
                    "#818cf8".into(),
                    "#1e293b".into(),
                ],
            });
            widgets.push(ToolWidget::ToggleGroup {
                id: format!("{id}:shape_mode"),
                label: "Geometry Mode".into(),
                options: vec![
                    ("rect".into(), "▭".into(), "Rectangle".into()),
                    ("circle".into(), "◯".into(), "Circle / Ellipse".into()),
                    ("poly".into(), "⬡".into(), "Polygon".into()),
                    ("arrow".into(), "➔".into(), "Arrow Vector".into()),
                    ("spline".into(), "〰".into(), "Bézier Spline".into()),
                ],
                default_selected: "rect".into(),
            });
        }

        // Spreadsheet / Tensor Grid
        id if id.contains("sheet") || id.contains("grid") || id.contains("tensor") => {
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:dimension"),
                label: "Tensor Dimensionality".into(),
                options: vec![
                    ("1d".into(), "1D Vector Array".into()),
                    ("2d".into(), "2D Matrix / Table".into()),
                    ("3d".into(), "3D Volume Tensor".into()),
                    ("10d".into(), "10D Manifold State".into()),
                ],
                default_val: "2d".into(),
            });
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:format"),
                label: "Cell Number Format".into(),
                options: vec![
                    ("standard".into(), "General / Automatic".into()),
                    ("currency".into(), "Currency ($ USD)".into()),
                    ("percent".into(), "Percentage (%)".into()),
                    ("scientific".into(), "Scientific (1.23e+4)".into()),
                    ("formula".into(), "Formula (=fx)".into()),
                    ("quin".into(), "48-byte Super-Quin".into()),
                ],
                default_val: "standard".into(),
            });
        }

        // Code IDE & Vibe REPL
        id if id.contains("code") || id.contains("repl") || id.contains("lang") => {
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:dialect"),
                label: "Execution Dialect".into(),
                options: vec![
                    ("vibe".into(), "VibeScript 0.1 AST".into()),
                    ("wgsl".into(), "WGSL WebGPU Shader".into()),
                    ("turtle".into(), "Turtle RDF 1.2".into()),
                    ("sparql".into(), "SPARQL 1.1 Query".into()),
                    ("wasm".into(), "Rust WASM Engine".into()),
                ],
                default_val: "vibe".into(),
            });
            widgets.push(ToolWidget::Slider {
                id: format!("{id}:gas_limit"),
                label: "Gas Budget".into(),
                min: 5000.0,
                max: 200000.0,
                step: 5000.0,
                default_val: 50000.0,
                unit: " gas".into(),
            });
        }

        // Audio Synth
        id if id.contains("audio") || id.contains("synth") => {
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:waveform"),
                label: "Oscillator Waveform".into(),
                options: vec![
                    ("sine".into(), "Pure Sine Wave ∿".into()),
                    ("triangle".into(), "Triangle Wave ⋀".into()),
                    ("sawtooth".into(), "Sawtooth Wave ⩘".into()),
                    ("square".into(), "Square Wave ⊓".into()),
                    ("formant".into(), "Triad Vocal Formant 🗣️".into()),
                ],
                default_val: "sine".into(),
            });
            widgets.push(ToolWidget::Slider {
                id: format!("{id}:freq"),
                label: "Pitch / Base Frequency".into(),
                min: 110.0,
                max: 1760.0,
                step: 10.0,
                default_val: 440.0,
                unit: " Hz".into(),
            });
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:vowel"),
                label: "Formant Vowel".into(),
                options: vec![
                    ("a".into(), "/a/ — Open (Father)".into()),
                    ("i".into(), "/i/ — Front (See)".into()),
                    ("u".into(), "/u/ — Back (Too)".into()),
                    ("e".into(), "/e/ — Mid (Bed)".into()),
                    ("o".into(), "/o/ — Rounded (Call)".into()),
                ],
                default_val: "a".into(),
            });
        }

        // 3D Spatial
        id if id.contains("spatial") || id.contains("3d") || id.contains("viewport") => {
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:projection"),
                label: "Camera Projection".into(),
                options: vec![
                    ("perspective".into(), "Perspective (FOV 60°)".into()),
                    ("orthographic".into(), "Orthographic Parallel".into()),
                    ("isometric".into(), "Isometric 30° / 60°".into()),
                    ("tensor10d".into(), "10D Tensor Projector".into()),
                ],
                default_val: "perspective".into(),
            });
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:pipeline"),
                label: "WGSL Render Pipeline".into(),
                options: vec![
                    ("cyber_glass".into(), "Cyber Glass (Bloom + Glass)".into()),
                    ("wireframe".into(), "Wireframe Topology".into()),
                    ("pbr_metallic".into(), "PBR Metallic Roughness".into()),
                    ("normals".into(), "Geometric Normals Debug".into()),
                ],
                default_val: "cyber_glass".into(),
            });
        }

        // AI Co-Pilot & Sentinel
        id if id.contains("ai") || id.contains("copilot") || id.contains("sentinel") => {
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:model"),
                label: "Resident Model".into(),
                options: vec![
                    ("q42_sentinel".into(), "Q42-Sentinel 1.5B (GGUF)".into()),
                    ("llama3_8b".into(), "Llama-3-8B (Q4_K_M)".into()),
                    ("directml_gpu".into(), "DirectML GPU Autoregressive".into()),
                ],
                default_val: "q42_sentinel".into(),
            });
            widgets.push(ToolWidget::Slider {
                id: format!("{id}:halo"),
                label: "Epistemic Halo Threshold".into(),
                min: 60.0,
                max: 99.0,
                step: 1.0,
                default_val: 85.0,
                unit: "%".into(),
            });
            widgets.push(ToolWidget::Slider {
                id: format!("{id}:temp"),
                label: "Sampling Temperature".into(),
                min: 0.0,
                max: 1.2,
                step: 0.05,
                default_val: 0.7,
                unit: "".into(),
            });
        }

        // Governance & Rights
        id if id.contains("rights") || id.contains("governance") || id.contains("fiduciary") => {
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:lane"),
                label: "Privacy Routing Lane".into(),
                options: vec![
                    ("00".into(), "Public Commons (00)".into()),
                    ("01".into(), "Bilateral Micro-Commons (01)".into()),
                    ("11".into(), "Spatial Commons (11)".into()),
                    ("10".into(), "Classified Sanctuary (10)".into()),
                ],
                default_val: "01".into(),
            });
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:hohfeld"),
                label: "Hohfeldian Modality".into(),
                options: vec![
                    ("privilege".into(), "Privilege (Liberty)".into()),
                    ("duty".into(), "Duty (Obligation)".into()),
                    ("right".into(), "Right (Claim)".into()),
                    ("no_right".into(), "No-Right".into()),
                    ("immunity".into(), "Immunity".into()),
                    ("disability".into(), "Disability".into()),
                ],
                default_val: "privilege".into(),
            });
        }

        // Clinical Lab & Science
        id if id.contains("health") || id.contains("clinical") || id.contains("lab") => {
            widgets.push(ToolWidget::Dropdown {
                id: format!("{id}:calc"),
                label: "Clinical / Assay Engine".into(),
                options: vec![
                    ("framingham".into(), "Framingham 10-Year CVD Risk".into()),
                    ("cha2ds2_vasc".into(), "CHA₂DS₂-VASc Stroke Risk".into()),
                    ("score2".into(), "SCORE2 European Mortality".into()),
                    ("smiles_mol".into(), "SMILES Molecular Weight & LogP".into()),
                    ("protein_sw".into(), "Smith-Waterman Protein Align".into()),
                ],
                default_val: "framingham".into(),
            });
            widgets.push(ToolWidget::Slider {
                id: format!("{id}:sbp"),
                label: "Systolic Blood Pressure (SBP)".into(),
                min: 80.0,
                max: 220.0,
                step: 1.0,
                default_val: 125.0,
                unit: " mmHg".into(),
            });
        }

        _ => {}
    }

    // Append standard Tool buttons for all tools in this chain
    for tool in tools {
        widgets.push(ToolWidget::Button {
            id: tool.id.clone(),
            label: tool.label.clone(),
            icon: tool_glyph(&tool.icon).to_string(),
            kind_badge: kind_label(tool.kind).to_string(),
            action: tool.id.clone(),
        });
    }

    widgets
}

/// Extract cloneable views from the registry's toolboxes.
pub fn extract_toolbox_views(toolboxes: &[Toolbox]) -> Vec<ToolboxView> {
    toolboxes
        .iter()
        .map(|tb| ToolboxView {
            metadata: tb.metadata().clone(),
            chains: tb
                .chains()
                .iter()
                .map(|chain| {
                    let tools: Vec<ToolView> = chain
                        .tools()
                        .iter()
                        .map(|tool| {
                            let m = tool.metadata();
                            ToolView {
                                id: m.id.clone(),
                                label: m.label.clone(),
                                icon: m.icon.clone(),
                                kind: m.kind,
                                capability_scope: m.capability_scope.clone(),
                                description: m.description.clone(),
                            }
                        })
                        .collect();

                    let widgets = build_toolchain_widgets(&chain.metadata().id, &tools);

                    ToolChainView {
                        metadata: chain.metadata().clone(),
                        tools,
                        widgets,
                    }
                })
                .collect(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Thread-local storage for flyout rendering
// ---------------------------------------------------------------------------

thread_local! {
    static TOOLBOX_VIEWS: RefCell<Vec<ToolboxView>> = RefCell::new(Vec::new());
}

/// Store toolbox views in the thread-local for access from click handlers.
pub fn store_toolbox_views(views: Vec<ToolboxView>) {
    TOOLBOX_VIEWS.with(|v| {
        *v.borrow_mut() = views;
    });
}

// ---------------------------------------------------------------------------
// Glyph mapping
// ---------------------------------------------------------------------------

/// Map a toolbox id to a display glyph covering all 12 themed Master Toolboxes.
fn toolbox_glyph(id: &str) -> &'static str {
    match id {
        "epistemic" => "\u{1F9ED}",             // 🧭
        "office" | "word_processor" | "tb_word_processor" | "doc" => "\u{1F4DD}", // 📝
        "sheet" | "tb_spreadsheet" => "\u{1F4CA}", // 📊
        "image" | "graphics" | "tb_graphics" => "\u{1F3A8}", // 🎨
        "spatial" | "3d" | "tb_3d_spatial" => "\u{1F9CA}", // 🧊
        "audio" | "audio_synth" | "tb_audio_synth" => "\u{1F399}\u{FE0F}", // 🎙️
        "code" | "tb_code_ide" => "\u{1F4BB}",    // 💻
        "communication" | "mail" | "tb_mail_publish" => "\u{2709}\u{FE0F}", // ✉️
        "erp" | "tb_erp_workstream" => "\u{1F4C5}", // 📅
        "lab" | "science" | "tb_scientific_lab" => "\u{1F52C}", // 🔬
        "ai" | "tb_ai_copilot" => "\u{2728}",     // ✨
        "rights" | "governance" | "tb_governance_rights" => "\u{2696}\u{FE0F}", // ⚖️
        "sdn" | "tb_sdn_cooperative" => "\u{1F310}", // 🌐
        "health" => "\u{1FA7A}",                  // 🩺
        _ => "\u{1F9E9}",
    }
}

/// Map a tool icon identifier to a display glyph.
fn tool_glyph(icon: &str) -> &'static str {
    match icon {
        "doc" => "\u{1F4C4}",
        "ontology" => "\u{1F4D6}",
        "slide" => "\u{1F4CA}",
        "media" => "\u{1F3A8}",
        "marker" => "\u{1F4CD}",
        "heatmap" => "\u{1F525}",
        "sheet" => "\u{1F4CA}",
        "import" => "\u{21E9}",
        "map" => "\u{1F5FA}",
        "3d" => "\u{1F3AF}",
        "pin" => "\u{1F4CC}",
        "track" => "\u{1F50D}",
        "social" => "\u{1F4AC}",
        "webrtc" => "\u{1F4F7}",
        "webview" => "\u{1F310}",
        "group" => "\u{1F465}",
        "sign" => "\u{270D}",
        "did" => "\u{1F194}",
        "health" => "\u{1FA7A}",
        "pathology" => "\u{1F52C}",
        "anatomy" => "\u{1F9B2}",
        "vibe" => "\u{1F4BB}",
        "quin" => "\u{1F9EC}",
        "coauthor" => "\u{1F9D1}",
        "extractor" => "\u{26CF}",
        "sentinel" => "\u{1F6E1}",
        "triad" => "\u{1F3A8}",
        "objective" => "\u{1F4CD}",
        "subjective" => "\u{1F9ED}",
        "intersubjective" => "\u{1F91D}",
        "normative" => "\u{2696}",
        _ => "\u{1F4A1}",
    }
}

/// Short kind label for the tool button badge.
fn kind_label(kind: ToolKind) -> &'static str {
    match kind {
        ToolKind::PlaceContainer => "place",
        ToolKind::RunAction => "action",
        ToolKind::Query => "query",
        ToolKind::Navigate => "nav",
        ToolKind::Toggle => "toggle",
    }
}

// ---------------------------------------------------------------------------
// Dock builder with 4-Way Docking Architecture
// ---------------------------------------------------------------------------

/// Build the toolbox dock from a populated registry with 4-way docking anchor controls.
pub fn build_toolbox_dock(document: &Document, toolboxes: &[Toolbox]) -> Element {
    let dock = document.create_element("div").unwrap();
    dock.set_class_name("toolbox-dock dock-pos-left");

    // Dock Header: Brand + 4-Way Docking Anchor Bar
    let dock_header = document.create_element("div").unwrap();
    dock_header.set_class_name("dock-master-header");
    let dh_el: HtmlElement = dock_header.clone().dyn_into().unwrap();
    dh_el.style().set_css_text(
        "display: flex; align-items: center; justify-content: space-between; \
         padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); margin-bottom: 4px;",
    );

    let title_span = document.create_element("span").unwrap();
    let ts_el: HtmlElement = title_span.clone().dyn_into().unwrap();
    ts_el.style().set_css_text(
        "font-size: 9px; font-weight: 700; color: var(--accent-cyan); \
         text-transform: uppercase; letter-spacing: 0.5px; font-family: var(--font-mono);",
    );
    title_span.set_text_content(Some("\u{1F9F0} Tool Chest"));
    dock_header.append_child(&title_span).unwrap();

    // 4-Way Dock Anchor Controls
    let anchor_bar = document.create_element("div").unwrap();
    anchor_bar.set_class_name("dock-anchor-bar");
    let ab_el: HtmlElement = anchor_bar.clone().dyn_into().unwrap();
    ab_el.style().set_css_text("display: flex; gap: 2px;");

    let positions = [
        ("left", "\u{25C0}"),
        ("top", "\u{25B2}"),
        ("right", "\u{25B6}"),
        ("bottom", "\u{25BC}"),
    ];

    for (pos_id, glyph) in &positions {
        let pos_btn = document.create_element("button").unwrap();
        pos_btn.set_class_name("dock-pos-btn");
        pos_btn.set_attribute("data-pos", pos_id).unwrap();
        pos_btn.set_attribute("title", &format!("Dock {}", pos_id)).unwrap();
        let pb_el: HtmlElement = pos_btn.clone().dyn_into().unwrap();
        pb_el.style().set_css_text(
            "padding: 1px 3px; font-size: 8px; background: transparent; border: 1px solid transparent; \
             border-radius: 2px; color: var(--text-muted); cursor: pointer; transition: var(--trans-fast);",
        );
        pos_btn.set_text_content(Some(glyph));

        let dock_clone = dock.clone();
        let pos_str = pos_id.to_string();
        let pos_closure = Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            let d_el: HtmlElement = dock_clone.clone().dyn_into().unwrap();
            d_el.set_class_name(&format!("toolbox-dock dock-pos-{}", pos_str));
            if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok()).flatten() {
                let _ = storage.set_item("qualia_dock_pos", &pos_str);
            }
        }) as Box<dyn FnMut(web_sys::MouseEvent)>);
        pos_btn.add_event_listener_with_callback("click", pos_closure.as_ref().unchecked_ref()).unwrap();
        pos_closure.forget();

        anchor_bar.append_child(&pos_btn).unwrap();
    }
    dock_header.append_child(&anchor_bar).unwrap();
    dock.append_child(&dock_header).unwrap();

    let families = family_order();
    let mut first_toolbox = true;

    for family in &families {
        // Find toolboxes in this family
        let family_toolboxes: Vec<&Toolbox> = toolboxes
            .iter()
            .filter(|tb| tb.metadata().family == family.id)
            .collect();

        if family_toolboxes.is_empty() {
            continue;
        }

        // Family section
        let section = document.create_element("div").unwrap();
        section.set_class_name("dock-family-section");
        section.set_attribute("data-family", &family.id).unwrap();

        // Family header (collapsible)
        let header = document.create_element("button").unwrap();
        header.set_class_name("dock-family-header");
        header.set_attribute("data-family", &family.id).unwrap();
        header.set_attribute("title", &family.label).unwrap();

        let family_icon = document.create_element("span").unwrap();
        family_icon.set_class_name("dock-family-icon");
        family_icon.set_text_content(Some(&family.icon));
        header.append_child(&family_icon).unwrap();

        let family_label = document.create_element("span").unwrap();
        family_label.set_class_name("dock-family-label");
        family_label.set_text_content(Some(&family.label));
        header.append_child(&family_label).unwrap();

        let chevron = document.create_element("span").unwrap();
        chevron.set_class_name("dock-family-chevron");
        chevron.set_text_content(Some("\u{25BE}"));
        header.append_child(&chevron).unwrap();

        section.append_child(&header).unwrap();

        // Toolbox buttons (children, shown by default for first family)
        let children = document.create_element("div").unwrap();
        children.set_class_name("dock-family-children");
        if first_toolbox {
            children.class_list().add_1("expanded").unwrap();
        }

        for toolbox in &family_toolboxes {
            let meta = toolbox.metadata();
            let btn = document.create_element("button").unwrap();
            btn.set_class_name("toolbox-dock-btn");
            if first_toolbox {
                btn.class_list().add_1("active").unwrap();
                first_toolbox = false;
            }
            btn.set_attribute("data-toolbox", &meta.id).unwrap();
            btn.set_text_content(Some(toolbox_glyph(&meta.id)));

            let tooltip = document.create_element("span").unwrap();
            tooltip.set_class_name("dock-tooltip");
            tooltip.set_text_content(Some(&meta.label));
            btn.append_child(&tooltip).unwrap();

            children.append_child(&btn).unwrap();
        }

        section.append_child(&children).unwrap();
        dock.append_child(&section).unwrap();
    }

    dock
}

// ---------------------------------------------------------------------------
// Flyout panel — shows tool-chains and tools for the active toolbox
// ---------------------------------------------------------------------------

/// Show or replace the flyout panel for the given toolbox id.
/// Removes any existing flyout first.
pub fn show_flyout(document: &Document, toolbox_id: &str) {
    // Remove existing flyout
    if let Some(existing) = document.query_selector(".toolbox-flyout").unwrap() {
        existing.remove();
    }

    let view = TOOLBOX_VIEWS.with(|v| {
        v.borrow()
            .iter()
            .find(|t| t.metadata.id == toolbox_id)
            .cloned()
    });

    let view = match view {
        Some(v) => v,
        None => return,
    };

    let flyout = document.create_element("div").unwrap();
    flyout.set_class_name("toolbox-flyout");

    // Header: Icon + Title + Ontology Badge + Close button
    let header = document.create_element("div").unwrap();
    header.set_class_name("toolbox-flyout-header");

    let header_left = document.create_element("div").unwrap();
    header_left.set_class_name("flyout-header-left");

    let tb_icon = document.create_element("span").unwrap();
    tb_icon.set_class_name("flyout-header-icon");
    tb_icon.set_text_content(Some(toolbox_glyph(&view.metadata.id)));
    header_left.append_child(&tb_icon).unwrap();

    let title_wrap = document.create_element("div").unwrap();
    title_wrap.set_class_name("flyout-title-wrap");

    let title = document.create_element("div").unwrap();
    title.set_class_name("flyout-title-text");
    title.set_text_content(Some(&view.metadata.label));
    title_wrap.append_child(&title).unwrap();

    let desc = document.create_element("div").unwrap();
    desc.set_class_name("flyout-desc-text");
    desc.set_text_content(Some(&view.metadata.description));
    title_wrap.append_child(&desc).unwrap();

    header_left.append_child(&title_wrap).unwrap();
    header.append_child(&header_left).unwrap();

    let header_right = document.create_element("div").unwrap();
    header_right.set_class_name("flyout-header-right");

    let ont_badge = document.create_element("span").unwrap();
    ont_badge.set_class_name("flyout-ont-badge");
    ont_badge.set_text_content(Some(&format!("{}:", view.metadata.ontology_prefix)));
    header_right.append_child(&ont_badge).unwrap();

    let close_btn = document.create_element("button").unwrap();
    close_btn.set_class_name("flyout-close-btn");
    close_btn.set_attribute("title", "Close Drawer").unwrap();
    close_btn.set_text_content(Some("\u{2715}"));

    let close_closure = Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
        let doc = web_sys::window().unwrap().document().unwrap();
        hide_flyout(&doc);
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    close_btn
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .unwrap();
    close_closure.forget();

    header_right.append_child(&close_btn).unwrap();
    header.append_child(&header_right).unwrap();

    flyout.append_child(&header).unwrap();

    // Tool-chains & Interactive Domain Controls
    let chains_scroll = document.create_element("div").unwrap();
    chains_scroll.set_class_name("toolbox-flyout-body");

    for chain in &view.chains {
        let group = document.create_element("div").unwrap();
        group.set_class_name("toolchain-group");

        let chain_header = document.create_element("div").unwrap();
        chain_header.set_class_name("toolchain-label");
        chain_header
            .set_attribute("data-chain-id", &chain.metadata.id)
            .unwrap();
        chain_header
            .set_attribute("data-toolbox-id", &view.metadata.id)
            .unwrap();
        chain_header.set_attribute("draggable", "true").unwrap();
        chain_header
            .set_attribute(
                "title",
                "Click to activate on focused surface, or drag onto a container",
            )
            .unwrap();

        let chain_icon = document.create_element("span").unwrap();
        chain_icon.set_class_name("toolchain-label-icon");
        chain_icon.set_text_content(Some("\u{2630}"));
        chain_header.append_child(&chain_icon).unwrap();

        let chain_text = document.create_element("span").unwrap();
        chain_text.set_class_name("toolchain-label-text");
        chain_text.set_text_content(Some(&chain.metadata.label));
        chain_header.append_child(&chain_text).unwrap();

        if !chain.metadata.description.is_empty() {
            let chain_hint = document.create_element("span").unwrap();
            chain_hint.set_class_name("toolchain-label-hint");
            chain_hint.set_text_content(Some(&chain.metadata.description));
            chain_header.append_child(&chain_hint).unwrap();
        }

        group.append_child(&chain_header).unwrap();

        // Widgets container
        let widgets_box = document.create_element("div").unwrap();
        widgets_box.set_class_name("toolchain-widgets-container");

        if !chain.widgets.is_empty() {
            for widget in &chain.widgets {
                widgets_box.append_child(&widget.render(document)).unwrap();
            }
        } else {
            // Fallback for tools without rich widgets
            for tool in &chain.tools {
                let btn = document.create_element("button").unwrap();
                btn.set_class_name("tool-btn");
                btn.set_attribute("data-tool-id", &tool.id).unwrap();
                btn.set_attribute("data-chain-id", &chain.metadata.id)
                    .unwrap();
                btn.set_attribute("title", &tool.description).unwrap();

                let icon_el = document.create_element("span").unwrap();
                icon_el.set_class_name("tool-btn-icon");
                icon_el.set_text_content(Some(tool_glyph(&tool.icon)));
                btn.append_child(&icon_el).unwrap();

                let label_el = document.create_element("span").unwrap();
                label_el.set_class_name("tool-btn-label");
                label_el.set_text_content(Some(&tool.label));
                btn.append_child(&label_el).unwrap();

                let kind_el = document.create_element("span").unwrap();
                kind_el.set_class_name("tool-btn-kind");
                kind_el.set_text_content(Some(kind_label(tool.kind)));
                btn.append_child(&kind_el).unwrap();

                widgets_box.append_child(&btn).unwrap();
            }
        }

        group.append_child(&widgets_box).unwrap();
        chains_scroll.append_child(&group).unwrap();
    }

    flyout.append_child(&chains_scroll).unwrap();

    // Append to the workspace (so it positions relative to the dock)
    if let Some(workspace) = document.query_selector(".main-workspace").unwrap() {
        workspace.append_child(&flyout).unwrap();
    } else if let Some(body) = document.body() {
        body.append_child(&flyout).unwrap();
    }
}

/// Hide the flyout panel.
pub fn hide_flyout(document: &Document) {
    if let Some(existing) = document.query_selector(".toolbox-flyout").unwrap() {
        existing.remove();
    }
}

/// Create a collapsible dock panel with an interactive header, chevron indicator, title, optional badge, and collapsible body.
pub fn create_collapsible_dock_panel(
    document: &Document,
    title: &str,
    badge_text: Option<&str>,
    body: Element,
    initially_expanded: bool,
    flex_grow: bool,
) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("dock-panel");
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    if flex_grow {
        p_el.style()
            .set_css_text("flex: 1; min-height: 32px; overflow: hidden; display: flex; flex-direction: column;");
    } else {
        p_el.style()
            .set_css_text("min-height: 32px; display: flex; flex-direction: column;");
    }

    let header = document.create_element("div").unwrap();
    header.set_class_name("dock-panel-header");

    let left = document.create_element("div").unwrap();
    let l_el: HtmlElement = left.clone().dyn_into().unwrap();
    l_el.style()
        .set_css_text("display: flex; align-items: center; gap: 6px;");

    let chevron = document.create_element("span").unwrap();
    chevron.set_class_name("dock-panel-chevron");
    chevron.set_text_content(Some(if initially_expanded {
        "\u{25BE}" // ▾
    } else {
        "\u{25B8}" // ▸
    }));
    left.append_child(&chevron).unwrap();

    let title_span = document.create_element("span").unwrap();
    title_span.set_text_content(Some(title));
    left.append_child(&title_span).unwrap();
    header.append_child(&left).unwrap();

    if let Some(badge) = badge_text {
        let badge_span = document.create_element("span").unwrap();
        badge_span.set_class_name("dock-panel-badge");
        badge_span.set_text_content(Some(badge));
        header.append_child(&badge_span).unwrap();
    }

    panel.append_child(&header).unwrap();

    let b_el: HtmlElement = body.clone().dyn_into().unwrap();
    if !initially_expanded {
        b_el.style().set_property("display", "none").unwrap();
        let _ = panel.class_list().add_1("collapsed");
        if flex_grow {
            let _ = p_el.style().set_property("flex", "0 0 auto");
        }
    }
    panel.append_child(&body).unwrap();

    let is_exp = Rc::new(Cell::new(initially_expanded));
    let is_exp_c = is_exp.clone();
    let body_c = body.clone();
    let panel_c = panel.clone();
    let chev_c = chevron.clone();

    let toggle_closure = Closure::wrap(Box::new(move |_e: Event| {
        let next = !is_exp_c.get();
        is_exp_c.set(next);

        let body_h: HtmlElement = body_c.clone().dyn_into().unwrap();
        let pan_h: HtmlElement = panel_c.clone().dyn_into().unwrap();
        let chev_h: HtmlElement = chev_c.clone().dyn_into().unwrap();

        if next {
            body_h.style().set_property("display", "").unwrap();
            let _ = panel_c.class_list().remove_1("collapsed");
            chev_h.set_text_content(Some("\u{25BE}")); // ▾
            if flex_grow {
                let _ = pan_h.style().set_property("flex", "1");
                let _ = pan_h.style().set_property("overflow", "hidden");
            }
        } else {
            body_h.style().set_property("display", "none").unwrap();
            let _ = panel_c.class_list().add_1("collapsed");
            chev_h.set_text_content(Some("\u{25B8}")); // ▸
            if flex_grow {
                let _ = pan_h.style().set_property("flex", "0 0 auto");
                let _ = pan_h.style().set_property("overflow", "visible");
            }
        }
    }) as Box<dyn FnMut(Event)>);

    header
        .add_event_listener_with_callback("click", toggle_closure.as_ref().unchecked_ref())
        .unwrap();
    toggle_closure.forget();

    panel
}

/// Build the right dock (aura tray + pulse stream + job center).
pub fn build_right_dock(document: &Document) -> Element {
    let dock = document.create_element("div").unwrap();
    dock.set_class_name("right-dock");
    dock.set_id("right-dock");

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
    let shacl_results = super::diagnostics::default_shacl_results();
    let passed = shacl_results.iter().filter(|r| r.conformant).count();
    let aura_badge = format!("{}/{} valid", passed, shacl_results.len());
    let aura_body = super::diagnostics::render_aura_tray(document, &shacl_results);
    let aura_panel = create_collapsible_dock_panel(
        document,
        "Aura Tray",
        Some(&aura_badge),
        aura_body,
        true,  // initially expanded
        false, // flex_grow
    );
    content.append_child(&aura_panel).unwrap();

    // 2. Pulse Stream — wired to diagnostics module
    let pulse_events = super::diagnostics::default_pulse_events();
    let pulse_badge = format!("{} events", pulse_events.len());
    let pulse_body = super::diagnostics::render_pulse_stream(document, &pulse_events);
    let pulse_panel = create_collapsible_dock_panel(
        document,
        "Pulse Stream",
        Some(&pulse_badge),
        pulse_body,
        true, // initially expanded
        true, // flex_grow (occupies remaining height)
    );
    content.append_child(&pulse_panel).unwrap();

    // 3. Job Center — background job queue
    let jobs = super::diagnostics::default_jobs();
    let active_jobs = jobs
        .iter()
        .filter(|j| j.status == super::diagnostics::JobStatus::Running)
        .count();
    let jobs_badge = format!("{} running", active_jobs);
    let job_body = super::diagnostics::render_job_body(document, &jobs);
    let job_panel = create_collapsible_dock_panel(
        document,
        "Job Center",
        Some(&jobs_badge),
        job_body,
        true,  // initially expanded
        false, // flex_grow
    );
    content.append_child(&job_panel).unwrap();

    // 4. Live VibeScript UI Host (<q-vibe-ui>)
    let vibe_ui_script = super::vibe_ui::default_aura_tray_vibe_script();
    let vibe_ui_host = super::vibe_ui::render_live_vibe_ui(document, vibe_ui_script);
    let vibe_ui_panel = create_collapsible_dock_panel(
        document,
        "Vibe UI Live Engine",
        Some("Hot-Reload"),
        vibe_ui_host,
        false, // collapsed by default
        false, // flex_grow
    );
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

/// Build the bottom status bar.
pub fn build_bottom_statusbar(document: &Document) -> Element {
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("bottom-statusbar");

    // Left section
    let left = document.create_element("div").unwrap();
    left.set_class_name("statusbar-section");

    let graph = document.create_element("div").unwrap();
    graph.set_class_name("statusbar-item");
    let g_label = document.create_element("span").unwrap();
    g_label.set_class_name("statusbar-label");
    g_label.set_text_content(Some("Graph:"));
    let g_val = document.create_element("span").unwrap();
    g_val.set_class_name("statusbar-value");
    g_val.set_text_content(Some("catchment_sites"));
    graph.append_child(&g_label).unwrap();
    graph.append_child(&g_val).unwrap();
    left.append_child(&graph).unwrap();

    let merkle = document.create_element("div").unwrap();
    merkle.set_class_name("statusbar-item");
    let m_label = document.create_element("span").unwrap();
    m_label.set_class_name("statusbar-label");
    m_label.set_text_content(Some("Merkle:"));
    let m_val = document.create_element("span").unwrap();
    m_val.set_class_name("statusbar-value");
    m_val.set_text_content(Some("0x8f...a42"));
    merkle.append_child(&m_label).unwrap();
    merkle.append_child(&m_val).unwrap();
    left.append_child(&merkle).unwrap();

    bar.append_child(&left).unwrap();

    // Right section
    let right = document.create_element("div").unwrap();
    right.set_class_name("statusbar-section");

    let gas = document.create_element("div").unwrap();
    gas.set_class_name("statusbar-item");
    let g_label = document.create_element("span").unwrap();
    g_label.set_class_name("statusbar-label");
    g_label.set_text_content(Some("Gas:"));
    let g_val = document.create_element("span").unwrap();
    g_val.set_class_name("statusbar-gas");
    g_val.set_text_content(Some("984,500 / 1M"));
    gas.append_child(&g_label).unwrap();
    gas.append_child(&g_val).unwrap();
    right.append_child(&gas).unwrap();

    let strata = document.create_element("div").unwrap();
    strata.set_class_name("statusbar-item");
    let s_label = document.create_element("span").unwrap();
    s_label.set_class_name("statusbar-label");
    s_label.set_text_content(Some("Strata:"));
    let s_val = document.create_element("span").unwrap();
    s_val.set_class_name("statusbar-value");
    s_val.set_text_content(Some("social, legal"));
    strata.append_child(&s_label).unwrap();
    strata.append_child(&s_val).unwrap();
    right.append_child(&strata).unwrap();

    bar.append_child(&right).unwrap();
    bar
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_toolbox_families_count() {
        let families = family_order();
        assert!(families.len() >= 12, "Expected at least 12 master toolbox families, got {}", families.len());
        let ids: Vec<&str> = families.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"epistemic"));
        assert!(ids.contains(&"authoring"));
        assert!(ids.contains(&"sheet"));
        assert!(ids.contains(&"graphics"));
        assert!(ids.contains(&"spatial"));
        assert!(ids.contains(&"audio"));
        assert!(ids.contains(&"code"));
        assert!(ids.contains(&"erp"));
        assert!(ids.contains(&"mail"));
        assert!(ids.contains(&"lab"));
        assert!(ids.contains(&"ai"));
        assert!(ids.contains(&"governance"));
        assert!(ids.contains(&"sdn"));
    }

    #[test]
    fn test_dock_position_conversions() {
        assert_eq!(DockPosition::from_str("top"), DockPosition::Top);
        assert_eq!(DockPosition::from_str("right"), DockPosition::Right);
        assert_eq!(DockPosition::from_str("bottom"), DockPosition::Bottom);
        assert_eq!(DockPosition::from_str("left"), DockPosition::Left);
        assert_eq!(DockPosition::from_str("invalid"), DockPosition::Left);

        assert_eq!(DockPosition::Top.as_str(), "top");
        assert_eq!(DockPosition::Right.as_str(), "right");
        assert_eq!(DockPosition::Bottom.as_str(), "bottom");
        assert_eq!(DockPosition::Left.as_str(), "left");
    }
}
