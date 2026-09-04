//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_rights_toolbox(reg: &mut Registry) {
    let tools: Vec<Box<dyn crate::tool_chest::core::tool::Tool>> = vec![
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "rights:authors_group".into(),
                label: "Authors Group".into(),
                icon: "group".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("graph:write".into()),
                ontology_prefix: "rights".into(),
                description: "Manage the authors group.".into(),
            },
            ActionType::Mutate,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "rights:fiduciary_sign".into(),
                label: "✍️ Sign Contract".into(),
                icon: "sign".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("capability:invoke".into()),
                ontology_prefix: "rights".into(),
                description: "Sign with fiduciary authority.".into(),
            },
            ActionType::Invoke,
        )),
        Box::new(SimpleTool::new(
            ToolMetadata {
                id: "rights:did_sign".into(),
                label: "DID Sign".into(),
                icon: "did".into(),
                kind: ToolKind::RunAction,
                capability_scope: Some("capability:invoke".into()),
                ontology_prefix: "rights".into(),
                description: "Sign with a DID.".into(),
            },
            ActionType::Invoke,
        )),
    ];

    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "rights".into(),
            label: "Governance & Rights".into(),
            icon: "rights".into(),
            ontology_prefix: "rights".into(),
            description: "Fiduciary contracts, Hohfeldian rights, and DID signing.".into(),
            enabled_by_default: true,
            family: "governance".into(),
        },
        vec![
            ToolChain::new(
                ToolChainMetadata {
                    id: "rights:fiduciary".into(),
                    label: "Fiduciary & Routing Lanes".into(),
                    icon: "rights".into(),
                    description:
                        "Select privacy routing lane (00/01/10/11) and Hohfeldian modalities."
                            .into(),
                },
                vec![],
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "rights:tools".into(),
                    label: "Identity & Signatures".into(),
                    icon: "tools".into(),
                    description: "Fiduciary and rights signing tools.".into(),
                },
                tools,
            ),
        ],
    ));
}
