use super::*;
fn identity(length: u64) -> StorageIdentity {
    StorageIdentity::new(IdentityFields {
        descriptor: StrongDigest([1; 48]),
        context: StrongDigest([2; 48]),
        entry: 3,
        generation: 4,
        representation: Representation::Plaintext,
        storage_class: StorageClass::PublicPlaintextCandidate,
        key_epoch: 0,
        logical_length: length,
        root: StrongDigest([5; 48]),
        privacy_partition: StrongDigest([6; 48]),
        policy_generation: 1,
        storage_generation: 1,
    })
    .unwrap()
}
#[test]
fn exact_checked_final_leaf_page_and_one_tib_cost_geometry() {
    let id = identity(1 << 40);
    let layout = StoreLayout::for_identity(&id).unwrap();
    assert_eq!(layout.range_limits().proof_bytes, 103146323968);
    assert_eq!(layout.range_limits().checkpoint_bytes, 18415616);
    assert_eq!(layout.aggregate_allowance().unwrap(), 1202676367360);
    assert_eq!(layout.checkpoint_offset(1, 0).unwrap(), 8992);
    assert!(layout.checkpoint_offset(0, 2).is_err());
    assert!(layout.checkpoint_offset(2048, 0).is_err());
    let final_id = identity(16385);
    let final_layout = StoreLayout::for_identity(&final_id).unwrap();
    assert_eq!(final_layout.leaf_offset(1).unwrap(), 16384);
    assert_eq!(final_id.tree_spec().encoded_leaf_length(1).unwrap(), 1);
    assert!(final_layout.leaf_offset(2).is_err());
    assert!(matches!(
        StoreLayout::for_identity(&identity(0)),
        Err(StoreError::Empty)
    ));
}
#[test]
fn new_facade_requires_exact_layout_and_shared_selected_binding() {
    fixture(
        |mut store, workspace, rt, pins, bound, f, payload, _driver, _quota, binding| {
            assert_eq!(binding.identity(), store.identity());
            assert_eq!(
                store.layout().range_limits().payload_bytes,
                payload.len() as u64
            );
            assert!(!workspace.has_integrity_mark(0).unwrap());
            assert!(write(&mut store, workspace, rt, pins, bound, f, payload, 1).is_ok());
            store
        },
    );
}

#[test]
fn actual_signed_empty_entry_selects_but_needs_no_staging_allowance() {
    use crate::bundle::stream::{
        descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
        index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
        policy::{tests::Fixture, CLOSURE_PURPOSE},
        selection::select_entry,
    };
    let f = Fixture::new_with_payload(&[]);
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let bound = f.bound(&pins, pin, publisher);
    let limits = IndexReadLimits::default();
    let h = admit_header(
        &f.source()[..64],
        f.source().len() as u64,
        ExtentLimit::default(),
        limits,
    )
    .unwrap();
    let mut scratch = [EntryScratch::EMPTY; 1];
    let index = admit_index(
        &h,
        &f.source()[h.index_offset() as usize..],
        limits,
        &mut scratch,
    )
    .unwrap();
    let mut rows = [EntryRecord::EMPTY; 128];
    let mut framing = vec![0; HASH_SCRATCH_BYTES];
    let selected = select_entry(
        &bound,
        f.body(),
        &f.source()[..64],
        &index,
        f.entry_page(),
        0,
        &mut rows,
        &mut framing,
    )
    .unwrap();
    let binding = SelectedStoreBinding::from_selected(
        &selected,
        StorePartition {
            privacy: StrongDigest([1; 48]),
            policy_generation: Generation(1),
            storage_generation: 1,
        },
    )
    .unwrap();
    assert_eq!(selected.tree_spec().unwrap().leaf_count(), 0);
    assert!(matches!(
        StoreLayout::for_binding(&binding),
        Err(StoreError::Empty)
    ));
}
