//! Deterministic command catalog with alias mapping and collision rejection.

use crate::descriptor::CommandDescriptor;
use crate::error::ContractError;
use crate::id::CommandId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Canonical registry/catalog of available command descriptors.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CommandCatalog {
    /// Ordered list of unique registered descriptors.
    descriptors: Vec<CommandDescriptor>,
    /// Index mapping command ID to descriptor position.
    #[serde(skip)]
    id_to_index: BTreeMap<CommandId, usize>,
    /// Alias mapping (e.g. "bold" -> "wordcraft::format.bold").
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    aliases: BTreeMap<String, CommandId>,
}

impl CommandCatalog {
    /// Create an empty catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a command descriptor. Rejects duplicate IDs and invalid schemas.
    pub fn register(&mut self, descriptor: CommandDescriptor) -> Result<(), ContractError> {
        descriptor.validate()?;

        if self.id_to_index.contains_key(&descriptor.command_id) {
            return Err(ContractError::DuplicateCommandId(descriptor.command_id.as_str().to_string()));
        }

        let idx = self.descriptors.len();
        self.id_to_index.insert(descriptor.command_id.clone(), idx);
        self.descriptors.push(descriptor);
        Ok(())
    }

    /// Register an alias pointing to a registered command ID. Rejects duplicate aliases.
    pub fn register_alias(&mut self, alias: impl Into<String>, target: &CommandId) -> Result<(), ContractError> {
        let alias = alias.into();
        if !self.id_to_index.contains_key(target) {
            return Err(ContractError::MalformedParams(format!(
                "alias target '{}' is not registered in catalog",
                target.as_str()
            )));
        }

        if self.aliases.contains_key(&alias) {
            return Err(ContractError::DuplicateAlias(alias));
        }

        self.aliases.insert(alias, target.clone());
        Ok(())
    }

    /// Lookup a descriptor by its exact command ID.
    pub fn get(&self, id: &CommandId) -> Option<&CommandDescriptor> {
        self.id_to_index.get(id).map(|&idx| &self.descriptors[idx])
    }

    /// Resolve an alias or direct command ID to its descriptor.
    pub fn resolve(&self, alias_or_id: &str) -> Option<&CommandDescriptor> {
        if let Some(target_id) = self.aliases.get(alias_or_id) {
            return self.get(target_id);
        }
        if let Ok(cmd_id) = CommandId::parse(alias_or_id) {
            return self.get(&cmd_id);
        }
        None
    }

    /// Returns a slice of all registered descriptors in insertion order.
    pub fn descriptors(&self) -> &[CommandDescriptor] {
        &self.descriptors
    }

    /// Count of registered descriptors.
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    /// Check if the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }
}
