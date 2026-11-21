//! Synthetic first-paint producer tests. No website assets.
#![cfg(not(target_arch = "wasm32"))]
use super::*;
use crate::bundle::stream::publish::{
    build_first_paint_hmc, publish_first_paint, CandidateIdentityHex, FirstPaintError,
    FirstPaintSource,
};
use crate::bundle::{BundleError, BundleReader, BUNDLE_MAGIC};
use crate::net::qdnf::types::StrongDigest;

fn digest_hex(text: &str) -> StrongDigest {
    StrongDigest::from_bytes(&hex::decode(text).unwrap()).unwrap()
}

fn two_leaves() -> [FirstPaintSource<'static>; 2] {
    [
        FirstPaintSource {
            key: "leaf-a.10d",
            bytes: b"tiny-10d-a",
        },
        FirstPaintSource {
            key: "leaf-b.10d",
            bytes: b"tiny-10d-b\x00\x01",
        },
    ]
}

#[test]
fn empty_sources_are_rejected() {
    assert!(matches!(
        publish_first_paint(&[]),
        Err(FirstPaintError::EmptySources)
    ));
    assert!(matches!(
        build_first_paint_hmc(&[]),
        Err(FirstPaintError::EmptySources)
    ));
}

#[test]
fn duplicate_keys_are_rejected() {
    let sources = [
        FirstPaintSource {
            key: "same.10d",
            bytes: b"one",
        },
        FirstPaintSource {
            key: "same.10d",
            bytes: b"two",
        },
    ];
    assert!(matches!(
        build_first_paint_hmc(&sources),
        Err(FirstPaintError::Bundle(BundleError::DuplicateKey(_)))
    ));
}

#[test]
fn qbdl_hmc_keeps_kind_10d_and_inner_bytes() {
    let sources = two_leaves();
    let hmc = build_first_paint_hmc(&sources).unwrap();
    assert_eq!(&hmc[..4], &BUNDLE_MAGIC);
    assert_eq!(u16::from_le_bytes([hmc[6], hmc[7]]), 0);
    let reader = BundleReader::parse(&hmc).unwrap();
    assert!(!reader.is_page_aligned_4096());
    assert_eq!(reader.entries().len(), 2);
    for source in &sources {
        let entry = reader.entry(source.key).unwrap();
        assert_eq!(entry.kind, "10d");
        assert_eq!(reader.get(source.key).unwrap(), source.bytes);
        assert!(reader.verify_entry(source.key));
    }
}

#[test]
fn inventory_verifier_finish_yields_nonzero_hex_four_tuple() {
    let sources = two_leaves();
    let identity = publish_first_paint(&sources).unwrap();
    assert_eq!(identity.descriptor_sha384.len(), 96);
    assert_eq!(identity.context_sha384.len(), 96);
    assert_ne!(identity.descriptor_sha384, "0".repeat(96));
    assert_ne!(identity.context_sha384, "0".repeat(96));
    assert_eq!(identity.generation, 0);
    assert_eq!(identity.entries, 2);
    CandidateIdentity::from_parts(
        digest_hex(&identity.descriptor_sha384),
        digest_hex(&identity.context_sha384),
        identity.generation,
        identity.entries,
    )
    .unwrap();
}

#[test]
fn same_sources_are_deterministic_and_payload_changes_identity() {
    let sources = two_leaves();
    let a = publish_first_paint(&sources).unwrap();
    let b = publish_first_paint(&sources).unwrap();
    assert_eq!(a, b);
    let changed = [
        sources[0],
        FirstPaintSource {
            key: "leaf-b.10d",
            bytes: b"tiny-10d-b-changed",
        },
    ];
    let c = publish_first_paint(&changed).unwrap();
    assert_ne!(a.descriptor_sha384, c.descriptor_sha384);
    assert_ne!(a.context_sha384, c.context_sha384);
}

#[test]
fn sidecar_json_omits_zeros_and_keeps_streaming_off() {
    let identity = CandidateIdentityHex {
        descriptor_sha384: "ab".repeat(48),
        context_sha384: "cd".repeat(48),
        generation: 0,
        entries: 2,
    };
    let text = identity.sidecar_json();
    assert!(text.contains("\"status\": \"inventory-verifier-finish\""));
    assert!(text.contains("\"inner_ranges\": false"));
    assert!(text.contains("\"streaming_default\": false"));
    assert!(text.contains("\"browser_inventory_verifier\": false"));
    assert!(text.contains("CandidateIdentity::from_parts"));
    assert!(!text.contains(&"0".repeat(96)));
    assert!(!text.contains("ZIP"));
}
