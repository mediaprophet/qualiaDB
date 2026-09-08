//! Part of poet browser toolbox registration.

use super::*;

fn video_live_tool(
    id: &'static str,
    label: &'static str,
    scope: &'static str,
    description: &'static str,
) -> Box<dyn crate::tool_chest::core::tool::Tool> {
    Box::new(SimpleTool::new(
        ToolMetadata {
            id: id.into(),
            label: label.into(),
            icon: "media".into(),
            kind: ToolKind::RunAction,
            capability_scope: Some(scope.into()),
            ontology_prefix: "hm".into(),
            description: description.into(),
        },
        ActionType::Invoke,
    ))
}

fn image_edit_tool(
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    scope: &'static str,
    description: &'static str,
) -> Box<dyn crate::tool_chest::core::tool::Tool> {
    Box::new(SimpleTool::new(
        ToolMetadata {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            kind: ToolKind::RunAction,
            capability_scope: Some(scope.into()),
            ontology_prefix: "hm".into(),
            description: description.into(),
        },
        ActionType::Invoke,
    ))
}

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

    let edit_live: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        image_edit_tool(
            "image:edit_new",
            "New document",
            "media",
            "Image.new",
            "Create a new image document (default 1920×1080) on this surface.",
        ),
        image_edit_tool(
            "image:edit_add_layer",
            "Add layer",
            "media",
            "Image.add_layer",
            "Add a named layer to the selected image document.",
        ),
        image_edit_tool(
            "image:edit_remove_layer",
            "Remove layer",
            "media",
            "Image.remove_layer",
            "Remove a layer by index from the selected image document.",
        ),
        image_edit_tool(
            "image:edit_set_pixel",
            "Set pixel",
            "marker",
            "Image.set_pixel",
            "Set one pixel colour on the selected image document.",
        ),
        image_edit_tool(
            "image:edit_fill",
            "Fill",
            "heatmap",
            "Image.fill",
            "Fill the selected image document with an RGB colour.",
        ),
        image_edit_tool(
            "image:edit_brush",
            "Brush",
            "marker",
            "Image.brush",
            "Stroke a brush path on the selected image document.",
        ),
        image_edit_tool(
            "image:edit_apply_filter",
            "Apply filter",
            "heatmap",
            "Image.apply_filter",
            "Apply a filter (default blur) to the selected image document.",
        ),
        image_edit_tool(
            "image:edit_set_opacity",
            "Set opacity",
            "heatmap",
            "Image.set_opacity",
            "Set layer opacity on the selected image document.",
        ),
        image_edit_tool(
            "image:edit_set_blend_mode",
            "Blend mode",
            "heatmap",
            "Image.set_blend_mode",
            "Set layer blend mode (default normal) on the selected image document.",
        ),
        image_edit_tool(
            "image:edit_set_visible",
            "Set visible",
            "media",
            "Image.set_visible",
            "Show or hide a layer on the selected image document.",
        ),
        image_edit_tool(
            "image:edit_set_mask",
            "Set mask",
            "tools",
            "Image.set_mask",
            "Set a rectangular mask on the selected image document.",
        ),
        image_edit_tool(
            "image:edit_clear_mask",
            "Clear mask",
            "tools",
            "Image.clear_mask",
            "Clear the mask on the selected image document.",
        ),
        image_edit_tool(
            "image:edit_composite",
            "Composite",
            "media",
            "Image.composite",
            "Composite layers of the selected image document to RGBA8.",
        ),
        image_edit_tool(
            "image:edit_add_selection",
            "Add selection",
            "marker",
            "Image.add_selection",
            "Add a named selection to the selected image document.",
        ),
        image_edit_tool(
            "image:edit_clear_selections",
            "Clear selections",
            "marker",
            "Image.clear_selections",
            "Clear all selections on the selected image document.",
        ),
    ];

    let video_live: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        video_live_tool(
            "video:live_new_project",
            "New project",
            "Video.new_project",
            "Create a video project via Video.new_project.",
        ),
        video_live_tool(
            "video:live_add_track",
            "Add track",
            "Video.add_track",
            "Add a named track via Video.add_track.",
        ),
        video_live_tool(
            "video:live_add_clip",
            "Add clip",
            "Video.add_clip",
            "Add a source clip via Video.add_clip.",
        ),
        video_live_tool(
            "video:live_trim_clip",
            "Trim clip",
            "Video.trim_clip",
            "Trim in/out points via Video.trim_clip.",
        ),
        video_live_tool(
            "video:live_set_speed",
            "Set speed",
            "Video.set_speed",
            "Set playback speed via Video.set_speed.",
        ),
        video_live_tool(
            "video:live_colour_grade",
            "Colour grade",
            "Video.colour_grade",
            "Set brightness, contrast, and saturation via Video.colour_grade.",
        ),
        video_live_tool(
            "video:live_add_transition",
            "Add transition",
            "Video.add_transition",
            "Add a transition via Video.add_transition.",
        ),
        video_live_tool(
            "video:live_set_render_format",
            "Render format",
            "Video.set_render_format",
            "Set the render format via Video.set_render_format.",
        ),
        video_live_tool(
            "video:live_set_render_bitrate",
            "Render bitrate",
            "Video.set_render_bitrate",
            "Set the render bitrate via Video.set_render_bitrate.",
        ),
        video_live_tool(
            "video:live_remove_clip",
            "Remove clip",
            "Video.remove_clip",
            "Remove a clip via Video.remove_clip.",
        ),
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
            ToolChain::new(
                ToolChainMetadata {
                    id: "image:edit".into(),
                    label: "Live image edit".into(),
                    icon: "media".into(),
                    description:
                        "Curated Image.* binds — document, layers, pixels, filter, mask, composite."
                            .into(),
                },
                edit_live,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "video:live".into(),
                    label: "Live video edit".into(),
                    icon: "media".into(),
                    description:
                        "Curated Video.* project, track, clip, grade, transition, and render binds."
                            .into(),
                },
                video_live,
            ),
        ],
    ));
}
