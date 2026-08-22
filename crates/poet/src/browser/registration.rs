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
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
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
            label: "Office Toolbox".into(),
            icon: "office".into(),
            ontology_prefix: "hm".into(),
            description: "Documents, ontologies, slides.".into(),
            enabled_by_default: true,
            family: "authoring".into(),
        },
        vec![ToolChain::new(
            ToolChainMetadata {
                id: "office:containers".into(),
                label: "Containers".into(),
                icon: "containers".into(),
                description: "Place office containers.".into(),
            },
            tools,
        )],
    ));
}

fn register_image_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "image:place_media".into(),
                label: "+ Media".into(),
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
            label: "Image Toolbox".into(),
            icon: "image".into(),
            ontology_prefix: "hm".into(),
            description: "Media, markers, heatmaps.".into(),
            enabled_by_default: true,
            family: "media".into(),
        },
        vec![ToolChain::new(
            ToolChainMetadata {
                id: "image:tools".into(),
                label: "Image Tools".into(),
                icon: "tools".into(),
                description: "Image and media tools.".into(),
            },
            tools,
        )],
    ));
}

fn register_sheet_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "sheet:place_sheet".into(),
                label: "+ Sheet".into(),
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
                label: "Import".into(),
                icon: "import".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:write".into()),
                ontology_prefix: "hm".into(),
                description: "Import data into the active sheet.".into(),
            },
            ActionType::Mutate,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "sheet".into(),
            label: "Sheet Toolbox".into(),
            icon: "sheet".into(),
            ontology_prefix: "hm".into(),
            description: "Spreadsheets, import, resonance.".into(),
            enabled_by_default: true,
            family: "authoring".into(),
        },
        vec![ToolChain::new(
            ToolChainMetadata {
                id: "sheet:tools".into(),
                label: "Sheet Tools".into(),
                icon: "tools".into(),
                description: "Spreadsheet tools.".into(),
            },
            tools,
        )],
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
                label: "+ 3D".into(),
                icon: "3d".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "hm".into(),
                description: "Place a 3D viewport container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:pin".into(),
                label: "Pin".into(),
                icon: "pin".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:annotate".into()),
                ontology_prefix: "geo".into(),
                description: "Drop a pin on the active map.".into(),
            },
            ActionType::Annotate,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:track".into(),
                label: "Track".into(),
                icon: "track".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "geo".into(),
                description: "Track an agent on the active map.".into(),
            },
            ActionType::Query,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "spatial".into(),
            label: "Spatial Toolbox".into(),
            icon: "spatial".into(),
            ontology_prefix: "geo".into(),
            description: "Maps, 3D, portals, pins, tracks.".into(),
            enabled_by_default: true,
            family: "media".into(),
        },
        vec![ToolChain::new(
            ToolChainMetadata {
                id: "spatial:tools".into(),
                label: "Spatial Tools".into(),
                icon: "tools".into(),
                description: "Spatial and GIS tools.".into(),
            },
            tools,
        )],
    ));
}

fn register_communication_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "comm:place_social".into(),
                label: "+ Social".into(),
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
                label: "+ WebRTC".into(),
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
                label: "+ Webview".into(),
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
            label: "Communication Toolbox".into(),
            icon: "comm".into(),
            ontology_prefix: "comm".into(),
            description: "Social, WebRTC, webview.".into(),
            enabled_by_default: true,
            family: "communication".into(),
        },
        vec![ToolChain::new(
            ToolChainMetadata {
                id: "comm:containers".into(),
                label: "Containers".into(),
                icon: "containers".into(),
                description: "Communication containers.".into(),
            },
            tools,
        )],
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
                label: "Fiduciary Sign".into(),
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
            label: "Rights Toolbox".into(),
            icon: "rights".into(),
            ontology_prefix: "rights".into(),
            description: "Authors group, fiduciary, DID sign.".into(),
            enabled_by_default: true,
            family: "governance".into(),
        },
        vec![ToolChain::new(
            ToolChainMetadata {
                id: "rights:tools".into(),
                label: "Rights Tools".into(),
                icon: "tools".into(),
                description: "Fiduciary and rights tools.".into(),
            },
            tools,
        )],
    ));
}

fn register_health_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "health:place_health".into(),
                label: "+ Health".into(),
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
                label: "Pathology".into(),
                icon: "pathology".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("capability:invoke".into()),
                ontology_prefix: "health".into(),
                description: "Run pathology analysis.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "health:anatomy_10d".into(),
                label: "10D Anatomy".into(),
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
            label: "Health Toolbox".into(),
            icon: "health".into(),
            ontology_prefix: "health".into(),
            description: "Health, pathology, 10D anatomy.".into(),
            enabled_by_default: false,
            family: "life".into(),
        },
        vec![ToolChain::new(
            ToolChainMetadata {
                id: "health:tools".into(),
                label: "Health Tools".into(),
                icon: "tools".into(),
                description: "Health and anatomy tools.".into(),
            },
            tools,
        )],
    ));
}

fn register_code_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "code:place_vibe".into(),
                label: "+ Vibe Cell".into(),
                icon: "vibe".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("vibe:diagnose".into()),
                ontology_prefix: "vibe".into(),
                description: "Place a VibeScript cell container.".into(),
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
            label: "Code Toolbox".into(),
            icon: "code".into(),
            ontology_prefix: "vibe".into(),
            description: "Vibe cells, quin.statement, requires[].".into(),
            enabled_by_default: true,
            family: "authoring".into(),
        },
        vec![ToolChain::new(
            ToolChainMetadata {
                id: "code:tools".into(),
                label: "Code Tools".into(),
                icon: "tools".into(),
                description: "VibeScript and code tools.".into(),
            },
            tools,
        )],
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
                label: "Sentinel".into(),
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
                label: "Triad".into(),
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
            label: "AI Toolbox".into(),
            icon: "ai".into(),
            ontology_prefix: "ai".into(),
            description: "Co-author, extractor, sentinel, triad.".into(),
            enabled_by_default: true,
            family: "intelligence".into(),
        },
        vec![ToolChain::new(
            ToolChainMetadata {
                id: "ai:tools".into(),
                label: "AI Tools".into(),
                icon: "tools".into(),
                description: "AI and ML tools.".into(),
            },
            tools,
        )],
    ));
}
