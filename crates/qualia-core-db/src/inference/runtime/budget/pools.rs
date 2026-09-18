//! Device and host pool memory configurations and page geometry.

use super::model::ModelMemoryProfile;
use super::BudgetError;

/// Geometry of a physical KV cache block/page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockGeometry {
    /// Number of tokens packed per physical block (e.g. 16, 32).
    pub tokens_per_block: u32,
    /// Byte size of a single physical block (layers * heads * dim * elem_size).
    pub bytes_per_block: u32,
}

impl BlockGeometry {
    pub fn new(tokens_per_block: u32, bytes_per_block: u32) -> Result<Self, BudgetError> {
        if tokens_per_block == 0 || bytes_per_block == 0 {
            return Err(BudgetError::InvalidGeometry);
        }
        Ok(Self {
            tokens_per_block,
            bytes_per_block,
        })
    }

    /// Calculate the minimum number of blocks needed to house `tokens`,
    /// accounting for page boundary ceiling rounding (fragmentation).
    pub fn blocks_for_tokens(&self, tokens: u32) -> Result<u32, BudgetError> {
        if tokens == 0 {
            return Ok(0);
        }
        let round_up = tokens
            .checked_add(self.tokens_per_block - 1)
            .ok_or(BudgetError::IntegerOverflow)?;
        Ok(round_up / self.tokens_per_block)
    }

    /// Calculate the total bytes occupied by `blocks`.
    pub fn bytes_for_blocks(&self, blocks: u32) -> Result<u64, BudgetError> {
        (blocks as u64)
            .checked_mul(self.bytes_per_block as u64)
            .ok_or(BudgetError::IntegerOverflow)
    }
}

/// Joint memory plan configuration across device VRAM and pools.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryPoolBudget {
    /// Total device memory budget (e.g. 12 GB on RTX A2000).
    pub total_device_bytes: u64,
    /// Static model memory profile.
    pub model_profile: ModelMemoryProfile,
    /// Page block geometry.
    pub geometry: BlockGeometry,
    /// Physical block pool limit.
    pub max_physical_blocks: u32,
    /// Maximum concurrent COW transient blocks reserved across active requests.
    pub max_cow_transient_blocks: u32,
}

impl MemoryPoolBudget {
    pub fn new(
        total_device_bytes: u64,
        model_profile: ModelMemoryProfile,
        geometry: BlockGeometry,
        max_physical_blocks: u32,
        max_cow_transient_blocks: u32,
    ) -> Result<Self, BudgetError> {
        let static_req = model_profile.static_device_bytes();
        if static_req > total_device_bytes {
            return Err(BudgetError::InsufficientBudget {
                required_bytes: static_req,
                available_bytes: total_device_bytes,
            });
        }

        // Verify that the requested physical block count does not exceed device capacity
        let pool_bytes = geometry.bytes_for_blocks(max_physical_blocks)?;
        let total_required = static_req
            .checked_add(pool_bytes)
            .ok_or(BudgetError::IntegerOverflow)?;

        if total_required > total_device_bytes {
            return Err(BudgetError::InsufficientBudget {
                required_bytes: total_required,
                available_bytes: total_device_bytes,
            });
        }

        Ok(Self {
            total_device_bytes,
            model_profile,
            geometry,
            max_physical_blocks,
            max_cow_transient_blocks,
        })
    }

    /// Compute available dynamic memory for KV pools after static model requirements.
    pub fn available_kv_bytes(&self) -> u64 {
        self.total_device_bytes
            .saturating_sub(self.model_profile.static_device_bytes())
    }
}
