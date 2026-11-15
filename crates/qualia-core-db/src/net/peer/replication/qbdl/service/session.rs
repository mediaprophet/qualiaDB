//! Carrier send/recv over a [`Bearer`]. Default is the in-process `ipc_pair`.
//! Named-pipe / UDS `FramedStream` also implements [`Bearer`]. WouldBlock on a
//! non-blocking OS pipe retries the same framed record / recv buffer.
//!
//! This is a framing fixture: [`Session::attach`] takes a caller-supplied
//! transcript and sends plaintext carrier bytes. Production I/O uses an
//! established `ProtectedChannel` (handshake transcript, AEAD AAD).
use super::{from_carrier, from_qdnf, to_carrier, Error};
use crate::bundle::stream::descriptor::CandidateIdentity;
use crate::net::peer::replication::qbdl::carrier::{
    Assembly, CarrierError, MessageSequence, Progress,
};
use crate::net::peer::replication::qbdl::wire::{
    encode, Header, Kind, Message, ServiceRecordHeader, MAX_LEAF_RECORD_BYTES, MAX_RECORD_BYTES,
    PREFIX_BYTES,
};
use crate::net::qdnf::bearer::contract::{Bearer, RecvMeta};
use crate::net::qdnf::bearer::IpcEndpoint;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, StrongDigest};
use std::time::{Duration, Instant};

pub const ROLE_CLIENT: u8 = 1;
pub const ROLE_PROVIDER: u8 = 2;
pub(crate) const DEADLINE: u64 = 90;
const FRAME_BYTES: usize = ServiceRecordHeader::BYTES + 1024;
const IO_RETRY_SECS: u64 = 5;
const IO_POLL_MS: u64 = 1;

pub struct SessionOwner {
    data: [u8; MAX_RECORD_BYTES],
    control: [u8; 1024],
    record: [u8; MAX_RECORD_BYTES],
}

impl SessionOwner {
    pub fn new() -> Self {
        Self {
            data: [0; MAX_RECORD_BYTES],
            control: [0; 1024],
            record: [0; MAX_RECORD_BYTES],
        }
    }

    pub(crate) fn lanes(
        &mut self,
    ) -> (
        &mut [u8; MAX_RECORD_BYTES],
        &mut [u8; 1024],
        &mut [u8; MAX_RECORD_BYTES],
    ) {
        (&mut self.data, &mut self.control, &mut self.record)
    }
}

/// Framing fixture [`Session`] and production [`super::ProtectedSession`] share this I/O.
pub trait ServiceIo {
    fn send_message(&mut self, header: &Header, body: &[u8]) -> Result<(), Error>;
    fn recv_control(&mut self) -> Result<(Header, [u8; 80], usize), Error>;
    fn recv_describe(&mut self) -> Result<(Header, [u8; 96], usize), Error>;
    fn recv_descriptor(&mut self, page: &mut [u8]) -> Result<(Header, usize), Error>;
    fn recv_leaf(
        &mut self,
        payload: &mut [u8],
        proof: &mut [u8],
    ) -> Result<(Header, usize, usize), Error>;
    fn fail_close(&mut self, e: Error) -> Error;
    fn closed(&self) -> bool;
    fn next_transfer_id(&mut self) -> Result<[u8; 16], Error>;
    fn next_credit_serial(&mut self) -> Result<u64, Error>;
}

pub struct Session<'a, B: Bearer = IpcEndpoint> {
    endpoint: B,
    peer: ObservedLocator,
    sequence: MessageSequence,
    assembly: Assembly<'a>,
    record: &'a mut [u8],
    closed: bool,
    cancelled: bool,
    next_transfer: u64,
    next_credit: u64,
    role: u8,
}

impl<'a, B: Bearer> Session<'a, B> {
    pub fn attach(
        endpoint: B,
        peer: ObservedLocator,
        transcript: StrongDigest,
        role: u8,
        owner: &'a mut SessionOwner,
    ) -> Result<Self, Error> {
        if role == 0 {
            return Err(Error::Malformed);
        }
        let (data, control, record) = owner.lanes();
        Ok(Self {
            endpoint,
            peer,
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

    pub fn profile(&self) -> BearerProfile {
        self.endpoint.profile()
    }

    pub fn closed(&self) -> bool {
        self.closed || self.cancelled
    }

    pub(crate) fn fail_close(&mut self, e: Error) -> Error {
        self.closed = true;
        self.cancelled = true;
        self.assembly.abort();
        let _ = self.endpoint.shutdown();
        e
    }

    fn require_live(&self) -> Result<(), Error> {
        if self.closed {
            Err(Error::Closed)
        } else if self.cancelled {
            Err(Error::Cancelled)
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
        let mut pending = match self.sequence.begin(&self.record[..n]) {
            Ok(p) => p,
            Err(e) => return Err(self.fail_close(from_carrier(e))),
        };
        let mut frame = [0u8; FRAME_BYTES];
        while let Some((record, chunk)) = pending.current() {
            let hdr = match record.encode(&mut frame) {
                Ok(h) => h,
                Err(e) => return Err(self.fail_close(e.into())),
            };
            let end = hdr + chunk.len();
            if end > frame.len() {
                return Err(self.fail_close(Error::Capacity));
            }
            frame[hdr..end].copy_from_slice(chunk);
            if let Err(e) = Self::send_frame(&mut self.endpoint, &self.peer, &frame[..end]) {
                return Err(self.fail_close(e));
            }
            if let Err(e) = pending.drained() {
                return Err(self.fail_close(from_carrier(e)));
            }
        }
        Ok(())
    }

    /// Retry the same sealed payload until the sink accepts it. Does not reseal
    /// or rewind. WouldBlock is not fail-close on a non-blocking pipe.
    fn send_frame(endpoint: &mut B, peer: &ObservedLocator, frame: &[u8]) -> Result<(), Error> {
        let deadline = Instant::now() + Duration::from_secs(IO_RETRY_SECS);
        loop {
            match endpoint.send(peer, frame) {
                Ok(_) => return Ok(()),
                Err(QdnfError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        return Err(Error::Expired);
                    }
                    std::thread::sleep(Duration::from_millis(IO_POLL_MS));
                }
                Err(e) => return Err(from_qdnf(e)),
            }
        }
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

    fn recv_complete<T>(
        &mut self,
        mut check: impl FnMut(&Header, usize, bool) -> Result<(), Error>,
        consume: impl for<'b> FnOnce(Message<'b>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        if let Err(e) = self.require_live() {
            return Err(self.fail_close(e));
        }
        let mut frame = [0u8; FRAME_BYTES];
        let mut consume = Some(consume);
        loop {
            let (n, meta) = match self.recv_frame(&mut frame) {
                Ok(v) => v,
                Err(e) => return Err(e),
            };
            if meta.observed_source.as_slice() != self.peer.as_slice() {
                return Err(self.fail_close(Error::Association));
            }
            if n < ServiceRecordHeader::BYTES {
                return Err(self.fail_close(Error::Malformed));
            }
            let record = match ServiceRecordHeader::parse(&frame[..ServiceRecordHeader::BYTES]) {
                Ok(r) => r,
                Err(e) => return Err(self.fail_close(e.into())),
            };
            let chunk = &frame[ServiceRecordHeader::BYTES..n];
            let outcome = self.assembly.accept(
                record,
                chunk,
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

    /// Recv into the same buffer while assembling. WouldBlock is not fail-close.
    fn recv_frame(&mut self, frame: &mut [u8]) -> Result<(usize, RecvMeta), Error> {
        let deadline = Instant::now() + Duration::from_secs(IO_RETRY_SECS);
        loop {
            match self.endpoint.recv(frame) {
                Ok(v) => return Ok(v),
                Err(QdnfError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        return Err(self.fail_close(Error::Expired));
                    }
                    std::thread::sleep(Duration::from_millis(IO_POLL_MS));
                }
                Err(e) => return Err(self.fail_close(from_qdnf(e))),
            }
        }
    }
}

impl<B: Bearer> ServiceIo for Session<'_, B> {
    fn send_message(&mut self, header: &Header, body: &[u8]) -> Result<(), Error> {
        Session::send_message(self, header, body)
    }
    fn recv_control(&mut self) -> Result<(Header, [u8; 80], usize), Error> {
        Session::recv_control(self)
    }
    fn recv_describe(&mut self) -> Result<(Header, [u8; 96], usize), Error> {
        Session::recv_describe(self)
    }
    fn recv_descriptor(&mut self, page: &mut [u8]) -> Result<(Header, usize), Error> {
        Session::recv_descriptor(self, page)
    }
    fn recv_leaf(
        &mut self,
        payload: &mut [u8],
        proof: &mut [u8],
    ) -> Result<(Header, usize, usize), Error> {
        Session::recv_leaf(self, payload, proof)
    }
    fn fail_close(&mut self, e: Error) -> Error {
        Session::fail_close(self, e)
    }
    fn closed(&self) -> bool {
        Session::closed(self)
    }
    fn next_transfer_id(&mut self) -> Result<[u8; 16], Error> {
        Session::next_transfer_id(self)
    }
    fn next_credit_serial(&mut self) -> Result<u64, Error> {
        Session::next_credit_serial(self)
    }
}

enum Recv<T> {
    Body(T),
    Cancel,
    Close,
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

pub(crate) fn associate(
    kind: Kind,
    identity: CandidateIdentity,
    policy_generation: u64,
    transfer_id: [u8; 16],
    entry: u32,
    leaf: u64,
    count: u32,
    credit_serial: u64,
    credit_bytes: u64,
) -> Header {
    associate_until(
        kind,
        identity,
        policy_generation,
        transfer_id,
        entry,
        leaf,
        count,
        credit_serial,
        credit_bytes,
        DEADLINE,
    )
}

pub(crate) fn associate_until(
    kind: Kind,
    identity: CandidateIdentity,
    policy_generation: u64,
    transfer_id: [u8; 16],
    entry: u32,
    leaf: u64,
    count: u32,
    credit_serial: u64,
    credit_bytes: u64,
    deadline: u64,
) -> Header {
    Header {
        kind,
        descriptor: identity.descriptor_id(),
        context: identity.context_id(),
        publication_generation: identity.generation(),
        policy_generation,
        cancellation: if matches!(kind, Kind::Cancel | Kind::Close) {
            1
        } else {
            0
        },
        transfer_id,
        entry,
        leaf,
        count,
        credit_serial,
        credit_bytes,
        deadline,
    }
}

pub(crate) fn check_identity(
    header: &Header,
    identity: CandidateIdentity,
    policy_generation: u64,
) -> Result<(), Error> {
    if header.descriptor != identity.descriptor_id()
        || header.context != identity.context_id()
        || header.publication_generation != identity.generation()
        || header.policy_generation != policy_generation
    {
        Err(Error::Identity)
    } else {
        Ok(())
    }
}

pub(crate) fn reason_body(reason: u16) -> [u8; 8] {
    let mut body = [0u8; 8];
    body[..2].copy_from_slice(&reason.to_be_bytes());
    body
}

pub fn encode_open_body(ordinal: u32, page: u32, out: &mut [u8]) -> Result<usize, Error> {
    if out.len() < 72 {
        return Err(Error::Capacity);
    }
    out[..72].fill(0);
    super::profile_limits().encode(&mut out[..64])?;
    out[64..68].copy_from_slice(&ordinal.to_be_bytes());
    out[68..72].copy_from_slice(&page.to_be_bytes());
    Ok(72)
}

pub(crate) fn encode_grant_body(ordinal: u32, page: u32, out: &mut [u8]) -> Result<usize, Error> {
    if out.len() < 80 {
        return Err(Error::Capacity);
    }
    out[..80].fill(0);
    out[..2].copy_from_slice(&2u16.to_be_bytes());
    super::profile_limits().encode(&mut out[8..72])?;
    out[72..76].copy_from_slice(&ordinal.to_be_bytes());
    out[76..80].copy_from_slice(&page.to_be_bytes());
    Ok(80)
}

pub(crate) fn parse_selection(body: &[u8], at: usize) -> Result<(u32, u32), Error> {
    let ordinal = u32::from_be_bytes(
        body.get(at..at + 4)
            .ok_or(Error::Malformed)?
            .try_into()
            .map_err(|_| Error::Malformed)?,
    );
    let page = u32::from_be_bytes(
        body.get(at + 4..at + 8)
            .ok_or(Error::Malformed)?
            .try_into()
            .map_err(|_| Error::Malformed)?,
    );
    Ok((ordinal, page))
}

const _: () = assert!(PREFIX_BYTES == 208);
