//! Independent Node CBOR/SHA-2/CRC corpus, without the production writer.
use super::*;
use serde_json::Value;
fn hex(value: &Value) -> Vec<u8> {
    let text = value.as_str().unwrap();
    assert_eq!(text.len() % 2, 0);
    text.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(core::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn publisher_matches_independent_complete_source_and_paged_descriptor_vectors() {
    let corpus: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/qbdl-stream-conformance/fixtures/qasd-plain-v1.json"
    )))
    .unwrap();
    for fixture in corpus["fixtures"].as_array().unwrap() {
        let source = hex(&fixture["qbdl_source"]["complete_source_hex"]);
        let assignments: Vec<_> = fixture["qbdl_source"]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| EntryAssignment {
                id: e["entry_id"].as_u64().unwrap().try_into().unwrap(),
                inner_version: e["inner_version"].as_u64().unwrap(),
            })
            .collect();
        let p = &fixture["placeholders"];
        let options = PublisherOptions {
            parameters: ContextParameters {
                nonce: hex(&p["nonce_hex"]).try_into().unwrap(),
                generation: 0,
                predecessor: None,
                semantic_contract: StrongDigest(
                    hex(&p["semantic_contract_ref_hex"]).try_into().unwrap(),
                ),
                policy: StrongDigest(hex(&p["policy_ref_hex"]).try_into().unwrap()),
                consumer_profile: StrongDigest(
                    hex(&p["consumer_profile_ref_hex"]).try_into().unwrap(),
                ),
            },
            class: PublicationClass::PlainCandidate,
            metadata_limit: MAX_METADATA,
        };
        let out = publish_with_options(&source, &assignments, options);
        assert_eq!(
            out.context,
            hex(&fixture["context_root_hex"]),
            "{}",
            fixture["name"]
        );
        assert_eq!(
            out.body,
            hex(&fixture["descriptor_body_hex"]),
            "{}",
            fixture["name"]
        );
        assert_eq!(
            out.identity.context_id().0.as_slice(),
            hex(&fixture["context_id_sha384_hex"])
        );
        assert_eq!(
            out.identity.descriptor_id().0.as_slice(),
            hex(&fixture["descriptor_id_sha384_hex"])
        );
        for (actual, key) in [
            (&out.contexts, "context_pages"),
            (&out.entries, "entry_pages"),
        ] {
            let expected = fixture[key].as_array().unwrap();
            assert_eq!(actual.len(), expected.len());
            for (actual, expected) in actual.iter().zip(expected) {
                assert_eq!(*actual, hex(&expected["encoded_hex"]));
            }
        }
    }
}
