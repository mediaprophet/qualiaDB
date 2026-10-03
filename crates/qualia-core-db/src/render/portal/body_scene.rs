//! Multi-asset `.10d` scene assembly accumulator.
//!
//! Decomposed from `portal/mod.rs` (which exceeded the 1,400-line ownership
//! threshold): this module owns the cold Tier-2 accumulation lifecycle —
//! decode → CRC verify → governance gate → bounds → body-fit → orbit
//! normalisation — shared by the anatomy body path and generic multi-prop
//! scenes. `portal/mod.rs` keeps the renderer-facing `QualiaPortal` lifecycle.
//!
//! Semantic nodes carried inside each organ's `Tensor10DNodes` section are
//! collected in the same authored (world) frame as the mesh vertices and
//! transformed by the identical fit/normalise mapping, so the pick/projection
//! substrate stays spatially aligned with the drawn geometry.

use crate::container_10d;
use crate::container_10d::header::{Container10dHeader, FLAG_DEFAULT_DISPOSITION_REFUSE};
use crate::render::body_fit::AnatomyBodyFit;
use crate::tensor::Tensor10D;

/// Accumulates decoded organ meshes for the whole-body anatomy path.
/// Keeps decoding entirely in Rust so phone browsers never hold N organ copies in JS.
pub(crate) struct BodyMeshAccum {
    pub(crate) positions: Vec<[f32; 3]>,
    pub(crate) colors: Vec<[f32; 4]>,
    pub(crate) indices: Vec<u32>,
    /// Semantic nodes embedded in the organs' `Tensor10DNodes` sections,
    /// accumulated in authored space and normalised alongside `positions`.
    pub(crate) nodes: Vec<Tensor10D>,
    pub(crate) organs_loaded: u32,
    pub(crate) organs_refused: u32,
    pub(crate) total_triangles: u32,
    /// Inclusive-exclusive ranges into `indices`, one per loaded organ.
    /// Proof decimation keeps a silhouette of each range instead of
    /// striding the whole soup into stray triangles.
    pub(crate) index_spans: Vec<(usize, usize)>,
    gmin: [f32; 3],
    gmax: [f32; 3],
}

impl BodyMeshAccum {
    pub(crate) fn new() -> Self {
        Self {
            positions: Vec::new(),
            colors: Vec::new(),
            indices: Vec::new(),
            nodes: Vec::new(),
            organs_loaded: 0,
            organs_refused: 0,
            total_triangles: 0,
            index_spans: Vec::new(),
            gmin: [f32::INFINITY; 3],
            gmax: [f32::NEG_INFINITY; 3],
        }
    }

    /// Decode one sealed `.10d` organ. One owned buffer for CRC verify only —
    /// no JS heap intermediate, no second clone after `to_vec`.
    pub(crate) fn append_organ_10d(&mut self, organ_bytes: &[u8], rgba: [f32; 4]) {
        let mut bytes = organ_bytes.to_vec();
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
        let governance_refused =
            (header.flags & FLAG_DEFAULT_DISPOSITION_REFUSE) != 0 && !has_attestation;
        if governance_refused {
            self.organs_refused += 1;
            return;
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
        let index_start = self.indices.len();
        let [r, g, b, a] = rgba;
        for p in mesh.positions.iter() {
            self.positions.push([p[0], p[1], p[2]]);
            self.colors.push([r, g, b, a]);
        }
        for t in mesh.triangles.iter() {
            self.indices.push(base + t[0]);
            self.indices.push(base + t[1]);
            self.indices.push(base + t[2]);
        }
        self.index_spans.push((index_start, self.indices.len()));
        self.nodes.extend(nodes);
        self.total_triangles += mesh.triangles.len() as u32;
        self.organs_loaded += 1;
    }

    /// Apply a person-authored fit, then recompute bounds so orbit framing stays honest.
    /// Semantic nodes are transformed by the same fit so they stay on their meshes.
    pub(crate) fn apply_body_fit(&mut self, fit: &AnatomyBodyFit) {
        if fit.identity || self.positions.is_empty() {
            return;
        }
        fit.apply_in_place(&mut self.positions, self.gmin, self.gmax);
        if !self.nodes.is_empty() {
            let mut pts: Vec<[f32; 3]> =
                self.nodes.iter().map(|n| [n.x, n.y, n.z]).collect();
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
    }

    /// One global centre + scale so the scene fits ~1.7 of the orbit frame.
    /// Nodes ride the same transform so pick hits land on the drawn geometry.
    pub(crate) fn normalise_to_orbit_frame(&mut self) {
        if self.organs_loaded == 0 {
            return;
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
    }
}
