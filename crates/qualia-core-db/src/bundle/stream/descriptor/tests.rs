use super::*;
use crate::net::qdnf::types::StrongDigest;
fn digest(n: u8) -> StrongDigest {
    StrongDigest([n; 48])
}
fn parameters() -> ContextParameters {
    ContextParameters {
        nonce: [1; 32],
        generation: 0,
        predecessor: None,
        semantic_contract: digest(2),
        policy: digest(3),
        consumer_profile: digest(4),
    }
}
fn source() -> SourceCommit {
    SourceCommit {
        length: 65,
        version: 1,
        flags: 0,
        header_digest: digest(5),
        index_offset: 64,
        index_length: 1,
        index_digest: digest(6),
        source_digest: digest(7),
    }
}
fn reference(bytes: &[u8], domain: DigestDomain, frame: &mut [u8]) -> PageRef {
    PageRef {
        number: 0,
        first: 0,
        rows: 1,
        encoded_length: bytes.len() as u32,
        digest: domain_digest(domain, bytes, frame).unwrap(),
    }
}
#[test]
fn independently_spelled_context_page_bytes_and_both_directions_agree() {
    let rows = [ContextEntry { id: 42, length: 0 }];
    let mut out = [0u8; 256];
    let n = encode_context_page_into(0, 0, &rows, &mut out).unwrap();
    let expected =
        b"\xa5\x00\x70qbdl-stream-pq-1\x01\x01\x02\x00\x03\x00\x04\x81\x84\x18\x2a\x00\x00\x00";
    assert_eq!(&out[..n], expected);
    let mut decoded = [ContextEntry::EMPTY; 1];
    let h = decode_context_page(expected, &mut decoded).unwrap();
    assert_eq!(h.rows, 1);
    assert_eq!(decoded, rows);
    for end in 0..n {
        assert!(decode_context_page(&out[..end], &mut decoded).is_err());
    }
    assert_eq!(
        decode_context_page(&out[..n], &mut []),
        Err(DescriptorError::Capacity)
    );
    let mut nonminimal = vec![0xb8, 5];
    nonminimal.extend_from_slice(&out[1..n]);
    assert_eq!(
        decode_context_page(&nonminimal, &mut decoded),
        Err(DescriptorError::Encoding)
    );
}
#[test]
fn entry_page_mapping_is_strict_and_borrowed() {
    let rows = [EntryRecord {
        id: 17,
        ordinal: 0,
        kind: "blob",
        inner_version: 8,
        offset: 64,
        length: 0,
        root: digest(8),
    }];
    let mut out = [0u8; 512];
    let n = encode_entry_page_into(digest(9), 0, 0, &rows, &mut out).unwrap();
    let mut decoded = [EntryRecord::EMPTY; 1];
    let h = decode_entry_page(&out[..n], &mut decoded).unwrap();
    assert_eq!(h.context, digest(9));
    assert_eq!(decoded, rows);
    let mut changed = rows;
    changed[0].ordinal = 1;
    assert_eq!(
        encode_entry_page_into(digest(9), 0, 0, &changed, &mut out),
        Err(DescriptorError::Field)
    );
    assert_eq!(
        encode_context_page_into(1, 0, &[ContextEntry::EMPTY], &mut out),
        Err(DescriptorError::Page)
    );
}
#[test]
fn roots_roundtrip_exact_length_domains_and_reject_every_truncated_prefix() {
    let mut frame = [0u8; HASH_SCRATCH_BYTES];
    let mut page = [0u8; 256];
    let n =
        encode_context_page_into(0, 0, &[ContextEntry { id: 9, length: 0 }], &mut page).unwrap();
    let refs = [reference(&page[..n], DigestDomain::ContextPage, &mut frame)];
    let input = ContextInput {
        parameters: parameters(),
        source: source(),
        entries: 1,
        pages: &refs,
    };
    let mut context = [0u8; 2048];
    let n = encode_context_into(&input, &mut context).unwrap();
    assert_eq!(context_encoded_len(&input), Ok(n));
    let view = decode_context(&context[..n]).unwrap();
    assert_eq!(view.source(), source());
    assert_eq!(view.parameters(), parameters());
    assert_eq!(view.page_refs().get(0), Ok(refs[0]));
    let id = domain_digest(DigestDomain::Context, &context[..n], &mut frame).unwrap();
    assert_ne!(
        id,
        domain_digest(DigestDomain::Body, &context[..n], &mut frame).unwrap()
    );
    let mut explicit = b"QBDL-QASD-CONTEXT-V1\0".to_vec();
    explicit.extend_from_slice(&context[..n]);
    assert_eq!(id, crate::crypto::network::digest::sha384(&explicit));
    let body_input = BodyInput {
        context_id: id,
        context_root: &context[..n],
        generation: 0,
        predecessor: None,
        entries: 1,
        pages: &refs,
    };
    let mut body = [0u8; 4096];
    let m = encode_body_into(&body_input, &mut body).unwrap();
    assert_eq!(body_encoded_len(&body_input), Ok(m));
    let parsed = decode_body(&body[..m]).unwrap();
    assert_eq!(parsed.context_id(), id);
    for end in 0..m {
        assert!(decode_body(&body[..end]).is_err());
    }
    let mut trailing = body[..m].to_vec();
    trailing.push(0);
    assert_eq!(
        decode_body(&trailing).err(),
        Some(DescriptorError::Encoding)
    );
    assert_eq!(
        encode_body_into(&body_input, &mut body[..m - 1]),
        Err(DescriptorError::Capacity)
    );
}
#[test]
fn page_reference_geometry_required_fields_and_limits_refuse() {
    let bad = [PageRef {
        number: 0,
        first: 0,
        rows: 2,
        encoded_length: 1,
        digest: digest(1),
    }];
    let input = ContextInput {
        parameters: parameters(),
        source: source(),
        entries: 1,
        pages: &bad,
    };
    assert_eq!(
        encode_context_into(&input, &mut [0; 2048]),
        Err(DescriptorError::Coverage)
    );
    let mut options = parameters();
    options.nonce = [0; 32];
    let input = ContextInput {
        parameters: options,
        source: source(),
        entries: 0,
        pages: &[],
    };
    assert_eq!(
        encode_context_into(&input, &mut [0; 2048]),
        Err(DescriptorError::Context)
    );
    let mut options = parameters();
    options.generation = 1;
    let input = ContextInput {
        parameters: options,
        source: source(),
        entries: 0,
        pages: &[],
    };
    assert_eq!(
        encode_context_into(&input, &mut [0; 2048]),
        Err(DescriptorError::Context)
    );
    let mut frame = [0; 64];
    assert_eq!(
        domain_digest(DigestDomain::Body, &[0; 65], &mut frame),
        Err(DescriptorError::Capacity)
    );
    assert_eq!(
        decode_context(&vec![0; MAX_ENCODED + 1]).err(),
        Some(DescriptorError::Limit)
    );
}
