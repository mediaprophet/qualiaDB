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

    let threed_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:threed_add_object".into(),
                label: "Add object".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ThreeD.add_object".into()),
                ontology_prefix: "hm".into(),
                description: "Add a named 3D object via ThreeD.add_object.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:threed_set_transform".into(),
                label: "Set transform".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ThreeD.set_transform".into()),
                ontology_prefix: "hm".into(),
                description: "Set object position from three surface numbers.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:threed_set_material".into(),
                label: "Set material".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ThreeD.set_material".into()),
                ontology_prefix: "hm".into(),
                description: "Assign a material_id to a 3D object.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:threed_add_camera".into(),
                label: "Add camera".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ThreeD.add_camera".into()),
                ontology_prefix: "hm".into(),
                description: "Add a camera with numeric fov (degrees).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:threed_add_light".into(),
                label: "Add light".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ThreeD.add_light".into()),
                ontology_prefix: "hm".into(),
                description: "Add a point/directional/spot/ambient light.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:threed_add_rig".into(),
                label: "Add rig".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ThreeD.add_rig".into()),
                ontology_prefix: "hm".into(),
                description: "Add a named animation/control rig.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:threed_add_animation".into(),
                label: "Add animation".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ThreeD.add_animation".into()),
                ontology_prefix: "hm".into(),
                description: "Add an animation clip with numeric duration (seconds).".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:threed_set_mesh".into(),
                label: "Set mesh".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("ThreeD.set_mesh".into()),
                ontology_prefix: "hm".into(),
                description: "Assign a mesh_id to a 3D object.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    let scene_tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_lerp_camera".into(),
                label: "Lerp camera".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.lerp_camera".into()),
                ontology_prefix: "hm".into(),
                description: "Interpolate orbit camera yaw/pitch/zoom via Scene.lerp_camera.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_camera_frame_node".into(),
                label: "Frame node".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.camera_frame_node".into()),
                ontology_prefix: "hm".into(),
                description: "Orbit params that frame a world-space node via Scene.camera_frame_node."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_smooth_damp".into(),
                label: "Smooth damp".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.smooth_damp".into()),
                ontology_prefix: "hm".into(),
                description: "Smoothly damp a scalar toward a target via Scene.smooth_damp.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_smooth_damp_vec3".into(),
                label: "Smooth damp vec3".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.smooth_damp_vec3".into()),
                ontology_prefix: "hm".into(),
                description: "Smoothly damp a 3D vector toward a target via Scene.smooth_damp_vec3."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_ik_look_at".into(),
                label: "IK look-at".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.ik_look_at".into()),
                ontology_prefix: "hm".into(),
                description: "Aim a joint chain at a target via Scene.ik_look_at.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_ik_ccd".into(),
                label: "IK CCD".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.ik_ccd".into()),
                ontology_prefix: "hm".into(),
                description: "CCD inverse kinematics via Scene.ik_ccd.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_set_render_budget".into(),
                label: "Render budget".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.set_render_budget".into()),
                ontology_prefix: "hm".into(),
                description: "Set per-frame render budget_ms via Scene.set_render_budget.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_set_clear_colour".into(),
                label: "Clear colour".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.set_clear_colour".into()),
                ontology_prefix: "hm".into(),
                description: "Set viewport clear RGBA via Scene.set_clear_colour.".into(),
            },
            ActionType::Invoke,
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
                    id: "spatial:threed".into(),
                    label: "Live ThreeD".into(),
                    icon: "3d".into(),
                    description: "Curated ThreeD.* object, transform, camera, light, and mesh binds."
                        .into(),
                },
                threed_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "spatial:scene".into(),
                    label: "Live Scene".into(),
                    icon: "3d".into(),
                    description:
                        "Curated Scene.* camera, damp, IK, budget, and clear-colour numeric binds."
                            .into(),
                },
                scene_tools,
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
