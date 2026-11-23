//! Prepare the sparse SDFB isosurface for the existing native GPU draw path.

use crate::container_10d::{
    sdf_brick_section::{
        read_container_sdfb, SdfBrickKey, SdfBrickView, SdfSampleEncoding, SDFB_SAMPLES,
    },
    MaterialRecord, SubmeshRange,
};
use crate::render::assets::Mesh;
use crate::specialized_libs::computational_geometry::{
    indexed_tetra_surface::{indexed_tetra_surface_capped_into, TetraEdgeSlot},
    Point3,
};
use std::collections::BTreeMap;

#[path = "sdf_cutaway.rs"]
mod cutaway;
pub use cutaway::{
    decode_sdfb_cutaway, decode_sdfb_cutaway_with_emf, SdfCutawaySurface, SectionPlane,
};

// 257³ samples × 8 bytes plus bounded indexed-surface scratch fit the 192 MiB
// cold geometry budget. This is independent of the sealed .10d ring budget.
const GEOMETRY_SCRATCH_BYTES: usize = 192 * 1024 * 1024;
const MAX_DIMS: usize = 257;
const MAX_VERTICES: usize = 200_000;
const MAX_TRIANGLES: usize = 400_000;
const MAX_SEMANTIC_GROUPS: usize = 512;
const EDGE_SLOTS: usize = 1 << 20;
const MAX_EDGE_PROBES: usize = 32_000_000;

pub(crate) struct SdfBrickSurface {
    pub mesh: Mesh,
    pub ranges: Vec<SubmeshRange>,
}

pub(crate) fn decode_sdfb_mesh(bytes: &[u8]) -> Result<Option<SdfBrickSurface>, String> {
    read_container_sdfb(bytes)?.map(extract).transpose()
}

fn grid_index([x, y, z]: [usize; 3], dims: [usize; 3]) -> usize {
    (z * dims[1] + y) * dims[0] + x
}

fn grid_point(index: usize, [nx, ny, _]: [usize; 3]) -> [usize; 3] {
    [index % nx, (index / nx) % ny, index / (nx * ny)]
}

fn extract(view: SdfBrickView<'_>) -> Result<SdfBrickSurface, String> {
    let lattice = sample_lattice(view)?;
    extract_lattice(view, &lattice)
}

struct SampleLattice {
    dims: [usize; 3],
    level: u8,
    origin: [f64; 3],
    spacing: [f64; 3],
    distances: Vec<f64>,
    authored: Vec<bool>,
}

impl SampleLattice {
    fn position(&self, index: usize) -> [f64; 3] {
        let point = grid_point(index, self.dims);
        std::array::from_fn(|axis| self.origin[axis] + point[axis] as f64 * self.spacing[axis])
    }
}

fn sample_lattice(view: SdfBrickView<'_>) -> Result<SampleLattice, String> {
    let grid = view.grid();
    let level = view
        .bricks()
        .map(|brick| brick.key().refinement_level)
        .max()
        .unwrap_or(0);
    let divisor = 1usize << level;
    let mut dims = [0; 3];
    for (axis, dim) in dims.iter_mut().enumerate() {
        *dim = (grid.base_bricks[axis] as usize)
            .checked_mul(8)
            .and_then(|n| (n - 1).checked_mul(divisor))
            .and_then(|n| n.checked_add(1))
            .filter(|&n| n <= MAX_DIMS)
            .ok_or("SDFB rendering grid exceeds its calibrated geometry limit")?;
    }
    let samples = dims.into_iter().product::<usize>();
    let scratch = samples * (std::mem::size_of::<f64>() + std::mem::size_of::<bool>())
        + MAX_VERTICES * (std::mem::size_of::<Point3>() + std::mem::size_of::<[usize; 2]>())
        + MAX_TRIANGLES * std::mem::size_of::<[u32; 3]>()
        + EDGE_SLOTS * std::mem::size_of::<TetraEdgeSlot>();
    if scratch > GEOMETRY_SCRATCH_BYTES {
        return Err("SDFB surface extraction exceeds its geometry scratch budget".into());
    }
    let spacing = grid.voxel_spacing_m.map(|v| f64::from(v) / divisor as f64);
    let outside = match grid.encoding {
        SdfSampleEncoding::ScaledI16 => f64::from(grid.meters_per_i16) * f64::from(i16::MAX),
        SdfSampleEncoding::F16 => {
            4.0 * grid
                .voxel_spacing_m
                .map(f64::from)
                .into_iter()
                .fold(0.0, f64::max)
        }
    };
    // Exact zeros use the occupancy convention at a quarter of the smallest
    // codec step. This keeps boundary edges distinct in the indexed extractor.
    let zero_tie = match grid.encoding {
        SdfSampleEncoding::ScaledI16 => f64::from(grid.meters_per_i16) / 4.0,
        SdfSampleEncoding::F16 => 2.0_f64.powi(-26),
    };
    let mut distances = vec![outside; samples];
    let mut authored = vec![false; samples];
    let outside_cell = |point: [usize; 3]| (0..3).any(|axis| point[axis] >= dims[axis]);
    for current in 0..=level {
        let step = 1usize << (level - current);
        if current > 0 {
            // Interpolate the parent lattice at odd fine-grid coordinates,
            // then overlay the directly authored samples at this level.
            for z in (0..dims[2]).step_by(step) {
                for y in (0..dims[1]).step_by(step) {
                    for x in (0..dims[0]).step_by(step) {
                        let point = [x, y, z];
                        let lower = point.map(|v| v / (2 * step) * (2 * step));
                        if point == lower {
                            continue;
                        }
                        let mut total = 0.0;
                        for corner in 0..8 {
                            let at = std::array::from_fn(|axis| {
                                if point[axis] == lower[axis] {
                                    lower[axis]
                                } else {
                                    (lower[axis] + ((corner >> axis) & 1) * 2 * step)
                                        .min(dims[axis] - 1)
                                }
                            });
                            total += distances[grid_index(at, dims)];
                        }
                        distances[grid_index(point, dims)] = total / 8.0;
                    }
                }
            }
        }
        for brick in view
            .bricks()
            .filter(|brick| brick.key().refinement_level == current)
        {
            let offset = brick.key().coordinate.map(|v| v as usize * 8 * step);
            for sample in 0..SDFB_SAMPLES {
                let local = grid_point(sample, [8; 3]);
                let point = std::array::from_fn(|axis| offset[axis] + local[axis] * step);
                if outside_cell(point) {
                    continue;
                }
                let index = grid_index(point, dims);
                let distance =
                    f64::from(brick.distance_m(sample).ok_or("SDFB sample out of range")?);
                let occupied = brick
                    .occupied(sample)
                    .ok_or("SDFB occupancy out of range")?;
                distances[index] = if distance == 0.0 {
                    if occupied {
                        -zero_tie
                    } else {
                        zero_tie
                    }
                } else {
                    distance
                };
                authored[index] = true;
            }
        }
    }
    for distance in &mut distances {
        if *distance == 0.0 {
            *distance = zero_tie;
        }
    }

    let origin = grid.origin_m.map(f64::from);
    if (0..3).any(|axis| {
        let max = f64::from(grid.origin_m[axis]) + (dims[axis] - 1) as f64 * spacing[axis];
        !(max as f32).is_finite()
    }) {
        return Err("SDFB coordinates exceed GPU position precision".into());
    }
    Ok(SampleLattice {
        dims,
        level,
        origin,
        spacing,
        distances,
        authored,
    })
}

fn extract_lattice(
    view: SdfBrickView<'_>,
    lattice: &SampleLattice,
) -> Result<SdfBrickSurface, String> {
    let SampleLattice {
        dims,
        level,
        origin,
        spacing,
        distances,
        authored,
    } = lattice;
    let [x, y, z] = *origin;
    let origin = Point3::new(x, y, z);
    let [x, y, z] = *spacing;
    let spacing = Point3::new(x, y, z);
    let mut vertices = vec![Point3::default(); MAX_VERTICES];
    let mut triangles = vec![[0; 3]; MAX_TRIANGLES];
    let mut edge_sources = vec![[0; 2]; MAX_VERTICES];
    let mut slots = vec![TetraEdgeSlot::default(); EDGE_SLOTS];
    let (vertex_count, triangle_count) = indexed_tetra_surface_capped_into(
        distances,
        *dims,
        origin,
        spacing,
        0.0,
        MAX_DIMS.pow(3),
        MAX_EDGE_PROBES,
        0.0,
        &mut vertices,
        &mut triangles,
        &mut edge_sources,
        &mut slots,
    )
    .map_err(|error| format!("SDFB indexed surface: {error:?}"))?;
    if triangle_count == 0 {
        return Err("SDFB has no enclosed renderable level set".into());
    }
    drop(slots);
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let positions = vertices[..vertex_count]
        .iter()
        .map(|p| {
            let p = [p.x as f32, p.y as f32, p.z as f32];
            for axis in 0..3 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
            p
        })
        .collect();

    let material_id = MaterialRecord::legacy_default().id;
    let mut by_wall = BTreeMap::<u64, Vec<[u32; 3]>>::new();
    for triangle in &triangles[..triangle_count] {
        let sources = triangle
            .iter()
            .flat_map(|&vertex| edge_sources[vertex as usize]);
        let inside = sources
            .filter(|&index| distances[index] < 0.0)
            .min_by_key(|&index| (!authored[index], index))
            .ok_or("SDFB face has no inside sample")?;
        let point = grid_point(inside, *dims);
        let wall = (0..=*level)
            .rev()
            .find_map(|current| {
                let step = 1usize << (*level - current);
                let coordinate = point.map(|v| (v / (8 * step)) as u32);
                view.brick(SdfBrickKey {
                    refinement_level: current,
                    coordinate,
                })
                .map(|brick| brick.wall_semantic_id())
            })
            .ok_or("SDFB face has no lexical wall binding")?;
        by_wall.entry(wall).or_default().push(*triangle);
    }
    if by_wall.len() > MAX_SEMANTIC_GROUPS {
        return Err("SDFB wall groups exceed the GPU instance-group budget".into());
    }
    triangles.clear();
    let mut ranges = Vec::with_capacity(by_wall.len());
    for (wall, faces) in by_wall {
        ranges.push(SubmeshRange {
            first_index: (triangles.len() * 3) as u32,
            index_count: (faces.len() * 3) as u32,
            material_id,
            semantic_id: wall,
        });
        triangles.extend(faces);
    }
    Ok(SdfBrickSurface {
        mesh: Mesh {
            positions,
            triangles,
            min,
            max,
        },
        ranges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::sdf_brick_section::{
        encode_sdfb_into, sdfb_encoded_len, SdfBrickGrid, SdfBrickRecord, SDFB_OCCUPANCY_BYTES,
    };

    #[test]
    fn sphere_level_set_uses_sparse_samples_and_keeps_wall_id() {
        let grid = SdfBrickGrid {
            origin_m: [-3.5; 3],
            voxel_spacing_m: [1.0; 3],
            base_bricks: [1; 3],
            encoding: SdfSampleEncoding::ScaledI16,
            meters_per_i16: 0.01,
        };
        let mut brick = SdfBrickRecord {
            key: SdfBrickKey {
                refinement_level: 0,
                coordinate: [0; 3],
            },
            wall_semantic_id: crate::q_hash("urn:test:sdf-wall"),
            lumen_semantic_id: crate::q_hash("urn:test:sdf-lumen"),
            samples: [0; SDFB_SAMPLES],
            occupancy: [0; SDFB_OCCUPANCY_BYTES],
        };
        for (index, sample) in brick.samples.iter_mut().enumerate() {
            let p = grid_point(index, [8; 3]).map(|v| v as f32 - 3.5);
            let d = p.iter().map(|v| v * v).sum::<f32>().sqrt() - 2.0;
            *sample = (d * 100.0).round() as i16 as u16;
            if d < 0.0 {
                brick.occupancy[index / 8] |= 1 << (index % 8);
            }
        }
        let wall = brick.wall_semantic_id;
        let mut bytes = vec![0; sdfb_encoded_len(1).unwrap()];
        encode_sdfb_into(grid, &[brick], &mut bytes).unwrap();
        let surface = extract(SdfBrickView::parse(&bytes).unwrap()).unwrap();
        assert!(surface.mesh.triangles.len() > 100);
        assert!(surface
            .mesh
            .min
            .into_iter()
            .all(|v| (-2.0..-1.7).contains(&v)));
        assert!(surface
            .mesh
            .max
            .into_iter()
            .all(|v| (1.7..=2.0).contains(&v)));
        assert_eq!(surface.ranges.len(), 1);
        assert_eq!(surface.ranges[0].semantic_id, wall);
        assert_eq!(
            surface.ranges[0].index_count as usize,
            surface.mesh.triangles.len() * 3
        );
    }
}
