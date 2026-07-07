#[derive(Debug, Clone)]
pub struct TerrainMesh {
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

/// Generates a triangulated terrain mesh from a 2D array of height values.
/// The `heightfield` is expected to be in row-major order.
/// `cell_size` is the spatial distance between adjacent height samples.
pub fn generate_terrain_mesh(
    heightfield: &[f32],
    width: usize,
    height: usize,
    cell_size: f64,
) -> TerrainMesh {
    assert_eq!(
        heightfield.len(),
        width * height,
        "Heightfield length must match width * height"
    );

    let mut vertices = Vec::with_capacity(width * height);
    let mut indices = Vec::with_capacity((width - 1) * (height - 1) * 6);

    // offset so the center of the grid is at (0, 0)
    let offset_x = (width as f64 * cell_size) / 2.0;
    let offset_y = (height as f64 * cell_size) / 2.0;

    for y in 0..height {
        for x in 0..width {
            let px = (x as f64 * cell_size) - offset_x;
            let py = (y as f64 * cell_size) - offset_y;
            let pz = heightfield[y * width + x] as f64;
            vertices.push([px as f32, py as f32, pz as f32]);
        }
    }

    for y in 0..(height - 1) {
        for x in 0..(width - 1) {
            let i0 = (y * width + x) as u32;
            let i1 = i0 + 1;
            let i2 = ((y + 1) * width + x) as u32;
            let i3 = i2 + 1;

            // First triangle (bottom-left)
            indices.push(i0);
            indices.push(i2);
            indices.push(i1);

            // Second triangle (top-right)
            indices.push(i1);
            indices.push(i2);
            indices.push(i3);
        }
    }

    TerrainMesh { vertices, indices }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_terrain_mesh() {
        let width = 3;
        let height = 3;
        let heights = vec![0.0, 1.0, 0.0, 1.0, 2.0, 1.0, 0.0, 1.0, 0.0];

        let mesh = generate_terrain_mesh(&heights, width, height, 10.0);

        assert_eq!(mesh.vertices.len(), 9);
        // (width - 1) * (height - 1) * 2 triangles = 4 * 2 = 8 triangles = 24 indices
        assert_eq!(mesh.indices.len(), 24);

        // Center vertex should have height 2.0
        assert_eq!(mesh.vertices[4][2], 2.0);
    }
}
