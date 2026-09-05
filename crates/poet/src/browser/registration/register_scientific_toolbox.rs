//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_scientific_toolbox(reg: &mut Registry) {
    register_compact_toolbox(
        reg,
        "scientific",
        "Scientific Labs & Physics",
        "lab",
        "sci",
        "lab",
        "Clinical, molecular, and bounded physics laboratory surfaces.",
        "Clinical & Physics Labs",
        &[
            CompactTool {
                id: "place_health",
                label: "+ Health & Clinical Node",
                icon: "health",
                kind: ToolKind::PlaceContainer,
                action: ActionType::Query,
                description: "Place the clinical workbench.",
            },
            CompactTool {
                id: "place_3d",
                label: "+ Molecular 3D Viewer",
                icon: "3d",
                kind: ToolKind::PlaceContainer,
                action: ActionType::Query,
                description: "Place the available 3D scientific viewer.",
            },
            CompactTool {
                id: "thermodynamics",
                label: "Thermodynamics MCMC",
                icon: "physics",
                kind: ToolKind::RunAction,
                action: ActionType::Invoke,
                description: "Run the bounded thermodynamics capability.",
            },
        ],
    );
}
