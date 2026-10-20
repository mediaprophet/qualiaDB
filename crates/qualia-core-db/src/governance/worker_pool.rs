//! Webizen-governed worker pool for background `.hmc` / `.10d` / `.q42` hydration.
//!
//! Enforces deontic capability tokens ([`HydrationCapabilityToken`]), fuel budgets,
//! and byte budgets across three priority tiers ([`StreamPriorityTier`]) so that
//! CRC-32C section verification, SHA-256 entry checks, and KTX2 block transcoding
//! never exceed the caller's authorized compute or memory envelope.

use std::collections::VecDeque;

use crate::bundle::streaming_loader::StreamPriorityTier;
use crate::container_10d::crc32c::crc32c;
use crate::render::texture_transcode::{
    decode_astc_4x4_block, decode_bc7_block, decode_etc2_eac_rgba_block,
};
use sha2::{Digest, Sha256};

/// Capability bit: verify `.10d` section CRC-32C checksums.
pub const CAP_VERIFY_CRC32C: u32 = 1 << 0;
/// Capability bit: verify `.hmc` entry SHA-256 digests.
pub const CAP_VERIFY_SHA256: u32 = 1 << 1;
/// Capability bit: decode `.10d` mesh / morph / skeleton sections.
pub const CAP_DECODE_MESH_10D: u32 = 1 << 2;
/// Capability bit: transcode KTX2 (BC7 / ETC2 / ASTC) compressed texture blocks.
pub const CAP_TRANSCODE_KTX2: u32 = 1 << 3;
/// Capability bit: admit `.q42` semantic slices.
pub const CAP_ADMIT_Q42_SLICE: u32 = 1 << 4;
/// All hydration capabilities enabled.
pub const CAP_ALL_HYDRATION: u32 = CAP_VERIFY_CRC32C
    | CAP_VERIFY_SHA256
    | CAP_DECODE_MESH_10D
    | CAP_TRANSCODE_KTX2
    | CAP_ADMIT_Q42_SLICE;

/// Deontic capability token governing background hydration tasks.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HydrationCapabilityToken {
    pub holder_did_hash: u64,
    pub granted_mask: u32,
    pub max_tier_allowed: u8,
    pub _pad: [u8; 3],
    pub fuel_remaining: u64,
    pub byte_budget_remaining: u64,
    pub parity_seal: u64,
}

impl HydrationCapabilityToken {
    /// Construct and seal a new capability token for `holder_did_hash`.
    #[inline]
    pub const fn new_sealed(
        holder_did_hash: u64,
        granted_mask: u32,
        max_tier: StreamPriorityTier,
        fuel_budget: u64,
        byte_budget: u64,
    ) -> Self {
        let max_tier_allowed = max_tier as u8;
        let parity_seal = Self::compute_seal(
            holder_did_hash,
            granted_mask,
            max_tier_allowed,
            fuel_budget,
            byte_budget,
        );
        Self {
            holder_did_hash,
            granted_mask,
            max_tier_allowed,
            _pad: [0; 3],
            fuel_remaining: fuel_budget,
            byte_budget_remaining: byte_budget,
            parity_seal,
        }
    }

    #[inline]
    const fn compute_seal(
        holder_did_hash: u64,
        granted_mask: u32,
        max_tier_allowed: u8,
        fuel_remaining: u64,
        byte_budget_remaining: u64,
    ) -> u64 {
        let mixed = holder_did_hash
            ^ ((granted_mask as u64) << 17)
            ^ ((max_tier_allowed as u64) << 49)
            ^ fuel_remaining.rotate_left(13)
            ^ byte_budget_remaining.rotate_left(37);
        mixed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
    }

    /// Verify that the token seal has not been tampered with.
    #[inline]
    pub const fn verify_seal(&self) -> bool {
        self.parity_seal
            == Self::compute_seal(
                self.holder_did_hash,
                self.granted_mask,
                self.max_tier_allowed,
                self.fuel_remaining,
                self.byte_budget_remaining,
            )
    }

    /// Deduct `fuel_cost` and `byte_cost` after checking `required_cap` and `tier`, updating the seal.
    pub fn authorize_and_deduct(
        &mut self,
        required_cap: u32,
        tier: StreamPriorityTier,
        fuel_cost: u64,
        byte_cost: u64,
    ) -> Result<(), GovernancePoolError> {
        if !self.verify_seal() {
            return Err(GovernancePoolError::InvalidTokenSeal);
        }
        if (self.granted_mask & required_cap) != required_cap {
            return Err(GovernancePoolError::CapabilityDenied {
                required: required_cap,
                granted: self.granted_mask,
            });
        }
        if (tier as u8) > self.max_tier_allowed {
            return Err(GovernancePoolError::TierForbidden {
                requested: tier,
                max_allowed: self.max_tier_allowed,
            });
        }
        if self.fuel_remaining < fuel_cost {
            return Err(GovernancePoolError::FuelExhausted {
                needed: fuel_cost,
                remaining: self.fuel_remaining,
            });
        }
        if self.byte_budget_remaining < byte_cost {
            return Err(GovernancePoolError::ByteBudgetExceeded {
                needed: byte_cost,
                remaining: self.byte_budget_remaining,
            });
        }
        self.fuel_remaining -= fuel_cost;
        self.byte_budget_remaining -= byte_cost;
        self.parity_seal = Self::compute_seal(
            self.holder_did_hash,
            self.granted_mask,
            self.max_tier_allowed,
            self.fuel_remaining,
            self.byte_budget_remaining,
        );
        Ok(())
    }
}

/// Errors raised when scheduling or executing governed hydration work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernancePoolError {
    InvalidTokenSeal,
    CapabilityDenied { required: u32, granted: u32 },
    TierForbidden { requested: StreamPriorityTier, max_allowed: u8 },
    FuelExhausted { needed: u64, remaining: u64 },
    ByteBudgetExceeded { needed: u64, remaining: u64 },
    QueueFull { capacity: usize },
    IntegrityMismatch { task_id: u64 },
}

/// Compressed block codec for governed zero-heap texture transcoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscodeCodec {
    Bc7,
    Etc2EacRgba8,
    Astc4x4,
}

/// Hydration task descriptor queued into [`GovernedHydrationPool`].
#[derive(Debug, Clone)]
pub enum HydrationTaskKind {
    /// Verify `.10d` section CRC-32C checksum.
    VerifySectionCrc32c {
        payload: Vec<u8>,
        expected_crc32c: u32,
    },
    /// Verify `.hmc` entry SHA-256 digest.
    VerifyEntrySha256 {
        payload: Vec<u8>,
        expected_sha256: [u8; 32],
    },
    /// Transcode a 16-byte compressed texture block to a 64-byte 4x4 RGBA8 tile.
    TranscodeTextureBlock {
        codec: TranscodeCodec,
        block: [u8; 16],
    },
}

impl HydrationTaskKind {
    #[inline]
    pub fn required_capability(&self) -> u32 {
        match self {
            HydrationTaskKind::VerifySectionCrc32c { .. } => CAP_VERIFY_CRC32C,
            HydrationTaskKind::VerifyEntrySha256 { .. } => CAP_VERIFY_SHA256,
            HydrationTaskKind::TranscodeTextureBlock { .. } => CAP_TRANSCODE_KTX2,
        }
    }

    #[inline]
    pub fn byte_cost(&self) -> u64 {
        match self {
            HydrationTaskKind::VerifySectionCrc32c { payload, .. } => payload.len() as u64,
            HydrationTaskKind::VerifyEntrySha256 { payload, .. } => payload.len() as u64,
            HydrationTaskKind::TranscodeTextureBlock { .. } => 64,
        }
    }

    #[inline]
    pub fn fuel_cost(&self) -> u64 {
        match self {
            HydrationTaskKind::VerifySectionCrc32c { payload, .. } => {
                10 + (payload.len() as u64 / 64)
            }
            HydrationTaskKind::VerifyEntrySha256 { payload, .. } => {
                25 + (payload.len() as u64 / 32)
            }
            HydrationTaskKind::TranscodeTextureBlock { .. } => 16,
        }
    }
}

/// Queued task with priority tier and unique identifier.
#[derive(Debug, Clone)]
pub struct HydrationTask {
    pub task_id: u64,
    pub tier: StreamPriorityTier,
    pub kind: HydrationTaskKind,
}

/// Completed task receipt returned by the governed worker pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HydrationReceipt {
    pub task_id: u64,
    pub tier: StreamPriorityTier,
    pub verified: bool,
    pub transcoded_rgba8_4x4: Option<[u8; 64]>,
}

/// Zero-heap hot execution kernel for a single 16-byte texture block or CRC-32C slice check.
#[inline]
pub fn execute_crc_or_transcode_zero_heap(
    codec: Option<TranscodeCodec>,
    block_16: &[u8; 16],
    crc_slice: &[u8],
    expected_crc32c: u32,
    rgba_out: &mut [u8; 64],
) -> bool {
    if let Some(c) = codec {
        let pixels: &mut [[u8; 4]; 16] = bytemuck::cast_mut(rgba_out);
        match c {
            TranscodeCodec::Bc7 => decode_bc7_block(block_16, pixels),
            TranscodeCodec::Etc2EacRgba8 => decode_etc2_eac_rgba_block(block_16, pixels),
            TranscodeCodec::Astc4x4 => decode_astc_4x4_block(block_16, pixels),
        }
        true
    } else {
        crc32c(crc_slice) == expected_crc32c
    }
}

/// Webizen-governed 3-tier hydration worker pool.
pub struct GovernedHydrationPool {
    queues: [VecDeque<HydrationTask>; 3],
    max_queue_capacity: usize,
    completed_count: u64,
}

impl GovernedHydrationPool {
    /// Create a new governed hydration pool with bounded per-tier queue capacity.
    pub fn new(max_queue_capacity: usize) -> Self {
        let cap = max_queue_capacity.max(1);
        Self {
            queues: [
                VecDeque::with_capacity(cap.min(64)),
                VecDeque::with_capacity(cap.min(64)),
                VecDeque::with_capacity(cap.min(64)),
            ],
            max_queue_capacity: cap,
            completed_count: 0,
        }
    }

    /// Submit a hydration task after authorizing and deducting fuel + byte budgets from `token`.
    pub fn submit(
        &mut self,
        token: &mut HydrationCapabilityToken,
        task: HydrationTask,
    ) -> Result<(), GovernancePoolError> {
        let tier_idx = task.tier as usize;
        if self.queues[tier_idx].len() >= self.max_queue_capacity {
            return Err(GovernancePoolError::QueueFull {
                capacity: self.max_queue_capacity,
            });
        }
        token.authorize_and_deduct(
            task.kind.required_capability(),
            task.tier,
            task.kind.fuel_cost(),
            task.kind.byte_cost(),
        )?;
        self.queues[tier_idx].push_back(task);
        Ok(())
    }

    /// Number of pending tasks across all tiers.
    #[inline]
    pub fn pending_count(&self) -> usize {
        self.queues[0].len() + self.queues[1].len() + self.queues[2].len()
    }

    /// Total tasks completed so far.
    #[inline]
    pub fn completed_count(&self) -> u64 {
        self.completed_count
    }

    /// Pop the highest-priority pending task (`Tier0` -> `Tier1` -> `Tier2`) and execute it.
    pub fn step_one(&mut self) -> Result<Option<HydrationReceipt>, GovernancePoolError> {
        let task = if let Some(t) = self.queues[0].pop_front() {
            t
        } else if let Some(t) = self.queues[1].pop_front() {
            t
        } else if let Some(t) = self.queues[2].pop_front() {
            t
        } else {
            return Ok(None);
        };

        let receipt = match &task.kind {
            HydrationTaskKind::VerifySectionCrc32c {
                payload,
                expected_crc32c,
            } => {
                let mut dummy = [0u8; 64];
                let ok = execute_crc_or_transcode_zero_heap(
                    None,
                    &[0u8; 16],
                    payload,
                    *expected_crc32c,
                    &mut dummy,
                );
                if !ok {
                    return Err(GovernancePoolError::IntegrityMismatch {
                        task_id: task.task_id,
                    });
                }
                HydrationReceipt {
                    task_id: task.task_id,
                    tier: task.tier,
                    verified: true,
                    transcoded_rgba8_4x4: None,
                }
            }
            HydrationTaskKind::VerifyEntrySha256 {
                payload,
                expected_sha256,
            } => {
                let mut hasher = Sha256::new();
                hasher.update(payload);
                if hasher.finalize().as_slice() != expected_sha256.as_slice() {
                    return Err(GovernancePoolError::IntegrityMismatch {
                        task_id: task.task_id,
                    });
                }
                HydrationReceipt {
                    task_id: task.task_id,
                    tier: task.tier,
                    verified: true,
                    transcoded_rgba8_4x4: None,
                }
            }
            HydrationTaskKind::TranscodeTextureBlock { codec, block } => {
                let mut rgba = [0u8; 64];
                execute_crc_or_transcode_zero_heap(Some(*codec), block, &[], 0, &mut rgba);
                HydrationReceipt {
                    task_id: task.task_id,
                    tier: task.tier,
                    verified: true,
                    transcoded_rgba8_4x4: Some(rgba),
                }
            }
        };
        self.completed_count += 1;
        Ok(Some(receipt))
    }

    /// Drain up to `max_tasks` in strict priority order (`Tier0` before `Tier1` before `Tier2`).
    pub fn drain_bounded(
        &mut self,
        max_tasks: usize,
    ) -> Result<Vec<HydrationReceipt>, GovernancePoolError> {
        let mut out = Vec::with_capacity(max_tasks.min(self.pending_count()));
        for _ in 0..max_tasks {
            match self.step_one()? {
                Some(r) => out.push(r),
                None => break,
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    #[test]
    fn capability_token_enforces_seal_capabilities_and_budgets() {
        let mut token = HydrationCapabilityToken::new_sealed(
            0xABCD_1234,
            CAP_VERIFY_CRC32C | CAP_TRANSCODE_KTX2,
            StreamPriorityTier::Tier2HighPolyAndMip0,
            100,
            1024,
        );
        assert!(token.verify_seal());

        let mut pool = GovernedHydrationPool::new(16);

        // Unauthorized SHA-256 task is rejected fail-closed
        let sha_task = HydrationTask {
            task_id: 1,
            tier: StreamPriorityTier::Tier0SemanticAndProcedural,
            kind: HydrationTaskKind::VerifyEntrySha256 {
                payload: vec![1, 2, 3],
                expected_sha256: [0; 32],
            },
        };
        assert!(matches!(
            pool.submit(&mut token, sha_task),
            Err(GovernancePoolError::CapabilityDenied { .. })
        ));

        // Submit Tier 2 transcode first, then Tier 0 CRC check; Tier 0 must execute first!
        let mut block = [0u8; 16];
        block[0] = 1 << 6; // BC7 Mode 6
        pool.submit(
            &mut token,
            HydrationTask {
                task_id: 20,
                tier: StreamPriorityTier::Tier2HighPolyAndMip0,
                kind: HydrationTaskKind::TranscodeTextureBlock {
                    codec: TranscodeCodec::Bc7,
                    block,
                },
            },
        )
        .unwrap();

        let payload = b"tier0 procedural twin section".to_vec();
        let expected_crc = crc32c(&payload);
        pool.submit(
            &mut token,
            HydrationTask {
                task_id: 10,
                tier: StreamPriorityTier::Tier0SemanticAndProcedural,
                kind: HydrationTaskKind::VerifySectionCrc32c {
                    payload: payload.clone(),
                    expected_crc32c: expected_crc,
                },
            },
        )
        .unwrap();

        let receipts = pool.drain_bounded(4).unwrap();
        assert_eq!(receipts.len(), 2);
        assert_eq!(receipts[0].task_id, 10);
        assert_eq!(receipts[1].task_id, 20);
        assert!(receipts[1].transcoded_rgba8_4x4.is_some());

        // Hot kernel is zero-heap
        let mut rgba = [0u8; 64];
        assert_zero_alloc("worker_pool_hot_transcode", || {
            assert!(execute_crc_or_transcode_zero_heap(
                Some(TranscodeCodec::Bc7),
                &block,
                &[],
                0,
                &mut rgba
            ));
        });
    }
}
