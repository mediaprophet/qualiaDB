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

/// Maximum concurrent active chat sessions in a single continuous batch round.
pub const MAX_CONCURRENT_CHAT_SESSIONS: usize = 8;

/// State for an admitted chat session advancing through continuous decode rounds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatBatchSessionState {
    pub session_id: String,
    pub request_id: u64,
    pub slot: u32,
    pub current_token_id: u32,
    pub position: u32,
    pub block_table_offset: u32,
    pub logical_pages: u32,
    pub is_active: bool,
    pub is_finished: bool,
    pub generated_tokens: Vec<u32>,
}

/// Zero-heap-hot-path coordinator managing concurrent chat sessions across continuous batch decode rounds.
#[derive(Debug, Default)]
pub struct ChatContinuousBatchCoordinator {
    sessions: Vec<ChatBatchSessionState>,
    next_slot: u32,
}

impl ChatContinuousBatchCoordinator {
    /// Create a new coordinator for multi-session continuous batching.
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            next_slot: 0,
        }
    }

    /// Admit a new chat session to the batch pool.
    pub fn admit_session(
        &mut self,
        session_id: &str,
        request_id: u64,
        initial_token_id: u32,
        initial_position: u32,
        block_table_offset: u32,
        logical_pages: u32,
    ) -> Result<u32, ContinuousBatchError> {
        for s in &self.sessions {
            if s.is_active && s.request_id == request_id {
                return Err(ContinuousBatchError::DuplicateRequestId);
            }
        }
        let slot = self.next_slot;
        self.next_slot = self.next_slot.wrapping_add(1);

        self.sessions.push(ChatBatchSessionState {
            session_id: session_id.to_string(),
            request_id,
            slot,
            current_token_id: initial_token_id,
            position: initial_position,
            block_table_offset,
            logical_pages,
            is_active: true,
            is_finished: false,
            generated_tokens: Vec::new(),
        });

        Ok(slot)
    }

    /// Number of active, unfinished sessions currently tracked.
    pub fn active_session_count(&self) -> usize {
        self.sessions
            .iter()
            .filter(|s| s.is_active && !s.is_finished)
            .count()
    }

    /// Step all active chat sessions forward by one token step.
    ///
    /// The decode step executes via `execute_continuous_batch_round` over fixed stack
    /// scratch arrays (zero heap allocation in hot step loop).
    pub fn step_round<B: RaggedDecodeBackend>(
        &mut self,
        backend: &mut B,
        block_tables: &[u32],
        eos_token_id: u32,
    ) -> Result<ContinuousBatchReceipt, ContinuousBatchError> {
        let active_indices: Vec<usize> = self
            .sessions
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_active && !s.is_finished)
            .map(|(i, _)| i)
            .take(MAX_CONCURRENT_CHAT_SESSIONS)
            .collect();

        let count = active_indices.len();
        if count == 0 {
            return Ok(ContinuousBatchReceipt::default());
        }

        let mut inputs = [SessionDecodeInput::default(); MAX_CONCURRENT_CHAT_SESSIONS];
        let mut items = [RaggedBatchItem::default(); MAX_CONCURRENT_CHAT_SESSIONS];
        let mut backend_outputs = [RaggedBatchOutput::default(); MAX_CONCURRENT_CHAT_SESSIONS];
        let mut session_outputs = [SessionDecodeOutput::default(); MAX_CONCURRENT_CHAT_SESSIONS];

        for (idx, &session_idx) in active_indices.iter().enumerate() {
            let session = &self.sessions[session_idx];
            inputs[idx] = SessionDecodeInput {
                request_id: session.request_id,
                slot: session.slot,
                token_id: session.current_token_id,
                position: session.position,
                block_table_offset: session.block_table_offset,
                logical_pages: session.logical_pages,
            };
        }

        let receipt = execute_continuous_batch_round(
            backend,
            &inputs[..count],
            block_tables,
            &mut items[..count],
            &mut backend_outputs[..count],
            &mut session_outputs[..count],
        )?;

        for (idx, &session_idx) in active_indices.iter().enumerate() {
            let output = session_outputs[idx];
            let session = &mut self.sessions[session_idx];
            session.generated_tokens.push(output.next_token_id);
            session.current_token_id = output.next_token_id;
            session.position = session.position.saturating_add(1);
            if output.next_token_id == eos_token_id {
                session.is_finished = true;
            }
        }

        Ok(receipt)
    }

    /// Check if a specific session has completed generation.
    pub fn is_session_finished(&self, request_id: u64) -> bool {
        self.sessions
            .iter()
            .find(|s| s.request_id == request_id)
            .map(|s| s.is_finished)
            .unwrap_or(true)
    }

    /// Retrieve the generated token stream for a session.
    pub fn get_session_tokens(&self, request_id: u64) -> Option<&[u32]> {
        self.sessions
            .iter()
            .find(|s| s.request_id == request_id)
            .map(|s| s.generated_tokens.as_slice())
    }

    /// Mark a session as cancelled/inactive.
    pub fn cancel_session(&mut self, request_id: u64) {
        if let Some(session) = self.sessions.iter_mut().find(|s| s.request_id == request_id) {
            session.is_active = false;
        }
    }
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

    #[test]
    fn coordinator_advances_multiple_active_chat_sessions_to_completion() {
        let mut backend =
            MultiSequenceRaggedBackend::new(|_req, _slot, token, _pos, _pages| {
                if token >= 10 {
                    99 // EOS
                } else {
                    token + 1
                }
            });

        let mut coord = ChatContinuousBatchCoordinator::new();
        coord
            .admit_session("session-alpha", 101, 8, 0, 0, 1)
            .unwrap();
        coord
            .admit_session("session-beta", 102, 9, 0, 1, 1)
            .unwrap();

        assert_eq!(coord.active_session_count(), 2);
        let block_tables = [10, 20];

        // Round 1: alpha goes 8 -> 9; beta goes 9 -> 10
        let r1 = coord.step_round(&mut backend, &block_tables, 99).unwrap();
        assert_eq!(r1.sessions_completed, 2);
        assert_eq!(coord.active_session_count(), 2);
        assert!(!coord.is_session_finished(101));
        assert!(!coord.is_session_finished(102));

        // Round 2: alpha goes 9 -> 10; beta goes 10 -> 99 (EOS -> finishes)
        let r2 = coord.step_round(&mut backend, &block_tables, 99).unwrap();
        assert_eq!(r2.sessions_completed, 2);
        assert_eq!(coord.active_session_count(), 1);
        assert!(!coord.is_session_finished(101));
        assert!(coord.is_session_finished(102));

        // Round 3: only alpha runs (goes 10 -> 99 (EOS -> finishes))
        let r3 = coord.step_round(&mut backend, &block_tables, 99).unwrap();
        assert_eq!(r3.sessions_completed, 1);
        assert_eq!(coord.active_session_count(), 0);
        assert!(coord.is_session_finished(101));

        assert_eq!(coord.get_session_tokens(101), Some(&[9, 10, 99][..]));
        assert_eq!(coord.get_session_tokens(102), Some(&[10, 99][..]));
    }
}

