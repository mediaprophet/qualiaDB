//! Hostile loopback HTTP/1.1. EvidenceClass::Component, never Process.
//! No live or paid network. JSON is not an application object.
use super::*;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{adopt_local_whole_source, tests::Fixture, CLOSURE_PURPOSE},
    proof::{OutboardNode, TreeBuilder, LEAF_BYTES},
    selection::{select_entry, SelectedPlainEntry},
};
use crate::net::peer::replication::qbdl::provider::{OutboardCatalog, OutboardLimits};
use crate::net::peer::runtime::{ReservationLedger, ResourceBudget};
use crate::net::qdnf::authority::{AuthorityOwner, ContactState};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const PATH: &[u8] = b"/objects/fixture/representations/7";

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

fn native_entry<'a>(fixture: &'a Fixture, selected: SelectedPlainEntry<'_>) -> &'a [u8] {
    let start = selected.offset() as usize;
    let len = selected.logical_length() as usize;
    &fixture.source()[start..start + len]
}

fn build_outboard(
    fixture: &Fixture,
    selected: SelectedPlainEntry<'_>,
) -> (Vec<OutboardNode>, StrongDigest) {
    let entry = native_entry(fixture, selected);
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

fn ledger() -> ReservationLedger {
    let cap = ResourceBudget {
        bytes: 8 << 20,
        work: 64,
        io: 64,
    };
    ReservationLedger::new(cap, cap, cap, cap, cap)
}

fn etag_of(digest: StrongDigest) -> [u8; 98] {
    let mut out = [0u8; 98];
    write_strong_etag(digest, &mut out).unwrap();
    out
}

fn spawn_http(
    status: u16,
    hdrs: Vec<(String, String)>,
    body: Vec<u8>,
) -> (std::net::SocketAddr, Arc<Mutex<Vec<u8>>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let cap = captured.clone();
    let th = thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_secs(2))).ok();
        s.set_write_timeout(Some(Duration::from_secs(2))).ok();
        let mut buf = [0u8; HEADER_CAP];
        let mut n = 0;
        while n < buf.len() {
            match s.read(&mut buf[n..]) {
                Ok(0) => break,
                Ok(k) => {
                    n += k;
                    if buf[..n].windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        *cap.lock().unwrap() = buf[..n].to_vec();
        let mut resp = format!("HTTP/1.1 {status} X\r\n");
        for (k, v) in &hdrs {
            resp.push_str(k);
            resp.push_str(": ");
            resp.push_str(v);
            resp.push_str("\r\n");
        }
        resp.push_str("\r\n");
        let _ = s.write_all(resp.as_bytes());
        let _ = s.write_all(&body);
        let _ = s.flush();
    });
    (addr, captured, th)
}

fn connect(addr: std::net::SocketAddr) -> TcpStream {
    let s = TcpStream::connect(addr).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    s.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
    s
}

fn host_bytes(addr: std::net::SocketAddr) -> Vec<u8> {
    format!("{addr}").into_bytes()
}

fn range_req<'a>(
    host: &'a [u8],
    range: InclusiveRange,
    total: u64,
    fallback: FallbackPolicy,
    lig: LigGate,
    if_range: Option<&'a [u8]>,
    expected: Validator<'a>,
) -> RangeRequest<'a> {
    RangeRequest {
        host,
        path: PATH,
        range,
        pinned_total: total,
        if_range,
        expected,
        fallback,
        budget: FetchBudget {
            max_body: LEGACY_FULL_OBJECT_FALLBACK_BYTES,
            max_headers: HEADER_CAP,
        },
        lig,
    }
}

struct CountIo<T> {
    inner: T,
    read: usize,
}

impl<T: Read> Read for CountIo<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.read += n;
        Ok(n)
    }
}

impl<T: Write> Write for CountIo<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[test]
fn status_200_full_entry_matches_native_bytes() {
    assert_eq!(LEGACY_FULL_OBJECT_FALLBACK_BYTES, 1 << 20);
    assert_eq!(classify_status(200), Ok(HttpStatusClass::FullObject));
    assert!(!tls_is_pq_publisher_authority());

    let fixture = Fixture::new();
    let selected = select(&fixture);
    let native = native_entry(&fixture, selected);
    assert!(!native.is_empty());
    assert!(native.len() as u64 <= LEGACY_FULL_OBJECT_FALLBACK_BYTES);

    let (nodes, root) = build_outboard(&fixture, selected);
    let catalog = catalog(selected, &nodes, root);
    let provider = HttpProvider::bind(
        fixture.source(),
        catalog,
        fixture.body(),
        fixture.entry_page(),
    )
    .unwrap();
    let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
    let bound = fixture.bound(&pins, pin, publisher);
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
    grant
        .accept_local_scoped(&pins, || 20, |authority| {
            let mut ledger = ledger();
            let mut out = vec![0u8; native.len()];
            let served = provider
                .serve(
                    HttpObject::Representation,
                    None,
                    None,
                    Validator::NONE,
                    ServeOptions {
                        fallback: FallbackPolicy::explicit(LEGACY_FULL_OBJECT_FALLBACK_BYTES)
                            .unwrap(),
                        verified_peer: true,
                    },
                    authority,
                    &mut ledger,
                    &mut out,
                )
                .expect("provider 200");
            assert_eq!(served.status, 200);
            assert_eq!(&out[..served.body_len], native);
            assert_eq!(ledger.used().host.bytes, native.len() as u64);
            ledger.release(served.charge.unwrap()).unwrap();
            Ok(())
        })
        .unwrap();

    let etag = etag_of(selected.root());
    let etag_s = std::str::from_utf8(&etag).unwrap().to_string();
    let (addr, captured, th) = spawn_http(
        200,
        vec![
            ("Content-Length".into(), native.len().to_string()),
            ("Content-Encoding".into(), "identity".into()),
            ("ETag".into(), etag_s.clone()),
        ],
        native.to_vec(),
    );
    let host = host_bytes(addr);
    let range = InclusiveRange::from_start_length(0, native.len() as u64).unwrap();
    let fallback = FallbackPolicy::explicit(LEGACY_FULL_OBJECT_FALLBACK_BYTES).unwrap();
    let req = range_req(
        &host,
        range,
        native.len() as u64,
        fallback,
        LigGate::consented_active(),
        None,
        Validator::NONE,
    );
    let mut stream = connect(addr);
    let mut out = vec![0u8; native.len()];
    let got = get_range(&mut stream, &req, &mut out).unwrap();
    assert_eq!(got, Fetched::Full { length: native.len() });
    assert_eq!(&out[..native.len()], native);
    let wire = captured.lock().unwrap().clone();
    let text = String::from_utf8_lossy(&wire);
    assert!(text.contains("Accept-Encoding: identity"));
    assert!(text.contains("Range: bytes="));
    assert!(!text.contains("Authorization:"));
    th.join().unwrap();

    let (addr, _, th) = spawn_http(
        200,
        vec![
            ("Content-Length".into(), native.len().to_string()),
            ("Content-Encoding".into(), "identity".into()),
        ],
        native.to_vec(),
    );
    let host = host_bytes(addr);
    let req = range_req(
        &host,
        range,
        native.len() as u64,
        FallbackPolicy::FORBIDDEN,
        LigGate::consented_active(),
        None,
        Validator::NONE,
    );
    let mut stream = connect(addr);
    let mut out = vec![0u8; native.len()];
    assert_eq!(
        get_range(&mut stream, &req, &mut out),
        Err(ClientError::Status(StatusError::OverBudget))
    );
    th.join().unwrap();
}

#[test]
fn status_206_content_range_exact() {
    assert_eq!(classify_status(206), Ok(HttpStatusClass::Partial));
    let fixture = Fixture::new();
    let selected = select(&fixture);
    let native = native_entry(&fixture, selected);
    assert!(native.len() >= 4);
    let take = 4u64;
    let range = InclusiveRange::from_start_length(0, take).unwrap();
    let total = native.len() as u64;
    let mut cr = [0u8; 80];
    let cn = write_content_range(range, total, &mut cr).unwrap();
    require_exact_partial(&cr[..cn], range, total).unwrap();
    assert_eq!(
        require_exact_partial(&cr[..cn], range, total + 1),
        Err(RangeError::Mismatch)
    );

    let (nodes, root) = build_outboard(&fixture, selected);
    let catalog = catalog(selected, &nodes, root);
    let provider = HttpProvider::bind(
        fixture.source(),
        catalog,
        fixture.body(),
        fixture.entry_page(),
    )
    .unwrap();
    let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
    let bound = fixture.bound(&pins, pin, publisher);
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
    grant
        .accept_local_scoped(&pins, || 20, |authority| {
            let mut ledger = ledger();
            let mut out = vec![0u8; take as usize];
            let etag = etag_of(selected.root());
            let served = provider
                .serve(
                    HttpObject::Representation,
                    Some(range),
                    Some(&etag),
                    Validator {
                        etag: Some(&etag),
                        last_modified: None,
                    },
                    ServeOptions {
                        fallback: FallbackPolicy::FORBIDDEN,
                        verified_peer: true,
                    },
                    authority,
                    &mut ledger,
                    &mut out,
                )
                .expect("provider 206");
            assert_eq!(served.status, 206);
            assert_eq!(&out[..served.body_len], &native[..take as usize]);
            assert_eq!(
                &served.headers.content_range[..served.headers.content_range_len],
                &cr[..cn]
            );
            assert_eq!(ledger.used().host.bytes, take);
            ledger.release(served.charge.unwrap()).unwrap();
            Ok(())
        })
        .unwrap();

    let etag = etag_of(selected.root());
    let (addr, captured, th) = spawn_http(
        206,
        vec![
            (
                "Content-Range".into(),
                String::from_utf8_lossy(&cr[..cn]).into_owned(),
            ),
            ("Content-Length".into(), take.to_string()),
            ("Content-Encoding".into(), "identity".into()),
            (
                "ETag".into(),
                std::str::from_utf8(&etag).unwrap().to_string(),
            ),
        ],
        native[..take as usize].to_vec(),
    );
    let host = host_bytes(addr);
    let req = range_req(
        &host,
        range,
        total,
        FallbackPolicy::FORBIDDEN,
        LigGate::consented_active(),
        Some(&etag),
        Validator {
            etag: Some(&etag),
            last_modified: None,
        },
    );
    let mut stream = connect(addr);
    let mut out = vec![0u8; take as usize];
    let got = get_range(&mut stream, &req, &mut out).unwrap();
    assert_eq!(got, Fetched::Partial { length: take as usize });
    assert_eq!(&out[..take as usize], &native[..take as usize]);
    let text = String::from_utf8_lossy(&captured.lock().unwrap());
    assert!(text.contains("Range: bytes=0-"));
    assert!(text.contains("Accept-Encoding: identity"));
    assert!(text.contains("If-Range: "));
    th.join().unwrap();
}

#[test]
fn status_416_unsatisfiable() {
    assert_eq!(classify_status(416), Ok(HttpStatusClass::Unsatisfiable));
    assert_eq!(parse_range_spec(b"bytes=0-1,2-3"), Err(RangeError::Multipart));
    assert_eq!(parse_range_spec(b"bytes=*"), Err(RangeError::Unsatisfiable));
    assert_eq!(parse_unsatisfiable_total(b"bytes */100"), Ok(100));
    assert!(parse_content_range_value(b"bytes */100")
        .ok()
        .is_some_and(|v| matches!(v, ContentRange::Unsatisfiable { total: 100 })));

    let total = 100u64;
    let range = InclusiveRange::from_start_length(100, 1).unwrap();
    assert!(!range.fits(total));
    let mut star = [0u8; 32];
    let sn = write_unsatisfiable_content_range(total, &mut star).unwrap();
    assert_eq!(&star[..sn], b"bytes */100");

    let (addr, _, th) = spawn_http(
        416,
        vec![
            ("Content-Range".into(), "bytes */100".into()),
            ("Content-Length".into(), "0".into()),
        ],
        b"not-data".to_vec(),
    );
    let host = host_bytes(addr);
    let req = range_req(
        &host,
        InclusiveRange::from_start_length(0, 4).unwrap(),
        total,
        FallbackPolicy::FORBIDDEN,
        LigGate::consented_active(),
        None,
        Validator::NONE,
    );
    let mut stream = connect(addr);
    let mut out = [0u8; 32];
    assert_eq!(
        get_range(&mut stream, &req, &mut out),
        Err(ClientError::Status(StatusError::Unsatisfiable))
    );
    assert_eq!(out, [0u8; 32]);
    th.join().unwrap();
}

#[test]
fn changed_validator_or_etag_refuses() {
    let fixture = Fixture::new();
    let selected = select(&fixture);
    let native = native_entry(&fixture, selected);
    assert!(native.len() >= 4);
    let range = InclusiveRange::from_start_length(0, 4).unwrap();
    let total = native.len() as u64;
    let mut cr = [0u8; 80];
    let cn = write_content_range(range, total, &mut cr).unwrap();
    let want = etag_of(selected.root());
    let mut other = want;
    other[2] ^= 1;

    let (addr, captured, th) = spawn_http(
        206,
        vec![
            (
                "Content-Range".into(),
                String::from_utf8_lossy(&cr[..cn]).into_owned(),
            ),
            ("Content-Length".into(), "4".into()),
            ("Content-Encoding".into(), "identity".into()),
            (
                "ETag".into(),
                std::str::from_utf8(&other).unwrap().to_string(),
            ),
        ],
        native[..4].to_vec(),
    );
    let host = host_bytes(addr);
    let req = range_req(
        &host,
        range,
        total,
        FallbackPolicy::FORBIDDEN,
        LigGate::consented_active(),
        Some(&want),
        Validator {
            etag: Some(&want),
            last_modified: None,
        },
    );
    let mut stream = connect(addr);
    let mut out = [0x5A; 4];
    assert_eq!(
        get_range(&mut stream, &req, &mut out),
        Err(ClientError::Status(StatusError::ChangedValidator))
    );
    assert_eq!(out, [0x5A; 4]);
    let text = String::from_utf8_lossy(&captured.lock().unwrap());
    assert!(text.contains("If-Range: "));
    th.join().unwrap();

    let (addr, _, th) = spawn_http(
        206,
        vec![
            (
                "Content-Range".into(),
                String::from_utf8_lossy(&cr[..cn]).into_owned(),
            ),
            ("Content-Length".into(), "4".into()),
            ("ETag".into(), "W/\"weak\"".into()),
        ],
        native[..4].to_vec(),
    );
    let host = host_bytes(addr);
    let req = range_req(
        &host,
        range,
        total,
        FallbackPolicy::FORBIDDEN,
        LigGate::consented_active(),
        None,
        Validator::NONE,
    );
    let mut stream = connect(addr);
    let mut out = [0x5A; 4];
    assert_eq!(
        get_range(&mut stream, &req, &mut out),
        Err(ClientError::Status(StatusError::ChangedValidator))
    );
    th.join().unwrap();
}

#[test]
fn content_encoding_transform_refused() {
    let range = InclusiveRange::from_start_length(0, 4).unwrap();
    let mut cr = [0u8; 80];
    let cn = write_content_range(range, 21, &mut cr).unwrap();
    assert!(!encoding_is_identity(Some(b"gzip")));
    assert!(encoding_is_identity(Some(b"identity")));
    assert!(encoding_is_identity(None));

    let (addr, _, th) = spawn_http(
        206,
        vec![
            (
                "Content-Range".into(),
                String::from_utf8_lossy(&cr[..cn]).into_owned(),
            ),
            ("Content-Length".into(), "4".into()),
            ("Content-Encoding".into(), "gzip".into()),
        ],
        b"xxxx".to_vec(),
    );
    let host = host_bytes(addr);
    let req = range_req(
        &host,
        range,
        21,
        FallbackPolicy::FORBIDDEN,
        LigGate::consented_active(),
        None,
        Validator::NONE,
    );
    let mut stream = connect(addr);
    let mut out = [0x5A; 4];
    assert_eq!(
        get_range(&mut stream, &req, &mut out),
        Err(ClientError::Status(StatusError::Encoding))
    );
    assert_eq!(out, [0x5A; 4]);
    th.join().unwrap();
}

#[test]
fn redirect_does_not_bypass_authority() {
    assert!(matches!(
        classify_status(302),
        Ok(HttpStatusClass::Redirect { code: 302 })
    ));
    assert!(url_has_userinfo(b"http://user:pass@evil.example/x"));
    assert!(!url_has_userinfo(b"/objects/x"));

    let range = InclusiveRange::from_start_length(0, 4).unwrap();
    let (addr, captured, th) = spawn_http(
        302,
        vec![
            ("Location".into(), "http://evil.example/objects/x".into()),
            ("Content-Length".into(), "0".into()),
        ],
        b"".to_vec(),
    );
    let host = host_bytes(addr);
    let denied = LigGate {
        private_consent: false,
        contact: ContactState::Active,
        group_member: true,
        paid: true,
    };
    let req = range_req(
        &host,
        range,
        21,
        FallbackPolicy::FORBIDDEN,
        denied,
        None,
        Validator::NONE,
    );
    let mut stream = connect(addr);
    let mut out = [0x5A; 4];
    assert_eq!(
        get_range(&mut stream, &req, &mut out),
        Err(ClientError::Lig(QdnfError::Denied))
    );
    assert_eq!(out, [0x5A; 4]);
    let text = String::from_utf8_lossy(&captured.lock().unwrap());
    assert!(!text.contains("Authorization:"));
    th.join().unwrap();

    let (addr, _, th) = spawn_http(
        302,
        vec![
            (
                "Location".into(),
                "http://user:pass@127.0.0.1/secret".into(),
            ),
            ("Content-Length".into(), "0".into()),
        ],
        b"".to_vec(),
    );
    let host = host_bytes(addr);
    let req = range_req(
        &host,
        range,
        21,
        FallbackPolicy::FORBIDDEN,
        LigGate::consented_active(),
        None,
        Validator::NONE,
    );
    let mut stream = connect(addr);
    let mut out = [0x5A; 4];
    assert_eq!(
        get_range(&mut stream, &req, &mut out),
        Err(ClientError::CredentialsInUrl)
    );
    th.join().unwrap();

    let (addr, _, th) = spawn_http(
        302,
        vec![
            ("Location".into(), "http://evil.example/objects/x".into()),
            ("Content-Length".into(), "0".into()),
        ],
        b"ignored-body".to_vec(),
    );
    let host = host_bytes(addr);
    let req = range_req(
        &host,
        range,
        21,
        FallbackPolicy::FORBIDDEN,
        LigGate::consented_active(),
        None,
        Validator::NONE,
    );
    let mut stream = connect(addr);
    let mut out = [0x5A; 4];
    assert_eq!(
        get_range(&mut stream, &req, &mut out),
        Err(ClientError::OriginChange)
    );
    assert_eq!(out, [0x5A; 4]);
    th.join().unwrap();
}

#[test]
fn huge_200_aborted_by_budget() {
    let huge = LEGACY_FULL_OBJECT_FALLBACK_BYTES + 1;
    let range = InclusiveRange::from_start_length(0, 4).unwrap();
    let extra = vec![0xAAu8; 32 * 1024];
    let (addr, _, th) = spawn_http(
        200,
        vec![
            ("Content-Length".into(), huge.to_string()),
            ("Content-Encoding".into(), "identity".into()),
        ],
        extra,
    );
    let host = host_bytes(addr);
    let fallback = FallbackPolicy::explicit(LEGACY_FULL_OBJECT_FALLBACK_BYTES).unwrap();
    let req = range_req(
        &host,
        range,
        21,
        fallback,
        LigGate::consented_active(),
        None,
        Validator::NONE,
    );
    let stream = connect(addr);
    let mut io = CountIo {
        inner: stream,
        read: 0,
    };
    let mut out = [0x5A; 64];
    assert_eq!(
        get_range(&mut io, &req, &mut out),
        Err(ClientError::Status(StatusError::OverBudget))
    );
    assert!(io.read < 64 * 1024, "read {} of huge 200", io.read);
    assert_eq!(out, [0x5A; 64]);
    th.join().unwrap();
}
