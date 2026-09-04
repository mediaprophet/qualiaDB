//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_mail_toolbox(reg: &mut Registry) {
    register_compact_toolbox(
        reg,
        "mail",
        "Inalienable Mail & Web Publisher",
        "mail",
        "mail",
        "mail",
        "DID-addressed mail, CML composition, and web publishing.",
        "Inalienable Domain Communications",
        &[
            CompactTool {
                id: "place_mail",
                label: "+ Inalienable Mail Inbox",
                icon: "mail",
                kind: ToolKind::PlaceContainer,
                action: ActionType::Query,
                description: "Place the inalienable mail workspace.",
            },
            CompactTool {
                id: "composer",
                label: "CML Mail Composer",
                icon: "doc",
                kind: ToolKind::RunAction,
                action: ActionType::Navigate,
                description: "Open the CML mail composer.",
            },
            CompactTool {
                id: "publisher",
                label: "Web Site Publisher",
                icon: "webview",
                kind: ToolKind::RunAction,
                action: ActionType::Publish,
                description: "Publish an authorised web artefact.",
            },
        ],
    );
}
