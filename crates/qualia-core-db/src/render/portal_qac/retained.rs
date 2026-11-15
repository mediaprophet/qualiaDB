use super::{provenance_role_str, Error, Limits, Metadata, PcmWindow, StftWindow};
use crate::audio::qac::signal;
use crate::q42::{
    field_envelope::{Family, StructuralView},
    field_semantics::{field, NodeState, Scratch},
};
use std::{mem::size_of, ops::Range};

/// Owns exactly the admitted source. No self-reference or unchecked view constructor.
/// One selected entry is admitted together with all supported document entries.
/// Re-selecting requires a new cold admission; windows only borrow admitted bytes.
pub struct RetainedQac {
    source: Box<[u8]>,
    payload: Range<usize>,
    metadata: Metadata,
    limits: Limits,
}
impl RetainedQac {
    pub fn open(source: Box<[u8]>, selected_entry: u32, limits: Limits) -> Result<Self, Error> {
        if source.len() as u64 > limits.envelope.file_bytes || source.len() > limits.memory_bytes {
            return Err(Error::Limit);
        }
        let view = StructuralView::open(&source, Family::QacV1Candidate, limits.envelope)?;
        if selected_entry == 0 || selected_entry > view.entry_count() {
            return Err(Error::Index);
        }
        let entry = view.entry(selected_entry)?;
        if ![0x100, 0x101].contains(&entry.mode()) {
            return Err(Error::Unsupported);
        }
        let blocks = view.blocks().count();
        let scratch_bytes = blocks
            .checked_add(view.entry_count() as usize)
            .and_then(|n| n.checked_mul(size_of::<NodeState>()))
            .ok_or(Error::Overflow)?;
        // Every copied string originates in metadata; allow six complete metadata
        // copies conservatively, plus fixed owned state and transient graph scratch.
        let memory = source
            .len()
            .checked_add(scratch_bytes)
            .and_then(|n| n.checked_add(view.metadata_bytes().len().checked_mul(6)?))
            .and_then(|n| n.checked_add(size_of::<Self>()))
            .ok_or(Error::Overflow)?;
        if memory > limits.memory_bytes {
            return Err(Error::Limit);
        }
        if view.entry_count() > limits.signal.entries {
            return Err(Error::Limit);
        }
        let mut metadata_scratch = vec![NodeState::default(); blocks];
        let mut entry_scratch = vec![NodeState::default(); view.entry_count() as usize];
        let selected = signal::admit(
            &view,
            selected_entry,
            &mut Scratch {
                metadata: &mut metadata_scratch,
                entries: &mut entry_scratch,
            },
            limits.shared,
            limits.signal,
        )?;
        let start = usize::try_from(entry.payload_offset()).map_err(|_| Error::Overflow)?;
        let length = usize::try_from(entry.stored_bytes()).map_err(|_| Error::Overflow)?;
        let end = start.checked_add(length).ok_or(Error::Overflow)?;
        let payload = source.get(start..end).ok_or(Error::Invariant)?;
        if payload.len() != selected.as_bytes().len()
            || payload.as_ptr() != selected.as_bytes().as_ptr()
        {
            return Err(Error::Invariant);
        }
        let descriptor = view
            .blocks()
            .find(|b| b.id() == entry.descriptor_id())
            .ok_or(Error::Invariant)?;
        let provenance_id = u32::from_le_bytes(
            field(descriptor, 3)
                .ok_or(Error::Invariant)?
                .value()
                .try_into()
                .map_err(|_| Error::Invariant)?,
        );
        let provenance = view
            .blocks()
            .find(|b| b.id() == provenance_id)
            .ok_or(Error::Invariant)?;
        let role_code = u32::from_le_bytes(
            field(provenance, 1)
                .ok_or(Error::Invariant)?
                .value()
                .try_into()
                .map_err(|_| Error::Invariant)?,
        );
        let epistemic_role = provenance_role_str(role_code).ok_or(Error::Invariant)?;
        let model = field(descriptor, 12).ok_or(Error::Invariant)?;
        let metadata = Metadata {
            selected_entry,
            descriptor_id: entry.descriptor_id(),
            clock_id: u32::from_le_bytes(
                field(descriptor, 1)
                    .ok_or(Error::Invariant)?
                    .value()
                    .try_into()
                    .map_err(|_| Error::Invariant)?,
            ),
            provenance_id,
            epistemic_role,
            stream_id: selected.stream_id(),
            frame_id: selected.frame_id(),
            mode: selected.mode(),
            scalar: selected.scalar(),
            channels: selected.channels(),
            bins: selected.bins(),
            records: selected.records(),
            timing: selected.timing().ok_or(Error::Invariant)?,
            transform: selected.transform(),
            window_coefficients_le: selected.window_coefficients_le().map(Into::into),
            channel_order: selected.channel_order().into(),
            purpose: selected.purpose().into(),
            relation: selected.relation().into(),
            quantity: selected.quantity().into(),
            units: selected.units().into(),
            model_uri: std::str::from_utf8(model.value())
                .map_err(|_| Error::Invariant)?
                .into(),
            units_per_stored_sample: selected.units_per_stored_sample(),
            stored_offset: selected.stored_offset(),
            index_digest: view
                .index_digest()
                .try_into()
                .map_err(|_| Error::Invariant)?,
        };
        Ok(Self {
            source,
            payload: start..end,
            metadata,
            limits,
        })
    }
    pub fn source_bytes(&self) -> &[u8] {
        &self.source
    }
    pub fn selected_payload_bytes(&self) -> &[u8] {
        &self.source[self.payload.clone()]
    }
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }
    pub fn time_at(&self, record: u64) -> Result<signal::ExactTime, Error> {
        if record >= self.metadata.records {
            return Err(Error::Index);
        }
        Ok(self.metadata.timing.time_at(record)?)
    }
    pub fn pcm_window(&self, start: u64, count: u64, channel: u32) -> Result<PcmWindow<'_>, Error> {
        if self.metadata.mode != 0x100 {
            return Err(Error::Unsupported);
        }
        Ok(PcmWindow {
            inner: self.window(start, count, channel)?,
        })
    }
    pub fn stft_window(
        &self,
        start: u64,
        count: u64,
        channel: u32,
    ) -> Result<StftWindow<'_>, Error> {
        if self.metadata.mode != 0x101 {
            return Err(Error::Unsupported);
        }
        Ok(StftWindow {
            inner: self.window(start, count, channel)?,
        })
    }
    fn window(
        &self,
        start: u64,
        count: u64,
        channel: u32,
    ) -> Result<super::window::Window<'_>, Error> {
        if channel >= self.metadata.channels
            || start > self.metadata.records
            || count > self.metadata.records - start
        {
            return Err(Error::Index);
        }
        let samples = count
            .checked_mul(u64::from(self.metadata.bins))
            .ok_or(Error::Overflow)?;
        if count > self.limits.window_records || samples > self.limits.window_samples {
            return Err(Error::Limit);
        }
        let stride = u64::from(self.metadata.channels)
            .checked_mul(self.metadata.bins.into())
            .and_then(|n| n.checked_mul(self.metadata.scalar.bytes().into()))
            .ok_or(Error::Overflow)?;
        let begin = usize::try_from(start.checked_mul(stride).ok_or(Error::Overflow)?)
            .map_err(|_| Error::Overflow)?;
        let n = usize::try_from(count.checked_mul(stride).ok_or(Error::Overflow)?)
            .map_err(|_| Error::Overflow)?;
        let bytes = self
            .selected_payload_bytes()
            .get(begin..begin.checked_add(n).ok_or(Error::Overflow)?)
            .ok_or(Error::Invariant)?;
        Ok(super::window::Window {
            bytes,
            metadata: &self.metadata,
            start,
            count,
            channel,
        })
    }
}
