//! Model precision targets and mapping registry.
//!
//! Represents a model's operational envelope (architecture family, context window,
//! backend capabilities, template family, target domain, and active precision profile).

use std::collections::BTreeMap;
use std::sync::{OnceLock, RwLock};

use crate::inference::conditioning::BackendCapabilities;
use crate::inference::gguf_sharder::ChatFamily;

use super::runtime_contract::ModelPrecisionContract;

/// Model architecture families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelFamily {
    Llama,
    Qwen,
    Mistral,
    Phi,
    Gemma,
    Generic,
}

impl ModelFamily {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Llama => "llama",
            Self::Qwen => "qwen",
            Self::Mistral => "mistral",
            Self::Phi => "phi",
            Self::Gemma => "gemma",
            Self::Generic => "generic",
        }
    }

    pub fn default_template_family(&self) -> ChatFamily {
        match self {
            Self::Llama => ChatFamily::Llama3,
            Self::Qwen => ChatFamily::ChatMl,
            Self::Mistral => ChatFamily::ChatMl,
            Self::Phi => ChatFamily::ChatMl,
            Self::Gemma => ChatFamily::Gemma,
            Self::Generic => ChatFamily::None,
        }
    }
}

/// Operational envelope and precision profile target for a specific model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPrecisionTarget {
    pub model_id: String,
    pub family: ModelFamily,
    pub context_window: usize,
    pub capabilities: BackendCapabilities,
    pub template_family: ChatFamily,
    pub target_domain: String,
    pub active_profile_id: Option<String>,
    /// Runtime controls may be used only after identity validation against the
    /// active profile in `ConditioningRegistry`.
    pub active_contract: Option<ModelPrecisionContract>,
}

impl ModelPrecisionTarget {
    pub fn new(
        model_id: impl Into<String>,
        family: ModelFamily,
        context_window: usize,
        capabilities: BackendCapabilities,
        target_domain: impl Into<String>,
    ) -> Self {
        let template_family = family.default_template_family();
        Self {
            model_id: model_id.into(),
            family,
            context_window,
            capabilities,
            template_family,
            target_domain: target_domain.into(),
            active_profile_id: None,
            active_contract: None,
        }
    }

    /// Construct a target for a native GGUF local model.
    pub fn native_gguf(
        model_id: impl Into<String>,
        family: ModelFamily,
        context_window: usize,
        domain: impl Into<String>,
    ) -> Self {
        Self::new(
            model_id,
            family,
            context_window,
            BackendCapabilities::native_gguf_baseline(),
            domain,
        )
    }

    /// Construct a target for an MCP remote tool/agent endpoint.
    pub fn mcp_endpoint(
        model_id: impl Into<String>,
        context_window: usize,
        domain: impl Into<String>,
    ) -> Self {
        Self::new(
            model_id,
            ModelFamily::Generic,
            context_window,
            BackendCapabilities::mcp_tool_baseline(),
            domain,
        )
    }
}

/// Registry mapping model identifiers to precision targets and active profiles.
#[derive(Debug, Default)]
pub struct ModelPrecisionRegistry {
    targets: BTreeMap<String, ModelPrecisionTarget>,
}

impl ModelPrecisionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a model precision target.
    pub fn register(&mut self, target: ModelPrecisionTarget) {
        self.targets.insert(target.model_id.clone(), target);
    }

    /// Retrieve a model precision target by ID.
    pub fn get(&self, model_id: &str) -> Option<&ModelPrecisionTarget> {
        self.targets.get(model_id)
    }

    /// Retrieve a mutable reference to a model precision target by ID.
    pub fn get_mut(&mut self, model_id: &str) -> Option<&mut ModelPrecisionTarget> {
        self.targets.get_mut(model_id)
    }

    /// Associate an active conditioning profile with a model.
    pub fn set_active_profile(
        &mut self,
        model_id: &str,
        profile_id: impl Into<String>,
    ) -> Result<(), &'static str> {
        let target = self.targets.get_mut(model_id).ok_or("model not found")?;
        target.active_profile_id = Some(profile_id.into());
        target.active_contract = None;
        Ok(())
    }

    /// Clear any active conditioning profile and contract from a model target.
    pub fn clear_active_profile(&mut self, model_id: &str) -> Result<(), &'static str> {
        let target = self.targets.get_mut(model_id).ok_or("model not found")?;
        target.active_profile_id = None;
        target.active_contract = None;
        Ok(())
    }

    /// Bind a contract to a target whose profile pointer has already been set.
    /// A later profile change clears this binding, preventing a stale contract
    /// from being applied to a new profile version.
    pub fn bind_active_contract(
        &mut self,
        model_id: &str,
        contract: ModelPrecisionContract,
    ) -> Result<(), &'static str> {
        let target = self.targets.get_mut(model_id).ok_or("model not found")?;
        if target.active_profile_id.as_deref() != Some(contract.profile_id.as_str()) {
            return Err("contract profile does not match active model profile");
        }
        target.active_contract = Some(contract);
        Ok(())
    }

    /// Resolve an eligible contract only when the active model mapping and
    /// conditioning lifecycle record agree exactly on profile ID, version, and
    /// specification identity.
    pub fn resolve_active_contract(
        &self,
        model_id: &str,
        conditioning: &crate::inference::conditioning::ConditioningRegistry,
    ) -> Option<&ModelPrecisionContract> {
        let target = self.targets.get(model_id)?;
        let profile_id = target.active_profile_id.as_deref()?;
        let contract = target.active_contract.as_ref()?;
        if contract.profile_id != profile_id {
            return None;
        }
        let profile = conditioning.get_active(profile_id)?;
        if profile.version != contract.profile_version
            || profile.spec_identity != contract.spec_identity
        {
            return None;
        }
        Some(contract)
    }

    /// Returns an iterator over all mapped model targets.
    pub fn iter(&self) -> impl Iterator<Item = &ModelPrecisionTarget> {
        self.targets.values()
    }

    /// Total count of mapped models.
    pub fn len(&self) -> usize {
        self.targets.len()
    }

    /// Whether the registry contains any targets.
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }
}

static GLOBAL_MODEL_PRECISION_REGISTRY: OnceLock<RwLock<ModelPrecisionRegistry>> = OnceLock::new();

/// Process-global registry used by model activation and chat lowering. The
/// registry holds cold-path configuration; it is never accessed in decode loops.
pub fn global_model_precision_registry() -> &'static RwLock<ModelPrecisionRegistry> {
    GLOBAL_MODEL_PRECISION_REGISTRY.get_or_init(|| RwLock::new(ModelPrecisionRegistry::new()))
}

/// Resolve a copy of the active contract from the two lifecycle registries.
/// Mismatches fail closed by returning `None`, leaving the regular chat path
/// unchanged until a coherent calibration is registered.
pub fn resolve_global_active_contract(model_id: &str) -> Option<ModelPrecisionContract> {
    let precision = global_model_precision_registry().read().ok()?;
    let conditioning = crate::inference::conditioning::global_registry()
        .read()
        .ok()?;
    precision
        .resolve_active_contract(model_id, &conditioning)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::conditioning::{ConditioningBudget, ConditioningRegistry};
    use crate::inference::conditioning_opt::{CompressionStrategy, PrefixConfiguration};

    fn contract(profile_id: &str, version: u64, spec_identity: u64) -> ModelPrecisionContract {
        ModelPrecisionContract::new(
            profile_id,
            version,
            spec_identity,
            ConditioningBudget {
                input_tokens: 2048,
                output_tokens: 128,
                tool_rounds: 2,
                max_bytes: 4096,
            },
            PrefixConfiguration::CanonicalCacheAligned,
            CompressionStrategy::PreservePrefix,
        )
        .unwrap()
    }

    #[test]
    fn contract_requires_matching_active_conditioning_identity() {
        let model_id = "precision-test-model";
        let profile_id = "urn:qualia:profile:model:precision-test-model";
        let mut models = ModelPrecisionRegistry::new();
        models.register(ModelPrecisionTarget::native_gguf(
            model_id,
            ModelFamily::Llama,
            8192,
            "test",
        ));
        models.set_active_profile(model_id, profile_id).unwrap();
        models
            .bind_active_contract(model_id, contract(profile_id, 2, 0x22))
            .unwrap();

        let mut conditioning = ConditioningRegistry::new();
        conditioning.register(profile_id, 2, 0x22, None).unwrap();
        conditioning.activate(profile_id, 2).unwrap();
        assert!(models
            .resolve_active_contract(model_id, &conditioning)
            .is_some());

        conditioning.register(profile_id, 3, 0x33, None).unwrap();
        conditioning.activate(profile_id, 3).unwrap();
        assert!(models
            .resolve_active_contract(model_id, &conditioning)
            .is_none());
    }

    #[test]
    fn profile_change_clears_existing_contract() {
        let model_id = "precision-profile-change";
        let mut models = ModelPrecisionRegistry::new();
        models.register(ModelPrecisionTarget::native_gguf(
            model_id,
            ModelFamily::Generic,
            4096,
            "test",
        ));
        models
            .set_active_profile(model_id, "urn:profile:a")
            .unwrap();
        models
            .bind_active_contract(model_id, contract("urn:profile:a", 1, 1))
            .unwrap();
        models
            .set_active_profile(model_id, "urn:profile:b")
            .unwrap();

        assert!(models.get(model_id).unwrap().active_contract.is_none());
    }
}
