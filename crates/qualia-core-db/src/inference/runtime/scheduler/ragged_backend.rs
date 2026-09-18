//! Multi-sequence ragged decode backend with CUDA graph replay bucketing (Work Package F6).
//!
//! Replaces test mocks with a zero-heap continuous-batching backend that supports:
//! - Padded batch bucketing (1, 2, 4, 8) for fixed-shape CUDA graph capture/replay.
//! - Strict isolation between real requests and dummy/padded rows.
//! - Independent per-sequence KV block-table indexing and execution.
//! - Deterministic execution receipts and device-to-host bandwidth accounting.

use super::batch::{
    RaggedBackendError, RaggedBatchItem, RaggedBatchOutput, RaggedBatchReceipt, RaggedDecodeBackend,
};

/// Maximum supported graph replay bucket size.
pub const MAX_BATCH_BUCKET: usize = 8;

/// Standard power-of-two graph replay buckets.
pub const GRAPH_BUCKET_SIZES: [usize; 4] = [1, 2, 4, 8];

/// Sentinel request ID for padded dummy rows.
pub const PADDED_REQUEST_ID: u64 = u64::MAX;

/// Sentinel slot ID for padded dummy rows.
pub const PADDED_SLOT_ID: u32 = u32::MAX;

/// Find the smallest graph bucket size that fits `batch_size`.
#[inline]
pub fn select_graph_bucket(batch_size: usize) -> Option<usize> {
    if batch_size == 0 || batch_size > MAX_BATCH_BUCKET {
        return None;
    }
    for &bucket in &GRAPH_BUCKET_SIZES {
        if bucket >= batch_size {
            return Some(bucket);
        }
    }
    None
}

/// Statistics and accounting for the ragged decode backend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RaggedBackendStats {
    pub total_rounds: u64,
    pub total_items_processed: u64,
    pub total_padded_rows: u64,
    pub bucket_launches: [u64; GRAPH_BUCKET_SIZES.len()],
}

impl RaggedBackendStats {
    /// Percentage of total processed execution slots consumed by padding (0..100).
    pub fn padding_overhead_percent(&self) -> u32 {
        let total_slots = self.total_items_processed + self.total_padded_rows;
        if total_slots == 0 {
            0
        } else {
            ((self.total_padded_rows * 100) / total_slots) as u32
        }
    }
}

/// A zero-heap multi-sequence decode backend with padded graph bucketing.
pub struct MultiSequenceRaggedBackend<F>
where
    F: FnMut(u64, u32, u32, u32, &[u32]) -> u32,
{
    step_fn: F,
    stats: RaggedBackendStats,
    allow_unbucketed_fallback: bool,
}

impl<F> MultiSequenceRaggedBackend<F>
where
    F: FnMut(u64, u32, u32, u32, &[u32]) -> u32,
{
    /// Construct a new backend with a token-step function.
    pub fn new(step_fn: F) -> Self {
        Self {
            step_fn,
            stats: RaggedBackendStats::default(),
            allow_unbucketed_fallback: false,
        }
    }

    /// Enable or disable unbucketed execution when batch size exceeds `MAX_BATCH_BUCKET`.
    pub fn with_unbucketed_fallback(mut self, allow: bool) -> Self {
        self.allow_unbucketed_fallback = allow;
        self
    }

    /// Get current backend statistics.
    pub fn stats(&self) -> &RaggedBackendStats {
        &self.stats
    }
}

impl<F> RaggedDecodeBackend for MultiSequenceRaggedBackend<F>
where
    F: FnMut(u64, u32, u32, u32, &[u32]) -> u32,
{
    fn execute_ragged(
        &mut self,
        items: &[RaggedBatchItem],
        block_tables: &[u32],
        out: &mut [RaggedBatchOutput],
    ) -> Result<RaggedBatchReceipt, RaggedBackendError> {
        let item_count = items.len();
        if item_count == 0 {
            return Ok(RaggedBatchReceipt::default());
        }
        if out.len() < item_count {
            return Err(RaggedBackendError::OutputTooSmall);
        }

        // 1. Determine graph bucket size
        let bucket_size = match select_graph_bucket(item_count) {
            Some(bucket) => bucket,
            None => {
                if self.allow_unbucketed_fallback {
                    item_count
                } else {
                    return Err(RaggedBackendError::Rejected);
                }
            }
        };

        // 2. Validate all real items and their block table slices first (fail-fast)
        for item in items {
            let table_start = item.block_table_offset as usize;
            let table_end = table_start.saturating_add(item.logical_pages as usize);
            if table_end > block_tables.len() {
                return Err(RaggedBackendError::Rejected);
            }
        }

        // 3. Prepare padded batch slots in stack storage (zero heap)
        let mut padded_batch = [RaggedBatchItem::default(); MAX_BATCH_BUCKET];
        let copy_count = item_count.min(MAX_BATCH_BUCKET);
        padded_batch[..copy_count].copy_from_slice(&items[..copy_count]);

        let padded_count = bucket_size.saturating_sub(item_count);
        for pad_idx in item_count..bucket_size.min(MAX_BATCH_BUCKET) {
            padded_batch[pad_idx] = RaggedBatchItem {
                request_id: PADDED_REQUEST_ID,
                slot: PADDED_SLOT_ID,
                token_id: 0,
                position: 0,
                block_table_offset: 0,
                logical_pages: 0,
                _reserved: 0,
            };
        }

        // 4. Execute active sequence steps
        for (index, item) in items.iter().enumerate() {
            let table_start = item.block_table_offset as usize;
            let table_end = table_start + item.logical_pages as usize;
            let page_slice = &block_tables[table_start..table_end];

            let next_token = (self.step_fn)(
                item.request_id,
                item.slot,
                item.token_id,
                item.position,
                page_slice,
            );

            out[index] = RaggedBatchOutput {
                request_id: item.request_id,
                slot: item.slot,
                next_token_id: next_token,
            };
        }

        // 5. Execute padded dummy steps if inside bucket (guaranteeing side-effect isolation)
        for pad_idx in item_count..bucket_size.min(MAX_BATCH_BUCKET) {
            let pad_item = &padded_batch[pad_idx];
            let _dummy_token = (self.step_fn)(
                pad_item.request_id,
                pad_item.slot,
                pad_item.token_id,
                pad_item.position,
                &[],
            );
            // Padded results are deliberately NEVER written to `out`
        }

        // 6. Update telemetry and accounting
        self.stats.total_rounds += 1;
        self.stats.total_items_processed += item_count as u64;
        self.stats.total_padded_rows += padded_count as u64;

        if let Some(bucket_idx) = GRAPH_BUCKET_SIZES.iter().position(|&b| b == bucket_size) {
            self.stats.bucket_launches[bucket_idx] += 1;
        }

        let d2h_bytes = (item_count * core::mem::size_of::<RaggedBatchOutput>()) as u64;

        Ok(RaggedBatchReceipt {
            batch_size: item_count as u32,
            backend_launches: 1,
            device_to_host_bytes: d2h_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_graph_bucket() {
        assert_eq!(select_graph_bucket(0), None);
        assert_eq!(select_graph_bucket(1), Some(1));
        assert_eq!(select_graph_bucket(2), Some(2));
        assert_eq!(select_graph_bucket(3), Some(4));
        assert_eq!(select_graph_bucket(4), Some(4));
        assert_eq!(select_graph_bucket(5), Some(8));
        assert_eq!(select_graph_bucket(8), Some(8));
        assert_eq!(select_graph_bucket(9), None);
    }

    #[test]
    fn test_multi_sequence_execution_and_padding_isolation() {
        let executed_ids = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let executed_ids_clone = executed_ids.clone();
        let mut backend = MultiSequenceRaggedBackend::new(
            move |req_id, slot, token_id, pos, page_slice| {
                executed_ids_clone.lock().unwrap().push(req_id);
                token_id + pos + page_slice.len() as u32 + slot
            },
        );

        let items = [
            RaggedBatchItem {
                request_id: 101,
                slot: 0,
                token_id: 10,
                position: 5,
                block_table_offset: 0,
                logical_pages: 2,
                _reserved: 0,
            },
            RaggedBatchItem {
                request_id: 202,
                slot: 1,
                token_id: 20,
                position: 8,
                block_table_offset: 2,
                logical_pages: 3,
                _reserved: 0,
            },
            RaggedBatchItem {
                request_id: 303,
                slot: 2,
                token_id: 30,
                position: 12,
                block_table_offset: 5,
                logical_pages: 1,
                _reserved: 0,
            },
        ];

        let block_tables = [10, 11, 20, 21, 22, 30];
        let mut out = [RaggedBatchOutput::default(); 3];

        let receipt = backend
            .execute_ragged(&items, &block_tables, &mut out)
            .unwrap();

        assert_eq!(receipt.batch_size, 3);
        assert_eq!(receipt.backend_launches, 1);

        // Verification of real outputs
        // item 0: token_id(10) + pos(5) + pages(2) + slot(0) = 17
        assert_eq!(out[0].request_id, 101);
        assert_eq!(out[0].slot, 0);
        assert_eq!(out[0].next_token_id, 17);

        // item 1: token_id(20) + pos(8) + pages(3) + slot(1) = 32
        assert_eq!(out[1].request_id, 202);
        assert_eq!(out[1].slot, 1);
        assert_eq!(out[1].next_token_id, 32);

        // item 2: token_id(30) + pos(12) + pages(1) + slot(2) = 45
        assert_eq!(out[2].request_id, 303);
        assert_eq!(out[2].slot, 2);
        assert_eq!(out[2].next_token_id, 45);

        // Bucket chosen was 4 (items = 3 -> bucket = 4)
        // Check executed_ids: 101, 202, 303, and one PADDED_REQUEST_ID
        assert_eq!(
            *executed_ids.lock().unwrap(),
            vec![101, 202, 303, PADDED_REQUEST_ID]
        );

        // Check stats
        let stats = backend.stats();
        assert_eq!(stats.total_rounds, 1);
        assert_eq!(stats.total_items_processed, 3);
        assert_eq!(stats.total_padded_rows, 1);
        // bucket_launches[2] is bucket size 4 (indices: 0=1, 1=2, 2=4, 3=8)
        assert_eq!(stats.bucket_launches[2], 1);
        assert_eq!(stats.padding_overhead_percent(), 25); // 1 / (3 + 1) = 25%
    }

    #[test]
    fn test_rejects_out_of_bounds_block_table() {
        let mut backend = MultiSequenceRaggedBackend::new(|_, _, token, _, _| token + 1);
        let items = [RaggedBatchItem {
            request_id: 1,
            slot: 0,
            token_id: 10,
            position: 0,
            block_table_offset: 4,
            logical_pages: 2, // 4 + 2 = 6, but block_tables has len 5
            _reserved: 0,
        }];
        let block_tables = [0, 1, 2, 3, 4];
        let mut out = [RaggedBatchOutput::default(); 1];

        let err = backend
            .execute_ragged(&items, &block_tables, &mut out)
            .unwrap_err();
        assert_eq!(err, RaggedBackendError::Rejected);
    }
}
