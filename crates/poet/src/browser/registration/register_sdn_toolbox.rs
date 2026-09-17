//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_sdn_toolbox(reg: &mut Registry) {
    register_compact_toolbox(
        reg,
        "sdn",
        "SDN & Cooperative Economics",
        "sdn",
        "sdn",
        "sdn",
        "Peer distribution, cooperative economics, and energy governance.",
        "SDN & Cooperative Economics",
        &[
            CompactTool {
                id: "place_webrtc",
                label: "+ WebTorrent Swarm Seeder",
                icon: "webrtc",
                kind: ToolKind::PlaceContainer,
                action: ActionType::Publish,
                description: "Place the peer distribution surface.",
            },
            CompactTool {
                id: "place_finance",
                label: "+ Unit Economics Modeler",
                icon: "finance",
                kind: ToolKind::PlaceContainer,
                action: ActionType::Query,
                description: "Place the unit economics model.",
            },
            CompactTool {
                id: "energy_governor",
                label: "Battery & Solar Governor",
                icon: "energy",
                kind: ToolKind::RunAction,
                action: ActionType::Invoke,
                description: "Invoke the energy governor capability.",
            },
        ],
    );
}
