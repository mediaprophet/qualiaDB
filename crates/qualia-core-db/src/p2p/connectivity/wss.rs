//! Authenticated WebSocket binary-datagram relay (RFC 6455). Local TCP; not a public URL.

#![cfg(not(target_arch = "wasm32"))]

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};
use sha1::{Digest, Sha1};
use sha2::Sha256;

use crate::net::qdnf::errors::QdnfError;

#[path = "wss_frame.rs"]
mod wss_frame;

pub use wss_frame::{
    fresh_client_mask, io_from_qdnf, map_io, ReadCursor, WriteCursor, OP_BINARY, OP_CLOSE,
    WIRE_CAP, WS_HDR_MAX,
};

const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
pub const MAX_FRAME: usize = 2048;
pub const ENVELOPE_HDR: usize = 12;
pub const MAX_PAYLOAD: usize = MAX_FRAME - ENVELOPE_HDR;
pub const ENVELOPE_VERSION: u8 = 2;
type HmacSha256 = Hmac<Sha256>;

pub const ENV_CHILD: &str = "QDNF_WSS_RELAY_CHILD";
pub const ENV_ADDR: &str = "QDNF_WSS_ADDR";
pub const ENV_SECRET: &str = "QDNF_WSS_SECRET";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Auth = 0,
    Datagram = 1,
    Close = 2,
}

/// RFC 6455 Sec-WebSocket-Accept: base64(SHA-1(key + GUID)).
pub fn accept_key(sec_key: &str) -> String {
    let mut h = Sha1::new();
    h.update(sec_key.as_bytes());
    h.update(GUID);
    STANDARD.encode(h.finalize())
}

/// HMAC-SHA256 over `(circuit, gen)` under `secret` — per-circuit auth token.
pub fn hmac_auth(secret: &[u8], circuit: u32, gen: u32) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(secret).expect("hmac key");
    mac.update(&circuit.to_be_bytes());
    mac.update(&gen.to_be_bytes());
    let out = mac.finalize().into_bytes();
    let mut a = [0u8; 32];
    a.copy_from_slice(&out);
    a
}

/// Encodes a versioned frame envelope `(type, circuit, gen, len, payload)` into `out`; fails closed on capacity.
pub fn encode_envelope(
    typ: FrameType,
    circuit: u32,
    gen: u32,
    payload: &[u8],
    out: &mut [u8],
) -> Result<usize, &'static str> {
    if payload.len() > MAX_PAYLOAD {
        return Err("capacity");
    }
    let n = ENVELOPE_HDR + payload.len();
    if n > MAX_FRAME || out.len() < n {
        return Err("capacity");
    }
    out[0] = ENVELOPE_VERSION;
    out[1] = typ as u8;
    out[2..6].copy_from_slice(&circuit.to_be_bytes());
    out[6..10].copy_from_slice(&gen.to_be_bytes());
    out[10..12].copy_from_slice(&(payload.len() as u16).to_be_bytes());
    out[12..n].copy_from_slice(payload);
    Ok(n)
}

/// Decodes a frame envelope; fails closed on bad version/obsolete format/truncation.
pub fn decode_envelope(buf: &[u8]) -> Result<(FrameType, u32, u32, &[u8]), &'static str> {
    if buf.len() < ENVELOPE_HDR {
        return Err("envelope");
    }
    if buf[0] == 1 {
        return Err("obsolete-u8-length");
    }
    if buf[0] != ENVELOPE_VERSION {
        return Err("envelope");
    }
    let typ = match buf[1] {
        0 => FrameType::Auth,
        1 => FrameType::Datagram,
        2 => FrameType::Close,
        _ => return Err("type"),
    };
    let circuit = u32::from_be_bytes([buf[2], buf[3], buf[4], buf[5]]);
    let gen = u32::from_be_bytes([buf[6], buf[7], buf[8], buf[9]]);
    let len = u16::from_be_bytes([buf[10], buf[11]]) as usize;
    if len > MAX_PAYLOAD {
        return Err("capacity");
    }
    if buf.len() < ENVELOPE_HDR + len {
        return Err("truncated");
    }
    Ok((typ, circuit, gen, &buf[ENVELOPE_HDR..ENVELOPE_HDR + len]))
}

/// Bound circuit/generation. Wrong circuit is unauthorized; older/other gen is stale.
pub fn check_circuit_gen(
    circuit: u32,
    gen: u32,
    expect_c: u32,
    expect_g: u32,
) -> Result<(), QdnfError> {
    if circuit != expect_c {
        return Err(QdnfError::Unauthorized);
    }
    if gen != expect_g {
        return Err(QdnfError::StaleGeneration);
    }
    Ok(())
}

fn encode_env(
    typ: FrameType,
    circuit: u32,
    gen: u32,
    payload: &[u8],
    out: &mut [u8],
) -> Result<usize, QdnfError> {
    encode_envelope(typ, circuit, gen, payload, out).map_err(|_| QdnfError::Capacity)
}

/// Seal an application envelope as a WS binary frame. Fresh mask only when idle.
pub fn seal_datagram(
    write: &mut WriteCursor,
    circuit: u32,
    gen: u32,
    payload: &[u8],
    client: bool,
) -> Result<(), QdnfError> {
    let mut env = [0u8; MAX_FRAME];
    let n = encode_env(FrameType::Datagram, circuit, gen, payload, &mut env)?;
    let mask = if write.pending() {
        None
    } else if client {
        Some(fresh_client_mask()?)
    } else {
        None
    };
    write.seal(OP_BINARY, &env[..n], mask)
}

/// Seal an application Close envelope as RFC 6455 opcode 8. Same pending close
/// resumes. A pending datagram is Conflict and is not aborted or resealed.
pub fn seal_close(
    write: &mut WriteCursor,
    circuit: u32,
    gen: u32,
    client: bool,
) -> Result<(), QdnfError> {
    if write.pending() {
        if write.sealed_opcode() == Some(OP_CLOSE) {
            return Ok(());
        }
        return Err(QdnfError::Conflict);
    }
    let mut env = [0u8; ENVELOPE_HDR];
    let n = encode_env(FrameType::Close, circuit, gen, &[], &mut env)?;
    let mask = if client {
        Some(fresh_client_mask()?)
    } else {
        None
    };
    write.seal(OP_CLOSE, &env[..n], mask)
}

fn decode_env(buf: &[u8]) -> Result<(FrameType, u32, u32, &[u8]), QdnfError> {
    decode_envelope(buf).map_err(|e| match e {
        "obsolete-u8-length" => QdnfError::Downgrade,
        "capacity" => QdnfError::Capacity,
        "truncated" => QdnfError::Truncated,
        "type" => QdnfError::Unsupported,
        _ => QdnfError::Malformed,
    })
}

/// Receive one WS frame, decode the envelope, and refuse wrong circuit / stale gen.
pub fn recv_checked<R: Read>(
    stream: &mut R,
    read: &mut ReadCursor,
    circuit: u32,
    gen: u32,
    expect_masked: Option<bool>,
    out: &mut [u8],
) -> Result<(FrameType, usize), QdnfError> {
    let mut env = [0u8; MAX_FRAME];
    let (opcode, n) = read.recv(stream, &mut env, expect_masked)?;
    let (typ, c, g, payload) = decode_env(&env[..n])?;
    check_circuit_gen(c, g, circuit, gen)?;
    if opcode == OP_CLOSE {
        if typ != FrameType::Close {
            return Err(QdnfError::Malformed);
        }
        return Ok((FrameType::Close, 0));
    }
    if opcode != OP_BINARY {
        return Err(QdnfError::Unsupported);
    }
    if typ == FrameType::Close {
        return Ok((FrameType::Close, 0));
    }
    if payload.len() > out.len() {
        return Err(QdnfError::Capacity);
    }
    if !payload.is_empty() {
        out[..payload.len()].copy_from_slice(payload);
    }
    Ok((typ, payload.len()))
}

/// Drives the client-side WSS handshake on `stream` with `key_b64`.
pub fn client_handshake<S: Read + Write>(stream: &mut S, key_b64: &str) -> std::io::Result<()> {
    let req = format!(
        "GET /relay HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key_b64}\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    stream.write_all(req.as_bytes())?;
    stream.flush()?;
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf)?;
    let text = core::str::from_utf8(&buf[..n]).unwrap_or("");
    if !text.contains("101") || !text.to_ascii_lowercase().contains("upgrade") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "no 101",
        ));
    }
    let expect = accept_key(key_b64);
    if !text.contains(&expect) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "bad accept",
        ));
    }
    Ok(())
}

/// Drives the server-side handshake — reads the request, responds with the computed accept key.
pub fn server_handshake<S: Read + Write>(stream: &mut S) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf)?;
    let text = core::str::from_utf8(&buf[..n]).unwrap_or("");
    let key = text
        .lines()
        .find_map(|l| l.strip_prefix("Sec-WebSocket-Key: "))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "no key"))?
        .trim();
    let accept = accept_key(key);
    let resp = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
    );
    stream.write_all(resp.as_bytes())?;
    stream.flush()
}

/// Pair of already-upgraded WebSocket ends (same process).
pub fn loopback_pair() -> std::io::Result<(TcpStream, TcpStream)> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let client = TcpStream::connect(addr)?;
    let (server, _) = listener.accept()?;
    client.set_read_timeout(Some(Duration::from_secs(2)))?;
    server.set_read_timeout(Some(Duration::from_secs(2)))?;
    let key = STANDARD.encode([7u8; 16]);
    let mut c = client.try_clone()?;
    let mut s = server.try_clone()?;
    let th = std::thread::spawn(move || client_handshake(&mut c, &key));
    server_handshake(&mut s)?;
    th.join().unwrap()?;
    Ok((client, server))
}

/// Sends a datagram frame on the circuit.
pub fn send_datagram<W: Write>(
    stream: &mut W,
    circuit: u32,
    gen: u32,
    payload: &[u8],
    client: bool,
) -> std::io::Result<()> {
    let mut write = WriteCursor::new();
    seal_datagram(&mut write, circuit, gen, payload, client).map_err(io_from_qdnf)?;
    write.flush(stream).map_err(io_from_qdnf)?;
    stream.flush()
}

/// Receives a datagram frame; returns `(circuit, gen, payload)`.
pub fn recv_datagram<R: Read>(stream: &mut R) -> std::io::Result<(u32, u32, Vec<u8>)> {
    let mut read = ReadCursor::new();
    let mut env = [0u8; MAX_FRAME];
    let (opcode, n) = read.recv(stream, &mut env, None).map_err(io_from_qdnf)?;
    if opcode != OP_BINARY {
        return Err(io_from_qdnf(QdnfError::Malformed));
    }
    let (typ, c, g, p) = decode_envelope(&env[..n])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if typ != FrameType::Datagram {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not datagram",
        ));
    }
    let mut payload = vec![0u8; p.len()];
    payload.copy_from_slice(p);
    Ok((c, g, payload))
}

/// Sends the HMAC auth frame for `(circuit, gen)`.
pub fn send_auth<W: Write>(
    stream: &mut W,
    secret: &[u8],
    circuit: u32,
    gen: u32,
) -> std::io::Result<()> {
    let tag = hmac_auth(secret, circuit, gen);
    let mut env = [0u8; MAX_FRAME];
    let n = encode_envelope(FrameType::Auth, circuit, gen, &tag, &mut env)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let mut write = WriteCursor::new();
    let mask = fresh_client_mask().map_err(io_from_qdnf)?;
    write
        .seal(OP_BINARY, &env[..n], Some(mask))
        .map_err(io_from_qdnf)?;
    write.flush(stream).map_err(io_from_qdnf)?;
    stream.flush()
}

/// Reads and verifies the peer's auth frame; fails closed on mismatch.
pub fn expect_auth<R: Read>(
    stream: &mut R,
    secret: &[u8],
    circuit: u32,
    gen: u32,
) -> std::io::Result<()> {
    let mut read = ReadCursor::new();
    let mut body = [0u8; MAX_FRAME];
    let (opcode, n) = read
        .recv(stream, &mut body, Some(true))
        .map_err(io_from_qdnf)?;
    if opcode != OP_BINARY {
        return Err(io_from_qdnf(QdnfError::Malformed));
    }
    let (typ, c, g, p) = decode_envelope(&body[..n])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if typ != FrameType::Auth || c != circuit || g != gen {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "auth header",
        ));
    }
    let expect = hmac_auth(secret, circuit, gen);
    if p != expect {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "hmac",
        ));
    }
    Ok(())
}

/// Send opcode-8 Close with envelope `(circuit, gen)` then TCP half-close.
pub fn close_and_shutdown(
    stream: &mut TcpStream,
    circuit: u32,
    gen: u32,
    client: bool,
) -> std::io::Result<()> {
    let mut write = WriteCursor::new();
    seal_close(&mut write, circuit, gen, client).map_err(io_from_qdnf)?;
    write.flush(stream).map_err(io_from_qdnf)?;
    stream.flush()?;
    stream.shutdown(Shutdown::Write)
}

/// Local two-client hub: first two accepted sockets exchange datagrams after HMAC.
pub fn serve_pair(listener: TcpListener, secret: &[u8]) -> std::io::Result<()> {
    listener.set_nonblocking(false)?;
    let (mut a, _) = listener.accept()?;
    let (mut b, _) = listener.accept()?;
    a.set_read_timeout(Some(Duration::from_secs(5)))?;
    b.set_read_timeout(Some(Duration::from_secs(5)))?;
    server_handshake(&mut a)?;
    server_handshake(&mut b)?;
    expect_auth(&mut a, secret, 1, 1)?;
    expect_auth(&mut b, secret, 1, 1)?;
    a.set_read_timeout(Some(Duration::from_millis(400)))?;
    b.set_read_timeout(Some(Duration::from_millis(400)))?;
    let mut ab = false;
    let mut ba = false;
    for _ in 0..8 {
        if !ab {
            if let Ok((c, g, p)) = recv_datagram(&mut a) {
                check_circuit_gen(c, g, 1, 1).map_err(io_from_qdnf)?;
                send_datagram(&mut b, 1, 1, &p, false)?;
                ab = true;
            }
        }
        if !ba {
            if let Ok((c, g, p)) = recv_datagram(&mut b) {
                check_circuit_gen(c, g, 1, 1).map_err(io_from_qdnf)?;
                send_datagram(&mut a, 1, 1, &p, false)?;
                ba = true;
            }
        }
        if ab && ba {
            break;
        }
    }
    Ok(())
}

/// TCP-connects to `addr` then performs client handshake + auth exchange under `secret`.
pub fn dial(addr: std::net::SocketAddr, secret: &[u8]) -> std::io::Result<TcpStream> {
    let mut s = TcpStream::connect(addr)?;
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    let key = STANDARD.encode([3u8; 16]);
    client_handshake(&mut s, &key)?;
    send_auth(&mut s, secret, 1, 1)?;
    Ok(s)
}

/// Set after a successful local two-process WSS exchange.
pub fn local_authenticated_wss_executed() -> bool {
    crate::p2p::connectivity::two_process::local_wss_two_process_executed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Read, Write};

    struct PartialWrite {
        sink: Vec<u8>,
        allow: usize,
    }

    impl Write for PartialWrite {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.allow == 0 {
                return Err(io::Error::new(io::ErrorKind::WouldBlock, "block"));
            }
            let n = buf.len().min(self.allow);
            self.sink.extend_from_slice(&buf[..n]);
            self.allow -= n;
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct PartialRead {
        src: Vec<u8>,
        off: usize,
        allow: usize,
    }

    impl Read for PartialRead {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.allow == 0 {
                return Err(io::Error::new(io::ErrorKind::WouldBlock, "block"));
            }
            if self.off >= self.src.len() {
                return Ok(0);
            }
            let n = buf.len().min(self.src.len() - self.off).min(self.allow);
            buf[..n].copy_from_slice(&self.src[self.off..self.off + n]);
            self.off += n;
            self.allow -= n;
            Ok(n)
        }
    }

    #[test]
    fn accept_key_rfc6455_sample() {
        assert_eq!(
            accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn loopback_ws_datagram() {
        let (mut c, mut s) = loopback_pair().unwrap();
        send_datagram(&mut c, 1, 1, b"ping", true).unwrap();
        let (_, _, p) = recv_datagram(&mut s).unwrap();
        assert_eq!(p, b"ping");
    }

    #[test]
    fn u16_length_keeps_256_and_rejects_oversize() {
        let mut env = [0u8; MAX_FRAME];
        let p256 = [0x5au8; 256];
        let n = encode_envelope(FrameType::Datagram, 1, 1, &p256, &mut env).unwrap();
        let (_, _, _, body) = decode_envelope(&env[..n]).unwrap();
        assert_eq!(body, &p256);
        let too_big = vec![0u8; MAX_PAYLOAD + 1];
        assert_eq!(
            encode_envelope(FrameType::Datagram, 1, 1, &too_big, &mut env),
            Err("capacity")
        );
        let mut legacy = [0u8; 20];
        legacy[0] = 1;
        assert_eq!(decode_envelope(&legacy), Err("obsolete-u8-length"));
        let mut oversize = [0u8; 4];
        oversize[0] = 0x82;
        oversize[1] = 127;
        let mut read = ReadCursor::new();
        let err = read
            .recv(&mut &oversize[..], &mut [0u8; 8], None)
            .unwrap_err();
        assert_eq!(err, QdnfError::Unsupported);
        let mut frag = [0u8; 4];
        frag[0] = 0x02;
        frag[1] = 1;
        let mut read = ReadCursor::new();
        let err = read.recv(&mut &frag[..], &mut [0u8; 8], None).unwrap_err();
        assert_eq!(err, QdnfError::Unsupported);
    }

    #[test]
    fn fresh_mask_differs_across_client_frames() {
        let mut env = [0u8; MAX_FRAME];
        let n = encode_envelope(FrameType::Datagram, 1, 1, b"abc", &mut env).unwrap();
        let mut a = WriteCursor::new();
        let mut b = WriteCursor::new();
        let ma = fresh_client_mask().unwrap();
        let mb = fresh_client_mask().unwrap();
        a.seal(OP_BINARY, &env[..n], Some(ma)).unwrap();
        b.seal(OP_BINARY, &env[..n], Some(mb)).unwrap();
        let ga = a.client_mask().unwrap();
        let gb = b.client_mask().unwrap();
        assert_ne!(ga, gb);
        assert_ne!(ga, [1, 2, 3, 4]);
        assert_ne!(gb, [1, 2, 3, 4]);
        assert_eq!(ga, ma);
        assert_eq!(gb, mb);
        let mut first = Vec::new();
        let mut second = Vec::new();
        let mut m1 = [1u8, 2, 3, 4];
        let mut m2 = [1u8, 2, 3, 4];
        for _ in 0..8 {
            first.clear();
            second.clear();
            send_datagram(&mut first, 1, 1, b"one", true).unwrap();
            send_datagram(&mut second, 1, 1, b"two", true).unwrap();
            assert_eq!(first[1] & 0x80, 0x80);
            assert_eq!(second[1] & 0x80, 0x80);
            m1 = [first[2], first[3], first[4], first[5]];
            m2 = [second[2], second[3], second[4], second[5]];
            if m1 != m2 && m1 != [1, 2, 3, 4] && m2 != [1, 2, 3, 4] {
                break;
            }
        }
        assert_ne!(m1, m2);
        assert_ne!(m1, [1, 2, 3, 4]);
        assert_ne!(m2, [1, 2, 3, 4]);
    }

    #[test]
    fn would_block_write_resumes_same_masked_payload() {
        let mut env = [0u8; MAX_FRAME];
        let n = encode_envelope(FrameType::Datagram, 7, 3, b"retain", &mut env).unwrap();
        let mask = fresh_client_mask().unwrap();
        let mut write = WriteCursor::new();
        write.seal(OP_BINARY, &env[..n], Some(mask)).unwrap();
        let sealed: Vec<u8> = write.sealed_bytes().to_vec();
        assert_eq!(write.client_mask(), Some(mask));
        let mut partial = PartialWrite {
            sink: Vec::new(),
            allow: 3,
        };
        assert_eq!(write.flush(&mut partial), Err(QdnfError::WouldBlock));
        assert!(write.pending());
        assert_eq!(write.sealed_bytes(), sealed.as_slice());
        assert_eq!(write.accepted(), 3);
        write
            .seal(OP_BINARY, &env[..n], Some([1, 2, 3, 4]))
            .unwrap();
        assert_eq!(write.sealed_bytes(), sealed.as_slice());
        assert_eq!(write.client_mask(), Some(mask));
        partial.allow = sealed.len();
        write.flush(&mut partial).unwrap();
        assert!(!write.pending());
        assert_eq!(partial.sink, sealed);
    }

    #[test]
    fn would_block_read_resumes_same_frame() {
        let mut env = [0u8; MAX_FRAME];
        let n = encode_envelope(FrameType::Datagram, 1, 1, b"split-read", &mut env).unwrap();
        let mut write = WriteCursor::new();
        let mask = fresh_client_mask().unwrap();
        write.seal(OP_BINARY, &env[..n], Some(mask)).unwrap();
        let sealed: Vec<u8> = write.sealed_bytes().to_vec();
        let mut src = PartialRead {
            src: sealed,
            off: 0,
            allow: 2,
        };
        let mut read = ReadCursor::new();
        let mut out = [0u8; MAX_FRAME];
        assert_eq!(
            read.recv(&mut src, &mut out, Some(true)),
            Err(QdnfError::WouldBlock)
        );
        assert!(read.pending());
        src.allow = 3;
        assert_eq!(
            read.recv(&mut src, &mut out, Some(true)),
            Err(QdnfError::WouldBlock)
        );
        src.allow = usize::MAX;
        let (opcode, got) = read.recv(&mut src, &mut out, Some(true)).unwrap();
        assert_eq!(opcode, OP_BINARY);
        let (typ, c, g, p) = decode_envelope(&out[..got]).unwrap();
        assert_eq!(typ, FrameType::Datagram);
        assert_eq!((c, g), (1, 1));
        assert_eq!(p, b"split-read");
        assert!(!read.pending());
    }

    #[test]
    fn pending_datagram_then_seal_close_is_conflict() {
        let mut env = [0u8; MAX_FRAME];
        let n = encode_envelope(FrameType::Datagram, 1, 1, b"live", &mut env).unwrap();
        let mut write = WriteCursor::new();
        write
            .seal(OP_BINARY, &env[..n], Some(fresh_client_mask().unwrap()))
            .unwrap();
        let sealed: Vec<u8> = write.sealed_bytes().to_vec();
        let mut partial = PartialWrite {
            sink: Vec::new(),
            allow: 2,
        };
        assert_eq!(write.flush(&mut partial), Err(QdnfError::WouldBlock));
        let off = write.accepted();
        assert_eq!(seal_close(&mut write, 1, 1, true), Err(QdnfError::Conflict));
        assert_eq!(write.sealed_bytes(), sealed.as_slice());
        assert_eq!(write.accepted(), off);
        assert_eq!(write.sealed_opcode(), Some(OP_BINARY));
    }

    #[test]
    fn close_frame_then_shutdown() {
        let mut write = WriteCursor::new();
        seal_close(&mut write, 1, 1, true).unwrap();
        assert_eq!(write.sealed_opcode(), Some(OP_CLOSE));
        let sealed: Vec<u8> = write.sealed_bytes().to_vec();
        assert_eq!(sealed[0], 0x88);
        assert_eq!(sealed[1] & 0x80, 0x80);
        let mut partial = PartialWrite {
            sink: Vec::new(),
            allow: 1,
        };
        assert_eq!(write.flush(&mut partial), Err(QdnfError::WouldBlock));
        seal_close(&mut write, 1, 1, true).unwrap();
        assert_eq!(write.sealed_bytes(), sealed.as_slice());
        partial.allow = sealed.len();
        write.flush(&mut partial).unwrap();
        assert_eq!(partial.sink, sealed);

        let (mut c, mut s) = loopback_pair().unwrap();
        close_and_shutdown(&mut c, 1, 1, true).unwrap();
        let mut read = ReadCursor::new();
        let mut out = [0u8; MAX_FRAME];
        let (opcode, got) = read.recv(&mut s, &mut out, Some(true)).unwrap();
        assert_eq!(opcode, OP_CLOSE);
        let (typ, circ, gen, p) = decode_envelope(&out[..got]).unwrap();
        assert_eq!(typ, FrameType::Close);
        assert_eq!((circ, gen), (1, 1));
        assert!(p.is_empty());
        let mut tail = [0u8; 1];
        assert_eq!(s.read(&mut tail).unwrap(), 0);
        assert!(c.write(&[1]).is_err());
    }

    #[test]
    fn stale_generation_or_wrong_circuit_refused() {
        let (mut c, mut s) = loopback_pair().unwrap();
        send_datagram(&mut c, 1, 2, b"stale", true).unwrap();
        let mut read = ReadCursor::new();
        let mut out = [0u8; MAX_FRAME];
        let err = recv_checked(&mut s, &mut read, 1, 1, Some(true), &mut out).unwrap_err();
        assert_eq!(err, QdnfError::StaleGeneration);
        send_datagram(&mut c, 9, 1, b"wrong", true).unwrap();
        let mut read = ReadCursor::new();
        let err = recv_checked(&mut s, &mut read, 1, 1, Some(true), &mut out).unwrap_err();
        assert_eq!(err, QdnfError::Unauthorized);
        send_datagram(&mut c, 1, 1, b"ok", true).unwrap();
        let mut read = ReadCursor::new();
        let (typ, n) = recv_checked(&mut s, &mut read, 1, 1, Some(true), &mut out).unwrap();
        assert_eq!(typ, FrameType::Datagram);
        assert_eq!(&out[..n], b"ok");
        assert_eq!(
            check_circuit_gen(1, 0, 1, 1),
            Err(QdnfError::StaleGeneration)
        );
        assert_eq!(check_circuit_gen(2, 1, 1, 1), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn server_frames_unmasked() {
        let mut wire = Vec::new();
        send_datagram(&mut wire, 1, 1, b"srv", false).unwrap();
        assert_eq!(wire[0], 0x82);
        assert_eq!(wire[1] & 0x80, 0);
        let mut env = [0u8; MAX_FRAME];
        let n = encode_envelope(FrameType::Datagram, 1, 1, b"srv", &mut env).unwrap();
        let mut write = WriteCursor::new();
        write.seal(OP_BINARY, &env[..n], None).unwrap();
        assert!(write.client_mask().is_none());
        let mut sealed = write.sealed_bytes();
        assert_eq!(sealed[1] & 0x80, 0);
        let hdr = if n < 126 { 2 } else { 4 };
        assert_eq!(&sealed[hdr..], &env[..n]);
        let mut read = ReadCursor::new();
        let mut out = [0u8; MAX_FRAME];
        let (opcode, got) = read.recv(&mut sealed, &mut out, Some(false)).unwrap();
        assert_eq!(opcode, OP_BINARY);
        let (_, _, _, p) = decode_envelope(&out[..got]).unwrap();
        assert_eq!(p, b"srv");
        let mut masked = WriteCursor::new();
        masked
            .seal(OP_BINARY, &env[..n], Some([9, 8, 7, 6]))
            .unwrap();
        let mut masked_bytes = masked.sealed_bytes();
        let mut read = ReadCursor::new();
        let err = read
            .recv(&mut masked_bytes, &mut out, Some(false))
            .unwrap_err();
        assert_eq!(err, QdnfError::Malformed);
    }
}
