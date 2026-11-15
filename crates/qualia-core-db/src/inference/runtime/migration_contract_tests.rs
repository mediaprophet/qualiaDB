//! Consuming checks for the canonical numerical/storage types behind old imports.
use super::{budget, graph_assist, kv};

#[test]
fn migration_kernel_configuration_and_budget_types_cross_public_imports() {
    let kernel: qualia_inference_kernel::paged_attention::PagedKvConfig =
        kv::paged::PagedKvConfig::new(2, 1, 2, 17).unwrap();
    let canonical: etalon_inference::runtime::kv::paged::PagedKvConfig = kernel;
    let mut legacy_table = [99; 6];
    let mut canonical_table = [99; 6];
    assert_eq!(
        kv::paged::fill_identity_block_table(&kernel, &mut legacy_table),
        Ok(4)
    );
    assert_eq!(
        etalon_inference::runtime::kv::paged::fill_identity_block_table(
            &canonical,
            &mut canonical_table
        ),
        Ok(4)
    );
    assert_eq!(legacy_table, [0, 1, 2, 3, 99, 99]);
    assert_eq!(canonical_table, legacy_table);
    let model: etalon_inference::runtime::budget::ModelMemoryProfile =
        budget::ModelMemoryProfile::new(40, 30, 20, 10).unwrap();
    let plan: etalon_inference::runtime::budget::MemoryPoolBudget = budget::MemoryPoolBudget::new(
        356,
        model,
        budget::BlockGeometry::new(16, 32).unwrap(),
        8,
        2,
    )
    .unwrap();
    let result: budget::RequestReservation = plan.reserve(17, 15, 1, 160).unwrap();
    assert_eq!((result.required_blocks, result.reserved_bytes), (2, 96));
}

#[test]
fn migration_graph_derived_identity_passes_directly_to_canonical_prefix_owners() {
    let facts = [crate::NQuin {
        subject: 1,
        predicate: 2,
        object: 3,
        context: 4,
        metadata: 5,
        parity: 4,
    }];
    let derived: graph_assist::PrefixIdentity =
        graph_assist::derive_prefix_identity(11, 22, 33, 44, &facts);
    // Executed immutable original identity.rs supplied these four words.
    assert_eq!(
        derived.words,
        [
            3212527960042339854,
            3823670575526963825,
            10391133995467941483,
            3640244517614164343
        ]
    );
    let canonical: etalon_inference::runtime::kv::prefix::PrefixIdentity = derived;
    let mut pool: kv::paged::BlockPool = etalon_inference::runtime::kv::paged::BlockPool::new(2);
    let page = pool.allocate().unwrap();
    let set: etalon_inference::runtime::kv::prefix::PrefixPageSet<2> =
        graph_assist::PrefixPageSet::new(derived, &[page], 16).unwrap();
    let mut registry = graph_assist::PrefixPageRegistry::<2, 2>::new();
    registry.insert(set).unwrap();
    assert_eq!(registry.get(canonical).unwrap().page_slice(), &[page]);
    let mut owner = etalon_inference::runtime::kv::prefix::PrefixKvStore::<2, 2>::new();
    owner.publish(set, &mut pool).unwrap();
    let mut request = kv::paged::SequenceBlockTable::new(2);
    assert_eq!(owner.attach(derived, &mut request, &mut pool), Ok(Some(16)));
    let changed = graph_assist::derive_prefix_identity(11, 22, 33, 45, &facts);
    assert_ne!(changed, derived);
    assert!(registry.get(changed).is_none());
    assert!(owner.get(changed).is_none());
    assert_eq!(pool.ref_count(page), Some(3));
    owner.remove(canonical, &mut pool).unwrap();
    assert_eq!(request.get(0), Some(page));
    assert_eq!(pool.ref_count(page), Some(2));
    request.release_all(&mut pool).unwrap();
    pool.release(page).unwrap();
    assert_eq!(pool.free_count(), 2);
}
