//! Backend-independent terrain seam acceptance fixtures.
//!
//! These tests model the projected pixels along a shared tile edge rather than relying on
//! adapter readback. That makes the seam contract useful on CI and on targets without a native
//! GPU, while the existing adapter-backed pixel tests remain responsible for rasterizer and
//! backend qualification.

use crate::domains::geospatial::dem::{generate_terrain_mesh, TerrainMesh};

const SEAM_PIXEL_COUNT: usize = 256;
const HEIGHT_TOLERANCE: f32 = 3.0e-5;
const POSITION_TOLERANCE: f32 = 3.0e-5;

#[derive(Clone, Copy)]
enum Edge {
    West,
    East,
    South,
    North,
}

#[derive(Clone, Copy, Debug)]
struct EdgePixel {
    world: [f32; 3],
}

fn fixture_height(x: f64, z: f64) -> f32 {
    // The mixed frequencies exercise interpolation at sub-cell pixel positions without relying
    // on a renderer, image asset, or backend-specific floating-point result.
    (0.7 * (x * 0.31).sin() * (z * 0.23).cos()
        + 0.21 * (x * 0.17 + z * 0.11).sin()
        + 0.04 * x
        - 0.03 * z) as f32
}

fn tile(
    origin_x: f64,
    origin_z: f64,
    extent_x: f64,
    extent_z: f64,
    columns: usize,
    rows: usize,
) -> TerrainMesh {
    assert!(columns >= 2 && rows >= 2);
    let step_x = extent_x / (columns - 1) as f64;
    let step_z = extent_z / (rows - 1) as f64;
    let heights: Vec<f32> = (0..rows)
        .flat_map(|row| {
            (0..columns).map(move |column| {
                fixture_height(
                    origin_x + column as f64 * step_x,
                    origin_z + row as f64 * step_z,
                )
            })
        })
        .collect();
    let dem = generate_terrain_mesh(&heights, columns, rows, step_x);

    // `generate_terrain_mesh` centres a width*cell_size grid. Add the half-cell correction and
    // the declared extent so the output is expressed in the same world frame as the fixture.
    let positions = dem
        .vertices
        .into_iter()
        .map(|point| {
            [
                point[0] + (origin_x + extent_x * 0.5 + step_x * 0.5) as f32,
                point[2],
                point[1] + (origin_z + extent_z * 0.5 + step_z * 0.5) as f32,
            ]
        })
        .collect();
    TerrainMesh {
        vertices: positions,
        indices: dem.indices,
    }
}

fn edge_pixel(mesh: &TerrainMesh, columns: usize, rows: usize, edge: Edge, u: f32) -> EdgePixel {
    let (axis_length, fixed_index) = match edge {
        Edge::West | Edge::East => (rows, if matches!(edge, Edge::West) { 0 } else { columns - 1 }),
        Edge::South | Edge::North => (columns, if matches!(edge, Edge::South) { 0 } else { rows - 1 }),
    };
    let coordinate = u.clamp(0.0, 1.0) * (axis_length - 1) as f32;
    let lower = coordinate.floor() as usize;
    let upper = (lower + 1).min(axis_length - 1);
    let fraction = coordinate - lower as f32;
    let vertex = |along: usize| {
        let index = match edge {
            Edge::West | Edge::East => along * columns + fixed_index,
            Edge::South | Edge::North => fixed_index * columns + along,
        };
        mesh.vertices[index]
    };
    let a = vertex(lower);
    let b = vertex(upper);
    EdgePixel {
        world: core::array::from_fn(|axis| a[axis] + (b[axis] - a[axis]) * fraction),
    }
}

fn assert_shared_edge(
    first: &TerrainMesh,
    first_edge: Edge,
    second: &TerrainMesh,
    second_edge: Edge,
    columns: usize,
    rows: usize,
    label: &str,
) {
    let mut max_height_error = 0.0_f32;
    let mut max_position_error = 0.0_f32;
    for pixel in 0..SEAM_PIXEL_COUNT {
        let u = (pixel as f32 + 0.5) / SEAM_PIXEL_COUNT as f32;
        let left = edge_pixel(first, columns, rows, first_edge, u).world;
        let right = edge_pixel(second, columns, rows, second_edge, u).world;
        let position_error = (left[0] - right[0]).abs().max((left[2] - right[2]).abs());
        let height_error = (left[1] - right[1]).abs();
        max_position_error = max_position_error.max(position_error);
        max_height_error = max_height_error.max(height_error);
        assert!(
            position_error <= POSITION_TOLERANCE,
            "{label} seam pixel {pixel} has a world-position gap of {position_error:.8}"
        );
        assert!(
            height_error <= HEIGHT_TOLERANCE,
            "{label} seam pixel {pixel} has a height gap of {height_error:.8}: left={left:?}, right={right:?}"
        );
    }
    assert_eq!(
        SEAM_PIXEL_COUNT, 256,
        "the fixture must keep broad pixel coverage rather than regressing to a small sample cap"
    );
    assert!(max_position_error <= POSITION_TOLERANCE);
    assert!(max_height_error <= HEIGHT_TOLERANCE);
}

#[test]
fn terrain_seam_pixel_oracle_covers_full_edges_across_resolutions() {
    for side in [9, 17, 33, 65] {
        let west = tile(0.0, 0.0, 16.0, 16.0, side, side);
        let east = tile(16.0, 0.0, 16.0, 16.0, side, side);
        assert_shared_edge(
            &west,
            Edge::East,
            &east,
            Edge::West,
            side,
            side,
            "east-west",
        );

        let south = tile(0.0, 0.0, 16.0, 16.0, side, side);
        let north = tile(0.0, 16.0, 16.0, 16.0, side, side);
        assert_shared_edge(
            &south,
            Edge::North,
            &north,
            Edge::South,
            side,
            side,
            "south-north",
        );
    }
}
