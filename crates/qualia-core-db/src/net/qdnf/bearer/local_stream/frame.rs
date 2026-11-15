//! Length-prefixed framing over a reliable ordered byte stream.
//!
//! Wire: `u32be payload_len || payload`. Not JSON, not CBOR maps. Handshake
//! datagram reassembly stays in `bearer/fragment.rs`.

use crate::net::qdnf::errors::QdnfError;

use super::StreamBytes;

/// Maximum payload bytes. Same value as in-process `ipc.rs` `FRAME_CAP`.
pub const FRAME_CAP: usize = 8192;
/// Big-endian length prefix.
pub const LEN_SIZE: usize = 4;
/// Maximum sealed wire bytes (`LEN_SIZE + FRAME_CAP`).
pub const WIRE_CAP: usize = LEN_SIZE + FRAME_CAP;

const _: () = assert!(FRAME_CAP == 8192);
const _: () = assert!(WIRE_CAP == 8196);

/// Encode `payload` as `u32be(len) || payload` into `out`.
pub fn encode_stream_frame(payload: &[u8], out: &mut [u8]) -> Result<usize, QdnfError> {
    if payload.len() > FRAME_CAP {
        return Err(QdnfError::Capacity);
    }
    let total = LEN_SIZE
        .checked_add(payload.len())
        .ok_or(QdnfError::Range)?;
    if out.len() < total {
        return Err(QdnfError::Capacity);
    }
    let len = u32::try_from(payload.len()).map_err(|_| QdnfError::Capacity)?;
    out[..LEN_SIZE].copy_from_slice(&len.to_be_bytes());
    if !payload.is_empty() {
        out[LEN_SIZE..total].copy_from_slice(payload);
    }
    Ok(total)
}

/// Decode one frame. Extra bytes after the declared length are trailing
/// ([`QdnfError::Malformed`]).
pub fn decode_stream_frame(src: &[u8]) -> Result<&[u8], QdnfError> {
    let (payload, consumed) = next_stream_frame(src)?;
    if consumed != src.len() {
        return Err(QdnfError::Malformed);
    }
    Ok(payload)
}

/// Decode one frame; leftover bytes after `consumed` are for the caller.
pub fn next_stream_frame(src: &[u8]) -> Result<(&[u8], usize), QdnfError> {
    if src.len() < LEN_SIZE {
        return Err(QdnfError::Truncated);
    }
    let mut len_bytes = [0u8; LEN_SIZE];
    len_bytes.copy_from_slice(&src[..LEN_SIZE]);
    let declared = u32::from_be_bytes(len_bytes) as usize;
    if declared > FRAME_CAP {
        return Err(QdnfError::Capacity);
    }
    let total = LEN_SIZE.checked_add(declared).ok_or(QdnfError::Range)?;
    if src.len() < total {
        return Err(QdnfError::Truncated);
    }
    Ok((&src[LEN_SIZE..total], total))
}

/// Sealed pending write. WouldBlock retains the exact wire bytes; `off`
/// advances only by accepted sink bytes.
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

    /// Sealed wire length, including the prefix.
    #[inline]
    pub const fn sealed_len(&self) -> usize {
        self.len as usize
    }

    /// Exact sealed bytes. Empty when idle.
    #[inline]
    pub fn sealed_bytes(&self) -> &[u8] {
        &self.buf[..self.len as usize]
    }

    /// Seal `payload` if idle. The same payload while pending is a resume.
    /// A different payload while pending is [`QdnfError::Conflict`] and does
    /// not reseal or rewind `off`.
    pub fn seal(&mut self, payload: &[u8]) -> Result<(), QdnfError> {
        if self.len != 0 {
            if self.payload_matches(payload) {
                return Ok(());
            }
            return Err(QdnfError::Conflict);
        }
        let n = encode_stream_frame(payload, &mut self.buf)?;
        self.len = n as u16;
        self.off = 0;
        Ok(())
    }

    /// Write remaining sealed bytes. Advances `off` only on `Ok(n > 0)`.
    pub fn flush<S: StreamBytes + ?Sized>(&mut self, sink: &mut S) -> Result<(), QdnfError> {
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
                Err(QdnfError::WouldBlock) => return Err(QdnfError::WouldBlock),
                Err(e) => return Err(e),
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

    fn payload_matches(&self, payload: &[u8]) -> bool {
        let n = self.len as usize;
        n >= LEN_SIZE && &self.buf[LEN_SIZE..n] == payload
    }
}

impl Default for WriteCursor {
    fn default() -> Self {
        Self::new()
    }
}

/// Partial-read assembly. Oversize length poisons the cursor; a short `out`
/// after a complete frame is [`QdnfError::Capacity`] and does not drop it.
pub struct ReadCursor {
    buf: [u8; WIRE_CAP],
    filled: u16,
    expected: u16,
    refused: bool,
}

impl ReadCursor {
    /// Empty cursor.
    pub const fn new() -> Self {
        Self {
            buf: [0u8; WIRE_CAP],
            filled: 0,
            expected: 0,
            refused: false,
        }
    }

    /// Bytes buffered toward the current frame.
    #[inline]
    pub const fn filled(&self) -> usize {
        self.filled as usize
    }

    /// `true` while a prefix or body is incomplete.
    #[inline]
    pub const fn pending(&self) -> bool {
        self.filled != 0 || self.expected != 0
    }

    /// Sticky oversize refusal.
    #[inline]
    pub const fn refused(&self) -> bool {
        self.refused
    }

    /// Pull until one complete payload is copied into `out`.
    pub fn recv<S: StreamBytes + ?Sized>(
        &mut self,
        src: &mut S,
        out: &mut [u8],
    ) -> Result<usize, QdnfError> {
        if self.refused {
            return Err(QdnfError::Capacity);
        }
        if self.expected == 0 {
            self.fill(src, LEN_SIZE)?;
            let mut len_bytes = [0u8; LEN_SIZE];
            len_bytes.copy_from_slice(&self.buf[..LEN_SIZE]);
            let declared = u32::from_be_bytes(len_bytes) as usize;
            if declared > FRAME_CAP {
                self.refuse();
                return Err(QdnfError::Capacity);
            }
            let total = LEN_SIZE.checked_add(declared).ok_or(QdnfError::Range)?;
            self.expected = total as u16;
        }
        self.fill(src, self.expected as usize)?;
        let payload_len = self.expected as usize - LEN_SIZE;
        if out.len() < payload_len {
            return Err(QdnfError::Capacity);
        }
        if payload_len != 0 {
            out[..payload_len].copy_from_slice(&self.buf[LEN_SIZE..self.expected as usize]);
        }
        self.filled = 0;
        self.expected = 0;
        Ok(payload_len)
    }

    /// Drop partial assembly. Does not clear oversize poison.
    pub fn abort(&mut self) {
        self.filled = 0;
        self.expected = 0;
    }

    fn refuse(&mut self) {
        self.abort();
        self.refused = true;
    }

    fn fill<S: StreamBytes + ?Sized>(&mut self, src: &mut S, want: usize) -> Result<(), QdnfError> {
        while (self.filled as usize) < want {
            let dest = &mut self.buf[self.filled as usize..want];
            match src.read(dest) {
                Ok(0) => {
                    let idle_prefix = self.filled == 0 && want == LEN_SIZE && self.expected == 0;
                    self.abort();
                    return Err(if idle_prefix {
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
                Err(QdnfError::WouldBlock) => return Err(QdnfError::WouldBlock),
                Err(e) => return Err(e),
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
