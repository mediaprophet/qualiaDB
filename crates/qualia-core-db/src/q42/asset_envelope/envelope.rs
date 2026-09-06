//! Versioned Q42 governed-asset envelope (AST-01).

use super::error::AssetEnvelopeError;
use super::licence::LicencePolicy;

/// Envelope schema version encoded on the wire.
pub const ASSET_ENVELOPE_VERSION: u16 = 1;
/// Soft ceiling for a single serialized envelope (cold metadata, not payload).
pub const MAX_ENVELOPE_BYTES: usize = 256 * 1024;
/// 42 MiB Sentinel pass budget referenced by every chunk plan.
pub const SENTINEL_PASS_BUDGET_BYTES: u64 = 42 * 1024 * 1024;
/// Maximum derived-from parents retained on one envelope.
pub const MAX_DERIVED_FROM: usize = 16;
/// Maximum rejection reason strings retained.
pub const MAX_REJECTION_REASONS: usize = 32;
/// Maximum identifier namespaces retained.
pub const MAX_NAMESPACES: usize = 16;

/// Sensitivity class for the asset graph (mirrors Quin sensitivity tiers used for medical).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetSensitivity {
    Public = 0,
    Restricted = 1,
    Medical = 3,
    Fiduciary = 4,
}

impl AssetSensitivity {
    pub fn from_u8(value: u8) -> Result<Self, AssetEnvelopeError> {
        match value {
            0 => Ok(Self::Public),
            1 => Ok(Self::Restricted),
            3 => Ok(Self::Medical),
            4 => Ok(Self::Fiduciary),
            _ => Err(AssetEnvelopeError::InvalidSensitivity),
        }
    }
}

/// Commons-routing lane hint for derived Quins.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetRoutingLane {
    Passthrough = 0,
    Commons = 1,
    Bilateral = 2,
    Spatial = 3,
}

impl AssetRoutingLane {
    pub fn from_u8(value: u8) -> Result<Self, AssetEnvelopeError> {
        match value {
            0 => Ok(Self::Passthrough),
            1 => Ok(Self::Commons),
            2 => Ok(Self::Bilateral),
            3 => Ok(Self::Spatial),
            _ => Err(AssetEnvelopeError::InvalidRoutingLane),
        }
    }
}

/// One bounded ingestion chunk under the 42 MiB Sentinel pass budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkSpec {
    pub index: u32,
    pub byte_budget: u64,
    pub record_budget: u64,
}

/// Counts retained from a release import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RecordCounts {
    pub source: u64,
    pub accepted: u64,
    pub quarantined: u64,
}

/// Upstream release identity for an immutable raw artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamRelease {
    pub source_name: String,
    pub release_id: String,
    pub source_url: String,
    pub retrieved_unix: u64,
    pub byte_length: u64,
    pub sha256: [u8; 32],
}

/// Parser / mapping toolchain versions that produced derived assets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainVersions {
    pub parser_version: String,
    pub mapping_version: String,
}

/// Versioned core schema for a governed Q42 dataset asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Q42AssetEnvelope {
    pub asset_id: String,
    pub upstream: UpstreamRelease,
    pub licence: LicencePolicy,
    pub toolchain: ToolchainVersions,
    pub raw_format: String,
    pub media_type: String,
    pub counts: RecordCounts,
    pub rejection_reasons: Vec<String>,
    pub identifier_namespaces: Vec<String>,
    pub cross_reference_policy: String,
    pub evidence_grade: String,
    pub citation: String,
    pub curation_status: String,
    pub sensitivity: AssetSensitivity,
    pub routing_lane: AssetRoutingLane,
    pub derived_from: Vec<String>,
    pub shacl_profile: String,
    pub validation_report: String,
    pub chunk_plan: Vec<ChunkSpec>,
}

impl Q42AssetEnvelope {
    /// Fail closed on unknown licence, empty identity, inconsistent counts, or oversize chunks.
    pub fn validate(&self) -> Result<(), AssetEnvelopeError> {
        if self.asset_id.trim().is_empty() {
            return Err(AssetEnvelopeError::MissingAssetId);
        }
        if self.upstream.source_name.trim().is_empty()
            || self.upstream.release_id.trim().is_empty()
        {
            return Err(AssetEnvelopeError::MissingUpstreamRelease);
        }
        if self.licence.class == super::licence::LicenceClass::Unknown {
            return Err(AssetEnvelopeError::UnknownLicence);
        }
        if self.counts.accepted.saturating_add(self.counts.quarantined) > self.counts.source {
            return Err(AssetEnvelopeError::CountInconsistency);
        }
        if self.chunk_plan.is_empty() {
            return Err(AssetEnvelopeError::EmptyChunkPlan);
        }
        for chunk in &self.chunk_plan {
            if chunk.byte_budget == 0 || chunk.byte_budget > SENTINEL_PASS_BUDGET_BYTES {
                return Err(AssetEnvelopeError::ChunkBudgetExceeded);
            }
        }
        if self.derived_from.len() > MAX_DERIVED_FROM
            || self.rejection_reasons.len() > MAX_REJECTION_REASONS
            || self.identifier_namespaces.len() > MAX_NAMESPACES
        {
            return Err(AssetEnvelopeError::Oversize);
        }
        Ok(())
    }

    /// Build a derived envelope that inherits the union of licence obligations.
    pub fn derive_with(
        &self,
        asset_id: impl Into<String>,
        other_licence: &LicencePolicy,
        toolchain: ToolchainVersions,
        chunk_plan: Vec<ChunkSpec>,
    ) -> Result<Self, AssetEnvelopeError> {
        let mut derived = self.clone();
        derived.asset_id = asset_id.into();
        derived.licence = self.licence.union_obligations(other_licence);
        derived.toolchain = toolchain;
        derived.chunk_plan = chunk_plan;
        derived.derived_from = {
            let mut parents = vec![self.asset_id.clone()];
            parents.extend(self.derived_from.iter().cloned());
            parents.truncate(MAX_DERIVED_FROM);
            parents
        };
        derived.validate()?;
        Ok(derived)
    }

    /// Verify a payload digest against the upstream SHA-256 recorded on the envelope.
    pub fn verify_payload_digest(&self, digest: &[u8; 32]) -> Result<(), AssetEnvelopeError> {
        if digest == &self.upstream.sha256 {
            Ok(())
        } else {
            Err(AssetEnvelopeError::DigestMismatch)
        }
    }
}

/// Compute SHA-256 into a caller-owned buffer (ABI-facing digest path).
pub fn sha256_into(bytes: &[u8], out: &mut [u8; 32]) {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    out.copy_from_slice(&digest);
}

/// Allocate-free convenience for cold tests and tooling.
pub fn sha256_of(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    sha256_into(bytes, &mut out);
    out
}
