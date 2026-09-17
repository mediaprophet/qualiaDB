//! Immutable, verified model-precision configuration for runtime chat lowering.
//!
//! A contract carries only the calibration controls that are safe to apply on
//! every request. It is not a learned adapter and does not load model weights.
//! The caller must bind it to a registered target and the matching active
//! `ConditioningRegistry` profile before it is eligible for use.

use crate::inference::conditioning::ConditioningBudget;

/// How a chat lowerer must arrange the invariant portion of a prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrefixConfiguration {
    /// Keep the ordinary deterministic conditioning header before request data.
    Standard,
    /// Keep a canonical header byte-for-byte stable to make compatible prefix
    /// cache implementations eligible for reuse.
    CanonicalCacheAligned,
}

impl PrefixConfiguration {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::CanonicalCacheAligned => "canonical-cache-aligned",
        }
    }
}

/// Context reduction policy chosen during a verified calibration campaign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompressionStrategy {
    /// Preserve all context when it fits; otherwise preserve the deterministic
    /// prefix and user request inside the configured cap.
    PreservePrefix,
    /// Prefer a compact context envelope while always retaining the user
    /// request and conditioning header.
    CompactContext,
}

impl CompressionStrategy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PreservePrefix => "preserve-prefix",
            Self::CompactContext => "compact-context",
        }
    }
}

/// A model-specific precision contract that has passed an external evaluation
/// and can be bound to an active profile version.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModelPrecisionContract {
    pub profile_id: String,
    pub profile_version: u64,
    pub spec_identity: u64,
    pub budget: ConditioningBudget,
    pub prefix: PrefixConfiguration,
    pub compression: CompressionStrategy,
}

impl ModelPrecisionContract {
    pub fn new(
        profile_id: impl Into<String>,
        profile_version: u64,
        spec_identity: u64,
        budget: ConditioningBudget,
        prefix: PrefixConfiguration,
        compression: CompressionStrategy,
    ) -> Result<Self, &'static str> {
        let profile_id = profile_id.into();
        if profile_id.is_empty()
            || profile_version == 0
            || budget.input_tokens == 0
            || budget.output_tokens == 0
            || budget.max_bytes == 0
        {
            return Err("invalid model precision contract");
        }
        Ok(Self {
            profile_id,
            profile_version,
            spec_identity,
            budget,
            prefix,
            compression,
        })
    }
}
