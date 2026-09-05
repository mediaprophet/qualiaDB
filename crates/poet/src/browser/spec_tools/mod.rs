//! Spec Tool Chest rows — directory-backed, one toolbox file (or folder) each.
//!
//! `mod.rs` only routes. Do not add toolbox rows here.

mod ai;
mod audio;
mod code;
mod dispatch;
mod epistemics;
mod hypermedia;
mod image;
mod investigation;
mod office;
mod portals;
mod productions;
mod research;
mod row;
mod spatial;
mod spatial3d;
mod video;

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

    #[test]
    fn office_spec_rows_have_human_copy() {
        for row in office::rows() {
            let blob = format!("{} {}", row.label, row.tooltip).to_lowercase();
            assert!(!blob.contains("capability.invoke"), "{}", row.id);
            assert!(!blob.contains("sparql"), "{}", row.id);
            assert!(!row.label.is_empty());
            assert!(!row.tooltip.is_empty());
        }
        assert!(office::rows().len() >= 20);
    }

    #[test]
    fn lookup_finds_underline() {
        assert!(lookup("office:underline").is_some());
    }

    #[test]
    fn swarm_landed_hundreds_of_spec_rows() {
        assert!(
            all_rows().len() >= 600,
            "spec swarm under-filled: {}",
            all_rows().len()
        );
    }
}
