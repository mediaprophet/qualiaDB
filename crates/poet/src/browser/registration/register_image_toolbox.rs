//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_image_toolbox(reg: &mut Registry) {
    let shape_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "image:place_media".into(),
                label: "+ Media Viewport".into(),
                icon: "media".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
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
                capability_scope: None,
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
                capability_scope: None,
                ontology_prefix: "hm".into(),
                description: "Generate a heatmap overlay.".into(),
            },
            ActionType::Query,
        )),
    ];

    let vision_live: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "image:equalize_hist".into(),
                label: "Equalize tones".into(),
                icon: "heatmap".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputerVision.equalize_hist".into()),
                ontology_prefix: "hm".into(),
                description: "Spread greyscale tones more evenly on the selected picture."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "image:rgb_to_gray".into(),
                label: "Greyscale from colour".into(),
                icon: "media".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputerVision.rgb_to_gray".into()),
                ontology_prefix: "hm".into(),
                description: "Convert RGB pixels on this surface to greyscale.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "image:dhash".into(),
                label: "Difference hash".into(),
                icon: "marker".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputerVision.dhash".into()),
                ontology_prefix: "hm".into(),
                description: "Compute a perceptual difference hash from greyscale pixels."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "image:hamming_distance".into(),
                label: "Hash distance".into(),
                icon: "tools".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputerVision.hamming_distance".into()),
                ontology_prefix: "hm".into(),
                description: "Count differing bits between two perceptual hashes on this surface."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "image:cosine_similarity".into(),
                label: "Embedding similarity".into(),
                icon: "tools".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ComputerVision.cosine_similarity".into()),
                ontology_prefix: "hm".into(),
                description: "Cosine similarity of two embedding vectors on this surface."
                    .into(),
            },
            ActionType::Invoke,
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
            family: "graphics".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "image:brushes".into(),
                    label: "Brushes & Stroke".into(),
                    icon: "media".into(),
                    description: "Select brush type, adjust brush stroke size and opacity.".into(),
                },
                vec![
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "image:brush_stroke".into(),
                            label: "Stroke".into(),
                            icon: "marker".into(),
                            kind: ToolKind::RunAction,
                            capability_scope: None,
                            ontology_prefix: "hm".into(),
                            description: "Apply a visible stroke outline to the selected surface."
                                .into(),
                        },
                        ActionType::Mutate,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "image:brush_clear".into(),
                            label: "Clear stroke".into(),
                            icon: "marker".into(),
                            kind: ToolKind::RunAction,
                            capability_scope: None,
                            ontology_prefix: "hm".into(),
                            description: "Remove the stroke outline from the selected surface."
                                .into(),
                        },
                        ActionType::Mutate,
                    )),
                ],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "image:palette".into(),
                    label: "Color & Palette".into(),
                    icon: "heatmap".into(),
                    description:
                        "Stroke & fill color pickers with preset swatches and geometry modes."
                            .into(),
                },
                vec![
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "image:fill_warm".into(),
                            label: "Warm fill".into(),
                            icon: "heatmap".into(),
                            kind: ToolKind::RunAction,
                            capability_scope: None,
                            ontology_prefix: "hm".into(),
                            description: "Apply a warm fill token to the selected surface.".into(),
                        },
                        ActionType::Mutate,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "image:fill_cool".into(),
                            label: "Cool fill".into(),
                            icon: "heatmap".into(),
                            kind: ToolKind::RunAction,
                            capability_scope: None,
                            ontology_prefix: "hm".into(),
                            description: "Apply a cool fill token to the selected surface.".into(),
                        },
                        ActionType::Mutate,
                    )),
                ],
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
            ToolChain::new(
                ToolChainMetadata {
                    id: "image:vision".into(),
                    label: "Live computer vision".into(),
                    icon: "media".into(),
                    description:
                        "Curated ComputerVision.* binds — equalize, greyscale, hashes, similarity."
                            .into(),
                },
                vision_live,
            ),
        ],
    ));
}
