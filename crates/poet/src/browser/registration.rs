//! Toolbox registration — populates the Registry at startup.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Registers all 10 toolboxes with their tool-chains and tools,
//! plus all manifold seeds, into a shared Registry.

use crate::tool_chest::core::intent_bus::ActionType;
use crate::tool_chest::core::registry::Registry;
use crate::tool_chest::core::tool::{SimpleTool, ToolKind, ToolMetadata};
use crate::tool_chest::core::tool_chain::{ToolChain, ToolChainMetadata};
use crate::tool_chest::core::toolbox::{Toolbox, ToolboxMetadata};

use crate::tool_chest::manifolds;

/// Build a fully populated Registry with all toolboxes and manifold seeds.
pub fn build_registry() -> Registry {
    let mut reg = Registry::new();

    register_epistemic_toolbox(&mut reg);
    register_office_toolbox(&mut reg);
    register_image_toolbox(&mut reg);
    register_sheet_toolbox(&mut reg);
    register_spatial_toolbox(&mut reg);
    register_communication_toolbox(&mut reg);
    register_rights_toolbox(&mut reg);
    register_health_toolbox(&mut reg);
    register_code_toolbox(&mut reg);
    register_ai_toolbox(&mut reg);

    // Register all manifold seeds
    for seed in manifolds::all_seeds() {
        reg.register_manifold(seed);
    }

    reg
}

// ---------------------------------------------------------------------------
// Toolbox registrations
// ---------------------------------------------------------------------------

fn register_epistemic_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "epistemic:tag_objective".into(),
                label: "Tag Objective".into(),
                icon: "objective".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:annotate".into()),
                ontology_prefix: "epi".into(),
                description: "Tag selected node as objective epistemic modality.".into(),
            },
            ActionType::Annotate,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "epistemic:tag_subjective".into(),
                label: "Tag Subjective".into(),
                icon: "subjective".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:annotate".into()),
                ontology_prefix: "epi".into(),
                description: "Tag selected node as subjective epistemic modality.".into(),
            },
            ActionType::Annotate,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "epistemic:tag_intersubjective".into(),
                label: "Tag Intersubjective".into(),
                icon: "intersubjective".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:annotate".into()),
                ontology_prefix: "epi".into(),
                description: "Tag selected node as intersubjective epistemic modality.".into(),
            },
            ActionType::Annotate,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "epistemic:tag_normative".into(),
                label: "Tag Normative".into(),
                icon: "normative".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:annotate".into()),
                ontology_prefix: "epi".into(),
                description: "Tag selected node as normative epistemic modality.".into(),
            },
            ActionType::Annotate,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "epistemic".into(),
            label: "Epistemic Toolbox".into(),
            icon: "epistemic".into(),
            ontology_prefix: "epi".into(),
            description: "Tag nodes with epistemic modalities (objective, subjective, intersubjective, normative).".into(),
            enabled_by_default: true,
            family: "epistemic".into(),
        },
        vec![ToolChain::new(
            ToolChainMetadata {
                id: "epistemic:modalities".into(),
                label: "Epistemic Modalities".into(),
                icon: "modalities".into(),
                description: "Set the epistemic modality of a selected node.".into(),
            },
            tools,
        )],
    ));
}

fn register_office_toolbox(reg: &mut Registry) {
    let container_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "office:place_doc".into(),
                label: "+ Document".into(),
                icon: "doc".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "hm".into(),
                description: "Place a rich text document container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "office:place_ontology".into(),
                label: "+ Ontology".into(),
                icon: "ontology".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "ont".into(),
                description: "Place an ontology browser container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "office:place_slide".into(),
                label: "+ Slide".into(),
                icon: "slide".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "hm".into(),
                description: "Place a presentation slide container.".into(),
            },
            ActionType::Query,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "office".into(),
            label: "Word Processor & CML".into(),
            icon: "office".into(),
            ontology_prefix: "hm".into(),
            description: "Documents, typography, ontologies, and presentation slides.".into(),
            enabled_by_default: true,
            family: "authoring".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "office:typography".into(),
                    label: "Typography & Fonts".into(),
                    icon: "doc".into(),
                    description: "Select font family, size, styles (Bold/Italic/Code), and text colors.".into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "office:paragraph".into(),
                    label: "Paragraph & Headings".into(),
                    icon: "slide".into(),
                    description: "Configure heading levels, text alignment, and block formats.".into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "office:containers".into(),
                    label: "Office Containers".into(),
                    icon: "containers".into(),
                    description: "Place documents, ontology lenses, and slides on canvas.".into(),
                },
                container_tools,
            ),
        ],
    ));
}

fn register_image_toolbox(reg: &mut Registry) {
    let shape_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "image:place_media".into(),
                label: "+ Media Viewport".into(),
                icon: "media".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "hm".into(),
                description: "Place a media viewport container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "image:marker".into(),
                label: "Marker".into(),
                icon: "marker".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:annotate".into()),
                ontology_prefix: "hm".into(),
                description: "Place a marker on the active map.".into(),
            },
            ActionType::Annotate,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "image:heatmap".into(),
                label: "Heatmap".into(),
                icon: "heatmap".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "hm".into(),
                description: "Generate a heatmap overlay.".into(),
            },
            ActionType::Query,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "image".into(),
            label: "Graphics & Vector Drawing".into(),
            icon: "image".into(),
            ontology_prefix: "hm".into(),
            description: "Brushes, color palettes, vector geometry, and media viewports.".into(),
            enabled_by_default: true,
            family: "media".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "image:brushes".into(),
                    label: "Brushes & Stroke".into(),
                    icon: "media".into(),
                    description: "Select brush type, adjust brush stroke size and opacity.".into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "image:palette".into(),
                    label: "Color & Palette".into(),
                    icon: "heatmap".into(),
                    description: "Stroke & fill color pickers with preset swatches and geometry modes.".into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "image:tools".into(),
                    label: "Vector Shapes & Media".into(),
                    icon: "tools".into(),
                    description: "Place media viewports, markers, and heatmaps.".into(),
                },
                shape_tools,
            ),
        ],
    ));
}

fn register_sheet_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "sheet:place_sheet".into(),
                label: "+ Spreadsheet".into(),
                icon: "sheet".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "hm".into(),
                description: "Place a spreadsheet container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "sheet:import".into(),
                label: "Import Data".into(),
                icon: "import".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:write".into()),
                ontology_prefix: "hm".into(),
                description: "Import CSV/HCF data into active sheet.".into(),
            },
            ActionType::Mutate,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "sheet".into(),
            label: "Spreadsheet & Tensors".into(),
            icon: "sheet".into(),
            ontology_prefix: "hm".into(),
            description: "Spreadsheets, tensor arrays, formulas, and data import.".into(),
            enabled_by_default: true,
            family: "authoring".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "sheet:grid".into(),
                    label: "Tensor Dimensions & Formats".into(),
                    icon: "sheet".into(),
                    description: "Configure 1D/2D/3D/10D tensor dimensions and cell formatting.".into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "sheet:tools".into(),
                    label: "Spreadsheet Tools".into(),
                    icon: "tools".into(),
                    description: "Place spreadsheets and import external tabular data.".into(),
                },
                tools,
            ),
        ],
    ));
}

fn register_spatial_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:place_map".into(),
                label: "+ Map".into(),
                icon: "map".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "geo".into(),
                description: "Place a GIS map container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:place_3d".into(),
                label: "+ 3D Viewport".into(),
                icon: "3d".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "hm".into(),
                description: "Place a 3D WebGPU viewport container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:pin".into(),
                label: "Drop Pin".into(),
                icon: "pin".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:annotate".into()),
                ontology_prefix: "geo".into(),
                description: "Drop a geo-pin on the active map.".into(),
            },
            ActionType::Annotate,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:track".into(),
                label: "Track Agent".into(),
                icon: "track".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "geo".into(),
                description: "Track an agent trajectory on the map.".into(),
            },
            ActionType::Query,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "spatial".into(),
            label: "3D Spatial & Geospatial".into(),
            icon: "spatial".into(),
            ontology_prefix: "geo".into(),
            description: "3D scenes, GIS maps, WGSL shaders, and spatial tracking.".into(),
            enabled_by_default: true,
            family: "media".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "spatial:viewport".into(),
                    label: "3D Cameras & Shaders".into(),
                    icon: "3d".into(),
                    description: "Select perspective/orthographic projections and WGSL pipelines.".into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "spatial:tools".into(),
                    label: "GIS Maps & Tracking".into(),
                    icon: "tools".into(),
                    description: "Place 3D/GIS viewports and drop spatial pins.".into(),
                },
                tools,
            ),
        ],
    ));
}

fn register_communication_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "comm:place_social".into(),
                label: "+ Social Graph".into(),
                icon: "social".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "soc".into(),
                description: "Place a social graph container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "comm:place_webrtc".into(),
                label: "+ WebRTC Audio/Video".into(),
                icon: "webrtc".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: None,
                ontology_prefix: "comm".into(),
                description: "Place a WebRTC stream container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "comm:place_webview".into(),
                label: "+ Web Presence".into(),
                icon: "webview".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: None,
                ontology_prefix: "hm".into(),
                description: "Place a web frame container.".into(),
            },
            ActionType::Query,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "communication".into(),
            label: "Communication & Presence".into(),
            icon: "comm".into(),
            ontology_prefix: "comm".into(),
            description: "Pulse streams, social graphs, WebRTC, and web presence.".into(),
            enabled_by_default: true,
            family: "communication".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "comm:pulse".into(),
                    label: "Pulse Streams & Messaging".into(),
                    icon: "comm".into(),
                    description: "Select protocol and encryption tiers.".into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "comm:containers".into(),
                    label: "Presence Containers".into(),
                    icon: "containers".into(),
                    description: "Communication and streaming containers.".into(),
                },
                tools,
            ),
        ],
    ));
}

fn register_rights_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "rights:authors_group".into(),
                label: "Authors Group".into(),
                icon: "group".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:write".into()),
                ontology_prefix: "rights".into(),
                description: "Manage the authors group.".into(),
            },
            ActionType::Mutate,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "rights:fiduciary_sign".into(),
                label: "✍️ Sign Contract".into(),
                icon: "sign".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("capability:invoke".into()),
                ontology_prefix: "rights".into(),
                description: "Sign with fiduciary authority.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "rights:did_sign".into(),
                label: "DID Sign".into(),
                icon: "did".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("capability:invoke".into()),
                ontology_prefix: "rights".into(),
                description: "Sign with a DID.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "rights".into(),
            label: "Governance & Rights".into(),
            icon: "rights".into(),
            ontology_prefix: "rights".into(),
            description: "Fiduciary contracts, Hohfeldian rights, and DID signing.".into(),
            enabled_by_default: true,
            family: "governance".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "rights:fiduciary".into(),
                    label: "Fiduciary & Routing Lanes".into(),
                    icon: "rights".into(),
                    description: "Select privacy routing lane (00/01/10/11) and Hohfeldian modalities.".into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "rights:tools".into(),
                    label: "Identity & Signatures".into(),
                    icon: "tools".into(),
                    description: "Fiduciary and rights signing tools.".into(),
                },
                tools,
            ),
        ],
    ));
}

fn register_health_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "health:place_health".into(),
                label: "+ Health Vault".into(),
                icon: "health".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "health".into(),
                description: "Place a health vault container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "health:pathology".into(),
                label: "🔬 Pathology Assay".into(),
                icon: "pathology".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("capability:invoke".into()),
                ontology_prefix: "health".into(),
                description: "Run pathology and diagnostic assay.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "health:anatomy_10d".into(),
                label: "+ 10D Anatomy".into(),
                icon: "anatomy".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: None,
                ontology_prefix: "health".into(),
                description: "Place a 10D anatomy container.".into(),
            },
            ActionType::Query,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "health".into(),
            label: "Scientific & Clinical Lab".into(),
            icon: "health".into(),
            ontology_prefix: "health".into(),
            description: "Cardiovascular risk engines, SMILES chemistry, and 10D anatomy.".into(),
            enabled_by_default: false,
            family: "life".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "health:clinical".into(),
                    label: "Clinical Engines & Biomarkers".into(),
                    icon: "health".into(),
                    description: "Select CVD risk models and adjust blood pressure biomarkers.".into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "health:tools".into(),
                    label: "Lab Viewports & Assays".into(),
                    icon: "tools".into(),
                    description: "Place health vaults and run pathology assays.".into(),
                },
                tools,
            ),
        ],
    ));
}

fn register_code_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "code:place_vibe".into(),
                label: "+ VibeScript Cell".into(),
                icon: "vibe".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("vibe:diagnose".into()),
                ontology_prefix: "vibe".into(),
                description: "Place a reactive VibeScript cell container.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "code:quin_statement".into(),
                label: "quin.statement".into(),
                icon: "quin".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:write".into()),
                ontology_prefix: "vibe".into(),
                description: "Construct a quin.statement.".into(),
            },
            ActionType::Mutate,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "code".into(),
            label: "Code IDE & Vibe REPL".into(),
            icon: "code".into(),
            ontology_prefix: "vibe".into(),
            description: "VibeScript 0.1, WebGPU WGSL, SPARQL, and reactive AST cells.".into(),
            enabled_by_default: true,
            family: "authoring".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "code:repl".into(),
                    label: "Runtime & Language Dialects".into(),
                    icon: "code".into(),
                    description: "Select language dialect (Vibe/WGSL/Turtle) and configure gas budget.".into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "code:tools".into(),
                    label: "IDE Cells & Statements".into(),
                    icon: "tools".into(),
                    description: "Place VibeScript cells and construct quin statements.".into(),
                },
                tools,
            ),
        ],
    ));
}

fn register_ai_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:co_author".into(),
                label: "Co-Author".into(),
                icon: "coauthor".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("capability:invoke".into()),
                ontology_prefix: "ai".into(),
                description: "Invoke co-author assistance.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:extractor".into(),
                label: "Extractor".into(),
                icon: "extractor".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "ai".into(),
                description: "Extract entities from text.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:sentinel".into(),
                label: "Sentinel Guard".into(),
                icon: "sentinel".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("capability:invoke".into()),
                ontology_prefix: "ai".into(),
                description: "Invoke sentinel monitoring.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "ai:triad".into(),
                label: "+ Triad Viewport".into(),
                icon: "triad".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: None,
                ontology_prefix: "ai".into(),
                description: "Place a triad (q42+p64+d10) container.".into(),
            },
            ActionType::Query,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "ai".into(),
            label: "AI Co-Pilot & Sentinel".into(),
            icon: "ai".into(),
            ontology_prefix: "ai".into(),
            description: "Resident GGUF LLMs, Epistemic Halo guard, and Triad execution.".into(),
            enabled_by_default: true,
            family: "intelligence".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "ai:copilot".into(),
                    label: "Sentinel Guard & Model".into(),
                    icon: "ai".into(),
                    description: "Select resident GGUF model, halo confidence threshold, and temperature.".into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "ai:tools".into(),
                    label: "Co-Pilot Capabilities".into(),
                    icon: "tools".into(),
                    description: "Invoke co-authoring, text extraction, and triad viewports.".into(),
                },
                tools,
            ),
        ],
    ));
}
