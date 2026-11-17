//! EWM-01b: from_parts, whole-entry verify_arrived_range, windowed inventory.

use super::*;
use crate::bundle::stream::descriptor::{CandidateIdentity, DescriptorError};
use crate::bundle::stream::plan::{
    EntryFamily, Inventory, Obligation, PlanEntry, PlanEntrySpec, RepresentationClass,
    ResourceEstimate,
};
use crate::bundle::writer::BundleWriter;
use crate::net::qdnf::types::StrongDigest;

fn digest(fill: u8) -> StrongDigest {
    StrongDigest([fill; 48])
}

#[test]
fn from_parts_rejects_zero_digests() {
    assert!(matches!(
        CandidateIdentity::from_parts(StrongDigest::ZERO, digest(1), 1, 10),
        Err(DescriptorError::Commitment)
    ));
    assert!(matches!(
        CandidateIdentity::from_parts(digest(1), StrongDigest::ZERO, 1, 10),
        Err(DescriptorError::Commitment)
    ));
    assert!(CandidateIdentity::from_parts(digest(1), digest(2), 1, 10).is_ok());
}

#[test]
fn whole_entry_sha256_via_planner_and_verify_arrived_range() {
    let payload = vec![0x11u8; 96];
    let mut writer = BundleWriter::new();
    writer
        .add_file("leaf.10d", "10d", payload.clone(), None)
        .unwrap();
    let bundle = writer.build().unwrap();
    let probe = HmcHeaderProbe::parse_header_range(&bundle[..BUNDLE_HEADER_SIZE]).unwrap();
    let footer =
        &bundle[probe.index_offset as usize..(probe.index_offset + probe.index_length) as usize];
    let planner = HmcProgressiveRangePlanner::from_header_and_index(probe, footer).unwrap();
    let reqs = planner.plan_entry_ranges();
    assert_eq!(reqs.len(), 1);
    assert!(reqs[0].expected_crc32c.is_none());
    let start = reqs[0].offset as usize;
    let end = start + reqs[0].length as usize;
    verify_arrived_range(&reqs[0], &bundle[start..end]).unwrap();
    assert_eq!(&bundle[start..end], payload.as_slice());
}

#[test]
fn interior_crc_is_not_v1_leaf_proof() {
    let payload = vec![0x22u8; 32];
    let mut writer = BundleWriter::new();
    writer.add_file("x.10d", "10d", payload, None).unwrap();
    let bundle = writer.build().unwrap();
    let probe = HmcHeaderProbe::parse_header_range(&bundle[..BUNDLE_HEADER_SIZE]).unwrap();
    let footer =
        &bundle[probe.index_offset as usize..(probe.index_offset + probe.index_length) as usize];
    let planner = HmcProgressiveRangePlanner::from_header_and_index(probe, footer).unwrap();
    assert!(
        planner.plan_entry_ranges()[0].expected_crc32c.is_none(),
        "v1 whole-entry SHA-256; CRC is not the leaf proof"
    );
}

#[test]
fn windowed_inventory_accepts_fewer_entries_than_candidate_count() {
    let candidate = CandidateIdentity::from_parts(digest(3), digest(4), 1, 78).unwrap();
    let spec = PlanEntrySpec {
        candidate,
        entry_id: 1,
        family: EntryFamily::TenD,
        role: RepresentationClass::Base,
        interval: None,
        timebase: None,
        estimate: ResourceEstimate::new(64, 0, 0),
        obligation: Obligation::Mandatory,
    };
    let entry = PlanEntry::new(spec).unwrap();
    let entries = [entry];
    let inventory = Inventory::compile(candidate, &entries, &[]).unwrap();
    assert_eq!(inventory.len(), 1);
    assert_eq!(candidate.entry_count(), 78);
}

#[test]
fn whole_file_hmc_crc_not_claimed_from_header_index_probe() {
    let mut writer = BundleWriter::new();
    writer
        .add_file("a.10d", "10d", vec![1, 2, 3, 4], None)
        .unwrap();
    let bundle = writer.build().unwrap();
    let probe = HmcHeaderProbe::parse_header_range(&bundle[..BUNDLE_HEADER_SIZE]).unwrap();
    assert!(probe.total_length > 0);
    // Header+index admit directory structure only; payload CRC remains incomplete.
}