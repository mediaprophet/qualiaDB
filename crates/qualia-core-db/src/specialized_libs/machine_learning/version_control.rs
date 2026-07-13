//! Model version control impls.

use super::*;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::collections::HashMap;

impl ModelVersionControl {
    pub fn new() -> Self {
        Self {
            versions: HashMap::new(),
            branches: HashMap::new(),
            tags: HashMap::new(),
            initialized: false,
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        // Seed the default `main` branch so the controller is usable immediately.
        self.branches
            .entry("main".to_string())
            .or_insert_with(Vec::new);
        self.initialized = true;
        Ok(())
    }

    /// Register a new version for a model. Returns an error if a version with
    /// the same `version_id` is already registered for that model.
    pub fn create_version(&mut self, model_id: &str, version: ModelVersion) -> Result<(), MLError> {
        let key = version_key(model_id, &version.version_id);
        if self.versions.contains_key(&key) {
            return Err(MLError::ModelError(format!(
                "version '{}' already exists for model '{}'",
                version.version_id, model_id
            )));
        }
        let version_id = version.version_id.clone();
        self.versions.insert(key, version);
        // Append the new version to the `main` branch if it exists.
        if let Some(branch) = self.branches.get_mut("main") {
            if !branch.iter().any(|v| v == &version_id) {
                branch.push(version_id);
            }
        }
        Ok(())
    }

    /// Get a specific version of a model by its version id.
    pub fn get_version(&self, model_id: &str, version_id: &str) -> Option<&ModelVersion> {
        self.versions.get(&version_key(model_id, version_id))
    }

    /// List all version ids registered for a model.
    pub fn list_versions(&self, model_id: &str) -> Vec<String> {
        let prefix = format!("{}::", model_id);
        self.versions
            .keys()
            .filter_map(|k| k.strip_prefix(&prefix).map(|s| s.to_string()))
            .collect()
    }

    /// Create a branch starting from an existing version. The branch initially
    /// contains only the originating version.
    pub fn create_branch(&mut self, branch_name: &str, from_version: &str) -> Result<(), MLError> {
        if self.branches.contains_key(branch_name) {
            return Err(MLError::ModelError(format!(
                "branch '{}' already exists",
                branch_name
            )));
        }
        // Validate that the originating version is registered somewhere.
        let exists = self.versions.values().any(|v| v.version_id == from_version);
        if !exists {
            return Err(MLError::ModelError(format!(
                "cannot branch from unknown version '{}'",
                from_version
            )));
        }
        self.branches
            .insert(branch_name.to_string(), vec![from_version.to_string()]);
        Ok(())
    }

    /// Get the list of version ids in a branch.
    pub fn get_branch(&self, branch_name: &str) -> Option<&Vec<String>> {
        self.branches.get(branch_name)
    }

    /// Tag a version. Multiple tags may be attached to the same version.
    pub fn tag_version(&mut self, version_id: &str, tag: &str) -> Result<(), MLError> {
        // The version must exist somewhere in the registry.
        let exists = self.versions.values().any(|v| v.version_id == version_id);
        if !exists {
            return Err(MLError::ModelError(format!(
                "cannot tag unknown version '{}'",
                version_id
            )));
        }
        let entry = self
            .tags
            .entry(version_id.to_string())
            .or_insert_with(Vec::new);
        if !entry.iter().any(|t| t == tag) {
            entry.push(tag.to_string());
        }
        Ok(())
    }

    /// Get all tags attached to a version.
    pub fn get_tags(&self, version_id: &str) -> Vec<String> {
        self.tags.get(version_id).cloned().unwrap_or_default()
    }

    /// Find all version ids that carry the given tag.
    pub fn get_by_tag(&self, tag: &str) -> Vec<String> {
        self.tags
            .iter()
            .filter_map(|(version_id, tags)| {
                if tags.iter().any(|t| t == tag) {
                    Some(version_id.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Build the composite key used to store versions per model.
fn version_key(model_id: &str, version_id: &str) -> String {
    format!("{}::{}", model_id, version_id)
}

impl ModelVersion {
    pub fn new() -> Self {
        Self {
            version_id: "v1.0.0".to_string(),
            version_number: "1.0.0".to_string(),
            changes: Vec::new(),
            created_at: 0,
            created_by: "system".to_string(),
        }
    }
}

impl ModelChange {
    pub fn new() -> Self {
        Self {
            change_id: "change_1".to_string(),
            change_type: ChangeType::Architecture,
            description: "Initial model".to_string(),
            affected_layers: Vec::new(),
        }
    }
}

impl ModelBranch {
    pub fn new() -> Self {
        Self {
            branch_id: "main".to_string(),
            branch_name: "main".to_string(),
            base_version: "v1.0.0".to_string(),
            head_version: "v1.0.0".to_string(),
        }
    }
}

impl ModelTag {
    pub fn new() -> Self {
        Self {
            tag_id: "latest".to_string(),
            tag_name: "latest".to_string(),
            version: "v1.0.0".to_string(),
            description: "Latest version".to_string(),
        }
    }
}
