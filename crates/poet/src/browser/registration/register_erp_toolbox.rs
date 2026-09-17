//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_erp_toolbox(reg: &mut Registry) {
    register_compact_toolbox(
        reg,
        "erp",
        "Cooperative ERP & Workstream",
        "erp",
        "erp",
        "erp",
        "Cooperative project planning, timelines, and M-of-N decisions.",
        "Cooperative ERP & Workstream A",
        &[
            CompactTool {
                id: "place_kanban",
                label: "+ Cooperative Kanban Board",
                icon: "kanban",
                kind: ToolKind::PlaceContainer,
                action: ActionType::Query,
                description: "Place the cooperative Kanban board.",
            },
            CompactTool {
                id: "place_gantt",
                label: "+ Gantt Timeline Cascade",
                icon: "gantt",
                kind: ToolKind::PlaceContainer,
                action: ActionType::Query,
                description: "Place a Gantt planning surface.",
            },
            CompactTool {
                id: "place_voting",
                label: "+ M-of-N Voting Ballot",
                icon: "voting",
                kind: ToolKind::PlaceContainer,
                action: ActionType::Query,
                description: "Place the live M-of-N voting surface.",
            },
        ],
    );
}
