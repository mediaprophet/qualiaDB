use super::*;
fn spec(indexing: GridIndexing) -> GridSpec {
    GridSpec {
        dimensions: [5, 3, 2],
        brick_dimensions: [2, 2, 2],
        spacing_m: [0.5, 2.0, 3.0],
        indexing,
    }
}
fn grid_metadata(spec: GridSpec, temporal: bool) -> Vec<Meta> {
    let mut m = metadata(0, temporal);
    replace(&mut m, 2, 5, numbers(5, &spec.spacing_m));
    replace(
        &mut m,
        2,
        6,
        f(
            6,
            1,
            3,
            &spec
                .dimensions
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        ),
    );
    if let Some(order) = spec.indexing.common_order() {
        replace(&mut m, 2, 7, n32(7, 1, order));
    }
    replace(&mut m, 4, 7, s(7, spec.indexing.name()));
    replace(
        &mut m,
        4,
        8,
        f(
            8,
            2,
            3,
            &spec
                .dimensions
                .iter()
                .flat_map(|v| u64::from(*v).to_le_bytes())
                .collect::<Vec<_>>(),
        ),
    );
    replace(
        &mut m,
        4,
        9,
        f(
            9,
            1,
            3,
            &spec
                .brick_dimensions
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        ),
    );
    replace(&mut m, 4, 10, n32(10, 1, spec.indexing.order()));
    m
}
pub(super) fn grid_payload(spec: GridSpec, key: u64, times: u64) -> Vec<u8> {
    let brick = spec.validate().unwrap().brick(key).unwrap();
    let mut out = Vec::new();
    for time in 0..times {
        for index in 0..brick.cell_count() {
            let tick = if times > 1 { -2 + time as i64 * 2 } else { 0 };
            let mut record = field_payload(tick);
            let p = brick.local_position_m(index).unwrap();
            for axis in 0..3 {
                put(&mut record, 48 + axis * 8, p[axis]);
            }
            put(&mut record, 0, -3.0 - index as f64);
            out.extend_from_slice(&record);
        }
    }
    out
}
#[test]
fn independent_directory_key_orientation_and_clipped_xyz_cases() {
    let cube = GridSpec {
        dimensions: [2; 3],
        brick_dimensions: [1; 3],
        spacing_m: [1.0; 3],
        indexing: GridIndexing::Hilbert { order: 1 },
    }
    .validate()
    .unwrap();
    let expected = [
        [0, 0, 0],
        [0, 0, 1],
        [0, 1, 1],
        [0, 1, 0],
        [1, 1, 0],
        [1, 1, 1],
        [1, 0, 1],
        [1, 0, 0],
    ];
    for (key, coordinate) in expected.into_iter().enumerate() {
        assert_eq!(cube.decode_key(key as u64).unwrap(), coordinate);
        assert_eq!(cube.encode_key(coordinate).unwrap(), key as u64);
    }
    let linear = GridSpec {
        dimensions: [4; 3],
        brick_dimensions: [1; 3],
        spacing_m: [1.0; 3],
        indexing: GridIndexing::Linear,
    }
    .validate()
    .unwrap();
    let morton = GridSpec {
        indexing: GridIndexing::Morton { order: 2 },
        ..linear.spec()
    }
    .validate()
    .unwrap();
    assert_eq!(linear.encode_key([2, 1, 0]).unwrap(), 6);
    assert_eq!(morton.encode_key([2, 1, 0]).unwrap(), 10);
    let brick = spec(GridIndexing::Linear)
        .validate()
        .unwrap()
        .brick(5)
        .unwrap();
    assert_eq!(brick.origin_cells(), [4, 2, 0]);
    assert_eq!(brick.shape(), [1, 1, 2]);
    assert_eq!(brick.global_cell(0).unwrap(), [4, 2, 0]);
    assert_eq!(brick.global_cell(1).unwrap(), [4, 2, 1]);
    assert_eq!(brick.local_position_m(0).unwrap(), [2.0, 4.0, 0.0]);
    assert_eq!(brick.local_position_m(1).unwrap(), [2.0, 4.0, 3.0]);
}
#[test]
fn all_three_declared_orders_bind_real_grid_field_chunks() {
    for indexing in [
        GridIndexing::Linear,
        GridIndexing::Morton { order: 2 },
        GridIndexing::Hilbert { order: 2 },
    ] {
        let spec = spec(indexing);
        let key = spec.validate().unwrap().encode_key([2, 1, 0]).unwrap();
        let payload = grid_payload(spec, key, 1);
        let mut row = row(0, &payload, 2, false);
        row.flags |= 4;
        row.spatial_key = key;
        let encoded = encode(&grid_metadata(spec, false), &[row]);
        let view =
            StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
        let chunk = admit(&view).unwrap();
        assert_eq!(chunk.spatial_key(), Some(key));
        assert_eq!(chunk.brick_layout().unwrap().shape(), [1, 1, 2]);
        assert_eq!(
            chunk.field(0).unwrap().unwrap().position_m(),
            [2.0, 4.0, 0.0]
        );
        assert_eq!(
            chunk.field(1).unwrap().unwrap().position_m(),
            [2.0, 4.0, 3.0]
        );
        assert_eq!(chunk.verified_stored_bytes(), payload);
    }
}
#[test]
fn temporal_grid_rows_are_time_z_y_x_and_wrong_order_is_rejected() {
    let spec = spec(GridIndexing::Hilbert { order: 2 });
    let key = spec.validate().unwrap().encode_key([2, 1, 0]).unwrap();
    let payload = grid_payload(spec, key, 2);
    let mut row = row(0, &payload, 4, true);
    row.flags |= 4;
    row.spatial_key = key;
    row.time_count = 2;
    let encoded = encode(&grid_metadata(spec, true), &[row]);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let chunk = admit(&view).unwrap();
    assert_eq!(
        (0..4)
            .map(|i| chunk.field(i).unwrap().unwrap().tick())
            .collect::<Vec<_>>(),
        vec![-2, -2, 0, 0]
    );
    let mut wrong = payload.clone();
    wrong[128..256].copy_from_slice(&payload[256..384]);
    let mut row = EntryInput {
        payload: &wrong,
        ..row
    };
    let encoded = encode(&grid_metadata(spec, true), &[row]);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    assert!(admit(&view).is_err());
    row.time_count = 4;
    let encoded = encode(&grid_metadata(spec, true), &[row]);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    assert!(admit(&view).is_err());
}
#[test]
fn malformed_keys_orders_extents_and_grid_positions_fail_closed() {
    for indexing in [
        GridIndexing::Linear,
        GridIndexing::Morton { order: 2 },
        GridIndexing::Hilbert { order: 2 },
    ] {
        let layout = spec(indexing).validate().unwrap();
        assert!(layout.decode_key(u64::MAX).is_err());
        assert!(layout.encode_key([3, 0, 0]).is_err());
    }
    assert!(spec(GridIndexing::Hilbert { order: 22 })
        .validate()
        .is_err());
    assert!(spec(GridIndexing::Morton { order: 0 }).validate().is_err());
    assert_eq!(
        GridSpec {
            dimensions: [u32::MAX; 3],
            ..spec(GridIndexing::Linear)
        }
        .validate(),
        Err(Error::Overflow)
    );
    let spec = spec(GridIndexing::Linear);
    let mut payload = grid_payload(spec, 5, 1);
    put(&mut payload, 48, 99.0);
    let mut row = row(0, &payload, 2, false);
    row.flags |= 4;
    row.spatial_key = 5;
    let encoded = encode(&grid_metadata(spec, false), &[row]);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    assert_eq!(admit(&view).unwrap_err(), Error::Frame);
}

#[test]
fn sparse_morton_brick_keys_allow_large_full_cell_dimensions_only_with_family_admission() {
    let spec = GridSpec {
        dimensions: [4194304, 1, 1],
        brick_dimensions: [2, 1, 1],
        spacing_m: [0.25, 1.0, 1.0],
        indexing: GridIndexing::Morton { order: 21 },
    };
    let layout = spec.validate().unwrap();
    assert_eq!(layout.brick_grid_dimensions(), [2097152, 1, 1]);
    let key = layout.encode_key([2097151, 0, 0]).unwrap();
    let payload = grid_payload(spec, key, 1);
    let mut r = row(0, &payload, 2, false);
    r.flags |= 4;
    r.spatial_key = key;
    let encoded = encode(&grid_metadata(spec, false), &[r]);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let typed = admit(&view).unwrap();
    assert_eq!(
        typed.field(0).unwrap().unwrap().position_m(),
        [1048575.5, 0.0, 0.0]
    );
    let mut m = [semantics::NodeState::default(); 8];
    let mut e = [semantics::NodeState::default(); 1];
    assert!(matches!(
        semantics::admit_shared(
            &view,
            &semantics::SharedOnly,
            &mut Scratch {
                metadata: &mut m,
                entries: &mut e
            },
            semantics::Limits::default()
        ),
        Err(semantics::Error::UnsupportedDomain)
    ));
}
