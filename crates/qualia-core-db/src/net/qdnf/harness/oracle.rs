//! Independent header parser. Does not call the production encoder or decoder.

use crate::net::qdnf::errors::QdnfError;

/// Minimum bytes needed to read magic, type and hop_limit (wire table §2).
const MAGIC_LEN: usize = 4;
const HOP_OFFSET: usize = 12;
const TYPE_OFFSET: usize = 5;
const HEADER_LEN_OFFSET: usize = 8;
const PAYLOAD_LEN_OFFSET: usize = 10;
const BASE_HEADER_BYTES: usize = 80;

/// Oracle view of the QFrame base header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OracleHeader {
    pub magic_ok: bool,
    pub version: u8,
    pub frame_type: u8,
    pub header_len: u16,
    pub payload_len: u16,
    pub hop_limit: u8,
}

/// Independent magic check. Does not call `decode_frame`.
#[inline]
pub fn independent_magic_ok(src: &[u8]) -> bool {
    src.len() >= MAGIC_LEN && src[0] == b'Q' && src[1] == b'D' && src[2] == b'N' && src[3] == b'F'
}

/// Wire-table link-only type codes (not forwarded). Independent of `FrameType`.
#[inline]
const fn link_only_type(frame_type: u8) -> bool {
    matches!(frame_type, 1 | 2 | 3 | 4)
}

/// Independent hop_limit check. Does not call `decode_frame`.
pub fn independent_hop_limit(src: &[u8]) -> Result<u8, QdnfError> {
    if src.len() <= HOP_OFFSET {
        return Err(QdnfError::Truncated);
    }
    if !independent_magic_ok(src) {
        return Err(QdnfError::Malformed);
    }
    let hop = src[HOP_OFFSET];
    if link_only_type(src[TYPE_OFFSET]) && hop != 0 {
        return Err(QdnfError::HopLimit);
    }
    Ok(hop)
}

/// Byte-at-a-time parse used as an independent check of golden vectors.
pub fn independent_decode_header(src: &[u8]) -> Result<OracleHeader, QdnfError> {
    if src.len() < BASE_HEADER_BYTES {
        return Err(QdnfError::Truncated);
    }
    let magic_ok = independent_magic_ok(src);
    if !magic_ok {
        return Err(QdnfError::Malformed);
    }
    let hop_limit = independent_hop_limit(src)?;
    let header_len = ((src[HEADER_LEN_OFFSET] as u16) << 8) | src[HEADER_LEN_OFFSET + 1] as u16;
    let payload_len = ((src[PAYLOAD_LEN_OFFSET] as u16) << 8) | src[PAYLOAD_LEN_OFFSET + 1] as u16;
    Ok(OracleHeader {
        magic_ok,
        version: src[4],
        frame_type: src[TYPE_OFFSET],
        header_len,
        payload_len,
        hop_limit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::frame::{encode_frame, FrameHeader};
    use crate::net::qdnf::registries::{FrameType, NextProtocol};

    fn hand_header(frame_type: u8, hop: u8) -> [u8; BASE_HEADER_BYTES] {
        let mut src = [0u8; BASE_HEADER_BYTES];
        src[0] = b'Q';
        src[1] = b'D';
        src[2] = b'N';
        src[3] = b'F';
        src[4] = 1;
        src[5] = frame_type;
        src[8] = 0;
        src[9] = 80;
        src[12] = hop;
        src
    }

    #[test]
    fn oracle_matches_encoder_lengths() {
        let mut header = FrameHeader::new(FrameType::LinkChallenge, NextProtocol::QLink);
        header.payload_len = 3;
        let mut wire = [0u8; 128];
        let n = encode_frame(&header, &[1, 2, 3], &mut wire).unwrap();
        let oracle = independent_decode_header(&wire[..n]).unwrap();
        assert_eq!(oracle.version, 1);
        assert_eq!(oracle.frame_type, FrameType::LinkChallenge as u8);
        assert_eq!(oracle.payload_len, 3);
        assert_eq!(oracle.header_len, 80);
        assert_eq!(oracle.hop_limit, 0);
        assert!(oracle.magic_ok);
    }

    #[test]
    fn magic_reject_does_not_call_decode_frame() {
        let mut src = hand_header(1, 0);
        src[0] = b'X';
        assert!(!independent_magic_ok(&src));
        assert_eq!(independent_hop_limit(&src), Err(QdnfError::Malformed));
        assert_eq!(independent_decode_header(&src), Err(QdnfError::Malformed));
    }

    #[test]
    fn hop_limit_rejects_link_only_nonzero() {
        let src = hand_header(2, 16);
        assert!(independent_magic_ok(&src));
        assert_eq!(independent_hop_limit(&src), Err(QdnfError::HopLimit));
    }

    #[test]
    fn hop_limit_allows_forwarded_nonzero() {
        let src = hand_header(64, 16);
        assert_eq!(independent_hop_limit(&src), Ok(16));
        let oracle = independent_decode_header(&src).unwrap();
        assert_eq!(oracle.hop_limit, 16);
        assert_eq!(oracle.frame_type, 64);
    }

    #[test]
    fn hop_limit_truncated() {
        assert_eq!(
            independent_hop_limit(&[b'Q', b'D', b'N']),
            Err(QdnfError::Truncated)
        );
    }
}
