//! Spec Tool Chest rows — directory-backed, one toolbox file (or folder) each.
//!
//! `mod.rs` only routes. Do not add toolbox rows here.

mod ai;
mod ai_actions;
mod audio;
mod audio_actions;
mod code;
mod code_actions;
mod dispatch;
mod epistemic_actions;
mod epistemics;
mod hypermedia;
mod hypermedia_actions;
mod image;
mod image_actions;
mod investigation;
mod investigation_actions;
mod live_args;
mod local_effects;
mod media_actions;
mod office;
mod office_actions;
mod portals;
mod portals_actions;
mod productions;
mod productions_actions;
mod research;
mod research_actions;
mod row;
mod spatial;
mod spatial_actions;
mod spatial3d;
mod spatial3d_actions;
mod video;
mod video_actions;

pub use dispatch::{gated_reason, run};
pub use row::SpecTool;

use crate::tool_chest::core::registry::Registry;
use crate::tool_chest::core::tool::{SimpleTool, ToolMetadata};
use crate::tool_chest::core::tool_chain::{ToolChain, ToolChainMetadata};
use crate::tool_chest::core::toolbox::{Toolbox, ToolboxMetadata};

fn all_rows() -> Vec<&'static SpecTool> {
    office::rows()
        .iter()
        .chain(image::rows())
        .chain(audio::rows())
        .chain(video::rows())
        .chain(spatial3d::rows())
        .chain(portals::rows())
        .chain(productions::rows())
        .chain(hypermedia::rows())
        .chain(code::rows())
        .chain(ai::rows())
        .chain(spatial::rows())
        .chain(epistemics::rows())
        .chain(investigation::rows())
        .chain(research::rows())
        .collect()
}

pub fn lookup(id: &str) -> Option<&'static SpecTool> {
    all_rows().into_iter().find(|row| row.id == id)
}

pub fn register_all(reg: &mut Registry) {
    for tool in all_rows() {
        merge_row(reg, tool);
    }
}

fn merge_row(reg: &mut Registry, spec: &SpecTool) {
    let chain_id = spec.chain_id();
    let tool = Box::new(SimpleTool::new(
        ToolMetadata {
            id: spec.id.into(),
            label: spec.label.into(),
            icon: spec.icon.into(),
            kind: spec.kind,
            capability_scope: spec.capability_scope(),
            ontology_prefix: spec.toolbox.into(),
            description: spec.tooltip.into(),
        },
        spec.action,
    ));
    if let Some(toolbox) = reg.toolbox_mut(spec.toolbox) {
        if let Some(chain) = toolbox.chain_mut(&chain_id) {
            chain.add_tool(tool);
            return;
        }
        toolbox.add_chain(ToolChain::new(
            ToolChainMetadata {
                id: chain_id,
                label: spec.chain_label.into(),
                icon: spec.icon.into(),
                description: spec.chain_label.into(),
            },
            vec![tool],
        ));
        return;
    }
    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: spec.toolbox.into(),
            label: spec.toolbox_label.into(),
            icon: spec.icon.into(),
            ontology_prefix: spec.toolbox.into(),
            description: spec.toolbox_label.into(),
            enabled_by_default: true,
            family: spec.toolbox.into(),
        },
        vec![ToolChain::new(
            ToolChainMetadata {
                id: chain_id,
                label: spec.chain_label.into(),
                icon: spec.icon.into(),
                description: spec.chain_label.into(),
            },
            vec![tool],
        )],
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::tool_proficiency::Proficiency;
    use crate::tool_chest::core::tool::ToolKind;
    use std::collections::HashSet;

    #[test]
    fn every_row_has_human_copy() {
        for row in all_rows() {
            let blob = format!("{} {}", row.label, row.tooltip).to_lowercase();
            assert!(!row.label.trim().is_empty(), "{} has no label", row.id);
            assert!(!row.tooltip.trim().is_empty(), "{} has no tooltip", row.id);
            if row.proficiency != Proficiency::Expert {
                assert!(!blob.contains("capability.invoke"), "{}", row.id);
                assert!(!blob.contains("sparql"), "{}", row.id);
                assert!(!blob.contains("quin.statement"), "{}", row.id);
            }
        }
    }

    #[test]
    fn lookup_finds_underline() {
        assert!(lookup("office:underline").is_some());
    }

    #[test]
    fn row_count_matches_named_spec_tools() {
        assert_eq!(all_rows().len(), 702, "named spec-tool inventory drifted");
    }

    #[test]
    fn ids_are_unique_and_contracts_are_well_formed() {
        let mut ids = HashSet::new();
        for row in all_rows() {
            assert!(ids.insert(row.id), "duplicate spec-tool id: {}", row.id);
            assert!(!row.toolbox.trim().is_empty(), "{} has no toolbox", row.id);
            assert!(!row.chain.trim().is_empty(), "{} has no chain", row.id);
            match row.contract {
                row::Contract::Place(container_type) => {
                    assert_eq!(row.kind, ToolKind::PlaceContainer, "{}", row.id);
                    assert!(!container_type.trim().is_empty(), "{}", row.id);
                }
                row::Contract::Live(capability) => {
                    assert!(capability.contains('.'), "{}: {capability}", row.id);
                    assert!(
                        live_args::supports(capability),
                        "{} has no checked live argument adapter for {capability}",
                        row.id
                    );
                }
                row::Contract::Gated(reason) | row::Contract::Parked(reason) => {
                    assert!(!reason.trim().is_empty(), "{} has no gate reason", row.id);
                }
                row::Contract::Local => {}
            }
        }
    }
}
