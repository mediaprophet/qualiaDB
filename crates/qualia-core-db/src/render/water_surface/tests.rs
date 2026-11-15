use super::*;
#[test]
fn shoreline_follows_mesh_outline_and_leaves_interior_free_of_foam() {
    let p = [
        [-1.0, 0.0, -1.0],
        [1.0, 0.0, -1.0],
        [1.0, 0.0, 1.0],
        [-1.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
    ];
    let triangles = [0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4];
    let mut distance = [9.0; 5];
    let mut scale = [0.0; 5];
    let mut edges = [0; 17];
    let receipt =
        shore_distance_into(&p, &triangles, &mut distance, &mut scale, &mut edges).unwrap();
    assert_eq!(receipt.boundary_edges, 4);
    assert_eq!(distance, [0.0, 0.0, 0.0, 0.0, 1.0]);
}
#[test]
fn invalid_topology_does_not_change_distance_output() {
    let p = [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
    let mut distance = [9.0; 3];
    let mut scale = [9.0; 3];
    let mut edges = [0; 12];
    assert_eq!(
        shore_distance_into(
            &p,
            &[0, 1, 2, 0, 1, 2, 0, 1, 2],
            &mut distance,
            &mut scale,
            &mut edges
        ),
        Err(ShoreError::NonManifold)
    );
    assert_eq!(distance, [9.0; 3]);
    assert_eq!(scale, [9.0; 3]);
}
#[test]
fn overlapping_surfaces_keep_independent_shorelines_and_depth_scales() {
    let p = [
        [-1.0, 0.0, -1.0],
        [1.0, 0.0, -1.0],
        [1.0, 0.0, 1.0],
        [-1.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        [0.1, 2.0, 0.0],
        [0.0, 2.0, 0.1],
    ];
    let indices = [0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4, 5, 6, 7];
    let mut distance = [9.0; 8];
    let mut scale = [0.0; 8];
    let mut edges = [0; 23];
    shore_distance_into(&p, &indices, &mut distance, &mut scale, &mut edges).unwrap();
    assert_eq!(distance[4], 1.0);
    assert_eq!(&scale[..5], &[1.0; 5]);
    assert_eq!(&scale[5..], &[0.0; 3]);
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn shoreline_kernel_has_no_heap_allocations() {
    let p = [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
    let mut distance = [0.0; 3];
    let mut scale = [0.0; 3];
    let mut edges = [0; 6];
    crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc(
        "water_shoreline",
        || {
            shore_distance_into(&p, &[0, 1, 2], &mut distance, &mut scale, &mut edges).unwrap();
        },
    );
}
