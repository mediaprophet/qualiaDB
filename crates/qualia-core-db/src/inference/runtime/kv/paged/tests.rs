use super::*;

fn config() -> PagedKvConfig {
    PagedKvConfig::new(2, 4, 64, 64).unwrap()
}

#[test]
fn identity_gpu_table_matches_dense_page_order() {
    let config = config();
    let plan = GpuBlockTablePlan::identity(config).unwrap();
    assert_eq!(plan.entries(), &[0, 1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(plan.config().block_elems(), 16 * 2 * 4 * 64);
}

#[test]
fn caller_buffer_rejects_short_table() {
    let config = config();
    let mut out = [0u32; 7];
    assert_eq!(
        fill_identity_block_table(&config, &mut out),
        Err(TableError::OutputTooSmall)
    );
}

#[test]
fn fork_and_partial_page_write_use_copy_on_write() {
    let mut pool = BlockPool::new(4);
    let mut parent = SequenceBlockTable::new(2);
    let first = parent.ensure_writable(0, &mut pool).unwrap();
    let physical = match first {
        CopyOnWrite::Allocated(block) => block,
        other => panic!("unexpected allocation action: {other:?}"),
    };
    let mut child = SequenceBlockTable::new(2);
    parent.fork_into(&mut child, &mut pool).unwrap();
    assert_eq!(pool.ref_count(physical), Some(2));

    let action = child.ensure_writable(0, &mut pool).unwrap();
    let destination = match action {
        CopyOnWrite::Copy {
            source,
            destination,
        } => {
            assert_eq!(source, physical);
            destination
        }
        other => panic!("expected copy-on-write, got {other:?}"),
    };
    assert_ne!(destination, physical);
    assert_eq!(parent.get(0), Some(physical));
    assert_eq!(child.get(0), Some(destination));
    assert_eq!(pool.ref_count(physical), Some(1));
}

#[test]
fn release_is_bounded_and_detects_double_release() {
    let mut pool = BlockPool::new(2);
    let block = pool.allocate().unwrap();
    pool.release(block).unwrap();
    assert_eq!(pool.release(block), Err(PoolError::DoubleRelease));
    assert_eq!(pool.free_count(), 2);
}

#[test]
fn paged_online_gqa_matches_dense_reference_across_positions() {
    let mut config = PagedKvConfig::new(1, 2, 4, 32).unwrap();
    config.physical_blocks = 2;
    let block_elems = config.block_elems();
    let slot_kv = config.slot_kv_elems() as usize;
    let mut dense = vec![0.0f32; config.max_context as usize * slot_kv * 2];
    for (index, value) in dense.iter_mut().enumerate() {
        *value = (((index * 37 + 11) % 101) as f32 - 50.0) / 31.0;
    }
    // Reverse physical pages to exercise real indirection rather than identity addressing.
    let table = [1u32, 0u32];
    let mut arena = vec![0.0f32; block_elems * 2];
    for token in 0..config.max_context as usize {
        let logical = token / config.block_size as usize;
        let offset = token % config.block_size as usize;
        let src = token * slot_kv * 2;
        let dst = table[logical] as usize * block_elems + offset * slot_kv * 2;
        arena[dst..dst + slot_kv * 2].copy_from_slice(&dense[src..src + slot_kv * 2]);
    }
    let query: Vec<f32> = (0..16)
        .map(|index| (((index * 13 + 5) % 29) as f32 - 14.0) / 9.0)
        .collect();
    let mut paged = [0.0f32; 16];
    let mut identity = [0.0f32; 16];
    let identity_table = [0u32, 1u32];
    for &position in &[0, 1, 15, 16, 31] {
        paged_gqa_attention_into(&query, &arena, &table, &config, position, 4, &mut paged).unwrap();
        paged_gqa_attention_into(
            &query,
            &dense,
            &identity_table,
            &config,
            position,
            4,
            &mut identity,
        )
        .unwrap();
        for (index, (actual, expected)) in paged.iter().zip(identity).enumerate() {
            assert!(
                (actual - expected).abs() <= 2.0e-6,
                "position={position} index={index} paged={actual} dense={expected}"
            );
        }
    }
}

#[test]
fn graph_prefix_pages_install_with_refcounts_and_cow() {
    let mut pool = BlockPool::new(4);
    let first = pool.allocate().unwrap();
    let second = pool.allocate().unwrap();
    let mut request = SequenceBlockTable::new(4);
    request
        .install_shared_prefix(&[first, second], &mut pool)
        .unwrap();
    assert_eq!(pool.ref_count(first), Some(2));
    assert_eq!(pool.ref_count(second), Some(2));
    assert!(matches!(
        request.ensure_writable(1, &mut pool).unwrap(),
        CopyOnWrite::Copy { source, .. } if source == second
    ));
    assert_eq!(pool.ref_count(second), Some(1));
}
