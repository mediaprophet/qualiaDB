//! MRP1 deformation bound to matching local CBLD and mounted Q42 identities.

use bytemuck::Zeroable;

use super::{graph::index, MacroChannel, MacroValue, MotionGraph, SemanticMotionError as Error};
use crate::{
    container_10d::{
        apply_morph_targets_zero_heap, decode_morph_section_into, decode_symbolic_rdf_cbor_ld,
        integrity::check_whole_file_crc32c, parse_mesh_header, MorphSectionHeader,
        QuantizedMorphDelta, SectionType, SymbolicRdfCborLdPayload, TypedFieldRecord,
        TypedPayloadResolver, FIELD_KIND_SYMBOLIC_RDF_CBOR_LD, FLAG_MORPH_HAS_NORMALS,
        GRAPHICS_ASSET_RING_BYTES, MANIFOLD_IDENTITY_V2_MAX_FIELDS, MAX_MORPH_DELTAS,
        MAX_MORPH_TARGETS, MORPH_SECTION_HEADER_SIZE, QUANTIZED_MORPH_DELTA_SIZE,
    },
    q_hash, NQuin,
};

/// Local and bundle predicate relating a target IRI to its zero-based MRP1 slot.
pub const MORPH_TARGET_SLOT_IRI: &str = "https://qualiadb.org/ns/kinematics#morphTargetSlot";
/// Local and bundle predicate relating a target IRI to its authored skin group.
pub const MORPH_TARGET_GROUP_IRI: &str = "https://qualiadb.org/ns/kinematics#morphTargetGroup";
const SLOT: u64 = q_hash(MORPH_TARGET_SLOT_IRI);
const GROUP: u64 = q_hash(MORPH_TARGET_GROUP_IRI);

/// Validated sparse deformation for one geometry binding and its requested macro targets.
pub struct PreparedMorphTargets {
    header: MorphSectionHeader,
    deltas: Vec<QuantizedMorphDelta>,
    semantic_by_slot: [u64; MAX_MORPH_TARGETS],
    bound_count: usize,
}

impl PreparedMorphTargets {
    /// Decode the physical morph field at setup time. Slot and group claims must
    /// agree with the local CBLD facts in the same named graph as the Q42 macro.
    /// `vertex_count` describes the unchanged authored quantized topology.
    pub fn bind(
        asset: &[u8],
        vertex_count: usize,
        graph: &mut MotionGraph<'_, '_, '_>,
        group: u64,
        target_ids: &[u64],
        delta_budget_bytes: usize,
    ) -> Result<Self, Error> {
        if asset.len() > GRAPHICS_ASSET_RING_BYTES {
            return Err(Error::BudgetExceeded);
        }
        if vertex_count == 0 || vertex_count > super::super::MAX_VERTICES || target_ids.is_empty() {
            return Err(Error::InvalidCommand);
        }
        if target_ids.len() > MAX_MORPH_TARGETS {
            return Err(Error::BudgetExceeded);
        }
        graph.check_budget(std::mem::size_of::<[u64; MAX_MORPH_TARGETS]>())?;
        check_whole_file_crc32c(asset).map_err(|_| Error::InvalidFact)?;
        let mut field_scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
        let mut fields = field_scratch;
        let view = TypedPayloadResolver::new(asset, &[], &mut field_scratch, &mut fields)
            .map_err(|_| Error::UnboundTarget)?;
        let mesh = view
            .section_payload(SectionType::QuantizedMesh)
            .ok_or(Error::UnboundTarget)?;
        let (mesh_header, _) = parse_mesh_header(mesh).map_err(|_| Error::InvalidFact)?;
        if mesh_header.vertex_count as usize != vertex_count {
            return Err(Error::UnboundTarget);
        }
        let morph = view
            .morph_targets()
            .map_err(|_| Error::UnboundTarget)?
            .ok_or(Error::UnboundTarget)?;
        let body_len = morph
            .bytes
            .len()
            .checked_sub(MORPH_SECTION_HEADER_SIZE)
            .ok_or(Error::InvalidFact)?;
        if body_len % QUANTIZED_MORPH_DELTA_SIZE != 0 {
            return Err(Error::InvalidFact);
        }
        let delta_count = body_len / QUANTIZED_MORPH_DELTA_SIZE;
        if body_len > delta_budget_bytes || delta_count > MAX_MORPH_DELTAS {
            return Err(Error::BudgetExceeded);
        }
        let mut deltas = Vec::new();
        deltas
            .try_reserve_exact(delta_count)
            .map_err(|_| Error::BudgetExceeded)?;
        deltas.resize(delta_count, QuantizedMorphDelta::zeroed());
        let (header, written) =
            decode_morph_section_into(morph.bytes, &mut deltas).map_err(|_| Error::InvalidFact)?;
        if header.vertex_count as usize != vertex_count || written != delta_count {
            return Err(Error::UnboundTarget);
        }

        let mut local = SymbolicRdfCborLdPayload::default();
        let mut dictionary_bytes = 0usize;
        for dictionary in view.payloads_of_kind(FIELD_KIND_SYMBOLIC_RDF_CBOR_LD) {
            let dictionary = dictionary.map_err(|_| Error::InvalidFact)?;
            dictionary_bytes = dictionary_bytes.checked_add(dictionary.bytes.len())
                .ok_or(Error::BudgetExceeded)?;
            graph.check_budget(dictionary_bytes)?;
            let payload = decode_symbolic_rdf_cbor_ld(dictionary.bytes).map_err(|_| Error::InvalidFact)?;
            local.lexicon.extend(payload.lexicon);
            local.nquins.extend(payload.nquins);
        }
        local.nquins.sort_unstable();
        local.nquins.dedup();
        validate_local_iri(&local, group, graph.iri(group)?)?;
        validate_local_iri(&local, SLOT, graph.iri(SLOT)?)?;
        validate_local_iri(&local, GROUP, graph.iri(GROUP)?)?;
        let context = graph.context;
        validate_local_iri(&local, context, graph.iri(context)?)?;
        let mut semantic_by_slot = [0; MAX_MORPH_TARGETS];
        for &target in target_ids {
            validate_local_iri(&local, target, graph.iri(target)?)?;
            let slot_fact = graph.one(target, SLOT)?;
            let slot = index(slot_fact)?;
            if graph.one(target, GROUP)? != group || slot >= usize::from(header.target_count) {
                return Err(Error::UnboundTarget);
            }
            validate_local_binding(&local, context, target, group, slot_fact, slot)?;
            if semantic_by_slot[slot] != 0 {
                return Err(Error::AmbiguousTarget);
            }
            semantic_by_slot[slot] = target;
        }

        deltas.retain(|delta| semantic_by_slot[usize::from(delta.target_index)] != 0);
        let mut populated = [false; MAX_MORPH_TARGETS];
        for delta in &deltas {
            if delta.tangent_oct_delta != [0; 2] {
                // The resident skin reconstructs tangents from geometry and UVs.
                return Err(Error::UnboundTarget);
            }
            populated[usize::from(delta.target_index)] = true;
        }
        if semantic_by_slot
            .iter()
            .zip(populated)
            .any(|(&id, populated)| id != 0 && !populated)
        {
            return Err(Error::UnboundTarget);
        }

        Ok(Self {
            header,
            deltas,
            semantic_by_slot,
            bound_count: target_ids.len(),
        })
    }

    /// Stage a complete set of bound weights and apply the native sparse MRP1 kernel.
    /// Rebuild from immutable bind geometry each frame. Output slices are scratch
    /// and the caller publishes them only when this operation succeeds.
    pub fn sample_from_weights_into(
        &self,
        values: &[Option<MacroValue>],
        base_positions: &[[f32; 3]],
        base_normals: &[[f32; 3]],
        out_positions: &mut [[f32; 3]],
        out_normals: &mut [[f32; 3]],
    ) -> Result<(), Error> {
        if values.len() > 5 * MAX_MORPH_TARGETS {
            return Err(Error::BudgetExceeded);
        }
        let mut weights = [0.0f32; MAX_MORPH_TARGETS];
        let mut seen = [false; MAX_MORPH_TARGETS];
        let mut applied = 0;
        for value in values
            .iter()
            .flatten()
            .filter(|value| value.channel == MacroChannel::MorphWeight)
        {
            if value.semantic_id == 0
                || value.rotation.is_some()
                || !(0.0..=1.0).contains(&value.value)
            {
                return Err(Error::InvalidCommand);
            }
            let slot = self
                .semantic_by_slot
                .iter()
                .position(|&id| id == value.semantic_id)
                .ok_or(Error::UnboundTarget)?;
            if seen[slot] {
                return Err(Error::AmbiguousTarget);
            }
            seen[slot] = true;
            weights[slot] = value.value as f32;
            applied += 1;
        }
        if applied != self.bound_count {
            return Err(Error::InvalidCommand);
        }
        let vertex_count = self.header.vertex_count as usize;
        if base_positions.len() != vertex_count || base_normals.len() != vertex_count {
            return Err(Error::UnboundTarget);
        }
        let positions = out_positions
            .get_mut(..vertex_count)
            .ok_or(Error::OutputTooSmall)?;
        let normals = out_normals
            .get_mut(..vertex_count)
            .ok_or(Error::OutputTooSmall)?;
        positions.copy_from_slice(base_positions);
        normals.copy_from_slice(base_normals);
        let morph_normals =
            (self.header.flags & FLAG_MORPH_HAS_NORMALS != 0).then_some(&mut *normals);
        apply_morph_targets_zero_heap(
            &self.header,
            &self.deltas,
            &weights,
            positions,
            morph_normals,
        )
        .map_err(|_| Error::InvalidFact)?;
        if positions
            .iter()
            .chain(&*normals)
            .flatten()
            .any(|value| !value.is_finite() || value.abs() > 1e8)
            || normals
                .iter()
                .any(|normal| normal.iter().map(|v| v * v).sum::<f32>() < 1e-12)
        {
            return Err(Error::InvalidCommand);
        }
        Ok(())
    }
}

fn validate_local_iri(local: &SymbolicRdfCborLdPayload, id: u64, iri: &str) -> Result<(), Error> {
    let mut matched = false;
    for term in &local.lexicon {
        if term.term_hash == id || term.iri_utf8 == iri {
            if term.term_hash != id || term.iri_utf8 != iri {
                return Err(Error::UnknownIdentity);
            }
            matched = true;
        }
    }
    matched.then_some(()).ok_or(Error::UnknownIdentity)
}

fn validate_local_binding(
    local: &SymbolicRdfCborLdPayload,
    context: u64,
    target: u64,
    group: u64,
    slot_fact: u64,
    slot: usize,
) -> Result<(), Error> {
    let mut seen_slot = false;
    let mut seen_group = false;
    for &[subject, predicate, object, row_context, metadata, parity] in &local.nquins {
        if row_context != context || !matches!(predicate, SLOT | GROUP) {
            continue;
        }
        if subject != target {
            if predicate == SLOT && index(object).ok() == Some(slot) {
                return Err(Error::AmbiguousTarget);
            }
            continue;
        }
        let row = NQuin {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        };
        if !row.verify_ecc_parity() {
            return Err(Error::InvalidFact);
        }
        if predicate == SLOT {
            if object != slot_fact || seen_slot {
                return Err(Error::ConflictingFact);
            }
            seen_slot = true;
        } else {
            if object != group || seen_group {
                return Err(Error::ConflictingFact);
            }
            seen_group = true;
        }
    }
    (seen_slot && seen_group)
        .then_some(())
        .ok_or(Error::UnboundTarget)
}
