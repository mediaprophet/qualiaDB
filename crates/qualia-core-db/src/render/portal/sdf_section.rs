//! Browser entry points for immutable SDFB section sources.

use super::{MountedEmfTarget, QualiaPortal};
use crate::render::gpu::PortalGpu;
use crate::render::portal_emf_binding::{BoundLeafEmfCompanion, EmfCompanionRegistration};
use crate::render::portal_load_10d::{
    PickSurfaceRegion, SectionLeafOccurrence, SectionPlane, SemanticSurfacePick,
};
use js_sys::{Array, Object, Reflect};
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl QualiaPortal {
    /// Queue the nearest visible pixel within a radius of 0 to 3 canvas pixels.
    /// Region codes are 0 for uncut surface, 1 for cut tissue, and 2 for lumen.
    /// Selection completes on a later tick and uses the existing pick accessors.
    pub fn select_section_at(
        &mut self,
        x: f32,
        y: f32,
        radius_px: u32,
        surface_region: u32,
    ) -> Result<(), JsValue> {
        let region = u8::try_from(surface_region)
            .ok()
            .and_then(PickSurfaceRegion::from_u8)
            .ok_or_else(|| JsValue::from_str("Invalid section surface region"))?;
        let radius_px = u8::try_from(radius_px)
            .map_err(|_| JsValue::from_str("Invalid section pick radius"))?;
        self.gpu
            .as_mut()
            .ok_or_else(|| JsValue::from_str("SDFB section picking requires active WebGPU"))?
            .queue_nearest_section_pick(x, y, radius_px, region)
            .map_err(|error| JsValue::from_str(&error))?;
        self.selected_node = None;
        self.selected_semantic_pick = None;
        self.pending_gpu_pick = true;
        Ok(())
    }

    /// Resolve a canonical RDF IRI for the exact currently resident section tuple.
    /// Returns an empty string if the tuple is outside the resident section.
    pub fn section_iri_for_pick(
        &self,
        instance_slot: u32,
        semantic_id_hex: &str,
        surface_region: u8,
    ) -> Result<String, JsValue> {
        if semantic_id_hex.len() != 16 {
            return Err(JsValue::from_str(
                "Section semantic ID requires 16 hexadecimal digits",
            ));
        }
        let semantic_id = u64::from_str_radix(semantic_id_hex, 16)
            .map_err(|_| JsValue::from_str("Invalid section semantic ID"))?;
        let surface_region = PickSurfaceRegion::from_u8(surface_region)
            .ok_or_else(|| JsValue::from_str("Invalid section surface region"))?;
        let pick = SemanticSurfacePick {
            instance_slot,
            semantic_id,
            surface_region,
        };
        Ok(self
            .gpu
            .as_ref()
            .and_then(|gpu| gpu.section_iri_for_pick(pick))
            .unwrap_or_default()
            .into())
    }

    /// Canonical RDF IRI of the latest admitted section pick.
    pub fn selected_section_pick_iri(&self) -> String {
        self.gpu
            .as_ref()
            .zip(self.selected_semantic_pick)
            .and_then(|(gpu, pick)| gpu.section_iri_for_pick(pick))
            .unwrap_or_default()
            .into()
    }

    /// Invert an admitted IRI to one or more stable section pick tuples.
    pub fn section_picks_for_iri(&self, iri: &str) -> Result<JsValue, JsValue> {
        let matches = Array::new();
        if let Some(gpu) = self.gpu.as_ref() {
            for pick in gpu.section_picks_for_iri(iri) {
                let address = Object::new();
                let hex = format!("{:016x}", pick.semantic_id);
                for (name, value) in [
                    (
                        "instance_slot",
                        JsValue::from_f64(f64::from(pick.instance_slot)),
                    ),
                    ("semantic_id_hex", JsValue::from_str(&hex)),
                    (
                        "surface_region",
                        JsValue::from_f64(f64::from(pick.surface_region as u8)),
                    ),
                ] {
                    Reflect::set(&address, &JsValue::from_str(name), &value)?;
                }
                matches.push(&address);
            }
        }
        Ok(matches.into())
    }

    /// Slice a verified HMC assembly in its authored metre frame. An empty
    /// mask selects all SDFB leaves. Slots use terminal occurrence order and
    /// the response includes their authored paths through the assembly.
    pub fn load_hmc_section_cutaway(
        &mut self,
        hmc: &[u8],
        assembly_key: &str,
        plane_abcd: &[f64],
        slab_thickness_m: f64,
        leaf_slots: &[u32],
    ) -> Result<JsValue, JsValue> {
        let plane = parse_plane(plane_abcd, slab_thickness_m)?;
        let bundle = crate::bundle::BundleReader::parse(hmc)
            .map_err(|error| JsValue::from_str(&format!("HMC section: {error}")))?;
        let admitted_emf = self.last_emf.accepted();
        let gpu = self
            .gpu
            .as_mut()
            .ok_or_else(|| JsValue::from_str("SDFB section requires active WebGPU"))?;
        let prior_generation = gpu.mesh_generation();
        let result = gpu.set_hmc_section_plane_with_emf(
            &bundle,
            assembly_key,
            plane,
            leaf_slots,
            admitted_emf,
        );
        if result.is_ok() || gpu.mesh_generation() != prior_generation {
            self.hmc_mesh_residency.clear();
            self.semantic_motion = Default::default();
            self.selected_node = None;
            self.selected_semantic_pick = None;
            self.pending_gpu_pick = false;
        }
        let (counts, occurrences) = result.map_err(|error| JsValue::from_str(&error))?;
        self.mounted_emf_target = MountedEmfTarget::HmcCutaway {
            hmc_bytes: std::sync::Arc::from(hmc.to_vec().into_boxed_slice()),
            assembly_key: assembly_key.to_owned(),
            plane,
            leaf_slots: leaf_slots.to_vec(),
        };
        self.tier = 2;
        self.description = format!(
            "{} tissue cut triangles · {} lumen aperture triangles · T2",
            counts[2], counts[3]
        );
        section_result(counts, &occurrences)
    }

    /// Cut a sealed SDFB .10d in authored metres. The first three values form
    /// the normal; the fourth is the offset. A zero slab retains the negative
    /// half-space, and a positive slab retains its centered finite thickness.
    pub fn set_section_plane(
        &mut self,
        sdfb_10d: &[u8],
        plane_abcd: &[f64],
        slab_thickness_m: f64,
    ) -> Result<JsValue, JsValue> {
        let plane = parse_plane(plane_abcd, slab_thickness_m)?;
        let leaf_digest: [u8; 32] = Sha256::digest(sdfb_10d).into();
        let registration = match &self.mounted_emf_target {
            MountedEmfTarget::HmcLeaf {
                leaf_digest: d,
                registration,
                ..
            }
            | MountedEmfTarget::Standalone10d {
                leaf_digest: d,
                registration,
                ..
            }
            | MountedEmfTarget::StandaloneCutaway {
                leaf_digest: d,
                registration,
                ..
            } if *d == leaf_digest => registration.clone(),
            _ => None,
        };
        let companion = self.last_emf.accepted().and_then(|field| {
            let mut reg = registration.clone().unwrap_or_else(|| {
                EmfCompanionRegistration::identity_metres("standalone.10d", "admitted.emf")
            });
            if reg.geometry_sha256.is_none() {
                reg.geometry_sha256 = Some(leaf_digest);
            }
            if reg.field_sha256.is_none() {
                reg.field_sha256 = Some(field.source_sha256());
            }
            BoundLeafEmfCompanion::bind(reg, leaf_digest, field, 0).ok()
        });
        let gpu = self
            .gpu
            .as_mut()
            .ok_or_else(|| JsValue::from_str("SDFB section requires active WebGPU"))?;
        let prior_generation = gpu.mesh_generation();
        let result = gpu.set_section_plane_with_emf(sdfb_10d, plane, companion.as_ref());
        if result.is_ok() || gpu.mesh_generation() != prior_generation {
            self.hmc_mesh_residency.clear();
            self.semantic_motion = Default::default();
            self.selected_node = None;
            self.selected_semantic_pick = None;
            self.pending_gpu_pick = false;
        }
        let counts = result.map_err(|error| JsValue::from_str(&error))?;
        self.mounted_emf_target = MountedEmfTarget::StandaloneCutaway {
            leaf_bytes: std::sync::Arc::from(sdfb_10d.to_vec().into_boxed_slice()),
            leaf_digest,
            plane,
            registration,
        };
        self.tier = 2;
        self.description = format!(
            "{} tissue cut triangles · {} lumen aperture triangles · T2",
            counts[2], counts[3]
        );
        section_result(counts, &[])
    }
}

impl QualiaPortal {
    pub(crate) fn refresh_mounted_target_with_emf(&mut self) -> Result<(), String> {
        use crate::render::portal_load_10d::{
            assembly_residency_digest, decode_hmc_leaf_assembly_with_emf, decode_load_10d_with_emf,
        };
        let target = self.mounted_emf_target.clone();
        match target {
            MountedEmfTarget::None => Ok(()),
            MountedEmfTarget::HmcLeaf {
                asset_key,
                leaf_bytes,
                leaf_digest,
                registration,
            } => {
                let bound = match (self.last_emf.accepted(), registration.as_ref()) {
                    (Some(field), Some(reg))
                        if reg
                            .field_sha256
                            .is_none_or(|sha| sha == field.source_sha256()) =>
                    {
                        Some(BoundLeafEmfCompanion::bind(
                            reg.clone(),
                            leaf_digest,
                            field,
                            0,
                        )?)
                    }
                    (Some(field), None) => {
                        let mut reg =
                            EmfCompanionRegistration::identity_metres(&asset_key, "admitted.emf");
                        reg.geometry_sha256 = Some(leaf_digest);
                        reg.field_sha256 = Some(field.source_sha256());
                        BoundLeafEmfCompanion::bind(reg, leaf_digest, field, 0).ok()
                    }
                    _ => None,
                };
                let digest = if let Some(ref b) = bound {
                    super::combine_residency_digest_with_emf(leaf_digest, &[(0, b.binding_digest())])
                } else {
                    leaf_digest
                };
                if self.hmc_mesh_residency.resident(digest).is_some() {
                    return Ok(());
                }
                let decoded = decode_load_10d_with_emf(&leaf_bytes, true, bound.as_ref())?;
                if let Some(ref mut gpu) = self.gpu {
                    crate::render::portal_load_10d::upload_load_10d_to_gpu(gpu, &decoded)?;
                    self.tier = 2;
                }
                self.hmc_mesh_residency.record_upload(
                    digest,
                    decoded.vertex_count,
                    decoded.triangle_count.saturating_mul(3),
                    decoded.triangle_count,
                );
                Ok(())
            }
            MountedEmfTarget::HmcAssembly {
                hmc_bytes,
                assembly_key,
            } => {
                let bundle = crate::bundle::BundleReader::parse(&hmc_bytes)
                    .map_err(|e| format!("HMC bundle: {e}"))?;
                let source = super::resolve_hmc_mesh_in_bundle(&bundle, &assembly_key)?;
                let base_digest =
                    assembly_residency_digest(&bundle, source.bytes, source.digest)?
                        .unwrap_or(source.digest);
                let digest = if let Some(field) = self.last_emf.accepted() {
                    super::combine_residency_digest_with_emf(
                        base_digest,
                        &[(0, field.source_sha256())],
                    )
                } else {
                    base_digest
                };
                if self.hmc_mesh_residency.resident(digest).is_some() {
                    return Ok(());
                }
                let assembly = decode_hmc_leaf_assembly_with_emf(
                    &bundle,
                    &assembly_key,
                    self.last_emf.accepted(),
                )?;
                if let Some(ref mut gpu) = self.gpu {
                    assembly.upload(gpu)?;
                    self.tier = 2;
                }
                self.hmc_mesh_residency.record_upload(
                    digest,
                    assembly.vertex_count,
                    assembly.triangle_count.saturating_mul(3),
                    assembly.triangle_count,
                );
                Ok(())
            }
            MountedEmfTarget::Standalone10d {
                leaf_bytes,
                leaf_digest,
                registration,
            } => {
                let bound = self.last_emf.accepted().and_then(|field| {
                    let mut reg = registration.unwrap_or_else(|| {
                        EmfCompanionRegistration::identity_metres("standalone.10d", "admitted.emf")
                    });
                    if reg.geometry_sha256.is_none() {
                        reg.geometry_sha256 = Some(leaf_digest);
                    }
                    if reg.field_sha256.is_none() {
                        reg.field_sha256 = Some(field.source_sha256());
                    }
                    BoundLeafEmfCompanion::bind(reg, leaf_digest, field, 0).ok()
                });
                let decoded = decode_load_10d_with_emf(&leaf_bytes, true, bound.as_ref())?;
                if let Some(ref mut gpu) = self.gpu {
                    crate::render::portal_load_10d::upload_load_10d_to_gpu(gpu, &decoded)?;
                    self.tier = 2;
                }
                Ok(())
            }
            MountedEmfTarget::StandaloneCutaway {
                leaf_bytes,
                leaf_digest,
                plane,
                registration,
            } => {
                let bound = self.last_emf.accepted().and_then(|field| {
                    let mut reg = registration.unwrap_or_else(|| {
                        EmfCompanionRegistration::identity_metres("standalone.10d", "admitted.emf")
                    });
                    if reg.geometry_sha256.is_none() {
                        reg.geometry_sha256 = Some(leaf_digest);
                    }
                    if reg.field_sha256.is_none() {
                        reg.field_sha256 = Some(field.source_sha256());
                    }
                    BoundLeafEmfCompanion::bind(reg, leaf_digest, field, 0).ok()
                });
                if let Some(ref mut gpu) = self.gpu {
                    gpu.set_section_plane_with_emf(&leaf_bytes, plane, bound.as_ref())?;
                    self.tier = 2;
                }
                Ok(())
            }
            MountedEmfTarget::HmcCutaway {
                hmc_bytes,
                assembly_key,
                plane,
                leaf_slots,
            } => {
                let bundle = crate::bundle::BundleReader::parse(&hmc_bytes)
                    .map_err(|e| format!("HMC section: {e}"))?;
                let admitted_emf = self.last_emf.accepted();
                if let Some(ref mut gpu) = self.gpu {
                    gpu.set_hmc_section_plane_with_emf(
                        &bundle,
                        &assembly_key,
                        plane,
                        &leaf_slots,
                        admitted_emf,
                    )?;
                    self.tier = 2;
                }
                Ok(())
            }
        }
    }
}

fn parse_plane(plane: &[f64], slab_thickness_m: f64) -> Result<SectionPlane, JsValue> {
    let plane = <[f64; 4]>::try_from(plane)
        .map_err(|_| JsValue::from_str("Section plane requires [a, b, c, d]"))?;
    SectionPlane::new(plane, slab_thickness_m).map_err(|error| JsValue::from_str(&error))
}

fn section_result(
    counts: [u32; 4],
    occurrences: &[SectionLeafOccurrence],
) -> Result<JsValue, JsValue> {
    let result = Object::new();
    for (key, value) in [
        "vertex_count",
        "triangle_count",
        "cut_wall_triangles",
        "lumen_triangles",
    ]
    .into_iter()
    .zip(counts)
    {
        Reflect::set(
            &result,
            &JsValue::from_str(key),
            &JsValue::from_f64(f64::from(value)),
        )?;
    }
    if !occurrences.is_empty() {
        let leaves = Array::new();
        for occurrence in occurrences {
            let leaf = Object::new();
            let slots = Array::new();
            for &slot in &occurrence.authored_slots {
                slots.push(&JsValue::from_f64(f64::from(slot)));
            }
            for (name, value) in [
                ("instance_slot", JsValue::from_f64(f64::from(occurrence.instance_slot))),
                ("entry_key", JsValue::from_str(&occurrence.entry_key)),
                ("authored_slots", slots.into()),
            ] {
                Reflect::set(&leaf, &JsValue::from_str(name), &value)?;
            }
            leaves.push(&leaf);
        }
        Reflect::set(&result, &JsValue::from_str("leaf_occurrences"), &leaves)?;
    }
    Ok(result.into())
}
