//! QW-10 acceptance: triple-only N-Quads → populated Q42LEX → full verify →
//! no public transport for review-gated access policy.

#![cfg(not(target_arch = "wasm32"))]

use std::io::Write;

use qualia_core_db::external_sort::ExternalSorter;
use qualia_core_db::parsers::nquads_star::parse_nquads_star_stream;
use qualia_core_db::q42_volume::{
    classify_q42_path, IngestAccessPolicy, IngestProvenanceRecord, PublicationIntent, Q42Volume,
    Q42PublicationClass, VerifyLevel, FLAG_PERMISSIVE_COMMONS,
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

    let receipt = qualia_core_db::q42_volume::Q42VerifyReceipt::from_volume(
        &out,
        &volume,
        VerifyLevel::Full,
    );
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
        !verdict.may_emit_public_magnet
            && !verdict.may_http_webseed
            && !verdict.may_ipfs_pin,
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
