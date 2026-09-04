//! Toolbox registration — populates the Registry at startup.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Registers all 15 toolboxes with their tool-chains and tools,
//! plus all manifold seeds, into a shared Registry.

use crate::tool_chest::core::intent_bus::ActionType;
use crate::tool_chest::core::registry::Registry;
use crate::tool_chest::core::tool::{SimpleTool, ToolKind, ToolMetadata};
use crate::tool_chest::core::tool_chain::{ToolChain, ToolChainMetadata};
use crate::tool_chest::core::toolbox::{Toolbox, ToolboxMetadata};

use crate::tool_chest::manifolds;

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

    // Register all manifold seeds
    for seed in manifolds::all_seeds() {
        reg.register_manifold(seed);
    }

    for construct in crate::tool_chest::constructs::all_constructs() {
        reg.register_construct(construct);
    }

    reg
}
