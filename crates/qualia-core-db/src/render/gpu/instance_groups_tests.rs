use super::*;
const IDENTITY: [[f32; 4]; 4] = [
    [1., 0., 0., 0.],
    [0., 1., 0., 0.],
    [0., 0., 1., 0.],
    [0., 0., 0., 1.],
];
fn triangle() -> [[f32; 3]; 3] {
    [[-0.2, -0.2, 0.5], [0.2, -0.2, 0.5], [0., 0.2, 0.5]]
}
fn record(x: f32, id: u64) -> GpuInstanceRecord {
    let mut m = IDENTITY;
    m[3][0] = x;
    GpuInstanceRecord::new(m, id)
}

#[test]
fn workload_receipt_matches_actual_split_material_draws_after_culling() {
    let mut groups = compile_groups(
        &[0, 6, 0, 3, 6, 3, 3, 2],
        &triangle(),
        &[0, 1, 2, 0, 1, 2, 0, 1, 2],
        5,
    )
    .unwrap();
    groups[0].visible = 0..1;
    groups[1].visible = 1..3;
    let submitted: u64 = [(0, 3), (3, 6)]
        .into_iter()
        .flat_map(|(first, count)| draw_ranges(&groups, first, count, 3, true))
        .map(|(indices, instances)| {
            u64::from((indices.end - indices.start) / 3)
                * u64::from(instances.end - instances.start)
        })
        .sum();
    assert_eq!(triangle_workload(&groups, 3, 5, 3), (8, submitted));
    assert_eq!(submitted, 4);
    for group in &mut groups {
        group.visible = 0..0;
    }
    assert_eq!(triangle_workload(&groups, 3, 5, 0), (8, 0));
}

#[test]
fn ordinary_single_prototype_and_empty_scene_counts_use_actual_stream_counts() {
    assert_eq!(triangle_workload(&[], 7, 4, 2), (28, 14));
    assert_eq!(triangle_workload(&[], 7, 4, 0), (28, 0));
    assert_eq!(triangle_workload(&[], 0, 0, 0), (0, 0));
}
#[test]
fn partition_is_exact_and_bounds_come_from_geometry() {
    let groups = compile_groups(
        &[0, 3, 0, 1, 3, 3, 1, 2],
        &triangle(),
        &[0, 1, 2, 2, 1, 0],
        3,
    )
    .unwrap();
    assert_eq!(groups[0].bounds_min, [-0.2, -0.2, 0.5]);
    assert_eq!(groups[1].source, 1..3);
}
#[test]
fn malformed_partitions_refuse_before_submission() {
    for words in [
        &[0, 3, 0, 1, 0, 3, 1, 1][..],
        &[0, 3, 0, 1, 3, 3, 0, 1],
        &[0, 3, 0, 1],
        &[0, 4, 0, 2],
        &[0, u32::MAX, 0, 2],
        &[0, 6, 0, 0],
    ] {
        assert!(compile_groups(words, &triangle(), &[0, 1, 2, 0, 1, 2], 2).is_err());
    }
}
#[test]
fn bad_geometry_is_not_a_culling_hint() {
    assert!(compile_groups(&[0, 3, 0, 1], &triangle(), &[0, 1, 9], 1).is_err());
    let mut p = triangle();
    p[0][1] = f32::NAN;
    assert!(compile_groups(&[0, 3, 0, 1], &p, &[0, 1, 2], 1).is_err());
}
#[test]
fn material_draw_spanning_prototypes_is_split_without_duplication() {
    let groups = compile_groups(
        &[0, 3, 0, 1, 3, 3, 1, 2],
        &triangle(),
        &[0, 1, 2, 0, 1, 2],
        3,
    )
    .unwrap();
    let draws: Vec<_> = draw_ranges(&groups, 0, 6, 3, true).collect();
    assert_eq!(draws, vec![(0..3, 0..1), (3..6, 1..3)]);
}
#[test]
fn native_visibility_preserves_source_semantics_and_shadow_casters() {
    let mut groups = compile_groups(
        &[0, 3, 0, 2, 3, 3, 2, 2],
        &triangle(),
        &[0, 1, 2, 0, 1, 2],
        4,
    )
    .unwrap();
    let source = [
        record(0., 10),
        record(10., 11),
        record(20., 12),
        record(0., 13),
    ];
    let mut indices = [0; 4];
    let mut output = [record(0., 0); 4];
    let count = compact_group_visibility(
        &mut groups,
        &source,
        IDENTITY,
        IDENTITY,
        &mut indices,
        &mut output,
    )
    .unwrap();
    assert_eq!(count, 2);
    assert_eq!(
        output[..2]
            .iter()
            .map(|r| r.semantic_id())
            .collect::<Vec<_>>(),
        vec![10, 13]
    );
    assert_eq!(groups[0].visible, 0..1);
    assert_eq!(groups[1].visible, 1..2);
    assert_eq!(
        draw_ranges(&groups, 0, 6, 4, false).collect::<Vec<_>>(),
        vec![(0..3, 0..2), (3..6, 2..4)]
    );
}
#[test]
fn empty_visible_groups_emit_no_draws() {
    let mut groups = compile_groups(&[0, 3, 0, 1], &triangle(), &[0, 1, 2], 1).unwrap();
    let mut output = [record(0., 0)];
    assert_eq!(
        compact_group_visibility(
            &mut groups,
            &[record(10., 42)],
            IDENTITY,
            IDENTITY,
            &mut [0],
            &mut output
        )
        .unwrap(),
        0
    );
    assert_eq!(draw_ranges(&groups, 0, 3, 1, true).count(), 0);
    assert_eq!(draw_ranges(&groups, 0, 3, 1, false).count(), 1);
}
#[test]
fn legacy_shared_stream_keeps_its_draw_contract() {
    assert_eq!(
        draw_ranges(&[], 3, 9, 5, true).collect::<Vec<_>>(),
        vec![(3..12, 0..5)]
    );
}
#[test]
fn transformed_world_bounds_cover_every_instance() {
    let groups = compile_groups(&[0, 3, 0, 2], &triangle(), &[0, 1, 2], 2).unwrap();
    let (min, max) = world_bounds(&groups, &[record(-10., 1), record(20., 2)]).unwrap();
    assert_eq!(min, [-10.2, -0.2, 0.5]);
    assert_eq!(max, [20.2, 0.2, 0.5]);
}
#[test]
fn short_visibility_scratch_fails_before_mutating_ranges() {
    let mut groups = compile_groups(&[0, 3, 0, 2], &triangle(), &[0, 1, 2], 2).unwrap();
    let mut output = [record(0., 0); 2];
    assert!(compact_group_visibility(
        &mut groups,
        &[record(0., 1); 2],
        IDENTITY,
        IDENTITY,
        &mut [0],
        &mut output
    )
    .is_err());
    assert_eq!(groups[0].visible, 0..2);
}
