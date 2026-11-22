//! Producer-side validation of a rig against the `.10d`'s own CBLD dictionary.

use std::collections::BTreeMap;

use super::{SkeletalSectionError, SkeletalSectionPayload};
use crate::{container_10d as ten, q_hash};
use ten::skeletal_container::{SkeletalContainerError, attach_skl2_section_into};

#[derive(Debug)]
pub enum Skl2BuildError {
    MissingFrame,
    InvalidRig(SkeletalSectionError),
    InvalidFrame(String),
    InvalidLexicalBinding(u64),
    AmbiguousIdentity(u64),
    Enclosure(SkeletalContainerError),
}

/// Validate and attach an authored SKL2/SSM1 rig. This cold producer operation
/// borrows the existing envelope and writes the new envelope to caller storage.
pub fn seal_authored_skl2_into(
    source: &[u8], rig: &SkeletalSectionPayload, scratch: &mut [u8], output: &mut [u8],
) -> Result<usize, Skl2BuildError> {
    if rig.joints.is_empty() || source.len() > ten::GRAPHICS_ASSET_RING_BYTES {
        return Err(Skl2BuildError::InvalidRig(SkeletalSectionError::CountLimit));
    }
    rig.validate().map_err(Skl2BuildError::InvalidRig)?;
    let mut field_scratch = [ten::TypedFieldRecord::default(); ten::MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let mut fields = field_scratch;
    let view = ten::TypedPayloadResolver::new(source, &[], &mut field_scratch, &mut fields)
        .map_err(|error| Skl2BuildError::InvalidFrame(error.to_string()))?;
    if view.identity().is_none()
        || view.payloads_of_kind(ten::FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE).count() != 1
        || view.payloads_of_kind(ten::FIELD_KIND_MULTISCALE_STRATA).count() != 1
    {
        return Err(Skl2BuildError::MissingFrame);
    }
    let mut lexicon = BTreeMap::new();
    for dictionary in view.payloads_of_kind(ten::FIELD_KIND_SYMBOLIC_RDF_CBOR_LD) {
        let dictionary = dictionary.map_err(|error| Skl2BuildError::InvalidFrame(error.to_string()))?;
        let dictionary = ten::decode_symbolic_rdf_cbor_ld(dictionary.bytes)
            .map_err(|error| Skl2BuildError::InvalidFrame(format!("{error:?}")))?;
        for entry in dictionary.lexicon {
            let hash = entry.term_hash;
            let iri = entry.iri_utf8;
            if hash == 0 || q_hash(&iri) != hash || !crate::bundle::semantic_model::valid_iri(&iri) {
                return Err(Skl2BuildError::InvalidLexicalBinding(hash));
            }
            match lexicon.entry(hash) {
                std::collections::btree_map::Entry::Vacant(slot) => { slot.insert(iri); }
                std::collections::btree_map::Entry::Occupied(slot) if slot.get() == &iri => {}
                std::collections::btree_map::Entry::Occupied(_) => {
                    return Err(Skl2BuildError::AmbiguousIdentity(hash));
                }
            }
        }
    }
    if lexicon.is_empty() {
        return Err(Skl2BuildError::MissingFrame);
    }
    let mut joint_ids = std::collections::BTreeSet::new();
    for joint in &rig.joints {
        if !lexicon.contains_key(&joint.joint_id) {
            return Err(Skl2BuildError::InvalidLexicalBinding(joint.joint_id));
        }
        if !joint_ids.insert(joint.joint_id) {
            return Err(Skl2BuildError::AmbiguousIdentity(joint.joint_id));
        }
    }
    let extra_ids = rig.loop_closures.iter().map(|row| row.constraint_id)
        .chain(rig.landmarks.iter().map(|row| row.landmark_id))
        .chain(rig.actuation_pathways.iter().map(|row| row.pathway_id))
        .chain(rig.ssm_headers.iter().map(|row| row.species_taxon_hash));
    for id in extra_ids {
        if !lexicon.contains_key(&id) {
            return Err(Skl2BuildError::InvalidLexicalBinding(id));
        }
    }
    for closure in &rig.loop_closures {
        if !closure.anchor_a_local.iter().chain(&closure.anchor_b_local).all(|v| v.is_finite())
            || !closure.rest_angle_rad.is_finite()
            || ![closure.stiffness_n_m, closure.damping_n_s_m, closure.tolerance_m]
                .into_iter().all(|v| v.is_finite() && v >= 0.0)
        {
            return Err(Skl2BuildError::InvalidRig(SkeletalSectionError::InvalidData));
        }
    }
    let mut modes = rig.ssm_modes.as_slice();
    for header in &rig.ssm_headers {
        let count = usize::from(header.mode_count);
        if count == 0 || usize::from(header.target_joint_count) != rig.joints.len() || count > modes.len() {
            return Err(Skl2BuildError::InvalidRig(SkeletalSectionError::InvalidData));
        }
        let (basis, remaining) = modes.split_at(count);
        let mut ids = std::collections::BTreeSet::new();
        if basis.iter().any(|mode| {
            !ids.insert(mode.mode_id)
                || !mode.eigenvalue.is_finite() || mode.eigenvalue < 0.0
                || !mode.min_sigma.is_finite() || !mode.max_sigma.is_finite()
                || mode.min_sigma > mode.max_sigma
        }) {
            return Err(Skl2BuildError::InvalidRig(SkeletalSectionError::InvalidData));
        }
        modes = remaining;
    }
    if !modes.is_empty() {
        return Err(Skl2BuildError::InvalidRig(SkeletalSectionError::InvalidData));
    }
    attach_skl2_section_into(source, rig, scratch, output).map_err(Skl2BuildError::Enclosure)
}
