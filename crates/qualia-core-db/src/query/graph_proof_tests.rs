use super::graph_proof::{
    prove_cli_ntriples_q42_equivalence, GraphProofOptions, RdfIsomorphismStatus,
};
use super::mini_parser::hash_token;
use crate::q42_volume::UnifiedVolumeBuilder;
use tempfile::TempDir;

fn quin(subject: &str, predicate: &str, object: &str) -> crate::NQuin {
    let subject = hash_token(subject);
    let predicate = hash_token(predicate);
    let object = hash_token(object);
    crate::NQuin {
        subject,
        predicate,
        object,
        context: 0,
        metadata: 0,
        parity: crate::NQuin::calculate_parity(subject, predicate, object, 0, 0),
    }
}

fn options() -> GraphProofOptions {
    GraphProofOptions {
        memory_limit_bytes: 64,
        temporary_byte_budget: 1024 * 1024,
    }
}

#[test]
fn graph_proof_proves_equal_ground_graph_with_forced_disk_runs() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.nt");
    let q42 = dir.path().join("source.q42");
    std::fs::write(
        &source,
        "<urn:s2> <urn:p> <urn:o2> .\n<urn:s1> <urn:p> <urn:o1> .\n<urn:s1> <urn:p> <urn:o1> .\n",
    )
    .unwrap();
    let mut builder = UnifiedVolumeBuilder::with_empty_lex();
    let mut q42_quins = vec![
        quin("<urn:s1>", "<urn:p>", "<urn:o1>"),
        quin("<urn:s2>", "<urn:p>", "<urn:o2>"),
    ];
    q42_quins.sort_unstable_by_key(|quin| quin.object);
    builder.push_block(0, &q42_quins).unwrap();
    builder.finish(&q42).unwrap();

    let report = prove_cli_ntriples_q42_equivalence(&source, &q42, options()).unwrap();
    assert!(report.encoded_sets_match());
    assert_eq!(report.source_records, 3);
    assert_eq!(report.q42_records, 2);
    assert_eq!(report.source_unique_records, 2);
    assert_eq!(
        report.rdf_isomorphism,
        RdfIsomorphismStatus::GroundGraphProven
    );
}

#[test]
fn graph_proof_reports_exact_set_difference() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.nt");
    let q42 = dir.path().join("source.q42");
    std::fs::write(&source, "<urn:s> <urn:p> <urn:expected> .\n").unwrap();
    let mut builder = UnifiedVolumeBuilder::with_empty_lex();
    builder
        .push_block(0, &[quin("<urn:s>", "<urn:p>", "<urn:actual>")])
        .unwrap();
    builder.finish(&q42).unwrap();

    let report = prove_cli_ntriples_q42_equivalence(&source, &q42, options()).unwrap();
    assert!(!report.encoded_sets_match());
    assert_eq!(report.missing_from_q42, 1);
    assert_eq!(report.unexpected_in_q42, 1);
    assert!(report.first_missing.is_some());
    assert!(report.first_unexpected.is_some());
    assert_eq!(report.rdf_isomorphism, RdfIsomorphismStatus::Different);
}

#[test]
fn graph_proof_flags_blank_nodes_as_requiring_canonicalization() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source.nt");
    let q42 = dir.path().join("source.q42");
    std::fs::write(&source, "_:local <urn:p> <urn:o> .\n").unwrap();
    let mut builder = UnifiedVolumeBuilder::with_empty_lex();
    builder
        .push_block(0, &[quin("_:local", "<urn:p>", "<urn:o>")])
        .unwrap();
    builder.finish(&q42).unwrap();

    let report = prove_cli_ntriples_q42_equivalence(&source, &q42, options()).unwrap();
    assert!(report.encoded_sets_match());
    assert_eq!(
        report.rdf_isomorphism,
        RdfIsomorphismStatus::BlankNodeCanonicalizationRequired
    );
}
