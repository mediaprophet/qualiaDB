//! Byte-exact candidate fixtures, retained for tests without private programme docs.
//! Provenance and original hashes: fixtures/manifest.json. Not release qualification.
pub(super) fn load(path: &str) -> &'static [u8] {
    match path {
        "candidate/emf_candidate-aligned4096.emf" => {
            include_bytes!("fixtures/candidate/emf_candidate-aligned4096.emf")
        }
        "candidate/emf_candidate-metadata-only.emf" => {
            include_bytes!("fixtures/candidate/emf_candidate-metadata-only.emf")
        }
        "candidate/emf_candidate-opaque.emf" => {
            include_bytes!("fixtures/candidate/emf_candidate-opaque.emf")
        }
        "candidate/emf_candidate-two-entries.emf" => {
            include_bytes!("fixtures/candidate/emf_candidate-two-entries.emf")
        }
        "candidate/qac_candidate-aligned4096.qac" => {
            include_bytes!("fixtures/candidate/qac_candidate-aligned4096.qac")
        }
        "candidate/qac_candidate-metadata-only.qac" => {
            include_bytes!("fixtures/candidate/qac_candidate-metadata-only.qac")
        }
        "candidate/qac_candidate-opaque.qac" => {
            include_bytes!("fixtures/candidate/qac_candidate-opaque.qac")
        }
        "candidate/qac_candidate-pcm-signed-zero.qac" => {
            include_bytes!("fixtures/candidate/qac_candidate-pcm-signed-zero.qac")
        }
        "candidate/qac_candidate-two-entries.qac" => {
            include_bytes!("fixtures/candidate/qac_candidate-two-entries.qac")
        }
        "malformed/emf_candidate-count-overflow.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-count-overflow.emf")
        }
        "malformed/emf_candidate-count-product-overflow.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-count-product-overflow.emf")
        }
        "malformed/emf_candidate-directory-crc.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-directory-crc.emf")
        }
        "malformed/emf_candidate-duplicate.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-duplicate.emf")
        }
        "malformed/emf_candidate-header-crc.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-header-crc.emf")
        }
        "malformed/emf_candidate-metadata-duplicate-id.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-metadata-duplicate-id.emf")
        }
        "malformed/emf_candidate-padding.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-padding.emf")
        }
        "malformed/emf_candidate-payload-digest-tail.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-payload-digest-tail.emf")
        }
        "malformed/emf_candidate-payload-overlap.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-payload-overlap.emf")
        }
        "malformed/emf_candidate-payload.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-payload.emf")
        }
        "malformed/emf_candidate-reordered.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-reordered.emf")
        }
        "malformed/emf_candidate-reserved-directory.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-reserved-directory.emf")
        }
        "malformed/emf_candidate-reserved-header.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-reserved-header.emf")
        }
        "malformed/emf_candidate-root-tail.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-root-tail.emf")
        }
        "malformed/emf_candidate-tlv-count.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-tlv-count.emf")
        }
        "malformed/emf_candidate-tlv-padding.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-tlv-padding.emf")
        }
        "malformed/emf_candidate-tlv-reserved.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-tlv-reserved.emf")
        }
        "malformed/emf_candidate-tlv-zero-key.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-tlv-zero-key.emf")
        }
        "malformed/emf_candidate-truncated.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-truncated.emf")
        }
        "malformed/emf_candidate-unknown-codec.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-unknown-codec.emf")
        }
        "malformed/emf_candidate-unknown-critical-schema.emf" => {
            include_bytes!("fixtures/malformed/emf_candidate-unknown-critical-schema.emf")
        }
        "malformed/qac_candidate-count-overflow.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-count-overflow.qac")
        }
        "malformed/qac_candidate-count-product-overflow.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-count-product-overflow.qac")
        }
        "malformed/qac_candidate-directory-crc.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-directory-crc.qac")
        }
        "malformed/qac_candidate-duplicate.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-duplicate.qac")
        }
        "malformed/qac_candidate-header-crc.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-header-crc.qac")
        }
        "malformed/qac_candidate-metadata-duplicate-id.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-metadata-duplicate-id.qac")
        }
        "malformed/qac_candidate-padding.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-padding.qac")
        }
        "malformed/qac_candidate-payload-digest-tail.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-payload-digest-tail.qac")
        }
        "malformed/qac_candidate-payload-overlap.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-payload-overlap.qac")
        }
        "malformed/qac_candidate-payload.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-payload.qac")
        }
        "malformed/qac_candidate-reordered.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-reordered.qac")
        }
        "malformed/qac_candidate-reserved-directory.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-reserved-directory.qac")
        }
        "malformed/qac_candidate-reserved-header.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-reserved-header.qac")
        }
        "malformed/qac_candidate-root-tail.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-root-tail.qac")
        }
        "malformed/qac_candidate-tlv-count.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-tlv-count.qac")
        }
        "malformed/qac_candidate-tlv-padding.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-tlv-padding.qac")
        }
        "malformed/qac_candidate-tlv-reserved.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-tlv-reserved.qac")
        }
        "malformed/qac_candidate-tlv-zero-key.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-tlv-zero-key.qac")
        }
        "malformed/qac_candidate-truncated.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-truncated.qac")
        }
        "malformed/qac_candidate-unknown-codec.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-unknown-codec.qac")
        }
        "malformed/qac_candidate-unknown-critical-schema.qac" => {
            include_bytes!("fixtures/malformed/qac_candidate-unknown-critical-schema.qac")
        }
        _ => panic!("unknown test fixture: {path}"),
    }
}
