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

pub(super) struct CompactTool {
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    kind: ToolKind,
    action: ActionType,
    description: &'static str,
}

pub(super) use crate::tool_chest::core::intent_bus::ActionType;
pub(super) use crate::tool_chest::core::registry::Registry;
pub(super) use crate::tool_chest::core::tool::{SimpleTool, ToolKind, ToolMetadata};
pub(super) use crate::tool_chest::core::tool_chain::{ToolChain, ToolChainMetadata};
pub(super) use crate::tool_chest::core::toolbox::{Toolbox, ToolboxMetadata};
pub(super) use crate::tool_chest::manifolds;

mod register_compact_toolbox;
mod register_audio_toolbox;
mod register_erp_toolbox;
mod register_mail_toolbox;
mod register_scientific_toolbox;
mod register_sdn_toolbox;
mod register_epistemic_toolbox;
mod register_office_toolbox;
mod register_image_toolbox;
mod register_sheet_toolbox;
mod register_spatial_toolbox;
mod register_communication_toolbox;
mod register_rights_toolbox;
mod register_health_toolbox;
mod register_code_toolbox;
mod register_ai_toolbox;

use register_compact_toolbox::register_compact_toolbox;
use register_audio_toolbox::register_audio_toolbox;
use register_erp_toolbox::register_erp_toolbox;
use register_mail_toolbox::register_mail_toolbox;
use register_scientific_toolbox::register_scientific_toolbox;
use register_sdn_toolbox::register_sdn_toolbox;
use register_epistemic_toolbox::register_epistemic_toolbox;
use register_office_toolbox::register_office_toolbox;
use register_image_toolbox::register_image_toolbox;
use register_sheet_toolbox::register_sheet_toolbox;
use register_spatial_toolbox::register_spatial_toolbox;
use register_communication_toolbox::register_communication_toolbox;
use register_rights_toolbox::register_rights_toolbox;
use register_health_toolbox::register_health_toolbox;
use register_code_toolbox::register_code_toolbox;
use register_ai_toolbox::register_ai_toolbox;
