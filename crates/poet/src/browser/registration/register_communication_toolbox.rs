//! Part of poet browser toolbox registration.

use super::*;

fn hid_live_tool(
    id: &'static str,
    label: &'static str,
    scope: &'static str,
    description: &'static str,
) -> Box<dyn crate::tool_chest::core::tool::Tool> {
    Box::new(SimpleTool::new(
        ToolMetadata {
            id: id.into(),
            label: label.into(),
            icon: "comm".into(),
            kind: ToolKind::RunAction,
            capability_scope: Some(scope.into()),
            ontology_prefix: "comm".into(),
            description: description.into(),
        },
        ActionType::Invoke,
    ))
}

pub(super) fn register_communication_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "comm:place_social".into(),
                label: "+ Social Graph".into(),
                icon: "social".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some(SCOPE_PLACE.into()),
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
                capability_scope: Some(SCOPE_PLACE.into()),
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
                capability_scope: Some(SCOPE_PLACE.into()),
                ontology_prefix: "hm".into(),
                description: "Place a web frame container.".into(),
            },
            ActionType::Query,
        )),
    ];

    let hid_live: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        hid_live_tool(
            "hid:live_poll",
            "Poll HID",
            "HID.poll",
            "Poll the next HID event via HID.poll.",
        ),
        hid_live_tool(
            "hid:live_wait",
            "Wait HID",
            "HID.wait",
            "Wait for a HID event via HID.wait.",
        ),
        hid_live_tool(
            "hid:live_clear",
            "Clear HID",
            "HID.clear",
            "Clear queued HID events via HID.clear.",
        ),
        hid_live_tool(
            "hid:live_pointer_capture",
            "Pointer capture",
            "HID.pointer_capture",
            "Capture pointer focus via HID.pointer_capture.",
        ),
        hid_live_tool(
            "hid:live_pointer_release",
            "Pointer release",
            "HID.pointer_release",
            "Release pointer capture via HID.pointer_release.",
        ),
        hid_live_tool(
            "hid:live_set_cursor",
            "Set cursor",
            "HID.set_cursor",
            "Set cursor style via HID.set_cursor.",
        ),
        hid_live_tool(
            "hid:live_gamepad_poll",
            "Gamepad poll",
            "HID.gamepad_poll",
            "Poll gamepad state via HID.gamepad_poll.",
        ),
        hid_live_tool(
            "hid:live_gamepad_vibrate",
            "Gamepad rumble",
            "HID.gamepad_vibrate",
            "Dispatch gamepad rumble via HID.gamepad_vibrate.",
        ),
        hid_live_tool(
            "hid:live_midi_send",
            "MIDI send",
            "HID.midi_send",
            "Send a MIDI packet via HID.midi_send.",
        ),
        hid_live_tool(
            "hid:live_midi_poll",
            "MIDI poll",
            "HID.midi_poll",
            "Poll incoming MIDI via HID.midi_poll.",
        ),
        hid_live_tool(
            "hid:live_haptic_pulse",
            "Haptic pulse",
            "HID.haptic_pulse",
            "Trigger a haptic pulse via HID.haptic_pulse.",
        ),
        hid_live_tool(
            "hid:live_haptic_pattern",
            "Haptic pattern",
            "HID.haptic_pattern",
            "Play a haptic pattern via HID.haptic_pattern.",
        ),
        hid_live_tool(
            "hid:live_spatial_head_pose",
            "Head pose",
            "HID.spatial_head_pose",
            "Read spatial head pose via HID.spatial_head_pose.",
        ),
        hid_live_tool(
            "hid:live_spatial_hand_skeleton",
            "Hand skeleton",
            "HID.spatial_hand_skeleton",
            "Read hand skeleton via HID.spatial_hand_skeleton.",
        ),
        hid_live_tool(
            "hid:live_spatial_gaze_ray",
            "Gaze ray",
            "HID.spatial_gaze_ray",
            "Read gaze ray via HID.spatial_gaze_ray.",
        ),
        hid_live_tool(
            "hid:live_biosignal_poll",
            "Biosignal poll",
            "HID.biosignal_poll",
            "Poll privacy-filtered biosignal via HID.biosignal_poll.",
        ),
    ];

    let pulse_live: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        hid_live_tool(
            "comm:pulse_live_publish",
            "Publish pulse",
            "Pulse.publish",
            "Publish a generic pulse via Pulse.publish.",
        ),
        hid_live_tool(
            "comm:pulse_live_graph_mutation",
            "Publish graph mutation",
            "Pulse.publish_graph_mutation",
            "Publish a graph-mutation pulse via Pulse.publish_graph_mutation.",
        ),
        hid_live_tool(
            "comm:pulse_live_notification",
            "Publish notification",
            "Pulse.publish_notification",
            "Publish a notification pulse via Pulse.publish_notification.",
        ),
        hid_live_tool(
            "comm:pulse_live_telemetry",
            "Publish telemetry",
            "Pulse.publish_telemetry",
            "Publish a telemetry pulse via Pulse.publish_telemetry.",
        ),
        hid_live_tool(
            "comm:pulse_live_agent_message",
            "Publish agent message",
            "Pulse.publish_agent_message",
            "Publish an agent-message pulse via Pulse.publish_agent_message.",
        ),
        hid_live_tool(
            "comm:pulse_live_sync",
            "Publish sync",
            "Pulse.publish_sync",
            "Publish a sync pulse via Pulse.publish_sync.",
        ),
        hid_live_tool(
            "comm:pulse_live_open_channel",
            "Open channel",
            "Pulse.open_channel",
            "Open a pulse channel via Pulse.open_channel.",
        ),
        hid_live_tool(
            "comm:pulse_live_close_channel",
            "Close channel",
            "Pulse.close_channel",
            "Close a pulse channel via Pulse.close_channel.",
        ),
        hid_live_tool(
            "comm:pulse_live_set_transport",
            "Set transport",
            "Pulse.set_transport",
            "Set pulse transport via Pulse.set_transport.",
        ),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "communication".into(),
            label: "Communication & Presence".into(),
            icon: "comm".into(),
            ontology_prefix: "comm".into(),
            description: "Pulse streams, social graphs, WebRTC, and web presence.".into(),
            enabled_by_default: true,
            family: "mail".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "comm:pulse".into(),
                    label: "Pulse Streams & Messaging".into(),
                    icon: "comm".into(),
                    description: "Select protocol and encryption tiers.".into(),
                },
                vec![Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "comm:pulse_presence".into(),
                        label: "Publish presence".into(),
                        icon: "comm".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Pulse.publish_presence".into()),
                        ontology_prefix: "comm".into(),
                        description: "Mark local presence; daemon upgrades to Pulse.publish_presence.".into(),
                    },
                    ActionType::Publish,
                ))],
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
            ToolChain::new(
                ToolChainMetadata {
                    id: "hid:live".into(),
                    label: "Live HID & sensors".into(),
                    icon: "comm".into(),
                    description: "Curated HID.* pointer, gamepad, MIDI, haptic, spatial, and biosignal binds."
                        .into(),
                },
                hid_live,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "comm:pulse_live".into(),
                    label: "Live Pulse".into(),
                    icon: "comm".into(),
                    description: "Curated Pulse.* publish, channel, and transport binds.".into(),
                },
                pulse_live,
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "comm:hbbtv".into(),
                    label: "Live HbbTV".into(),
                    icon: "comm".into(),
                    description: "Curated HbbTV.* app, page, navigate, and state binds.".into(),
                },
                super::register_wave33_live::hbbtv_tools(),
            ),
        ],
    ));
}
