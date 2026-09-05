//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_spatial_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:place_map".into(),
                label: "+ Map".into(),
                icon: "map".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "geo".into(),
                description: "Place a GIS map container.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:place_dual_studio".into(),
                label: "+ Dual Studio".into(),
                icon: "studio".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "hm".into(),
                description: "Place Dual Studio (VibeScript + GPU) on the active manifold.".into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:place_scene_view".into(),
                label: "+ Scene session".into(),
                icon: "3d".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "hm".into(),
                description: "Place a Scene session inspector. GPU frames live in Dual Studio."
                    .into(),
            },
            ActionType::Query,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:place_3d".into(),
                label: "+ 3D Viewport".into(),
                icon: "3d".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
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
                capability_scope: None,
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
                capability_scope: None,
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
            description: "Dual Studio, Scene sessions, GIS maps, and spatial tracking — tools on POET manifolds, not a nested DCC.".into(),
            enabled_by_default: true,
            family: "spatial".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "spatial:viewport".into(),
                    label: "3D Cameras & Shaders".into(),
                    icon: "3d".into(),
                    description: "Select perspective/orthographic projections and WGSL pipelines."
                        .into(),
                },
                vec![
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "spatial:camera_reset".into(),
                            label: "Reset camera".into(),
                            icon: "3d".into(),
                            kind: ToolKind::RunAction,
                            capability_scope: None,
                            ontology_prefix: "hm".into(),
                            description: "Reset yaw/pitch/zoom on the selected map or 3D surface.".into(),
                        },
                        ActionType::Mutate,
                    )),
                    Box::new(SimpleTool::new(
                        ToolMetadata {
                            id: "spatial:orbit_preview".into(),
                            label: "Orbit preview".into(),
                            icon: "3d".into(),
                            kind: ToolKind::Query,
                            capability_scope: Some("Animation.evaluate_preset".into()),
                            ontology_prefix: "hm".into(),
                            description: "In-process Animation.evaluate_preset orbit_spin sample.".into(),
                        },
                        ActionType::Query,
                    )),
                ],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "spatial:tools".into(),
                    label: "GIS Maps & Tracking".into(),
                    icon: "tools".into(),
                    description: "Place Dual Studio, Scene sessions, GIS maps, and spatial pins.".into(),
                },
                tools,
            ),
        ],
    ));
}
