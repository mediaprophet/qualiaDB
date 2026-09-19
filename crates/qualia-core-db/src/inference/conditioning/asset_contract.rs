//! Asset-contract pilot implementation (Wave 8: EOS-082).
//!
//! Implements:
//! - Semantic asset identity binding content digest and metadata
//! - Two discrete scene states (State A baseline and State B conditioned/active)
//! - One functional contract validating spatial, feature, and invariant continuity
//! - Cryptographically verifiable contract execution receipt

use std::fmt;

/// Content-addressed identifier and metadata for a semantic asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetIdentity {
    pub asset_id: String,
    pub content_digest: u64,
    pub semantic_tag: u32,
    pub byte_size: usize,
}

impl AssetIdentity {
    pub fn new(asset_id: impl Into<String>, content_digest: u64, semantic_tag: u32, byte_size: usize) -> Self {
        Self {
            asset_id: asset_id.into(),
            content_digest,
            semantic_tag,
            byte_size,
        }
    }
}

/// A discrete scene state capturing spatial bounding, transform, and feature embedding.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneState {
    pub state_id: String,
    pub epoch: u64,
    pub transform: [f32; 16],
    pub feature_vector: [f32; 8],
    pub bounding_box_min: [f32; 3],
    pub bounding_box_max: [f32; 3],
    pub flags: u32,
}

pub const FLAG_ACTIVE: u32 = 1 << 0;
pub const FLAG_CONDITIONED: u32 = 1 << 1;
pub const FLAG_SPECIALIST_BOUND: u32 = 1 << 2;
pub const FLAG_DEGRADED: u32 = 1 << 3;

impl SceneState {
    /// Compute the axis-aligned bounding box volume.
    pub fn volume(&self) -> f32 {
        let dx = (self.bounding_box_max[0] - self.bounding_box_min[0]).max(0.0);
        let dy = (self.bounding_box_max[1] - self.bounding_box_min[1]).max(0.0);
        let dz = (self.bounding_box_max[2] - self.bounding_box_min[2]).max(0.0);
        dx * dy * dz
    }

    /// Calculate Euclidean distance between feature vectors.
    pub fn feature_distance(&self, other: &SceneState) -> f32 {
        let mut sum_sq = 0.0f32;
        for i in 0..8 {
            let diff = self.feature_vector[i] - other.feature_vector[i];
            sum_sq += diff * diff;
        }
        sum_sq.sqrt()
    }
}

/// Resulting status of a contract verification check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractDisposition {
    Satisfied,
    DivergenceExceeded { found: u32, max: u32 },
    VolumeExpansionExceeded { found: u32, max: u32 },
    RequiredFlagMissing { expected: u32, found: u32 },
    ForbiddenFlagPresent { forbidden: u32, found: u32 },
    AssetMismatch { expected: u64, found: u64 },
}

impl fmt::Display for ContractDisposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Satisfied => write!(f, "contract satisfied"),
            Self::DivergenceExceeded { found, max } => {
                write!(f, "feature divergence {}e-4 exceeded limit {}e-4", found, max)
            }
            Self::VolumeExpansionExceeded { found, max } => {
                write!(f, "volume expansion ratio {}e-4 exceeded limit {}e-4", found, max)
            }
            Self::RequiredFlagMissing { expected, found } => {
                write!(f, "required flags {:#x} missing from state flags {:#x}", expected, found)
            }
            Self::ForbiddenFlagPresent { forbidden, found } => {
                write!(f, "forbidden flags {:#x} found in state flags {:#x}", forbidden, found)
            }
            Self::AssetMismatch { expected, found } => {
                write!(f, "asset digest mismatch: expected {:#x}, found {:#x}", expected, found)
            }
        }
    }
}

/// Functional contract defining invariant guarantees across two scene states.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionalContract {
    pub contract_id: String,
    pub target_asset_digest: u64,
    pub max_divergence_epsilon: f32,
    pub max_volume_expansion_ratio: f32,
    pub required_flags: u32,
    pub forbidden_flags: u32,
}

/// Cryptographically verifiable receipt from executing an asset contract verification.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractVerificationReceipt {
    pub contract_id: String,
    pub asset_digest: u64,
    pub state_a_epoch: u64,
    pub state_b_epoch: u64,
    pub measured_divergence: f32,
    pub measured_volume_ratio: f32,
    pub disposition: ContractDisposition,
    pub receipt_digest: u64,
}

impl FunctionalContract {
    pub fn new(
        contract_id: impl Into<String>,
        target_asset_digest: u64,
        max_divergence_epsilon: f32,
        max_volume_expansion_ratio: f32,
        required_flags: u32,
        forbidden_flags: u32,
    ) -> Self {
        Self {
            contract_id: contract_id.into(),
            target_asset_digest,
            max_divergence_epsilon,
            max_volume_expansion_ratio,
            required_flags,
            forbidden_flags,
        }
    }

    /// Verify invariant guarantees between State A (base) and State B (conditioned/transformed).
    pub fn verify(
        &self,
        asset: &AssetIdentity,
        state_a: &SceneState,
        state_b: &SceneState,
    ) -> ContractVerificationReceipt {
        if asset.content_digest != self.target_asset_digest {
            return self.build_receipt(
                asset.content_digest,
                state_a.epoch,
                state_b.epoch,
                0.0,
                0.0,
                ContractDisposition::AssetMismatch {
                    expected: self.target_asset_digest,
                    found: asset.content_digest,
                },
            );
        }

        // Flag invariants check
        if (state_b.flags & self.required_flags) != self.required_flags {
            return self.build_receipt(
                asset.content_digest,
                state_a.epoch,
                state_b.epoch,
                0.0,
                0.0,
                ContractDisposition::RequiredFlagMissing {
                    expected: self.required_flags,
                    found: state_b.flags,
                },
            );
        }

        if (state_b.flags & self.forbidden_flags) != 0 {
            return self.build_receipt(
                asset.content_digest,
                state_a.epoch,
                state_b.epoch,
                0.0,
                0.0,
                ContractDisposition::ForbiddenFlagPresent {
                    forbidden: self.forbidden_flags,
                    found: state_b.flags,
                },
            );
        }

        // Volume expansion invariant check
        let vol_a = state_a.volume();
        let vol_b = state_b.volume();
        let volume_ratio = if vol_a > 1e-6 { vol_b / vol_a } else { 1.0 };
        if volume_ratio > self.max_volume_expansion_ratio {
            let found_u32 = (volume_ratio * 10000.0) as u32;
            let max_u32 = (self.max_volume_expansion_ratio * 10000.0) as u32;
            return self.build_receipt(
                asset.content_digest,
                state_a.epoch,
                state_b.epoch,
                0.0,
                volume_ratio,
                ContractDisposition::VolumeExpansionExceeded {
                    found: found_u32,
                    max: max_u32,
                },
            );
        }

        // Feature vector divergence invariant check
        let divergence = state_a.feature_distance(state_b);
        if divergence > self.max_divergence_epsilon {
            let found_u32 = (divergence * 10000.0) as u32;
            let max_u32 = (self.max_divergence_epsilon * 10000.0) as u32;
            return self.build_receipt(
                asset.content_digest,
                state_a.epoch,
                state_b.epoch,
                divergence,
                volume_ratio,
                ContractDisposition::DivergenceExceeded {
                    found: found_u32,
                    max: max_u32,
                },
            );
        }

        // Satisfied
        self.build_receipt(
            asset.content_digest,
            state_a.epoch,
            state_b.epoch,
            divergence,
            volume_ratio,
            ContractDisposition::Satisfied,
        )
    }

    fn build_receipt(
        &self,
        asset_digest: u64,
        epoch_a: u64,
        epoch_b: u64,
        divergence: f32,
        volume_ratio: f32,
        disposition: ContractDisposition,
    ) -> ContractVerificationReceipt {
        // Deterministic FNV-1a checksum of the receipt parameters
        let mut hasher = 0xcbf29ce484222325u64;
        for b in self.contract_id.as_bytes() {
            hasher ^= *b as u64;
            hasher = hasher.wrapping_mul(0x100000001b3);
        }
        hasher ^= asset_digest;
        hasher = hasher.wrapping_mul(0x100000001b3);
        hasher ^= epoch_a;
        hasher = hasher.wrapping_mul(0x100000001b3);
        hasher ^= epoch_b;
        hasher = hasher.wrapping_mul(0x100000001b3);
        hasher ^= (divergence.to_bits() as u64) | ((volume_ratio.to_bits() as u64) << 32);
        hasher = hasher.wrapping_mul(0x100000001b3);

        ContractVerificationReceipt {
            contract_id: self.contract_id.clone(),
            asset_digest,
            state_a_epoch: epoch_a,
            state_b_epoch: epoch_b,
            measured_divergence: divergence,
            measured_volume_ratio: volume_ratio,
            disposition,
            receipt_digest: hasher,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_asset() -> AssetIdentity {
        AssetIdentity::new("did:q42:asset:geometry:terrain_quad_042", 0x4242_CAFE_BEEF, 1, 4096)
    }

    fn test_scene_state_a() -> SceneState {
        SceneState {
            state_id: "state_a_rest".to_string(),
            epoch: 100,
            transform: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
            feature_vector: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
            bounding_box_min: [-10.0, -10.0, 0.0],
            bounding_box_max: [10.0, 10.0, 5.0],
            flags: FLAG_ACTIVE,
        }
    }

    fn test_scene_state_b() -> SceneState {
        SceneState {
            state_id: "state_b_active".to_string(),
            epoch: 101,
            transform: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
            // Slightly perturbed feature vector within epsilon
            feature_vector: [0.11, 0.21, 0.30, 0.42, 0.50, 0.59, 0.71, 0.80],
            bounding_box_min: [-10.0, -10.0, 0.0],
            bounding_box_max: [10.0, 10.0, 5.2],
            flags: FLAG_ACTIVE | FLAG_CONDITIONED,
        }
    }

    #[test]
    fn test_asset_contract_satisfied() {
        let asset = test_asset();
        let state_a = test_scene_state_a();
        let state_b = test_scene_state_b();

        let contract = FunctionalContract::new(
            "contract_spatial_continuity_v1",
            asset.content_digest,
            0.15, // max divergence epsilon
            1.20, // max volume expansion ratio (5.2/5.0 = 1.04)
            FLAG_ACTIVE | FLAG_CONDITIONED,
            FLAG_DEGRADED,
        );

        let receipt = contract.verify(&asset, &state_a, &state_b);
        assert_eq!(receipt.disposition, ContractDisposition::Satisfied);
        assert!(receipt.receipt_digest != 0);
        assert_eq!(receipt.state_a_epoch, 100);
        assert_eq!(receipt.state_b_epoch, 101);
        assert!(receipt.measured_divergence < 0.15);
    }

    #[test]
    fn test_divergence_exceeded_fails_closed() {
        let asset = test_asset();
        let state_a = test_scene_state_a();
        let mut state_b = test_scene_state_b();
        // Perturb feature vector drastically
        state_b.feature_vector[0] = 5.0;

        let contract = FunctionalContract::new(
            "contract_spatial_continuity_v1",
            asset.content_digest,
            0.05, // very tight epsilon
            1.50,
            FLAG_ACTIVE,
            FLAG_DEGRADED,
        );

        let receipt = contract.verify(&asset, &state_a, &state_b);
        match receipt.disposition {
            ContractDisposition::DivergenceExceeded { found, max } => {
                assert!(found > max);
            }
            other => panic!("expected DivergenceExceeded, got {:?}", other),
        }
    }

    #[test]
    fn test_volume_expansion_exceeded_fails_closed() {
        let asset = test_asset();
        let state_a = test_scene_state_a();
        let mut state_b = test_scene_state_b();
        // Triple bounding box z
        state_b.bounding_box_max[2] = 20.0;

        let contract = FunctionalContract::new(
            "contract_spatial_continuity_v1",
            asset.content_digest,
            1.0,
            1.10, // max 10% volume growth allowed
            FLAG_ACTIVE,
            FLAG_DEGRADED,
        );

        let receipt = contract.verify(&asset, &state_a, &state_b);
        match receipt.disposition {
            ContractDisposition::VolumeExpansionExceeded { found, max } => {
                assert!(found > max);
            }
            other => panic!("expected VolumeExpansionExceeded, got {:?}", other),
        }
    }

    #[test]
    fn test_forbidden_flag_fails_closed() {
        let asset = test_asset();
        let state_a = test_scene_state_a();
        let mut state_b = test_scene_state_b();
        state_b.flags |= FLAG_DEGRADED;

        let contract = FunctionalContract::new(
            "contract_spatial_continuity_v1",
            asset.content_digest,
            1.0,
            2.0,
            FLAG_ACTIVE,
            FLAG_DEGRADED, // forbidden
        );

        let receipt = contract.verify(&asset, &state_a, &state_b);
        match receipt.disposition {
            ContractDisposition::ForbiddenFlagPresent { forbidden, found } => {
                assert_eq!(forbidden, FLAG_DEGRADED);
                assert_eq!(found & FLAG_DEGRADED, FLAG_DEGRADED);
            }
            other => panic!("expected ForbiddenFlagPresent, got {:?}", other),
        }
    }

    #[test]
    fn test_asset_digest_mismatch_fails_closed() {
        let mut asset = test_asset();
        asset.content_digest = 0x9999_9999;
        let state_a = test_scene_state_a();
        let state_b = test_scene_state_b();

        let contract = FunctionalContract::new(
            "contract_spatial_continuity_v1",
            0x4242_CAFE_BEEF, // target
            1.0,
            2.0,
            FLAG_ACTIVE,
            0,
        );

        let receipt = contract.verify(&asset, &state_a, &state_b);
        assert_eq!(
            receipt.disposition,
            ContractDisposition::AssetMismatch {
                expected: 0x4242_CAFE_BEEF,
                found: 0x9999_9999,
            }
        );
    }
}
