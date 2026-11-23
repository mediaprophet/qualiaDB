//! Cold HMC assembly admission, framed native draw groups, and stable leaf identity.

use super::{
    admit_leaf_assembly, decode_load_10d_in_frame_with_emf, decode_sdfb_cutaway_with_emf,
    AdmittedAssemblyLeaf, Load10dGpuBuffers, PickSurfaceRegion, SectionPlane,
};
use crate::{
    bundle::{
        semantic_model::{open_semantic_model, valid_iri, SEMANTIC_MODEL_KEY},
        BundleReader,
    },
    container_10d::{
        MaterialRecord, SubmeshRange, TypedFieldRecord, TypedPayloadResolver,
        FIELD_KIND_ARCHETYPE_REF, FIELD_KIND_INSTANCE_BATCH, FIELD_KIND_INSTANCE_OVERRIDES,
        MANIFOLD_IDENTITY_V2_MAX_FIELDS, MAX_MATERIALS,
    },
    render::{
        instance_culling::GpuInstanceRecord,
        mesh_surface_frames::vertex_normals_from_indices_into,
        mesh_tangent_frames::tangents_from_uvs_into,
        portal_emf_binding::{
            resolve_bundle_emf_registrations, BoundLeafEmfCompanion, EmfCompanionRegistration,
        },
        portal_emf_state::LoadedEmf,
        portal_load_emf::decode_emf_scientific,
        skeletal::{valid_motor, Motor},
    },
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const MAX_VERTICES: usize = 1_048_576;
const MAX_INDICES: usize = 6_291_456;
const MAX_MATERIAL_RANGES: usize = 4_096;
const MAX_SECTION_GROUPS: usize = 512;
const MAX_SECTION_REFERENCE_OCCURRENCES: usize = 4_096;
const MAX_SECTION_ASSEMBLY_DEPTH: usize = 16;

type Transform = [[f64; 4]; 4];

pub struct LoadAssemblyGpuBuffers {
    pub positions: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub uv0: Vec<[f32; 2]>,
    pub normals: Vec<[f32; 3]>,
    pub tangents: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub materials: Vec<MaterialRecord>,
    pub ranges: Vec<SubmeshRange>,
    /// Native draw descriptors, in first authored use order.
    pub descriptors: Vec<[u32; 4]>,
    /// One record per original leaf slot, in unmodified authored order.
    pub instances: Vec<GpuInstanceRecord>,
    pub vertex_count: u32,
    pub triangle_count: u32,
}

pub struct LoadAssemblyCutaway {
    pub draw: LoadAssemblyGpuBuffers,
    pub pick_metadata: Vec<(u32, PickSurfaceRegion)>,
    /// Complete preorder terminal-occurrence table, including leaves clipped
    /// away by this section. GPU picks use these slots.
    pub leaf_occurrences: Vec<SectionLeafOccurrence>,
    pub cut_wall_triangles: u32,
    pub lumen_triangles: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SectionLeafOccurrence {
    /// Depth-first flattened slot. Equals the authored slot of a flat leaf.
    pub instance_slot: u32,
    /// Physical entry key of the terminal .10d asset.
    pub entry_key: String,
    /// Local AREF/IOVR slots from the selected assembly root to this leaf.
    pub authored_slots: Vec<u32>,
}

struct SectionLeaf<'a> {
    occurrence: SectionLeafOccurrence,
    semantic_id: u64,
    bytes: &'a [u8],
    world_from_local: Transform,
}

struct SectionTraversal<'a> {
    bundle: &'a BundleReader<'a>,
    assemblies: BTreeMap<String, Arc<[AdmittedAssemblyLeaf<'a>]>>,
    ancestor_digests: BTreeSet<[u8; 32]>,
    leaves: Vec<SectionLeaf<'a>>,
    remaining_references: usize,
}

impl<'a> SectionTraversal<'a> {
    fn visit(
        &mut self,
        key: &str,
        parent_frame: Option<Transform>,
        authored_slots: &mut Vec<u32>,
    ) -> Result<(), String> {
        if authored_slots.len() >= MAX_SECTION_ASSEMBLY_DEPTH {
            return Err("HMC section exceeds the nested assembly depth limit".into());
        }
        let entry = self.bundle.entry(key).ok_or("Missing HMC section assembly")?;
        let digest: [u8; 32] = entry
            .sha256
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid HMC assembly SHA-256 length")?;
        if !self.ancestor_digests.insert(digest) {
            return Err(format!("Cyclic HMC section assembly reference at {key}"));
        }
        let template = if let Some(template) = self.assemblies.get(key) {
            Arc::clone(template)
        } else {
            // The same bytes may be instanced multiple times. Reuse admission
            // for their entry, but traverse each occurrence in authored order.
            let template: Arc<[AdmittedAssemblyLeaf<'a>]> =
                admit_leaf_assembly(self.bundle, key)?.into();
            self.assemblies.insert(key.to_owned(), Arc::clone(&template));
            template
        };
        self.remaining_references = self
            .remaining_references
            .checked_sub(template.len())
            .ok_or("HMC section exceeds the nested reference occurrence budget")?;
        for child in template.iter() {
            if child.coefficients.iter().any(|c| c.sigma_weight != 0.0) {
                return Err("HMC cutaways require assemblies without SSM1 deformation".into());
            }
            let local_frame = leaf_matrix(
                child.overrides.local_pga_motor,
                child.overrides.uniform_scale,
            )?;
            let frame = match parent_frame {
                Some(parent) => compose_section_frames(&parent, &local_frame)?,
                None => local_frame,
            };
            authored_slots.push(child.slot as u32);
            if declares_leaf_assembly(child.bytes)? {
                self.visit(child.key, Some(frame), authored_slots)?;
            } else {
                if self.leaves.len() >= MAX_SECTION_GROUPS {
                    return Err("HMC section exceeds the terminal leaf occurrence budget".into());
                }
                self.leaves.push(SectionLeaf {
                    occurrence: SectionLeafOccurrence {
                        instance_slot: self.leaves.len() as u32,
                        entry_key: child.key.to_owned(),
                        authored_slots: authored_slots.clone(),
                    },
                    semantic_id: child.semantic_id,
                    bytes: child.bytes,
                    world_from_local: frame,
                });
            }
            authored_slots.pop();
        }
        self.ancestor_digests.remove(&digest);
        Ok(())
    }
}

fn flatten_section_leaves<'a>(
    bundle: &'a BundleReader<'a>,
    key: &str,
) -> Result<Vec<SectionLeaf<'a>>, String> {
    let mut traversal = SectionTraversal {
        bundle,
        assemblies: BTreeMap::new(),
        ancestor_digests: BTreeSet::new(),
        leaves: Vec::new(),
        remaining_references: MAX_SECTION_REFERENCE_OCCURRENCES,
    };
    traversal.visit(key, None, &mut Vec::new())?;
    Ok(traversal.leaves)
}

/// Each column-major frame contains the rotation and uniform scale derived
/// from its PGA placement and its translation expressed in the parent frame.
fn compose_section_frames(parent: &Transform, child: &Transform) -> Result<Transform, String> {
    let frame: Transform = std::array::from_fn(|column| {
        std::array::from_fn(|row| (0..4).map(|k| parent[k][row] * child[column][k]).sum())
    });
    if frame.iter().flatten().any(|value| !value.is_finite())
        || frame[..3].iter().any(|axis| axis[0].hypot(axis[1]).hypot(axis[2]) == 0.0)
    {
        return Err("Nested HMC placement exceeds finite authored metre coordinates".into());
    }
    Ok(frame)
}

/// Resolve only the hashes actually submitted by the decoded section. The HMC
/// model is authoritative for both leaf identities and SDFB region identities.
pub fn resolve_hmc_section_iris(
    bundle: &BundleReader<'_>,
    section: &LoadAssemblyCutaway,
) -> Result<BTreeMap<u64, String>, String> {
    let model = open_semantic_model(bundle).map_err(|error| error.to_string())?;
    let mut page = vec![0; 1024 * 1024];
    let mut token = [0; 4096];
    let mut lexicon = BTreeMap::new();
    for record in &section.draw.instances {
        let hash = record.semantic_id();
        if lexicon.contains_key(&hash) {
            continue;
        }
        let size = model
            .lookup_lexicon_hash_into(hash, &mut page, &mut token)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Unresolved HMC section pick {hash:#018x}"))?;
        let iri = std::str::from_utf8(&token[..size]).map_err(|error| error.to_string())?;
        if hash == 0 || !valid_iri(iri) || crate::q_hash(iri) != hash {
            return Err("Embedded Q42 contains an invalid section q_hash binding".into());
        }
        lexicon.insert(hash, iri.to_owned());
    }
    Ok(lexicon)
}

fn buffers(capacity: usize) -> LoadAssemblyGpuBuffers {
    LoadAssemblyGpuBuffers {
        positions: Vec::new(),
        colors: Vec::new(),
        uv0: Vec::new(),
        normals: Vec::new(),
        tangents: Vec::new(),
        indices: Vec::new(),
        materials: Vec::new(),
        ranges: Vec::new(),
        descriptors: Vec::with_capacity(capacity),
        instances: Vec::with_capacity(capacity),
        vertex_count: 0,
        triangle_count: 0,
    }
}

/// Distinguish assemblies from legacy meshes and standalone population containers
/// with INST records but no external-leaf overrides.
fn declares_leaf_assembly(bytes: &[u8]) -> Result<bool, String> {
    let mut scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let mut fields = scratch;
    let view = TypedPayloadResolver::new(bytes, &[], &mut scratch, &mut fields)
        .map_err(|e| e.to_string())?;
    let count = view.payloads_of_kind(FIELD_KIND_ARCHETYPE_REF).count();
    let has = |kind| view.payloads_of_kind(kind).next().is_some();
    Ok(count > 1
        || (count == 1 && has(FIELD_KIND_INSTANCE_BATCH) && has(FIELD_KIND_INSTANCE_OVERRIDES)))
}

/// Include the actual semantic model and any registered companion EMF digests in the resident key.
/// The root entry already binds each leaf's digest and authored order through AREF.
pub fn assembly_residency_digest(
    bundle: &BundleReader<'_>,
    assembly: &[u8],
    root_digest: [u8; 32],
) -> Result<Option<[u8; 32]>, String> {
    if !declares_leaf_assembly(assembly)? {
        return Ok(None);
    }
    let model = match (
        bundle.entry(SEMANTIC_MODEL_KEY),
        bundle.entry("manifest.q42"),
    ) {
        (Some(entry), None) | (None, Some(entry)) if entry.kind == "q42" => entry,
        _ => return Err("Native leaf assembly requires one embedded semantic model".into()),
    };
    let registrations = resolve_bundle_emf_registrations(bundle)?;
    let mut hasher = Sha256::new();
    hasher.update(b"qualia:portal:leaf-assembly:v1");
    hasher.update(root_digest);
    hasher.update(&model.sha256);
    for (geom_key, reg) in &registrations {
        hasher.update(geom_key.as_bytes());
        hasher.update([0]);
        hasher.update(reg.field_member.as_bytes());
        hasher.update([0]);
        if let Some(sha) = reg.field_sha256 {
            hasher.update(sha);
        }
        for col in &reg.field_from_leaf {
            for val in col {
                hasher.update(val.to_bits().to_le_bytes());
            }
        }
        hasher.update(reg.authored_wavelength_nm.unwrap_or(0.0).to_bits().to_le_bytes());
    }
    Ok(Some(hasher.finalize().into()))
}

fn resolve_bundle_leaf_companions(
    bundle: &BundleReader<'_>,
    admitted_emf: Option<&LoadedEmf>,
) -> Result<(BTreeMap<String, EmfCompanionRegistration>, BTreeMap<String, LoadedEmf>), String> {
    let mut registrations = resolve_bundle_emf_registrations(bundle)?;
    let mut decoded_fields: BTreeMap<String, LoadedEmf> = BTreeMap::new();
    for reg in registrations.values() {
        if decoded_fields.contains_key(&reg.field_member) {
            continue;
        }
        if let Some(admitted) = admitted_emf {
            if reg.field_sha256.is_none_or(|sha| sha == admitted.source_sha256()) {
                continue;
            }
        }
        let emf_bytes = bundle
            .get(&reg.field_member)
            .ok_or_else(|| format!("Missing registered EMF companion {:?}", reg.field_member))?;
        let loaded = decode_emf_scientific(emf_bytes)?;
        decoded_fields.insert(reg.field_member.clone(), loaded);
    }
    // If an admitted_emf was explicitly loaded via load_emf and matches a single-companion bundle
    // or overrides the active field, allow it to bind when its digest matches or when a leaf
    // has no bundle-embedded .emf entry.
    if let Some(admitted) = admitted_emf {
        for reg in registrations.values_mut() {
            if !decoded_fields.contains_key(&reg.field_member) {
                reg.field_sha256 = Some(admitted.source_sha256());
            }
        }
    }
    Ok((registrations, decoded_fields))
}

fn bind_leaf_companion<'a>(
    leaf_key: &str,
    leaf_bytes: &[u8],
    registrations: &BTreeMap<String, EmfCompanionRegistration>,
    decoded_fields: &'a BTreeMap<String, LoadedEmf>,
    admitted_emf: Option<&'a LoadedEmf>,
) -> Result<Option<BoundLeafEmfCompanion<'a>>, String> {
    let Some(reg) = registrations.get(leaf_key) else {
        return Ok(None);
    };
    let field = if let Some(admitted) = admitted_emf {
        if reg.field_sha256 == Some(admitted.source_sha256())
            || !decoded_fields.contains_key(&reg.field_member)
        {
            admitted
        } else {
            decoded_fields
                .get(&reg.field_member)
                .ok_or_else(|| format!("Unresolved EMF companion {:?}", reg.field_member))?
        }
    } else {
        decoded_fields
            .get(&reg.field_member)
            .ok_or_else(|| format!("Unresolved EMF companion {:?}", reg.field_member))?
    };
    let leaf_sha: [u8; 32] = Sha256::digest(leaf_bytes).into();
    Ok(Some(BoundLeafEmfCompanion::bind(
        reg.clone(),
        leaf_sha,
        field,
        0,
    )?))
}

/// Build one bounded native submission. Adjacent references to the same verified
/// template share geometry and draw ranges while retaining each source slot.
pub fn decode_hmc_leaf_assembly(
    bundle: &BundleReader<'_>,
    assembly_key: &str,
) -> Result<LoadAssemblyGpuBuffers, String> {
    decode_hmc_leaf_assembly_with_emf(bundle, assembly_key, None)
}

/// Build one bounded native submission, binding any registered `.emf` optical companions
/// in each leaf's authored metre frame before assembly world placement.
pub fn decode_hmc_leaf_assembly_with_emf(
    bundle: &BundleReader<'_>,
    assembly_key: &str,
    admitted_emf: Option<&LoadedEmf>,
) -> Result<LoadAssemblyGpuBuffers, String> {
    let leaves = admit_leaf_assembly(bundle, assembly_key)?;
    let (registrations, decoded_fields) = resolve_bundle_leaf_companions(bundle, admitted_emf)?;
    let mut result = buffers(leaves.len());
    let mut current: Option<(&str, Option<[u8; 32]>)> = None;
    let mut wall_semantic_id = None;
    let mut local_bounds = [[0.0; 3]; 2];
    let mut world_bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let mut placement = Vec::with_capacity(leaves.len());
    for leaf in leaves {
        if leaf.slot != placement.len() {
            return Err("Native leaf assembly slots are not contiguous".into());
        }
        if leaf
            .coefficients
            .iter()
            .any(|coefficient| coefficient.sigma_weight != 0.0)
        {
            return Err(format!(
                "Assembly leaf {} requires SSM1 deformation support",
                leaf.slot
            ));
        }
        let transform = leaf_matrix(leaf.overrides.local_pga_motor, leaf.overrides.uniform_scale)?;
        let leaf_companion = bind_leaf_companion(
            leaf.key,
            leaf.bytes,
            &registrations,
            &decoded_fields,
            admitted_emf,
        )?;
        let companion_digest = leaf_companion.as_ref().map(|c| c.binding_digest());
        if current == Some((leaf.key, companion_digest)) {
            let group = result
                .descriptors
                .last_mut()
                .ok_or("Missing assembly prototype group")?;
            group[3] += 1;
        } else {
            if declares_leaf_assembly(leaf.bytes)? {
                return Err("Nested HMC assemblies require a resolved leaf hierarchy".into());
            }
            let asset =
                decode_load_10d_in_frame_with_emf(leaf.bytes, false, leaf_companion.as_ref())?;
            if asset.governance_refused {
                return Err(format!(
                    "HMC assembly leaf {} is governance-refused",
                    leaf.slot
                ));
            }
            local_bounds = position_bounds(&asset.positions)?;
            wall_semantic_id = if asset.field_representation == "SignedDistanceBrickSurface" {
                asset
                    .ranges
                    .first()
                    .map(|range| range.semantic_id)
                    .filter(|&wall| asset.ranges.iter().all(|range| range.semantic_id == wall))
            } else {
                None
            };
            result.descriptors.push([
                result.indices.len() as u32,
                asset.indices.len() as u32,
                leaf.slot as u32,
                1,
            ]);
            append_mesh(&mut result, asset, leaf.semantic_id)?;
            current = Some((leaf.key, companion_digest));
        }
        let index_count = result
            .descriptors
            .last()
            .ok_or("Missing assembly prototype group")?[1];
        result.triangle_count = result
            .triangle_count
            .checked_add(index_count / 3)
            .ok_or("HMC expanded assembly triangle count exceeds u32")?;
        update_world_bounds(&mut world_bounds, local_bounds, &transform);
        placement.push((transform, wall_semantic_id.unwrap_or(leaf.semantic_id)));
    }
    fit_instances(&mut result, world_bounds, placement)?;
    Ok(result)
}

/// Slice selected flattened leaf slots in assembly-authored coordinates. Slots
/// follow depth-first authored occurrence order, which preserves flat slot
/// numbers. An empty mask selects every SDFB leaf and preserves ordinary meshes.
pub fn decode_hmc_leaf_cutaway(
    bundle: &BundleReader<'_>,
    assembly_key: &str,
    plane: SectionPlane,
    leaf_slots: &[u32],
) -> Result<LoadAssemblyCutaway, String> {
    decode_hmc_leaf_cutaway_with_emf(bundle, assembly_key, plane, leaf_slots, None)
}

/// Slice selected flattened leaf slots in assembly-authored coordinates, applying any registered
/// `.emf` optical companions to both exterior and cut-tissue surfaces in each leaf's authored metre frame.
pub fn decode_hmc_leaf_cutaway_with_emf(
    bundle: &BundleReader<'_>,
    assembly_key: &str,
    plane: SectionPlane,
    leaf_slots: &[u32],
    admitted_emf: Option<&LoadedEmf>,
) -> Result<LoadAssemblyCutaway, String> {
    let leaves = flatten_section_leaves(bundle, assembly_key)?;
    let (registrations, decoded_fields) = resolve_bundle_leaf_companions(bundle, admitted_emf)?;
    let mask: BTreeSet<_> = leaf_slots.iter().copied().collect();
    if mask.len() != leaf_slots.len() || mask.iter().any(|&slot| slot as usize >= leaves.len()) {
        return Err("SDFB section leaf mask contains invalid or repeated authored slots".into());
    }
    let mut draw = buffers(leaves.len());
    let mut metadata = Vec::new();
    let mut leaf_occurrences = Vec::with_capacity(leaves.len());
    let mut placement = Vec::new();
    let mut world_bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let (mut cut_wall_triangles, mut lumen_triangles) = (0, 0);
    let mut selected = 0;
    for leaf in leaves {
        let slot = leaf.occurrence.instance_slot;
        let leaf_key = leaf.occurrence.entry_key.clone();
        leaf_occurrences.push(leaf.occurrence);
        let transform = leaf.world_from_local;
        let leaf_companion = bind_leaf_companion(
            &leaf_key,
            leaf.bytes,
            &registrations,
            &decoded_fields,
            admitted_emf,
        )?;
        let cut_this = if mask.is_empty() {
            crate::container_10d::sdf_brick_section::read_container_sdfb(leaf.bytes)?.is_some()
        } else {
            mask.contains(&slot)
        };
        let first_index = draw.indices.len() as u32;
        if cut_this {
            selected += 1;
            let cut = decode_sdfb_cutaway_with_emf(
                leaf.bytes,
                plane.in_local_frame(&transform)?,
                leaf_companion.as_ref(),
            )?;
            update_world_bounds(&mut world_bounds, cut.source_bounds_m, &transform);
            if cut.mesh.triangles.is_empty() {
                continue;
            }
            let first_group = placement.len();
            let asset = Load10dGpuBuffers {
                positions: cut.mesh.positions,
                colors: cut.colors,
                uv0: None,
                normals: None,
                tangents: None,
                indices: cut.mesh.triangles.into_iter().flatten().collect(),
                materials: cut.materials,
                ranges: cut.ranges,
                provenance_mu: 0.0,
                governance_refused: false,
                field_representation: "SignedDistanceBrickCutaway",
                holo_segment_count: 0,
                active_harmonic_order: 0,
                vertex_count: 0,
                triangle_count: 0,
            };
            for (index, (range, region)) in asset.ranges.iter().zip(cut.regions).enumerate() {
                draw.descriptors.push([
                    first_index + range.first_index,
                    range.index_count,
                    (first_group + index) as u32,
                    1,
                ]);
                placement.push((transform, range.semantic_id));
                metadata.push((slot, region));
                match region {
                    PickSurfaceRegion::UncutSurface => {}
                    PickSurfaceRegion::CutTissueWall => cut_wall_triangles += range.index_count / 3,
                    PickSurfaceRegion::InternalCavityLumen => {
                        lumen_triangles += range.index_count / 3
                    }
                }
            }
            append_mesh(&mut draw, asset, leaf.semantic_id)?;
        } else {
            let asset =
                decode_load_10d_in_frame_with_emf(leaf.bytes, false, leaf_companion.as_ref())?;
            if asset.governance_refused {
                return Err(format!("HMC section leaf {slot} is governance-refused"));
            }
            update_world_bounds(
                &mut world_bounds,
                position_bounds(&asset.positions)?,
                &transform,
            );
            let semantic_id = if asset.field_representation == "SignedDistanceBrickSurface" {
                asset
                    .ranges
                    .first()
                    .map(|range| range.semantic_id)
                    .filter(|&wall| asset.ranges.iter().all(|range| range.semantic_id == wall))
            } else {
                None
            };
            draw.descriptors.push([
                first_index,
                asset.indices.len() as u32,
                placement.len() as u32,
                1,
            ]);
            placement.push((transform, semantic_id.unwrap_or(leaf.semantic_id)));
            metadata.push((slot, PickSurfaceRegion::UncutSurface));
            append_mesh(&mut draw, asset, leaf.semantic_id)?;
        }
        if placement.len() > MAX_SECTION_GROUPS {
            return Err("HMC section exceeds the GPU prototype-group budget".into());
        }
    }
    if selected == 0 {
        return Err("HMC section mask did not select a verified SDFB leaf".into());
    }
    draw.triangle_count = (draw.indices.len() / 3) as u32;
    fit_instances(&mut draw, world_bounds, placement)?;
    Ok(LoadAssemblyCutaway {
        draw,
        pick_metadata: metadata,
        leaf_occurrences,
        cut_wall_triangles,
        lumen_triangles,
    })
}

fn fit_instances(
    result: &mut LoadAssemblyGpuBuffers,
    world_bounds: [[f64; 3]; 2],
    placement: Vec<([[f64; 4]; 4], u64)>,
) -> Result<(), String> {
    let [min, max] = world_bounds;
    let span = (0..3)
        .map(|axis| max[axis] - min[axis])
        .fold(0.0_f64, f64::max);
    if !span.is_finite() || span <= 0.0 {
        return Err("Invalid world bounds for native leaf assembly".into());
    }
    let center: [f64; 3] = std::array::from_fn(|axis| (max[axis] + min[axis]) * 0.5);
    let fit = 1.6 / span.max(1e-6);
    for (mut matrix, identity) in placement {
        for (axis, center) in center.into_iter().enumerate() {
            matrix[3][axis] -= center;
            for column in &mut matrix {
                column[axis] *= fit;
            }
        }
        let matrix = matrix.map(|column| column.map(|value| value as f32));
        let instance = GpuInstanceRecord::new(matrix, identity);
        if instance
            .world_from_local
            .iter()
            .flatten()
            .chain(instance.normal_from_local.iter().flatten())
            .any(|v| !v.is_finite())
        {
            return Err("Assembly transform exceeds GPU position or normal precision".into());
        }
        result.instances.push(instance);
    }
    result.vertex_count =
        u32::try_from(result.positions.len()).map_err(|_| "Assembly vertex count exceeds u32")?;
    Ok(())
}

fn leaf_matrix(pga: [f32; 8], scale: f32) -> Result<[[f64; 4]; 4], String> {
    let [s, e23, e31, e12, e01, e02, e03, e0123] = pga.map(f64::from);
    let motor = Motor {
        r_w: s,
        r_x: e23,
        r_y: e31,
        r_z: e12,
        d_w: e0123,
        d_x: -e01,
        d_y: -e02,
        d_z: -e03,
    };
    if !valid_motor(&motor) || !scale.is_finite() || scale <= 0.0 {
        return Err("Assembly transform requires a unit PGA motor and positive scale".into());
    }
    let translation = motor.transform_point([0.0; 3]);
    // Rotate independently of the translation so a small leaf axis retains
    // its precision even when the authored origin is far from the model.
    let rotation = Motor {
        d_w: 0.0,
        d_x: 0.0,
        d_y: 0.0,
        d_z: 0.0,
        ..motor
    };
    let mut matrix = [[0.0; 4]; 4];
    for (axis, column) in matrix[..3].iter_mut().enumerate() {
        let mut point = [0.0; 3];
        point[axis] = f64::from(scale);
        column[..3].copy_from_slice(&rotation.transform_point(point));
    }
    matrix[3] = [translation[0], translation[1], translation[2], 1.0];
    Ok(matrix)
}

fn position_bounds(positions: &[[f32; 3]]) -> Result<[[f32; 3]; 2], String> {
    if positions.is_empty() || positions.iter().flatten().any(|v| !v.is_finite()) {
        return Err("HMC assembly leaf has empty or non-finite geometry".into());
    }
    let (mut min, mut max) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
    for point in positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    Ok([min, max])
}

fn update_world_bounds(bounds: &mut [[f64; 3]; 2], local: [[f32; 3]; 2], matrix: &[[f64; 4]; 4]) {
    for corner in 0..8 {
        let point: [f64; 3] =
            std::array::from_fn(|axis| f64::from(local[(corner >> axis) & 1][axis]));
        for axis in 0..3 {
            let value = point
                .iter()
                .enumerate()
                .fold(matrix[3][axis], |sum, (col, &value)| {
                    sum + matrix[col][axis] * value
                });
            bounds[0][axis] = bounds[0][axis].min(value);
            bounds[1][axis] = bounds[1][axis].max(value);
        }
    }
}

fn append_mesh(
    result: &mut LoadAssemblyGpuBuffers,
    mut mesh: Load10dGpuBuffers,
    identity: u64,
) -> Result<(), String> {
    let vertices = result.positions.len();
    let indices = result.indices.len();
    if mesh.positions.len() > MAX_VERTICES.saturating_sub(vertices)
        || mesh.indices.len() > MAX_INDICES.saturating_sub(indices)
        || mesh.indices.is_empty()
    {
        return Err("HMC assembly geometry exceeds the native draw budget".into());
    }
    let len = mesh.positions.len();
    let uv = mesh.uv0.take().unwrap_or_else(|| vec![[0.0; 2]; len]);
    let normals = match mesh.normals.take() {
        Some(normals) => normals,
        None => {
            let mut output = vec![[0.0; 3]; len];
            vertex_normals_from_indices_into(&mesh.positions, &mesh.indices, &mut output)
                .map_err(|e| format!("Assembly mesh normals: {e:?}"))?;
            output
        }
    };
    let tangents = match mesh.tangents.take() {
        Some(tangents) => tangents,
        None => {
            let mut output = vec![[0.0; 4]; len];
            let mut scratch = vec![[0.0; 6]; len];
            tangents_from_uvs_into(
                &mesh.positions,
                &uv,
                &normals,
                &mesh.indices,
                &mut output,
                &mut scratch,
            )
            .map_err(|e| format!("Assembly mesh tangents: {e:?}"))?;
            output
        }
    };
    if mesh.materials.is_empty() {
        let default = MaterialRecord::legacy_default();
        mesh.ranges.push(SubmeshRange {
            first_index: 0,
            index_count: mesh.indices.len() as u32,
            material_id: default.id,
            semantic_id: identity,
        });
        mesh.materials.push(default);
    }
    if mesh.materials.len() > MAX_MATERIALS.saturating_sub(result.materials.len())
        || mesh.ranges.len() > MAX_MATERIAL_RANGES.saturating_sub(result.ranges.len())
    {
        return Err("Assembly material count exceeds the native draw budget".into());
    }
    let mut material_map = BTreeMap::new();
    for mut material in mesh.materials {
        let old = material.id;
        material.id = result.materials.len() as u64 + 1;
        material_map.insert(old, material.id);
        result.materials.push(material);
    }
    for mut range in mesh.ranges {
        range.first_index += indices as u32;
        range.material_id = *material_map
            .get(&range.material_id)
            .ok_or("Assembly material range references a missing material")?;
        result.ranges.push(range);
    }
    result.positions.extend(mesh.positions);
    if mesh.colors.is_empty() {
        result
            .colors
            .resize(result.positions.len(), [0.63, 0.68, 0.77, 1.0]);
    } else {
        result.colors.extend(mesh.colors);
    }
    result.uv0.extend(uv);
    result.normals.extend(normals);
    result.tangents.extend(tangents);
    result.indices.extend(
        mesh.indices
            .into_iter()
            .map(|index| index + vertices as u32),
    );
    Ok(())
}

#[cfg(feature = "gpu-runtime")]
impl LoadAssemblyGpuBuffers {
    pub fn upload(&self, gpu: &mut crate::render::gpu::PortalGpu) -> Result<(), String> {
        let packed = bytemuck::cast_slice(&self.instances);
        etalon_render::instance_stream::validate_dirty_ranges_packed(
            packed,
            &[etalon_render::instance_stream::DirtyInstanceRange::new(
                0,
                self.instances.len() as u32,
            )],
        )
        .map_err(|e| format!("HMC assembly instance admission: {e:?}"))?;
        gpu.upload_mesh_colored_with_frames_and_materials_and_uv0(
            &self.positions,
            &self.colors,
            Some(&self.normals),
            Some(&self.tangents),
            Some(&self.uv0),
            &self.indices,
            &self.materials,
            &self.ranges,
        )?;
        if let Err(error) =
            gpu.set_mesh_instance_groups_packed(bytemuck::cast_slice(&self.descriptors), packed)
        {
            // The local mesh cannot be displayed with a missing instance transform.
            gpu.upload_mesh(&[], &[]);
            return Err(error);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembly_matrix_applies_authored_motor_and_uniform_scale_separately() {
        let motor = Motor::from_translation(2.0, -3.0, 4.0);
        let pga = [
            motor.r_w, motor.r_x, motor.r_y, motor.r_z, -motor.d_x, -motor.d_y, -motor.d_z,
            motor.d_w,
        ]
        .map(|v| v as f32);
        let matrix = leaf_matrix(pga, 2.0).unwrap();
        assert_eq!(matrix[0], [2.0, 0.0, 0.0, 0.0]);
        assert_eq!(matrix[3], [2.0, -3.0, 4.0, 1.0]);
    }

    #[test]
    fn nested_section_applies_parent_rotation_and_scale_to_child_translation() {
        let parent = [
            [0.0, 2.0, 0.0, 0.0],
            [-2.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 2.0, 0.0],
            [10.0, 20.0, 30.0, 1.0],
        ];
        let child = [
            [0.5, 0.0, 0.0, 0.0],
            [0.0, 0.5, 0.0, 0.0],
            [0.0, 0.0, 0.5, 0.0],
            [3.0, 4.0, 5.0, 1.0],
        ];
        let world = compose_section_frames(&parent, &child).unwrap();
        assert_eq!(world[3], [2.0, 26.0, 40.0, 1.0]);
        assert_eq!(world[0], [0.0, 1.0, 0.0, 0.0]);
        assert_eq!(world[1], [-1.0, 0.0, 0.0, 0.0]);
        let plane = SectionPlane::new([1.0, 0.0, 0.0, -2.0], 0.2).unwrap();
        assert_eq!(
            plane.in_local_frame(&world).unwrap(),
            SectionPlane::new([0.0, -1.0, 0.0, 0.0], 0.2).unwrap()
        );
    }
}
