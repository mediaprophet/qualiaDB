//! Caller-buffered QFrame decoder. Untrusted input is never cast.

use super::errors::FrameError;
use super::header::{FrameHeader, BASE_HEADER_LEN, MAGIC, VERSION};
use crate::net::qdnf::registries::{FrameType, NextProtocol};
use crate::net::qdnf::types::{FlowId, LinkId, Sequence};

/// Decode a complete frame from `src`. Returns `(header, payload_offset, payload_len)`.
///
/// On any failure, consumed bytes are zero and `out` views are invalid.
pub fn decode_frame(src: &[u8]) -> Result<(FrameHeader, usize, usize), FrameError> {
    if src.len() < BASE_HEADER_LEN {
        return Err(FrameError::Truncated);
    }
    if src[0..4] != MAGIC {
        return Err(FrameError::Malformed);
    }
    if src[4] != VERSION {
        return Err(FrameError::Unsupported);
    }
    let frame_type = FrameType::from_u8(src[5])?;
    let flags = u16::from_be_bytes([src[6], src[7]]);
    let header_len = u16::from_be_bytes([src[8], src[9]]);
    let payload_len = u16::from_be_bytes([src[10], src[11]]);
    if header_len as usize != BASE_HEADER_LEN {
        return Err(FrameError::Unsupported);
    }
    if src[14] != 0 || src[15] != 0 {
        return Err(FrameError::Malformed);
    }
    let next_protocol = NextProtocol::from_u8(src[13])?;
    let hop_limit = src[12];
    if !frame_type.forwarded() && hop_limit != 0 {
        return Err(FrameError::Malformed);
    }

    let mut source_link_id = LinkId::ZERO;
    source_link_id.0.copy_from_slice(&src[16..32]);
    let mut destination_link_id = LinkId::ZERO;
    destination_link_id.0.copy_from_slice(&src[32..48]);
    if frame_type == FrameType::DiscoveryBeacon && !destination_link_id.is_zero() {
        // Discovery may use a group selector; zero dest is also permitted.
    }
    let mut flow_id = FlowId::ZERO;
    flow_id.0.copy_from_slice(&src[48..56]);
    let sequence = Sequence(u64::from_be_bytes([
        src[56], src[57], src[58], src[59], src[60], src[61], src[62], src[63],
    ]));
    let mut header_tag = [0u8; 16];
    header_tag.copy_from_slice(&src[64..80]);

    let total = (header_len as usize)
        .checked_add(payload_len as usize)
        .ok_or(FrameError::Range)?;
    if src.len() < total {
        return Err(FrameError::Truncated);
    }

    let header = FrameHeader {
        version: src[4],
        frame_type,
        flags,
        header_len,
        payload_len,
        hop_limit,
        next_protocol,
        source_link_id,
        destination_link_id,
        flow_id,
        sequence,
        header_tag,
    };
    Ok((header, BASE_HEADER_LEN, payload_len as usize))
}

/// Copy payload into `out` after a successful `decode_frame`.
pub fn copy_payload(
    src: &[u8],
    payload_offset: usize,
    payload_len: usize,
    out: &mut [u8],
) -> Result<usize, FrameError> {
    let end = payload_offset
        .checked_add(payload_len)
        .ok_or(FrameError::Range)?;
    if src.len() < end {
        return Err(FrameError::Truncated);
    }
    if out.len() < payload_len {
        return Err(FrameError::Capacity);
    }
    if payload_len != 0 {
        out[..payload_len].copy_from_slice(&src[payload_offset..end]);
    }
    Ok(payload_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::frame::encode::encode_frame;
    use crate::net::qdnf::registries::{FrameType, NextProtocol};

    #[test]
    fn round_trip_discovery() {
        let mut header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
        header.payload_len = 4;
        let payload = [9u8, 8, 7, 6];
        let mut wire = [0u8; 128];
        let n = encode_frame(&header, &payload, &mut wire).unwrap();
        let (decoded, off, len) = decode_frame(&wire[..n]).unwrap();
        assert_eq!(decoded.frame_type, FrameType::DiscoveryBeacon);
        let mut out = [0u8; 8];
        copy_payload(&wire[..n], off, len, &mut out).unwrap();
        assert_eq!(&out[..4], &payload);
    }

    #[test]
    fn truncated_is_zero_consumed() {
        let err = decode_frame(&[0u8; 16]).unwrap_err();
        assert_eq!(err, FrameError::Truncated);
        assert_eq!(err.consumed_on_failure(), 0);
    }

    #[test]
    fn bad_magic_is_malformed() {
        let mut buf = [0u8; 80];
        buf[0..4].copy_from_slice(b"XXXX");
        buf[4] = VERSION;
        assert_eq!(decode_frame(&buf), Err(FrameError::Malformed));
    }
}
