//! Piecewise-linear SDFB section geometry. The solid cap and positive cavity
//! apertures are sliced from the same lattice tetrahedra as the retained surface.

use super::{
    extract_lattice, grid_index, grid_point, sample_lattice, Mesh, SampleLattice, SdfBrickKey,
    SdfBrickView, MAX_SEMANTIC_GROUPS, MAX_TRIANGLES, MAX_VERTICES,
};
use crate::container_10d::{
    sdf_brick_section::read_container_sdfb_with_iris, MaterialRecord, OpacityMode, SubmeshRange,
};
use crate::render::portal_load_10d::PickSurfaceRegion;
use std::collections::BTreeMap;

const CORNERS: [[usize; 3]; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [1, 1, 0],
    [0, 1, 0],
    [0, 0, 1],
    [1, 0, 1],
    [1, 1, 1],
    [0, 1, 1],
];
// Same diagonal and six tetrahedra as indexed_tetra_surface.
const TETS: [[usize; 4]; 6] = [
    [0, 1, 2, 6],
    [0, 2, 3, 6],
    [0, 3, 7, 6],
    [0, 7, 4, 6],
    [0, 4, 5, 6],
    [0, 5, 1, 6],
];
const MAX_SECTION_PATCHES: usize = 500_000;

/// Plane in authored metres. With no slab, retain `n·p + d <= 0`.
/// A positive slab thickness retains both bounded half spaces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionPlane {
    plane: [f64; 4],
    slab_thickness_m: f64,
}

impl SectionPlane {
    pub fn new(abcd: [f64; 4], slab_thickness_m: f64) -> Result<Self, String> {
        let [x, y, z, _] = abcd;
        let norm = x.hypot(y).hypot(z);
        if !norm.is_finite()
            || norm == 0.0
            || !abcd[3].is_finite()
            || !slab_thickness_m.is_finite()
            || slab_thickness_m < 0.0
        {
            return Err(
                "A section plane requires a finite nonzero normal and nonnegative slab thickness"
                    .into(),
            );
        }
        let plane = abcd.map(|v| v / norm);
        if plane.iter().any(|v| !v.is_finite()) {
            return Err("Section plane normalization exceeds finite metre coordinates".into());
        }
        Ok(Self {
            plane,
            slab_thickness_m,
        })
    }

    pub(crate) fn in_local_frame(self, world_from_local: &[[f64; 4]; 4]) -> Result<Self, String> {
        let local: [f64; 4] = std::array::from_fn(|column| {
            (0..4)
                .map(|axis| self.plane[axis] * world_from_local[column][axis])
                .sum()
        });
        let norm = local[0].hypot(local[1]).hypot(local[2]);
        Self::new(local, self.slab_thickness_m / norm)
    }

    fn boundaries(self) -> impl Iterator<Item = [f64; 4]> {
        let [a, b, c, d] = self.plane;
        let half = self.slab_thickness_m * 0.5;
        std::iter::once([a, b, c, d - half]).chain((half > 0.0).then_some([-a, -b, -c, -d - half]))
    }
}

pub struct SdfCutawaySurface {
    pub mesh: Mesh,
    pub materials: Vec<MaterialRecord>,
    pub ranges: Vec<SubmeshRange>,
    pub regions: Vec<PickSurfaceRegion>,
    /// Fit the original field, so a sweep retains the same orbit frame.
    pub source_center_m: [f32; 3],
    pub source_span_m: f32,
    pub source_bounds_m: [[f32; 3]; 2],
    /// Source-admitted CBLD IRIs for SDFB wall and lumen hashes.
    pub semantic_iris: BTreeMap<u64, String>,
}

pub fn decode_sdfb_cutaway(bytes: &[u8], plane: SectionPlane) -> Result<SdfCutawaySurface, String> {
    use crate::container_10d::{
        compute_whole_file_crc32c,
        header::{Container10dHeader, FLAG_DEFAULT_DISPOSITION_REFUSE},
        provenance_section::{decode_provenance_section, validate_provenance},
        SectionType, TypedFieldRecord, TypedPayloadResolver, MANIFOLD_IDENTITY_V2_MAX_FIELDS,
    };
    let header = Container10dHeader::parse(bytes).map_err(|error| error.to_string())?;
    if compute_whole_file_crc32c(bytes) != header.header_crc32c {
        return Err("SDFB section source has an invalid 10D checksum".into());
    }
    if header.flags & FLAG_DEFAULT_DISPOSITION_REFUSE != 0 {
        let mut scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
        let mut fields = scratch;
        let view = TypedPayloadResolver::new(bytes, &[], &mut scratch, &mut fields)
            .map_err(|error| error.to_string())?;
        let payload = view
            .section_payload(SectionType::ProvenanceSidecar)
            .ok_or("SDFB section source is missing its required attestation")?;
        let provenance =
            decode_provenance_section(payload).map_err(|error| format!("{error:?}"))?;
        validate_provenance(&provenance).map_err(|error| format!("{error:?}"))?;
    }
    let (view, iris) =
        read_container_sdfb_with_iris(bytes)?.ok_or("Section source has no verified SDFB field")?;
    let mut section = cutaway(view, plane)?;
    section.semantic_iris = iris;
    Ok(section)
}

fn cutaway(view: SdfBrickView<'_>, plane: SectionPlane) -> Result<SdfCutawaySurface, String> {
    let grid = sample_lattice(view)?;
    let source = extract_lattice(view, &grid)?;
    let center = source.mesh.centroid();
    let extents = [source.mesh.min, source.mesh.max];
    let span = (0..3)
        .map(|axis| source.mesh.max[axis] - source.mesh.min[axis])
        .fold(1e-6, f32::max);
    let bounds: Vec<_> = plane.boundaries().collect();
    let mut mesh = MeshBuilder::default();
    for range in &source.ranges {
        let first = range.first_index as usize / 3;
        let last = first + range.index_count as usize / 3;
        for triangle in &source.mesh.triangles[first..last] {
            let a = source.mesh.positions[triangle[0] as usize].map(f64::from);
            let b = source.mesh.positions[triangle[1] as usize].map(f64::from);
            let c = source.mesh.positions[triangle[2] as usize].map(f64::from);
            let mut polygon = vec![a, b, c];
            for &plane in &bounds {
                polygon = clip_polygon(&polygon, |point| plane_value(plane, point));
            }
            mesh.append(&polygon, PickSurfaceRegion::UncutSurface, range.semantic_id)?;
        }
    }
    drop(source);
    for boundary in bounds {
        section(view, &grid, boundary, &mut mesh)?;
    }
    mesh.finish(center, span, extents)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn plane_value([a, b, c, d]: [f64; 4], point: [f64; 3]) -> f64 {
    dot([a, b, c], point) + d
}

fn interpolate(a: [f64; 3], b: [f64; 3], fa: f64, fb: f64) -> [f64; 3] {
    let t = fa / (fa - fb);
    std::array::from_fn(|axis| a[axis] + t * (b[axis] - a[axis]))
}

fn clip_polygon(points: &[[f64; 3]], value: impl Fn([f64; 3]) -> f64) -> Vec<[f64; 3]> {
    let mut polygon = Vec::with_capacity(points.len() + 1);
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let fa = value(a);
        let fb = value(b);
        if fa <= 0.0 {
            polygon.push(a);
        }
        if (fa < 0.0 && fb > 0.0) || (fa > 0.0 && fb < 0.0) {
            // Stable endpoint order gives the same shared-edge point in both faces.
            let a_first = a
                .iter()
                .zip(b)
                .find(|(l, r)| **l != *r)
                .is_some_and(|(l, r)| *l < r);
            polygon.push(if a_first {
                interpolate(a, b, fa, fb)
            } else {
                interpolate(b, a, fb, fa)
            });
        }
    }
    polygon
}

#[derive(Clone, Copy, Default)]
struct SliceSample {
    position: [f64; 3],
    sdf: f64,
    /// Original lattice edge intersected by the plane. Equal endpoints are an
    /// original sample that is itself on the plane.
    key: [usize; 2],
}

fn slice_tetra(
    grid: &SampleLattice,
    mut ids: [usize; 4],
    plane: [f64; 4],
    epsilon: f64,
) -> ([SliceSample; 4], usize) {
    ids.sort_unstable();
    let positions = ids.map(|id| grid.position(id));
    let distances = positions.map(|point| {
        let value = plane_value(plane, point);
        if value.abs() <= epsilon {
            0.0
        } else {
            value
        }
    });
    let mut polygon = [SliceSample::default(); 4];
    // Retained-side ownership removes duplicate caps on coincident tetra faces.
    if !distances.iter().any(|&d| d < 0.0) || !distances.iter().any(|&d| d >= 0.0) {
        return (polygon, 0);
    }
    let mut count = 0;
    for a in 0..4 {
        if distances[a] == 0.0 {
            polygon[count] = SliceSample {
                position: positions[a],
                sdf: grid.distances[ids[a]],
                key: [ids[a]; 2],
            };
            count += 1;
        }
        for b in a + 1..4 {
            let fa = distances[a];
            let fb = distances[b];
            if (fa < 0.0 && fb > 0.0) || (fa > 0.0 && fb < 0.0) {
                let da = grid.distances[ids[a]];
                let db = grid.distances[ids[b]];
                polygon[count] = SliceSample {
                    position: interpolate(positions[a], positions[b], fa, fb),
                    sdf: da + (fa / (fa - fb)) * (db - da),
                    key: [ids[a], ids[b]],
                };
                count += 1;
            }
        }
    }
    if count >= 3 {
        let center: [f64; 3] = std::array::from_fn(|axis| {
            polygon[..count]
                .iter()
                .map(|v| v.position[axis])
                .sum::<f64>()
                / count as f64
        });
        let n = [plane[0], plane[1], plane[2]];
        let reference = if n[0].abs() < 0.8 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let tangent = cross(reference, n);
        let bitangent = cross(n, tangent);
        let angle = |sample: &SliceSample| {
            let delta = std::array::from_fn(|axis| sample.position[axis] - center[axis]);
            dot(delta, bitangent).atan2(dot(delta, tangent))
        };
        polygon[..count].sort_unstable_by(|a, b| angle(a).total_cmp(&angle(b)));
    }
    (polygon, count)
}

fn clip_sdf(polygon: &[SliceSample], keep_solid: bool) -> Vec<[f64; 3]> {
    let mut output = Vec::with_capacity(polygon.len() + 1);
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        if (keep_solid && a.sdf <= 0.0) || (!keep_solid && a.sdf >= 0.0) {
            output.push(a.position);
        }
        if (a.sdf < 0.0 && b.sdf > 0.0) || (a.sdf > 0.0 && b.sdf < 0.0) {
            let [a, b] = if a.key <= b.key { [a, b] } else { [b, a] };
            output.push(interpolate(a.position, b.position, a.sdf, b.sdf));
        }
    }
    output
}

fn ids_for_voxel(view: SdfBrickView<'_>, grid: &SampleLattice, index: usize) -> Option<(u64, u64)> {
    let point = grid_point(index, grid.dims);
    (0..=grid.level).rev().find_map(|level| {
        let width = (1usize << (grid.level - level)) * 8;
        let key = SdfBrickKey {
            refinement_level: level,
            coordinate: point.map(|v| (v / width) as u32),
        };
        view.brick(key)
            .map(|brick| (brick.wall_semantic_id(), brick.lumen_semantic_id()))
    })
}

struct PositivePatch {
    tet: [usize; 4],
    parent: usize,
    rank: u8,
    exterior: bool,
    lumen: Option<u64>,
    different_lumen_ids: bool,
}

fn root(patches: &mut [PositivePatch], mut patch: usize) -> usize {
    while patches[patch].parent != patch {
        let parent = patches[patch].parent;
        patches[patch].parent = patches[parent].parent;
        patch = patches[patch].parent;
    }
    patch
}

fn unite(patches: &mut [PositivePatch], a: usize, b: usize) {
    let (mut a, mut b) = (root(patches, a), root(patches, b));
    if a == b {
        return;
    }
    if patches[a].rank < patches[b].rank {
        std::mem::swap(&mut a, &mut b);
    }
    let b_ext = patches[b].exterior;
    let b_lumen = patches[b].lumen;
    let b_conflict = patches[b].different_lumen_ids;
    patches[b].parent = a;
    patches[a].exterior |= b_ext;
    let a_lumen = patches[a].lumen;
    patches[a].different_lumen_ids |=
        b_conflict || a_lumen.zip(b_lumen).is_some_and(|(l, r)| l != r);
    patches[a].lumen = a_lumen.or(b_lumen);
    if patches[a].rank == patches[b].rank {
        patches[a].rank += 1;
    }
}

fn on_domain_edge(grid: &SampleLattice, keys: [[usize; 2]; 2]) -> bool {
    let points = keys.map(|edge| edge.map(|index| grid_point(index, grid.dims)));
    (0..3).any(|axis| {
        let first = points[0][0][axis];
        (first == 0 || first + 1 == grid.dims[axis])
            && points.iter().flatten().all(|point| point[axis] == first)
    })
}

fn section(
    view: SdfBrickView<'_>,
    grid: &SampleLattice,
    plane: [f64; 4],
    mesh: &mut MeshBuilder,
) -> Result<(), String> {
    let epsilon = grid.spacing.into_iter().fold(f64::INFINITY, f64::min) * 1e-8;
    let pivot = (0..3)
        .max_by(|&a, &b| plane[a].abs().total_cmp(&plane[b].abs()))
        .unwrap();
    let [u, v] = [(pivot + 1) % 3, (pivot + 2) % 3];
    let mut edges = BTreeMap::new();
    let mut positive = Vec::<PositivePatch>::new();
    let mut patches = 0;
    for y in 0..grid.dims[v] - 1 {
        for x in 0..grid.dims[u] - 1 {
            let local =
                |axis: usize, index: usize| grid.origin[axis] + index as f64 * grid.spacing[axis];
            let mut min = f64::INFINITY;
            let mut max = f64::NEG_INFINITY;
            for a in [x, x + 1] {
                for b in [y, y + 1] {
                    let world = -(plane[3] + plane[u] * local(u, a) + plane[v] * local(v, b))
                        / plane[pivot];
                    let sample = (world - grid.origin[pivot]) / grid.spacing[pivot];
                    min = min.min(sample);
                    max = max.max(sample);
                }
            }
            if max < 0.0 || min > (grid.dims[pivot] - 1) as f64 {
                continue;
            }
            let first = (min.floor() as usize).saturating_sub(1);
            let last = (max.ceil() as usize).min(grid.dims[pivot] - 2);
            for z in first..=last {
                let mut coord = [0; 3];
                coord[u] = x;
                coord[v] = y;
                coord[pivot] = z;
                let corners = CORNERS.map(|offset| {
                    grid_index(
                        std::array::from_fn(|axis| coord[axis] + offset[axis]),
                        grid.dims,
                    )
                });
                for tet in TETS {
                    let tet = tet.map(|i| corners[i]);
                    let (slice, count) = slice_tetra(grid, tet, plane, epsilon);
                    if count < 3 {
                        continue;
                    }
                    patches += 1;
                    if patches > MAX_SECTION_PATCHES {
                        return Err(
                            "SDFB section exceeds the bounded lattice-intersection budget".into(),
                        );
                    }
                    let polygon = &slice[..count];
                    let inside = tet
                        .into_iter()
                        .filter(|&index| grid.distances[index] < 0.0)
                        .min_by_key(|&index| (!grid.authored[index], index));
                    let ids = inside.and_then(|index| ids_for_voxel(view, grid, index));
                    if polygon.iter().any(|v| v.sdf < 0.0) {
                        let (wall, _) = ids.ok_or(
                            "SDFB section found cut tissue without a lexical wall binding",
                        )?;
                        mesh.append(
                            &clip_sdf(polygon, true),
                            PickSurfaceRegion::CutTissueWall,
                            wall,
                        )?;
                    }
                    if !polygon.iter().any(|v| v.sdf > 0.0) {
                        continue;
                    }
                    let patch = positive.len();
                    positive.push(PositivePatch {
                        tet,
                        parent: patch,
                        rank: 0,
                        exterior: false,
                        lumen: ids.map(|(_, lumen)| lumen),
                        different_lumen_ids: false,
                    });
                    for i in 0..count {
                        let a = polygon[i];
                        let b = polygon[(i + 1) % count];
                        if a.sdf <= 0.0 && b.sdf <= 0.0 {
                            continue;
                        }
                        let mut keys = [a.key, b.key];
                        keys.sort_unstable();
                        if on_domain_edge(grid, keys) {
                            let component = root(&mut positive, patch);
                            positive[component].exterior = true;
                        }
                        if let Some(other) = edges.insert(keys, patch) {
                            unite(&mut positive, patch, other);
                        }
                    }
                }
            }
        }
    }
    drop(edges);
    for patch in 0..positive.len() {
        let component = root(&mut positive, patch);
        if positive[component].exterior {
            continue;
        }
        if positive[component].different_lumen_ids {
            return Err("Enclosed SDFB aperture is bound to competing lumen IRIs".into());
        }
        let lumen = positive[component]
            .lumen
            .ok_or("Enclosed SDFB section contour has no lexical lumen binding")?;
        let (polygon, count) = slice_tetra(grid, positive[patch].tet, plane, epsilon);
        mesh.append(
            &clip_sdf(&polygon[..count], false),
            PickSurfaceRegion::InternalCavityLumen,
            lumen,
        )?;
    }
    Ok(())
}

#[derive(Default)]
struct MeshBuilder {
    vertices: BTreeMap<(u8, [u32; 3]), u32>,
    positions: Vec<[f32; 3]>,
    faces: BTreeMap<(u8, u64), Vec<[u32; 3]>>,
    count: usize,
}

impl MeshBuilder {
    fn append(
        &mut self,
        polygon: &[[f64; 3]],
        region: PickSurfaceRegion,
        semantic: u64,
    ) -> Result<(), String> {
        if polygon.len() < 3 {
            return Ok(());
        }
        let first = polygon[0].map(|v| v as f32);
        for pair in polygon[1..].windows(2) {
            let positions = [first, pair[0].map(|v| v as f32), pair[1].map(|v| v as f32)];
            let a =
                std::array::from_fn(|axis| f64::from(positions[1][axis]) - f64::from(first[axis]));
            let b =
                std::array::from_fn(|axis| f64::from(positions[2][axis]) - f64::from(first[axis]));
            if cross(a, b) == [0.0; 3] {
                continue;
            }
            if self.count >= MAX_TRIANGLES {
                return Err("SDFB cutaway exceeds the triangle budget".into());
            }
            let mut triangle = [0; 3];
            for (index, position) in positions.into_iter().enumerate() {
                let next = self.positions.len() as u32;
                let key = (
                    region as u8,
                    position.map(|v| if v == 0.0 { 0 } else { v.to_bits() }),
                );
                let entry = self.vertices.entry(key).or_insert(next);
                if *entry == next {
                    if self.positions.len() >= MAX_VERTICES {
                        return Err("SDFB cutaway exceeds the vertex budget".into());
                    }
                    self.positions.push(position);
                }
                triangle[index] = *entry;
            }
            self.faces
                .entry((region as u8, semantic))
                .or_default()
                .push(triangle);
            self.count += 1;
        }
        Ok(())
    }

    fn finish(
        self,
        source_center_m: [f32; 3],
        source_span_m: f32,
        source_bounds_m: [[f32; 3]; 2],
    ) -> Result<SdfCutawaySurface, String> {
        if self.faces.len() > MAX_SEMANTIC_GROUPS {
            return Err("SDFB cutaway exceeds the draw-group budget".into());
        }
        let mut wall = MaterialRecord::legacy_default();
        wall.multiply_vertex_color = false;
        wall.base_color = [0.69, 0.58, 0.49, 1.0];
        let mut cut_wall = wall;
        cut_wall.id = crate::q_hash("urn:qualia:material:cut-tissue-wall");
        cut_wall.base_color = [0.68, 0.32, 0.18, 1.0];
        cut_wall.double_sided = true;
        let mut lumen = wall;
        lumen.id = crate::q_hash("urn:qualia:material:internal-cavity-lumen");
        lumen.base_color = [0.14, 0.45, 0.65, 0.24];
        lumen.opacity_mode = OpacityMode::Blend;
        lumen.double_sided = true;
        lumen.casts_shadow = false;
        let mut materials = vec![wall, cut_wall, lumen];
        materials.sort_unstable_by_key(|material| material.id);
        let mut triangles = Vec::with_capacity(self.count);
        let mut ranges = Vec::with_capacity(self.faces.len());
        let mut regions = Vec::with_capacity(self.faces.len());
        for ((region, semantic_id), faces) in self.faces {
            let (region, material) = match region {
                0 => (PickSurfaceRegion::UncutSurface, wall),
                1 => (PickSurfaceRegion::CutTissueWall, cut_wall),
                2 => (PickSurfaceRegion::InternalCavityLumen, lumen),
                _ => return Err("Invalid SDFB cutaway surface region".into()),
            };
            ranges.push(SubmeshRange {
                first_index: (triangles.len() * 3) as u32,
                index_count: (faces.len() * 3) as u32,
                material_id: material.id,
                semantic_id,
            });
            regions.push(region);
            triangles.extend(faces);
        }
        let (mut min, mut max) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for position in &self.positions {
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(position[axis]);
            }
        }
        if self.positions.is_empty() {
            [min, max] = source_bounds_m;
        }
        Ok(SdfCutawaySurface {
            mesh: Mesh {
                positions: self.positions,
                triangles,
                min,
                max,
            },
            materials,
            ranges,
            regions,
            source_center_m,
            source_span_m,
            source_bounds_m,
            semantic_iris: BTreeMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::sdf_brick_section::{
        encode_sdfb_into, sdfb_encoded_len, SdfBrickGrid, SdfBrickKey, SdfBrickRecord,
        SdfSampleEncoding, SDFB_OCCUPANCY_BYTES, SDFB_SAMPLES,
    };

    #[test]
    fn through_open_tube_section_has_distinct_wall_and_lumen_hashes() {
        let wall = crate::q_hash("urn:test:tube-wall");
        let lumen = crate::q_hash("urn:test:tube-lumen");
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
            wall_semantic_id: wall,
            lumen_semantic_id: lumen,
            samples: [0; SDFB_SAMPLES],
            occupancy: [0; SDFB_OCCUPANCY_BYTES],
        };
        for (index, bits) in brick.samples.iter_mut().enumerate() {
            let [x, y, z] = grid_point(index, [8; 3]).map(|v| v as f32 - 3.5);
            let r = x.hypot(y);
            let d = (r - 2.5).max(1.2 - r).max(z.abs() - 2.8);
            *bits = (d * 100.0).round() as i16 as u16;
            if d < 0.0 {
                brick.occupancy[index / 8] |= 1 << (index % 8);
            }
        }
        let mut bytes = vec![0; sdfb_encoded_len(1).unwrap()];
        encode_sdfb_into(grid, &[brick], &mut bytes).unwrap();
        let plane = SectionPlane::new([0.0, 0.0, 1.0, 0.0], 0.0).unwrap();
        let output = cutaway(SdfBrickView::parse(&bytes).unwrap(), plane).unwrap();
        assert!(output
            .ranges
            .iter()
            .zip(&output.regions)
            .any(|(range, region)| range.semantic_id == wall
                && *region == PickSurfaceRegion::CutTissueWall));
        let lumen_range = output
            .ranges
            .iter()
            .zip(&output.regions)
            .find_map(|(range, region)| {
                (*region == PickSurfaceRegion::InternalCavityLumen).then_some(range)
            })
            .unwrap();
        assert_eq!(lumen_range.semantic_id, lumen);
        let start = lumen_range.first_index as usize / 3;
        let end = start + lumen_range.index_count as usize / 3;
        for triangle in &output.mesh.triangles[start..end] {
            for &vertex in triangle {
                let [x, y, z] = output.mesh.positions[vertex as usize];
                assert!(x.hypot(y) <= 1.3);
                assert!(z.abs() < 1e-6);
            }
        }
    }
}
