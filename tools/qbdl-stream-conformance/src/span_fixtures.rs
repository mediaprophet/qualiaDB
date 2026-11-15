use crate::{allocation::measured, proof::*,  qdnf_types::StrongDigest};
fn digest(hex: &str) -> StrongDigest {
    let mut b = [0; 48];
    for (i, byte) in b.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap();
    }
    StrongDigest(b)
}
fn limits() -> SpanLimits {
    SpanLimits {
        max_leaves: MAX_SPAN_LEAVES,
        max_encoded_bytes: MAX_SPAN_BYTES,
    }
}
#[test]
fn independent_contiguous_leaf_spans_and_zero_heap() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/tree-v1.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let count = case["count"].as_u64().unwrap();
        if count == 0 {
            continue;
        }
        let rep = if case["representation"] == 0 {
            Representation::Plaintext
        } else {
            Representation::Ciphertext
        };
        let spec = TreeSpec::new(
            digest(fixture["context_hex"].as_str().unwrap()),
            19,
            29,
            rep,
            case["epoch"].as_u64().unwrap() as u32,
            case["logical_length"].as_u64().unwrap(),
        )
        .unwrap();
        let root = digest(case["root"].as_str().unwrap());
        for first in [0, count / 2, count - 1] {
            let mut bytes = Vec::new();
            let mut encoded_proofs = Vec::new();
            for index in first..count {
                bytes.extend(
                    (0..spec.encoded_leaf_length(index).unwrap())
                        .map(|j| (index as usize * 31 + j * 7) as u8),
                );
                let siblings: Vec<_> = case["proofs"][index as usize]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| digest(v.as_str().unwrap()))
                    .collect();
                let mut proof = vec![0; 1 + siblings.len() * 48];
                let n = encode_proof(&siblings, &mut proof).unwrap();
                assert_eq!(n, proof.len());
                encoded_proofs.push(proof);
            }
            let proofs: Vec<_> = encoded_proofs.iter().map(Vec::as_slice).collect();
            let mut scratch = [StrongDigest::ZERO; 32];
            assert_eq!(
                measured(|| {
                    let span = verify_leaf_span(
                        &spec,
                        root,
                        first,
                        &bytes,
                        &proofs,
                        &mut scratch,
                        limits(),
                    )
                    .unwrap();
                    assert_eq!(span.tree_spec(), &spec);
                    assert_eq!(span.root(), root);
                    assert_eq!(span.first_leaf(), first);
                    assert_eq!(span.leaf_count(), (count - first) as usize);
                    assert_eq!(span.encoded_bytes().as_ptr(), bytes.as_ptr());
                    if rep == Representation::Plaintext {
                        let base = first * LEAF_BYTES;
                        assert_eq!(span.plaintext_range(base, bytes.len()).unwrap(), bytes);
                        assert_eq!(
                            span.plaintext_range(base + 1, bytes.len() - 1).unwrap(),
                            &bytes[1..]
                        );
                        assert!(span.plaintext_range(base, bytes.len() + 1).is_err());
                        if base > 0 {
                            assert!(span.plaintext_range(base - 1, 1).is_err());
                        }
                        assert!(span.plaintext_range(u64::MAX, usize::MAX).is_err());
                    } else {
                        assert_eq!(
                            span.plaintext_range(first * LEAF_BYTES, 0),
                            Err(ProofError::InvalidSpec)
                        );
                    }
                    assert!(verify_leaf_span(
                        &spec,
                        StrongDigest::ZERO,
                        first,
                        &bytes,
                        &proofs,
                        &mut scratch,
                        limits()
                    )
                    .is_err());
                    assert!(verify_leaf_span(
                        &spec,
                        root,
                        first,
                        &bytes[..bytes.len() - 1],
                        &proofs,
                        &mut scratch,
                        limits()
                    )
                    .is_err());
                    assert!(verify_leaf_span(
                        &spec,
                        root,
                        first,
                        &bytes,
                        &proofs,
                        &mut scratch,
                        SpanLimits {
                            max_leaves: proofs.len() - 1,
                            max_encoded_bytes: MAX_SPAN_BYTES
                        }
                    )
                    .is_err());
                    assert!(verify_leaf_span(
                        &spec,
                        root,
                        first,
                        &bytes,
                        &proofs,
                        &mut scratch,
                        SpanLimits {
                            max_leaves: MAX_SPAN_LEAVES,
                            max_encoded_bytes: bytes.len() - 1
                        }
                    )
                    .is_err());
                    assert!(verify_leaf_span(
                        &spec,
                        root,
                        count,
                        &bytes,
                        &proofs,
                        &mut scratch,
                        limits()
                    )
                    .is_err());
                }),
                0
            );
            let last = bytes.len() - 1;
            bytes[last] ^= 1;
            assert!(
                verify_leaf_span(&spec, root, first, &bytes, &proofs, &mut scratch, limits())
                    .is_err()
            );
            bytes[last] ^= 1;
            bytes.push(0);
            assert!(
                verify_leaf_span(&spec, root, first, &bytes, &proofs, &mut scratch, limits())
                    .is_err()
            );
            bytes.pop();
            drop(proofs);
            encoded_proofs.last_mut().unwrap().push(0);
            let proofs: Vec<_> = encoded_proofs.iter().map(Vec::as_slice).collect();
            assert!(
                verify_leaf_span(&spec, root, first, &bytes, &proofs, &mut scratch, limits())
                    .is_err()
            );
        }
    }
}
#[test]
fn empty_and_overflow_spans_are_never_presence_receipts() {
    let s = TreeSpec::new(StrongDigest::ZERO, 1, 1, Representation::Plaintext, 0, 0).unwrap();
    let mut scratch = [StrongDigest::ZERO; 32];
    assert!(verify_leaf_span(
        &s,
        empty_root(&s).unwrap(),
        0,
        &[],
        &[],
        &mut scratch,
        limits()
    )
    .is_err());
    let s = TreeSpec::new(StrongDigest::ZERO, 1, 1, Representation::Plaintext, 0, 1).unwrap();
    assert!(verify_leaf_span(
        &s,
        StrongDigest::ZERO,
        u64::MAX,
        &[0],
        &[&[0]],
        &mut scratch,
        limits()
    )
    .is_err());
    assert!(verify_leaf_span(
        &s,
        StrongDigest::ZERO,
        0,
        &[0],
        &[&[0]],
        &mut scratch,
        SpanLimits {
            max_leaves: MAX_SPAN_LEAVES + 1,
            max_encoded_bytes: 1
        }
    )
    .is_err());
}
