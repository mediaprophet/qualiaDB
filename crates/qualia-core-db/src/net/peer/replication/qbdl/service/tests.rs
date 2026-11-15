use super::*;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{adopt_local_whole_source, tests::Fixture, CLOSURE_PURPOSE, PolicyError},
    proof::{OutboardNode, Representation, TreeBuilder, LEAF_BYTES, MAX_SIBLINGS},
    publish::PublicationClass,
    runtime::{
        ColdPlan, RuntimeParents, RuntimePoll, RuntimeProfile, TransferRequirements, TransferSpec,
    },
    selection::{select_entry, SelectedPlainEntry},
};
use crate::net::peer::replication::qbdl::provider::{OutboardCatalog, OutboardLimits};
use crate::net::peer::replication::qbdl::wire::{
    self, Header, Kind, ServiceRecordHeader, MAX_LEAF_RECORD_BYTES,
};
use crate::net::peer::runtime::{
    FairScheduler, LeaseTable, OperationTable, ReservationLedger, ResourceBudget, ResourceGovernor,
    WorkClass,
};
use crate::net::qdnf::authority::AuthorityOwner;
use crate::net::qdnf::bearer::contract::Bearer;
use crate::net::qdnf::bearer::ipc_pair;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{Generation, OperationId, ScopeEpoch, StrongDigest};
use std::cell::Cell;
use std::sync::Mutex;

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
    assert_eq!(root, selected.root());
    nodes.sort_unstable_by_key(|node| (node.start_leaf, node.leaf_count));
    (nodes, root)
}

fn catalog<'p, 'n>(
    selected: SelectedPlainEntry<'p>,
    nodes: &'n [OutboardNode],
    root: StrongDigest,
) -> OutboardCatalog<'p, 'n> {
    let spec = selected.tree_spec().unwrap();
    OutboardCatalog::new(
        selected,
        spec,
        root,
        nodes,
        OutboardLimits {
            max_nodes: nodes.len(),
            max_bytes: nodes.len() * std::mem::size_of::<OutboardNode>(),
        },
    )
    .unwrap()
}

#[test]
fn useful_leaf_transfers_while_unrelated_payload_stays_unrequested() {
    assert_eq!(SERVICE_PREFIX_BYTES, 208);
    assert_eq!(STREAM_SERVICE_IRI, b"q42:QBDLStream/1");
    let bytes = payload();
    let fixture = Fixture::new_with_payload(&bytes);
    let selected = select(&fixture);
    let spec = selected.tree_spec().unwrap();
    assert_eq!(spec.leaf_count(), 3);
    let (nodes, root) = build_outboard(&fixture, selected);
    let catalog = catalog(selected, &nodes, root);
    let provider = PlainProvider::bind(
        PublicationClass::PlainCandidate,
        fixture.source(),
        catalog,
        POLICY,
    )
    .unwrap();
    let client = PlainClient::bind(selected, POLICY, 0).unwrap();
    let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
    let bound = fixture.bound(&pins, pin, publisher);
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
    let (client_ep, provider_ep) = ipc_pair(ScopeEpoch { scope: 1, epoch: 1 }, 1280).unwrap();
    assert_eq!(client_ep.profile(), BearerProfile::LocalIpcV1);
    assert!(client_ep.profile().native_independent());
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
    assert_eq!(client_session.profile(), BearerProfile::LocalIpcV1);
    let tail = spec.leaf_count() - 1;
    let mut got = [0u8; 64];
    let served = transfer_one_leaf(
        &provider,
        &client,
        &mut provider_session,
        &mut client_session,
        &mut grant,
        &pins,
        || 20,
        tail,
        &mut got,
    )
    .unwrap();
    let start = selected.offset() as usize + tail as usize * LEAF_BYTES as usize;
    let expect = &fixture.source()[start..start + 19];
    assert_eq!(served.leaf, tail);
    assert_eq!(served.entry, selected.entry_id());
    assert_eq!(served.length, 19);
    assert_eq!(served.source_offset, start as u64);
    assert_eq!(&got[..19], expect);
    assert!(!got[..19].contains(&0xA1));
    assert!(!got[..19].windows(4).any(|w| w == [0xDE, 0xAD, 0xBE, 0xEF]));
    assert!(!got[..19].windows(4).any(|w| w == [0x0D, 0xD0, 0xFE, 0xED]));
    assert_eq!(served.source_offset, selected.offset() + 2 * LEAF_BYTES);
}

#[test]
fn malformed_prefix_refuses_and_fail_closes() {
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
    let (mut attacker, provider_ep) = ipc_pair(ScopeEpoch { scope: 1, epoch: 1 }, 1280).unwrap();
    let dest = provider_ep.locator();
    let mut lanes = SessionOwner::new();
    let mut provider_session = Session::attach(
        provider_ep,
        attacker.locator(),
        TRANSCRIPT,
        ROLE_PROVIDER,
        &mut lanes,
    )
    .unwrap();
    let mut body = [0u8; 72];
    let bn = super::session::encode_open_body(selected.ordinal(), 0, &mut body).unwrap();
    let header = super::session::associate(
        Kind::Open,
        selected.candidate_identity(),
        POLICY,
        [1; 16],
        selected.entry_id(),
        0,
        0,
        0,
        0,
    );
    let mut record = [0u8; 280];
    let rn = wire::encode(&header, &body[..bn], &mut record).unwrap();
    record[10] = 0xFF;
    let rec = ServiceRecordHeader {
        transcript: TRANSCRIPT,
        message_id: 1u128.to_be_bytes(),
        total: rn as u32,
        offset: 0,
        chunk: rn as u16,
        final_chunk: true,
    };
    let mut frame = [0u8; 88 + 280];
    let hn = rec.encode(&mut frame).unwrap();
    frame[hn..hn + rn].copy_from_slice(&record[..rn]);
    attacker.send(&dest, &frame[..hn + rn]).unwrap();
    let outcome = grant.accept_local_scoped(
        &pins,
        || 20,
        |authority| {
            Ok({
                let first = provider.answer_open(&mut provider_session, authority);
                assert_eq!(first, Err(Error::Malformed));
                let second = provider.answer_open(&mut provider_session, authority);
                assert_eq!(second, Err(Error::Closed));
                Ok::<(), Error>(())
            })
        },
    );
    assert_eq!(outcome.unwrap(), Ok(()));
}

#[test]
fn cancel_fail_closes_without_leaf_disclosure() {
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
    let client = PlainClient::bind(selected, POLICY, 0).unwrap();
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
    let outcome = grant.accept_local_scoped(
        &pins,
        || 20,
        |authority| {
            Ok((|| {
                let open = client.send_open(&mut client_session)?;
                let opened = provider.answer_open(&mut provider_session, authority)?;
                client.send_cancel(&mut client_session, &open)?;
                let err = provider
                    .answer_leaf(&mut provider_session, authority, &opened)
                    .unwrap_err();
                assert_eq!(err, Error::Cancelled);
                assert!(provider_session.closed());
                assert_eq!(
                    client.send_open(&mut client_session).unwrap_err(),
                    Error::Closed
                );
                Ok::<(), Error>(())
            })())
        },
    );
    assert_eq!(outcome.unwrap(), Ok(()));
}

#[test]
fn close_fail_closes_session() {
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
    let client = PlainClient::bind(selected, POLICY, 0).unwrap();
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
    let outcome = grant.accept_local_scoped(
        &pins,
        || 20,
        |authority| {
            Ok((|| {
                let _open = client.send_open(&mut client_session)?;
                let opened = provider.answer_open(&mut provider_session, authority)?;
                client.send_close(&mut client_session)?;
                let err = provider
                    .answer_leaf(&mut provider_session, authority, &opened)
                    .unwrap_err();
                assert_eq!(err, Error::Closed);
                assert!(provider_session.closed());
                assert_eq!(
                    client.send_close(&mut client_session).unwrap_err(),
                    Error::Closed
                );
                Ok::<(), Error>(())
            })())
        },
    );
    assert_eq!(outcome.unwrap(), Ok(()));
}

#[test]
fn mixed_private_protected_wraps_are_unsupported() {
    assert_eq!(
        refuse_non_plain(
            PublicationClass::PrivateProjection,
            Representation::Plaintext
        ),
        Err(Error::Unsupported)
    );
    assert_eq!(
        refuse_non_plain(
            PublicationClass::RecipientWrapped,
            Representation::Plaintext
        ),
        Err(Error::Unsupported)
    );
    assert_eq!(
        refuse_non_plain(PublicationClass::PlainCandidate, Representation::Ciphertext),
        Err(Error::Unsupported)
    );
    assert_eq!(
        refuse_non_plain(PublicationClass::PlainCandidate, Representation::Plaintext),
        Ok(())
    );
    let fixture = Fixture::new();
    let selected = select(&fixture);
    let (nodes, root) = build_outboard(&fixture, selected);
    let catalog = catalog(selected, &nodes, root);
    assert_eq!(
        PlainProvider::bind(
            PublicationClass::PrivateProjection,
            fixture.source(),
            catalog,
            POLICY,
        )
        .err(),
        Some(Error::Unsupported)
    );
}

#[test]
fn request_leaves_identity_fields_refused_independently_of_credit() {
    let mutations: [fn(&mut Header); 4] = [
        |h| h.descriptor = StrongDigest([0xEE; 48]),
        |h| h.context = StrongDigest([0xEF; 48]),
        |h| h.publication_generation = h.publication_generation.wrapping_add(1),
        |h| h.policy_generation = 99,
    ];
    for mutate in mutations {
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
        let client = PlainClient::bind(selected, POLICY, 0).unwrap();
        let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
        let bound = fixture.bound(&pins, pin, publisher);
        let mut owner = AuthorityOwner::new();
        let mut grant =
            adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
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
        let outcome = grant.accept_local_scoped(
            &pins,
            || 20,
            |authority| {
                Ok((|| {
                    let open = client.send_open(&mut client_session)?;
                    let opened = provider.answer_open(&mut provider_session, authority)?;
                    let (grant_header, _, _) = client_session.recv_control()?;
                    if grant_header.kind != Kind::Grant
                        || grant_header.transfer_id != open.transfer_id
                    {
                        return Err(Error::State);
                    }
                    let transfer_id = open.transfer_id;
                    let credit_serial = client_session.next_credit_serial()?;
                    let credit_bytes = MAX_LEAF_RECORD_BYTES as u64;
                    let credit = super::session::associate(
                        Kind::Credit,
                        selected.candidate_identity(),
                        POLICY,
                        transfer_id,
                        selected.entry_id(),
                        0,
                        1,
                        credit_serial,
                        credit_bytes,
                    );
                    client_session.send_message(&credit, &[])?;
                    let mut request = super::session::associate(
                        Kind::RequestLeaves,
                        selected.candidate_identity(),
                        POLICY,
                        transfer_id,
                        selected.entry_id(),
                        0,
                        1,
                        credit_serial,
                        credit_bytes,
                    );
                    mutate(&mut request);
                    client_session.send_message(&request, &[])?;
                    let err = provider
                        .answer_leaf(&mut provider_session, authority, &opened)
                        .unwrap_err();
                    assert_eq!(err, Error::Identity);
                    assert!(provider_session.closed());
                    Ok::<(), Error>(())
                })())
            },
        );
        assert_eq!(outcome.unwrap(), Ok(()));
    }
}

#[test]
fn credit_transfer_id_must_match_opened_transfer() {
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
    let client = PlainClient::bind(selected, POLICY, 0).unwrap();
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
    let outcome = grant.accept_local_scoped(
        &pins,
        || 20,
        |authority| {
            Ok((|| {
                let open = client.send_open(&mut client_session)?;
                let opened = provider.answer_open(&mut provider_session, authority)?;
                let (grant_header, _, _) = client_session.recv_control()?;
                if grant_header.kind != Kind::Grant
                    || grant_header.transfer_id != open.transfer_id
                    || grant_header.transfer_id != opened.transfer_id
                {
                    return Err(Error::State);
                }
                let foreign = client_session.next_transfer_id()?;
                let credit_serial = client_session.next_credit_serial()?;
                let credit_bytes = MAX_LEAF_RECORD_BYTES as u64;
                let credit = super::session::associate(
                    Kind::Credit,
                    selected.candidate_identity(),
                    POLICY,
                    foreign,
                    selected.entry_id(),
                    0,
                    1,
                    credit_serial,
                    credit_bytes,
                );
                client_session.send_message(&credit, &[])?;
                let request = super::session::associate(
                    Kind::RequestLeaves,
                    selected.candidate_identity(),
                    POLICY,
                    foreign,
                    selected.entry_id(),
                    0,
                    1,
                    credit_serial,
                    credit_bytes,
                );
                client_session.send_message(&request, &[])?;
                let err = provider
                    .answer_leaf(&mut provider_session, authority, &opened)
                    .unwrap_err();
                assert_eq!(err, Error::Credit);
                assert!(provider_session.closed());
                Ok::<(), Error>(())
            })())
        },
    );
    assert_eq!(outcome.unwrap(), Ok(()));
}

#[test]
fn second_issue_credit_on_inflight_ticket_is_replay_without_second_payload() {
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
    let client = PlainClient::bind(selected, POLICY, 0).unwrap();
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
    let cap = ResourceBudget {
        bytes: 8 * 1024 * 1024,
        work: 1024,
        io: 64,
    };
    let ledger = Mutex::new(ReservationLedger::new(cap, cap, cap, cap, cap));
    let mut leases = Mutex::new(LeaseTable::new());
    let mut ops = Mutex::new(OperationTable::new());
    let mut scheduler = Mutex::new(FairScheduler::new());
    let governor = ResourceGovernor::new(4096);
    let parents =
        RuntimeParents::adopt(&ledger, &mut leases, &mut ops, &mut scheduler, &governor).unwrap();
    let mut retained = parents
        .admit(
            RuntimeProfile::Constrained,
            ResourceBudget {
                bytes: 4096,
                work: 1,
                io: 1,
            },
        )
        .unwrap();
    let part = ResourceBudget {
        bytes: 128 * 1024,
        work: 1,
        io: 1,
    };
    let rows = [TransferSpec {
        operation: OperationId([7; 16]),
        principal: 1,
        policy_generation: Generation(1),
        deadline: 100,
        class: WorkClass::Background,
        cost: 1,
        credit_bytes: MAX_LEAF_RECORD_BYTES as u32,
        requirements: TransferRequirements {
            network: part,
            proof: part,
            storage: part,
            consumer: part,
            scratch: part,
        },
        verified_peer: true,
    }];
    let plan = ColdPlan::compile(&rows).unwrap();
    let ticket = parents
        .try_with_runtime(&mut retained, |mut access| {
            let ticket = access.admit(&plan, 0, 20, Generation(1))?;
            for _ in 0..8 {
                match access.poll(20)? {
                    RuntimePoll::Ready(key) if key == ticket.completion_key() => return Ok(ticket),
                    RuntimePoll::External(_) => continue,
                    RuntimePoll::Ready(_) => continue,
                    RuntimePoll::Cancelled(_, _) => {
                        return Err(crate::net::qdnf::errors::QdnfError::Cancelled)
                    }
                }
            }
            Err(crate::net::qdnf::errors::QdnfError::WouldBlock)
        })
        .unwrap();
    let outcome = grant.accept_local_scoped(
        &pins,
        || 20,
        |authority| {
            Ok((|| {
                let open = client.send_open(&mut client_session)?;
                let opened = provider.answer_open(&mut provider_session, authority)?;
                let ticket_wire =
                    client.recv_grant_send_credit_request(&mut client_session, &open, 0)?;
                let served = provider.answer_leaf_after_credit(
                    &mut provider_session,
                    authority,
                    &opened,
                    || {
                        parents
                            .try_with_runtime(&mut retained, |mut access| {
                                access.issue_credit(&ticket, 20, Generation(1)).map(|_| ())
                            })
                            .map_err(super::from_qdnf)
                    },
                )?;
                let mut payload = [0u8; 16384];
                let n = client.recv_leaf(&mut client_session, &ticket_wire, &mut payload)?;
                assert_eq!(n, served.length);
                let credit_serial = client_session.next_credit_serial()?;
                let credit_bytes = MAX_LEAF_RECORD_BYTES as u64;
                let credit = super::session::associate(
                    Kind::Credit,
                    selected.candidate_identity(),
                    POLICY,
                    open.transfer_id,
                    selected.entry_id(),
                    0,
                    1,
                    credit_serial,
                    credit_bytes,
                );
                client_session.send_message(&credit, &[])?;
                let request = super::session::associate(
                    Kind::RequestLeaves,
                    selected.candidate_identity(),
                    POLICY,
                    open.transfer_id,
                    selected.entry_id(),
                    0,
                    1,
                    credit_serial,
                    credit_bytes,
                );
                client_session.send_message(&request, &[])?;
                let err = provider
                    .answer_leaf_after_credit(
                        &mut provider_session,
                        authority,
                        &opened,
                        || {
                            parents
                                .try_with_runtime(&mut retained, |mut access| {
                                    access.issue_credit(&ticket, 20, Generation(1)).map(|_| ())
                                })
                                .map_err(super::from_qdnf)
                        },
                    )
                    .unwrap_err();
                assert_eq!(err, Error::Credit);
                assert!(provider_session.closed());
                Ok::<(), Error>(())
            })())
        },
    );
    assert_eq!(outcome.unwrap(), Ok(()));
}

#[test]
fn expire_after_open_before_request_leaves_refuses_without_leaf() {
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
    let client = PlainClient::bind(selected, POLICY, 0).unwrap();
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
    let t = Cell::new(20u64);
    let outcome = grant.accept_local_scoped(
        &pins,
        || t.get(),
        |authority| -> Result<(), PolicyError> {
            let _open = client
                .send_open(&mut client_session)
                .expect("open");
            let opened = provider
                .answer_open(&mut provider_session, authority)
                .expect("answer_open");
            t.set(91);
            let err = provider
                .answer_leaf(&mut provider_session, authority, &opened)
                .unwrap_err();
            assert_eq!(err, Error::Expired);
            assert!(provider_session.closed());
            Err(PolicyError::Authority(QdnfError::Expired))
        },
    );
    assert_eq!(
        outcome,
        Err(PolicyError::Authority(QdnfError::Expired))
    );
}

const _: () = assert!(MAX_SIBLINGS == 32);
