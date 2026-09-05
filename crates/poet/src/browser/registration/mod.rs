//! Toolbox registration — populates the Registry at startup.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Registers all 15 toolboxes with their tool-chains and tools,
//! plus all manifold seeds, into a shared Registry.
//!
//! Split into per-toolbox modules so MCP-sized commits can land (G-POET-TOOLCHEST).

pub(super) use crate::tool_chest::core::intent_bus::ActionType;
pub(super) use crate::tool_chest::core::registry::Registry;
pub(super) use crate::tool_chest::core::tool::{SimpleTool, ToolKind, ToolMetadata};
pub(super) use crate::tool_chest::core::tool_chain::{ToolChain, ToolChainMetadata};
pub(super) use crate::tool_chest::core::toolbox::{Toolbox, ToolboxMetadata};
pub(super) use crate::tool_chest::manifolds;

mod register_ai_toolbox;
mod register_audio_toolbox;
mod register_code_toolbox;
mod register_communication_toolbox;
mod register_compact_toolbox;
mod register_epistemic_toolbox;
mod register_erp_toolbox;
mod register_health_toolbox;
mod register_image_toolbox;
mod register_mail_toolbox;
mod register_office_toolbox;
mod register_rights_toolbox;
mod register_scientific_toolbox;
mod register_sdn_toolbox;
mod register_sheet_toolbox;
mod register_spatial_toolbox;

use register_ai_toolbox::register_ai_toolbox;
use register_audio_toolbox::register_audio_toolbox;
use register_code_toolbox::register_code_toolbox;
use register_communication_toolbox::register_communication_toolbox;
use register_compact_toolbox::register_compact_toolbox;
use register_epistemic_toolbox::register_epistemic_toolbox;
use register_erp_toolbox::register_erp_toolbox;
use register_health_toolbox::register_health_toolbox;
use register_image_toolbox::register_image_toolbox;
use register_mail_toolbox::register_mail_toolbox;
use register_office_toolbox::register_office_toolbox;
use register_rights_toolbox::register_rights_toolbox;
use register_scientific_toolbox::register_scientific_toolbox;
use register_sdn_toolbox::register_sdn_toolbox;
use register_sheet_toolbox::register_sheet_toolbox;
use register_spatial_toolbox::register_spatial_toolbox;

/// Live ALL_BOUND id for placing a container on the manifold.
pub(super) const SCOPE_PLACE: &str = "Poet.container_place";

/// Shared by compact toolbox helpers (`register_*` siblings).
pub(super) struct CompactTool {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) icon: &'static str,
    pub(super) kind: ToolKind,
    pub(super) action: ActionType,
    pub(super) description: &'static str,
}

/// Build a fully populated Registry with all toolboxes and manifold seeds.
pub fn build_registry() -> Registry {
    let mut reg = Registry::new();

    register_epistemic_toolbox(&mut reg);
    register_office_toolbox(&mut reg);
    register_image_toolbox(&mut reg);
    register_sheet_toolbox(&mut reg);
    register_spatial_toolbox(&mut reg);
    register_audio_toolbox(&mut reg);
    register_communication_toolbox(&mut reg);
    register_erp_toolbox(&mut reg);
    register_mail_toolbox(&mut reg);
    register_scientific_toolbox(&mut reg);
    register_rights_toolbox(&mut reg);
    register_health_toolbox(&mut reg);
    register_code_toolbox(&mut reg);
    register_ai_toolbox(&mut reg);
    register_sdn_toolbox(&mut reg);

    for seed in manifolds::all_seeds() {
        reg.register_manifold(seed);
    }

    for construct in crate::tool_chest::constructs::all_constructs() {
        reg.register_construct(construct);
    }

    reg
}

#[cfg(test)]
mod tests {
    use super::SCOPE_PLACE;
    use crate::tool_chest::core::tool::ToolKind;

    fn is_legacy_scope(scope: &str) -> bool {
        scope.starts_with("graph:")
            || scope.starts_with("capability:")
            || scope.starts_with("vibe:")
            || scope.starts_with("pulse:")
            || scope.starts_with("aura:")
            || scope.starts_with("ui:")
            || scope.starts_with("intent:")
    }

    #[test]
    fn capability_scopes_are_live_family_method_or_local() {
        let registry = super::build_registry();
        for toolbox in registry.toolboxes() {
            for chain in toolbox.chains() {
                for tool in chain.tools() {
                    let meta = tool.metadata();
                    if let Some(scope) = meta.capability_scope.as_deref() {
                        assert!(
                            scope.contains('.'),
                            "{}: capability_scope `{scope}` must be Family.method",
                            meta.id
                        );
                        assert!(
                            !is_legacy_scope(scope),
                            "{}: legacy scope `{scope}` is not a live ALL_BOUND id",
                            meta.id
                        );
                    }
                    if meta.kind == ToolKind::PlaceContainer {
                        assert_eq!(
                            meta.capability_scope.as_deref(),
                            Some(SCOPE_PLACE),
                            "{}: PlaceContainer must cite {SCOPE_PLACE}",
                            meta.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn office_shapes_chain_binds_live_n3_and_shacl() {
        let registry = super::build_registry();
        let office = registry.toolbox("office").expect("office toolbox");
        let chain = office
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "office:shapes")
            .expect("office:shapes toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("n3:evaluate", Some("N3Logic.evaluate"))));
        assert!(tools.contains(&("shacl:validate", Some("SHACL.validate"))));
    }

    #[test]
    fn every_chain_has_at_least_one_tool() {
        let registry = super::build_registry();
        for toolbox in registry.toolboxes() {
            for chain in toolbox.chains() {
                assert!(
                    !chain.tools().is_empty(),
                    "empty chain {} in toolbox {}",
                    chain.metadata().id,
                    toolbox.metadata().id
                );
            }
        }
    }
}
