//! RFC 6455 binary/close framing with retained WouldBlock cursors.
//!
//! Client frames take a fresh 4-byte mask from `fill_os` per seal. Server
//! frames stay unmasked. WouldBlock retains the sealed wire bytes / recv
//! buffer; the mask is never regenerated mid-frame.

#![cfg(not(target_arch = "wasm32"))]

use std::io::{ErrorKind, Read, Write};

use crate::crypto::network::entropy::fill_os;
use crate::net::qdnf::errors::QdnfError;

use super::{ENVELOPE_HDR, MAX_FRAME};

/// RFC 6455 binary opcode. FIN is set at encode.
pub const OP_BINARY: u8 = 0x2;
/// RFC 6455 close opcode. FIN is set at encode.
pub const OP_CLOSE: u8 = 0x8;
/// 2-byte base + 2-byte ext-len + 4-byte mask.
pub const WS_HDR_MAX: usize = 8;
/// Sealed WS frame capacity (`WS_HDR_MAX + MAX_FRAME`).
pub const WIRE_CAP: usize = WS_HDR_MAX + MAX_FRAME;

const _: () = assert!(MAX_FRAME == 2048);
const _: () = assert!(ENVELOPE_HDR == 12);
const _: () = assert!(WIRE_CAP == 2056);

/// Fill a 4-byte client mask from OS entropy. Fail closed on entropy failure.
pub fn fresh_client_mask() -> Result<[u8; 4], QdnfError> {
    let mut mask = [0u8; 4];
    fill_os(&mut mask).map_err(|_| QdnfError::EntropyFailure)?;
    Ok(mask)
}

/// Map `std::io::Error` onto a QDNF outcome. Recovers a wrapped `QdnfError`.
pub fn map_io(err: std::io::Error) -> QdnfError {
    if let Some(&inner) = err.get_ref().and_then(|e| e.downcast_ref::<QdnfError>()) {
        return inner;
    }
    match err.kind() {
        ErrorKind::WouldBlock | ErrorKind::Interrupted => QdnfError::WouldBlock,
        ErrorKind::UnexpectedEof
        | ErrorKind::BrokenPipe
        | ErrorKind::ConnectionReset
        | ErrorKind::ConnectionAborted
        | ErrorKind::NotConnected => QdnfError::Closed,
        ErrorKind::InvalidData => QdnfError::Malformed,
        ErrorKind::InvalidInput => QdnfError::Capacity,
        ErrorKind::AlreadyExists => QdnfError::Conflict,
        ErrorKind::PermissionDenied => QdnfError::Denied,
        ErrorKind::TimedOut => QdnfError::Expired,
        _ => QdnfError::Closed,
    }
}

/// Wrap a QDNF outcome as `io::Error` so existing `send_datagram` callers keep
/// `io::Result`. Bearer paths downcast via [`map_io`].
pub fn io_from_qdnf(err: QdnfError) -> std::io::Error {
    let kind = match err {
        QdnfError::WouldBlock => ErrorKind::WouldBlock,
        QdnfError::Closed => ErrorKind::ConnectionAborted,
        QdnfError::Capacity | QdnfError::Range => ErrorKind::InvalidInput,
        QdnfError::Malformed
        | QdnfError::Truncated
        | QdnfError::Unsupported
        | QdnfError::StaleGeneration
        | QdnfError::Unauthorized => ErrorKind::InvalidData,
        QdnfError::EntropyFailure => ErrorKind::Other,
        QdnfError::Conflict => ErrorKind::AlreadyExists,
        QdnfError::Denied => ErrorKind::PermissionDenied,
        _ => ErrorKind::Other,
    };
    std::io::Error::new(kind, err)
}

struct WsHeader {
    opcode: u8,
    masked: bool,
    mask: [u8; 4],
    hdr_len: usize,
    payload_len: usize,
}

/// Header size from the first two bytes. Refuses RSV, FIN=0, 8-byte length,
/// and opcodes other than binary/close.
fn header_need(b0: u8, b1: u8) -> Result<usize, QdnfError> {
    if b0 & 0x70 != 0 {
        return Err(QdnfError::Malformed);
    }
    if b0 & 0x80 == 0 {
        return Err(QdnfError::Unsupported);
    }
    let opcode = b0 & 0x0f;
    if opcode != OP_BINARY && opcode != OP_CLOSE {
        return Err(QdnfError::Unsupported);
    }
    let len7 = b1 & 0x7f;
    if len7 == 127 {
        return Err(QdnfError::Unsupported);
    }
    let mut n = 2usize;
    if len7 == 126 {
        n += 2;
    }
    if b1 & 0x80 != 0 {
        n += 4;
    }
    Ok(n)
}

fn parse_header(buf: &[u8]) -> Result<WsHeader, QdnfError> {
    if buf.len() < 2 {
        return Err(QdnfError::Truncated);
    }
    let hdr_len = header_need(buf[0], buf[1])?;
    if buf.len() < hdr_len {
        return Err(QdnfError::Truncated);
    }
    let len7 = buf[1] & 0x7f;
    let payload_len = if len7 == 126 {
        u16::from_be_bytes([buf[2], buf[3]]) as usize
    } else {
        len7 as usize
    };
    if payload_len > MAX_FRAME {
        return Err(QdnfError::Capacity);
    }
    let masked = buf[1] & 0x80 != 0;
    let mut mask = [0u8; 4];
    if masked {
        let off = hdr_len - 4;
        mask.copy_from_slice(&buf[off..hdr_len]);
    }
    Ok(WsHeader {
        opcode: buf[0] & 0x0f,
        masked,
        mask,
        hdr_len,
        payload_len,
    })
}

/// Encode one FIN binary/close frame into `out`. Never uses 8-byte length.
pub fn encode_ws_frame(
    out: &mut [u8],
    opcode: u8,
    payload: &[u8],
    mask: Option<[u8; 4]>,
) -> Result<usize, QdnfError> {
    if opcode != OP_BINARY && opcode != OP_CLOSE {
        return Err(QdnfError::Unsupported);
    }
    if payload.len() > MAX_FRAME || payload.len() > 0xffff {
        return Err(QdnfError::Capacity);
    }
    let mut n = 2usize;
    if payload.len() >= 126 {
        n += 2;
    }
    if mask.is_some() {
        n += 4;
    }
    let total = n.checked_add(payload.len()).ok_or(QdnfError::Range)?;
    if out.len() < total {
        return Err(QdnfError::Capacity);
    }
    out[0] = 0x80 | opcode;
    if payload.len() < 126 {
        out[1] = payload.len() as u8;
    } else {
        out[1] = 126;
        out[2..4].copy_from_slice(&(payload.len() as u16).to_be_bytes());
    }
    if let Some(m) = mask {
        out[1] |= 0x80;
        let mask_off = n - 4;
        out[mask_off..n].copy_from_slice(&m);
        for (i, b) in payload.iter().enumerate() {
            out[n + i] = b ^ m[i % 4];
        }
    } else if !payload.is_empty() {
        out[n..total].copy_from_slice(payload);
    }
    Ok(total)
}

/// Sealed pending write. WouldBlock retains exact wire bytes; `off` advances
/// only on `Ok(n > 0)`. A different payload while pending is Conflict.
pub struct WriteCursor {
    buf: [u8; WIRE_CAP],
    len: u16,
    off: u16,
}

impl WriteCursor {
    /// Empty cursor.
    pub const fn new() -> Self {
        Self {
            buf: [0u8; WIRE_CAP],
            len: 0,
            off: 0,
        }
    }

    /// `true` while a sealed frame is not fully accepted.
    #[inline]
    pub const fn pending(&self) -> bool {
        self.len != 0
    }

    /// Bytes the sink has accepted.
    #[inline]
    pub const fn accepted(&self) -> usize {
        self.off as usize
    }

    /// Sealed wire length, including the WS header and mask.
    #[inline]
    pub const fn sealed_len(&self) -> usize {
        self.len as usize
    }

    /// Exact sealed bytes. Empty when idle.
    #[inline]
    pub fn sealed_bytes(&self) -> &[u8] {
        &self.buf[..self.len as usize]
    }

    /// Opcode of the sealed frame.
    #[inline]
    pub fn sealed_opcode(&self) -> Option<u8> {
        if self.len == 0 {
            None
        } else {
            Some(self.buf[0] & 0x0f)
        }
    }

    /// Mask bytes of a sealed client frame.
    pub fn client_mask(&self) -> Option<[u8; 4]> {
        if self.len < 2 || self.buf[1] & 0x80 == 0 {
            return None;
        }
        parse_header(&self.buf[..self.len as usize])
            .ok()
            .map(|h| h.mask)
    }

    /// Seal `payload` as a FIN frame. Same opcode+payload while pending
    /// resumes (mask argument is ignored). A different payload is Conflict
    /// and does not reseal or rewind `off`.
    pub fn seal(
        &mut self,
        opcode: u8,
        payload: &[u8],
        mask: Option<[u8; 4]>,
    ) -> Result<(), QdnfError> {
        if self.len != 0 {
            if self.sealed_opcode() == Some(opcode) && self.unmasked_payload_eq(payload) {
                return Ok(());
            }
            return Err(QdnfError::Conflict);
        }
        let n = encode_ws_frame(&mut self.buf, opcode, payload, mask)?;
        self.len = n as u16;
        self.off = 0;
        Ok(())
    }

    /// Write remaining sealed bytes. Advances `off` only on `Ok(n > 0)`.
    pub fn flush<W: Write + ?Sized>(&mut self, sink: &mut W) -> Result<(), QdnfError> {
        if self.len == 0 {
            return Ok(());
        }
        while (self.off as usize) < (self.len as usize) {
            let remaining = &self.buf[self.off as usize..self.len as usize];
            match sink.write(remaining) {
                Ok(0) => return Err(QdnfError::WouldBlock),
                Ok(n) => {
                    if n > remaining.len() {
                        return Err(QdnfError::Range);
                    }
                    self.off += n as u16;
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    return Err(QdnfError::WouldBlock);
                }
                Err(e) => return Err(map_io(e)),
            }
        }
        self.abort();
        Ok(())
    }

    /// Drop sealed bytes without sending.
    pub fn abort(&mut self) {
        self.len = 0;
        self.off = 0;
    }

    fn unmasked_payload_eq(&self, payload: &[u8]) -> bool {
        let hdr = match parse_header(&self.buf[..self.len as usize]) {
            Ok(h) => h,
            Err(_) => return false,
        };
        let start = hdr.hdr_len;
        let end = self.len as usize;
        if end < start || end - start != payload.len() {
            return false;
        }
        let body = &self.buf[start..end];
        if hdr.masked {
            body.iter()
                .enumerate()
                .all(|(i, b)| *b ^ hdr.mask[i % 4] == payload[i])
        } else {
            body == payload
        }
    }
}

impl Default for WriteCursor {
    fn default() -> Self {
        Self::new()
    }
}

/// Partial-read assembly. Oversize / 8-byte / fragmented poisons the cursor.
/// A short `out` after a complete frame is Capacity and does not drop it.
pub struct ReadCursor {
    buf: [u8; WIRE_CAP],
    filled: u16,
    expected: u16,
    hdr: u8,
    opcode: u8,
    complete: bool,
    unmasked: bool,
    refused: bool,
}

impl ReadCursor {
    /// Empty cursor.
    pub const fn new() -> Self {
        Self {
            buf: [0u8; WIRE_CAP],
            filled: 0,
            expected: 0,
            hdr: 0,
            opcode: 0,
            complete: false,
            unmasked: false,
            refused: false,
        }
    }

    /// Bytes buffered toward the current frame.
    #[inline]
    pub const fn filled(&self) -> usize {
        self.filled as usize
    }

    /// `true` while a header or body is incomplete, or a complete frame is
    /// waiting to be taken.
    #[inline]
    pub const fn pending(&self) -> bool {
        self.filled != 0 || self.expected != 0 || self.complete
    }

    /// Sticky framing refusal.
    #[inline]
    pub const fn refused(&self) -> bool {
        self.refused
    }

    /// Pull until one complete unmasked payload is copied into `out`.
    /// `expect_masked`: `Some(true)` requires a mask, `Some(false)` forbids it,
    /// `None` accepts either.
    pub fn recv<R: Read + ?Sized>(
        &mut self,
        src: &mut R,
        out: &mut [u8],
        expect_masked: Option<bool>,
    ) -> Result<(u8, usize), QdnfError> {
        if self.refused {
            return Err(QdnfError::Capacity);
        }
        self.assemble(src, expect_masked)?;
        let hdr_len = self.hdr as usize;
        let payload_len = self.expected as usize - hdr_len;
        if out.len() < payload_len {
            return Err(QdnfError::Capacity);
        }
        if payload_len != 0 {
            out[..payload_len].copy_from_slice(&self.buf[hdr_len..hdr_len + payload_len]);
        }
        let opcode = self.opcode;
        self.reset();
        Ok((opcode, payload_len))
    }

    /// Drop partial assembly. Does not clear oversize poison.
    pub fn abort(&mut self) {
        self.filled = 0;
        self.expected = 0;
        self.hdr = 0;
        self.opcode = 0;
        self.complete = false;
        self.unmasked = false;
    }

    fn reset(&mut self) {
        self.abort();
    }

    fn refuse(&mut self) {
        self.abort();
        self.refused = true;
    }

    fn assemble<R: Read + ?Sized>(
        &mut self,
        src: &mut R,
        expect_masked: Option<bool>,
    ) -> Result<(), QdnfError> {
        if self.complete {
            return Ok(());
        }
        if self.expected == 0 {
            self.pull(src, 2)?;
            let need = match header_need(self.buf[0], self.buf[1]) {
                Ok(n) => n,
                Err(e) => {
                    self.refuse();
                    return Err(e);
                }
            };
            self.pull(src, need)?;
            let hdr = match parse_header(&self.buf[..need]) {
                Ok(h) => h,
                Err(e) => {
                    self.refuse();
                    return Err(e);
                }
            };
            if let Some(expect) = expect_masked {
                if hdr.masked != expect {
                    self.refuse();
                    return Err(QdnfError::Malformed);
                }
            }
            if hdr.payload_len > MAX_FRAME {
                self.refuse();
                return Err(QdnfError::Capacity);
            }
            let total = hdr
                .hdr_len
                .checked_add(hdr.payload_len)
                .ok_or(QdnfError::Range)?;
            if total > WIRE_CAP {
                self.refuse();
                return Err(QdnfError::Capacity);
            }
            self.hdr = hdr.hdr_len as u8;
            self.opcode = hdr.opcode;
            self.expected = total as u16;
        }
        self.pull(src, self.expected as usize)?;
        if !self.unmasked {
            let start = self.hdr as usize;
            let hdr = parse_header(&self.buf[..start]).map_err(|e| {
                self.refuse();
                e
            })?;
            if hdr.masked {
                let body = &mut self.buf[start..self.expected as usize];
                for (i, b) in body.iter_mut().enumerate() {
                    *b ^= hdr.mask[i % 4];
                }
            }
            self.unmasked = true;
        }
        self.complete = true;
        Ok(())
    }

    fn pull<R: Read + ?Sized>(&mut self, src: &mut R, want: usize) -> Result<(), QdnfError> {
        while (self.filled as usize) < want {
            let dest = &mut self.buf[self.filled as usize..want];
            match src.read(dest) {
                Ok(0) => {
                    let idle = self.filled == 0 && self.expected == 0 && !self.complete;
                    self.abort();
                    return Err(if idle {
                        QdnfError::Closed
                    } else {
                        QdnfError::Truncated
                    });
                }
                Ok(n) => {
                    if n > dest.len() {
                        return Err(QdnfError::Range);
                    }
                    self.filled += n as u16;
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    return Err(QdnfError::WouldBlock);
                }
                Err(e) => return Err(map_io(e)),
            }
        }
        Ok(())
    }
}

impl Default for ReadCursor {
    fn default() -> Self {
        Self::new()
    }
}
