//! Caller-buffered QFrame encoder. No heap on success or failure.

use super::errors::FrameError;
use super::header::{FrameHeader, BASE_HEADER_LEN, MAGIC, MAX_FRAME_LEN, VERSION};

/// Encode `header` and `payload` into `out`. Returns bytes written.
///
/// `out` must not overlap `payload`. The encoder never reads back from `out`.
pub fn encode_frame(
    header: &FrameHeader,
    payload: &[u8],
    out: &mut [u8],
) -> Result<usize, FrameError> {
    if header.version != VERSION {
        return Err(FrameError::Unsupported);
    }
    if payload.len() > u16::MAX as usize {
        return Err(FrameError::Capacity);
    }
    if payload.len() != header.payload_len as usize {
        return Err(FrameError::Malformed);
    }
    let header_len = header.header_len as usize;
    if header_len < BASE_HEADER_LEN {
        return Err(FrameError::Malformed);
    }
    let total = header_len
        .checked_add(payload.len())
        .ok_or(FrameError::Range)?;
    if total > MAX_FRAME_LEN {
        return Err(FrameError::Capacity);
    }
    if out.len() < total {
        return Err(FrameError::Capacity);
    }
    if header_len > BASE_HEADER_LEN {
        return Err(FrameError::Unsupported);
    }

    out[0..4].copy_from_slice(&MAGIC);
    out[4] = header.version;
    out[5] = header.frame_type.as_u8();
    out[6..8].copy_from_slice(&header.flags.to_be_bytes());
    out[8..10].copy_from_slice(&header.header_len.to_be_bytes());
    out[10..12].copy_from_slice(&header.payload_len.to_be_bytes());
    out[12] = header.hop_limit;
    out[13] = header.next_protocol as u8;
    out[14..16].copy_from_slice(&[0, 0]);
    out[16..32].copy_from_slice(&header.source_link_id.0);
    out[32..48].copy_from_slice(&header.destination_link_id.0);
    out[48..56].copy_from_slice(&header.flow_id.0);
    out[56..64].copy_from_slice(&header.sequence.0.to_be_bytes());
    out[64..80].copy_from_slice(&header.header_tag);
    if !payload.is_empty() {
        out[header_len..total].copy_from_slice(payload);
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::registries::{FrameType, NextProtocol};

    #[test]
    fn encode_rejects_short_output() {
        let header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
        let mut out = [0u8; 16];
        assert_eq!(
            encode_frame(&header, &[], &mut out),
            Err(FrameError::Capacity)
        );
    }

    #[test]
    fn encode_rejects_payload_len_mismatch() {
        let header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
        let mut out = [0u8; 128];
        assert_eq!(
            encode_frame(&header, &[1], &mut out),
            Err(FrameError::Malformed)
        );
    }
}
