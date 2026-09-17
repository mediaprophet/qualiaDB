//! One spec tool row. Human copy lives here so `tool_copy.rs` does not grow
//! into a monolith as hundreds of tools land.

use crate::browser::tool_proficiency::Proficiency;
use crate::tool_chest::core::intent_bus::ActionType;
use crate::tool_chest::core::tool::ToolKind;

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum Contract {
    /// Place a container type on the surface.
    Place(&'static str),
    /// Mutate the selected surface (data attributes and/or CSS).
    Local,
    /// Live ALL_BOUND id; local surface mark if the daemon is down.
    Live(&'static str),
    /// Visible, honest why-text; not stub-broken.
    Gated(&'static str),
    /// Named programme gate.
    Parked(&'static str),
}

#[derive(Clone, Copy, Debug)]
pub struct SpecTool {
    pub toolbox: &'static str,
    pub toolbox_label: &'static str,
    pub chain: &'static str,
    pub chain_label: &'static str,
    pub id: &'static str,
    pub label: &'static str,
    pub tooltip: &'static str,
    pub icon: &'static str,
    pub kind: ToolKind,
    pub action: ActionType,
    pub proficiency: Proficiency,
    pub contract: Contract,
}

impl SpecTool {
    pub const fn new(
        toolbox: &'static str,
        toolbox_label: &'static str,
        chain: &'static str,
        chain_label: &'static str,
        id: &'static str,
        label: &'static str,
        tooltip: &'static str,
        icon: &'static str,
        kind: ToolKind,
        action: ActionType,
        proficiency: Proficiency,
        contract: Contract,
    ) -> Self {
        Self {
            toolbox,
            toolbox_label,
            chain,
            chain_label,
            id,
            label,
            tooltip,
            icon,
            kind,
            action,
            proficiency,
            contract,
        }
    }

    pub fn chain_id(&self) -> String {
        format!("{}:{}", self.toolbox, self.chain)
    }

    pub fn capability_scope(&self) -> Option<String> {
        match self.contract {
            Contract::Place(_) => Some("Poet.container_place".into()),
            Contract::Live(id) => Some(id.into()),
            Contract::Local | Contract::Gated(_) | Contract::Parked(_) => None,
        }
    }
}
