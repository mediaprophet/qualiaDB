use super::*;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

const RX_CAP: usize = WIRE_CAP * 2;

struct ScriptedStream {
    rx: [u8; RX_CAP],
    rx_len: usize,
    rx_off: usize,
    tx: [u8; RX_CAP],
    tx_len: usize,
    write_quota: usize,
    read_quota: usize,
    write_blocked: bool,
    read_blocked: bool,
    block_after_write: bool,
    block_after_read: bool,
    eof: bool,
    shut: bool,
}

impl ScriptedStream {
    fn new() -> Self {
        Self {
            rx: [0u8; RX_CAP],
            rx_len: 0,
            rx_off: 0,
            tx: [0u8; RX_CAP],
            tx_len: 0,
            write_quota: WIRE_CAP,
            read_quota: WIRE_CAP,
            write_blocked: false,
            read_blocked: false,
            block_after_write: false,
            block_after_read: false,
            eof: false,
            shut: false,
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        let n = bytes.len();
        self.rx[self.rx_len..self.rx_len + n].copy_from_slice(bytes);
        self.rx_len += n;
    }

    fn splice_tx_to_rx(&mut self) {
        let n = self.tx_len;
        self.rx[self.rx_len..self.rx_len + n].copy_from_slice(&self.tx[..n]);
        self.rx_len += n;
        self.tx_len = 0;
    }

    fn clear_write_block(&mut self) {
        self.write_blocked = false;
    }

    fn clear_read_block(&mut self) {
        self.read_blocked = false;
    }
}

impl StreamBytes for ScriptedStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, QdnfError> {
        if self.shut {
            return Err(QdnfError::Closed);
        }
        if self.write_blocked {
            return Err(QdnfError::WouldBlock);
        }
        let space = self.tx.len() - self.tx_len;
        let n = buf.len().min(self.write_quota).min(space);
        if n == 0 {
            return Err(QdnfError::WouldBlock);
        }
        self.tx[self.tx_len..self.tx_len + n].copy_from_slice(&buf[..n]);
        self.tx_len += n;
        if self.block_after_write {
            self.write_blocked = true;
        }
        Ok(n)
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, QdnfError> {
        if self.shut {
            return Err(QdnfError::Closed);
        }
        if self.read_blocked {
            return Err(QdnfError::WouldBlock);
        }
        let avail = self.rx_len - self.rx_off;
        if avail == 0 {
            return if self.eof {
                Ok(0)
            } else {
                Err(QdnfError::WouldBlock)
            };
        }
        let n = buf.len().min(self.read_quota).min(avail);
        if n == 0 {
            return Err(QdnfError::WouldBlock);
        }
        buf[..n].copy_from_slice(&self.rx[self.rx_off..self.rx_off + n]);
        self.rx_off += n;
        if self.block_after_read {
            self.read_blocked = true;
        }
        Ok(n)
    }

    fn shutdown(&mut self) -> Result<(), QdnfError> {
        self.shut = true;
        Ok(())
    }
}

fn loc(b: u8) -> ObservedLocator {
    ObservedLocator::from_slice(&[b]).unwrap()
}

fn scope() -> ScopeEpoch {
    ScopeEpoch { scope: 1, epoch: 1 }
}

fn connect(io: ScriptedStream) -> FramedStream<ScriptedStream> {
    FramedStream::connect(io, loc(0x01), loc(0x02), scope(), 1280).unwrap()
}

fn encode_into(payload: &[u8]) -> ([u8; WIRE_CAP], usize) {
    let mut wire = [0u8; WIRE_CAP];
    let n = encode_stream_frame(payload, &mut wire).unwrap();
    (wire, n)
}

#[test]
fn split_writes_retain_sealed_pending_across_partial_accept() {
    let mut io = ScriptedStream::new();
    io.write_quota = 3;
    io.block_after_write = true;
    let mut s = connect(io);
    let payload = [0xAAu8, 0xBB, 0xCC, 0xDD, 0xEE];
    let (wire, wire_len) = encode_into(&payload);
    let dest = loc(0x02);
    assert_eq!(s.send(&dest, &payload), Err(QdnfError::WouldBlock));
    assert!(s.write_cursor().pending());
    assert_eq!(s.write_cursor().accepted(), 3);
    assert_eq!(s.write_cursor().sealed_bytes(), &wire[..wire_len]);
    s.inner_mut().clear_write_block();
    assert_eq!(s.send(&dest, &payload), Err(QdnfError::WouldBlock));
    assert_eq!(s.write_cursor().accepted(), 6);
    assert_eq!(s.write_cursor().sealed_bytes(), &wire[..wire_len]);
    s.inner_mut().clear_write_block();
    assert_eq!(s.send(&dest, &payload), Ok(payload.len()));
    assert!(!s.write_cursor().pending());
    assert_eq!(&s.inner().tx[..wire_len], &wire[..wire_len]);
}

#[test]
fn split_reads_assemble_one_length_prefixed_frame() {
    let payload = [1u8, 2, 3, 4, 5, 6, 7];
    let (wire, wire_len) = encode_into(&payload);
    let mut io = ScriptedStream::new();
    io.feed(&wire[..wire_len]);
    io.read_quota = 2;
    io.block_after_read = true;
    let mut s = connect(io);
    let mut out = [0u8; 16];
    let mut n = 0usize;
    let mut meta = None;
    for _ in 0..8 {
        match s.recv(&mut out) {
            Err(QdnfError::WouldBlock) => {
                assert!(s.read_cursor().pending());
                s.inner_mut().clear_read_block();
            }
            Ok(got) => {
                n = got.0;
                meta = Some(got.1);
                break;
            }
            Err(e) => panic!("split read: {e:?}"),
        }
    }
    let meta = meta.expect("frame assembled after split reads");
    assert_eq!(n, payload.len());
    assert_eq!(&out[..n], &payload);
    assert_eq!(meta.observed_source, loc(0x02));
    assert_eq!(meta.scope, scope());
    assert!(!s.read_cursor().pending());
}

#[test]
fn would_block_then_resume_does_not_reseal_or_drop_cursor() {
    let mut io = ScriptedStream::new();
    io.write_quota = 4;
    io.block_after_write = true;
    let mut s = connect(io);
    let payload = [9u8, 8, 7, 6, 5, 4, 3, 2];
    let (wire, wire_len) = encode_into(&payload);
    let dest = loc(0x02);
    assert_eq!(s.send(&dest, &payload), Err(QdnfError::WouldBlock));
    let sealed = {
        let mut tmp = [0u8; WIRE_CAP];
        let n = s.write_cursor().sealed_len();
        tmp[..n].copy_from_slice(s.write_cursor().sealed_bytes());
        (tmp, n)
    };
    assert_eq!(&sealed.0[..sealed.1], &wire[..wire_len]);
    assert_eq!(s.write_cursor().accepted(), 4);
    assert_eq!(s.send(&dest, &[0xFF, 0xEE]), Err(QdnfError::Conflict));
    assert_eq!(s.write_cursor().accepted(), 4);
    assert_eq!(s.write_cursor().sealed_bytes(), &sealed.0[..sealed.1]);
    s.inner_mut().clear_write_block();
    assert_eq!(s.send(&dest, &payload), Err(QdnfError::WouldBlock));
    assert_eq!(s.write_cursor().accepted(), 8);
    assert_eq!(s.write_cursor().sealed_bytes(), &sealed.0[..sealed.1]);
    s.inner_mut().clear_write_block();
    assert_eq!(s.send(&dest, &payload), Ok(payload.len()));
    assert!(!s.write_cursor().pending());
}

#[test]
fn oversize_payload_and_declared_length_are_capacity() {
    assert_eq!(FRAME_CAP, 8192);
    let mut huge = [0u8; FRAME_CAP + 1];
    huge[0] = 1;
    let mut out = [0u8; WIRE_CAP];
    assert_eq!(
        encode_stream_frame(&huge, &mut out),
        Err(QdnfError::Capacity)
    );

    let mut prefix = [0u8; LEN_SIZE];
    prefix.copy_from_slice(&((FRAME_CAP as u32) + 1).to_be_bytes());
    assert_eq!(decode_stream_frame(&prefix), Err(QdnfError::Capacity));
    assert_eq!(next_stream_frame(&prefix), Err(QdnfError::Capacity));

    let mut io = ScriptedStream::new();
    io.feed(&prefix);
    let mut s = connect(io);
    let mut recv_out = [0u8; 8];
    assert_eq!(s.recv(&mut recv_out), Err(QdnfError::Capacity));
    assert!(s.read_cursor().refused());
    assert_eq!(s.recv(&mut recv_out), Err(QdnfError::Capacity));

    let dest = loc(0x02);
    let mut s2 = FramedStream::connect(
        ScriptedStream::new(),
        loc(0x01),
        loc(0x02),
        scope(),
        FRAME_CAP as u16,
    )
    .unwrap();
    assert_eq!(s2.send(&dest, &huge), Err(QdnfError::Capacity));
    assert!(!s2.write_cursor().pending());
    assert_eq!(
        FramedStream::connect(
            ScriptedStream::new(),
            loc(0x01),
            loc(0x02),
            scope(),
            FRAME_CAP as u16 + 1
        )
        .err(),
        Some(QdnfError::Capacity)
    );
}

#[test]
fn drain_after_shutdown_is_closed() {
    let payload = [0x11u8, 0x22];
    let (wire, wire_len) = encode_into(&payload);
    let mut io = ScriptedStream::new();
    io.feed(&wire[..wire_len]);
    let mut s = connect(io);
    s.begin_drain().unwrap();
    let dest = loc(0x02);
    assert_eq!(s.send(&dest, &payload), Err(QdnfError::Closed));
    let mut out = [0u8; 8];
    let (n, meta) = s.recv(&mut out).unwrap();
    assert_eq!(&out[..n], &payload);
    assert_eq!(meta.observed_source, loc(0x02));
    s.shutdown().unwrap();
    assert_eq!(s.begin_drain(), Err(QdnfError::Closed));
    assert_eq!(s.drain_write(), Err(QdnfError::Closed));
    assert_eq!(s.send(&dest, &payload), Err(QdnfError::Closed));
    assert_eq!(s.recv(&mut out), Err(QdnfError::Closed));
    assert!(!s.write_cursor().pending());
    assert!(!s.read_cursor().pending());
}

#[test]
fn encode_is_exact_length_binary_not_json() {
    let payload = br#"{"k":1,"p":"AAAA"}"#;
    let mut wire = [0u8; 64];
    let n = encode_stream_frame(payload, &mut wire).unwrap();
    assert_eq!(n, LEN_SIZE + payload.len());
    assert_eq!(&wire[..LEN_SIZE], &(payload.len() as u32).to_be_bytes());
    assert_eq!(&wire[LEN_SIZE..n], payload);
    assert_ne!(wire[0], b'{');
    assert_eq!(decode_stream_frame(&wire[..n]).unwrap(), payload.as_ref());

    let json = br#"{"len":4,"payload":"AAAA"}"#;
    match decode_stream_frame(json) {
        Err(QdnfError::Capacity) | Err(QdnfError::Malformed) | Err(QdnfError::Truncated) => {}
        other => panic!("JSON object must not decode as a frame: {other:?}"),
    }
}

#[test]
fn truncated_and_trailing_refuse_without_payload_view() {
    assert_eq!(decode_stream_frame(&[0u8; 3]), Err(QdnfError::Truncated));
    let payload = [1u8, 2, 3, 4];
    let (wire, wire_len) = encode_into(&payload);
    assert_eq!(
        decode_stream_frame(&wire[..wire_len - 1]),
        Err(QdnfError::Truncated)
    );
    let mut trailing = [0u8; 16];
    trailing[..wire_len].copy_from_slice(&wire[..wire_len]);
    trailing[wire_len] = 0xFF;
    assert_eq!(
        decode_stream_frame(&trailing[..wire_len + 1]),
        Err(QdnfError::Malformed)
    );
    let (got, consumed) = next_stream_frame(&trailing[..wire_len + 1]).unwrap();
    assert_eq!(got, payload.as_ref());
    assert_eq!(consumed, wire_len);

    let mut io = ScriptedStream::new();
    io.feed(&wire[..3]);
    io.eof = true;
    let mut s = connect(io);
    let mut out = [0u8; 8];
    assert_eq!(s.recv(&mut out), Err(QdnfError::Truncated));
}

#[test]
fn short_recv_buffer_does_not_drop_completed_frame() {
    let payload = [0xAAu8, 0xBB, 0xCC];
    let (wire, wire_len) = encode_into(&payload);
    let mut io = ScriptedStream::new();
    io.feed(&wire[..wire_len]);
    let mut s = connect(io);
    let mut tiny = [0u8; 1];
    assert_eq!(s.recv(&mut tiny), Err(QdnfError::Capacity));
    assert!(s.read_cursor().pending());
    let mut out = [0u8; 8];
    let (n, meta) = s.recv(&mut out).unwrap();
    assert_eq!(n, payload.len());
    assert_eq!(&out[..n], &payload);
    assert_eq!(meta.observed_source.as_slice(), &[0x02]);
}

#[test]
fn framed_stream_reuses_local_ipc_profile_and_observed_locator() {
    let mut io = ScriptedStream::new();
    io.block_after_write = false;
    let mut s = connect(io);
    assert_eq!(s.profile(), BearerProfile::LocalIpcV1);
    assert!(s.profile().native_independent());
    assert_eq!(s.capabilities().profile, BearerProfile::LocalIpcV1);
    assert!(s.capabilities().ordered);
    assert!(!s.capabilities().group_delivery);
    assert!(!s.capabilities().may_duplicate);
    let payload = [0x42u8; 4];
    s.send(&loc(0x02), &payload).unwrap();
    s.inner_mut().splice_tx_to_rx();
    let mut out = [0u8; 8];
    let (n, meta) = s.recv(&mut out).unwrap();
    assert_eq!(&out[..n], &payload);
    assert_eq!(meta.observed_source, loc(0x02));
    assert_eq!(s.locator().as_slice(), &[0x01]);
}
