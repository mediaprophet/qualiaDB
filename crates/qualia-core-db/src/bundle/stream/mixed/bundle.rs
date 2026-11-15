//! One QBDL-profile HMC whose inner files are real `.10d` / `.q42` / `.emf` /
//! `.qac` / `.m64` payloads. Family is an assignment tag, not a suffix.
use crate::bundle::stream::descriptor::{
    CandidateIdentity, ContextParameters, DescriptorError, EntryRecord, PageRef,
    HASH_SCRATCH_BYTES, MAX_ENCODED, MAX_METADATA, MAX_PAGES,
};
use crate::bundle::stream::index::{AdmittedIndex, EntryScratch};
use crate::bundle::stream::plan::{
    DependencyEdge, DependencyReason, EntryFamily, Inventory, Obligation, PlanEntry, PlanEntrySpec,
    PlanError, RepresentationClass, ResourceEstimate, MAX_PLAN_ENTRIES,
};
use crate::bundle::stream::policy::{
    authenticate_closure, encode_manifest_into, BoundClosure, LocalGrantRequest, ManifestFields,
    PolicyError, PublicationProof, CLOSURE_PURPOSE, MAX_MANIFEST_BYTES, PUBLICATION_PURPOSE,
};
use crate::bundle::stream::publish::{
    EntryAssignment, PlainPublisher, PublicationClass, PublisherOptions,
};
use crate::bundle::stream::selection::{select_entry, SelectedPlainEntry, SelectionError};
use crate::bundle::{BundleError, BundleReadLimits, BundleWriter};
use crate::crypto::network::{
    authority::{LocalBootstrap, PinAuthority, PinConfig, PinHandle},
    cose,
    digest::sha384,
    types::{KeyEpoch, KeyPurpose},
};
use crate::net::qdnf::types::{OperationId, StrongDigest};
use crate::semantic_instruments::resolve::{bounded_closure::*, DependencyKind};
use ed25519_dalek::SigningKey;
use fips204::{
    ml_dsa_65,
    traits::{KeyGen, SerDes},
};

/// Default corpus ids. Stable, nonzero, unique across families.
pub const MIXED_ID_TEND_A: u32 = 1;
pub const MIXED_ID_TEND_B: u32 = 2;
pub const MIXED_ID_EMF_OPAQUE: u32 = 10;
pub const MIXED_ID_EMF_ALIGN: u32 = 11;
pub const MIXED_ID_QAC_OPAQUE: u32 = 20;
pub const MIXED_ID_QAC_PCM: u32 = 21;
pub const MIXED_ID_Q42: u32 = 30;
pub const MIXED_ID_M64: u32 = 40;
pub const MIXED_ID_P64: u32 = 41;
pub const MIXED_ID_OPAQUE: u32 = 50;
/// Distinct non-format optional enhancement.
pub const MIXED_OPAQUE_BYTES: &[u8] = b"qbdl-mixed-opaque-v1";

/// One inner file to publish. `id` is the assignment identity, never a filename.
#[derive(Clone, Copy)]
pub struct MixedEntry<'a> {
    pub id: u32,
    pub family: EntryFamily,
    pub role: RepresentationClass,
    pub obligation: Obligation,
    pub bytes: &'a [u8],
}

/// Published mixed HMC plus the closure material tests need for `select_entry`.
pub struct MixedHmc {
    source: Vec<u8>,
    body: Vec<u8>,
    entry_page: Vec<u8>,
    identity: CandidateIdentity,
    ids: [u32; MAX_PLAN_ENTRIES],
    families: [EntryFamily; MAX_PLAN_ENTRIES],
    roles: [RepresentationClass; MAX_PLAN_ENTRIES],
    obligations: [Obligation; MAX_PLAN_ENTRIES],
    payloads: Vec<Vec<u8>>,
    len: usize,
    nodes: [ClosureNode<'static>; 3],
    edges: [ClosureEdge; 2],
    fields: ManifestFields,
    manifest: Vec<u8>,
    ml_sk: ml_dsa_65::PrivateKey,
    ml_pk: [u8; 1952],
    ed_sk: SigningKey,
    ed_pk: [u8; 32],
}

/// Plan entries and geometry/calibration edges bound to one published HMC.
pub struct MixedPlan {
    candidate: CandidateIdentity,
    entries: Vec<PlanEntry>,
    edges: Vec<DependencyEdge>,
    geometry_id: Option<u32>,
    calibration_id: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MixedError {
    DuplicateId,
    Limit,
    Identity,
    Unsupported,
}

impl core::fmt::Display for MixedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "mixed {:?}", self)
    }
}
impl core::error::Error for MixedError {}

impl From<DescriptorError> for MixedError {
    fn from(e: DescriptorError) -> Self {
        match e {
            DescriptorError::DuplicateId => Self::DuplicateId,
            DescriptorError::Capacity
            | DescriptorError::Limit
            | DescriptorError::Overflow
            | DescriptorError::Length => Self::Limit,
            DescriptorError::Unsupported => Self::Unsupported,
            _ => Self::Identity,
        }
    }
}

impl From<PolicyError> for MixedError {
    fn from(e: PolicyError) -> Self {
        match e {
            PolicyError::Capacity | PolicyError::Limit => Self::Limit,
            PolicyError::Encoding => Self::Unsupported,
            _ => Self::Identity,
        }
    }
}

impl From<PlanError> for MixedError {
    fn from(e: PlanError) -> Self {
        match e {
            PlanError::DuplicateId => Self::DuplicateId,
            PlanError::Capacity | PlanError::Limit => Self::Limit,
            PlanError::Unsupported | PlanError::Field | PlanError::Refused => Self::Unsupported,
            PlanError::Identity | PlanError::Cycle | PlanError::Graph => Self::Identity,
        }
    }
}

impl From<BundleError> for MixedError {
    fn from(e: BundleError) -> Self {
        match e {
            BundleError::DuplicateKey(_) => Self::DuplicateId,
            BundleError::EmptyKey
            | BundleError::TooShort
            | BundleError::LengthMismatch { .. }
            | BundleError::BadIndexPointer { .. }
            | BundleError::EntryOutOfBounds { .. }
            | BundleError::UnpageAligned { .. }
            | BundleError::Cbor(_)
            | BundleError::Io(_) => Self::Limit,
            BundleError::BadMagic
            | BundleError::UnsupportedVersion(_)
            | BundleError::CrcMismatch { .. } => Self::Unsupported,
        }
    }
}

impl From<SelectionError> for MixedError {
    fn from(_: SelectionError) -> Self {
        Self::Identity
    }
}

fn kind_of(family: EntryFamily) -> &'static str {
    match family {
        EntryFamily::TenD => "10d",
        EntryFamily::Q42 => "q42",
        EntryFamily::Emf => "emf",
        EntryFamily::Qac => "qac",
        EntryFamily::M64 => "m64",
        EntryFamily::P64 => "p64",
        EntryFamily::Opaque => "blob",
    }
}

/// Publish one plain-candidate HMC. Duplicate `id` (even same family) refuses.
/// Empty bytes and more than [`MAX_PLAN_ENTRIES`] refuse as Limit. Id 0 refuses.
pub fn publish_mixed(entries: &[MixedEntry<'_>]) -> Result<MixedHmc, MixedError> {
    if entries.is_empty() || entries.len() > MAX_PLAN_ENTRIES {
        return Err(MixedError::Limit);
    }
    for (i, entry) in entries.iter().enumerate() {
        if entry.id == 0 {
            return Err(MixedError::Identity);
        }
        if entry.bytes.is_empty() {
            return Err(MixedError::Limit);
        }
        for prior in &entries[..i] {
            if prior.id == entry.id {
                return Err(MixedError::DuplicateId);
            }
        }
    }
    let nodes = [
        ClosureNode {
            digest: sha384(b"root"),
            kind: DependencyKind::Instrument,
            original_bytes: b"root",
        },
        ClosureNode {
            digest: sha384(b"policy"),
            kind: DependencyKind::LawPackage,
            original_bytes: b"policy",
        },
        ClosureNode {
            digest: sha384(b"consumer"),
            kind: DependencyKind::NativeModule,
            original_bytes: b"consumer",
        },
    ];
    let edges = [
        ClosureEdge {
            dependent: nodes[0].digest,
            dependency: nodes[1].digest,
        },
        ClosureEdge {
            dependent: nodes[0].digest,
            dependency: nodes[2].digest,
        },
    ];
    let mut writer = BundleWriter::new();
    let mut payloads = Vec::with_capacity(entries.len());
    for entry in entries {
        payloads.push(entry.bytes.to_vec());
        let key = format!("entry-{}", entry.id);
        writer.add_file(key, kind_of(entry.family), entry.bytes.to_vec(), None)?;
    }
    let source = writer.build()?;
    let fields = ManifestFields {
        source: sha384(&source),
        source_length: source.len() as u64,
        semantic_root: nodes[0].digest,
        policy: nodes[1].digest,
        consumer: nodes[2].digest,
        issuer: StrongDigest([31; 48]),
    };
    let candidate = verify_closure(nodes[0].digest, &nodes, &edges).map_err(|_| MixedError::Identity)?;
    let mut manifest = vec![0; MAX_MANIFEST_BYTES];
    let n = encode_manifest_into(fields, &candidate, &mut manifest)?;
    manifest.truncate(n);
    let parameters = ContextParameters {
        nonce: [1; 32],
        generation: 0,
        predecessor: None,
        semantic_contract: sha384(&manifest),
        policy: fields.policy,
        consumer_profile: fields.consumer,
    };
    let n_entries = entries.len();
    let mut assignments = [EntryAssignment {
        id: 0,
        inner_version: 1,
    }; MAX_PLAN_ENTRIES];
    for (i, entry) in entries.iter().enumerate() {
        assignments[i] = EntryAssignment {
            id: entry.id,
            inner_version: 1,
        };
    }
    let mut index = [EntryScratch::EMPTY; MAX_PLAN_ENTRIES];
    let mut ids_scratch = [0u32; MAX_PLAN_ENTRIES];
    let mut context_refs = [PageRef::EMPTY; MAX_PAGES];
    let mut entry_refs = [PageRef::EMPTY; MAX_PAGES];
    let mut publisher = PlainPublisher::prepare(
        &source,
        BundleReadLimits::default(),
        &assignments[..n_entries],
        PublisherOptions {
            parameters,
            class: PublicationClass::PlainCandidate,
            metadata_limit: MAX_METADATA,
        },
        &mut index[..n_entries],
        &mut ids_scratch[..n_entries],
        &mut context_refs,
        &mut entry_refs,
    )?;
    let mut output = vec![0; MAX_ENCODED];
    let mut scratch = vec![0; HASH_SCRATCH_BYTES];
    for page in 0..publisher.page_count() {
        publisher.emit_context_page(page, &mut output, &mut scratch, |_, _| Ok(()))?;
    }
    let n = publisher.encode_context_root(&mut output, &mut scratch)?;
    let context = output[..n].to_vec();
    let mut entry_page = Vec::new();
    for page in 0..publisher.page_count() {
        publisher.emit_entry_page(
            page,
            &mut output,
            &mut scratch,
            |_, _| Ok(()),
            |bytes, _| {
                entry_page = bytes.to_vec();
                Ok(())
            },
        )?;
    }
    let identity = publisher.finish(&context, &mut output, &mut scratch)?;
    let n = publisher.body_encoded_length(&context)?;
    let body = output[..n].to_vec();
    drop(publisher);
    let (ml_pk, ml_sk) = ml_dsa_65::KG::keygen_from_seed(&[71; 32]);
    let ed_sk = SigningKey::from_bytes(&[72; 32]);
    let ed_pk = ed_sk.verifying_key().to_bytes();
    let mut ids = [0u32; MAX_PLAN_ENTRIES];
    let mut families = [EntryFamily::Opaque; MAX_PLAN_ENTRIES];
    let mut roles = [RepresentationClass::Enhancement; MAX_PLAN_ENTRIES];
    let mut obligations = [Obligation::Optional; MAX_PLAN_ENTRIES];
    for (i, entry) in entries.iter().enumerate() {
        ids[i] = entry.id;
        families[i] = entry.family;
        roles[i] = entry.role;
        obligations[i] = entry.obligation;
    }
    Ok(MixedHmc {
        source,
        body,
        entry_page,
        identity,
        ids,
        families,
        roles,
        obligations,
        payloads,
        len: n_entries,
        nodes,
        edges,
        fields,
        manifest,
        ml_sk,
        ml_pk: ml_pk.into_bytes(),
        ed_sk,
        ed_pk,
    })
}

impl MixedHmc {
    pub fn source(&self) -> &[u8] {
        &self.source
    }
    pub fn body(&self) -> &[u8] {
        &self.body
    }
    pub fn entry_page(&self) -> &[u8] {
        &self.entry_page
    }
    pub fn identity(&self) -> CandidateIdentity {
        self.identity
    }
    pub fn entry_id(&self, i: usize) -> u32 {
        self.ids[..self.len][i]
    }
    pub fn family(&self, i: usize) -> EntryFamily {
        self.families[..self.len][i]
    }
    pub fn role(&self, i: usize) -> RepresentationClass {
        self.roles[..self.len][i]
    }
    pub fn obligation(&self, i: usize) -> Obligation {
        self.obligations[..self.len][i]
    }
    /// Original inner bytes passed to [`publish_mixed`], not a filename.
    pub fn payload(&self, i: usize) -> &[u8] {
        &self.payloads[i]
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn index_of(&self, id: u32) -> Option<usize> {
        self.ids[..self.len].iter().position(|x| *x == id)
    }
    fn keys(&self) -> cose::PairedPublicKeys<'_> {
        cose::PairedPublicKeys {
            ml_dsa_65: &self.ml_pk,
            ed25519: &self.ed_pk,
        }
    }
    /// Install the Scene/Fixture pins used by `select_entry`.
    pub fn installed(
        &self,
        purpose: &[u8],
    ) -> Result<(PinAuthority, LocalBootstrap, PinHandle, PinHandle), MixedError> {
        let (mut pins, admin) =
            PinAuthority::new_local_bootstrap().map_err(|_| MixedError::Identity)?;
        let handle = pins
            .install_pin(
                &admin,
                PinConfig {
                    principal: self.fields.issuer,
                    provider_epoch: KeyEpoch {
                        controller: 7,
                        epoch: 1,
                        purpose: KeyPurpose::ControllerSign,
                    },
                    keys: self.keys(),
                    purpose,
                    audience: b"reader",
                    not_before_unix: 10,
                    expires_unix: 100,
                },
                10,
            )
            .map_err(|_| MixedError::Identity)?;
        let publisher = pins
            .install_pin(
                &admin,
                PinConfig {
                    principal: self.fields.issuer,
                    provider_epoch: KeyEpoch {
                        controller: 7,
                        epoch: 1,
                        purpose: KeyPurpose::ControllerSign,
                    },
                    keys: self.keys(),
                    purpose: PUBLICATION_PURPOSE,
                    audience: b"reader",
                    not_before_unix: 10,
                    expires_unix: 100,
                },
                10,
            )
            .map_err(|_| MixedError::Identity)?;
        Ok((pins, admin, handle, publisher))
    }
    fn signed_digest(&self, digest: StrongDigest, purpose: &[u8]) -> Result<Vec<u8>, MixedError> {
        let binding = cose::Binding {
            descriptor_digest: &digest.0,
            key_ids: self.keys().key_ids(),
            controller_epoch: 1,
            purpose,
            audience: b"reader",
        };
        let keys = cose::SigningKeys {
            ml_dsa_65: &self.ml_sk,
            ed25519: &self.ed_sk,
        };
        let mut out = vec![0; cose::MAX_ENVELOPE_BYTES];
        let n = cose::sign_pair_raw(&binding, &keys, &mut cose::Workspace::new(), &mut out)
            .map_err(|_| MixedError::Identity)?;
        out.truncate(n);
        Ok(out)
    }
    fn authenticated<'a>(
        &'a self,
        pins: &PinAuthority,
        pin: PinHandle,
        now: u64,
    ) -> Result<crate::bundle::stream::policy::AuthenticatedClosure<'a>, MixedError> {
        let candidate = verify_closure(self.fields.semantic_root, &self.nodes, &self.edges)
            .map_err(|_| MixedError::Identity)?;
        let signed = self.signed_digest(sha384(&self.manifest), CLOSURE_PURPOSE)?;
        Ok(authenticate_closure(
            candidate,
            &self.manifest,
            pins,
            pin,
            self.keys().key_ids(),
            1,
            b"reader",
            &signed,
            now,
            &mut cose::Workspace::new(),
        )?)
    }
    /// Bind the published descriptor so callers can `select_entry` without republishing.
    pub fn bound<'a>(
        &'a self,
        pins: &PinAuthority,
        pin: PinHandle,
        publisher: PinHandle,
    ) -> Result<BoundClosure<'a>, MixedError> {
        let envelope = self.signed_digest(self.identity.descriptor_id(), PUBLICATION_PURPOSE)?;
        Ok(self.authenticated(pins, pin, 10)?.bind_descriptor(
            self.identity,
            &self.body,
            &mut vec![0; HASH_SCRATCH_BYTES],
            pins,
            self.publication(publisher, &envelope),
            10,
            &mut cose::Workspace::new(),
        )?)
    }
    fn publication<'a>(&self, pin: PinHandle, envelope: &'a [u8]) -> PublicationProof<'a> {
        PublicationProof {
            pin,
            key_ids: self.keys().key_ids(),
            epoch: 1,
            audience: b"reader",
            envelope,
        }
    }
    pub fn request(&self) -> LocalGrantRequest {
        LocalGrantRequest {
            principal: StrongDigest([41; 48]),
            recipient: StrongDigest([42; 48]),
            purpose: sha384(b"local-read"),
            operation: OperationId([1; 16]),
            not_before: 10,
            expires: 90,
            resource_ceiling: self.fields.source_length,
        }
    }
    /// `select_entry` by publish ordinal. `ordinal` is 0-based assignment order.
    pub fn select<'p>(
        &'p self,
        bound: &BoundClosure<'_>,
        ordinal: u32,
        index: &AdmittedIndex<'_>,
        rows: &mut [EntryRecord<'p>],
        framing: &mut [u8],
    ) -> Result<SelectedPlainEntry<'p>, MixedError> {
        Ok(select_entry(
            bound,
            self.body(),
            &self.source()[..64],
            index,
            self.entry_page(),
            ordinal,
            rows,
            framing,
        )?)
    }
}

/// Geometry: each EMF/QAC depends on the first TenD. Calibration: first EMF
/// depends on the first Q42. Missing-mandatory-calibration tests drop that id
/// from `present`.
pub fn mixed_plan(hmc: &MixedHmc) -> Result<MixedPlan, MixedError> {
    if hmc.is_empty() {
        return Err(MixedError::Limit);
    }
    let candidate = hmc.identity();
    let mut entries = Vec::with_capacity(hmc.len());
    for i in 0..hmc.len() {
        entries.push(PlanEntry::new(PlanEntrySpec {
            candidate,
            entry_id: hmc.entry_id(i),
            family: hmc.family(i),
            role: hmc.role(i),
            interval: None,
            timebase: None,
            estimate: ResourceEstimate::new(hmc.payload(i).len() as u64, 1, 1),
            obligation: hmc.obligation(i),
        })?);
    }
    let geometry_id = entries
        .iter()
        .find(|e| e.family() == EntryFamily::TenD)
        .map(|e| e.entry_id());
    let calibration_id = entries
        .iter()
        .find(|e| e.family() == EntryFamily::Q42)
        .map(|e| e.entry_id());
    let mut edges = Vec::new();
    if let Some(geo) = geometry_id {
        let to = entries
            .iter()
            .find(|e| e.entry_id() == geo)
            .ok_or(MixedError::Identity)?;
        for from in &entries {
            if from.entry_id() == geo {
                continue;
            }
            if from.family() == EntryFamily::Emf || from.family() == EntryFamily::Qac {
                edges.push(DependencyEdge::bind(
                    from,
                    to,
                    Obligation::Mandatory,
                    DependencyReason::Geometry,
                )?);
            }
        }
    }
    if let Some(calib) = calibration_id {
        let to = entries
            .iter()
            .find(|e| e.entry_id() == calib)
            .ok_or(MixedError::Identity)?;
        if let Some(from) = entries.iter().find(|e| e.family() == EntryFamily::Emf) {
            edges.push(DependencyEdge::bind(
                from,
                to,
                Obligation::Mandatory,
                DependencyReason::Calibration,
            )?);
        }
    }
    Ok(MixedPlan {
        candidate,
        entries,
        edges,
        geometry_id,
        calibration_id,
    })
}

impl MixedPlan {
    pub fn candidate(&self) -> CandidateIdentity {
        self.candidate
    }
    pub fn entries(&self) -> &[PlanEntry] {
        &self.entries
    }
    pub fn edges(&self) -> &[DependencyEdge] {
        &self.edges
    }
    pub fn geometry_id(&self) -> Option<u32> {
        self.geometry_id
    }
    pub fn calibration_id(&self) -> Option<u32> {
        self.calibration_id
    }
    pub fn inventory(&self) -> Result<Inventory<'_>, MixedError> {
        Ok(Inventory::compile(
            self.candidate,
            &self.entries,
            &self.edges,
        )?)
    }
}

/// Copied construction logic from `formats/m64_tests.rs` `m64_bytes`.
#[cfg(all(test, not(target_arch = "wasm32")))]
fn m64_container_bytes(with_metric: bool, magic: [u8; 4], full_codec: bool) -> Vec<u8> {
    use crate::q42::{
        chunk_integrity::blake3_trailer,
        m64_matter::{
            M64ChunkDirectoryEntry, M64Header, M64LocalMetricHeader, M64StrataColumnRecord,
            M64_CODEC_BLAKE3_FULL, M64_FLAG_LOCAL_METRIC_LRM3, M64_MODE_STRATA,
        },
    };
    let header_size = 64usize;
    let metric_len = if with_metric { 128 } else { 0 };
    let dir = header_size + metric_len;
    let payload_off = dir + 64;
    let rec_end = payload_off + 64;
    let total = rec_end + if full_codec { 64 } else { 0 };
    let mut rec = M64StrataColumnRecord::default();
    rec.surface_elevation_mm = 1234;
    rec.update_crc();
    let record = bytemuck::bytes_of(&rec);
    let mut header = M64Header::default();
    header.magic = magic;
    header.total_file_size = total as u64;
    header.chunk_count = 1;
    header.chunk_table_offset = dir as u64;
    header.container_flags = if with_metric {
        M64_FLAG_LOCAL_METRIC_LRM3 as u32
    } else {
        0
    };
    header.update_crc();
    let mut row = M64ChunkDirectoryEntry {
        chunk_uuid: [0x31; 16],
        spatial_hilbert_index: 7,
        byte_offset: payload_off as u64,
        byte_length: (64 + if full_codec { 64 } else { 0 }) as u32,
        record_count: 1,
        srid_geodetic_code: 4326,
        lut_bank_epoch_id: 2,
        chunk_record_mode: M64_MODE_STRATA,
        compression_flags: if full_codec { M64_CODEC_BLAKE3_FULL } else { 2 },
        bao_root_hash_prefix: [0; 12],
        entry_crc32c: 0,
    };
    if !full_codec {
        row.bao_root_hash_prefix
            .copy_from_slice(&blake3::hash(record).as_bytes()[..12]);
    }
    row.update_crc();
    let mut out = vec![0; total];
    out[..64].copy_from_slice(bytemuck::bytes_of(&header));
    if with_metric {
        let mut metric = M64LocalMetricHeader::default();
        metric.anchor_did_hash = 0x1122_3344_5566_7788;
        metric.anisotropic_scale_um = [2.0, 3.0, 4.0];
        metric.update_crc();
        out[64..192].copy_from_slice(bytemuck::bytes_of(&metric));
    }
    out[dir..dir + 64].copy_from_slice(bytemuck::bytes_of(&row));
    out[payload_off..rec_end].copy_from_slice(record);
    if full_codec {
        out[rec_end..].copy_from_slice(&blake3_trailer(record));
    }
    out
}

#[cfg(all(test, not(target_arch = "wasm32")))]
fn p64_container_bytes() -> Result<Vec<u8>, MixedError> {
    use crate::p64_weight::{align_up, compile_gguf_to_p64, has_p64_magic};
    fn put_kv_u32(out: &mut Vec<u8>, key: &str, value: u32) {
        out.extend_from_slice(&(key.len() as u64).to_le_bytes());
        out.extend_from_slice(key.as_bytes());
        out.extend_from_slice(&4u32.to_le_bytes());
        out.extend_from_slice(&value.to_le_bytes());
    }
    fn put_tensor(out: &mut Vec<u8>, name: &str, dims: &[u64], offset: u64) {
        out.extend_from_slice(&(name.len() as u64).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&(dims.len() as u32).to_le_bytes());
        for dimension in dims {
            out.extend_from_slice(&dimension.to_le_bytes());
        }
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
    }
    let mut gguf = Vec::new();
    gguf.extend_from_slice(b"GGUF");
    gguf.extend_from_slice(&3u32.to_le_bytes());
    gguf.extend_from_slice(&2u64.to_le_bytes());
    gguf.extend_from_slice(&3u64.to_le_bytes());
    put_kv_u32(&mut gguf, "llama.block_count", 1);
    put_kv_u32(&mut gguf, "llama.embedding_length", 4);
    put_kv_u32(&mut gguf, "llama.attention.head_count", 1);
    put_tensor(&mut gguf, "blk.0.attn_q.weight", &[4, 4], 0);
    put_tensor(&mut gguf, "token_embd.weight", &[4, 2], 64);
    gguf.resize(align_up(gguf.len(), 32), 0);
    for byte in 0u8..96 {
        gguf.push(byte.wrapping_mul(17).wrapping_add(3));
    }
    let p64 = compile_gguf_to_p64(&gguf, 8).map_err(|_| MixedError::Unsupported)?;
    if !has_p64_magic(&p64) || p64.is_empty() {
        return Err(MixedError::Unsupported);
    }
    Ok(p64)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
fn q42_volume_bytes() -> Result<Vec<u8>, MixedError> {
    use crate::mini_parser::hash_token;
    use crate::q42::q42_volume::write_unified_volume;
    use crate::NQuin;
    use std::collections::HashMap;
    use std::io::Read;
    use tempfile::NamedTempFile;
    let subject = hash_token("urn:q42:authenticated-subject");
    let predicate = hash_token("urn:q42:authenticated-predicate");
    let object = hash_token("urn:q42:authenticated-object");
    let quin = NQuin {
        subject,
        predicate,
        object,
        context: 0,
        metadata: 0,
        parity: 0,
    };
    let mut lexicon = HashMap::new();
    lexicon.insert(subject, "urn:q42:authenticated-subject".to_string());
    lexicon.insert(predicate, "urn:q42:authenticated-predicate".to_string());
    lexicon.insert(object, "urn:q42:authenticated-object".to_string());
    let file = NamedTempFile::new().map_err(|_| MixedError::Limit)?;
    write_unified_volume(file.path(), &lexicon, &[(object, object)], &[vec![quin]])
        .map_err(|_| MixedError::Unsupported)?;
    let mut q42 = Vec::new();
    std::fs::File::open(file.path())
        .map_err(|_| MixedError::Limit)?
        .read_to_end(&mut q42)
        .map_err(|_| MixedError::Limit)?;
    if q42.is_empty() {
        return Err(MixedError::Limit);
    }
    Ok(q42)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
fn mesh_triangle(positions: [[f32; 3]; 3]) -> crate::render::assets::Mesh {
    let mut min = positions[0];
    let mut max = positions[0];
    for p in positions {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    crate::render::assets::Mesh {
        positions: positions.to_vec(),
        triangles: vec![[0, 1, 2]],
        min,
        max,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
const QAC_OPAQUE: &[u8] =
    include_bytes!("../../../q42/field_envelope/fixtures/candidate/qac_candidate-opaque.qac");
#[cfg(all(test, not(target_arch = "wasm32")))]
const QAC_PCM: &[u8] = include_bytes!(
    "../../../q42/field_envelope/fixtures/candidate/qac_candidate-pcm-signed-zero.qac"
);
#[cfg(all(test, not(target_arch = "wasm32")))]
const EMF_OPAQUE: &[u8] =
    include_bytes!("../../../q42/field_envelope/fixtures/candidate/emf_candidate-opaque.emf");
#[cfg(all(test, not(target_arch = "wasm32")))]
const EMF_ALIGN: &[u8] = include_bytes!(
    "../../../q42/field_envelope/fixtures/candidate/emf_candidate-aligned4096.emf"
);

/// Default §6 corpus: ≥2 TenD, ≥2 EMF, ≥2 QAC, plus Q42, M64, Opaque.
#[cfg(all(test, not(target_arch = "wasm32")))]
pub fn default_mixed_entries() -> Result<MixedHmc, MixedError> {
    let tend_a = crate::render::compile_10d::compile_mesh_to_10d(&mesh_triangle([
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ]))
    .map_err(|_| MixedError::Unsupported)?;
    let tend_b = crate::render::compile_10d::compile_mesh_to_10d(&mesh_triangle([
        [2.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
        [2.0, 1.0, 0.0],
    ]))
    .map_err(|_| MixedError::Unsupported)?;
    if tend_a == tend_b {
        return Err(MixedError::Identity);
    }
    let m64 = m64_container_bytes(false, *b"m64\0", true);
    let q42 = q42_volume_bytes()?;
    let p64 = p64_container_bytes()?;
    let entries = [
        MixedEntry {
            id: MIXED_ID_TEND_A,
            family: EntryFamily::TenD,
            role: RepresentationClass::Base,
            obligation: Obligation::Mandatory,
            bytes: &tend_a,
        },
        MixedEntry {
            id: MIXED_ID_TEND_B,
            family: EntryFamily::TenD,
            role: RepresentationClass::Base,
            obligation: Obligation::Mandatory,
            bytes: &tend_b,
        },
        MixedEntry {
            id: MIXED_ID_EMF_OPAQUE,
            family: EntryFamily::Emf,
            role: RepresentationClass::Base,
            obligation: Obligation::Mandatory,
            bytes: EMF_OPAQUE,
        },
        MixedEntry {
            id: MIXED_ID_EMF_ALIGN,
            family: EntryFamily::Emf,
            role: RepresentationClass::Base,
            obligation: Obligation::Mandatory,
            bytes: EMF_ALIGN,
        },
        MixedEntry {
            id: MIXED_ID_QAC_OPAQUE,
            family: EntryFamily::Qac,
            role: RepresentationClass::Base,
            obligation: Obligation::Mandatory,
            bytes: QAC_OPAQUE,
        },
        MixedEntry {
            id: MIXED_ID_QAC_PCM,
            family: EntryFamily::Qac,
            role: RepresentationClass::Base,
            obligation: Obligation::Mandatory,
            bytes: QAC_PCM,
        },
        MixedEntry {
            id: MIXED_ID_Q42,
            family: EntryFamily::Q42,
            role: RepresentationClass::Metadata,
            obligation: Obligation::Mandatory,
            bytes: &q42,
        },
        MixedEntry {
            id: MIXED_ID_M64,
            family: EntryFamily::M64,
            role: RepresentationClass::Enhancement,
            obligation: Obligation::Optional,
            bytes: &m64,
        },
        MixedEntry {
            id: MIXED_ID_P64,
            family: EntryFamily::P64,
            role: RepresentationClass::Enhancement,
            obligation: Obligation::Optional,
            bytes: &p64,
        },
        MixedEntry {
            id: MIXED_ID_OPAQUE,
            family: EntryFamily::Opaque,
            role: RepresentationClass::Enhancement,
            obligation: Obligation::Optional,
            bytes: MIXED_OPAQUE_BYTES,
        },
    ];
    publish_mixed(&entries)
}
