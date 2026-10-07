//! Bounded mesh-normal generation for cold uploads and animated pose updates.
//!
//! Upload builds an area-weighted vertex-normal workspace and a CSR vertex-to-face
//! adjacency table. Pose updates then visit only faces incident to changed vertices;
//! all scratch vectors are preallocated. Authored normals remain an AG-04 extension.

pub(crate) struct MeshNormalWorkspace {
    adjacency_offsets: Vec<usize>,
    adjacent_faces: Vec<u32>,
    face_vertices: Vec<u32>,
    face_sums: Vec<[f64; 3]>,
    vertex_sums: Vec<[f64; 3]>,
    face_marks: Vec<u32>,
    vertex_marks: Vec<u32>,
    generation: u32,
    dirty_vertices: Vec<u32>,
    normals: Vec<[f32; 3]>,
}

impl MeshNormalWorkspace {
    /// Build the area-weighted vertex normals and adjacency in deterministic
    /// index order. Invalid, incomplete or nonfinite geometry is rejected.
    pub(crate) fn new(positions: &[[f32; 3]], indices: &[u32]) -> Result<Self, &'static str> {
        if positions.is_empty() {
            return Err("mesh normals need at least one position");
        }
        if indices.is_empty() || indices.len() % 3 != 0 {
            return Err("mesh normal indices must contain complete triangles");
        }
        if positions.iter().flatten().any(|v| !v.is_finite()) {
            return Err("mesh normal positions must be finite");
        }
        if indices.iter().any(|&i| i as usize >= positions.len()) {
            return Err("mesh normal index is out of range");
        }
        let face_count = indices.len() / 3;
        if face_count > u32::MAX as usize {
            return Err("mesh normal face count exceeds the ABI limit");
        }

        let mut adjacency_offsets = vec![0usize; positions.len() + 1];
        for &index in indices {
            adjacency_offsets[index as usize + 1] += 1;
        }
        for vertex in 0..positions.len() {
            adjacency_offsets[vertex + 1] += adjacency_offsets[vertex];
        }
        let mut adjacent_faces = vec![0u32; indices.len()];
        let mut cursors = adjacency_offsets[..positions.len()].to_vec();
        for (face, triangle) in indices.chunks_exact(3).enumerate() {
            for &index in triangle {
                let cursor = &mut cursors[index as usize];
                adjacent_faces[*cursor] = face as u32;
                *cursor += 1;
            }
        }

        let mut workspace = Self {
            adjacency_offsets,
            adjacent_faces,
            face_vertices: indices.to_vec(),
            face_sums: vec![[0.0; 3]; face_count],
            vertex_sums: vec![[0.0; 3]; positions.len()],
            face_marks: vec![0; face_count],
            vertex_marks: vec![0; positions.len()],
            generation: 0,
            dirty_vertices: Vec::with_capacity(positions.len()),
            normals: vec![[0.0, 0.0, 1.0]; positions.len()],
        };
        for (face, triangle) in indices.chunks_exact(3).enumerate() {
            let normal = face_normal(positions, triangle);
            workspace.face_sums[face] = normal;
            for &index in triangle {
                add(&mut workspace.vertex_sums[index as usize], normal);
            }
        }
        for (normal, sum) in workspace.normals.iter_mut().zip(&workspace.vertex_sums) {
            *normal = normalized(*sum);
        }
        Ok(workspace)
    }

    /// Update normals after a contiguous position span changes. Complexity is
    /// proportional to incident faces plus their vertices, not the full mesh.
    /// The position buffer must already contain the changed, finite positions.
    pub(crate) fn update_positions(
        &mut self,
        positions: &[[f32; 3]],
        start: usize,
        end: usize,
    ) -> Result<(), &'static str> {
        if positions.len() != self.vertex_sums.len() || start > end || end > positions.len() {
            return Err("mesh normal update span is out of range");
        }
        if positions[start..end]
            .iter()
            .flatten()
            .any(|v| !v.is_finite())
        {
            return Err("mesh normal update positions must be finite");
        }
        self.dirty_vertices.clear();
        if start == end {
            return Ok(());
        }
        self.next_generation();
        for vertex in start..end {
            self.mark_vertex(vertex);
            for adjacency in self.adjacency_offsets[vertex]..self.adjacency_offsets[vertex + 1] {
                let face = self.adjacent_faces[adjacency] as usize;
                if self.face_marks[face] == self.generation {
                    continue;
                }
                self.face_marks[face] = self.generation;
                let old = self.face_sums[face];
                for corner in 0..3 {
                    let index = self.face_vertices[face * 3 + corner] as usize;
                    sub(&mut self.vertex_sums[index], old);
                    self.mark_vertex(index);
                }
                let triangle = &self.face_vertices[face * 3..face * 3 + 3];
                let new = face_normal(positions, &triangle);
                self.face_sums[face] = new;
                for corner in 0..3 {
                    let index = self.face_vertices[face * 3 + corner] as usize;
                    add(&mut self.vertex_sums[index], new);
                }
            }
        }
        for &vertex in &self.dirty_vertices {
            self.normals[vertex as usize] = normalized(self.vertex_sums[vertex as usize]);
        }
        Ok(())
    }

    /// Sorted, unique vertices whose normal values changed in the last update.
    /// Sorting is in-place, reuses capacity, and lets callers coalesce GPU writes.
    pub(crate) fn sorted_dirty_vertices_and_normals(&mut self) -> (&[u32], &[[f32; 3]]) {
        self.dirty_vertices.sort_unstable();
        (&self.dirty_vertices, &self.normals)
    }

    pub(crate) fn normals(&self) -> &[[f32; 3]] {
        &self.normals
    }

    fn mark_vertex(&mut self, vertex: usize) {
        if self.vertex_marks[vertex] != self.generation {
            self.vertex_marks[vertex] = self.generation;
            self.dirty_vertices.push(vertex as u32);
        }
    }

    fn next_generation(&mut self) {
        if self.generation == u32::MAX {
            self.face_marks.fill(0);
            self.vertex_marks.fill(0);
            self.generation = 1;
        } else {
            self.generation += 1;
            if self.generation == 0 {
                self.generation = 1;
            }
        }
    }
}

fn face_normal(positions: &[[f32; 3]], triangle: &[u32]) -> [f64; 3] {
    let a = positions[triangle[0] as usize];
    let b = positions[triangle[1] as usize];
    let c = positions[triangle[2] as usize];
    let ab = [
        f64::from(b[0]) - f64::from(a[0]),
        f64::from(b[1]) - f64::from(a[1]),
        f64::from(b[2]) - f64::from(a[2]),
    ];
    let ac = [
        f64::from(c[0]) - f64::from(a[0]),
        f64::from(c[1]) - f64::from(a[1]),
        f64::from(c[2]) - f64::from(a[2]),
    ];
    [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ]
}

fn add(sum: &mut [f64; 3], value: [f64; 3]) {
    for axis in 0..3 {
        sum[axis] += value[axis];
    }
}

fn sub(sum: &mut [f64; 3], value: [f64; 3]) {
    for axis in 0..3 {
        sum[axis] -= value[axis];
    }
}

fn normalized(sum: [f64; 3]) -> [f32; 3] {
    let length_squared = sum[0] * sum[0] + sum[1] * sum[1] + sum[2] * sum[2];
    if length_squared > 1.0e-30 && length_squared.is_finite() {
        let inv_length = length_squared.sqrt().recip();
        [
            (sum[0] * inv_length) as f32,
            (sum[1] * inv_length) as f32,
            (sum[2] * inv_length) as f32,
        ]
    } else {
        [0.0, 0.0, 1.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normals_are_area_weighted_and_shared_at_vertices() {
        let positions = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let workspace = MeshNormalWorkspace::new(&positions, &[0, 1, 2, 0, 3, 1]).unwrap();
        let normals = workspace.normals();
        assert_eq!(normals.len(), positions.len());
        assert!(normals[0][1] > 0.0 && normals[0][2] > 0.0);
        assert!((normals[2][2] - 1.0).abs() < 1.0e-6);
        assert!(normals[3][1] > 0.0);
    }

    #[test]
    fn malformed_or_nonfinite_mesh_data_is_rejected() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        assert!(MeshNormalWorkspace::new(&positions, &[0, 1]).is_err());
        assert!(MeshNormalWorkspace::new(&positions, &[0, 1, 3]).is_err());
        assert!(MeshNormalWorkspace::new(
            &[[f32::NAN, 0.0, 0.0], positions[1], positions[2]],
            &[0, 1, 2]
        )
        .is_err());
    }

    #[test]
    fn pose_update_only_marks_incident_vertices_and_updates_the_normal() {
        let mut positions = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [9.0, 9.0, 9.0],
        ];
        let mut workspace = MeshNormalWorkspace::new(&positions, &[0, 1, 2]).unwrap();
        positions[2][2] = 1.0;
        workspace.update_positions(&positions, 2, 3).unwrap();
        let (dirty, _) = workspace.sorted_dirty_vertices_and_normals();
        assert_eq!(dirty, [0, 1, 2]);
        assert!(workspace.normals()[0][1] < 0.0);
        assert_eq!(workspace.normals()[3], [0.0, 0.0, 1.0]);
    }
}
