//! Production QBDL I/O over an established [`ProtectedChannel`].
//!
//! [`super::session::Session`] remains a framing fixture: a caller-supplied
//! transcript plus raw [`Bearer`] send/recv is not AEAD. This type takes the
//! handshake-derived transcript from the channel and seals each carrier chunk
//! as QSession AAD. WouldBlock retries drop ledger and authority guards first.
use super::session::{ServiceIo, SessionOwner};
use super::{from_carrier, from_qdnf, to_carrier, Error};
use crate::crypto::network::authority::PinAuthority;
use crate::net::peer::replication::qbdl::carrier::{
    Assembly, CarrierError, MessageSequence, Progress,
};
use crate::net::peer::replication::qbdl::wire::{
    encode, Header, Kind, Message, ServiceRecordHeader, MAX_LEAF_RECORD_BYTES, MAX_RECORD_BYTES,
};
use crate::net::qdnf::bearer::contract::Bearer;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::handshake::endpoint::ProtectedChannel;
use crate::net::qdnf::types::StrongDigest;
use std::time::{Duration, Instant};

const IO_POLL_MS: u64 = 1;
const IO_RETRY_SECS: u64 = 5;
const AAD_BYTES: usize = 88;
const CHUNK_BYTES: usize = 1024;

pub struct ProtectedSession<'a, 's, B: Bearer> {
    channel: &'s mut ProtectedChannel<'a, B>,
    pins: &'s PinAuthority,
    clock: &'s dyn Fn() -> u64,
    deadline: u64,
    sequence: MessageSequence,
    assembly: Assembly<'s>,
    record: &'s mut [u8],
    closed: bool,
    cancelled: bool,
    next_transfer: u64,
    next_credit: u64,
    role: u8,
}

enum Recv<T> {
    Body(T),
    Cancel,
    Close,
}

impl<'a, 's, B: Bearer> ProtectedSession<'a, 's, B> {
    /// Construct only from an established channel. The transcript is the
    /// handshake value; arbitrary digest bytes cannot manufacture this session.
    pub fn attach(
        channel: &'s mut ProtectedChannel<'a, B>,
        pins: &'s PinAuthority,
        clock: &'s dyn Fn() -> u64,
        deadline: u64,
        role: u8,
        owner: &'s mut SessionOwner,
    ) -> Result<Self, Error> {
        if role == 0 {
            return Err(Error::Malformed);
        }
        if (clock)() >= deadline {
            return Err(Error::Expired);
        }
        let transcript = channel.transcript();
        if transcript == StrongDigest::ZERO {
            return Err(Error::Association);
        }
        let (data, control, record) = owner.lanes();
        Ok(Self {
            channel,
            pins,
            clock,
            deadline,
            sequence: MessageSequence::new(transcript).map_err(from_carrier)?,
            assembly: Assembly::new(transcript, data, control).map_err(from_carrier)?,
            record,
            closed: false,
            cancelled: false,
            next_transfer: 0,
            next_credit: 0,
            role,
        })
    }

    pub fn closed(&self) -> bool {
        self.closed || self.cancelled
    }

    pub(crate) fn fail_close(&mut self, e: Error) -> Error {
        self.closed = true;
        self.cancelled = true;
        self.assembly.abort();
        e
    }

    fn require_live(&self) -> Result<(), Error> {
        if self.closed {
            Err(Error::Closed)
        } else if self.cancelled {
            Err(Error::Cancelled)
        } else if (self.clock)() >= self.deadline {
            Err(Error::Expired)
        } else {
            Ok(())
        }
    }

    pub(crate) fn next_transfer_id(&mut self) -> Result<[u8; 16], Error> {
        let n = self.next_transfer.checked_add(1).ok_or(Error::Overflow)?;
        self.next_transfer = n;
        let mut id = [0u8; 16];
        id[0] = self.role;
        id[8..].copy_from_slice(&n.to_be_bytes());
        Ok(id)
    }

    pub(crate) fn next_credit_serial(&mut self) -> Result<u64, Error> {
        let n = self.next_credit.checked_add(1).ok_or(Error::Overflow)?;
        self.next_credit = n;
        Ok(n)
    }

    pub(crate) fn send_message(&mut self, header: &Header, body: &[u8]) -> Result<(), Error> {
        if let Err(e) = self.require_live() {
            return Err(self.fail_close(e));
        }
        let n = match encode(header, body, self.record) {
            Ok(n) => n,
            Err(e) => return Err(self.fail_close(e.into())),
        };
        let channel = &mut *self.channel;
        let pins = self.pins;
        let clock = self.clock;
        let deadline = self.deadline;
        let mut pending = match self.sequence.begin(&self.record[..n]) {
            Ok(p) => p,
            Err(e) => return Err(self.fail_close(from_carrier(e))),
        };
        loop {
            let (aad, chunk_buf, chunk_n) = match pending.current() {
                None => break,
                Some((record, chunk)) => {
                    let mut aad = [0u8; AAD_BYTES];
                    if let Err(e) = record.encode(&mut aad) {
                        return Err(self.fail_close(e.into()));
                    }
                    let chunk_n = chunk.len();
                    if chunk_n > CHUNK_BYTES {
                        return Err(self.fail_close(Error::Capacity));
                    }
                    let mut chunk_buf = [0u8; CHUNK_BYTES];
                    chunk_buf[..chunk_n].copy_from_slice(chunk);
                    (aad, chunk_buf, chunk_n)
                }
            };
            if let Err(e) =
                Self::seal_or_wait(channel, pins, clock, deadline, &aad, &chunk_buf[..chunk_n])
            {
                return Err(self.fail_close(e));
            }
            if let Err(e) = Self::flush_or_wait(channel, pins, clock, deadline) {
                return Err(self.fail_close(e));
            }
            if let Err(e) = pending.drained() {
                return Err(self.fail_close(from_carrier(e)));
            }
        }
        Ok(())
    }

    fn seal_or_wait(
        channel: &mut ProtectedChannel<'a, B>,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
        deadline: u64,
        aad: &[u8; AAD_BYTES],
        chunk: &[u8],
    ) -> Result<(), Error> {
        let cap = Instant::now() + Duration::from_secs(IO_RETRY_SECS);
        loop {
            if (clock)() >= deadline {
                return Err(Error::Expired);
            }
            match Self::try_queue(channel, pins, clock, aad, chunk) {
                Ok(()) => return Ok(()),
                Err(QdnfError::WouldBlock) => {
                    match Self::try_flush(channel, pins, clock) {
                        Ok(()) | Err(QdnfError::WouldBlock) => {}
                        Err(e) => return Err(from_qdnf(e)),
                    }
                    if Instant::now() >= cap || (clock)() >= deadline {
                        return Err(Error::Expired);
                    }
                    std::thread::sleep(Duration::from_millis(IO_POLL_MS));
                }
                Err(e) => return Err(from_qdnf(e)),
            }
        }
    }

    fn flush_or_wait(
        channel: &mut ProtectedChannel<'a, B>,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
        deadline: u64,
    ) -> Result<(), Error> {
        let cap = Instant::now() + Duration::from_secs(IO_RETRY_SECS);
        loop {
            if (clock)() >= deadline {
                return Err(Error::Expired);
            }
            match Self::try_flush(channel, pins, clock) {
                Ok(()) => return Ok(()),
                Err(QdnfError::WouldBlock) => {
                    if Instant::now() >= cap || (clock)() >= deadline {
                        return Err(Error::Expired);
                    }
                    std::thread::sleep(Duration::from_millis(IO_POLL_MS));
                }
                Err(e) => return Err(from_qdnf(e)),
            }
        }
    }

    fn try_queue(
        channel: &mut ProtectedChannel<'a, B>,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
        aad: &[u8; AAD_BYTES],
        chunk: &[u8],
    ) -> Result<(), QdnfError> {
        channel.with_resources(|mut io, _ledger| {
            io.with_current(pins, clock, |mut frame, current| {
                frame.queue_record(current, pins, clock, aad, chunk)?;
                Ok(())
            })
        })
    }

    fn try_flush(
        channel: &mut ProtectedChannel<'a, B>,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<(), QdnfError> {
        if channel.pending_frame_bytes() == 0 {
            return Ok(());
        }
        channel.with_resources(|mut io, _ledger| {
            io.with_current(pins, clock, |mut frame, current| {
                frame.send_pending(current, pins, clock)
            })
        })
    }

    pub(crate) fn recv_leaf(
        &mut self,
        payload: &mut [u8],
        proof: &mut [u8],
    ) -> Result<(Header, usize, usize), Error> {
        self.recv_complete(
            |h, total, _| {
                if h.kind != Kind::LeafData {
                    Err(Error::State)
                } else if total > MAX_LEAF_RECORD_BYTES {
                    Err(Error::Capacity)
                } else {
                    Ok(())
                }
            },
            |msg| split_leaf(msg, payload, proof),
        )
    }

    pub(crate) fn recv_control(&mut self) -> Result<(Header, [u8; 80], usize), Error> {
        self.recv_complete(
            |h, total, _| {
                if matches!(
                    h.kind,
                    Kind::LeafData | Kind::DescriptorPage | Kind::Describe
                ) {
                    Err(Error::State)
                } else if total > 304 {
                    Err(Error::Capacity)
                } else {
                    Ok(())
                }
            },
            |msg| {
                if msg.body.len() > 80 {
                    return Err(Error::Capacity);
                }
                let mut body = [0u8; 80];
                body[..msg.body.len()].copy_from_slice(msg.body);
                Ok((msg.header, body, msg.body.len()))
            },
        )
    }

    pub(crate) fn recv_describe(&mut self) -> Result<(Header, [u8; 96], usize), Error> {
        self.recv_complete(
            |h, total, _| {
                if h.kind != Kind::Describe {
                    Err(Error::State)
                } else if total > 304 {
                    Err(Error::Capacity)
                } else {
                    Ok(())
                }
            },
            |msg| {
                if msg.body.len() != 96 {
                    return Err(Error::Malformed);
                }
                let mut body = [0u8; 96];
                body.copy_from_slice(msg.body);
                Ok((msg.header, body, 96))
            },
        )
    }

    pub(crate) fn recv_descriptor(&mut self, page: &mut [u8]) -> Result<(Header, usize), Error> {
        self.recv_complete(
            |h, total, _| {
                if h.kind != Kind::DescriptorPage {
                    Err(Error::State)
                } else if total > MAX_RECORD_BYTES {
                    Err(Error::Capacity)
                } else {
                    Ok(())
                }
            },
            |msg| {
                if page.len() < msg.body.len() {
                    return Err(Error::Capacity);
                }
                page[..msg.body.len()].copy_from_slice(msg.body);
                Ok((msg.header, msg.body.len()))
            },
        )
    }

    fn recv_complete<T>(
        &mut self,
        mut check: impl FnMut(&Header, usize, bool) -> Result<(), Error>,
        consume: impl for<'b> FnOnce(Message<'b>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        if let Err(e) = self.require_live() {
            return Err(self.fail_close(e));
        }
        let mut consume = Some(consume);
        loop {
            let (aad, chunk, n) =
                match Self::recv_chunk(self.channel, self.pins, self.clock, self.deadline) {
                    Ok(v) => v,
                    Err(e) => return Err(self.fail_close(e)),
                };
            let record = match ServiceRecordHeader::parse(&aad) {
                Ok(r) => r,
                Err(e) => return Err(self.fail_close(e.into())),
            };
            let outcome = self.assembly.accept(
                record,
                &chunk[..n],
                |h, total, first| {
                    if matches!(h.kind, Kind::Cancel | Kind::Close) {
                        return Ok(());
                    }
                    check(h, total, first).map_err(to_carrier)
                },
                |msg| match msg.header.kind {
                    Kind::Cancel => Ok(Recv::Cancel),
                    Kind::Close => Ok(Recv::Close),
                    _ => consume
                        .take()
                        .ok_or(CarrierError::Order)
                        .and_then(|c| c(msg).map(Recv::Body).map_err(to_carrier)),
                },
            );
            match outcome {
                Ok((Progress::Partial, None)) => continue,
                Ok((Progress::Complete, Some(Recv::Body(v)))) => return Ok(v),
                Ok((Progress::Complete, Some(Recv::Cancel))) => {
                    return Err(self.fail_close(Error::Cancelled));
                }
                Ok((Progress::Complete, Some(Recv::Close))) => {
                    return Err(self.fail_close(Error::Closed));
                }
                Ok(_) => return Err(self.fail_close(Error::State)),
                Err(e) => return Err(self.fail_close(from_carrier(e))),
            }
        }
    }

    fn recv_chunk(
        channel: &mut ProtectedChannel<'a, B>,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
        deadline: u64,
    ) -> Result<([u8; AAD_BYTES], [u8; CHUNK_BYTES], usize), Error> {
        let cap = Instant::now() + Duration::from_secs(IO_RETRY_SECS);
        loop {
            if (clock)() >= deadline {
                return Err(Error::Expired);
            }
            match Self::try_recv(channel, pins, clock) {
                Ok(v) => return Ok(v),
                Err(QdnfError::WouldBlock) => {
                    if Instant::now() >= cap || (clock)() >= deadline {
                        return Err(Error::Expired);
                    }
                    std::thread::sleep(Duration::from_millis(IO_POLL_MS));
                }
                Err(e) => return Err(from_qdnf(e)),
            }
        }
    }

    fn try_recv(
        channel: &mut ProtectedChannel<'a, B>,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<([u8; AAD_BYTES], [u8; CHUNK_BYTES], usize), QdnfError> {
        channel.with_resources(|mut io, _ledger| {
            let mut aad = [0u8; AAD_BYTES];
            let mut chunk = [0u8; CHUNK_BYTES];
            let n = io.receive_record(pins, clock, &mut aad, &mut chunk)?;
            Ok((aad, chunk, n))
        })
    }
}

impl<B: Bearer> ServiceIo for ProtectedSession<'_, '_, B> {
    fn send_message(&mut self, header: &Header, body: &[u8]) -> Result<(), Error> {
        ProtectedSession::send_message(self, header, body)
    }
    fn recv_control(&mut self) -> Result<(Header, [u8; 80], usize), Error> {
        ProtectedSession::recv_control(self)
    }
    fn recv_describe(&mut self) -> Result<(Header, [u8; 96], usize), Error> {
        ProtectedSession::recv_describe(self)
    }
    fn recv_descriptor(&mut self, page: &mut [u8]) -> Result<(Header, usize), Error> {
        ProtectedSession::recv_descriptor(self, page)
    }
    fn recv_leaf(
        &mut self,
        payload: &mut [u8],
        proof: &mut [u8],
    ) -> Result<(Header, usize, usize), Error> {
        ProtectedSession::recv_leaf(self, payload, proof)
    }
    fn fail_close(&mut self, e: Error) -> Error {
        ProtectedSession::fail_close(self, e)
    }
    fn closed(&self) -> bool {
        ProtectedSession::closed(self)
    }
    fn next_transfer_id(&mut self) -> Result<[u8; 16], Error> {
        ProtectedSession::next_transfer_id(self)
    }
    fn next_credit_serial(&mut self) -> Result<u64, Error> {
        ProtectedSession::next_credit_serial(self)
    }
}

fn split_leaf(
    msg: Message<'_>,
    payload: &mut [u8],
    proof: &mut [u8],
) -> Result<(Header, usize, usize), Error> {
    if msg.body.len() < 8 {
        return Err(Error::Malformed);
    }
    let p = u32::from_be_bytes(msg.body[..4].try_into().map_err(|_| Error::Malformed)?) as usize;
    let q = u16::from_be_bytes(msg.body[4..6].try_into().map_err(|_| Error::Malformed)?) as usize;
    if msg.body.len() != 8 + p + q {
        return Err(Error::Malformed);
    }
    if payload.len() < p || proof.len() < q {
        return Err(Error::Capacity);
    }
    payload[..p].copy_from_slice(&msg.body[8..8 + p]);
    proof[..q].copy_from_slice(&msg.body[8 + p..8 + p + q]);
    Ok((msg.header, p, q))
}
