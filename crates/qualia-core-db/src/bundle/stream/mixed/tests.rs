#![cfg(not(target_arch = "wasm32"))]
use super::*;
use crate::audio::qac::QacStorageOptions;
use crate::bundle::stream::descriptor::{EntryRecord, HASH_SCRATCH_BYTES};
use crate::bundle::stream::formats::{
    emf_present_id, p64_present_id, qac_present_id, AuthenticatedRange, EmfStreamView, FormatLimits,
    M64DirectoryView, P64StreamView, PayloadInterval, Q42AuthenticatedRangeSource, QacAdmitError,
    QacCompleteView, QacStreamView, TenDDirectoryView,
};
use crate::bundle::stream::index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits};
use crate::bundle::stream::plan::{evaluate, EntryFamily, Obligation, Readiness, RepresentationClass};
use crate::bundle::stream::policy::CLOSURE_PURPOSE;
use crate::bundle::stream::proof::{verify_leaf_span, SpanLimits, VerifiedLeafSpan};
use crate::bundle::stream::selection::SelectedPlainEntry;
use crate::container_10d::section::{SectionDescriptor, SECTION_DESCRIPTOR_SIZE};
use crate::net::qdnf::types::StrongDigest;
use crate::q42::field_envelope::Limits as EmfLimits;
use crate::q42::q42_volume::Q42RangeVolume;
const QAC_OPAQUE: &[u8] =
    include_bytes!("../../../q42/field_envelope/fixtures/candidate/qac_candidate-opaque.qac");
const QAC_PCM: &[u8] = include_bytes!(
    "../../../q42/field_envelope/fixtures/candidate/qac_candidate-pcm-signed-zero.qac"
);
const LEGACY_UA4Q: &[u8] = include_bytes!("../../../audio/qac/fixtures/q4au-v2-contract.qac");
const EMPTY_SECTION: SectionDescriptor = SectionDescriptor {
    section_type: 0,
    alignment_tier: 0,
    reserved16: 0,
    byte_offset: 0,
    byte_length: 0,
    stride: 0,
    element_count: 0,
    crc32c: 0,
};
fn ten_d_limits() -> FormatLimits {
    FormatLimits {
        max_entry_bytes: 16_384,
        max_rows: 8,
        max_metadata_bytes: 512,
        max_chunk_bytes: 4096,
    }
}
fn m64_limits() -> FormatLimits {
    FormatLimits {
        max_entry_bytes: 16_384,
        max_rows: 4,
        max_metadata_bytes: 512,
        max_chunk_bytes: 256,
    }
}
fn count_family(hmc: &MixedHmc, family: EntryFamily) -> usize {
    (0..hmc.len())
        .filter(|i| hmc.family(*i) == family)
        .count()
}
fn bind<'a>(
    hmc: &'a MixedHmc,
) -> (
    crate::crypto::network::authority::PinAuthority,
    crate::bundle::stream::policy::BoundClosure<'a>,
) {
    let (pins, _, pin, publisher) = match hmc.installed(CLOSURE_PURPOSE) {
        Ok(v) => v,
        Err(_) => panic!("installed"),
    };
    let bound = match hmc.bound(&pins, pin, publisher) {
        Ok(v) => v,
        Err(_) => panic!("bound"),
    };
    (pins, bound)
}
fn with_selected<R>(
    hmc: &MixedHmc,
    bound: &crate::bundle::stream::policy::BoundClosure<'_>,
    i: usize,
    run: impl FnOnce(SelectedPlainEntry<'_>, &VerifiedLeafSpan<'_>, &[u8]) -> R,
) -> R {
    let header = match admit_header(
        &hmc.source()[..64],
        hmc.source().len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    ) {
        Ok(h) => h,
        Err(_) => panic!("header"),
    };
    let mut entry_scratch = vec![EntryScratch::EMPTY; hmc.len()];
    let start = match usize::try_from(header.index_offset()) {
        Ok(s) => s,
        Err(_) => panic!("index offset"),
    };
    let index = match admit_index(
        &header,
        &hmc.source()[start..],
        IndexReadLimits::default(),
        &mut entry_scratch,
    ) {
        Ok(v) => v,
        Err(_) => panic!("index"),
    };
    let mut rows = [EntryRecord::EMPTY; 128];
    let mut framing = vec![0; HASH_SCRATCH_BYTES];
    let selected = match hmc.select(bound, i as u32, &index, &mut rows, &mut framing) {
        Ok(v) => v,
        Err(_) => panic!("select"),
    };
    let payload = hmc.payload(i);
    let mut sibling_scratch = [StrongDigest::ZERO; 32];
    let spec = match selected.tree_spec() {
        Ok(s) => s,
        Err(_) => panic!("tree spec"),
    };
    let span = match verify_leaf_span(
        &spec,
        selected.root(),
        0,
        payload,
        &[&[0]],
        &mut sibling_scratch,
        SpanLimits {
            max_leaves: 1,
            max_encoded_bytes: 16_384,
        },
    ) {
        Ok(v) => v,
        Err(_) => panic!("span"),
    };
    run(selected, &span, payload)
}
fn complete<'a>(
    selected: SelectedPlainEntry<'a>,
    span: &'a VerifiedLeafSpan<'_>,
    payload: &[u8],
) -> AuthenticatedRange<'a> {
    match AuthenticatedRange::new(selected, span, 0, payload.len()) {
        Ok(v) => v,
        Err(_) => panic!("range"),
    }
}
fn range_at<'a>(
    selected: SelectedPlainEntry<'a>,
    span: &'a VerifiedLeafSpan<'_>,
    offset: u64,
    length: usize,
) -> AuthenticatedRange<'a> {
    match AuthenticatedRange::new(selected, span, offset, length) {
        Ok(v) => v,
        Err(_) => panic!("range"),
    }
}
fn u32_le(bytes: &[u8], offset: usize) -> u32 {
    match bytes.get(offset..offset + 4).and_then(|s| <[u8; 4]>::try_from(s).ok()) {
        Some(b) => u32::from_le_bytes(b),
        None => panic!("u32"),
    }
}
fn admitted_present(
    hmc: &MixedHmc,
    bound: &crate::bundle::stream::policy::BoundClosure<'_>,
) -> Vec<u32> {
    let mut present = Vec::new();
    for i in 0..hmc.len() {
        let id = hmc.entry_id(i);
        match hmc.family(i) {
            EntryFamily::Opaque => {}
            EntryFamily::Qac => {
                with_selected(hmc, bound, i, |selected, span, payload| {
                    let view = match QacStreamView::admit(
                        complete(selected, span, payload),
                        QacStorageOptions::default(),
                    ) {
                        Ok(v) => v,
                        Err(_) => panic!("qac present admit"),
                    };
                    if let Some(pid) = qac_present_id(id, &view) {
                        present.push(pid);
                    }
                });
            }
            EntryFamily::Emf => {
                with_selected(hmc, bound, i, |selected, span, payload| {
                    let view = match EmfStreamView::admit(
                        complete(selected, span, payload),
                        EmfLimits::default(),
                    ) {
                        Ok(v) => v,
                        Err(_) => panic!("emf present admit"),
                    };
                    match view.select_region(0) {
                        Ok(_) => {}
                        Err(_) => panic!("emf region"),
                    }
                    if let Some(pid) = emf_present_id(id, &view) {
                        present.push(pid);
                    }
                });
            }
            EntryFamily::P64 => {
                with_selected(hmc, bound, i, |selected, span, payload| {
                    let view = match P64StreamView::admit(complete(selected, span, payload)) {
                        Ok(v) => v,
                        Err(_) => panic!("p64 present admit"),
                    };
                    if let Some(pid) = p64_present_id(id, &view) {
                        present.push(pid);
                    }
                });
            }
            EntryFamily::TenD | EntryFamily::Q42 | EntryFamily::M64 => {
                present.push(id);
            }
        }
    }
    present
}
#[test]
fn default_mixed_publishes_many_per_family() {
    let hmc = match default_mixed_entries() {
        Ok(v) => v,
        Err(_) => panic!("default mixed"),
    };
    assert!(count_family(&hmc, EntryFamily::TenD) >= 2);
    assert!(count_family(&hmc, EntryFamily::Emf) >= 2);
    assert!(count_family(&hmc, EntryFamily::Qac) >= 2);
    assert!(count_family(&hmc, EntryFamily::Q42) >= 1);
    assert!(count_family(&hmc, EntryFamily::M64) >= 1);
    assert!(count_family(&hmc, EntryFamily::P64) >= 1);
    assert!(count_family(&hmc, EntryFamily::Opaque) >= 1);
    assert_eq!(hmc.identity().entry_count() as usize, hmc.len());
    assert_eq!(hmc.len(), 10);
    assert_eq!(hmc.entry_id(0), MIXED_ID_TEND_A);
    assert_eq!(hmc.family(0), EntryFamily::TenD);
    assert_eq!(hmc.entry_id(2), MIXED_ID_EMF_OPAQUE);
    assert_eq!(hmc.family(2), EntryFamily::Emf);
    assert_eq!(hmc.entry_id(4), MIXED_ID_QAC_OPAQUE);
    assert_eq!(hmc.family(4), EntryFamily::Qac);
}
#[test]
fn each_family_admits_through_format_view() {
    let hmc = match default_mixed_entries() {
        Ok(v) => v,
        Err(_) => panic!("default mixed"),
    };
    let (_pins, bound) = bind(&hmc);
    for i in 0..hmc.len() {
        let id = hmc.entry_id(i);
        with_selected(&hmc, &bound, i, |selected, span, payload| {
            assert_eq!(selected.entry_id(), id);
            assert_eq!(selected.logical_length(), payload.len() as u64);
            match hmc.family(i) {
                EntryFamily::TenD => {
                    let table_off = u32_le(payload, 56) as u64;
                    let table_len =
                        u32_le(payload, 60) as usize * SECTION_DESCRIPTOR_SIZE;
                    let header = range_at(selected, span, 0, 64);
                    let table = range_at(selected, span, table_off, table_len);
                    let mut rows = [EMPTY_SECTION; 8];
                    let mut scratch = [PayloadInterval::default(); 8];
                    match TenDDirectoryView::admit(
                        header,
                        table,
                        ten_d_limits(),
                        &mut rows,
                        &mut scratch,
                    ) {
                        Ok(view) => assert!(!view.descriptors().is_empty()),
                        Err(_) => panic!("10d admit"),
                    }
                }
                EntryFamily::Qac => {
                    let range = complete(selected, span, payload);
                    match QacCompleteView::admit(range, QacStorageOptions::default()) {
                        Ok(_) => {}
                        Err(_) => panic!("qac complete"),
                    }
                    let view = match QacStreamView::admit(range, QacStorageOptions::default()) {
                        Ok(v) => v,
                        Err(_) => panic!("qac stream"),
                    };
                    match qac_present_id(id, &view) {
                        Some(pid) => assert_eq!(pid, id),
                        None => panic!("qac present"),
                    }
                }
                EntryFamily::Emf => {
                    let view = match EmfStreamView::admit(
                        complete(selected, span, payload),
                        EmfLimits::default(),
                    ) {
                        Ok(v) => v,
                        Err(_) => panic!("emf stream"),
                    };
                    match view.select_region(0) {
                        Ok(chunk) => assert!(!chunk.is_empty()),
                        Err(_) => panic!("emf region"),
                    }
                    match emf_present_id(id, &view) {
                        Some(pid) => assert_eq!(pid, id),
                        None => panic!("emf present"),
                    }
                }
                EntryFamily::M64 => {
                    let header = range_at(selected, span, 0, 64);
                    let directory = range_at(selected, span, 64, 64);
                    let chunk = range_at(selected, span, 128, 128);
                    let mut scratch = [PayloadInterval::default(); 4];
                    let view = match M64DirectoryView::admit(
                        header,
                        directory,
                        None,
                        m64_limits(),
                        8,
                        &mut scratch,
                    ) {
                        Ok(v) => v,
                        Err(_) => panic!("m64 admit"),
                    };
                    match view.selected_chunk(0, chunk) {
                        Ok(c) => assert_eq!(c.records().len(), 64),
                        Err(_) => panic!("m64 chunk"),
                    }
                }
                EntryFamily::Q42 => {
                    let ranges = [complete(selected, span, payload)];
                    let source = match Q42AuthenticatedRangeSource::new(
                        selected,
                        &ranges,
                        payload.len() as u64,
                        1,
                    ) {
                        Ok(v) => v,
                        Err(_) => panic!("q42 source"),
                    };
                    match Q42RangeVolume::open(source) {
                        Ok(volume) => assert!(volume.block_count() >= 1),
                        Err(_) => panic!("q42 volume"),
                    }
                }
                EntryFamily::P64 => {
                    let view = match P64StreamView::admit(complete(selected, span, payload)) {
                        Ok(v) => v,
                        Err(_) => panic!("p64 stream"),
                    };
                    match p64_present_id(id, &view) {
                        Some(pid) => assert_eq!(pid, id),
                        None => panic!("p64 present"),
                    }
                }
                EntryFamily::Opaque => {}
            }
        });
    }
}
#[test]
fn original_entry_bytes_preserved() {
    let hmc = match default_mixed_entries() {
        Ok(v) => v,
        Err(_) => panic!("default mixed"),
    };
    let (_pins, bound) = bind(&hmc);
    for i in 0..hmc.len() {
        with_selected(&hmc, &bound, i, |selected, span, payload| {
            let start = match usize::try_from(selected.offset()) {
                Ok(s) => s,
                Err(_) => panic!("offset"),
            };
            let len = match usize::try_from(selected.logical_length()) {
                Ok(n) => n,
                Err(_) => panic!("length"),
            };
            assert_eq!(&hmc.source()[start..start + len], payload);
            let leaf = match AuthenticatedRange::new(selected, span, 0, payload.len()) {
                Ok(v) => v,
                Err(_) => panic!("leaf"),
            };
            assert_eq!(leaf.bytes(), payload);
        });
    }
}
#[test]
fn inventory_compiles_with_geometry_edges() {
    let hmc = match default_mixed_entries() {
        Ok(v) => v,
        Err(_) => panic!("default mixed"),
    };
    let plan = match mixed_plan(&hmc) {
        Ok(v) => v,
        Err(_) => panic!("plan"),
    };
    let inventory = match plan.inventory() {
        Ok(v) => v,
        Err(_) => panic!("inventory"),
    };
    assert_eq!(inventory.len(), hmc.len());
    assert!(plan.geometry_id().is_some());
    assert!(plan.calibration_id().is_some());
    assert!(!plan.edges().is_empty());
    let (_pins, bound) = bind(&hmc);
    let present = admitted_present(&hmc, &bound);
    assert!(present.contains(&MIXED_ID_TEND_A));
    assert!(present.contains(&MIXED_ID_EMF_OPAQUE));
    assert!(present.contains(&MIXED_ID_QAC_OPAQUE));
    assert!(present.contains(&MIXED_ID_QAC_PCM));
    assert!(present.contains(&MIXED_ID_Q42));
    assert!(present.contains(&MIXED_ID_P64));
    assert_eq!(
        evaluate(
            &inventory,
            MIXED_ID_TEND_A,
            Some(EntryFamily::TenD),
            &present
        ),
        Readiness::Ready
    );
    assert_eq!(
        evaluate(
            &inventory,
            MIXED_ID_EMF_OPAQUE,
            Some(EntryFamily::Emf),
            &present
        ),
        Readiness::Ready
    );
    assert_eq!(
        evaluate(
            &inventory,
            MIXED_ID_QAC_OPAQUE,
            Some(EntryFamily::Qac),
            &present
        ),
        Readiness::Ready
    );
    assert_eq!(
        evaluate(
            &inventory,
            MIXED_ID_QAC_PCM,
            Some(EntryFamily::Qac),
            &present
        ),
        Readiness::Ready
    );
}
#[test]
fn missing_mandatory_calibration_is_incomplete() {
    let hmc = match default_mixed_entries() {
        Ok(v) => v,
        Err(_) => panic!("default mixed"),
    };
    let plan = match mixed_plan(&hmc) {
        Ok(v) => v,
        Err(_) => panic!("plan"),
    };
    let calib = match plan.calibration_id() {
        Some(id) => id,
        None => panic!("calibration"),
    };
    let inventory = match plan.inventory() {
        Ok(v) => v,
        Err(_) => panic!("inventory"),
    };
    let (_pins, bound) = bind(&hmc);
    let present: Vec<u32> = admitted_present(&hmc, &bound)
        .into_iter()
        .filter(|id| *id != calib)
        .collect();
    assert!(!present.contains(&calib));
    let r = evaluate(
        &inventory,
        MIXED_ID_EMF_OPAQUE,
        Some(EntryFamily::Emf),
        &present,
    );
    assert_eq!(
        r,
        Readiness::Incomplete {
            missing_mandatory: calib
        }
    );
    match r {
        Readiness::Incomplete { missing_mandatory } => {
            assert_eq!(missing_mandatory, calib)
        }
        Readiness::Ready | Readiness::Opaque | Readiness::Refused => {
            panic!("missing calibration is not silence, empty field, or a complete answer")
        }
    }
}
#[test]
fn duplicate_entry_id_refuses() {
    let a = MixedEntry {
        id: MIXED_ID_QAC_OPAQUE,
        family: EntryFamily::Qac,
        role: RepresentationClass::Base,
        obligation: Obligation::Mandatory,
        bytes: QAC_OPAQUE,
    };
    let b = MixedEntry {
        id: MIXED_ID_QAC_OPAQUE,
        family: EntryFamily::Qac,
        role: RepresentationClass::Base,
        obligation: Obligation::Mandatory,
        bytes: QAC_PCM,
    };
    match publish_mixed(&[a, b]) {
        Err(MixedError::DuplicateId) => {}
        Err(_) => panic!("expected DuplicateId"),
        Ok(_) => panic!("duplicate qac id published"),
    }
}
#[test]
fn legacy_ua4q_as_qac_entry_refuses_admit() {
    let published = match publish_mixed(&[MixedEntry {
        id: 99,
        family: EntryFamily::Qac,
        role: RepresentationClass::Base,
        obligation: Obligation::Mandatory,
        bytes: LEGACY_UA4Q,
    }]) {
        Ok(v) => v,
        Err(_) => panic!("legacy publish"),
    };
    let (_pins, bound) = bind(&published);
    with_selected(&published, &bound, 0, |selected, span, payload| {
        match QacCompleteView::admit(
            complete(selected, span, payload),
            QacStorageOptions::default(),
        ) {
            Ok(_) => panic!("legacy ua4q admitted as qac"),
            Err(QacAdmitError::Family | QacAdmitError::Extent | QacAdmitError::Envelope(_)) => {}
            Err(_) => panic!("legacy refuse"),
        }
    });
}
