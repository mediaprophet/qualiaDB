//! Part of poet browser toolbox registration.

use super::*;

pub(super) fn register_compact_toolbox(
    reg: &mut Registry,
    id: &str,
    label: &str,
    icon: &str,
    prefix: &str,
    family: &str,
    description: &str,
    chain_label: &str,
    tools: &[CompactTool],
) {
    let registered = tools
        .iter()
        .map(|tool| {
            Box::new(SimpleTool::new(
                ToolMetadata {
                    id: format!("{}:{}", id, tool.id),
                    label: tool.label.into(),
                    icon: tool.icon.into(),
                    kind: tool.kind,
                    capability_scope: if tool.kind == ToolKind::PlaceContainer {
                        Some(SCOPE_PLACE.into())
                    } else {
                        None
                    },
                    ontology_prefix: prefix.into(),
                    description: tool.description.into(),
                },
                tool.action,
            )) as Box<dyn crate::tool_chest::core::tool::Tool>
        })
        .collect();
    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            ontology_prefix: prefix.into(),
            description: description.into(),
            enabled_by_default: true,
            family: family.into(),
        },
        vec![ToolChain::new(
            ToolChainMetadata {
                id: format!("{}:tools", id),
                label: chain_label.into(),
                icon: icon.into(),
                description: description.into(),
            },
            registered,
        )],
    ));
}
