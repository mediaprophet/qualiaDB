//! Governed Q42 asset envelope and licence policy (AST-01).
//!
//! Cold-construction schema for upstream release identity, digests, formats,
//! record counts, parser/mapping versions, provenance, licence obligations,
//! sensitivity, validation, and bounded chunk plans. Unknown licences fail
//! closed. Derived assets inherit the union of upstream obligations.

mod codec;
mod envelope;
mod error;
mod licence;

pub use codec::ASSET_ENVELOPE_MAGIC;
pub use envelope::{
    sha256_into, sha256_of, AssetRoutingLane, AssetSensitivity, ChunkSpec, Q42AssetEnvelope,
    RecordCounts, ToolchainVersions, UpstreamRelease, ASSET_ENVELOPE_VERSION, MAX_ENVELOPE_BYTES,
    MAX_DERIVED_FROM, MAX_NAMESPACES, MAX_REJECTION_REASONS, SENTINEL_PASS_BUDGET_BYTES,
};
pub use error::AssetEnvelopeError;
pub use licence::{
    LicenceClass, LicenceObligations, LicencePolicy, RedistributionClass, UseClass,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_envelope() -> Q42AssetEnvelope {
        let payload = b"CHEBI:15377 water fixture";
        Q42AssetEnvelope {
            asset_id: "did:q42:asset:chebi:261".into(),
            upstream: UpstreamRelease {
                source_name: "ChEBI".into(),
                release_id: "rel261".into(),
                source_url: "https://www.ebi.ac.uk/chebi/".into(),
                retrieved_unix: 1_725_000_000,
                byte_length: payload.len() as u64,
                sha256: sha256_of(payload),
            },
            licence: LicencePolicy::from_tag(
                "CC-BY-4.0",
                "https://creativecommons.org/licenses/by/4.0/",
                "ChEBI consortium",
            )
            .unwrap(),
            toolchain: ToolchainVersions {
                parser_version: "chebi-tsv-0.1".into(),
                mapping_version: "chebi-quin-0.1".into(),
            },
            raw_format: "tsv".into(),
            media_type: "text/tab-separated-values".into(),
            counts: RecordCounts {
                source: 10,
                accepted: 9,
                quarantined: 1,
            },
            rejection_reasons: vec!["malformed row 4".into()],
            identifier_namespaces: vec!["CHEBI".into()],
            cross_reference_policy: "preserve-upstream-id".into(),
            evidence_grade: "ontology-identity".into(),
            citation: "https://doi.org/10.1093/nar/gkaa991".into(),
            curation_status: "release".into(),
            sensitivity: AssetSensitivity::Public,
            routing_lane: AssetRoutingLane::Commons,
            derived_from: vec![],
            shacl_profile: "q42:ChebiReleaseShape".into(),
            validation_report: "ok".into(),
            chunk_plan: vec![ChunkSpec {
                index: 0,
                byte_budget: 8 * 1024 * 1024,
                record_budget: 50_000,
            }],
        }
    }

    #[test]
    fn round_trip_preserves_envelope() {
        let original = sample_envelope();
        let bytes = original.encode().unwrap();
        let decoded = Q42AssetEnvelope::decode(&bytes).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn envelope_digest_is_deterministic() {
        let a = sample_envelope().envelope_digest().unwrap();
        let b = sample_envelope().envelope_digest().unwrap();
        assert_eq!(a, b);
        assert_ne!(a, [0u8; 32]);
    }

    #[test]
    fn payload_digest_verification() {
        let env = sample_envelope();
        let good = sha256_of(b"CHEBI:15377 water fixture");
        assert!(env.verify_payload_digest(&good).is_ok());
        let bad = sha256_of(b"tampered");
        assert_eq!(
            env.verify_payload_digest(&bad),
            Err(AssetEnvelopeError::DigestMismatch)
        );
    }

    #[test]
    fn invalid_magic_rejected() {
        let mut bytes = sample_envelope().encode().unwrap();
        bytes[0] = b'X';
        assert_eq!(
            Q42AssetEnvelope::decode(&bytes),
            Err(AssetEnvelopeError::InvalidMagic)
        );
    }

    #[test]
    fn unknown_licence_cannot_build_policy() {
        assert_eq!(
            LicencePolicy::try_new(
                LicenceClass::Unknown,
                UseClass::Research,
                RedistributionClass::NoRedistribution,
                "https://example.test",
                "x",
            ),
            Err(AssetEnvelopeError::UnknownLicence)
        );
    }

    #[test]
    fn derived_asset_inherits_stricter_obligations() {
        let raw = sample_envelope();
        let nc = LicencePolicy::from_tag(
            "CC-BY-NC-4.0",
            "https://creativecommons.org/licenses/by-nc/4.0/",
            "overlay",
        )
        .unwrap();
        let derived = raw
            .derive_with(
                "did:q42:asset:chebi:261:normalized",
                &nc,
                ToolchainVersions {
                    parser_version: "chebi-tsv-0.1".into(),
                    mapping_version: "chebi-quin-0.2".into(),
                },
                vec![ChunkSpec {
                    index: 0,
                    byte_budget: 4 * 1024 * 1024,
                    record_budget: 10_000,
                }],
            )
            .unwrap();
        assert_eq!(derived.licence.class, LicenceClass::CcByNc);
        assert!(!derived.licence.allows_redistribution());
        assert_eq!(derived.derived_from, vec![raw.asset_id]);
        let bytes = derived.encode().unwrap();
        let again = Q42AssetEnvelope::decode(&bytes).unwrap();
        assert_eq!(again.licence.obligations, derived.licence.obligations);
    }

    #[test]
    fn chunk_over_sentinel_budget_fails() {
        let mut env = sample_envelope();
        env.chunk_plan[0].byte_budget = SENTINEL_PASS_BUDGET_BYTES + 1;
        assert_eq!(env.validate(), Err(AssetEnvelopeError::ChunkBudgetExceeded));
    }

    #[test]
    fn sha256_into_matches_sha256_of() {
        let data = b"caller-buffered";
        let mut out = [0u8; 32];
        sha256_into(data, &mut out);
        assert_eq!(out, sha256_of(data));
    }
}
