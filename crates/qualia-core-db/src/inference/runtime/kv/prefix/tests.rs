use super::*;
use crate::inference::runtime::graph_assist::{PrefixIdentity, PrefixPageSet};
use crate::inference::runtime::kv::paged::{BlockPool, CopyOnWrite, SequenceBlockTable};

#[test]
fn registry_ownership_survives_request_attach_and_eviction() {
    let mut pool = BlockPool::new(8);
    let p0 = pool.allocate().unwrap();
    let p1 = pool.allocate().unwrap();
    let identity = PrefixIdentity { words: [7; 4] };
    let set = PrefixPageSet::<4>::new(identity, &[p0, p1], 32).unwrap();
    let mut store = PrefixKvStore::<2, 4>::new();
    store.publish(set, &mut pool).unwrap();
    assert_eq!(pool.ref_count(p0), Some(2)); // producer + registry

    let mut request = SequenceBlockTable::new(4);
    assert_eq!(
        store.attach(identity, &mut request, &mut pool).unwrap(),
        Some(32)
    );
    assert_eq!(pool.ref_count(p0), Some(3));
    store.remove(identity, &mut pool).unwrap();
    assert_eq!(pool.ref_count(p0), Some(2)); // producer + active request
    request.release_all(&mut pool).unwrap();
    assert_eq!(pool.ref_count(p0), Some(1));
}

#[test]
fn attached_prefix_writes_use_copy_on_write() {
    let mut pool = BlockPool::new(4);
    let shared = pool.allocate().unwrap();
    let identity = PrefixIdentity { words: [9; 4] };
    let mut store = PrefixKvStore::<1, 2>::new();
    store
        .publish(
            PrefixPageSet::new(identity, &[shared], 16).unwrap(),
            &mut pool,
        )
        .unwrap();
    let mut request = SequenceBlockTable::new(2);
    store.attach(identity, &mut request, &mut pool).unwrap();
    let action = request.ensure_writable(0, &mut pool).unwrap();
    assert!(matches!(
        action,
        CopyOnWrite::Copy {
            source,
            destination: _
        } if source == shared
    ));
}
