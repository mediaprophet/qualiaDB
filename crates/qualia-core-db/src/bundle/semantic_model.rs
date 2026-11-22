//! Canonical semantic model for a package of intact assets.

use std::{collections::{hash_map::Entry, BTreeMap, BTreeSet, HashMap}, io};

use super::{BundleReader, BUNDLE_PAGE_ALIGN_4096};
use crate::{
    container_10d::{
        compute_whole_file_crc32c, decode_archetype_ref_record, decode_entity_ontology_section_into,
        decode_instance_batch_into, decode_multiscale_strata_section_into,
        decode_symbolic_rdf_cbor_ld, AffordanceBinaryRecord, Container10dHeader,
        InstanceBatchRecord, NestedStratumNodeRecord, SectionType, SymbolicRdfCborLdPayload,
        TypedFieldRecord, TypedPayloadResolver,
        FIELD_KIND_ARCHETYPE_REF, FIELD_KIND_GEOMETRY_TOPOLOGY_COMPLEX, FIELD_KIND_INSTANCE_BATCH, INSTANCE_BATCH_HEADER_SIZE,
        INSTANCE_BATCH_RECORD_SIZE, MAX_INSTANCE_BATCH_RECORDS,
        FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE, MAX_AFFORDANCES_PER_SECTION,
        FIELD_KIND_MULTISCALE_STRATA,
        FIELD_KIND_SYMBOLIC_RDF_CBOR_LD, MANIFOLD_IDENTITY_V2_MAX_FIELDS,
        MAX_NESTED_STRATUM_NODES, MULTISCALE_STRATA_HEADER_SIZE, NESTED_STRATUM_NODE_SIZE,
    },
    container_10d::manifold_identity_v2::FIELD_KIND_SIGNED_DISTANCE_BRICK,
    container_10d::sdf_brick_section::SdfBrickView,
    q42_lex::LexiconEntry,
    q42_volume::{payload_catalogue::PayloadCatalogueBlob, UnifiedVolumeBuilder, WasmSliceQ42Reader},
    q_hash, NQuin, QUINS_PER_BLOCK,
};

mod packing;
mod q42_import;
mod matter;
mod holo_address;
mod signals;
mod turtle;
pub use holo_address::{holo_segment_iri, open_hmc_holo_segment, HOLO_ADDRESS_CONTEXT};
pub(crate) use holo_address::{holo_address_payload, validated_holo_header};
pub use signals::{open_hmc_signal_range, HmcSignalRange};
pub use packing::{
    pack_semantic_hmc, HmcPackAsset, HmcPackLimits, HmcPackOutput, HmcPackedRange, HmcSalience,
};

pub const SEMANTIC_MODEL_KEY: &str = "semantic_model.q42";
const MODEL_ALIAS: &str = "manifest.q42";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
const RDF_GRAPH: &str = "https://qualiadb.org/ontology/hmc#SemanticModel";
const RULE_SOURCE: &str = "https://qualiadb.org/ontology/hmc#n3RuleSource";
const GOVERNING_SHAPE: &str = "https://qualiadb.org/ontology/hmc#governingShape";
const PACKAGE_MEMBER: &str = "https://qualiadb.org/ontology/hmc#hasEntry";
const PART_OF: &str = "http://purl.obolibrary.org/obo/BFO_0000050";
const STRATUM: &str = "https://qualiadb.org/ontology/hmc#computationalStratum";
const AFFORDANCE: &str = "https://qualiadb.org/ontology/hmc#Affordance";
const REQUIRED_CAPABILITY: &str = "https://qualiadb.org/ontology/hmc#requiredCapability";
const EMITTED_TOPIC: &str = "https://qualiadb.org/ontology/hmc#emittedTopic";
const GTOP_GEOMETRY: &str = "urn:qualia:gtop:geometry";
const GTOP_STALK_SCHEMA: &str = "urn:qualia:gtop:hasStalkSchema";

/// Names a bundle-relative entry key as a canonical RDF IRI. The bytewise
/// encoding preserves Unicode and distinguishes keys containing percent signs.
pub fn hmc_entry_key_iri(key: &str) -> String {
    use std::fmt::Write;
    let mut iri = String::from("urn:qualia:hmc:entry:");
    for byte in key.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            iri.push(char::from(byte));
        } else {
            let _ = write!(iri, "%{byte:02X}");
        }
    }
    iri
}

pub fn resource_identity_meta(iri: &str) -> io::Result<Vec<u8>> {
    if !valid_iri(iri) || q_hash(iri) == 0 {
        return Err(invalid("resource identity must be an absolute IRI"));
    }
    let mut bytes = Vec::new();
    ciborium::into_writer(
        &std::collections::BTreeMap::from([
            ("root_entity_iri", ciborium::Value::Text(iri.to_owned())),
            ("root_q_hash", ciborium::Value::Integer(q_hash(iri).into())),
        ]),
        &mut bytes,
    ).map_err(|error| invalid(error.to_string()))?;
    Ok(bytes)
}

/// Root identity and bounds read from the authored MID2, HOLO and MSTR fields.
#[derive(serde::Serialize)]
pub struct SpatialEntryMeta {
    pub root_entity_iri: String,
    pub root_q_hash: u64,
    pub stratum_mask: u16,
    pub bounds_min_10d: [f32; 10],
    pub bounds_max_10d: [f32; 10],
}

impl SpatialEntryMeta {
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        ciborium::into_writer(self, &mut bytes).map_err(|error| invalid(error.to_string()))?;
        Ok(bytes)
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

pub(crate) fn valid_iri(iri: &str) -> bool {
    let Some((scheme, value)) = iri.split_once(':') else { return false };
    let mut scheme = scheme.bytes();
    scheme.next().is_some_and(|first| first.is_ascii_alphabetic())
        && scheme.all(|byte| byte.is_ascii_alphanumeric() || b"+-.".contains(&byte))
        && !value.is_empty()
        && !value.chars().any(|ch| ch.is_whitespace() || ch.is_control() || "<>\"{}|\\^`".contains(ch))
}

/// Cold builder. Byte-backed readers borrow the finished volume during lookup.
pub struct SemanticModelBuilder {
    lex: HashMap<u64, LexiconEntry>,
    quins: Vec<NQuin>,
    payloads: BTreeMap<String, PayloadCatalogueBlob>,
    sanctuary: bool,
    source_bytes: usize,
    limit: usize,
}

impl SemanticModelBuilder {
    pub fn new(limit: usize) -> Self {
        Self {
            lex: HashMap::new(),
            quins: Vec::new(),
            payloads: BTreeMap::new(),
            sanctuary: false,
            source_bytes: 0,
            limit,
        }
    }

    fn reserve(total: &mut usize, limit: usize, bytes: usize) -> io::Result<()> {
        *total = total.checked_add(bytes).filter(|&size| size <= limit)
            .ok_or_else(|| invalid("semantic model exceeds byte budget"))?;
        Ok(())
    }

    fn register(&mut self, hash: u64, term: &str) -> io::Result<()> {
        if hash == 0 || q_hash(term) != hash {
            return Err(invalid(format!("semantic term hash mismatch for {term}")));
        }
        self.register_spelling(hash, term)
    }

    fn register_spelling(&mut self, hash: u64, term: &str) -> io::Result<()> {
        if hash == 0 {
            return Err(invalid("semantic term uses the zero hash"));
        }
        match self.lex.entry(hash) {
            Entry::Occupied(existing) => match existing.get() {
                LexiconEntry::String(previous) if previous == term => Ok(()),
                _ => Err(invalid(format!("semantic hash collision at {hash:#018x}"))),
            },
            Entry::Vacant(slot) => {
                Self::reserve(&mut self.source_bytes, self.limit, term.len().saturating_add(16))?;
                slot.insert(LexiconEntry::String(term.to_owned()));
                Ok(())
            }
        }
    }

    fn push(&mut self, quin: NQuin) -> io::Result<()> {
        if !quin.verify_ecc_parity() {
            return Err(invalid("semantic graph contains an invalid NQuin"));
        }
        Self::reserve(&mut self.source_bytes, self.limit, std::mem::size_of::<NQuin>())?;
        self.quins.push(quin);
        Ok(())
    }

    fn fact(&mut self, subject: &str, predicate: &str, object: &str, context: &str) -> io::Result<()> {
        let [s, p, o, c] = [subject, predicate, object, context].map(q_hash);
        for (hash, term) in [(s, subject), (p, predicate), (o, object), (c, context)] {
            self.register(hash, term)?;
        }
        self.push(NQuin {
            subject: s, predicate: p, object: o, context: c, metadata: 0,
            parity: NQuin::calculate_parity(s, p, o, c, 0),
        })
    }

    fn resolve_iri(&self, hash: u64) -> io::Result<&str> {
        self.lex.get(&hash)
            .and_then(|entry| match entry {
                LexiconEntry::String(term) if valid_iri(term) && q_hash(term) == hash => {
                    Some(term.as_str())
                }
                _ => None,
            })
            .ok_or_else(|| invalid(format!("unresolved spatial semantic identity {hash:#018x}")))
    }

    fn resolve_local_iri(&self, local: &BTreeSet<u64>, hash: u64) -> io::Result<&str> {
        if !local.contains(&hash) {
            return Err(invalid(format!("10D CBLD omits addressed semantic identity {hash:#018x}")));
        }
        self.resolve_iri(hash)
    }

    fn merge_strata(mut self, bytes: &[u8], local: &BTreeSet<u64>) -> io::Result<(Self, u16)> {
        let count = bytes.len().saturating_sub(MULTISCALE_STRATA_HEADER_SIZE)
            / NESTED_STRATUM_NODE_SIZE;
        if count == 0 || count > MAX_NESTED_STRATUM_NODES || bytes.len() > self.limit {
            return Err(invalid("invalid MSTR size for package semantic model"));
        }
        let empty = <NestedStratumNodeRecord as bytemuck::Zeroable>::zeroed();
        let mut nodes = vec![empty; count];
        let (header, count) = decode_multiscale_strata_section_into(bytes, &mut nodes)
            .map_err(|error| invalid(error.to_string()))?;
        for node in &nodes[..count] {
            let entity = self.resolve_local_iri(local, node.host_anchor.entity_hash)?.to_owned();
            let class = self.resolve_local_iri(local, node.taxon_or_ontology_iri_hash)?.to_owned();
            self.fact(&entity, RDF_TYPE, &class, RDF_GRAPH)?;
            let stratum = format!("https://qualiadb.org/ontology/hmc#L{}", node.stratum_level);
            self.fact(&entity, STRATUM, &stratum, RDF_GRAPH)?;
            if node.parent_node_index != u16::MAX {
                let parent_hash = nodes[usize::from(node.parent_node_index)].host_anchor.entity_hash;
                let parent = self.resolve_iri(parent_hash)?.to_owned();
                self.fact(&entity, PART_OF, &parent, RDF_GRAPH)?;
            }
        }
        Ok((self, header.populated_strata_mask))
    }

    fn merge_affordances(mut self, bytes: &[u8], local: &BTreeSet<u64>) -> io::Result<Self> {
        let empty = <AffordanceBinaryRecord as bytemuck::Zeroable>::zeroed();
        let mut records = [empty; MAX_AFFORDANCES_PER_SECTION];
        let (_, count) = decode_entity_ontology_section_into(bytes, &mut records)
            .map_err(|error| invalid(error.to_string()))?;
        for record in &records[..count] {
            let action = self.resolve_local_iri(local, record.action_id)?.to_owned();
            self.fact(&action, RDF_TYPE, AFFORDANCE, RDF_GRAPH)?;
            for (predicate, hash) in [
                (REQUIRED_CAPABILITY, record.required_capability_hash),
                (EMITTED_TOPIC, record.emitted_topic_hash),
            ] {
                if hash != 0 {
                    let iri = self.resolve_local_iri(local, hash)?.to_owned();
                    self.fact(&action, predicate, &iri, RDF_GRAPH)?;
                }
            }
        }
        Ok(self)
    }

    pub fn identify_package(mut self, iri: &str) -> io::Result<Self> {
        if !valid_iri(iri) {
            return Err(invalid("package identity must be an absolute IRI"));
        }
        self.fact(iri, RDF_TYPE, "https://qualiadb.org/ontology/hmc#Package", RDF_GRAPH)?;
        Ok(self)
    }

    pub fn add_package_entry(mut self, package_iri: &str, resource_iri: &str) -> io::Result<Self> {
        if !valid_iri(package_iri) || !valid_iri(resource_iri) {
            return Err(invalid("package and resource identities must be absolute IRIs"));
        }
        self.fact(package_iri, PACKAGE_MEMBER, resource_iri, RDF_GRAPH)?;
        Ok(self)
    }

    /// Import CBLD v1 facts and preserve lexical labels, N3 sources and SHACL shape references.
    pub fn merge_cbld(self, bytes: &[u8]) -> io::Result<Self> {
        let payload = self.decode_cbld(bytes)?;
        self.import_cbld(payload)
    }

    fn decode_cbld(&self, bytes: &[u8]) -> io::Result<SymbolicRdfCborLdPayload> {
        if bytes.len() > self.limit {
            return Err(invalid("CBLD exceeds semantic model byte budget"));
        }
        decode_symbolic_rdf_cbor_ld(bytes).map_err(|error| invalid(error.to_string()))
    }

    fn import_cbld(mut self, payload: SymbolicRdfCborLdPayload) -> io::Result<Self> {
        if !valid_iri(&payload.context_iri) {
            return Err(invalid("CBLD context must be an absolute IRI"));
        }
        for term in payload.lexicon {
            if !valid_iri(&term.iri_utf8) {
                return Err(invalid("CBLD lexicon contains a non-IRI binding"));
            }
            self.register(term.term_hash, &term.iri_utf8)?;
            if !term.label_utf8.is_empty() {
                let mut literal = serde_json::to_string(&term.label_utf8)
                    .map_err(|error| invalid(error.to_string()))?;
                if let Some(language) = term.lang_bcp47 {
                    if language.is_empty()
                        || language.split('-').any(|part| part.is_empty()
                            || !part.bytes().all(|byte| byte.is_ascii_alphanumeric()))
                    {
                        return Err(invalid("invalid CBLD label language tag"));
                    }
                    literal.push('@');
                    literal.push_str(&language);
                }
                self.fact(&term.iri_utf8, RDFS_LABEL, &literal, &payload.context_iri)?;
            }
        }
        for [subject, predicate, object, context, metadata, parity] in payload.nquins {
            self.push(NQuin { subject, predicate, object, context, metadata, parity })?;
        }
        for source in payload.n3_logic_rules_utf8 {
            let literal = serde_json::to_string(&source).map_err(|error| invalid(error.to_string()))?;
            self.fact(&payload.context_iri, RULE_SOURCE, &literal, RDF_GRAPH)?;
        }
        for shape in payload.shacl_shape_iris_utf8 {
            if !valid_iri(&shape) {
                return Err(invalid("CBLD SHACL shape must be an absolute IRI"));
            }
            self.fact(&payload.context_iri, GOVERNING_SHAPE, &shape, RDF_GRAPH)?;
        }
        Ok(self)
    }

    /// Import the local CBLD and MSTR slices of an intact .10d asset.
    pub fn merge_10d(mut self, bytes: &[u8]) -> io::Result<(Self, Option<SpatialEntryMeta>)> {
        let header = Container10dHeader::parse(bytes).map_err(|error| invalid(error.to_string()))?;
        if compute_whole_file_crc32c(bytes) != header.header_crc32c {
            return Err(invalid("10D resource has an invalid whole-file checksum"));
        }
        let mut scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
        let mut fields = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
        let view = TypedPayloadResolver::new(bytes, &[], &mut scratch, &mut fields)
            .map_err(|error| invalid(error.to_string()))?;
        let mut local = BTreeSet::new();
        let mut declared_holo_addresses = Vec::new();
        for field in view.payloads_of_kind(FIELD_KIND_SYMBOLIC_RDF_CBOR_LD) {
            let bytes = field.map_err(|error| invalid(error.to_string()))?.bytes;
            let payload = self.decode_cbld(bytes)?;
            if payload.context_iri == HOLO_ADDRESS_CONTEXT {
                declared_holo_addresses.push(payload);
                continue;
            }
            local.extend(payload.lexicon.iter().map(|term| term.term_hash));
            self = self.import_cbld(payload)?;
        }
        let mut root = None::<SpatialEntryMeta>;
        for field in view.payloads_of_kind(FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE) {
            let field = field.map_err(|error| invalid(error.to_string()))?;
            let holo = validated_holo_header(field.bytes)?;
            let iri = self.resolve_local_iri(&local, holo.nquin_subject_hash)?.to_owned();
            let address = holo_address_payload(&iri, field.record.field_id, holo.holo_segment_count)?;
            if let Some(index) = declared_holo_addresses.iter().position(|payload| payload == &address) {
                declared_holo_addresses.swap_remove(index);
                local.extend(address.lexicon.iter().map(|term| term.term_hash));
            }
            self = self.import_cbld(address)?;
            let coordinates = view.identity().ok_or_else(|| invalid("missing MID2 identity"))?
                .coordinates;
            let meta = root.get_or_insert_with(|| {
                let mut bounds_min_10d = coordinates;
                let mut bounds_max_10d = coordinates;
                bounds_min_10d[3..6].copy_from_slice(&holo.proxy_aabb_min);
                bounds_max_10d[3..6].copy_from_slice(&holo.proxy_aabb_max);
                SpatialEntryMeta {
                    root_entity_iri: iri,
                    root_q_hash: holo.nquin_subject_hash,
                    stratum_mask: 0,
                    bounds_min_10d,
                    bounds_max_10d,
                }
            });
            if meta.root_q_hash != holo.nquin_subject_hash {
                return Err(invalid("10d HOLO fields disagree on root identity"));
            }
            for axis in 0..3 {
                let min = holo.proxy_aabb_min[axis];
                let max = holo.proxy_aabb_max[axis];
                if !min.is_finite() || !max.is_finite() || min > max {
                    return Err(invalid("invalid HOLO bounds"));
                }
                meta.bounds_min_10d[3 + axis] = meta.bounds_min_10d[3 + axis].min(min);
                meta.bounds_max_10d[3 + axis] = meta.bounds_max_10d[3 + axis].max(max);
            }
        }
        if !declared_holo_addresses.is_empty() {
            return Err(invalid("CBLD HOLO addresses do not match the validated native segment extents"));
        }
        for field in view.payloads_of_kind(FIELD_KIND_MULTISCALE_STRATA) {
            let field = field.map_err(|error| invalid(error.to_string()))?;
            let (next, mask) = self.merge_strata(field.bytes, &local)?;
            self = next;
            if let Some(root) = root.as_mut() {
                root.stratum_mask |= mask;
            }
        }
        for field in view.payloads_of_kind(FIELD_KIND_SIGNED_DISTANCE_BRICK) {
            let field = field.map_err(|error| invalid(error.to_string()))?;
            let sdf = SdfBrickView::parse(field.bytes).map_err(|error| invalid(error.to_string()))?;
            for brick in sdf.bricks() {
                self.resolve_local_iri(&local, brick.wall_semantic_id())?;
                self.resolve_local_iri(&local, brick.lumen_semantic_id())?;
            }
        }
        for field in view.payloads_of_kind(FIELD_KIND_GEOMETRY_TOPOLOGY_COMPLEX) {
            let field = field.map_err(|error| invalid(error.to_string()))?;
            if let Some(sheaf) = crate::topology::admit_gtop_segment_sheaf(field.bytes)
                .map_err(|error| invalid(error.to_string()))?
            {
                let geometry = self.resolve_local_iri(&local, sheaf.geometry_id)?.to_owned();
                let schema = self.resolve_local_iri(&local, sheaf.stalk_schema_q_hash)?.to_owned();
                self.fact(&geometry, GTOP_STALK_SCHEMA, &schema, RDF_GRAPH)?;
                if let Some(meta) = root.as_ref() {
                    self.fact(&meta.root_entity_iri, GTOP_GEOMETRY, &geometry, RDF_GRAPH)?;
                }
            }
        }
        if let Some(affordances) = view.section_payload(SectionType::EntityOntology) {
            self = self.merge_affordances(affordances, &local)?;
        }
        self.verify_addressed_10d_records(bytes, &view, &local)?;
        Ok((self, root))
    }

    /// Validate a detached GTOP envelope and bind its sheaf identity to admitted lexical terms.
    fn merge_gtop(mut self, bytes: &[u8], resource_iri: &str) -> io::Result<Self> {
        let raw = crate::q42_volume::payload_catalogue::decode_gtop_section(bytes).map_err(invalid)?;
        if let Some(sheaf) = crate::topology::read_gtop_sheaf_metadata(raw)
            .map_err(|error| invalid(error.to_string()))?
        {
            let geometry = self.resolve_iri(sheaf.geometry_id)?.to_owned();
            let schema = self.resolve_iri(sheaf.stalk_schema_q_hash)?.to_owned();
            self.fact(resource_iri, GTOP_GEOMETRY, &geometry, RDF_GRAPH)?;
            self.fact(&geometry, GTOP_STALK_SCHEMA, &schema, RDF_GRAPH)?;
        }
        Ok(self)
    }

    fn verify_addressed_10d_records(&self, bytes: &[u8], view: &TypedPayloadResolver<'_, '_>, local: &BTreeSet<u64>) -> io::Result<()> {
        use crate::container_10d::{
            material_section::{decode_material_section_into, material_section_counts, MaterialRecord, SubmeshRange},
            mesh_section::parse_mesh_header,
            skeletal_container::read_skl2_section,
            skeletal_section::SKL2_MAGIC,
        };
        view.validate_instance_overrides().map_err(|error| invalid(error.to_string()))?;
        for entry in view.payloads_of_kind(FIELD_KIND_ARCHETYPE_REF) {
            let entry = entry.map_err(|error| invalid(error.to_string()))?;
            let reference = decode_archetype_ref_record(entry.bytes).map_err(|error| invalid(format!("{error:?}")))?;
            self.resolve_local_iri(local, reference.taxon_or_class_iri_hash)?;
            self.resolve_local_iri(local, reference.subresource_key_hash)?;
        }
        for entry in view.payloads_of_kind(FIELD_KIND_INSTANCE_BATCH) {
            let entry = entry.map_err(|error| invalid(error.to_string()))?;
            let count = entry.bytes.len().checked_sub(INSTANCE_BATCH_HEADER_SIZE)
                .filter(|size| size % INSTANCE_BATCH_RECORD_SIZE == 0)
                .map(|size| size / INSTANCE_BATCH_RECORD_SIZE)
                .filter(|count| *count <= MAX_INSTANCE_BATCH_RECORDS)
                .ok_or_else(|| invalid("Invalid 10D instance identity table extent"))?;
            let mut records = vec![InstanceBatchRecord::default(); count];
            let (_, decoded) = decode_instance_batch_into(entry.bytes, &mut records)
                .map_err(|error| invalid(format!("{error:?}")))?;
            for record in &records[..decoded] {
                self.resolve_local_iri(local, record.entity_id_hash)?;
            }
        }
        if let Some(payload) = view.section_payload(SectionType::Materials) {
            let mesh = view.section_payload(SectionType::QuantizedMesh)
                .ok_or_else(|| invalid("Addressed material ranges require QuantizedMesh"))?;
            let (header, _) = parse_mesh_header(mesh).map_err(|error| invalid(error.to_string()))?;
            let (materials, ranges) = material_section_counts(payload).map_err(|error| invalid(error.to_string()))?;
            let mut materials = vec![MaterialRecord::legacy_default(); materials];
            let mut ranges = vec![SubmeshRange { first_index: 0, index_count: 0, material_id: 0, semantic_id: 0 }; ranges];
            decode_material_section_into(payload, header.triangle_count * 3, &mut materials, &mut ranges)
                .map_err(|error| invalid(error.to_string()))?;
            for range in ranges {
                self.resolve_local_iri(local, range.semantic_id)?;
            }
        }
        if view.section_payload(SectionType::SkeletalAnimation)
            .is_some_and(|payload| payload.starts_with(&SKL2_MAGIC))
        {
            let rig = read_skl2_section(bytes).map_err(|error| invalid(format!("{error:?}")))?;
            for joint in rig.joints {
                self.resolve_local_iri(local, joint.joint_id)?;
            }
            for closure in rig.loop_closures {
                self.resolve_local_iri(local, closure.constraint_id)?;
            }
            for landmark in rig.landmarks {
                self.resolve_local_iri(local, landmark.landmark_id)?;
            }
            for pathway in rig.actuation_pathways {
                self.resolve_local_iri(local, pathway.pathway_id)?;
            }
            for basis in rig.ssm_headers {
                self.resolve_local_iri(local, basis.species_taxon_hash)?;
            }
        }
        Ok(())
    }

    /// Seal an object-sorted Q42 volume suitable for a page-aligned HMC entry.
    pub fn finish(self) -> io::Result<Vec<u8>> {
        if self.quins.is_empty() {
            return Err(invalid("semantic model contains no facts"));
        }
        self.seal_volume()
    }

    fn seal_volume(mut self) -> io::Result<Vec<u8>> {
        self.quins.sort_unstable_by_key(|q| (q.object, q.subject, q.predicate, q.context, q.metadata, q.parity));
        self.quins.dedup_by_key(|q| (q.object, q.subject, q.predicate, q.context, q.metadata, q.parity));
        // Keep the standard namespace-compressed dictionary for string-only models.
        // RDF-star and Webizen tokens require the tagged entry encoder.
        let volume = if self.lex.values().all(|entry| matches!(entry, LexiconEntry::String(_))) {
            let terms = self.lex.into_iter().filter_map(|(hash, entry)| match entry {
                LexiconEntry::String(term) => Some((hash, term)),
                _ => None,
            }).collect();
            UnifiedVolumeBuilder::with_lex_map(&terms)
        } else {
            UnifiedVolumeBuilder::with_lex_entries(&self.lex)
        };
        let mut volume = volume.map_err(|error| invalid(error.to_string()))?
            .with_fixed_timestamp_ms(0)
            .with_payload_catalogue_blobs(&self.payloads.into_values().collect::<Vec<_>>())?;
        if self.sanctuary {
            volume = volume.with_sanctuary();
        }
        for (index, block) in self.quins.chunks(QUINS_PER_BLOCK).enumerate() {
            volume.push_block(index as u64, block)?;
        }
        let bytes = volume.finish_to_bytes();
        if bytes.len() > self.limit {
            return Err(invalid("sealed semantic model exceeds byte budget"));
        }
        Ok(bytes)
    }
}

/// Borrow a page-aligned, headered Q42 volume directly from an admitted bundle.
pub fn open_semantic_model<'a>(bundle: &BundleReader<'a>) -> io::Result<WasmSliceQ42Reader<'a>> {
    let key = match (bundle.entry(SEMANTIC_MODEL_KEY), bundle.entry(MODEL_ALIAS)) {
        (Some(_), None) => SEMANTIC_MODEL_KEY,
        (None, Some(_)) => MODEL_ALIAS,
        _ => return Err(invalid("HMC must contain exactly one package-root semantic model")),
    };
    let entry = bundle.entry(key).ok_or_else(|| invalid("semantic model entry missing"))?;
    if entry.kind != "q42" || entry.offset % BUNDLE_PAGE_ALIGN_4096 as u64 != 0 {
        return Err(invalid("semantic model entry must be a page-aligned q42"));
    }
    if !bundle.verify_entry(key) {
        return Err(invalid("semantic model entry SHA-256 mismatch"));
    }
    let bytes = bundle.get(key).ok_or_else(|| invalid("semantic model entry is out of bounds"))?;
    let reader = WasmSliceQ42Reader::open(bytes)?;
    if !reader.is_unified_volume() {
        return Err(invalid("semantic model requires a unified Q42 volume"));
    }
    Ok(reader)
}
