//! Loopback TLS HTTP/2 Extended CONNECT carrying RFC 9297 DATAGRAM capsules.
//!
//! Two peers complete RFC 8441 Extended CONNECT with `:protocol` = `capsule`.
//! CSCP (and QSession) bytes then travel as DATAGRAM capsule **values** on
//! HTTP/2 DATA. This is **not UDP** and **not MASQUE Internet**. Nested
//! recovery over a reliable stream is degraded (head-of-line blocking).

#![cfg(not(target_arch = "wasm32"))]

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use h2::ext::Protocol;
use http::{Method, Request, Response, StatusCode, Version};
use rustls::pki_types::ServerName;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::net::peer::fabric::capsule::{
    encode_datagram_capsule, CapsuleError, CAPSULE_DATAGRAM, MAX_CAPSULE_VALUE,
};
use crate::p2p::connectivity::wss_tls::{client_config, mint_server, server_config, LocalCa};

/// RFC 9297 Capsule Protocol token for `:protocol`.
pub const CAPSULE_PROTOCOL: &str = "capsule";

/// QSession fragments are MIN_QDNF_MTU (1280). CSCP control still uses the
/// fabric encoder (`MAX_CAPSULE_VALUE` = 1024). Not a MASQUE/HTTP/3 claim.
const STREAM_VALUE_MAX: usize = 2048;
const VARINT_1: u64 = 1 << 6;
const VARINT_2: u64 = 1 << 14;
const VARINT_4: u64 = 1 << 30;
const MAX_VARINT: u64 = (1 << 62) - 1;

enum Outgoing {
    Datagram(Vec<u8>),
    Raw(Vec<u8>),
}

struct Drive {
    rt: Option<tokio::runtime::Runtime>,
}

impl Drop for Drive {
    fn drop(&mut self) {
        if let Some(rt) = self.rt.take() {
            rt.shutdown_background();
        }
    }
}

struct Half {
    tx: tokio::sync::mpsc::Sender<Outgoing>,
    rx: std::sync::mpsc::Receiver<Result<Vec<u8>, String>>,
}

/// One end of a capsule-protocol HTTP/2 CONNECT stream (loopback TLS).
pub struct CapsuleEndpoint {
    tx: tokio::sync::mpsc::Sender<Outgoing>,
    rx: std::sync::mpsc::Receiver<Result<Vec<u8>, String>>,
    _drive: Arc<Drive>,
}

impl CapsuleEndpoint {
    /// Send `payload` as a DATAGRAM capsule value (`encode_datagram_capsule` when ≤ 1024).
    pub fn send_payload(&self, payload: &[u8]) -> Result<(), String> {
        self.tx
            .blocking_send(Outgoing::Datagram(payload.to_vec()))
            .map_err(|_| "h2 capsule send closed".to_string())
    }

    /// Send an already-framed capsule (used to inject unknown-critical types).
    pub fn send_raw_capsule(&self, framed: &[u8]) -> Result<(), String> {
        self.tx
            .blocking_send(Outgoing::Raw(framed.to_vec()))
            .map_err(|_| "h2 capsule send closed".to_string())
    }

    /// Receive the next DATAGRAM capsule value. Unknown critical / truncated fail closed.
    pub fn recv_payload(&self) -> Result<Vec<u8>, String> {
        match self.rx.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("h2 capsule recv timeout or closed".into()),
        }
    }
}

fn h2_tls_cfgs(
    ca: &LocalCa,
) -> Result<(Arc<rustls::ServerConfig>, Arc<rustls::ClientConfig>), String> {
    let (cert, key) = mint_server(ca)?;
    let mut server = (*server_config(&cert, &key)?).clone();
    server.alpn_protocols = vec![b"h2".to_vec()];
    let mut client = (*client_config(ca)?).clone();
    client.alpn_protocols = vec![b"h2".to_vec()];
    Ok((Arc::new(server), Arc::new(client)))
}

/// Two loopback TLS HTTP/2 peers with `:protocol` = `capsule`.
pub fn loopback_tls_h2_capsule(
    ca: &LocalCa,
) -> Result<(CapsuleEndpoint, CapsuleEndpoint), String> {
    loopback_tls_h2_connect(ca, CAPSULE_PROTOCOL)
}

/// Same CONNECT path; `protocol` is the client `:protocol` pseudo-header.
pub fn loopback_tls_h2_connect(
    ca: &LocalCa,
    protocol: &'static str,
) -> Result<(CapsuleEndpoint, CapsuleEndpoint), String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| format!("rt: {e}"))?;
    let halves = rt.block_on(establish(ca, protocol))?;
    let drive = Arc::new(Drive { rt: Some(rt) });
    Ok((
        CapsuleEndpoint {
            tx: halves.0.tx,
            rx: halves.0.rx,
            _drive: drive.clone(),
        },
        CapsuleEndpoint {
            tx: halves.1.tx,
            rx: halves.1.rx,
            _drive: drive,
        },
    ))
}

async fn establish(
    ca: &LocalCa,
    protocol: &'static str,
) -> Result<(Half, Half), String> {
    let (srv_cfg, cli_cfg) = h2_tls_cfgs(ca)?;
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;

    let server_fut = async move {
        let (tcp, _) = listener.accept().await.map_err(|e| e.to_string())?;
        let tls = TlsAcceptor::from(srv_cfg)
            .accept(tcp)
            .await
            .map_err(|e| format!("tls-s: {e}"))?;
        let mut h2 = h2::server::Builder::new()
            .enable_connect_protocol()
            .handshake(tls)
            .await
            .map_err(|e| format!("h2-s: {e}"))?;
        let (req, mut respond) = h2
            .accept()
            .await
            .ok_or_else(|| "h2 server closed before CONNECT".to_string())?
            .map_err(|e| format!("accept: {e}"))?;
        tokio::spawn(async move {
            let _ = std::future::poll_fn(|cx| h2.poll_closed(cx)).await;
        });
        let proto = req.extensions().get::<Protocol>().map(|p| p.as_str());
        if req.method() != Method::CONNECT || proto != Some(CAPSULE_PROTOCOL) {
            let _ = respond.send_response(
                Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(())
                    .unwrap(),
                true,
            );
            return Err(format!(
                "CONNECT requires method CONNECT and :protocol=capsule (got {:?} {:?})",
                req.method(),
                proto
            ));
        }
        let send = respond
            .send_response(
                Response::builder().status(StatusCode::OK).body(()).unwrap(),
                false,
            )
            .map_err(|e| format!("respond: {e}"))?;
        Ok(pump(send, req.into_body()))
    };

    let client_fut = async move {
        let tcp = TcpStream::connect(addr).await.map_err(|e| e.to_string())?;
        let name = ServerName::try_from("localhost").map_err(|e| e.to_string())?;
        let tls = TlsConnector::from(cli_cfg)
            .connect(name, tcp)
            .await
            .map_err(|e| format!("tls-c: {e}"))?;
        let (sender, conn) = h2::client::Builder::new()
            .handshake::<_, Bytes>(tls)
            .await
            .map_err(|e| format!("h2-c: {e}"))?;
        tokio::spawn(async move {
            let _ = conn.await;
        });
        let mut sender = sender.ready().await.map_err(|e| format!("ready: {e}"))?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !sender.is_extended_connect_protocol_enabled() {
            if tokio::time::Instant::now() > deadline {
                return Err("client did not receive SETTINGS_ENABLE_CONNECT_PROTOCOL".into());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
            sender = sender.ready().await.map_err(|e| format!("ready: {e}"))?;
        }
        let req = Request::builder()
            .method(Method::CONNECT)
            .uri("https://localhost/")
            .version(Version::HTTP_2)
            .extension(Protocol::from_static(protocol))
            .body(())
            .map_err(|e| format!("req: {e}"))?;
        let (resp_fut, send) = sender
            .send_request(req, false)
            .map_err(|e| format!("connect: {e}"))?;
        let resp = resp_fut.await.map_err(|e| format!("resp: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("CONNECT rejected: {}", resp.status()));
        }
        Ok(pump(send, resp.into_body()))
    };

    tokio::try_join!(client_fut, server_fut)
}

fn pump(mut send: h2::SendStream<Bytes>, mut recv: h2::RecvStream) -> Half {
    let (tx, mut rx_out) = tokio::sync::mpsc::channel::<Outgoing>(32);
    let (tx_in, rx_in) = std::sync::mpsc::sync_channel(32);
    tokio::spawn(async move {
        let mut acc = Vec::new();
        loop {
            tokio::select! {
                out = rx_out.recv() => {
                    match out {
                        None => break,
                        Some(Outgoing::Datagram(p)) => {
                            let mut buf = [0u8; 2064];
                            match encode_stream_datagram(&p, &mut buf) {
                                Ok(n) => {
                                    if send.send_data(Bytes::copy_from_slice(&buf[..n]), false).is_err() {
                                        let _ = tx_in.send(Err("send_data".into()));
                                        break;
                                    }
                                }
                                Err(e) => {
                                    let _ = tx_in.send(Err(e));
                                    break;
                                }
                            }
                        }
                        Some(Outgoing::Raw(p)) => {
                            if send.send_data(Bytes::from(p), false).is_err() {
                                let _ = tx_in.send(Err("send_data".into()));
                                break;
                            }
                        }
                    }
                }
                chunk = recv.data() => {
                    match chunk {
                        None => {
                            if !acc.is_empty() {
                                let _ = tx_in.send(Err("truncated capsule".into()));
                            }
                            break;
                        }
                        Some(Err(e)) => {
                            let _ = tx_in.send(Err(format!("h2 data: {e}")));
                            break;
                        }
                        Some(Ok(bytes)) => {
                            let n = bytes.len();
                            acc.extend_from_slice(&bytes);
                            let _ = recv.flow_control().release_capacity(n);
                            match drain_capsules(&mut acc) {
                                Ok(vals) => {
                                    for v in vals {
                                        if tx_in.send(Ok(v)).is_err() {
                                            return;
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = tx_in.send(Err(e));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    });
    Half { tx, rx: rx_in }
}

fn encode_stream_datagram(payload: &[u8], out: &mut [u8]) -> Result<usize, String> {
    if payload.len() <= MAX_CAPSULE_VALUE {
        encode_datagram_capsule(payload, out).map_err(|e| format!("{e:?}"))
    } else if payload.len() <= STREAM_VALUE_MAX {
        encode_local(CAPSULE_DATAGRAM, payload, out)
    } else {
        Err("stream capsule capacity".into())
    }
}

fn encode_local(ty: u64, payload: &[u8], out: &mut [u8]) -> Result<usize, String> {
    let n_ty = encode_varint(ty, out).map_err(|e| format!("{e:?}"))?;
    let n_len =
        encode_varint(payload.len() as u64, &mut out[n_ty..]).map_err(|e| format!("{e:?}"))?;
    let start = n_ty + n_len;
    let end = start + payload.len();
    if out.len() < end {
        return Err("capacity".into());
    }
    out[start..end].copy_from_slice(payload);
    Ok(end)
}

fn drain_capsules(acc: &mut Vec<u8>) -> Result<Vec<Vec<u8>>, String> {
    let mut out = Vec::new();
    loop {
        match take_capsule(acc)? {
            None => break,
            Some(None) => {}
            Some(Some(v)) => out.push(v),
        }
    }
    Ok(out)
}

/// `None` = need more bytes. `Some(None)` = skipped non-critical. `Some(Some(v))` = DATAGRAM.
fn take_capsule(acc: &mut Vec<u8>) -> Result<Option<Option<Vec<u8>>>, String> {
    if acc.is_empty() {
        return Ok(None);
    }
    let (ty, tlen) = match decode_varint(acc) {
        Ok(v) => v,
        Err(CapsuleError::Truncated) => return Ok(None),
        Err(e) => return Err(format!("{e:?}")),
    };
    let rest = match acc.get(tlen..) {
        Some(r) => r,
        None => return Ok(None),
    };
    let (vlen, llen) = match decode_varint(rest) {
        Ok(v) => v,
        Err(CapsuleError::Truncated) => return Ok(None),
        Err(e) => return Err(format!("{e:?}")),
    };
    if vlen > STREAM_VALUE_MAX as u64 {
        return Err("capsule capacity".into());
    }
    let header = tlen + llen;
    let end = header.saturating_add(vlen as usize);
    if acc.len() < end {
        return Ok(None);
    }
    let value = acc[header..end].to_vec();
    acc.drain(..end);
    if ty == CAPSULE_DATAGRAM {
        Ok(Some(Some(value)))
    } else if ty <= 0x3f {
        Ok(Some(None))
    } else {
        Err("unknown critical capsule".into())
    }
}

fn varint_len(value: u64) -> usize {
    if value < VARINT_1 {
        1
    } else if value < VARINT_2 {
        2
    } else if value < VARINT_4 {
        4
    } else {
        8
    }
}

fn encode_varint(value: u64, out: &mut [u8]) -> Result<usize, CapsuleError> {
    if value > MAX_VARINT {
        return Err(CapsuleError::Malformed);
    }
    let len = varint_len(value);
    if out.len() < len {
        return Err(CapsuleError::Capacity);
    }
    match len {
        1 => out[0] = value as u8,
        2 => {
            let n = (value as u16) | 0x4000;
            out[..2].copy_from_slice(&n.to_be_bytes());
        }
        4 => {
            let n = (value as u32) | 0x8000_0000;
            out[..4].copy_from_slice(&n.to_be_bytes());
        }
        _ => {
            let n = value | 0xC000_0000_0000_0000;
            out[..8].copy_from_slice(&n.to_be_bytes());
        }
    }
    Ok(len)
}

fn decode_varint(src: &[u8]) -> Result<(u64, usize), CapsuleError> {
    let first = *src.first().ok_or(CapsuleError::Truncated)?;
    let len = 1usize << (first >> 6);
    if src.len() < len {
        return Err(CapsuleError::Truncated);
    }
    let value = match len {
        1 => first as u64,
        2 => u16::from_be_bytes([src[0], src[1]]) as u64 & 0x3fff,
        4 => u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as u64 & 0x3fff_ffff,
        8 => u64::from_be_bytes([
            src[0], src[1], src[2], src[3], src[4], src[5], src[6], src[7],
        ]) & MAX_VARINT,
        _ => return Err(CapsuleError::Malformed),
    };
    if varint_len(value) != len {
        return Err(CapsuleError::Malformed);
    }
    Ok((value, len))
}
