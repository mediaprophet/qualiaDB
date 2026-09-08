//! FND-03.07 golden capture of a live empty DiscoveryBeacon (PARTIAL).
//!
//! Bytes come from the production `encode_frame` path. This module is not the
//! independent codec oracle (QA-01.03).

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::frame::{encode_frame, FrameError, FrameHeader};
use crate::net::qdnf::registries::{FrameType, NextProtocol};
use crate::net::qdnf::types::ProfileId;

/// Frozen MAGIC expected on the empty-beacon golden.
pub const GOLDEN_MAGIC: [u8; 4] = *b"QDNF";
/// Frozen VERSION expected on the empty-beacon golden.
pub const GOLDEN_VERSION: u8 = 1;
/// Frozen base-header length expected on the empty-beacon golden.
pub const GOLDEN_BASE_HEADER_LEN: usize = 80;

/// First 16 bytes of a live-encoded empty DiscoveryBeacon.
///
/// Layout: MAGIC | version | DiscoveryBeacon | flags | header_len | payload_len
/// | hop_limit | QLink | reserved.
pub const EMPTY_BEACON_PREFIX: [u8; 16] =
    [b'Q', b'D', b'N', b'F', 1, 1, 0, 0, 0, 80, 0, 0, 0, 1, 0, 0];

/// Encode an empty DiscoveryBeacon into `out` using the production encoder.
pub fn capture_empty_discovery_beacon(out: &mut [u8]) -> Result<usize, FrameError> {
    let header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
    encode_frame(&header, &[], out)
}

/// Known profiles fail closed. Unknown ids never infer a suite.
pub fn require_known_profile(id: ProfileId) -> Result<(), QdnfError> {
    if id == ProfileId::QPR_PQ_1 || id == ProfileId::QDNF_CRYPTO_1 {
        Ok(())
    } else {
        Err(QdnfError::UnknownProfile)
    }
}

/// Classical retry is not an automatic downgrade path.
#[inline]
pub fn allow_classical_retry() -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub use crate::q42::q42_volume::NetworkCursor;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::frame::decode_frame;

    #[test]
    fn empty_beacon_matches_frozen_prefix() {
        let mut wire = [0u8; GOLDEN_BASE_HEADER_LEN];
        let n = capture_empty_discovery_beacon(&mut wire).expect("encode");
        assert_eq!(n, GOLDEN_BASE_HEADER_LEN);
        assert_eq!(&wire[0..4], &GOLDEN_MAGIC);
        assert_eq!(wire[4], GOLDEN_VERSION);
        assert_eq!(&wire[0..16], &EMPTY_BEACON_PREFIX);
        assert_eq!(&wire[80..], &[] as &[u8]);
    }

    #[test]
    fn empty_beacon_round_trips_decode_frame() {
        let mut wire = [0u8; GOLDEN_BASE_HEADER_LEN];
        let n = capture_empty_discovery_beacon(&mut wire).expect("encode");
        let (header, off, len) = decode_frame(&wire[..n]).expect("decode");
        assert_eq!(header.version, GOLDEN_VERSION);
        assert_eq!(header.header_len as usize, GOLDEN_BASE_HEADER_LEN);
        assert_eq!(header.payload_len, 0);
        assert_eq!(header.frame_type, FrameType::DiscoveryBeacon);
        assert_eq!(header.next_protocol, NextProtocol::QLink);
        assert_eq!(off, GOLDEN_BASE_HEADER_LEN);
        assert_eq!(len, 0);
    }

    #[test]
    fn malformed_magic_and_truncated_input() {
        let mut wire = [0u8; GOLDEN_BASE_HEADER_LEN];
        let n = capture_empty_discovery_beacon(&mut wire).expect("encode");
        wire[0] ^= 0xFF;
        assert_eq!(decode_frame(&wire[..n]), Err(FrameError::Malformed));

        let short = [0u8; 16];
        assert_eq!(decode_frame(&short), Err(FrameError::Truncated));
    }

    #[test]
    fn unknown_profile_fails_closed_without_downgrade() {
        assert_eq!(require_known_profile(ProfileId::QPR_PQ_1), Ok(()));
        assert_eq!(require_known_profile(ProfileId::QDNF_CRYPTO_1), Ok(()));
        assert_eq!(
            require_known_profile(ProfileId(0x0000)),
            Err(QdnfError::UnknownProfile)
        );
        assert!(!allow_classical_retry());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn network_cursor_binds_and_rejects_cross_scope() {
        let source = [0xABu8; 32];
        let generation = 4u64;
        let profile = ProfileId::QPR_PQ_1.0;
        let scope = 0x51C0u64;
        let mut cursor = NetworkCursor::bind(source, generation, profile, scope);
        assert_eq!(
            cursor
                .advance(&source, generation, profile, scope, 1)
                .expect("same-scope"),
            1
        );
        assert_eq!(
            cursor.advance(&source, generation, profile, scope ^ 1, 1),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(cursor.offset, 1);
        assert_eq!(cursor.query_generation, generation);
        assert_eq!(cursor.scope, scope);
    }
}
