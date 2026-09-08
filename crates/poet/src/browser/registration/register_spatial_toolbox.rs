//! Part of poet browser toolbox registration.

use super::*;

fn dmx_live_tool(
    id: &'static str,
    label: &'static str,
    scope: &'static str,
    description: &'static str,
) -> Box<dyn crate::tool_chest::core::tool::Tool> {
    Box::new(SimpleTool::new(
        ToolMetadata {
            id: id.into(),
            label: label.into(),
            icon: "3d".into(),
            kind: ToolKind::RunAction,
            capability_scope: Some(scope.into()),
            ontology_prefix: "hm".into(),
            description: description.into(),
        },
        ActionType::Invoke,
    ))
}

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
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_create".into(),
                label: "Create scene".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.create".into()),
                ontology_prefix: "hm".into(),
                description: "Create a named scene graph via Scene.create.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_add_node".into(),
                label: "Add node".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.add_node".into()),
                ontology_prefix: "hm".into(),
                description: "Add a numbered node at x, y, z via Scene.add_node.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_set_transform".into(),
                label: "Set transform".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.set_transform".into()),
                ontology_prefix: "hm".into(),
                description: "Set a node's position, rotation, and scale via Scene.set_transform."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_set_mesh".into(),
                label: "Set mesh".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.set_mesh".into()),
                ontology_prefix: "hm".into(),
                description: "Assign a mesh IRI to a node via Scene.set_mesh.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_add_camera".into(),
                label: "Add camera".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.add_camera".into()),
                ontology_prefix: "hm".into(),
                description: "Add a camera with position and fov via Scene.add_camera.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_render".into(),
                label: "Render scene".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.render".into()),
                ontology_prefix: "hm".into(),
                description: "Request a scene render via Scene.render.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_set_viewport".into(),
                label: "Set viewport".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.set_viewport".into()),
                ontology_prefix: "hm".into(),
                description: "Set viewport width, height, and format via Scene.set_viewport."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_capture_frame".into(),
                label: "Capture frame".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.capture_frame".into()),
                ontology_prefix: "hm".into(),
                description: "Request a viewport frame capture via Scene.capture_frame.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_add_light".into(),
                label: "Add light".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.add_light".into()),
                ontology_prefix: "hm".into(),
                description: "Add a point, directional, spot, or ambient light via Scene.add_light."
                    .into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_link_semantic".into(),
                label: "Link semantic".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.link_semantic".into()),
                ontology_prefix: "hm".into(),
                description: "Link a scene node to a semantic IRI via Scene.link_semantic.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "spatial:scene_duplicate_node".into(),
                label: "Duplicate node".into(),
                icon: "3d".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("Scene.duplicate_node".into()),
                ontology_prefix: "hm".into(),
                description: "Duplicate a scene node under a new id via Scene.duplicate_node."
                    .into(),
            },
            ActionType::Invoke,
        )),
    ];

    let dmx_live: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        dmx_live_tool(
            "dmx:live_new_universe",
            "New universe",
            "Dmx.new_universe",
            "Create a 512-channel DMX universe via Dmx.new_universe.",
        ),
        dmx_live_tool(
            "dmx:live_set_channel",
            "Set channel",
            "Dmx.set_channel",
            "Set a universe channel value via Dmx.set_channel.",
        ),
        dmx_live_tool(
            "dmx:live_add_fixture",
            "Add fixture",
            "Dmx.add_fixture",
            "Add a fixture to the selected lighting surface via Dmx.add_fixture.",
        ),
        dmx_live_tool(
            "dmx:live_fixture_set_colour",
            "Fixture colour",
            "Dmx.fixture_set_colour",
            "Set fixture RGB via Dmx.fixture_set_colour.",
        ),
        dmx_live_tool(
            "dmx:live_fixture_set_intensity",
            "Fixture intensity",
            "Dmx.fixture_set_intensity",
            "Set fixture intensity via Dmx.fixture_set_intensity.",
        ),
        dmx_live_tool(
            "dmx:live_fixture_set_pan_tilt",
            "Fixture pan/tilt",
            "Dmx.fixture_set_pan_tilt",
            "Set fixture pan and tilt via Dmx.fixture_set_pan_tilt.",
        ),
        dmx_live_tool(
            "dmx:live_new_cue",
            "New cue",
            "Dmx.new_cue",
            "Create a lighting cue via Dmx.new_cue.",
        ),
        dmx_live_tool(
            "dmx:live_cue_set_channel",
            "Cue channel",
            "Dmx.cue_set_channel",
            "Set a cue channel value via Dmx.cue_set_channel.",
        ),
        dmx_live_tool(
            "dmx:live_cue_set_fade",
            "Cue fade",
            "Dmx.cue_set_fade",
            "Set cue fade in/out via Dmx.cue_set_fade.",
        ),
        dmx_live_tool(
            "dmx:live_new_cue_stack",
            "New cue stack",
            "Dmx.new_cue_stack",
            "Create a cue stack via Dmx.new_cue_stack.",
        ),
        dmx_live_tool(
            "dmx:live_cue_stack_add",
            "Stack add cue",
            "Dmx.cue_stack_add",
            "Add a cue to a stack via Dmx.cue_stack_add.",
        ),
        dmx_live_tool(
            "dmx:live_cue_stack_go",
            "Stack go",
            "Dmx.cue_stack_go",
            "Advance the cue stack via Dmx.cue_stack_go.",
        ),
        dmx_live_tool(
            "dmx:live_cue_stack_go_back",
            "Stack go back",
            "Dmx.cue_stack_go_back",
            "Step the cue stack back via Dmx.cue_stack_go_back.",
        ),
        dmx_live_tool(
            "dmx:live_cue_stack_reset",
            "Stack reset",
            "Dmx.cue_stack_reset",
            "Reset the cue stack via Dmx.cue_stack_reset.",
        ),
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
                        "Curated Scene.* camera, damp, IK, budget, clear-colour, and graph/build ops."
                            .into(),
                },
                scene_tools,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "dmx:live".into(),
                    label: "Live DMX lighting",
                    icon: "3d".into(),
                    description: "Curated Dmx.* universe, fixture, cue, and stack binds.".into(),
                },
                dmx_live,
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
