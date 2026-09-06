//! AST-05 chebi_query tests — fixtures via chebi_map encoding.

use super::*;
use crate::q42::chebi_map::{map_records_into, MapBudgets, PRED_HAS_PARENT};
use crate::q42::chebi_parse::ChebiRecord;
use crate::{q_hash, NQuin};

fn rec(
    id: u64,
    accession: &str,
    name: &str,
    parent_id: Option<u64>,
    source_line: u64,
) -> ChebiRecord {
    ChebiRecord {
        id,
        status: "C".into(),
        accession: accession.into(),
        source: String::new(),
        parent_id,
        name: name.into(),
        definition: String::new(),
        source_line,
    }
}

fn budgets(max_quins: usize) -> MapBudgets {
    MapBudgets {
        max_quins,
        max_conflicts: 8,
    }
}

fn limits(max_hits: usize, max_depth: usize, max_export: usize) -> QueryLimits {
    QueryLimits {
        max_hits,
        max_depth,
        max_export_quins: max_export,
    }
}

fn map_fixture(records: &[ChebiRecord], release: &str) -> (Vec<NQuin>, usize) {
    let mut out = vec![NQuin::default(); 64];
    let report = map_records_into(records, release, budgets(64), &mut out).expect("map");
    out.truncate(report.quins_written);
    (out, report.quins_written)
}

#[test]
fn empty_input_resolve_fails_closed() {
    let err = resolve_chemical_into(
        &[],
        "CHEBI:1",
        "",
        limits(4, 2, 16),
        None,
        &mut [],
    )
    .unwrap_err();
    assert_eq!(err, QueryError::EmptyInput);
}

#[test]
fn resolve_known_accession_and_numeric_id() {
    let records = [
        rec(15377, "CHEBI:15377", "water", None, 2),
        rec(16236, "CHEBI:16236", "ethanol", Some(15377), 3),
    ];
    let (quins, _) = map_fixture(&records, "chebi-test-2026");
    let mut hits = vec![empty_hit(); 2];
    let n = resolve_chemical_into(
        &quins,
        "CHEBI:15377",
        "test-licence",
        limits(4, 2, 32),
        Some(&records),
        &mut hits,
    )
    .expect("resolve");
    assert_eq!(n, 1);
    assert_eq!(hits[0].accession, "CHEBI:15377");
    assert_eq!(hits[0].name_hash, q_hash("water"));
    assert_eq!(hits[0].source_line, 2);
    assert_eq!(hits[0].licence_note, "test-licence");
    assert_eq!(hits[0].uncertainty, Uncertainty::Known);
    assert!(hits[0].parent_hash.is_none());

    let n2 = resolve_chemical_into(
        &quins,
        "16236",
        "test-licence",
        limits(4, 2, 32),
        Some(&records),
        &mut hits,
    )
    .expect("numeric");
    assert_eq!(n2, 1);
    assert_eq!(hits[0].accession, "CHEBI:16236");
    assert_eq!(hits[0].parent_hash, Some(q_hash("CHEBI:15377")));
}

fn empty_hit() -> ChemicalHit {
    ChemicalHit {
        subject_hash: 0,
        name_hash: 0,
        parent_hash: None,
        release_hash: 0,
        source_line: 0,
        uncertainty: Uncertainty::Unknown,
        accession: String::new(),
        licence_note: String::new(),
    }
}

#[test]
fn missing_and_bad_query() {
    let records = [rec(1, "CHEBI:1", "a", None, 2)];
    let (quins, _) = map_fixture(&records, "rel");
    let mut hits = [empty_hit()];
    assert_eq!(
        resolve_chemical_into(&quins, "CHEBI:999", "", limits(2, 1, 8), None, &mut hits),
        Err(QueryError::NotFound)
    );
    assert_eq!(
        resolve_chemical_into(&quins, "", "", limits(2, 1, 8), None, &mut hits),
        Err(QueryError::EmptyQuery)
    );
    assert_eq!(
        resolve_chemical_into(&quins, "not-an-id", "", limits(2, 1, 8), None, &mut hits),
        Err(QueryError::EmptyQuery)
    );
}

#[test]
fn ambiguous_same_accession_two_releases() {
    let records = [rec(1, "CHEBI:1", "a", None, 2)];
    let (a, _) = map_fixture(&records, "release-a");
    let (b, _) = map_fixture(&records, "release-b");
    let mut merged = a;
    merged.extend(b);
    let mut hits = [empty_hit()];
    let err = resolve_chemical_into(
        &merged,
        "CHEBI:1",
        "",
        limits(4, 2, 32),
        None,
        &mut hits,
    )
    .unwrap_err();
    assert_eq!(err, QueryError::Ambiguous { hits: 2 });
}

#[test]
fn resolve_limit_exceeded_zero_capacity() {
    let records = [rec(1, "CHEBI:1", "a", None, 2)];
    let (quins, _) = map_fixture(&records, "rel");
    let err = resolve_chemical_into(
        &quins,
        "CHEBI:1",
        "",
        limits(4, 1, 8),
        None,
        &mut [],
    )
    .unwrap_err();
    assert_eq!(
        err,
        QueryError::LimitExceeded {
            limit: 0,
            needed: 1
        }
    );
}

#[test]
fn parent_relationship_and_children() {
    let records = [
        rec(1, "CHEBI:1", "parent", None, 2),
        rec(2, "CHEBI:2", "child", Some(1), 3),
    ];
    let (quins, _) = map_fixture(&records, "rel-p");
    let mut rels = vec![empty_rel(); 2];
    let n = lookup_parents_into(
        &quins,
        "CHEBI:2",
        "lic",
        limits(4, 2, 16),
        Some(&records),
        &mut rels,
    )
    .expect("parents");
    assert_eq!(n, 1);
    assert_eq!(rels[0].parent_hash, q_hash("CHEBI:1"));
    assert_eq!(rels[0].parent_accession, "CHEBI:1");
    assert_eq!(rels[0].licence_note, "lic");

    let n2 = lookup_children_into(
        &quins,
        "1",
        "lic",
        limits(4, 2, 16),
        Some(&records),
        &mut rels,
    )
    .expect("children");
    assert_eq!(n2, 1);
    assert_eq!(rels[0].child_hash, q_hash("CHEBI:2"));
    assert_eq!(rels[0].child_accession, "CHEBI:2");
}

fn empty_rel() -> RelationHit {
    RelationHit {
        child_hash: 0,
        parent_hash: 0,
        release_hash: 0,
        source_line: 0,
        uncertainty: Uncertainty::Unknown,
        child_accession: String::new(),
        parent_accession: String::new(),
        licence_note: String::new(),
    }
}

#[test]
fn evidence_includes_source_line_and_licence() {
    let records = [rec(5, "CHEBI:5", "named", None, 42)];
    let (quins, _) = map_fixture(&records, "release-label-x");
    let mut ev = [EvidenceHit {
        subject_hash: 0,
        release_hash: 0,
        source_line: 0,
        uncertainty: Uncertainty::Unknown,
        accession: String::new(),
        licence_note: String::new(),
    }];
    let n = lookup_evidence_into(&quins, "CHEBI:5", "caller-lic", limits(2, 1, 8), &mut ev)
        .expect("evidence");
    assert_eq!(n, 1);
    assert_eq!(ev[0].source_line, 42);
    assert_eq!(ev[0].release_hash, q_hash("release-label-x"));
    assert_eq!(ev[0].licence_note, "caller-lic");
}

#[test]
fn subgraph_depth_limit_excludes_grandparent() {
    // chain: 3 → 2 → 1
    let records = [
        rec(1, "CHEBI:1", "root", None, 2),
        rec(2, "CHEBI:2", "mid", Some(1), 3),
        rec(3, "CHEBI:3", "leaf", Some(2), 4),
    ];
    let (quins, _) = map_fixture(&records, "chain");
    let mut out = vec![NQuin::default(); 64];

    // depth 1 from leaf: leaf + mid, not root
    let n = export_subgraph_into(&quins, "CHEBI:3", limits(8, 1, 64), &mut out).expect("export");
    assert!(n > 0);
    let subjects: Vec<u64> = out[..n].iter().map(|q| q.subject).collect();
    assert!(subjects.contains(&q_hash("CHEBI:3")));
    assert!(subjects.contains(&q_hash("CHEBI:2")));
    assert!(!subjects.contains(&q_hash("CHEBI:1")));

    // depth 2 includes root
    let n2 = export_subgraph_into(&quins, "CHEBI:3", limits(8, 2, 64), &mut out).expect("d2");
    let subjects2: Vec<u64> = out[..n2].iter().map(|q| q.subject).collect();
    assert!(subjects2.contains(&q_hash("CHEBI:1")));
}

#[test]
fn subgraph_includes_parent_cross_ref_quin() {
    let records = [
        rec(1, "CHEBI:1", "parent", None, 2),
        rec(2, "CHEBI:2", "child", Some(1), 3),
    ];
    let (quins, _) = map_fixture(&records, "xr");
    let mut out = vec![NQuin::default(); 32];
    let n = export_subgraph_into(&quins, "CHEBI:2", limits(4, 1, 32), &mut out).expect("xr");
    let parent_pred = q_hash(PRED_HAS_PARENT);
    assert!(out[..n]
        .iter()
        .any(|q| q.predicate == parent_pred && q.subject == q_hash("CHEBI:2")));
}

#[test]
fn describe_release_licence_fields_from_catalogue() {
    let records = [
        rec(1, "CHEBI:1", "a", None, 2),
        rec(2, "CHEBI:2", "b", Some(1), 3),
    ];
    let (quins, written) = map_fixture(&records, "chebi-desc");
    let desc = describe_release(&quins, "chebi-desc", "");
    assert_eq!(desc.release_label, "chebi-desc");
    assert_eq!(desc.release_hash, q_hash("chebi-desc"));
    assert_eq!(desc.record_count, 2);
    assert_eq!(desc.quin_count, written);
    assert!(desc.licence_obligation_present);
    assert!(!desc.licence_note.is_empty());
    assert!(
        desc.licence_note.contains("CC BY") || desc.licence_note.to_ascii_lowercase().contains("licence"),
        "unexpected licence stub: {}",
        desc.licence_note
    );

    let empty = describe_release(&[], "missing-rel", "explicit-note");
    assert_eq!(empty.record_count, 0);
    assert_eq!(empty.quin_count, 0);
    assert_eq!(empty.licence_note, "explicit-note");
    assert!(empty.licence_obligation_present);
}

#[test]
fn invalid_limits_fail_closed() {
    assert_eq!(
        QueryLimits {
            max_hits: 0,
            max_depth: 1,
            max_export_quins: 1
        }
        .validate(),
        Err(QueryError::InvalidLimits)
    );
}
