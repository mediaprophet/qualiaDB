//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Domain widgets attached to a tool-chain for the flyout.

use crate::browser::tool_widgets::ToolWidget;

use super::model::ToolView;

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
                    ("".into(), "Choose an engine…".into()),
                    ("framingham".into(), "Framingham 10-Year CVD Risk".into()),
                    ("cha2ds2_vasc".into(), "CHA₂DS₂-VASc Stroke Risk".into()),
                    ("score2".into(), "SCORE2 European Mortality".into()),
                    ("smiles_mol".into(), "SMILES Molecular Weight & LogP".into()),
                    ("protein_sw".into(), "Smith-Waterman Protein Align".into()),
                ],
                default_val: "".into(),
            });
        }

        _ => {}
    }

    // Append standard Tool buttons for all tools in this chain
    for tool in tools {
        widgets.push(ToolWidget::Button {
            id: tool.id.clone(),
            label: tool.label.clone(),
            icon: super::glyphs::tool_glyph(&tool.icon).to_string(),
            kind_badge: super::glyphs::kind_label(tool.kind).to_string(),
            action: tool.action.to_string(),
        });
    }

    widgets
}
