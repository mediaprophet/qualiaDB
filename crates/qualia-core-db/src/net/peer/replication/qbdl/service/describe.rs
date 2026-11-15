//! Describe / DescriptorPage bootstrap. Client binds [`CandidateIdentity`]
//! from an advertisement; it does not need a pre-built [`SelectedPlainEntry`].
//! Kind 1–12 stay frozen. Header pages are class 6 (64-byte HMC header);
//! index pages are class 7 slices of the live admitted footer; class 8 is a
//! QASD descriptor body page; class 9 is a QASD entry page.
use super::session::{associate_until, check_identity, ServiceIo, DEADLINE};
use super::{profile_limits, Error};
use crate::bundle::format::BUNDLE_HEADER_SIZE;
use crate::bundle::stream::descriptor::CandidateIdentity;
use crate::bundle::stream::index::{admit_header, ExtentLimit, IndexReadLimits};
use crate::bundle::stream::policy::LocalEffectControl;
use crate::net::peer::replication::qbdl::wire::{
    Header, Kind, Limits, ObjectSelector, MAX_LEAF_RECORD_BYTES, PREFIX_BYTES,
};
use crate::net::qdnf::types::StrongDigest;

const DESCRIBE_BODY: usize = 96;
const HEADER_PAGE_BODY: usize = 32 + BUNDLE_HEADER_SIZE;
const HEADER_PAGE_CREDIT: u64 = (PREFIX_BYTES + HEADER_PAGE_BODY) as u64;
const MAX_INDEX_CHUNK: u32 = (MAX_LEAF_RECORD_BYTES - PREFIX_BYTES - 32) as u32;
const MAX_PAGE_CHUNK: u32 = 65536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DescribeTicket {
    pub transfer_id: [u8; 16],
    pub credit_serial: u64,
    pub credit_bytes: u64,
    pub class: u8,
    pub number: u32,
    pub offset: u64,
    pub chunk: u32,
    pub total: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServedDescriptor {
    pub class: u8,
    pub offset: u64,
    pub length: usize,
}

pub struct DescribeClient {
    identity: CandidateIdentity,
    policy_generation: u64,
}

impl DescribeClient {
    pub fn bind(identity: CandidateIdentity, policy_generation: u64) -> Result<Self, Error> {
        if identity.descriptor_id() == StrongDigest::ZERO
            || identity.context_id() == StrongDigest::ZERO
        {
            return Err(Error::Malformed);
        }
        Ok(Self {
            identity,
            policy_generation,
        })
    }

    pub fn identity(&self) -> CandidateIdentity {
        self.identity
    }

    pub fn send_header_describe<S: ServiceIo>(
        &self,
        session: &mut S,
    ) -> Result<DescribeTicket, Error> {
        let selector = ObjectSelector {
            class: 1,
            number: 0,
            total: 0,
            offset: 0,
            chunk: 65536,
        };
        self.send_describe(session, selector, HEADER_PAGE_CREDIT, DEADLINE)
    }

    pub fn send_index_describe<S: ServiceIo>(
        &self,
        session: &mut S,
        offset: u64,
        chunk: u32,
        total: u64,
    ) -> Result<DescribeTicket, Error> {
        if chunk == 0 || chunk > MAX_INDEX_CHUNK {
            return Err(Error::Limit);
        }
        let selector = ObjectSelector {
            class: 7,
            number: 0,
            total,
            offset,
            chunk,
        };
        let credit = (PREFIX_BYTES + 32 + chunk as usize) as u64;
        self.send_describe(session, selector, credit, DEADLINE)
    }

    pub fn send_body_describe<S: ServiceIo>(
        &self,
        session: &mut S,
        offset: u64,
        chunk: u32,
        total: u64,
        number: u32,
    ) -> Result<DescribeTicket, Error> {
        self.send_object_describe(session, 8, offset, chunk, total, number)
    }

    pub fn send_entry_page_describe<S: ServiceIo>(
        &self,
        session: &mut S,
        offset: u64,
        chunk: u32,
        total: u64,
        number: u32,
    ) -> Result<DescribeTicket, Error> {
        self.send_object_describe(session, 9, offset, chunk, total, number)
    }

    fn send_object_describe<S: ServiceIo>(
        &self,
        session: &mut S,
        class: u8,
        offset: u64,
        chunk: u32,
        total: u64,
        number: u32,
    ) -> Result<DescribeTicket, Error> {
        if chunk == 0 || chunk > MAX_PAGE_CHUNK {
            return Err(Error::Limit);
        }
        let selector = ObjectSelector {
            class,
            number,
            total,
            offset,
            chunk,
        };
        let credit = (PREFIX_BYTES + 32 + chunk as usize) as u64;
        self.send_describe(session, selector, credit, DEADLINE)
    }

    fn send_describe<S: ServiceIo>(
        &self,
        session: &mut S,
        selector: ObjectSelector,
        credit_bytes: u64,
        deadline: u64,
    ) -> Result<DescribeTicket, Error> {
        let transfer_id = session.next_transfer_id()?;
        let credit_serial = session.next_credit_serial()?;
        let mut body = [0u8; DESCRIBE_BODY];
        let n = encode_describe_request(profile_limits(), selector, &mut body)?;
        let header = associate_until(
            Kind::Describe,
            self.identity,
            self.policy_generation,
            transfer_id,
            0,
            0,
            0,
            credit_serial,
            credit_bytes,
            deadline,
        );
        session.send_message(&header, &body[..n])?;
        Ok(DescribeTicket {
            transfer_id,
            credit_serial,
            credit_bytes,
            class: selector.class,
            number: selector.number,
            offset: selector.offset,
            chunk: selector.chunk,
            total: selector.total,
        })
    }

    pub fn recv_header_page<S: ServiceIo>(
        &self,
        session: &mut S,
        ticket: &DescribeTicket,
        out: &mut [u8; BUNDLE_HEADER_SIZE],
    ) -> Result<(), Error> {
        let mut page = [0u8; HEADER_PAGE_BODY];
        let (header, n) = session.recv_descriptor(&mut page)?;
        if let Err(e) = self.check_page(&header, ticket, n) {
            return Err(session.fail_close(e));
        }
        let (selector, chunk) = match parse_descriptor_page(&page[..n]) {
            Ok(v) => v,
            Err(e) => return Err(session.fail_close(e)),
        };
        if selector.class != 6
            || selector.total != 64
            || selector.offset != 0
            || selector.chunk != 64
            || chunk.len() != BUNDLE_HEADER_SIZE
        {
            return Err(session.fail_close(Error::Identity));
        }
        out.copy_from_slice(chunk);
        Ok(())
    }

    pub fn recv_index_page<S: ServiceIo>(
        &self,
        session: &mut S,
        ticket: &DescribeTicket,
        out: &mut [u8],
    ) -> Result<usize, Error> {
        let mut page = [0u8; MAX_LEAF_RECORD_BYTES];
        let (header, n) = session.recv_descriptor(&mut page)?;
        if let Err(e) = self.check_page(&header, ticket, n) {
            return Err(session.fail_close(e));
        }
        let (selector, chunk) = match parse_descriptor_page(&page[..n]) {
            Ok(v) => v,
            Err(e) => return Err(session.fail_close(e)),
        };
        if selector.class != 7
            || selector.offset != ticket.offset
            || selector.chunk != ticket.chunk
            || selector.total != ticket.total
        {
            return Err(session.fail_close(Error::Identity));
        }
        if out.len() < chunk.len() {
            return Err(session.fail_close(Error::Capacity));
        }
        out[..chunk.len()].copy_from_slice(chunk);
        Ok(chunk.len())
    }

    pub fn recv_body_page<S: ServiceIo>(
        &self,
        session: &mut S,
        ticket: &DescribeTicket,
        out: &mut [u8],
    ) -> Result<usize, Error> {
        self.recv_object_page(session, ticket, 8, out)
    }

    pub fn recv_entry_page<S: ServiceIo>(
        &self,
        session: &mut S,
        ticket: &DescribeTicket,
        out: &mut [u8],
    ) -> Result<usize, Error> {
        self.recv_object_page(session, ticket, 9, out)
    }

    fn recv_object_page<S: ServiceIo>(
        &self,
        session: &mut S,
        ticket: &DescribeTicket,
        class: u8,
        out: &mut [u8],
    ) -> Result<usize, Error> {
        let mut page = [0u8; MAX_LEAF_RECORD_BYTES];
        let (header, n) = session.recv_descriptor(&mut page)?;
        if let Err(e) = self.check_page(&header, ticket, n) {
            return Err(session.fail_close(e));
        }
        let (selector, chunk) = match parse_descriptor_page(&page[..n]) {
            Ok(v) => v,
            Err(e) => return Err(session.fail_close(e)),
        };
        if selector.class != class
            || selector.number != ticket.number
            || selector.offset != ticket.offset
            || selector.chunk != ticket.chunk
            || selector.total != ticket.total
        {
            return Err(session.fail_close(Error::Identity));
        }
        if out.len() < chunk.len() {
            return Err(session.fail_close(Error::Capacity));
        }
        out[..chunk.len()].copy_from_slice(chunk);
        Ok(chunk.len())
    }

    fn check_page(&self, header: &Header, ticket: &DescribeTicket, body_len: usize) -> Result<(), Error> {
        check_identity(header, self.identity, self.policy_generation)?;
        if header.kind != Kind::DescriptorPage
            || header.transfer_id != ticket.transfer_id
            || header.entry != 0
            || header.leaf != 0
            || header.count != 0
            || header.credit_serial != ticket.credit_serial
            || header.credit_bytes != ticket.credit_bytes
        {
            return Err(Error::Credit);
        }
        let need = PREFIX_BYTES.saturating_add(body_len) as u64;
        if header.credit_bytes < need {
            return Err(Error::Credit);
        }
        Ok(())
    }
}

pub fn encode_describe_request(
    limits: Limits,
    selector: ObjectSelector,
    out: &mut [u8],
) -> Result<usize, Error> {
    if out.len() < DESCRIBE_BODY {
        return Err(Error::Capacity);
    }
    out[..DESCRIBE_BODY].fill(0);
    limits.encode(&mut out[..64])?;
    selector.encode(true, &mut out[64..DESCRIBE_BODY])?;
    Ok(DESCRIBE_BODY)
}

pub fn parse_describe_request(body: &[u8]) -> Result<(Limits, ObjectSelector), Error> {
    if body.len() != DESCRIBE_BODY {
        return Err(Error::Malformed);
    }
    let limits = Limits::parse(&body[..64])?;
    let selector = ObjectSelector::parse(&body[64..], true)?;
    Ok((limits, selector))
}

pub fn encode_descriptor_page(
    selector: ObjectSelector,
    chunk: &[u8],
    out: &mut [u8],
) -> Result<usize, Error> {
    if chunk.len() != selector.chunk as usize {
        return Err(Error::Malformed);
    }
    let n = 32usize.checked_add(chunk.len()).ok_or(Error::Overflow)?;
    if out.len() < n {
        return Err(Error::Capacity);
    }
    selector.encode(false, out)?;
    out[32..n].copy_from_slice(chunk);
    Ok(n)
}

pub fn parse_descriptor_page(body: &[u8]) -> Result<(ObjectSelector, &[u8]), Error> {
    if body.len() < 32 {
        return Err(Error::Malformed);
    }
    let selector = ObjectSelector::parse(&body[..32], false)?;
    let n = 32usize
        .checked_add(selector.chunk as usize)
        .ok_or(Error::Overflow)?;
    if body.len() != n {
        return Err(Error::Malformed);
    }
    Ok((selector, &body[32..n]))
}

/// Recheck, then serve one DescriptorPage from the live source. Class 1 yields
/// the 64-byte header; class 7 yields an index slice. Other classes refuse.
pub fn answer_describe<S: ServiceIo>(
    session: &mut S,
    authority: &mut LocalEffectControl<'_, '_>,
    identity: CandidateIdentity,
    policy_generation: u64,
    source: &[u8],
) -> Result<ServedDescriptor, Error> {
    serve_describe(
        session,
        authority,
        identity,
        policy_generation,
        source,
        None,
        None,
    )
}

/// Same admission path as [`answer_describe`], plus class 8 (QASD body page)
/// and class 9 (entry page) from the live publication slices. Other classes refuse.
pub fn answer_describe_pages<S: ServiceIo>(
    session: &mut S,
    authority: &mut LocalEffectControl<'_, '_>,
    identity: CandidateIdentity,
    policy_generation: u64,
    source: &[u8],
    body: &[u8],
    entry_page: &[u8],
) -> Result<ServedDescriptor, Error> {
    serve_describe(
        session,
        authority,
        identity,
        policy_generation,
        source,
        Some(body),
        Some(entry_page),
    )
}

fn serve_describe<S: ServiceIo>(
    session: &mut S,
    authority: &mut LocalEffectControl<'_, '_>,
    identity: CandidateIdentity,
    policy_generation: u64,
    source: &[u8],
    body: Option<&[u8]>,
    entry_page: Option<&[u8]>,
) -> Result<ServedDescriptor, Error> {
    if authority.closure().identity() != identity {
        return Err(session.fail_close(Error::Identity));
    }
    authority
        .recheck()
        .map_err(|e| session.fail_close(super::map_policy(e)))?;
    let (header, request, n) = session.recv_describe()?;
    if header.kind != Kind::Describe {
        return Err(session.fail_close(Error::State));
    }
    if let Err(e) = check_identity(&header, identity, policy_generation) {
        return Err(session.fail_close(e));
    }
    if n != DESCRIBE_BODY {
        return Err(session.fail_close(Error::Malformed));
    }
    let (_limits, selector) = match parse_describe_request(&request[..n]) {
        Ok(v) => v,
        Err(e) => return Err(session.fail_close(e)),
    };
    let (page_sel, chunk) = match selector.class {
        1 => header_page(source).map_err(|e| session.fail_close(e))?,
        7 => index_page(source, selector).map_err(|e| session.fail_close(e))?,
        8 => match body {
            Some(bytes) => object_page(bytes, selector, 8).map_err(|e| session.fail_close(e))?,
            None => return Err(session.fail_close(Error::Unsupported)),
        },
        9 => match entry_page {
            Some(bytes) => object_page(bytes, selector, 9).map_err(|e| session.fail_close(e))?,
            None => return Err(session.fail_close(Error::Unsupported)),
        },
        _ => return Err(session.fail_close(Error::Unsupported)),
    };
    let mut page = [0u8; MAX_LEAF_RECORD_BYTES];
    let pn = match encode_descriptor_page(page_sel, chunk, &mut page) {
        Ok(v) => v,
        Err(e) => return Err(session.fail_close(e)),
    };
    let record_bytes = PREFIX_BYTES.saturating_add(pn) as u64;
    if header.credit_serial == 0 || header.credit_bytes < record_bytes {
        return Err(session.fail_close(Error::Credit));
    }
    authority
        .recheck()
        .map_err(|e| session.fail_close(super::map_policy(e)))?;
    let reply = associate_until(
        Kind::DescriptorPage,
        identity,
        policy_generation,
        header.transfer_id,
        0,
        0,
        0,
        header.credit_serial,
        header.credit_bytes,
        header.deadline,
    );
    session.send_message(&reply, &page[..pn])?;
    Ok(ServedDescriptor {
        class: page_sel.class,
        offset: page_sel.offset,
        length: chunk.len(),
    })
}

fn header_page(source: &[u8]) -> Result<(ObjectSelector, &[u8]), Error> {
    if source.len() < BUNDLE_HEADER_SIZE {
        return Err(Error::Identity);
    }
    Ok((
        ObjectSelector {
            class: 6,
            number: 0,
            total: 64,
            offset: 0,
            chunk: 64,
        },
        &source[..BUNDLE_HEADER_SIZE],
    ))
}

fn index_page(source: &[u8], request: ObjectSelector) -> Result<(ObjectSelector, &[u8]), Error> {
    if source.len() < BUNDLE_HEADER_SIZE {
        return Err(Error::Identity);
    }
    let header = admit_header(
        &source[..BUNDLE_HEADER_SIZE],
        source.len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .map_err(|_| Error::Identity)?;
    if request.total != header.index_length() || request.chunk == 0 || request.chunk > MAX_INDEX_CHUNK
    {
        return Err(Error::Identity);
    }
    let start = header
        .index_offset()
        .checked_add(request.offset)
        .ok_or(Error::Overflow)?;
    let start_us = usize::try_from(start).map_err(|_| Error::Limit)?;
    let n = request.chunk as usize;
    let end = start_us.checked_add(n).ok_or(Error::Overflow)?;
    let index_end = usize::try_from(
        header
            .index_offset()
            .checked_add(header.index_length())
            .ok_or(Error::Overflow)?,
    )
    .map_err(|_| Error::Limit)?;
    if end > index_end || end > source.len() {
        return Err(Error::Identity);
    }
    Ok((
        ObjectSelector {
            class: 7,
            number: 0,
            total: request.total,
            offset: request.offset,
            chunk: request.chunk,
        },
        &source[start_us..end],
    ))
}

fn object_page(
    bytes: &[u8],
    request: ObjectSelector,
    class: u8,
) -> Result<(ObjectSelector, &[u8]), Error> {
    if request.class != class {
        return Err(Error::Identity);
    }
    let total = u64::try_from(bytes.len()).map_err(|_| Error::Limit)?;
    if total == 0 || total > u64::from(MAX_PAGE_CHUNK) {
        return Err(Error::Limit);
    }
    if request.number != 0
        || request.offset != 0
        || request.total != total
        || u64::from(request.chunk) != total
    {
        return Err(Error::Identity);
    }
    Ok((
        ObjectSelector {
            class,
            number: 0,
            total,
            offset: 0,
            chunk: request.chunk,
        },
        bytes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::stream::{
        descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
        index::{admit_index, EntryScratch},
        policy::{adopt_local_whole_source, tests::Fixture, CLOSURE_PURPOSE},
        proof::{OutboardNode, TreeBuilder, LEAF_BYTES},
        publish::PublicationClass,
        selection::{select_entry, SelectedPlainEntry},
    };
    use crate::net::peer::replication::qbdl::provider::{OutboardCatalog, OutboardLimits};
    use crate::net::peer::replication::qbdl::service::{
        PlainProvider, Session, SessionOwner, ROLE_CLIENT, ROLE_PROVIDER,
    };
    use crate::net::peer::replication::qbdl::wire;
    use crate::net::qdnf::authority::AuthorityOwner;
    use crate::net::qdnf::bearer::ipc_pair;
    use crate::net::qdnf::types::{ScopeEpoch, StrongDigest};

    const TRANSCRIPT: StrongDigest = StrongDigest([9; 48]);
    const POLICY: u64 = 1;

    fn payload() -> Vec<u8> {
        let n = LEAF_BYTES as usize * 2 + 19;
        let mut bytes = vec![0x5E; n];
        bytes[..LEAF_BYTES as usize].fill(0xA1);
        bytes[..4].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
        let mid = LEAF_BYTES as usize;
        bytes[mid..mid * 2].fill(0xB2);
        bytes[mid..mid + 4].copy_from_slice(&[0x0D, 0xD0, 0xFE, 0xED]);
        bytes
    }

    fn select(fixture: &Fixture) -> SelectedPlainEntry<'_> {
        let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
        let closure = fixture.bound(&pins, pin, publisher);
        let header = admit_header(
            &fixture.source()[..64],
            fixture.source().len() as u64,
            ExtentLimit::default(),
            IndexReadLimits::default(),
        )
        .unwrap();
        let mut index_scratch = [EntryScratch::EMPTY; 1];
        let index = admit_index(
            &header,
            &fixture.source()[header.index_offset() as usize..],
            IndexReadLimits::default(),
            &mut index_scratch,
        )
        .unwrap();
        select_entry(
            &closure,
            fixture.body(),
            &fixture.source()[..64],
            &index,
            fixture.entry_page(),
            0,
            &mut [EntryRecord::EMPTY; 128],
            &mut vec![0; HASH_SCRATCH_BYTES],
        )
        .unwrap()
    }

    fn build_outboard(
        fixture: &Fixture,
        selected: SelectedPlainEntry<'_>,
    ) -> (Vec<OutboardNode>, StrongDigest) {
        let start = usize::try_from(selected.offset()).unwrap();
        let length = usize::try_from(selected.logical_length()).unwrap();
        let entry = &fixture.source()[start..start + length];
        let spec = selected.tree_spec().unwrap();
        let mut builder = TreeBuilder::new(spec);
        let mut nodes = Vec::new();
        for leaf in entry.chunks(LEAF_BYTES as usize) {
            builder
                .append(leaf, |node| {
                    nodes.push(node);
                    Ok(())
                })
                .unwrap();
        }
        let root = builder
            .finish(|node| {
                nodes.push(node);
                Ok(())
            })
            .unwrap();
        nodes.sort_unstable_by_key(|node| (node.start_leaf, node.leaf_count));
        (nodes, root)
    }

    fn catalog<'p, 'n>(
        selected: SelectedPlainEntry<'p>,
        nodes: &'n [OutboardNode],
        root: StrongDigest,
    ) -> OutboardCatalog<'p, 'n> {
        OutboardCatalog::new(
            selected,
            selected.tree_spec().unwrap(),
            root,
            nodes,
            OutboardLimits {
                max_nodes: nodes.len(),
                max_bytes: nodes.len() * std::mem::size_of::<OutboardNode>(),
            },
        )
        .unwrap()
    }

    fn one_page_chunk(bytes: &[u8]) -> Result<(u64, u32), Error> {
        let total = u64::try_from(bytes.len()).map_err(|_| Error::Limit)?;
        let chunk = u32::try_from(total).map_err(|_| Error::Limit)?;
        if chunk == 0 || chunk > MAX_PAGE_CHUNK {
            return Err(Error::Limit);
        }
        Ok((total, chunk))
    }

    fn attach_pair<'a>(
        client_lanes: &'a mut SessionOwner,
        provider_lanes: &'a mut SessionOwner,
    ) -> (Session<'a>, Session<'a>) {
        let (client_ep, provider_ep) = ipc_pair(ScopeEpoch { scope: 1, epoch: 1 }, 1280).unwrap();
        let client_peer = provider_ep.locator();
        let provider_peer = client_ep.locator();
        let client_session = Session::attach(
            client_ep,
            client_peer,
            TRANSCRIPT,
            ROLE_CLIENT,
            client_lanes,
        )
        .unwrap();
        let provider_session = Session::attach(
            provider_ep,
            provider_peer,
            TRANSCRIPT,
            ROLE_PROVIDER,
            provider_lanes,
        )
        .unwrap();
        (client_session, provider_session)
    }

    #[test]
    fn describe_header_and_index_without_selected_entry() {
        let bytes = payload();
        let fixture = Fixture::new_with_payload(&bytes);
        let selected = select(&fixture);
        let (nodes, root) = build_outboard(&fixture, selected);
        let catalog = catalog(selected, &nodes, root);
        let provider = PlainProvider::bind(
            PublicationClass::PlainCandidate,
            fixture.source(),
            catalog,
            POLICY,
        )
        .unwrap();
        let client = DescribeClient::bind(fixture.identity(), POLICY).unwrap();
        assert_eq!(client.identity(), selected.candidate_identity());
        let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
        let bound = fixture.bound(&pins, pin, publisher);
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
        let (client_ep, provider_ep) = ipc_pair(ScopeEpoch { scope: 1, epoch: 1 }, 1280).unwrap();
        let client_peer = provider_ep.locator();
        let provider_peer = client_ep.locator();
        let mut client_lanes = SessionOwner::new();
        let mut provider_lanes = SessionOwner::new();
        let mut client_session = Session::attach(
            client_ep,
            client_peer,
            TRANSCRIPT,
            ROLE_CLIENT,
            &mut client_lanes,
        )
        .unwrap();
        let mut provider_session = Session::attach(
            provider_ep,
            provider_peer,
            TRANSCRIPT,
            ROLE_PROVIDER,
            &mut provider_lanes,
        )
        .unwrap();
        grant
            .accept_local_scoped(&pins, || 20, |authority| {
                Ok((|| -> Result<(), Error> {
                    let ticket = client.send_header_describe(&mut client_session)?;
                    let served = provider.answer_describe(&mut provider_session, authority)?;
                    assert_eq!(served.class, 6);
                    assert_eq!(served.length, 64);
                    let mut header_bytes = [0u8; 64];
                    client.recv_header_page(&mut client_session, &ticket, &mut header_bytes)?;
                    let header = match admit_header(
                        &header_bytes,
                        fixture.source().len() as u64,
                        ExtentLimit::default(),
                        IndexReadLimits::default(),
                    ) {
                        Ok(h) => h,
                        Err(_) => return Err(Error::Identity),
                    };
                    assert_eq!(header.entry_count(), fixture.identity().entry_count());
                    let total = header.index_length();
                    let chunk = u32::try_from(total).map_err(|_| Error::Limit)?;
                    if chunk > MAX_INDEX_CHUNK {
                        return Err(Error::Limit);
                    }
                    let index_ticket =
                        client.send_index_describe(&mut client_session, 0, chunk, total)?;
                    let served = provider.answer_describe(&mut provider_session, authority)?;
                    assert_eq!(served.class, 7);
                    assert_eq!(served.length, chunk as usize);
                    let mut index_buf = vec![0u8; chunk as usize];
                    let n = client.recv_index_page(
                        &mut client_session,
                        &index_ticket,
                        &mut index_buf,
                    )?;
                    assert_eq!(n, chunk as usize);
                    let mut scratch = [EntryScratch::EMPTY; 1];
                    if admit_index(
                        &header,
                        &index_buf[..n],
                        IndexReadLimits::default(),
                        &mut scratch,
                    )
                    .is_err()
                    {
                        return Err(Error::Identity);
                    }
                    Ok(())
                })())
            })
            .unwrap()
            .unwrap();
    }

    #[test]
    fn identity_mutation_of_describe_refuses() {
        let fixture = Fixture::new();
        let selected = select(&fixture);
        let (nodes, root) = build_outboard(&fixture, selected);
        let catalog = catalog(selected, &nodes, root);
        let provider = PlainProvider::bind(
            PublicationClass::PlainCandidate,
            fixture.source(),
            catalog,
            POLICY,
        )
        .unwrap();
        let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
        let bound = fixture.bound(&pins, pin, publisher);
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
        let (client_ep, provider_ep) = ipc_pair(ScopeEpoch { scope: 1, epoch: 1 }, 1280).unwrap();
        let client_peer = provider_ep.locator();
        let provider_peer = client_ep.locator();
        let mut client_lanes = SessionOwner::new();
        let mut provider_lanes = SessionOwner::new();
        let mut client_session = Session::attach(
            client_ep,
            client_peer,
            TRANSCRIPT,
            ROLE_CLIENT,
            &mut client_lanes,
        )
        .unwrap();
        let mut provider_session = Session::attach(
            provider_ep,
            provider_peer,
            TRANSCRIPT,
            ROLE_PROVIDER,
            &mut provider_lanes,
        )
        .unwrap();
        let mut mutated = fixture.identity();
        mutated.descriptor = StrongDigest([0x11; 48]);
        let attacker = DescribeClient::bind(mutated, POLICY).unwrap();
        grant
            .accept_local_scoped(&pins, || 20, |authority| {
                Ok((|| -> Result<(), Error> {
                    let _ = attacker.send_header_describe(&mut client_session)?;
                    match provider.answer_describe(&mut provider_session, authority) {
                        Err(Error::Identity) => Ok(()),
                        Ok(_) => Err(Error::State),
                        Err(e) => Err(e),
                    }
                })())
            })
            .unwrap()
            .unwrap();
    }

    #[test]
    fn stale_policy_generation_refuses() {
        let fixture = Fixture::new();
        let selected = select(&fixture);
        let (nodes, root) = build_outboard(&fixture, selected);
        let catalog = catalog(selected, &nodes, root);
        let provider = PlainProvider::bind(
            PublicationClass::PlainCandidate,
            fixture.source(),
            catalog,
            POLICY,
        )
        .unwrap();
        let client = DescribeClient::bind(fixture.identity(), POLICY + 1).unwrap();
        let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
        let bound = fixture.bound(&pins, pin, publisher);
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
        let (client_ep, provider_ep) = ipc_pair(ScopeEpoch { scope: 1, epoch: 1 }, 1280).unwrap();
        let client_peer = provider_ep.locator();
        let provider_peer = client_ep.locator();
        let mut client_lanes = SessionOwner::new();
        let mut provider_lanes = SessionOwner::new();
        let mut client_session = Session::attach(
            client_ep,
            client_peer,
            TRANSCRIPT,
            ROLE_CLIENT,
            &mut client_lanes,
        )
        .unwrap();
        let mut provider_session = Session::attach(
            provider_ep,
            provider_peer,
            TRANSCRIPT,
            ROLE_PROVIDER,
            &mut provider_lanes,
        )
        .unwrap();
        grant
            .accept_local_scoped(&pins, || 20, |authority| {
                Ok((|| -> Result<(), Error> {
                    let _ = client.send_header_describe(&mut client_session)?;
                    match provider.answer_describe(&mut provider_session, authority) {
                        Err(Error::Identity) => Ok(()),
                        Ok(_) => Err(Error::State),
                        Err(e) => Err(e),
                    }
                })())
            })
            .unwrap()
            .unwrap();
    }

    #[test]
    fn kind_thirteen_is_unsupported_on_the_wire() {
        let mut h = Header {
            kind: Kind::Close,
            descriptor: StrongDigest([1; 48]),
            context: StrongDigest([2; 48]),
            publication_generation: 0,
            policy_generation: 1,
            cancellation: 1,
            transfer_id: [4; 16],
            entry: 0,
            leaf: 0,
            count: 0,
            credit_serial: 0,
            credit_bytes: 0,
            deadline: 90,
        };
        let mut raw = [0u8; PREFIX_BYTES + 8];
        let n = wire::encode(&h, &[0, 0, 0, 0, 0, 0, 0, 0], &mut raw).unwrap();
        raw[7] = 13;
        match wire::Message::parse(&raw[..n]) {
            Err(wire::Error::Unsupported) => {}
            Err(_) => panic!("kind 13 must be unsupported"),
            Ok(_) => panic!("kind 13 must refuse"),
        }
        h.kind = Kind::Describe;
        h.cancellation = 0;
        h.credit_serial = 1;
        h.credit_bytes = 304;
        let _ = h;
    }

    #[test]
    fn zero_descriptor_refuses_bind() {
        let mut identity = CandidateIdentity {
            descriptor: StrongDigest::ZERO,
            context: StrongDigest([2; 48]),
            generation: 0,
            entries: 1,
        };
        match DescribeClient::bind(identity, 1) {
            Err(Error::Malformed) => {}
            Ok(_) => panic!("zero descriptor must refuse"),
            Err(_) => panic!("zero descriptor must be malformed"),
        }
        identity.descriptor = StrongDigest([1; 48]);
        identity.context = StrongDigest::ZERO;
        match DescribeClient::bind(identity, 1) {
            Err(Error::Malformed) => {}
            Ok(_) => panic!("zero context must refuse"),
            Err(_) => panic!("zero context must be malformed"),
        }
    }

    #[test]
    fn describe_body_and_entry_page_then_select_without_local_entry() {
        let bytes = payload();
        let fixture = Fixture::new_with_payload(&bytes);
        let selected = select(&fixture);
        let (nodes, root) = build_outboard(&fixture, selected);
        let catalog = catalog(selected, &nodes, root);
        let _provider = PlainProvider::bind(
            PublicationClass::PlainCandidate,
            fixture.source(),
            catalog,
            POLICY,
        )
        .unwrap();
        let client = DescribeClient::bind(fixture.identity(), POLICY).unwrap();
        assert_eq!(client.identity(), selected.candidate_identity());
        let want_entry = selected.entry_id();
        let want_root = selected.root();
        let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
        let bound = fixture.bound(&pins, pin, publisher);
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
        let mut client_lanes = SessionOwner::new();
        let mut provider_lanes = SessionOwner::new();
        let (mut client_session, mut provider_session) =
            attach_pair(&mut client_lanes, &mut provider_lanes);
        grant
            .accept_local_scoped(&pins, || 20, |authority| {
                Ok((|| -> Result<(), Error> {
                    let header_ticket = client.send_header_describe(&mut client_session)?;
                    let served = answer_describe_pages(
                        &mut provider_session,
                        authority,
                        fixture.identity(),
                        POLICY,
                        fixture.source(),
                        fixture.body(),
                        fixture.entry_page(),
                    )?;
                    assert_eq!(served.class, 6);
                    assert_eq!(served.length, 64);
                    let mut header_bytes = [0u8; 64];
                    client.recv_header_page(
                        &mut client_session,
                        &header_ticket,
                        &mut header_bytes,
                    )?;
                    let header = match admit_header(
                        &header_bytes,
                        fixture.source().len() as u64,
                        ExtentLimit::default(),
                        IndexReadLimits::default(),
                    ) {
                        Ok(h) => h,
                        Err(_) => return Err(Error::Identity),
                    };
                    let total = header.index_length();
                    let chunk = u32::try_from(total).map_err(|_| Error::Limit)?;
                    if chunk > MAX_INDEX_CHUNK {
                        return Err(Error::Limit);
                    }
                    let index_ticket =
                        client.send_index_describe(&mut client_session, 0, chunk, total)?;
                    let served = answer_describe_pages(
                        &mut provider_session,
                        authority,
                        fixture.identity(),
                        POLICY,
                        fixture.source(),
                        fixture.body(),
                        fixture.entry_page(),
                    )?;
                    assert_eq!(served.class, 7);
                    assert_eq!(served.length, chunk as usize);
                    let mut index_buf = vec![0u8; chunk as usize];
                    let n = client.recv_index_page(
                        &mut client_session,
                        &index_ticket,
                        &mut index_buf,
                    )?;
                    assert_eq!(n, chunk as usize);
                    let mut index_scratch = [EntryScratch::EMPTY; 1];
                    let index = match admit_index(
                        &header,
                        &index_buf[..n],
                        IndexReadLimits::default(),
                        &mut index_scratch,
                    ) {
                        Ok(v) => v,
                        Err(_) => return Err(Error::Identity),
                    };
                    let (body_total, body_chunk) = one_page_chunk(fixture.body())?;
                    let body_ticket = client.send_body_describe(
                        &mut client_session,
                        0,
                        body_chunk,
                        body_total,
                        0,
                    )?;
                    let served = answer_describe_pages(
                        &mut provider_session,
                        authority,
                        fixture.identity(),
                        POLICY,
                        fixture.source(),
                        fixture.body(),
                        fixture.entry_page(),
                    )?;
                    assert_eq!(served.class, 8);
                    assert_eq!(served.length, body_chunk as usize);
                    let mut received_body = vec![0u8; body_chunk as usize];
                    let bn = client.recv_body_page(
                        &mut client_session,
                        &body_ticket,
                        &mut received_body,
                    )?;
                    assert_eq!(bn, body_chunk as usize);
                    received_body.truncate(bn);
                    let (entry_total, entry_chunk) = one_page_chunk(fixture.entry_page())?;
                    let entry_ticket = client.send_entry_page_describe(
                        &mut client_session,
                        0,
                        entry_chunk,
                        entry_total,
                        0,
                    )?;
                    let served = answer_describe_pages(
                        &mut provider_session,
                        authority,
                        fixture.identity(),
                        POLICY,
                        fixture.source(),
                        fixture.body(),
                        fixture.entry_page(),
                    )?;
                    assert_eq!(served.class, 9);
                    assert_eq!(served.length, entry_chunk as usize);
                    let mut received_entry = vec![0u8; entry_chunk as usize];
                    let en = client.recv_entry_page(
                        &mut client_session,
                        &entry_ticket,
                        &mut received_entry,
                    )?;
                    assert_eq!(en, entry_chunk as usize);
                    received_entry.truncate(en);
                    let mut rows = [EntryRecord::EMPTY; 128];
                    let mut framing = vec![0u8; HASH_SCRATCH_BYTES];
                    let got = match select_entry(
                        &bound,
                        &received_body,
                        &header_bytes,
                        &index,
                        &received_entry,
                        0,
                        &mut rows,
                        &mut framing,
                    ) {
                        Ok(v) => v,
                        Err(_) => return Err(Error::Identity),
                    };
                    if got.entry_id() != want_entry || got.root() != want_root {
                        return Err(Error::Identity);
                    }
                    Ok(())
                })())
            })
            .unwrap()
            .unwrap();
    }

    #[test]
    fn identity_mutation_of_class_eight_describe_refuses() {
        let fixture = Fixture::new();
        let selected = select(&fixture);
        let (nodes, root) = build_outboard(&fixture, selected);
        let catalog = catalog(selected, &nodes, root);
        let _provider = PlainProvider::bind(
            PublicationClass::PlainCandidate,
            fixture.source(),
            catalog,
            POLICY,
        )
        .unwrap();
        let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
        let bound = fixture.bound(&pins, pin, publisher);
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
        let mut client_lanes = SessionOwner::new();
        let mut provider_lanes = SessionOwner::new();
        let (mut client_session, mut provider_session) =
            attach_pair(&mut client_lanes, &mut provider_lanes);
        let mut mutated = fixture.identity();
        mutated.descriptor = StrongDigest([0x11; 48]);
        let attacker = DescribeClient::bind(mutated, POLICY).unwrap();
        grant
            .accept_local_scoped(&pins, || 20, |authority| {
                Ok((|| -> Result<(), Error> {
                    let _ = attacker.send_body_describe(&mut client_session, 0, 1, 1, 0)?;
                    match answer_describe_pages(
                        &mut provider_session,
                        authority,
                        fixture.identity(),
                        POLICY,
                        fixture.source(),
                        fixture.body(),
                        fixture.entry_page(),
                    ) {
                        Err(Error::Identity) => Ok(()),
                        Ok(_) => Err(Error::State),
                        Err(e) => Err(e),
                    }
                })())
            })
            .unwrap()
            .unwrap();
    }

    #[test]
    fn class_eight_and_nine_without_pages_stay_unsupported() {
        let fixture = Fixture::new();
        let selected = select(&fixture);
        let (nodes, root) = build_outboard(&fixture, selected);
        let catalog = catalog(selected, &nodes, root);
        let provider = PlainProvider::bind(
            PublicationClass::PlainCandidate,
            fixture.source(),
            catalog,
            POLICY,
        )
        .unwrap();
        let client = DescribeClient::bind(fixture.identity(), POLICY).unwrap();
        let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
        let bound = fixture.bound(&pins, pin, publisher);
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
        let mut client_lanes_a = SessionOwner::new();
        let mut provider_lanes_a = SessionOwner::new();
        let (mut client_a, mut provider_a) =
            attach_pair(&mut client_lanes_a, &mut provider_lanes_a);
        let mut client_lanes_b = SessionOwner::new();
        let mut provider_lanes_b = SessionOwner::new();
        let (mut client_b, mut provider_b) =
            attach_pair(&mut client_lanes_b, &mut provider_lanes_b);
        grant
            .accept_local_scoped(&pins, || 20, |authority| {
                Ok((|| -> Result<(), Error> {
                    let _ = client.send_body_describe(&mut client_a, 0, 1, 1, 0)?;
                    match provider.answer_describe(&mut provider_a, authority) {
                        Err(Error::Unsupported) => {}
                        Ok(_) => return Err(Error::State),
                        Err(e) => return Err(e),
                    }
                    let _ = client.send_entry_page_describe(&mut client_b, 0, 1, 1, 0)?;
                    match provider.answer_describe(&mut provider_b, authority) {
                        Err(Error::Unsupported) => Ok(()),
                        Ok(_) => Err(Error::State),
                        Err(e) => Err(e),
                    }
                })())
            })
            .unwrap()
            .unwrap();
    }

    #[test]
    fn class_eight_chunk_total_mismatch_refuses() {
        let fixture = Fixture::new();
        let selected = select(&fixture);
        let (nodes, root) = build_outboard(&fixture, selected);
        let catalog = catalog(selected, &nodes, root);
        let _provider = PlainProvider::bind(
            PublicationClass::PlainCandidate,
            fixture.source(),
            catalog,
            POLICY,
        )
        .unwrap();
        let client = DescribeClient::bind(fixture.identity(), POLICY).unwrap();
        let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
        let bound = fixture.bound(&pins, pin, publisher);
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
        let mut client_lanes = SessionOwner::new();
        let mut provider_lanes = SessionOwner::new();
        let (mut client_session, mut provider_session) =
            attach_pair(&mut client_lanes, &mut provider_lanes);
        grant
            .accept_local_scoped(&pins, || 20, |authority| {
                Ok((|| -> Result<(), Error> {
                    let wrong = if fixture.body().len() == 1 { 2u32 } else { 1u32 };
                    let _ = client.send_body_describe(
                        &mut client_session,
                        0,
                        wrong,
                        u64::from(wrong),
                        0,
                    )?;
                    match answer_describe_pages(
                        &mut provider_session,
                        authority,
                        fixture.identity(),
                        POLICY,
                        fixture.source(),
                        fixture.body(),
                        fixture.entry_page(),
                    ) {
                        Err(Error::Identity) => Ok(()),
                        Ok(_) => Err(Error::State),
                        Err(e) => Err(e),
                    }
                })())
            })
            .unwrap()
            .unwrap();
    }
}
