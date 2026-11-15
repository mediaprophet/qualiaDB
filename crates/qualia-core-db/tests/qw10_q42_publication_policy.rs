//! QW-10 acceptance: triple-only N-Quads → populated Q42LEX → full verify →
//! no public transport for review-gated access policy.

#![cfg(not(target_arch = "wasm32"))]

use std::io::Write;

use qualia_core_db::external_sort::ExternalSorter;
use qualia_core_db::parsers::nquads_star::parse_nquads_star_stream;
use qualia_core_db::q42_volume::{
    classify_q42_path, IngestAccessPolicy, IngestProvenanceRecord, PublicationIntent,
    Q42PublicationClass, Q42Volume, VerifyLevel, FLAG_PERMISSIVE_COMMONS,
};
use tempfile::TempDir;

#[test]
fn qw10_triple_only_nquads_lex_verify_and_transport_refusal() {
    let dir = TempDir::new().unwrap();
    let nq = dir.path().join("civics-review.nq");
    {
        let mut f = std::fs::File::create(&nq).unwrap();
        // Triple-only N-Quads (default graph) — previously InvalidSyntax.
        writeln!(
            f,
            "<https://civics.au/dataset/1> <https://civics.au/term/title> <https://civics.au/literal/Example> ."
        )
        .unwrap();
        writeln!(
            f,
            "<https://civics.au/dataset/1> <https://civics.au/term/licence> <https://civics.au/status/review-required> ."
        )
        .unwrap();
        writeln!(
            f,
            "<https://civics.au/dataset/1> <http://purl.org/dc/terms/publisher> \"Australian Institute of Health and Welfare\" ."
        )
        .unwrap();
        writeln!(
            f,
            "<https://civics.au/dataset/1> <https://civics.au/term/label> \"Communauté\"@fr ."
        )
        .unwrap();
        writeln!(
            f,
            "<https://civics.au/dataset/1> <https://civics.au/term/version> \"3\"^^<http://www.w3.org/2001/XMLSchema#integer> ."
        )
        .unwrap();
    }

    let out = dir.path().join("civics-review.q42");
    let mut sorter = ExternalSorter::new(dir.path().join("sort"));
    let reader = std::fs::File::open(&nq).unwrap();
    let triples = parse_nquads_star_stream(reader, 0, &mut sorter).expect("parse N-Quads");
    assert_eq!(triples, 5);

    let policy = IngestAccessPolicy::PublicNotForRedistribution;
    let prov = IngestProvenanceRecord::new(
        policy,
        "deadbeef",
        "civics-mapping-v1",
        "ctx-digest-test",
        "0.0.39-test",
    );
    sorter
        .merge_with_access_policy(&out, policy, Some(&prov))
        .expect("merge");

    let volume = Q42Volume::open(&out).unwrap();
    assert_eq!(
        volume.header().flags & FLAG_PERMISSIVE_COMMONS,
        0,
        "review-gated release must not set permissive-commons"
    );
    let lex = volume.lex_view().unwrap();
    assert!(
        lex.entry_count() >= 9,
        "Q42LEX must embed resolvable terms, got {}",
        lex.entry_count()
    );

    let receipt =
        qualia_core_db::q42_volume::Q42VerifyReceipt::from_volume(&out, &volume, VerifyLevel::Full);
    assert!(
        !matches!(
            receipt.overall,
            qualia_core_db::q42_volume::CheckStatus::Incomplete
                | qualia_core_db::q42_volume::CheckStatus::Fail
        ),
        "full verify must not be Incomplete/Fail: {:?}",
        receipt
    );

    let verdict = classify_q42_path(&out, PublicationIntent::Default).unwrap();
    assert!(
        !verdict.may_emit_public_magnet && !verdict.may_http_webseed && !verdict.may_ipfs_pin,
        "review-gated volume must refuse public transport: {:?}",
        verdict
    );
    assert_ne!(
        verdict.class,
        Q42PublicationClass::PermissiveCommons,
        "must not classify as permissive-commons"
    );

    let prov_path = qualia_core_db::q42_volume::provenance_path_for(&out);
    assert!(prov_path.is_file(), "missing {}", prov_path.display());
    let body = std::fs::read_to_string(&prov_path).unwrap();
    assert!(body.contains("public-not-for-redistribution"));
    assert!(body.contains("civics-mapping-v1"));
}

#[test]
fn qw10_natural_agent_sanctuary_claims_set_flag_sanctuary_and_reject_permissive_commons() {
    use qualia_core_db::q42_volume::{
        publication, StreamingQ42VolumeWriter, FLAG_SANCTUARY,
    };
    use qualia_core_db::{q_hash, NQuin};
    use std::collections::HashMap;

    let dir = TempDir::new().unwrap();
    let out = dir.path().join("natural-agent-sanctuary.q42");

    let mut writer = StreamingQ42VolumeWriter::new(&HashMap::new()).unwrap();
    // Caller attempts to force FLAG_PERMISSIVE_COMMONS while writing NaturalAgent personal/clinical/guardianship claims:
    writer.declare_permissive_commons();

    let mut personal_claim = NQuin {
        subject: q_hash("urn:qualia:entity:natural-agent:person-1"),
        predicate: q_hash("https://qualia.world/ontology/sanctuary#PersonalClaim"),
        object: q_hash("urn:qualia:claim:selfhood:1"),
        context: 0,
        metadata: 0,
        parity: 0,
    };
    personal_claim.recalculate_parity();

    let mut clinical_claim = NQuin {
        subject: q_hash("urn:qualia:entity:natural-agent:person-1"),
        predicate: q_hash("https://qualia.world/ontology/clinical#ClinicalClaim"),
        object: q_hash("urn:qualia:claim:clinical:diagnosis-1"),
        context: 0,
        metadata: 0,
        parity: 0,
    };
    clinical_claim.set_sensitivity_byte(NQuin::SENSITIVITY_CLASSIFIED);
    clinical_claim.set_sensitivity_tier(NQuin::SENSITIVITY_TIER_MEDICAL);
    clinical_claim.recalculate_parity();

    let mut guardianship_claim = NQuin {
        subject: q_hash("urn:qualia:entity:natural-agent:person-1"),
        predicate: q_hash("https://qualia.world/ontology/idf#hasGuardian"),
        object: q_hash("urn:qualia:entity:natural-agent:guardian-1"),
        context: 0,
        metadata: 0,
        parity: 0,
    };
    guardianship_claim.recalculate_parity();

    let mut claims = [personal_claim, clinical_claim, guardianship_claim];
    claims.sort_by_key(|q| q.object);
    writer.push_block(0, &claims).unwrap();
    writer.finish(&out).unwrap();

    let volume = Q42Volume::open(&out).unwrap();
    assert_ne!(
        volume.header().flags & FLAG_SANCTUARY,
        0,
        "volume carrying NaturalAgent personal/clinical/guardianship claims must set FLAG_SANCTUARY (0x0040)"
    );
    assert_eq!(
        volume.header().flags & FLAG_PERMISSIVE_COMMONS,
        0,
        "StreamingQ42VolumeWriter must strip caller's attempted FLAG_PERMISSIVE_COMMONS when sanctuary claims are present"
    );

    // Must be hard-rejected by publication::check_public_distribution_allowed under both Default and CommonsCatalog intents:
    let err_default = publication::check_public_distribution_allowed(&volume)
        .expect_err("FLAG_SANCTUARY volume must be rejected by check_public_distribution_allowed");
    assert_eq!(err_default.kind(), std::io::ErrorKind::PermissionDenied);

    let err_commons = publication::check_public_distribution_allowed((
        out.as_path(),
        PublicationIntent::CommonsCatalog,
    ))
    .expect_err("CommonsCatalog intent must not override FLAG_SANCTUARY");
    assert_eq!(err_commons.kind(), std::io::ErrorKind::PermissionDenied);

    // Even if a caller forcibly stamps FLAG_PERMISSIVE_COMMONS into the volume header bytes alongside FLAG_SANCTUARY,
    // publication::check_public_distribution_allowed must still hard-reject it:
    drop(volume);
    let mut forged_bytes = std::fs::read(&out).unwrap();
    let existing_flags = u16::from_le_bytes(forged_bytes[6..8].try_into().unwrap());
    let forged_flags = existing_flags | FLAG_PERMISSIVE_COMMONS;
    forged_bytes[6..8].copy_from_slice(&forged_flags.to_le_bytes());
    let forged_path = dir.path().join("natural_agent_sanctuary_forged_commons.q42");
    std::fs::write(&forged_path, &forged_bytes).unwrap();
    let forged_vol = Q42Volume::open(&forged_path).unwrap();
    assert_ne!(forged_vol.header().flags & FLAG_SANCTUARY, 0);
    assert_ne!(forged_vol.header().flags & FLAG_PERMISSIVE_COMMONS, 0);
    let err_forged = publication::check_public_distribution_allowed(&forged_vol)
        .expect_err("forged FLAG_PERMISSIVE_COMMONS + FLAG_SANCTUARY volume must be rejected");
    assert_eq!(err_forged.kind(), std::io::ErrorKind::PermissionDenied);

    let verdict = classify_q42_path(&forged_path, PublicationIntent::CommonsCatalog).unwrap();
    assert_eq!(verdict.class, Q42PublicationClass::MixedFailClosed);
    assert!(!verdict.may_emit_public_magnet);
    assert!(!verdict.may_http_webseed);
    assert!(!verdict.may_ipfs_pin);
}
