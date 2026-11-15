//! Handshake-pair tests for production [`ProtectedSession`].
//! Routed from `endpoint_tests.rs` so the managed pair fixture stays private.
use super::*;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{adopt_local_whole_source, tests::Fixture, CLOSURE_PURPOSE},
    proof::{OutboardNode, TreeBuilder, LEAF_BYTES},
    publish::PublicationClass,
    selection::{select_entry, SelectedPlainEntry},
};
use crate::net::peer::replication::qbdl::provider::{OutboardCatalog, OutboardLimits};
use crate::net::peer::replication::qbdl::service::{
    encode_open_body, transfer_one_leaf, Error as Svc, PlainClient, PlainProvider, ProtectedSession,
    SessionOwner, ROLE_CLIENT, ROLE_PROVIDER,
};
use crate::net::peer::replication::qbdl::wire::{Header, Kind, ServiceRecordHeader};
use crate::net::qdnf::authority::AuthorityOwner;
use crate::net::qdnf::errors::QdnfError as E;
use crate::net::qdnf::StrongDigest;

fn open_header() -> Header {
    Header {
        kind: Kind::Open,
        descriptor: StrongDigest([1; 48]),
        context: StrongDigest([2; 48]),
        publication_generation: 1,
        policy_generation: 1,
        cancellation: 0,
        transfer_id: [3; 16],
        entry: 1,
        leaf: 0,
        count: 0,
        credit_serial: 0,
        credit_bytes: 0,
        deadline: 90,
    }
}

fn open_body() -> ([u8; 72], usize) {
    let mut body = [0u8; 72];
    let n = encode_open_body(0, 0, &mut body).unwrap();
    (body, n)
}

#[test]
fn attach_refuses_zero_role_and_expired_deadline() {
    with_pair(|mut p| {
        let mut owner = SessionOwner::new();
        let clock = now;
        assert_eq!(
            ProtectedSession::attach(
                &mut p.i,
                &p.f.pins_i,
                &clock,
                now() + 60,
                0,
                &mut owner,
            )
            .err(),
            Some(Svc::Malformed)
        );
        assert_eq!(
            ProtectedSession::attach(
                &mut p.i,
                &p.f.pins_i,
                &clock,
                now().saturating_sub(1),
                ROLE_CLIENT,
                &mut owner,
            )
            .err(),
            Some(Svc::Expired)
        );
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}

#[test]
fn attach_uses_channel_transcript_and_roundtrips_control() {
    with_pair(|mut p| {
        let mut client_lanes = SessionOwner::new();
        let mut provider_lanes = SessionOwner::new();
        let clock = now;
        let deadline = now() + 300;
        let mut client = ProtectedSession::attach(
            &mut p.i,
            &p.f.pins_i,
            &clock,
            deadline,
            ROLE_CLIENT,
            &mut client_lanes,
        )
        .unwrap();
        let mut provider = ProtectedSession::attach(
            &mut p.r,
            &p.f.pins_r,
            &clock,
            deadline,
            ROLE_PROVIDER,
            &mut provider_lanes,
        )
        .unwrap();
        assert_eq!(client.closed(), false);
        let (body, n) = open_body();
        client.send_message(&open_header(), &body[..n]).unwrap();
        let (header, got, gn) = provider.recv_control().unwrap();
        assert_eq!(header.kind, Kind::Open);
        assert_eq!(header.transfer_id, [3; 16]);
        assert_eq!(gn, n);
        assert_eq!(&got[..gn], &body[..n]);
        drop(client);
        drop(provider);
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}

#[test]
fn foreign_transcript_in_aad_is_unauthorized() {
    with_pair(|mut p| {
        let mut aad = [0u8; 88];
        ServiceRecordHeader {
            transcript: StrongDigest([0xEE; 48]),
            message_id: [1; 16],
            total: 280,
            offset: 0,
            chunk: 280,
            final_chunk: true,
        }
        .encode(&mut aad)
        .unwrap();
        let chunk = [0xA5u8; 280];
        let err = p
            .i
            .with_resources(|mut io, _| {
                io.with_current(&p.f.pins_i, &now, |mut frame, current| {
                    frame.queue_record(current, &p.f.pins_i, &now, &aad, &chunk)
                })
            })
            .unwrap_err();
        assert_eq!(err, E::Unauthorized);
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}

#[test]
fn wouldblock_send_drops_authority_then_retries() {
    with_pair(|mut p| {
        let mut client_lanes = SessionOwner::new();
        let mut provider_lanes = SessionOwner::new();
        let clock = now;
        let deadline = now() + 300;
        let mut client = ProtectedSession::attach(
            &mut p.i,
            &p.f.pins_i,
            &clock,
            deadline,
            ROLE_CLIENT,
            &mut client_lanes,
        )
        .unwrap();
        let mut provider = ProtectedSession::attach(
            &mut p.r,
            &p.f.pins_r,
            &clock,
            deadline,
            ROLE_PROVIDER,
            &mut provider_lanes,
        )
        .unwrap();
        p.ti.lock().unwrap().blocks = 1;
        assert!(p.oi.try_lock().is_ok());
        let (body, n) = open_body();
        client.send_message(&open_header(), &body[..n]).unwrap();
        assert!(p.oi.try_lock().is_ok());
        let (header, _, _) = provider.recv_control().unwrap();
        assert_eq!(header.kind, Kind::Open);
        drop(client);
        drop(provider);
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}

#[test]
fn expire_after_attach_refuses_send_without_plaintext() {
    with_pair(|mut p| {
        let start = now();
        let t = std::cell::Cell::new(start);
        let clock = || t.get();
        let deadline = start + 60;
        let mut client_lanes = SessionOwner::new();
        let mut provider_lanes = SessionOwner::new();
        let mut client = ProtectedSession::attach(
            &mut p.i,
            &p.f.pins_i,
            &clock,
            deadline,
            ROLE_CLIENT,
            &mut client_lanes,
        )
        .unwrap();
        let _provider = ProtectedSession::attach(
            &mut p.r,
            &p.f.pins_r,
            &clock,
            deadline,
            ROLE_PROVIDER,
            &mut provider_lanes,
        )
        .unwrap();
        t.set(deadline);
        let (body, n) = open_body();
        assert_eq!(
            client.send_message(&open_header(), &body[..n]).err(),
            Some(Svc::Expired)
        );
        assert!(client.closed());
        drop(client);
        drop(_provider);
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}

fn leaf_payload() -> Vec<u8> {
    let n = LEAF_BYTES as usize * 2 + 19;
    let mut bytes = vec![0x5E; n];
    bytes[..LEAF_BYTES as usize].fill(0xA1);
    bytes[..4].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
    let mid = LEAF_BYTES as usize;
    bytes[mid..mid * 2].fill(0xB2);
    bytes[mid..mid + 4].copy_from_slice(&[0x0D, 0xD0, 0xFE, 0xED]);
    bytes
}

fn select_plain(fixture: &Fixture) -> SelectedPlainEntry<'_> {
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

fn build_nodes(
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

#[test]
fn useful_leaf_over_established_protected_session() {
    with_pair(|mut p| {
        let bytes = leaf_payload();
        let fixture = Fixture::new_with_payload(&bytes);
        let selected = select_plain(&fixture);
        let spec = selected.tree_spec().unwrap();
        let (nodes, root) = build_nodes(&fixture, selected);
        let catalog = OutboardCatalog::new(
            selected,
            spec,
            root,
            &nodes,
            OutboardLimits {
                max_nodes: nodes.len(),
                max_bytes: nodes.len() * std::mem::size_of::<OutboardNode>(),
            },
        )
        .unwrap();
        let provider = PlainProvider::bind(
            PublicationClass::PlainCandidate,
            fixture.source(),
            catalog,
            1,
        )
        .unwrap();
        let client = PlainClient::bind(selected, 1, 0).unwrap();
        let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
        let bound = fixture.bound(&pins, pin, publisher);
        let mut owner = AuthorityOwner::new();
        let mut grant =
            adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
        let mut client_lanes = SessionOwner::new();
        let mut provider_lanes = SessionOwner::new();
        let clock = now;
        let deadline = now() + 300;
        let mut client_session = ProtectedSession::attach(
            &mut p.i,
            &p.f.pins_i,
            &clock,
            deadline,
            ROLE_CLIENT,
            &mut client_lanes,
        )
        .unwrap();
        let mut provider_session = ProtectedSession::attach(
            &mut p.r,
            &p.f.pins_r,
            &clock,
            deadline,
            ROLE_PROVIDER,
            &mut provider_lanes,
        )
        .unwrap();
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
        assert_eq!(&got[..19], expect);
        drop(client_session);
        drop(provider_session);
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}
