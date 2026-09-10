//! Authenticated WebSocket binary-datagram relay (RFC 6455). Local TCP; not a public URL.

#![cfg(not(target_arch = "wasm32"))]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};
use sha1::{Digest, Sha1};
use sha2::Sha256;

const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const MAX_FRAME: usize = 2048;
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

pub fn accept_key(sec_key: &str) -> String {
    let mut h = Sha1::new();
    h.update(sec_key.as_bytes());
    h.update(GUID);
    STANDARD.encode(h.finalize())
}

pub fn hmac_auth(secret: &[u8], circuit: u32, gen: u32) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(secret).expect("hmac key");
    mac.update(&circuit.to_be_bytes());
    mac.update(&gen.to_be_bytes());
    let out = mac.finalize().into_bytes();
    let mut a = [0u8; 32];
    a.copy_from_slice(&out);
    a
}

pub fn encode_envelope(
    typ: FrameType,
    circuit: u32,
    gen: u32,
    payload: &[u8],
    out: &mut [u8],
) -> Result<usize, &'static str> {
    let n = 11 + payload.len();
    if n > MAX_FRAME || out.len() < n {
        return Err("capacity");
    }
    out[0] = 1;
    out[1] = typ as u8;
    out[2..6].copy_from_slice(&circuit.to_be_bytes());
    out[6..10].copy_from_slice(&gen.to_be_bytes());
    out[10] = payload.len() as u8;
    out[11..n].copy_from_slice(payload);
    Ok(n)
}

pub fn decode_envelope(buf: &[u8]) -> Result<(FrameType, u32, u32, &[u8]), &'static str> {
    if buf.len() < 11 || buf[0] != 1 {
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
    let len = buf[10] as usize;
    if buf.len() < 11 + len {
        return Err("truncated");
    }
    Ok((typ, circuit, gen, &buf[11..11 + len]))
}

fn write_ws_binary<W: Write>(
    w: &mut W,
    payload: &[u8],
    mask: Option<[u8; 4]>,
) -> std::io::Result<()> {
    if payload.len() > 0xffff {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "oversize",
        ));
    }
    let mut hdr = [0u8; 8];
    hdr[0] = 0x82;
    let mut n = 2;
    let mut masked = payload.to_vec();
    if let Some(m) = mask {
        hdr[1] = 0x80;
        if payload.len() < 126 {
            hdr[1] |= payload.len() as u8;
        } else {
            hdr[1] |= 126;
            hdr[2..4].copy_from_slice(&(payload.len() as u16).to_be_bytes());
            n = 4;
        }
        hdr[n..n + 4].copy_from_slice(&m);
        n += 4;
        for (i, b) in masked.iter_mut().enumerate() {
            *b ^= m[i % 4];
        }
    } else if payload.len() < 126 {
        hdr[1] = payload.len() as u8;
    } else {
        hdr[1] = 126;
        hdr[2..4].copy_from_slice(&(payload.len() as u16).to_be_bytes());
        n = 4;
    }
    w.write_all(&hdr[..n])?;
    w.write_all(&masked)?;
    w.flush()
}

fn read_ws_binary<R: Read>(r: &mut R) -> std::io::Result<Vec<u8>> {
    let mut h2 = [0u8; 2];
    r.read_exact(&mut h2)?;
    let mut len = (h2[1] & 0x7f) as usize;
    let masked = h2[1] & 0x80 != 0;
    if len == 126 {
        let mut ext = [0u8; 2];
        r.read_exact(&mut ext)?;
        len = u16::from_be_bytes(ext) as usize;
    } else if len == 127 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "8-byte len",
        ));
    }
    if len > MAX_FRAME {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "oversize",
        ));
    }
    let mut mask = [0u8; 4];
    if masked {
        r.read_exact(&mut mask)?;
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    if masked {
        for (i, b) in body.iter_mut().enumerate() {
            *b ^= mask[i % 4];
        }
    }
    Ok(body)
}

fn client_handshake(stream: &mut TcpStream, key_b64: &str) -> std::io::Result<()> {
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

fn server_handshake(stream: &mut TcpStream) -> std::io::Result<()> {
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

pub fn send_datagram(
    stream: &mut TcpStream,
    circuit: u32,
    gen: u32,
    payload: &[u8],
    client: bool,
) -> std::io::Result<()> {
    let mut env = [0u8; MAX_FRAME];
    let n = encode_envelope(FrameType::Datagram, circuit, gen, payload, &mut env)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let mask = if client { Some([1, 2, 3, 4]) } else { None };
    write_ws_binary(stream, &env[..n], mask)
}

pub fn recv_datagram(stream: &mut TcpStream) -> std::io::Result<(u32, u32, Vec<u8>)> {
    let body = read_ws_binary(stream)?;
    let (typ, c, g, p) = decode_envelope(&body)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if typ != FrameType::Datagram {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not datagram",
        ));
    }
    Ok((c, g, p.to_vec()))
}

pub fn send_auth(
    stream: &mut TcpStream,
    secret: &[u8],
    circuit: u32,
    gen: u32,
) -> std::io::Result<()> {
    let tag = hmac_auth(secret, circuit, gen);
    let mut env = [0u8; MAX_FRAME];
    let n = encode_envelope(FrameType::Auth, circuit, gen, &tag, &mut env)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    write_ws_binary(stream, &env[..n], Some([9, 8, 7, 6]))
}

pub fn expect_auth(
    stream: &mut TcpStream,
    secret: &[u8],
    circuit: u32,
    gen: u32,
) -> std::io::Result<()> {
    let body = read_ws_binary(stream)?;
    let (typ, c, g, p) = decode_envelope(&body)
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
            if let Ok((_, _, p)) = recv_datagram(&mut a) {
                send_datagram(&mut b, 1, 1, &p, false)?;
                ab = true;
            }
        }
        if !ba {
            if let Ok((_, _, p)) = recv_datagram(&mut b) {
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
}
