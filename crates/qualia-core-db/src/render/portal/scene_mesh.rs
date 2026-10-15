//! Multi-asset `.10d` scene assembly accumulator.
//!
//! Decomposed from `portal/mod.rs`: this module owns the cold Tier-2 accumulation lifecycle —
//! decode → CRC verify → governance gate → bounds → orbit normalisation —
//! for multi-asset 3D scenes (town layouts, environments, games, vehicles, props)
//! and human-centric models.
//!
//! Semantic nodes carried inside each asset's `Tensor10DNodes` section are
//! collected in the same authored (world) frame as the mesh vertices and
//! transformed by the identical normalise mapping, so the pick/projection
//! substrate stays spatially aligned with the drawn geometry.

use crate::container_10d;
use crate::container_10d::header::{Container10dHeader, FLAG_DEFAULT_DISPOSITION_REFUSE};
use crate::container_10d::{MaterialRecord, SubmeshRange};
use crate::render::body_fit::AnatomyBodyFit;
use crate::tensor::Tensor10D;

/// A scene upload is a single draw-list submission today. Keep its authored
/// material ranges below a practical streaming limit before allocating the
/// per-asset MAT1 buffers, even though the file container supports more.
const MAX_SCENE_MATERIAL_RANGES: usize = 4_096;

/// Accumulates decoded scene meshes for multi-asset 3D scenes.
/// Keeps decoding entirely in Rust so browsers never hold N asset copies in JS.
pub(crate) struct SceneMeshAccum {
    pub(crate) positions: Vec<[f32; 3]>,
    pub(crate) colors: Vec<[f32; 4]>,
    /// UV01 coordinates remain aligned with merged vertices. Legacy meshes
    /// use zero UVs; malformed authored streams refuse the asset atomically.
    pub(crate) uv0: Vec<[f32; 2]>,
    /// Authored source normals survive UV seam duplication and scene merging.
    pub(crate) normals: Vec<[f32; 3]>,
    pub(crate) indices: Vec<u32>,
    /// Deduplicated material records from the source assets' MAT1 sections.
    pub(crate) materials: Vec<MaterialRecord>,
    /// Global index ranges binding merged triangles to those materials.
    pub(crate) material_ranges: Vec<SubmeshRange>,
    /// Semantic nodes embedded in the assets' `Tensor10DNodes` sections,
    /// accumulated in authored space and normalised alongside `positions`.
    pub(crate) nodes: Vec<Tensor10D>,
    pub(crate) assets_loaded: u32,
    pub(crate) assets_refused: u32,
    pub(crate) total_triangles: u32,
    /// Inclusive-exclusive ranges into `indices`, one per loaded asset.
    /// Proof decimation keeps a silhouette of each range instead of
    /// striding the whole soup into stray triangles.
    pub(crate) index_spans: Vec<(usize, usize)>,
    pub(crate) animation: super::scene_animation::SceneAnimation,
    gmin: [f32; 3],
    gmax: [f32; 3],
}

/// Backward compatibility alias for the anatomy model path.
pub(crate) type BodyMeshAccum = SceneMeshAccum;

impl SceneMeshAccum {
    pub(crate) fn new() -> Self {
        Self {
            positions: Vec::new(),
            colors: Vec::new(),
            uv0: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
            materials: Vec::new(),
            material_ranges: Vec::new(),
            nodes: Vec::new(),
            assets_loaded: 0,
            assets_refused: 0,
            total_triangles: 0,
            index_spans: Vec::new(),
            animation: super::scene_animation::SceneAnimation::default(),
            gmin: [f32::INFINITY; 3],
            gmax: [f32::NEG_INFINITY; 3],
        }
    }

    /// Number of loaded assets / organs (backward compatibility field).
    #[inline]
    pub(crate) fn organs_loaded(&self) -> u32 {
        self.assets_loaded
    }

    /// Number of refused assets / organs (backward compatibility field).
    #[inline]
    pub(crate) fn organs_refused(&self) -> u32 {
        self.assets_refused
    }

    /// Decode one sealed `.10d` scene asset. One owned buffer for CRC verify only —
    /// no JS heap intermediate, no second clone after `to_vec`.
    pub(crate) fn append_asset_10d(&mut self, asset_bytes: &[u8], rgba: [f32; 4]) {
        let mut bytes = asset_bytes.to_vec();
        let header = match Container10dHeader::parse(&bytes) {
            Ok(h) => h,
            Err(_) => return,
        };
        if container_10d::verify_whole_file_crc32c(&mut bytes).is_err() {
            return;
        }
        let descs = match container_10d::parse_section_table(&bytes, &header) {
            Ok(d) => d,
            Err(_) => return,
        };

        let mut mesh = None;
        let mut nodes = Vec::new();
        let mut has_attestation = false;
        for desc in descs.iter() {
            let st = match container_10d::SectionType::from_u8(desc.section_type) {
                Some(st) => st,
                None => continue,
            };
            let off = desc.byte_offset as usize;
            let len = desc.byte_length as usize;
            if off.saturating_add(len) > bytes.len() {
                continue;
            }
            let payload = &bytes[off..off + len];
            match st {
                container_10d::SectionType::QuantizedMesh => {
                    if let Ok(m) = container_10d::decode_mesh_section(payload) {
                        mesh = Some(m);
                    }
                }
                container_10d::SectionType::Tensor10DNodes => {
                    let mut i = 0usize;
                    while let Ok(n) = container_10d::read_node(payload, i) {
                        nodes.push(n);
                        i += 1;
                    }
                }
                container_10d::SectionType::ProvenanceSidecar => {
                    if let Ok(view) =
                        container_10d::provenance_section::decode_provenance_section(payload)
                    {
                        if container_10d::provenance_section::validate_provenance(&view).is_ok() {
                            has_attestation = true;
                        }
                    }
                }
                _ => {}
            }
        }

        let Some(mesh) = mesh else {
            return;
        };
        // An optional rig must be valid and vertex-aligned even before runtime
        // playback is armed. Never silently admit malformed animation content.
        match container_10d::skeletal_container::read_skeletal_section(&bytes) {
            Ok(_) | Err(container_10d::skeletal_container::SkeletalContainerError::Missing) => {}
            Err(_) => {
                self.assets_refused += 1;
                return;
            }
        }
        let mut local_uv0 = vec![[0.0; 2]; mesh.positions.len()];
        match container_10d::read_texture_coordinate_section_into(
            &bytes,
            mesh.positions.len(),
            &mut local_uv0,
        ) {
            Ok(_) | Err(container_10d::TextureCoordinateContainerError::Missing) => {}
            Err(_) => {
                self.assets_refused += 1;
                return;
            }
        }
        let local_normals =
            match crate::render::compile_10d::decode_10d_normals(&bytes, mesh.positions.len()) {
                Ok(Some(normals)) => normals,
                Ok(None) => {
                    let mut normals = vec![[0.0; 3]; mesh.positions.len()];
                    if crate::render::mesh_surface_frames::vertex_normals_into(
                        &mesh.positions,
                        &mesh.triangles,
                        &mut normals,
                    )
                    .is_err()
                    {
                        self.assets_refused += 1;
                        return;
                    }
                    normals
                }
                Err(_) => {
                    self.assets_refused += 1;
                    return;
                }
            };
        let governance_refused =
            (header.flags & FLAG_DEFAULT_DISPOSITION_REFUSE) != 0 && !has_attestation;
        if governance_refused {
            self.assets_refused += 1;
            return;
        }

        // Preserve authored MAT1 PBR values and submesh bindings while the
        // scene is merged. Legacy assets without MAT1 receive the same safe
        // default used by the renderer's older coloured-mesh API.
        let index_count = match u32::try_from(mesh.triangles.len().saturating_mul(3)) {
            Ok(count) if count > 0 => count,
            _ => {
                self.assets_refused += 1;
                return;
            }
        };
        let (mut local_materials, local_ranges) =
            match container_10d::material_container_counts(&bytes) {
                Ok((material_count, range_count)) => {
                    if material_count == 0
                        || range_count == 0
                        || range_count > MAX_SCENE_MATERIAL_RANGES
                    {
                        self.assets_refused += 1;
                        return;
                    }
                    let mut materials = vec![MaterialRecord::legacy_default(); material_count];
                    let mut ranges = vec![
                        SubmeshRange {
                            first_index: 0,
                            index_count: 0,
                            material_id: 0,
                            semantic_id: 0,
                        };
                        range_count
                    ];
                    match container_10d::read_material_section_into(
                        &bytes,
                        index_count,
                        &mut materials,
                        &mut ranges,
                    ) {
                        Ok((read_materials, read_ranges))
                            if read_materials == material_count && read_ranges == range_count =>
                        {
                            (materials, ranges)
                        }
                        _ => {
                            self.assets_refused += 1;
                            return;
                        }
                    }
                }
                Err(container_10d::MaterialContainerError::Missing) => (
                    vec![MaterialRecord::legacy_default()],
                    vec![SubmeshRange {
                        first_index: 0,
                        index_count,
                        material_id: MaterialRecord::legacy_default().id,
                        semantic_id: 0,
                    }],
                ),
                Err(_) => {
                    self.assets_refused += 1;
                    return;
                }
            };
        if local_materials.iter().any(|material| {
            self.materials
                .iter()
                .any(|present| present.id == material.id && present != material)
        }) {
            self.assets_refused += 1;
            return;
        }
        let index_start = match u32::try_from(self.indices.len()) {
            Ok(start) => start,
            Err(_) => {
                self.assets_refused += 1;
                return;
            }
        };
        if local_ranges.iter().any(|range| {
            range.first_index.checked_add(index_start).is_none()
                || !local_materials
                    .iter()
                    .any(|material| material.id == range.material_id)
        }) {
            self.assets_refused += 1;
            return;
        }
        let new_material_count = local_materials
            .iter()
            .filter(|material| {
                !self
                    .materials
                    .iter()
                    .any(|present| present.id == material.id)
            })
            .count();
        if self.materials.len().saturating_add(new_material_count) > container_10d::MAX_MATERIALS {
            self.assets_refused += 1;
            return;
        }
        if self
            .material_ranges
            .len()
            .saturating_add(local_ranges.len())
            > MAX_SCENE_MATERIAL_RANGES
        {
            self.assets_refused += 1;
            return;
        }

        let actor = match container_10d::skeletal_container::read_skeletal_section(&bytes) {
            Ok(section) => {
                if self.animation.actors.len() >= 256 {
                    self.assets_refused += 1;
                    return;
                }
                let budget = (42usize * 1024 * 1024).saturating_sub(self.animation.resident_bytes);
                let indices: Vec<u32> = mesh.triangles.iter().flatten().copied().collect();
                match crate::render::skeletal::SkeletalPlayer::from_section_with_budget(
                    &section,
                    &mesh.positions,
                    &local_normals,
                    &local_uv0,
                    &indices,
                    budget,
                ) {
                    Ok(player) => Some(super::scene_animation::SceneActor {
                        start: self.positions.len() as u32,
                        player,
                    }),
                    Err(_) => {
                        self.assets_refused += 1;
                        return;
                    }
                }
            }
            Err(container_10d::skeletal_container::SkeletalContainerError::Missing) => None,
            Err(_) => {
                self.assets_refused += 1;
                return;
            }
        };
        if let Some(actor) = actor {
            self.animation.resident_bytes += actor.player.allocated_bytes();
            self.animation.actors.push(actor);
        }
        for k in 0..3 {
            if mesh.min[k] < self.gmin[k] {
                self.gmin[k] = mesh.min[k];
            }
            if mesh.max[k] > self.gmax[k] {
                self.gmax[k] = mesh.max[k];
            }
        }
        let base = self.positions.len() as u32;
        let index_start_usize = self.indices.len();
        let [r, g, b, a] = rgba;
        let mut surface = None;
        for desc in descs.iter() {
            if container_10d::SectionType::from_u8(desc.section_type)
                != Some(container_10d::SectionType::SurfaceReading)
            {
                continue;
            }
            let off = desc.byte_offset as usize;
            let len = desc.byte_length as usize;
            if off.saturating_add(len) > bytes.len() {
                continue;
            }
            surface =
                container_10d::surface_reading::decode_surface_reading(&bytes[off..off + len]);
        }
        let use_reading = surface
            .as_ref()
            .map(|s| s.len() == mesh.positions.len())
            .unwrap_or(false);
        for (i, p) in mesh.positions.iter().enumerate() {
            self.positions.push([p[0], p[1], p[2]]);
            self.uv0.push(local_uv0[i]);
            self.normals.push(local_normals[i]);
            if use_reading {
                self.colors.push(surface.as_ref().unwrap()[i]);
            } else {
                self.colors.push([r, g, b, a]);
            }
        }
        for t in mesh.triangles.iter() {
            self.indices.push(base + t[0]);
            self.indices.push(base + t[1]);
            self.indices.push(base + t[2]);
        }
        self.index_spans
            .push((index_start_usize, self.indices.len()));
        for material in local_materials.drain(..) {
            if !self
                .materials
                .iter()
                .any(|present| present.id == material.id)
            {
                self.materials.push(material);
            }
        }
        self.material_ranges
            .extend(local_ranges.into_iter().map(|mut range| {
                range.first_index += index_start;
                range
            }));
        self.nodes.extend(nodes);
        self.total_triangles += mesh.triangles.len() as u32;
        self.assets_loaded += 1;
    }

    /// Compatibility alias for anatomy model path.
    pub(crate) fn append_organ_10d(&mut self, organ_bytes: &[u8], rgba: [f32; 4]) {
        self.append_asset_10d(organ_bytes, rgba);
    }

    /// Apply a person-authored fit, then recompute bounds so orbit framing stays honest.
    /// Semantic nodes are transformed by the same fit so they stay on their meshes.
    pub(crate) fn apply_body_fit(&mut self, fit: &AnatomyBodyFit) -> Result<(), &'static str> {
        if fit.identity || self.positions.is_empty() {
            return Ok(());
        }
        let fit_bytes = fit
            .hidden_keys
            .len()
            .checked_mul(std::mem::size_of::<String>())
            .and_then(|n| n.checked_add(fit.hidden_keys.iter().map(|s| s.len()).sum::<usize>()))
            .ok_or("animated fit budget exceeded")?;
        if fit_bytes
            .saturating_mul(self.animation.actors.len())
            .saturating_add(self.animation.resident_bytes)
            > 42 * 1024 * 1024
        {
            return Err("animated fit budget exceeded");
        }
        for actor in &mut self.animation.actors {
            actor
                .player
                .set_body_fit(fit, self.gmin, self.gmax)
                .map_err(|_| "invalid animated body fit")?;
        }
        self.animation.resident_bytes = self
            .animation
            .actors
            .iter()
            .map(|actor| actor.player.allocated_bytes())
            .sum();
        fit.apply_in_place(&mut self.positions, self.gmin, self.gmax);
        // A nonuniform anatomical fit changes the geometric frame. Recompute
        // once after the cold fit; ordinary uniform orbit normalisation keeps
        // the authored frame intact.
        let _ = crate::render::mesh_surface_frames::vertex_normals_from_indices_into(
            &self.positions,
            &self.indices,
            &mut self.normals,
        );
        if !self.nodes.is_empty() {
            let mut pts: Vec<[f32; 3]> = self.nodes.iter().map(|n| [n.x, n.y, n.z]).collect();
            fit.apply_in_place(&mut pts, self.gmin, self.gmax);
            for (n, p) in self.nodes.iter_mut().zip(pts.iter()) {
                n.x = p[0];
                n.y = p[1];
                n.z = p[2];
            }
        }
        let mut gmin = [f32::INFINITY; 3];
        let mut gmax = [f32::NEG_INFINITY; 3];
        for p in &self.positions {
            for k in 0..3 {
                if p[k] < gmin[k] {
                    gmin[k] = p[k];
                }
                if p[k] > gmax[k] {
                    gmax[k] = p[k];
                }
            }
        }
        self.gmin = gmin;
        self.gmax = gmax;
        Ok(())
    }

    /// One global centre + scale so the scene fits ~1.7 of the orbit frame.
    /// Nodes ride the same transform so pick hits land on the drawn geometry.
    pub(crate) fn normalise_to_orbit_frame(&mut self) -> Result<(), &'static str> {
        if self.assets_loaded == 0 {
            return Ok(());
        }
        let gc = [
            (self.gmin[0] + self.gmax[0]) * 0.5,
            (self.gmin[1] + self.gmax[1]) * 0.5,
            (self.gmin[2] + self.gmax[2]) * 0.5,
        ];
        let gspan = (self.gmax[0] - self.gmin[0])
            .max(self.gmax[1] - self.gmin[1])
            .max(self.gmax[2] - self.gmin[2])
            .max(1e-6);
        let s = 1.7 / gspan;
        for actor in &mut self.animation.actors {
            // The scene bounds come from validated finite geometry. Preserve
            // the original rig frame; map its output like the static mesh.
            actor
                .player
                .set_output_frame(gc, s)
                .map_err(|_| "invalid animated orbit frame")?;
        }
        for p in self.positions.iter_mut() {
            p[0] = (p[0] - gc[0]) * s;
            p[1] = (p[1] - gc[1]) * s;
            p[2] = (p[2] - gc[2]) * s;
        }
        for n in self.nodes.iter_mut() {
            n.x = (n.x - gc[0]) * s;
            n.y = (n.y - gc[1]) * s;
            n.z = (n.z - gc[2]) * s;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "scene_mesh_tests.rs"]
mod tests;
