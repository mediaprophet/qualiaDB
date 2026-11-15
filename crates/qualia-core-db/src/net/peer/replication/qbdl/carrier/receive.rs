use super::super::wire::{
    Header, Kind, Message, ObjectSelector, ServiceRecordHeader, MAX_LEAF_RECORD_BYTES,
    MAX_RECORD_BYTES, PREFIX_BYTES,
};
use super::{transcript, CarrierError as E};
use crate::net::qdnf::types::StrongDigest;
use zeroize::Zeroize;
#[derive(Clone, Copy)]
struct Active {
    id: u128,
    total: usize,
    used: usize,
    header: Header,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Progress {
    Partial,
    Complete,
}
/// One data lane plus one separately supplied control lane. Never allocates.
/// Failed callbacks, framing errors and unwind erase both lanes and poison this
/// owner. Reconnect constructs a new owner under a fresh actual session.
pub(crate) struct Assembly<'a> {
    transcript: StrongDigest,
    data: &'a mut [u8],
    control: &'a mut [u8],
    active: Option<Active>,
    high_water: u128,
    closed: bool,
}
impl<'a> Assembly<'a> {
    pub(crate) fn new(
        transcript_id: StrongDigest,
        data: &'a mut [u8],
        control: &'a mut [u8],
    ) -> Result<Self, E> {
        transcript(transcript_id)?;
        if !(PREFIX_BYTES..=MAX_RECORD_BYTES).contains(&data.len()) || control.len() != 1024 {
            return Err(E::Capacity);
        }
        data.zeroize();
        control.zeroize();
        Ok(Self {
            transcript: transcript_id,
            data,
            control,
            active: None,
            high_water: 0,
            closed: false,
        })
    }
    pub(crate) fn abort(&mut self) {
        self.data.zeroize();
        self.control.zeroize();
        self.active = None;
        self.closed = true;
    }
    /// check runs BEFORE copying every chunk, including the first. It must
    /// compare actual outstanding credit, association and current policy;
    /// accepting a syntactic Header alone never establishes those facts.
    /// consume runs on a complete validated message only, while its borrowed
    /// bytes remain private. No message slice escapes this callback.
    pub(crate) fn accept<T>(
        &mut self,
        record: ServiceRecordHeader,
        bytes: &[u8],
        mut check: impl FnMut(&Header, usize, bool) -> Result<(), E>,
        consume: impl for<'b> FnOnce(Message<'b>) -> Result<T, E>,
    ) -> Result<(Progress, Option<T>), E> {
        self.with_attempt(|owner| owner.accept_inner(record, bytes, &mut check, consume))
    }
    /// Arm OUTSIDE the actual channel/application authority scope so an outer
    /// final expiry/revocation check also erases assembly. The inner consumer
    /// may stage private state only; this guard cannot undo external effects.
    pub(crate) fn with_attempt<T, X>(
        &mut self,
        effect: impl FnOnce(&mut Self) -> Result<T, X>,
    ) -> Result<T, X> {
        struct Guard<'s, 'a> {
            owner: &'s mut Assembly<'a>,
            success: bool,
        }
        impl Drop for Guard<'_, '_> {
            fn drop(&mut self) {
                if !self.success {
                    self.owner.abort();
                }
            }
        }
        let mut guard = Guard {
            owner: self,
            success: false,
        };
        let result = effect(guard.owner);
        guard.success = result.is_ok();
        result
    }
    fn accept_inner<T>(
        &mut self,
        record: ServiceRecordHeader,
        bytes: &[u8],
        check: &mut impl FnMut(&Header, usize, bool) -> Result<(), E>,
        consume: impl for<'b> FnOnce(Message<'b>) -> Result<T, E>,
    ) -> Result<(Progress, Option<T>), E> {
        if self.closed {
            return Err(E::Closed);
        }
        record.validate()?;
        if record.transcript != self.transcript || bytes.len() != record.chunk as usize {
            return Err(E::Association);
        }
        let id = u128::from_be_bytes(record.message_id);
        let total = record.total as usize;
        if record.offset != 0 {
            let active = self.active.as_mut().ok_or(E::Order)?;
            if active.id != id || active.total != total || active.used != record.offset as usize {
                return Err(E::Order);
            }
            check(&active.header, total, false)?;
            self.data[active.used..active.used + bytes.len()].copy_from_slice(bytes);
            active.used += bytes.len();
            if !record.final_chunk {
                return Ok((Progress::Partial, None));
            }
            let message = Message::parse(&self.data[..total])?;
            let output = consume(message)?;
            self.data.zeroize();
            self.active = None;
            return Ok((Progress::Complete, Some(output)));
        }
        if id <= self.high_water {
            return Err(E::Order);
        }
        let prefix = bytes.get(..PREFIX_BYTES).ok_or(E::Association)?;
        let header = Header::parse_prefix(prefix)?;
        let body = u32::from_be_bytes(prefix[180..184].try_into().unwrap()) as usize;
        if PREFIX_BYTES.checked_add(body) != Some(total) {
            return Err(E::Association);
        }
        let data_kind = matches!(header.kind, Kind::DescriptorPage | Kind::LeafData);
        if header.kind == Kind::LeafData && total > MAX_LEAF_RECORD_BYTES {
            return Err(E::Capacity);
        }
        if header.kind == Kind::DescriptorPage {
            let selector = ObjectSelector::parse(
                bytes
                    .get(PREFIX_BYTES..PREFIX_BYTES + 32)
                    .ok_or(E::Association)?,
                false,
            )?;
            if total != PREFIX_BYTES + 32 + selector.chunk as usize {
                return Err(E::Association);
            }
        }
        if data_kind {
            if self.active.is_some() {
                return Err(E::Order);
            }
            if total > self.data.len() {
                return Err(E::Capacity);
            }
        } else if total > 304 || !record.final_chunk {
            return Err(E::Capacity);
        }
        check(&header, total, true)?;
        self.high_water = id;
        if data_kind {
            self.data[..bytes.len()].copy_from_slice(bytes);
            if !record.final_chunk {
                self.active = Some(Active {
                    id,
                    total,
                    used: bytes.len(),
                    header,
                });
                return Ok((Progress::Partial, None));
            }
            let output = consume(Message::parse(&self.data[..total])?)?;
            self.data.zeroize();
            Ok((Progress::Complete, Some(output)))
        } else {
            self.control[..bytes.len()].copy_from_slice(bytes);
            let output = consume(Message::parse(&self.control[..total])?)?;
            self.control.zeroize();
            Ok((Progress::Complete, Some(output)))
        }
    }
}
impl Drop for Assembly<'_> {
    fn drop(&mut self) {
        self.abort();
    }
}
