//! Two OS-process QBDL transfer. Parent is provider; child is client.
//!
//! Reconstructs the service/tests.rs fixture independently in each process.
//! Does not pass source bytes via env or JSON. In-process `ipc_pair` is not a
//! fallback. HTTP/DNS/IP/libp2p/WSS TCP are not used.
//!
//! Framing [`super::Session`] plus caller transcript remains
//! (`EvidenceClass::Process`). A second fixture runs a real endpoint-v1
//! handshake over the same named-pipe / UDS [`FramedStream`] and then
//! [`super::ProtectedSession`] useful-leaf. Pre-shared keys travel as an
//! exact-length binary package (`QBDL-PSK1`), never JSON. This module does
//! not claim QBS-09 PhysicalNetwork.

use core::sync::atomic::{AtomicBool, Ordering};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::{
    Error, PlainClient, PlainProvider, ServedLeaf, Session, SessionOwner, ROLE_CLIENT,
    ROLE_PROVIDER,
};
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{adopt_local_whole_source, tests::Fixture, CLOSURE_PURPOSE},
    proof::{OutboardNode, TreeBuilder, LEAF_BYTES},
    publish::PublicationClass,
    selection::{select_entry, SelectedPlainEntry},
};
use crate::net::peer::replication::qbdl::provider::{OutboardCatalog, OutboardLimits};
use crate::net::qdnf::authority::AuthorityOwner;
use crate::net::qdnf::bearer::contract::Bearer;
use crate::net::qdnf::bearer::local_stream::{encode_stream_frame, LEN_SIZE};
use crate::net::qdnf::bearer::raw_ethernet::physical_two_host_qualified;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::harness::qualification::{
    EvidenceClass, Instrument, InstrumentResult, InstrumentVerdict,
};
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch, StrongDigest};

const CHILD_ENV: &str = "QDNF_TWO_PROCESS_CHILD";
const KEYS_ENV: &str = "QDNF_TWO_PROCESS_KEYS";
#[cfg(all(windows, target_arch = "x86_64"))]
const PIPE_ENV: &str = "QDNF_TWO_PROCESS_PIPE";
#[cfg(unix)]
const SOCK_ENV: &str = "QDNF_TWO_PROCESS_SOCK";
const MAGIC: &[u8; 12] = b"QBDL-PSK1\0\0\0";
const BLOB_COUNT: usize = 8;

const TRANSCRIPT: StrongDigest = StrongDigest([9; 48]);
const POLICY: u64 = 1;
const ACCEPT_SECS: u64 = 15;
const IO_SECS: u64 = 5;
const ACCEPT_POLL_MS: u64 = 5;
const MTU: u16 = 1280;
const _: () = assert!(IO_SECS == 5 && ACCEPT_SECS == 15 && MTU == 1280);

static SPAWNED: AtomicBool = AtomicBool::new(false);

fn is_child() -> bool {
    std::env::var(CHILD_ENV).ok().as_deref() == Some("1")
}

fn physical_two_host_from_this_module() -> bool {
    false
}

fn process_stream_evidence(spawned_child: bool) -> InstrumentResult {
    InstrumentResult {
        instrument: Instrument::GraphOracle,
        class: EvidenceClass::Process,
        verdict: if spawned_child {
            InstrumentVerdict::Pass
        } else {
            InstrumentVerdict::Fail
        },
        component_tables_pass: false,
    }
}

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

fn scope() -> ScopeEpoch {
    ScopeEpoch { scope: 1, epoch: 1 }
}

fn test_command() -> Result<Command, QdnfError> {
    let exe = std::env::current_exe().map_err(|_| QdnfError::Incomplete)?;
    let test_name = std::thread::current()
        .name()
        .ok_or(QdnfError::Incomplete)?
        .to_owned();
    let mut cmd = Command::new(exe);
    cmd.args(["--exact", &test_name, "--nocapture", "--test-threads", "1"])
        .env(CHILD_ENV, "1")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    Ok(cmd)
}

fn kill_wait(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn wait_success(child: &mut Child) -> Result<(), QdnfError> {
    match child.wait() {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err(QdnfError::Denied),
        Err(_) => Err(QdnfError::Closed),
    }
}

fn assert_unrelated_unrequested(got: &[u8]) {
    assert_eq!(got.len(), 19);
    assert!(!got.contains(&0xA1));
    assert!(!got.windows(4).any(|w| w == [0xDE, 0xAD, 0xBE, 0xEF]));
    assert!(!got.windows(4).any(|w| w == [0x0D, 0xD0, 0xFE, 0xED]));
}

fn run_client<B: Bearer>(framed: B, peer: ObservedLocator) {
    assert_eq!(framed.profile(), BearerProfile::LocalIpcV1);
    assert!(framed.profile().native_independent());
    let bytes = payload();
    let fixture = Fixture::new_with_payload(&bytes);
    let selected = select(&fixture);
    let spec = selected.tree_spec().unwrap();
    assert_eq!(spec.leaf_count(), 3);
    let client = PlainClient::bind(selected, POLICY, 0).unwrap();
    let mut lanes = SessionOwner::new();
    let mut session = Session::attach(framed, peer, TRANSCRIPT, ROLE_CLIENT, &mut lanes).unwrap();
    assert_eq!(session.profile(), BearerProfile::LocalIpcV1);
    let tail = spec.leaf_count() - 1;
    let open = client.send_open(&mut session).unwrap();
    let ticket = client
        .recv_grant_send_credit_request(&mut session, &open, tail)
        .unwrap();
    let mut got = [0u8; 64];
    let n = client.recv_leaf(&mut session, &ticket, &mut got).unwrap();
    assert_eq!(n, 19);
    let start = selected.offset() as usize + tail as usize * LEAF_BYTES as usize;
    let expect = &fixture.source()[start..start + 19];
    assert_eq!(&got[..19], expect);
    assert_unrelated_unrequested(&got[..19]);
}

fn run_provider<B: Bearer>(framed: B, peer: ObservedLocator) -> Result<ServedLeaf, Error> {
    assert_eq!(framed.profile(), BearerProfile::LocalIpcV1);
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
    let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
    let bound = fixture.bound(&pins, pin, publisher);
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
    let mut lanes = SessionOwner::new();
    let mut session = Session::attach(framed, peer, TRANSCRIPT, ROLE_PROVIDER, &mut lanes).unwrap();
    assert_eq!(session.profile(), BearerProfile::LocalIpcV1);
    let served = match grant.accept_local_scoped(
        &pins,
        || 20,
        |authority| {
            Ok((|| {
                let opened = provider.answer_open(&mut session, authority)?;
                provider.answer_leaf(&mut session, authority, &opened)
            })())
        },
    ) {
        Ok(inner) => inner?,
        Err(e) => return Err(super::map_policy(e)),
    };
    let tail = spec.leaf_count() - 1;
    let start = selected.offset() + 2 * LEAF_BYTES;
    if served.leaf != tail || served.length != 19 || served.source_offset != start {
        return Err(Error::Identity);
    }
    Ok(served)
}

fn finish_parent(served: ServedLeaf, child: &mut Child) {
    wait_success(child).unwrap();
    assert_eq!((served.length, served.leaf), (19, 2));
    assert!(SPAWNED.load(Ordering::SeqCst) || is_child());
    let evidence = process_stream_evidence(true);
    assert_eq!(evidence.class, EvidenceClass::Process);
    assert_ne!(evidence.class, EvidenceClass::PhysicalNetwork);
    assert!(!physical_two_host_from_this_module() && !physical_two_host_qualified());
}

#[cfg(all(windows, target_arch = "x86_64"))]
mod win {
    use super::*;
    use crate::net::qdnf::bearer::local_stream::{
        child_connect, framed_connect, FramedStream, NamedPipeListener, NamedPipeStream, PipeName,
        PIPE_NAME_PREFIX,
    };

    pub fn parent_bind_spawn_accept() -> Result<(FramedStream<NamedPipeStream>, Child), QdnfError> {
        parent_bind_spawn_accept_with(&[])
    }

    pub fn parent_bind_spawn_accept_with(
        extra: &[(&str, &str)],
    ) -> Result<(FramedStream<NamedPipeStream>, Child), QdnfError> {
        let mut listener = NamedPipeListener::bind()?;
        let name = listener.name().as_text()?;
        let mut cmd = test_command()?;
        cmd.env(PIPE_ENV, &name);
        for &(k, v) in extra {
            cmd.env(k, v);
        }
        let mut child = cmd
            .spawn()
            .map_err(|_| QdnfError::PlatformUnsupported)?;
        SPAWNED.store(true, Ordering::SeqCst);
        let deadline = Instant::now() + Duration::from_secs(ACCEPT_SECS);
        let io = loop {
            match listener.accept() {
                Ok(s) => break s,
                Err(QdnfError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        kill_wait(&mut child);
                        return Err(QdnfError::Expired);
                    }
                    if let Ok(Some(status)) = child.try_wait() {
                        return Err(if status.success() {
                            QdnfError::Closed
                        } else {
                            QdnfError::Denied
                        });
                    }
                    std::thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                }
                Err(e) => {
                    kill_wait(&mut child);
                    return Err(e);
                }
            }
        };
        match framed_connect(io, scope(), MTU) {
            Ok(framed) => Ok((framed, child)),
            Err(e) => {
                kill_wait(&mut child);
                Err(e)
            }
        }
    }

    pub fn child_connect_framed() -> Result<FramedStream<NamedPipeStream>, QdnfError> {
        let exported = std::env::var(PIPE_ENV).map_err(|_| QdnfError::Incomplete)?;
        let name = PipeName::from_exported(&exported)?;
        framed_connect(child_connect(&name)?, scope(), MTU)
    }

    pub fn refuse_tcp_and_foreign() {
        assert_eq!(
            PipeName::from_exported("127.0.0.1:0").err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            PipeName::from_exported(r"\\.\pipe\not-ours").err(),
            Some(QdnfError::Denied)
        );
        assert!(PIPE_NAME_PREFIX.starts_with(r"\\.\pipe\qbdl-"));
    }
}

#[cfg(unix)]
mod uds {
    use super::*;
    use crate::net::qdnf::bearer::local_stream::{FramedStream, UnixIpcListener, UnixStreamBytes};
    use std::path::Path;

    pub fn parent_bind_spawn_accept() -> Result<(FramedStream<UnixStreamBytes>, Child), QdnfError> {
        parent_bind_spawn_accept_with(&[])
    }

    pub fn parent_bind_spawn_accept_with(
        extra: &[(&str, &str)],
    ) -> Result<(FramedStream<UnixStreamBytes>, Child), QdnfError> {
        let mut listener = UnixIpcListener::bind()?;
        let mut cmd = test_command()?;
        cmd.env(SOCK_ENV, listener.path());
        for &(k, v) in extra {
            cmd.env(k, v);
        }
        let mut child = cmd
            .spawn()
            .map_err(|_| QdnfError::PlatformUnsupported)?;
        SPAWNED.store(true, Ordering::SeqCst);
        let deadline = Instant::now() + Duration::from_secs(ACCEPT_SECS);
        let io = loop {
            match listener.accept() {
                Ok(s) => break s,
                Err(QdnfError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        kill_wait(&mut child);
                        return Err(QdnfError::Expired);
                    }
                    if let Ok(Some(status)) = child.try_wait() {
                        return Err(if status.success() {
                            QdnfError::Closed
                        } else {
                            QdnfError::Denied
                        });
                    }
                    std::thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                }
                Err(e) => {
                    kill_wait(&mut child);
                    return Err(e);
                }
            }
        };
        match io.into_framed(scope(), MTU) {
            Ok(framed) => Ok((framed, child)),
            Err(e) => {
                kill_wait(&mut child);
                Err(e)
            }
        }
    }

    pub fn child_connect_framed() -> Result<FramedStream<UnixStreamBytes>, QdnfError> {
        let path = std::env::var(SOCK_ENV).map_err(|_| QdnfError::Incomplete)?;
        let deadline = Instant::now() + Duration::from_secs(IO_SECS);
        let stream = loop {
            match UnixStreamBytes::connect(Path::new(&path)) {
                Ok(s) => break s,
                Err(QdnfError::WouldBlock) | Err(QdnfError::Incomplete) => {
                    if Instant::now() >= deadline {
                        return Err(QdnfError::Expired);
                    }
                    std::thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                }
                Err(e) => return Err(e),
            }
        };
        stream.into_framed(scope(), MTU)
    }

    pub fn refuse_tcp_and_foreign() {
        assert_eq!(
            UnixStreamBytes::connect(Path::new("127.0.0.1:0")).err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            UnixStreamBytes::connect(Path::new(r"\\.\pipe\not-ours")).err(),
            Some(QdnfError::Denied)
        );
    }
}

#[cfg(all(windows, target_arch = "x86_64"))]
#[test]
fn two_os_process_transfers_useful_leaf() {
    assert!(!physical_two_host_from_this_module());
    assert!(!physical_two_host_qualified());
    if is_child() {
        let framed = win::child_connect_framed().unwrap();
        let peer = framed.peer();
        run_client(framed, peer);
        return;
    }
    let (framed, mut child) = match win::parent_bind_spawn_accept() {
        Ok(v) => v,
        Err(e) => panic!("named-pipe bind/accept failed (no ipc_pair/TCP fallback): {e:?}"),
    };
    let peer = framed.peer();
    let served = match run_provider(framed, peer) {
        Ok(s) => s,
        Err(e) => {
            kill_wait(&mut child);
            panic!("{e:?}");
        }
    };
    finish_parent(served, &mut child);
}

#[cfg(unix)]
#[test]
fn two_os_process_transfers_useful_leaf() {
    assert!(!physical_two_host_from_this_module());
    assert!(!physical_two_host_qualified());
    if is_child() {
        let framed = uds::child_connect_framed().unwrap();
        let peer = framed.peer();
        run_client(framed, peer);
        return;
    }
    let (framed, mut child) = match uds::parent_bind_spawn_accept() {
        Ok(v) => v,
        Err(e) => panic!("unix-domain bind/accept failed (no ipc_pair/TCP fallback): {e:?}"),
    };
    let peer = framed.peer();
    let served = match run_provider(framed, peer) {
        Ok(s) => s,
        Err(e) => {
            kill_wait(&mut child);
            panic!("{e:?}");
        }
    };
    finish_parent(served, &mut child);
}

#[cfg(all(windows, target_arch = "x86_64"))]
#[test]
fn child_refuses_tcp_and_foreign_pipe_name() {
    win::refuse_tcp_and_foreign();
}

#[cfg(unix)]
#[test]
fn child_refuses_tcp_and_foreign_pipe_name() {
    uds::refuse_tcp_and_foreign();
}

#[test]
fn wire_is_length_prefixed_binary_not_json() {
    let payload = b"\xA1\xB2\xC3\xD4";
    let mut wire_buf = [0u8; 16];
    let n = encode_stream_frame(payload, &mut wire_buf).unwrap();
    assert_ne!(wire_buf[0], b'{');
    let declared = u32::from_be_bytes(wire_buf[..LEN_SIZE].try_into().unwrap());
    assert_eq!(declared as usize, payload.len());
    assert_eq!(&wire_buf[LEN_SIZE..n], payload);
}

/// Endpoint-v1 handshake then [`super::ProtectedSession`] over the OS pipe.
/// `endpoint/tests.rs` Fixture is private; signer/intent/pin flow is rebuilt here.
#[cfg(any(unix, all(windows, target_arch = "x86_64")))]
mod protected_handshake {
    use super::*;
    use crate::crypto::network::{
        authority::{PinAuthority, PinConfig},
        cose::PairedPublicKeys,
        digest::sha384,
        key_provider::ProviderGrant,
        types::{
            KeyEpoch, KeyPurpose, ED25519_PK_LEN, ED25519_SK_LEN, ML_DSA_65_PK_LEN,
            ML_DSA_65_SK_LEN,
        },
    };
    use crate::net::peer::runtime::{ReservationLedger, ResourceBudget};
    use crate::net::qdnf::authority::{AuthorityOwner, CurrentAuthorityGuard};
    use crate::net::qdnf::session::handshake::endpoint::{
        EndpointAuthority, EndpointReservation, EndpointWorkspace, EstablishedSession, Initiator,
        Intent, PeerTrust, ProtectedChannel, Responder, Role, Route, SigningCommitment,
        ENDPOINT_RESERVATION_BYTES, INITIATOR_PURPOSE, INTENT_BYTES, RESPONDER_PURPOSE,
    };
    use crate::net::qdnf::types::OperationId;
    use crate::specialized_libs::cryptographic_library::{
        AccessLevel, AccessPolicy, AuthenticationMethod, Key, KeyAlgorithm, KeyManager,
        KeyMetadata, KeyOperation, KeyType, ManagedPairedSigning, PairedSigningRequest,
        SecurityLevel, SigningAccess, SigningControl, TimeRestrictions,
    };
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use super::super::ProtectedSession;
    use zeroize::Zeroize;

    pub(super) struct KeyFile(PathBuf);
    impl Drop for KeyFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    pub(super) struct SharedKeys {
        intent: Intent,
        initiator_ml_priv: Vec<u8>,
        initiator_ed_priv: Vec<u8>,
        initiator_ml_pub: [u8; ML_DSA_65_PK_LEN],
        initiator_ed_pub: [u8; ED25519_PK_LEN],
        responder_ml_priv: Vec<u8>,
        responder_ed_priv: Vec<u8>,
        responder_ml_pub: [u8; ML_DSA_65_PK_LEN],
        responder_ed_pub: [u8; ED25519_PK_LEN],
    }

    impl SharedKeys {
        fn zeroize_secrets(&mut self) {
            self.initiator_ml_priv.zeroize();
            self.initiator_ed_priv.zeroize();
            self.responder_ml_priv.zeroize();
            self.responder_ed_priv.zeroize();
        }
    }

    struct LocalSigner {
        library: KeyManager,
        control: SigningControl,
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn policy(id: &str) -> AccessPolicy {
        AccessPolicy {
            policy_id: id.into(),
            key_id: id.into(),
            allowed_operations: vec![KeyOperation::Sign],
            required_auth: vec![AuthenticationMethod::HardwareToken],
            time_restrictions: TimeRestrictions {
                allowed_hours: vec![],
                allowed_days: vec![],
                start_date: None,
                end_date: None,
            },
            ip_restrictions: vec![],
        }
    }

    fn private_key(id: &str, algorithm: KeyAlgorithm, data: &[u8]) -> Key {
        Key {
            key_id: id.into(),
            key_type: KeyType::Private,
            key_algorithm: algorithm,
            key_data: data.to_vec(),
            metadata: KeyMetadata {
                key_id: id.into(),
                key_type: KeyType::Private,
                key_algorithm: algorithm,
                key_size: data.len(),
                created_at: 1,
                expires_at: 0,
                last_used: 0,
                usage_count: 0,
                security_level: SecurityLevel::High,
                access_level: AccessLevel::Secret,
            },
        }
    }

    fn crypto_denied<T>(_: T) -> QdnfError {
        QdnfError::Denied
    }

    fn generate_pair(km: &mut KeyManager) -> Result<(Key, Key, Key, Key), QdnfError> {
        // Public equivalent of CryptographicLibrary::generate_managed_signing_pair
        // plus the private Key values that library.get_key would return.
        let (ml_priv, ml_pub) = km
            .generate_key_pair(
                "ml".into(),
                KeyType::Private,
                KeyAlgorithm::MLDSA,
                SecurityLevel::High,
            )
            .map_err(crypto_denied)?;
        let (ed_priv, ed_pub) = km
            .generate_key_pair(
                "ed".into(),
                KeyType::Private,
                KeyAlgorithm::EdDSA,
                SecurityLevel::High,
            )
            .map_err(crypto_denied)?;
        Ok((ml_priv, ed_priv, ml_pub, ed_pub))
    }

    fn array<const N: usize>(bytes: &[u8]) -> Result<[u8; N], QdnfError> {
        bytes.try_into().map_err(|_| QdnfError::Malformed)
    }

    fn take_digest(bytes: &[u8], p: &mut usize) -> Result<StrongDigest, QdnfError> {
        let start = *p;
        let end = start.checked_add(48).ok_or(QdnfError::Malformed)?;
        let d = StrongDigest::from_bytes(bytes.get(start..end).ok_or(QdnfError::Malformed)?)?;
        *p = end;
        Ok(d)
    }

    fn take_n<const N: usize>(bytes: &[u8], p: &mut usize) -> Result<[u8; N], QdnfError> {
        let start = *p;
        let end = start.checked_add(N).ok_or(QdnfError::Malformed)?;
        let slice = bytes.get(start..end).ok_or(QdnfError::Malformed)?;
        *p = end;
        array(slice)
    }

    fn decode_intent(bytes: &[u8]) -> Result<Intent, QdnfError> {
        if bytes.len() != INTENT_BYTES {
            return Err(QdnfError::Malformed);
        }
        let mut p = 0;
        let initiator = take_digest(bytes, &mut p)?;
        let responder = take_digest(bytes, &mut p)?;
        let service = take_digest(bytes, &mut p)?;
        let purpose = take_digest(bytes, &mut p)?;
        let instrument = take_digest(bytes, &mut p)?;
        let scope = take_digest(bytes, &mut p)?;
        let operation = take_n::<16>(bytes, &mut p)?;
        let initiator_ml = take_n::<48>(bytes, &mut p)?;
        let initiator_ed = take_n::<48>(bytes, &mut p)?;
        let responder_ml = take_n::<48>(bytes, &mut p)?;
        let responder_ed = take_n::<48>(bytes, &mut p)?;
        let initiator_epoch = u64::from_be_bytes(take_n::<8>(bytes, &mut p)?);
        let responder_epoch = u64::from_be_bytes(take_n::<8>(bytes, &mut p)?);
        let expires_unix = u64::from_be_bytes(take_n::<8>(bytes, &mut p)?);
        if p != INTENT_BYTES {
            return Err(QdnfError::Malformed);
        }
        let intent = Intent {
            initiator,
            responder,
            service,
            purpose,
            instrument,
            scope,
            operation: OperationId(operation),
            initiator_keys: crate::crypto::network::cose::KeyIds {
                ml_dsa_65: initiator_ml,
                ed25519: initiator_ed,
            },
            responder_keys: crate::crypto::network::cose::KeyIds {
                ml_dsa_65: responder_ml,
                ed25519: responder_ed,
            },
            initiator_epoch,
            responder_epoch,
            expires_unix,
        };
        intent.validate()?;
        Ok(intent)
    }

    fn push_blob(out: &mut Vec<u8>, blob: &[u8]) -> Result<(), QdnfError> {
        let len = u32::try_from(blob.len()).map_err(|_| QdnfError::Capacity)?;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(blob);
        Ok(())
    }

    fn take_blob<'a>(bytes: &'a [u8], p: &mut usize) -> Result<&'a [u8], QdnfError> {
        let start = *p;
        let end = start.checked_add(4).ok_or(QdnfError::Malformed)?;
        let len_bytes = bytes.get(start..end).ok_or(QdnfError::Malformed)?;
        let len = u32::from_be_bytes(len_bytes.try_into().map_err(|_| QdnfError::Malformed)?) as usize;
        let data_end = end.checked_add(len).ok_or(QdnfError::Malformed)?;
        let blob = bytes.get(end..data_end).ok_or(QdnfError::Malformed)?;
        *p = data_end;
        Ok(blob)
    }

    fn encode_package(keys: &SharedKeys) -> Result<Vec<u8>, QdnfError> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&keys.intent.expires_unix.to_be_bytes());
        let mut intent = [0u8; INTENT_BYTES];
        keys.intent.encode(&mut intent)?;
        out.extend_from_slice(&intent);
        push_blob(&mut out, &keys.initiator_ml_priv)?;
        push_blob(&mut out, &keys.initiator_ed_priv)?;
        push_blob(&mut out, &keys.initiator_ml_pub)?;
        push_blob(&mut out, &keys.initiator_ed_pub)?;
        push_blob(&mut out, &keys.responder_ml_priv)?;
        push_blob(&mut out, &keys.responder_ed_priv)?;
        push_blob(&mut out, &keys.responder_ml_pub)?;
        push_blob(&mut out, &keys.responder_ed_pub)?;
        Ok(out)
    }

    pub(super) fn parse_key_package(bytes: &[u8]) -> Result<SharedKeys, QdnfError> {
        if bytes.len() < MAGIC.len() {
            return Err(QdnfError::Malformed);
        }
        if &bytes[..MAGIC.len()] != MAGIC.as_slice() {
            return Err(QdnfError::Denied);
        }
        let mut p = MAGIC.len();
        let exp_end = p.checked_add(8).ok_or(QdnfError::Malformed)?;
        let expires = u64::from_be_bytes(
            bytes
                .get(p..exp_end)
                .ok_or(QdnfError::Malformed)?
                .try_into()
                .map_err(|_| QdnfError::Malformed)?,
        );
        p = exp_end;
        let intent_end = p.checked_add(INTENT_BYTES).ok_or(QdnfError::Malformed)?;
        let intent = decode_intent(bytes.get(p..intent_end).ok_or(QdnfError::Malformed)?)?;
        if intent.expires_unix != expires {
            return Err(QdnfError::Malformed);
        }
        if now() >= expires {
            return Err(QdnfError::Expired);
        }
        p = intent_end;
        let i_ml_priv = take_blob(bytes, &mut p)?;
        let i_ed_priv = take_blob(bytes, &mut p)?;
        let i_ml_pub = take_blob(bytes, &mut p)?;
        let i_ed_pub = take_blob(bytes, &mut p)?;
        let r_ml_priv = take_blob(bytes, &mut p)?;
        let r_ed_priv = take_blob(bytes, &mut p)?;
        let r_ml_pub = take_blob(bytes, &mut p)?;
        let r_ed_pub = take_blob(bytes, &mut p)?;
        if p != bytes.len() {
            return Err(QdnfError::Malformed);
        }
        if i_ml_priv.len() != ML_DSA_65_SK_LEN
            || r_ml_priv.len() != ML_DSA_65_SK_LEN
            || i_ed_priv.len() != ED25519_SK_LEN
            || r_ed_priv.len() != ED25519_SK_LEN
            || i_ml_pub.len() != ML_DSA_65_PK_LEN
            || r_ml_pub.len() != ML_DSA_65_PK_LEN
            || i_ed_pub.len() != ED25519_PK_LEN
            || r_ed_pub.len() != ED25519_PK_LEN
        {
            return Err(QdnfError::Malformed);
        }
        Ok(SharedKeys {
            intent,
            initiator_ml_priv: i_ml_priv.to_vec(),
            initiator_ed_priv: i_ed_priv.to_vec(),
            initiator_ml_pub: array(i_ml_pub)?,
            initiator_ed_pub: array(i_ed_pub)?,
            responder_ml_priv: r_ml_priv.to_vec(),
            responder_ed_priv: r_ed_priv.to_vec(),
            responder_ml_pub: array(r_ml_pub)?,
            responder_ed_pub: array(r_ed_pub)?,
        })
    }

    pub(super) fn generate_and_write_package() -> Result<(SharedKeys, PathBuf, KeyFile), QdnfError> {
        let mut km = KeyManager::new();
        km.initialize().map_err(crypto_denied)?;
        let (i_ml_priv, i_ed_priv, i_ml_pub, i_ed_pub) = generate_pair(&mut km)?;
        let (r_ml_priv, r_ed_priv, r_ml_pub, r_ed_pub) = generate_pair(&mut km)?;
        if i_ml_priv.key_data.len() != ML_DSA_65_SK_LEN
            || r_ml_priv.key_data.len() != ML_DSA_65_SK_LEN
            || i_ed_priv.key_data.len() != ED25519_SK_LEN
            || r_ed_priv.key_data.len() != ED25519_SK_LEN
        {
            return Err(QdnfError::Malformed);
        }
        let i_ml_pub_arr = array::<ML_DSA_65_PK_LEN>(i_ml_pub.key_data.as_slice())?;
        let i_ed_pub_arr = array::<ED25519_PK_LEN>(i_ed_pub.key_data.as_slice())?;
        let r_ml_pub_arr = array::<ML_DSA_65_PK_LEN>(r_ml_pub.key_data.as_slice())?;
        let r_ed_pub_arr = array::<ED25519_PK_LEN>(r_ed_pub.key_data.as_slice())?;
        let expires = now() + 300;
        let intent = Intent {
            initiator: sha384(b"initiator"),
            responder: sha384(b"responder"),
            service: sha384(b"qbdl"),
            purpose: sha384(b"stream"),
            instrument: sha384(b"grant"),
            scope: sha384(b"source"),
            operation: OperationId([1; 16]),
            initiator_keys: PairedPublicKeys {
                ml_dsa_65: &i_ml_pub_arr,
                ed25519: &i_ed_pub_arr,
            }
            .key_ids(),
            responder_keys: PairedPublicKeys {
                ml_dsa_65: &r_ml_pub_arr,
                ed25519: &r_ed_pub_arr,
            }
            .key_ids(),
            initiator_epoch: 1,
            responder_epoch: 1,
            expires_unix: expires,
        };
        intent.validate()?;
        let mut keys = SharedKeys {
            intent,
            initiator_ml_priv: i_ml_priv.key_data,
            initiator_ed_priv: i_ed_priv.key_data,
            initiator_ml_pub: i_ml_pub_arr,
            initiator_ed_pub: i_ed_pub_arr,
            responder_ml_priv: r_ml_priv.key_data,
            responder_ed_priv: r_ed_priv.key_data,
            responder_ml_pub: r_ml_pub_arr,
            responder_ed_pub: r_ed_pub_arr,
        };
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "qbdl-psk1-{}-{nanos}",
            std::process::id()
        ));
        let mut bytes = encode_package(&keys)?;
        std::fs::write(&path, &bytes).map_err(|_| QdnfError::Incomplete)?;
        bytes.zeroize();
        keys.initiator_ml_priv.zeroize();
        keys.initiator_ed_priv.zeroize();
        Ok((keys, path.clone(), KeyFile(path)))
    }

    fn read_keys_from_env() -> Result<SharedKeys, QdnfError> {
        let path = std::env::var(KEYS_ENV).map_err(|_| QdnfError::Incomplete)?;
        if path.is_empty() {
            return Err(QdnfError::Incomplete);
        }
        read_keys_from_path(Path::new(&path))
    }

    pub(super) fn read_keys_from_path(path: &Path) -> Result<SharedKeys, QdnfError> {
        let mut bytes = std::fs::read(path).map_err(|_| QdnfError::Incomplete)?;
        let keys = parse_key_package(&bytes)?;
        bytes.zeroize();
        Ok(keys)
    }

    impl LocalSigner {
        fn from_secrets(ml_priv: &[u8], ed_priv: &[u8]) -> Result<Self, QdnfError> {
            let mut library = KeyManager::new();
            library.initialize().map_err(crypto_denied)?;
            library
                .store_key(private_key("ml_private", KeyAlgorithm::MLDSA, ml_priv))
                .map_err(crypto_denied)?;
            library
                .store_key(private_key("ed_private", KeyAlgorithm::EdDSA, ed_priv))
                .map_err(crypto_denied)?;
            library.install_managed_signing_policy(policy("ml_private"));
            library.install_managed_signing_policy(policy("ed_private"));
            Ok(Self {
                library,
                control: SigningControl::new(1).map_err(|_| QdnfError::Unauthorized)?,
            })
        }

        fn capability<'a>(
            &'a self,
            c: &'a SigningCommitment,
            clock: &'a dyn Fn() -> u64,
        ) -> Result<ManagedPairedSigning<'a>, QdnfError> {
            self.library
                .begin_paired_signing(
                    PairedSigningRequest {
                        ml_key_id: "ml_private",
                        ed_key_id: "ed_private",
                        ml_policy_id: "ml_private",
                        ed_policy_id: "ed_private",
                        binding: c.binding(),
                        grant: ProviderGrant {
                            epoch: KeyEpoch {
                                purpose: KeyPurpose::ControllerSign,
                                epoch: 1,
                                controller: 7,
                            },
                            cell_epoch: 1,
                            authority_generation: 1,
                            deadline_unix: u64::MAX,
                            revoked: false,
                        },
                        access: SigningAccess {
                            authenticated_methods: &[AuthenticationMethod::HardwareToken],
                            ip_address: "local",
                        },
                    },
                    &self.control,
                    clock,
                )
                .map_err(|_| QdnfError::Denied)
        }
    }

    fn ledger() -> Mutex<ReservationLedger> {
        let cap = ResourceBudget {
            bytes: 2 * ENDPOINT_RESERVATION_BYTES,
            work: 64,
            io: 64,
        };
        Mutex::new(ReservationLedger::new(cap, cap, cap, cap, cap))
    }

    fn pin_peer(
        pins: &mut PinAuthority,
        admin: &crate::crypto::network::authority::LocalBootstrap,
        principal: StrongDigest,
        audience: &[u8; 48],
        purpose: &[u8],
        ml: &[u8; ML_DSA_65_PK_LEN],
        ed: &[u8; ED25519_PK_LEN],
        time: u64,
        expires: u64,
    ) -> Result<PeerTrust, QdnfError> {
        let pin = pins.install_pin(
            admin,
            PinConfig {
                principal,
                provider_epoch: KeyEpoch {
                    purpose: KeyPurpose::ControllerSign,
                    epoch: 1,
                    controller: 7,
                },
                keys: PairedPublicKeys {
                    ml_dsa_65: ml,
                    ed25519: ed,
                },
                purpose,
                audience,
                not_before_unix: time,
                expires_unix: expires,
            },
            time,
        )?;
        Ok(PeerTrust { pin })
    }

    fn poll_step(
        deadline: Instant,
        wait: bool,
        mut step: impl FnMut() -> Result<bool, QdnfError>,
    ) -> Result<(), QdnfError> {
        loop {
            match step() {
                Ok(true) => return Ok(()),
                Ok(false) if !wait => {
                    if Instant::now() >= deadline {
                        return Err(QdnfError::Expired);
                    }
                    continue;
                }
                Ok(false) | Err(QdnfError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        return Err(QdnfError::Expired);
                    }
                    std::thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn into_channel<'a, B: Bearer>(
        installed: EstablishedSession<'a>,
        framed: B,
        owner: &'a Mutex<AuthorityOwner>,
        pins: &PinAuthority,
    ) -> Result<ProtectedChannel<'a, B>, QdnfError> {
        let current = CurrentAuthorityGuard::try_lock(owner)?;
        let channel = match installed.into_protected_channel(framed, &current, pins, &now) {
            Ok(channel) => channel,
            Err(failed) => return Err(failed.error),
        };
        drop(current);
        Ok(channel)
    }

    fn useful_leaf_client<B: Bearer>(session: &mut ProtectedSession<'_, '_, B>) {
        let bytes = payload();
        let fixture = Fixture::new_with_payload(&bytes);
        let selected = select(&fixture);
        let spec = selected.tree_spec().unwrap();
        assert_eq!(spec.leaf_count(), 3);
        let client = PlainClient::bind(selected, POLICY, 0).unwrap();
        let tail = spec.leaf_count() - 1;
        let open = client.send_open(session).unwrap();
        let ticket = client
            .recv_grant_send_credit_request(session, &open, tail)
            .unwrap();
        let mut got = [0u8; 64];
        let n = client.recv_leaf(session, &ticket, &mut got).unwrap();
        assert_eq!(n, 19);
        let start = selected.offset() as usize + tail as usize * LEAF_BYTES as usize;
        let expect = &fixture.source()[start..start + 19];
        assert_eq!(&got[..19], expect);
        assert_unrelated_unrequested(&got[..19]);
    }

    fn useful_leaf_provider<B: Bearer>(
        session: &mut ProtectedSession<'_, '_, B>,
    ) -> Result<ServedLeaf, Error> {
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
        let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
        let bound = fixture.bound(&pins, pin, publisher);
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
        let served = match grant.accept_local_scoped(
            &pins,
            || 20,
            |authority| {
                Ok((|| {
                    let opened = provider.answer_open(session, authority)?;
                    provider.answer_leaf(session, authority, &opened)
                })())
            },
        ) {
            Ok(inner) => inner?,
            Err(e) => return Err(super::super::map_policy(e)),
        };
        let tail = spec.leaf_count() - 1;
        let start = selected.offset() + 2 * LEAF_BYTES;
        if served.leaf != tail || served.length != 19 || served.source_offset != start {
            return Err(Error::Identity);
        }
        Ok(served)
    }

    pub(super) fn run_protected_client<B: Bearer>(mut framed: B) {
        assert_eq!(framed.profile(), BearerProfile::LocalIpcV1);
        assert!(framed.profile().native_independent());
        let mut keys = match read_keys_from_env() {
            Ok(k) => k,
            Err(e) => panic!(
                "protected child refused without key package ({e:?}); no Session/TRANSCRIPT fallback"
            ),
        };
        let signer = LocalSigner::from_secrets(&keys.initiator_ml_priv, &keys.initiator_ed_priv)
            .expect("initiator store_key");
        keys.zeroize_secrets();
        let owner = Mutex::new(AuthorityOwner::new());
        let ledger = ledger();
        let (mut pins, admin) = PinAuthority::new_local_bootstrap().unwrap();
        let time = now();
        let peer = pin_peer(
            &mut pins,
            &admin,
            keys.intent.responder,
            &keys.intent.initiator.0,
            RESPONDER_PURPOSE,
            &keys.responder_ml_pub,
            &keys.responder_ed_pub,
            time,
            keys.intent.expires_unix,
        )
        .unwrap();
        let authority = EndpointAuthority::adopt_local(
            &owner,
            keys.intent,
            Role::Initiator,
            time,
            ENDPOINT_RESERVATION_BYTES,
        )
        .unwrap();
        let route = Route::observed(&framed, framed.peer()).unwrap();
        let mut workspace = Box::write(
            Box::<EndpointWorkspace>::new_uninit(),
            EndpointWorkspace::new(),
        );
        let mut initiator = Initiator::new(
            &authority,
            EndpointReservation::admit(&ledger).unwrap(),
            workspace.as_mut(),
            route,
            peer,
            &now,
        )
        .unwrap();
        let commitment = initiator.signing_commitment().unwrap();
        initiator
            .sign_client(signer.capability(&commitment, &now).unwrap(), &pins, &now)
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(ACCEPT_SECS);
        poll_step(deadline, false, || initiator.send_step(&mut framed, &pins, &now)).unwrap();
        poll_step(deadline, true, || initiator.receive_step(&mut framed, &pins, &now)).unwrap();
        poll_step(deadline, false, || initiator.send_step(&mut framed, &pins, &now)).unwrap();
        let installed = initiator.install(&pins, &now).unwrap();
        drop(initiator);
        drop(workspace);
        drop(signer);
        let mut channel = into_channel(installed, framed, &owner, &pins).unwrap();
        let mut lanes = SessionOwner::new();
        let clock = now;
        let io_deadline = now() + 300;
        let mut session = ProtectedSession::attach(
            &mut channel,
            &pins,
            &clock,
            io_deadline,
            ROLE_CLIENT,
            &mut lanes,
        )
        .unwrap();
        useful_leaf_client(&mut session);
        drop(session);
        let _ = channel.try_close();
    }

    pub(super) fn run_protected_provider<B: Bearer>(
        mut framed: B,
        keys: &mut SharedKeys,
    ) -> Result<ServedLeaf, Error> {
        assert_eq!(framed.profile(), BearerProfile::LocalIpcV1);
        let signer = LocalSigner::from_secrets(&keys.responder_ml_priv, &keys.responder_ed_priv)
            .map_err(|_| Error::Authority)?;
        keys.responder_ml_priv.zeroize();
        keys.responder_ed_priv.zeroize();
        let owner = Mutex::new(AuthorityOwner::new());
        let ledger = ledger();
        let (mut pins, admin) = PinAuthority::new_local_bootstrap().map_err(|_| Error::Authority)?;
        let time = now();
        let peer = pin_peer(
            &mut pins,
            &admin,
            keys.intent.initiator,
            &keys.intent.responder.0,
            INITIATOR_PURPOSE,
            &keys.initiator_ml_pub,
            &keys.initiator_ed_pub,
            time,
            keys.intent.expires_unix,
        )
        .map_err(|_| Error::Authority)?;
        let authority = EndpointAuthority::adopt_local(
            &owner,
            keys.intent,
            Role::Responder,
            time,
            ENDPOINT_RESERVATION_BYTES,
        )
        .map_err(super::super::from_qdnf)?;
        let route = Route::observed(&framed, framed.peer()).map_err(super::super::from_qdnf)?;
        let mut workspace = Box::write(
            Box::<EndpointWorkspace>::new_uninit(),
            EndpointWorkspace::new(),
        );
        let mut responder = Responder::new(
            &authority,
            EndpointReservation::admit(&ledger).map_err(super::super::from_qdnf)?,
            workspace.as_mut(),
            route,
            peer,
            &now,
        )
        .map_err(super::super::from_qdnf)?;
        let deadline = Instant::now() + Duration::from_secs(ACCEPT_SECS);
        poll_step(deadline, true, || responder.receive_step(&mut framed, &pins, &now))
            .map_err(super::super::from_qdnf)?;
        let commitment = responder
            .signing_commitment()
            .map_err(super::super::from_qdnf)?;
        responder
            .sign_server(
                signer
                    .capability(&commitment, &now)
                    .map_err(super::super::from_qdnf)?,
                &pins,
                &now,
            )
            .map_err(super::super::from_qdnf)?;
        poll_step(deadline, false, || responder.send_step(&mut framed, &pins, &now))
            .map_err(super::super::from_qdnf)?;
        poll_step(deadline, true, || responder.receive_step(&mut framed, &pins, &now))
            .map_err(super::super::from_qdnf)?;
        let installed = responder
            .install(&pins, &now)
            .map_err(super::super::from_qdnf)?;
        drop(responder);
        drop(workspace);
        drop(signer);
        let mut channel = into_channel(installed, framed, &owner, &pins).map_err(super::super::from_qdnf)?;
        let mut lanes = SessionOwner::new();
        let clock = now;
        let io_deadline = now() + 300;
        let mut session = ProtectedSession::attach(
            &mut channel,
            &pins,
            &clock,
            io_deadline,
            ROLE_PROVIDER,
            &mut lanes,
        )?;
        let served = useful_leaf_provider(&mut session);
        drop(session);
        let _ = channel.try_close();
        served
    }

    const _: () = assert!(BLOB_COUNT == 8);
    const _: () = assert!(MAGIC.len() == 12);
    const _: () = assert!(INTENT_BYTES == 520);
}

#[cfg(any(unix, all(windows, target_arch = "x86_64")))]
#[test]
fn truncated_key_package_is_malformed() {
    assert_eq!(
        protected_handshake::parse_key_package(&[]).err(),
        Some(QdnfError::Malformed)
    );
    assert_eq!(
        protected_handshake::parse_key_package(b"QBDL-PSK1\0\0").err(),
        Some(QdnfError::Malformed)
    );
    let mut mutated = *MAGIC;
    mutated[0] ^= 1;
    assert_eq!(
        protected_handshake::parse_key_package(&mutated).err(),
        Some(QdnfError::Denied)
    );
}

#[cfg(any(unix, all(windows, target_arch = "x86_64")))]
#[test]
fn protected_child_without_key_package_is_incomplete() {
    assert_eq!(
        protected_handshake::read_keys_from_path(std::path::Path::new("")).err(),
        Some(QdnfError::Incomplete)
    );
}

#[cfg(all(windows, target_arch = "x86_64"))]
#[test]
fn two_os_process_protected_session_transfers_useful_leaf() {
    assert!(!physical_two_host_from_this_module());
    assert!(!physical_two_host_qualified());
    if is_child() {
        let framed = win::child_connect_framed().unwrap();
        protected_handshake::run_protected_client(framed);
        return;
    }
    let (mut keys, path, _guard) = protected_handshake::generate_and_write_package().unwrap();
    let path_text = path.to_str().expect("utf8 key path");
    let (framed, mut child) = match win::parent_bind_spawn_accept_with(&[(KEYS_ENV, path_text)]) {
        Ok(v) => v,
        Err(e) => panic!("named-pipe bind/accept failed (no ipc_pair/TCP fallback): {e:?}"),
    };
    let served = match protected_handshake::run_protected_provider(framed, &mut keys) {
        Ok(s) => s,
        Err(e) => {
            kill_wait(&mut child);
            panic!("{e:?}");
        }
    };
    finish_parent(served, &mut child);
}

#[cfg(unix)]
#[test]
fn two_os_process_protected_session_transfers_useful_leaf() {
    assert!(!physical_two_host_from_this_module());
    assert!(!physical_two_host_qualified());
    if is_child() {
        let framed = uds::child_connect_framed().unwrap();
        protected_handshake::run_protected_client(framed);
        return;
    }
    let (mut keys, path, _guard) = protected_handshake::generate_and_write_package().unwrap();
    let path_text = path.to_str().expect("utf8 key path");
    let (framed, mut child) = match uds::parent_bind_spawn_accept_with(&[(KEYS_ENV, path_text)]) {
        Ok(v) => v,
        Err(e) => panic!("unix-domain bind/accept failed (no ipc_pair/TCP fallback): {e:?}"),
    };
    let served = match protected_handshake::run_protected_provider(framed, &mut keys) {
        Ok(s) => s,
        Err(e) => {
            kill_wait(&mut child);
            panic!("{e:?}");
        }
    };
    finish_parent(served, &mut child);
}
