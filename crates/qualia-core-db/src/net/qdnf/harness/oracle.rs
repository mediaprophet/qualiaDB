//! Independent header parser. Does not call the production encoder or decoder.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::frame::header::BASE_HEADER_LEN;

/// Oracle view of the QFrame base header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OracleHeader {
    pub magic_ok: bool,
    pub version: u8,
    pub frame_type: u8,
    pub header_len: u16,
    pub payload_len: u16,
}

/// Byte-at-a-time parse used as an independent check of golden vectors.
pub fn independent_decode_header(src: &[u8]) -> Result<OracleHeader, QdnfError> {
    if src.len() < BASE_HEADER_LEN {
        return Err(QdnfError::Truncated);
    }
    let magic_ok = src[0] == b'Q' && src[1] == b'D' && src[2] == b'N' && src[3] == b'F';
    if !magic_ok {
        return Err(QdnfError::Malformed);
    }
    let header_len = ((src[8] as u16) << 8) | src[9] as u16;
    let payload_len = ((src[10] as u16) << 8) | src[11] as u16;
    Ok(OracleHeader {
        magic_ok,
        version: src[4],
        frame_type: src[5],
        header_len,
        payload_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::frame::{encode_frame, FrameHeader};
    use crate::net::qdnf::registries::{FrameType, NextProtocol};

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
    }
}
