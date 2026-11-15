use super::*;
use crate::{crypto::network::object_stream::RecordHeader, net::qdnf::types::StrongDigest};
fn aad(total: u32, offset: u32, len: u16, final_chunk: bool) -> [u8; 88] {
    let mut out = [0; 88];
    RecordHeader {
        transcript: StrongDigest([1; 48]),
        request: [2; 16],
        total,
        offset,
        chunk: len,
        final_chunk,
    }
    .encode(&mut out)
    .unwrap();
    out
}
#[test]
fn common_geometry_reuses_actual_frozen_private_header_without_loosening_its_limit() {
    let a = aad(1025, 0, 1024, false);
    assert_eq!(send::check_aad(&a, 1024), Ok(()));
    let b = aad(1025, 1024, 1, true);
    assert_eq!(send::check_aad(&b, 1), Ok(()));
    assert!(send::check_aad(&b, 2).is_err());
    let mut changed = b;
    changed[87] = 0;
    assert!(send::check_aad(&changed, 1).is_err());
    changed = b;
    changed[60..76].fill(0);
    assert!(send::check_aad(&changed, 1).is_err());
    changed = b;
    changed[80..84].copy_from_slice(&1023u32.to_be_bytes());
    assert!(send::check_aad(&changed, 1).is_err());
    let private = RecordHeader {
        transcript: StrongDigest([1; 48]),
        request: [2; 16],
        total: 8193,
        offset: 0,
        chunk: 1024,
        final_chunk: false,
    };
    assert!(private.encode(&mut [0; 88]).is_err());
    assert_eq!(FRAME_BYTES, 1216);
}
