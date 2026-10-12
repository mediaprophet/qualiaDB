/// Triangle soup the canvas2d tick can draw without a GPU adapter.
pub(super) struct CpuBodyMesh {
    pub(super) positions: Vec<[f32; 3]>,
    pub(super) colors: Vec<[f32; 4]>,
    pub(super) indices: Vec<u32>,
    /// Full scene vertex lineage survives fallback decimation.
    source_vertices: Vec<u32>,
}

impl CpuBodyMesh {
    pub(super) fn decimate(
        positions: &[[f32; 3]],
        colors: &[[f32; 4]],
        indices: &[u32],
        spans: &[(usize, usize)],
    ) -> Self {
        // Keep each organ's own outline. A single stride across the whole
        // scene turns a car, a person, and a shelter into the same confetti.
        const PER_ORGAN: usize = 220;
        let mut out_i = Vec::new();
        let mut out_positions = Vec::new();
        let mut out_colors = Vec::new();
        let mut source_vertices = Vec::new();
        let mut remap = vec![u32::MAX; positions.len()];
        let ranges: Vec<(usize, usize)> = if spans.is_empty() {
            vec![(0, indices.len())]
        } else {
            spans.to_vec()
        };
        for (start, end) in ranges {
            if end > indices.len() || start >= end {
                continue;
            }
            let tris = (end - start) / 3;
            let keep = tris.min(PER_ORGAN).max(1);
            let step = (tris / keep).max(1);
            let mut t = 0usize;
            let mut kept = 0usize;
            while t < tris && kept < PER_ORGAN {
                let b = start + t * 3;
                if b + 2 < end {
                    let mut valid = true;
                    let mut mapped = [0u32; 3];
                    for (corner, &source) in indices[b..b + 3].iter().enumerate() {
                        let source = source as usize;
                        if source >= positions.len() || source >= colors.len() {
                            valid = false;
                            break;
                        }
                        if remap[source] == u32::MAX {
                            let Ok(new_index) = u32::try_from(out_positions.len()) else {
                                valid = false;
                                break;
                            };
                            remap[source] = new_index;
                            out_positions.push(positions[source]);
                            out_colors.push(colors[source]);
                            source_vertices.push(source as u32);
                        }
                        mapped[corner] = remap[source];
                    }
                    if valid {
                        out_i.extend_from_slice(&mapped);
                        kept += 1;
                    }
                }
                t += step;
            }
        }
        Self {
            positions: out_positions,
            colors: out_colors,
            indices: out_i,
            source_vertices,
        }
    }
    /// Update a full-scene animated span through its retained source mapping.
    /// The compact mesh has different offsets and may omit some source vertices.
    pub(super) fn update_authored_span(&mut self,start:u32,positions:&[[f32;3]])->bool {
        let Some(end)=(start as usize).checked_add(positions.len()) else {return false;};
        if positions.iter().flatten().any(|v|!v.is_finite()) {return false;}
        for (i,source) in self.source_vertices.iter().enumerate() {
            let source=*source as usize;
            if source>=start as usize&&source<end {self.positions[i]=positions[source-start as usize];}
        }
        true
    }
}

#[cfg(test)]
mod cpu_body_mesh_tests {
    use super::CpuBodyMesh;

    #[test]
    fn full_scene_updates_follow_source_vertices_after_reordering() {
        let positions:Vec<[f32;3]>=(0..9).map(|i|[i as f32,0.0,0.0]).collect();
        let mut reduced=CpuBodyMesh::decimate(&positions,&[[1.0;4];9],&[8,7,6],&[(0,3)]);
        assert_eq!(reduced.source_vertices,[8,7,6]);
        assert!(reduced.update_authored_span(7,&[[70.0,1.0,0.0],[80.0,2.0,0.0]]));
        assert_eq!(reduced.positions,[[80.0,2.0,0.0],[70.0,1.0,0.0],[6.0,0.0,0.0]]);
    }

    #[test]
    fn reduced_replay_mesh_compacts_vertices_and_keeps_valid_indices() {
        let positions: Vec<[f32; 3]> = (0..900)
            .map(|i| [i as f32, (i % 7) as f32, (i % 13) as f32])
            .collect();
        let colors = vec![[0.25, 0.5, 0.75, 1.0]; positions.len()];
        let indices: Vec<u32> = (0..positions.len() as u32).collect();
        let spans = [(0, indices.len())];

        let reduced = CpuBodyMesh::decimate(&positions, &colors, &indices, &spans);

        assert_eq!(reduced.indices.len(), 220 * 3);
        assert!(reduced.positions.len() < positions.len());
        assert_eq!(reduced.positions.len(), reduced.colors.len());
        assert!(reduced
            .indices
            .iter()
            .all(|&index| (index as usize) < reduced.positions.len()));
    }
}
