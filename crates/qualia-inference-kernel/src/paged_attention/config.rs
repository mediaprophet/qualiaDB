pub const DEFAULT_BLOCK_SIZE: u32 = 16;
pub const INVALID_BLOCK: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PagedKvConfig {
    pub block_size: u32,
    pub n_layer: u32,
    pub n_kv_head: u32,
    pub head_dim: u32,
    pub max_context: u32,
    /// Physical pages in the shared pool. A single sequence needs
    /// `n_layer * logical_blocks_per_layer()` pages.
    pub physical_blocks: u32,
}

impl PagedKvConfig {
    pub fn new(n_layer: u32, n_kv_head: u32, head_dim: u32, max_context: u32) -> Option<Self> {
        let block_size = DEFAULT_BLOCK_SIZE;
        let logical = max_context.checked_add(block_size - 1)? / block_size;
        let physical_blocks = n_layer.checked_mul(logical)?;
        let config = Self {
            block_size,
            n_layer,
            n_kv_head,
            head_dim,
            max_context,
            physical_blocks,
        };
        config.is_valid().then_some(config)
    }

    pub fn is_valid(&self) -> bool {
        self.block_size.is_power_of_two()
            && self.block_size <= 256
            && self.n_layer > 0
            && self.n_kv_head > 0
            && self.head_dim > 0
            && self.max_context > 0
            && self.physical_blocks >= self.required_single_sequence_blocks()
    }

    #[inline]
    pub fn logical_blocks_per_layer(&self) -> u32 {
        self.max_context.div_ceil(self.block_size)
    }

    #[inline]
    pub fn required_single_sequence_blocks(&self) -> u32 {
        self.n_layer.saturating_mul(self.logical_blocks_per_layer())
    }

    #[inline]
    pub fn slot_kv_elems(&self) -> u32 {
        self.n_kv_head.saturating_mul(self.head_dim)
    }

    #[inline]
    pub fn block_elems(&self) -> usize {
        self.block_size as usize * self.slot_kv_elems() as usize * 2
    }

    #[inline]
    pub fn arena_bytes(&self) -> Option<usize> {
        self.block_elems()
            .checked_mul(self.physical_blocks as usize)?
            .checked_mul(core::mem::size_of::<f32>())
    }
}
