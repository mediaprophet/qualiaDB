use crate::inference::runtime::graph_assist::PrefixIdentity;
use crate::inference::runtime::kv::paged::{
    BlockPool, CopyOnWrite, SequenceBlockTable, TableError,
};
use crate::inference::runtime::kv::prefix::{PrefixKvError, PrefixKvStore};

use super::batch::{
    RaggedBackendError, RaggedBatchItem, RaggedBatchOutput, RaggedBatchReceipt,
    RaggedDecodeBackend,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestState {
    Empty,
    Prefill,
    Decode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Admission {
    pub slot: u16,
    pub prefix_hit: bool,
    pub prefix_tokens: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestView {
    pub request_id: u64,
    pub slot: u16,
    pub state: RequestState,
    pub token_count: u32,
    pub prefix_tokens: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedulerError {
    DuplicateRequest,
    Full,
    UnknownRequest,
    OutputTooSmall,
    Prefix(PrefixKvError),
    Table(TableError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeRoundError {
    Scheduler(SchedulerError),
    ItemBufferTooSmall,
    BlockTableBufferTooSmall,
    OutputBufferTooSmall,
    MissingInputToken,
    Backend(RaggedBackendError),
    BackendLaunchCount,
    OutputIdentityMismatch,
}

impl From<SchedulerError> for DecodeRoundError {
    fn from(value: SchedulerError) -> Self {
        Self::Scheduler(value)
    }
}

impl From<RaggedBackendError> for DecodeRoundError {
    fn from(value: RaggedBackendError) -> Self {
        Self::Backend(value)
    }
}

impl From<PrefixKvError> for SchedulerError {
    fn from(value: PrefixKvError) -> Self {
        Self::Prefix(value)
    }
}

impl From<TableError> for SchedulerError {
    fn from(value: TableError) -> Self {
        Self::Table(value)
    }
}

struct RequestSlot {
    request_id: u64,
    state: RequestState,
    token_count: u32,
    prefix_tokens: u32,
    next_token_id: u32,
    token_ready: bool,
    blocks: SequenceBlockTable,
}

impl RequestSlot {
    fn new(logical_pages: u32) -> Self {
        Self {
            request_id: 0,
            state: RequestState::Empty,
            token_count: 0,
            prefix_tokens: 0,
            next_token_id: 0,
            token_ready: false,
            blocks: SequenceBlockTable::new(logical_pages),
        }
    }

    fn clear(&mut self) {
        self.request_id = 0;
        self.state = RequestState::Empty;
        self.token_count = 0;
        self.prefix_tokens = 0;
        self.next_token_id = 0;
        self.token_ready = false;
    }
}

/// Fixed-request-capacity scheduler. `REQUESTS` is the hard concurrent request bound.
pub struct RequestScheduler<const REQUESTS: usize> {
    slots: [RequestSlot; REQUESTS],
}

impl<const REQUESTS: usize> RequestScheduler<REQUESTS> {
    /// Cold construction; allocates each fixed-capacity logical page table exactly once.
    pub fn new(logical_pages: u32) -> Self {
        Self {
            slots: std::array::from_fn(|_| RequestSlot::new(logical_pages)),
        }
    }

    pub fn active_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.state != RequestState::Empty)
            .count()
    }

    /// Admit a request and atomically attach graph-derived prefix pages when available.
    pub fn admit_with_prefix<const ENTRIES: usize, const PAGES: usize>(
        &mut self,
        request_id: u64,
        identity: Option<PrefixIdentity>,
        prefixes: &PrefixKvStore<ENTRIES, PAGES>,
        pool: &mut BlockPool,
    ) -> Result<Admission, SchedulerError> {
        if self
            .slots
            .iter()
            .any(|slot| slot.state != RequestState::Empty && slot.request_id == request_id)
        {
            return Err(SchedulerError::DuplicateRequest);
        }
        let index = self
            .slots
            .iter()
            .position(|slot| slot.state == RequestState::Empty)
            .ok_or(SchedulerError::Full)?;
        let prefix_tokens = match identity {
            Some(identity) => prefixes
                .attach(identity, &mut self.slots[index].blocks, pool)?
                .unwrap_or(0),
            None => 0,
        };
        let slot = &mut self.slots[index];
        slot.request_id = request_id;
        slot.state = if prefix_tokens == 0 {
            RequestState::Prefill
        } else {
            RequestState::Decode
        };
        slot.token_count = prefix_tokens;
        slot.prefix_tokens = prefix_tokens;
        Ok(Admission {
            slot: index as u16,
            prefix_hit: prefix_tokens != 0,
            prefix_tokens,
        })
    }

    /// Make a logical page writable. A `Copy` result instructs the backend to copy the physical
    /// KV page before writing the next token.
    pub fn ensure_writable(
        &mut self,
        request_id: u64,
        logical_page: u32,
        pool: &mut BlockPool,
    ) -> Result<CopyOnWrite, SchedulerError> {
        let slot = self.find_mut(request_id)?;
        slot.blocks
            .ensure_writable(logical_page, pool)
            .map_err(Into::into)
    }

    pub fn mark_prefill_complete(&mut self, request_id: u64) -> Result<(), SchedulerError> {
        self.find_mut(request_id)?.state = RequestState::Decode;
        Ok(())
    }

    /// Publish the token consumed by the request's next decode step.
    pub fn seed_decode_token(
        &mut self,
        request_id: u64,
        token_id: u32,
    ) -> Result<(), SchedulerError> {
        let slot = self.find_mut(request_id)?;
        slot.next_token_id = token_id;
        slot.token_ready = true;
        Ok(())
    }

    pub fn record_token(&mut self, request_id: u64) -> Result<u32, SchedulerError> {
        let slot = self.find_mut(request_id)?;
        slot.token_count = slot.token_count.saturating_add(1);
        Ok(slot.token_count)
    }

    /// Write all runnable requests into caller storage in stable slot order.
    pub fn runnable_into(&self, out: &mut [RequestView]) -> Result<usize, SchedulerError> {
        let required = self.active_count();
        if out.len() < required {
            return Err(SchedulerError::OutputTooSmall);
        }
        let mut written = 0usize;
        for (index, slot) in self.slots.iter().enumerate() {
            if slot.state == RequestState::Empty {
                continue;
            }
            out[written] = RequestView {
                request_id: slot.request_id,
                slot: index as u16,
                state: slot.state,
                token_count: slot.token_count,
                prefix_tokens: slot.prefix_tokens,
            };
            written += 1;
        }
        Ok(written)
    }

    /// Execute one ragged decode round with exactly one backend call.
    ///
    /// Active decode requests are lowered in stable scheduler-slot order. Their logical page
    /// tables are concatenated into `block_table_scratch`; each item carries its table offset.
    /// Outputs are identity-checked as a complete batch before scheduler state is mutated.
    pub fn execute_decode_round<B: RaggedDecodeBackend>(
        &mut self,
        backend: &mut B,
        item_scratch: &mut [RaggedBatchItem],
        block_table_scratch: &mut [u32],
        output_scratch: &mut [RaggedBatchOutput],
    ) -> Result<RaggedBatchReceipt, DecodeRoundError> {
        let decode_count = self
            .slots
            .iter()
            .filter(|slot| slot.state == RequestState::Decode)
            .count();
        if item_scratch.len() < decode_count {
            return Err(DecodeRoundError::ItemBufferTooSmall);
        }
        if output_scratch.len() < decode_count {
            return Err(DecodeRoundError::OutputBufferTooSmall);
        }
        let required_table_entries = self
            .slots
            .iter()
            .filter(|slot| slot.state == RequestState::Decode)
            .map(|slot| slot.blocks.entries().len())
            .sum::<usize>();
        if block_table_scratch.len() < required_table_entries {
            return Err(DecodeRoundError::BlockTableBufferTooSmall);
        }

        let mut item_count = 0usize;
        let mut table_offset = 0usize;
        for (index, slot) in self.slots.iter().enumerate() {
            if slot.state != RequestState::Decode {
                continue;
            }
            if !slot.token_ready {
                return Err(DecodeRoundError::MissingInputToken);
            }
            let table = slot.blocks.entries();
            block_table_scratch[table_offset..table_offset + table.len()].copy_from_slice(table);
            item_scratch[item_count] = RaggedBatchItem {
                request_id: slot.request_id,
                slot: index as u32,
                token_id: slot.next_token_id,
                position: slot.token_count,
                block_table_offset: table_offset as u32,
                logical_pages: table.len() as u32,
                _reserved: 0,
            };
            item_count += 1;
            table_offset += table.len();
        }
        if item_count == 0 {
            return Ok(RaggedBatchReceipt::default());
        }

        let receipt = backend.execute_ragged(
            &item_scratch[..item_count],
            &block_table_scratch[..table_offset],
            &mut output_scratch[..item_count],
        )?;
        if receipt.batch_size != item_count as u32 {
            return Err(DecodeRoundError::OutputIdentityMismatch);
        }
        if receipt.backend_launches != 1 {
            return Err(DecodeRoundError::BackendLaunchCount);
        }
        for (item, output) in item_scratch[..item_count]
            .iter()
            .zip(&output_scratch[..item_count])
        {
            if output.request_id != item.request_id || output.slot != item.slot {
                return Err(DecodeRoundError::OutputIdentityMismatch);
            }
        }
        for output in &output_scratch[..item_count] {
            let slot = &mut self.slots[output.slot as usize];
            slot.next_token_id = output.next_token_id;
            slot.token_count = slot.token_count.saturating_add(1);
            slot.token_ready = true;
        }
        Ok(receipt)
    }

    /// Cancellation and normal completion have identical deterministic resource release.
    pub fn finish(&mut self, request_id: u64, pool: &mut BlockPool) -> Result<(), SchedulerError> {
        let slot = self.find_mut(request_id)?;
        slot.blocks.release_all(pool)?;
        slot.clear();
        Ok(())
    }

    fn find_mut(&mut self, request_id: u64) -> Result<&mut RequestSlot, SchedulerError> {
        self.slots
            .iter_mut()
            .find(|slot| slot.state != RequestState::Empty && slot.request_id == request_id)
            .ok_or(SchedulerError::UnknownRequest)
    }
}
