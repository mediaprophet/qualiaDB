//! Client-facing, caller-buffered boundary for one native continuous-decode round.
//!
//! This module intentionally does not own a model or KV cache.  The Studio/client
//! supplies stable session rows and caller-owned scratch; the native scheduler
//! backend performs the actual token step.  Keeping this boundary flat means a
//! future chat state machine can use the same path without serialising per-turn
//! requests through a global decode mutex.

use qualia_core_db::inference::runtime::scheduler::{
    RaggedBackendError, RaggedBatchItem, RaggedBatchOutput, RaggedBatchReceipt, RaggedDecodeBackend,
};

/// One client session's input to a native continuous decode round.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionDecodeInput {
    /// Stable native request identity; must be unique within the round.
    pub request_id: u64,
    /// Scheduler slot assigned to this session.
    pub slot: u32,
    /// Most recently accepted token for this sequence.
    pub token_id: u32,
    /// Position of `token_id` in the sequence.
    pub position: u32,
    /// Offset into the shared caller-owned logical KV page table.
    pub block_table_offset: u32,
    /// Number of logical pages for this sequence.
    pub logical_pages: u32,
}

/// Token returned for one client session, preserving its scheduler identity.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionDecodeOutput {
    pub request_id: u64,
    pub slot: u32,
    pub next_token_id: u32,
}

/// Receipt returned to the client/Studio telemetry path after a decode round.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContinuousBatchReceipt {
    pub sessions_completed: u32,
    pub backend: RaggedBatchReceipt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContinuousBatchError {
    ItemScratchTooSmall,
    BackendOutputScratchTooSmall,
    SessionOutputTooSmall,
    DuplicateRequestId,
    InvalidBlockTableRange,
    Backend(RaggedBackendError),
    OutputIdentityMismatch,
}

impl From<RaggedBackendError> for ContinuousBatchError {
    fn from(value: RaggedBackendError) -> Self {
        Self::Backend(value)
    }
}

/// Execute one real ragged decode round from client session rows.
///
/// No heap state is created here.  All intermediate rows are supplied by the
/// caller, and every returned output is checked against the session identity
/// that entered the round before it becomes visible to the client.
pub fn execute_continuous_batch_round<B: RaggedDecodeBackend>(
    backend: &mut B,
    sessions: &[SessionDecodeInput],
    block_tables: &[u32],
    item_scratch: &mut [RaggedBatchItem],
    backend_output_scratch: &mut [RaggedBatchOutput],
    out: &mut [SessionDecodeOutput],
) -> Result<ContinuousBatchReceipt, ContinuousBatchError> {
    let count = sessions.len();
    if item_scratch.len() < count {
        return Err(ContinuousBatchError::ItemScratchTooSmall);
    }
    if backend_output_scratch.len() < count {
        return Err(ContinuousBatchError::BackendOutputScratchTooSmall);
    }
    if out.len() < count {
        return Err(ContinuousBatchError::SessionOutputTooSmall);
    }

    for (index, session) in sessions.iter().enumerate() {
        let table_start = session.block_table_offset as usize;
        let table_end = table_start.saturating_add(session.logical_pages as usize);
        if table_end > block_tables.len() {
            return Err(ContinuousBatchError::InvalidBlockTableRange);
        }
        for prior in &sessions[..index] {
            if prior.request_id == session.request_id {
                return Err(ContinuousBatchError::DuplicateRequestId);
            }
        }
        item_scratch[index] = RaggedBatchItem {
            request_id: session.request_id,
            slot: session.slot,
            token_id: session.token_id,
            position: session.position,
            block_table_offset: session.block_table_offset,
            logical_pages: session.logical_pages,
            _reserved: 0,
        };
    }

    let backend_receipt = backend.execute_ragged(
        &item_scratch[..count],
        block_tables,
        &mut backend_output_scratch[..count],
    )?;
    if backend_receipt.batch_size != count as u32 || backend_receipt.backend_launches != 1 {
        return Err(ContinuousBatchError::OutputIdentityMismatch);
    }

    for index in 0..count {
        let input = sessions[index];
        let decoded = backend_output_scratch[index];
        if decoded.request_id != input.request_id || decoded.slot != input.slot {
            return Err(ContinuousBatchError::OutputIdentityMismatch);
        }
        out[index] = SessionDecodeOutput {
            request_id: decoded.request_id,
            slot: decoded.slot,
            next_token_id: decoded.next_token_id,
        };
    }

    Ok(ContinuousBatchReceipt {
        sessions_completed: count as u32,
        backend: backend_receipt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::inference::runtime::scheduler::MultiSequenceRaggedBackend;

    #[test]
    fn routes_multiple_client_sessions_through_one_backend_round() {
        let mut backend =
            MultiSequenceRaggedBackend::new(|request, slot, token, position, pages| {
                request as u32 + slot + token + position + pages.len() as u32
            });
        let sessions = [
            SessionDecodeInput {
                request_id: 7,
                slot: 0,
                token_id: 3,
                position: 4,
                block_table_offset: 0,
                logical_pages: 1,
            },
            SessionDecodeInput {
                request_id: 9,
                slot: 1,
                token_id: 5,
                position: 6,
                block_table_offset: 1,
                logical_pages: 2,
            },
        ];
        let block_tables = [4, 8, 12];
        let mut items = [RaggedBatchItem::default(); 2];
        let mut backend_out = [RaggedBatchOutput::default(); 2];
        let mut out = [SessionDecodeOutput::default(); 2];

        let receipt = execute_continuous_batch_round(
            &mut backend,
            &sessions,
            &block_tables,
            &mut items,
            &mut backend_out,
            &mut out,
        )
        .unwrap();

        assert_eq!(receipt.sessions_completed, 2);
        assert_eq!(receipt.backend.backend_launches, 1);
        assert_eq!(out[0].next_token_id, 15);
        assert_eq!(out[1].next_token_id, 23);
    }

    #[test]
    fn rejects_duplicate_session_request_ids_before_backend_execution() {
        let mut backend = MultiSequenceRaggedBackend::new(|_, _, token, _, _| token);
        let sessions = [
            SessionDecodeInput {
                request_id: 7,
                ..SessionDecodeInput::default()
            },
            SessionDecodeInput {
                request_id: 7,
                ..SessionDecodeInput::default()
            },
        ];
        let mut items = [RaggedBatchItem::default(); 2];
        let mut backend_out = [RaggedBatchOutput::default(); 2];
        let mut out = [SessionDecodeOutput::default(); 2];

        let error = execute_continuous_batch_round(
            &mut backend,
            &sessions,
            &[],
            &mut items,
            &mut backend_out,
            &mut out,
        )
        .unwrap_err();

        assert_eq!(error, ContinuousBatchError::DuplicateRequestId);
    }
}
