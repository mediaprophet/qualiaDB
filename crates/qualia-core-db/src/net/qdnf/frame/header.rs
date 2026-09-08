//! Fixed 80-byte QFrame base header. Network byte order.

use super::errors::FrameError;
use crate::net::qdnf::registries::{flags, FrameType, NextProtocol};
use crate::net::qdnf::types::{FlowId, LinkId, Sequence};

pub const MAGIC: [u8; 4] = *b"QDNF";
pub const VERSION: u8 = 1;
pub const BASE_HEADER_LEN: usize = 80;
pub const MAX_FRAME_LEN: usize = 65_535;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameHeader {
    pub version: u8,
    pub frame_type: FrameType,
    pub flags: u16,
    pub header_len: u16,
    pub payload_len: u16,
    pub hop_limit: u8,
    pub next_protocol: NextProtocol,
    pub source_link_id: LinkId,
    pub destination_link_id: LinkId,
    pub flow_id: FlowId,
    pub sequence: Sequence,
    pub header_tag: [u8; 16],
}

impl FrameHeader {
    pub fn new(frame_type: FrameType, next_protocol: NextProtocol) -> Self {
        Self {
            version: VERSION,
            frame_type,
            flags: 0,
            header_len: BASE_HEADER_LEN as u16,
            payload_len: 0,
            hop_limit: if frame_type.forwarded() { 16 } else { 0 },
            next_protocol,
            source_link_id: LinkId::ZERO,
            destination_link_id: LinkId::ZERO,
            flow_id: FlowId::ZERO,
            sequence: Sequence::ZERO,
            header_tag: [0u8; 16],
        }
    }

    #[inline]
    pub const fn is_critical(self) -> bool {
        self.flags & flags::CRITICAL != 0
    }

    #[inline]
    pub const fn is_fragment(self) -> bool {
        self.flags & flags::FRAGMENT != 0
    }

    /// Complete on-wire length of this frame (header + payload).
    pub fn wire_len(self) -> Result<usize, FrameError> {
        let header = self.header_len as usize;
        let payload = self.payload_len as usize;
        if header < BASE_HEADER_LEN {
            return Err(FrameError::Malformed);
        }
        header.checked_add(payload).ok_or(FrameError::Range)
    }
}
