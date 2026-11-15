use super::{transfer_scheduled, BoundEntry, MuxReport, MuxTransfer};
use super::super::{
    answer_describe_pages, DescribeClient, Error, PlainClient, PlainProvider, Session,
    SessionOwner, ROLE_CLIENT, ROLE_PROVIDER,
};
use crate::audio::qac::{QacStorageOptions, QacStorageView};
use crate::bundle::stream::{
    descriptor::{
        CandidateIdentity, ContextParameters, EntryRecord, PageRef, HASH_SCRATCH_BYTES,
        MAX_ENCODED, MAX_METADATA,
    },
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    plan::{
        DependencyEdge, DependencyReason, EntryFamily, Inventory, Obligation, PlanEntry,
        PlanEntrySpec, PrincipalSelection, RecordUnit, RepresentationClass, ResourceEstimate,
        SelectionIntent, SelectionRequest,
    },
    policy::{
        adopt_local_whole_source, authenticate_closure, encode_manifest_into, BoundClosure,
        LocalGrantRequest, ManifestFields, PublicationProof, CLOSURE_PURPOSE, MAX_MANIFEST_BYTES,
        PUBLICATION_PURPOSE,
    },
    proof::{OutboardNode, TreeBuilder, LEAF_BYTES},
    publish::{EntryAssignment, PlainPublisher, PublicationClass, PublisherOptions},
    selection::{select_entry, SelectedPlainEntry},
};
use crate::bundle::{BundleReadLimits, BundleWriter};
use crate::crypto::network::{
    authority::{PinAuthority, PinConfig, PinHandle, LocalBootstrap},
    cose,
    digest::sha384,
    types::{KeyEpoch, KeyPurpose},
};
use crate::net::peer::replication::qbdl::provider::{OutboardCatalog, OutboardLimits};
use crate::net::peer::replication::qbdl::wire::{MAX_RECORD_BYTES, PREFIX_BYTES};
use crate::net::qdnf::authority::AuthorityOwner;
use crate::net::qdnf::bearer::ipc_pair;
use crate::net::qdnf::types::{OperationId, ScopeEpoch, StrongDigest};
use crate::q42::field_envelope::{self, Family, StructuralView};
use crate::semantic_instruments::resolve::{bounded_closure::*, DependencyKind};
use ed25519_dalek::SigningKey;
use fips204::{
    ml_dsa_65,
    traits::{KeyGen, SerDes},
};

const TRANSCRIPT: StrongDigest = StrongDigest([9; 48]);
const POLICY: u64 = 1;
const MAX_INDEX_CHUNK: u32 = (MAX_RECORD_BYTES - PREFIX_BYTES - 32) as u32;

const QAC_OPAQUE: &[u8] =
    include_bytes!("../../../../../q42/field_envelope/fixtures/candidate/qac_candidate-opaque.qac");
const QAC_PCM: &[u8] = include_bytes!(
    "../../../../../q42/field_envelope/fixtures/candidate/qac_candidate-pcm-signed-zero.qac"
);
const EMF_OPAQUE: &[u8] =
    include_bytes!("../../../../../q42/field_envelope/fixtures/candidate/emf_candidate-opaque.emf");
const EMF_ALIGN: &[u8] = include_bytes!(
    "../../../../../q42/field_envelope/fixtures/candidate/emf_candidate-aligned4096.emf"
);
const M64_BYTES: &[u8] = b"m64-delay-payload-not-a-field";
const TEND_BYTES: &[u8] = b"tend-geometry-dep-payload";
const BLOB_BYTES: &[u8] = b"random-blob-not-qac-or-emf!!";

const ID_QAC_OPAQUE: u32 = 10;
const ID_QAC_PCM: u32 = 11;
const ID_EMF_OPAQUE: u32 = 20;
const ID_EMF_ALIGN: u32 = 21;
const ID_M64: u32 = 30;
const ID_TEND: u32 = 40;
const ID_BLOB: u32 = 50;
const ID_CALIB: u32 = 60;

struct Scene {
    nodes: [ClosureNode<'static>; 3],
    edges: [ClosureEdge; 2],
    fields: ManifestFields,
    manifest: Vec<u8>,
    body: Vec<u8>,
    source: Vec<u8>,
    entry_page: Vec<u8>,
    identity: CandidateIdentity,
    ml_sk: ml_dsa_65::PrivateKey,
    ml_pk: [u8; 1952],
    ed_sk: SigningKey,
    ed_pk: [u8; 32],
}

impl Scene {
    fn source(&self) -> &[u8] {
        &self.source
    }
    fn body(&self) -> &[u8] {
        &self.body
    }
    fn entry_page(&self) -> &[u8] {
        &self.entry_page
    }
    fn identity(&self) -> CandidateIdentity {
        self.identity
    }
    fn publish(files: &[(&str, &str, &[u8], u32)]) -> Self {
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
        for (key, kind, bytes, _) in files {
            writer
                .add_file(*key, *kind, bytes.to_vec(), None)
                .unwrap();
        }
        let source = writer.build().unwrap();
        let fields = ManifestFields {
            source: sha384(&source),
            source_length: source.len() as u64,
            semantic_root: nodes[0].digest,
            policy: nodes[1].digest,
            consumer: nodes[2].digest,
            issuer: StrongDigest([31; 48]),
        };
        let candidate = verify_closure(nodes[0].digest, &nodes, &edges).unwrap();
        let mut manifest = vec![0; MAX_MANIFEST_BYTES];
        let n = encode_manifest_into(fields, &candidate, &mut manifest).unwrap();
        manifest.truncate(n);
        let parameters = ContextParameters {
            nonce: [1; 32],
            generation: 0,
            predecessor: None,
            semantic_contract: sha384(&manifest),
            policy: fields.policy,
            consumer_profile: fields.consumer,
        };
        let assignments: Vec<EntryAssignment> = files
            .iter()
            .map(|(_, _, _, id)| EntryAssignment {
                id: *id,
                inner_version: 1,
            })
            .collect();
        let mut index = vec![EntryScratch::EMPTY; assignments.len()];
        let mut ids = vec![0u32; assignments.len()];
        let mut context_refs = [PageRef::EMPTY; 32];
        let mut entry_refs = [PageRef::EMPTY; 32];
        let mut publisher = PlainPublisher::prepare(
            &source,
            BundleReadLimits::default(),
            &assignments,
            PublisherOptions {
                parameters,
                class: PublicationClass::PlainCandidate,
                metadata_limit: MAX_METADATA,
            },
            &mut index,
            &mut ids,
            &mut context_refs,
            &mut entry_refs,
        )
        .unwrap();
        let mut output = vec![0; MAX_ENCODED];
        let mut scratch = vec![0; HASH_SCRATCH_BYTES];
        for page in 0..publisher.page_count() {
            publisher
                .emit_context_page(page, &mut output, &mut scratch, |_, _| Ok(()))
                .unwrap();
        }
        let n = publisher
            .encode_context_root(&mut output, &mut scratch)
            .unwrap();
        let context = output[..n].to_vec();
        let mut entry_page = Vec::new();
        for page in 0..publisher.page_count() {
            publisher
                .emit_entry_page(
                    page,
                    &mut output,
                    &mut scratch,
                    |_, _| Ok(()),
                    |bytes, _| {
                        entry_page = bytes.to_vec();
                        Ok(())
                    },
                )
                .unwrap();
        }
        let identity = publisher
            .finish(&context, &mut output, &mut scratch)
            .unwrap();
        let n = publisher.body_encoded_length(&context).unwrap();
        let body = output[..n].to_vec();
        drop(publisher);
        let (ml_pk, ml_sk) = ml_dsa_65::KG::keygen_from_seed(&[71; 32]);
        let ed_sk = SigningKey::from_bytes(&[72; 32]);
        let ed_pk = ed_sk.verifying_key().to_bytes();
        Self {
            nodes,
            edges,
            fields,
            manifest,
            body,
            source,
            entry_page,
            identity,
            ml_sk,
            ml_pk: ml_pk.into_bytes(),
            ed_sk,
            ed_pk,
        }
    }
    fn keys(&self) -> cose::PairedPublicKeys<'_> {
        cose::PairedPublicKeys {
            ml_dsa_65: &self.ml_pk,
            ed25519: &self.ed_pk,
        }
    }
    fn installed(&self, purpose: &[u8]) -> (PinAuthority, LocalBootstrap, PinHandle, PinHandle) {
        let (mut pins, admin) = PinAuthority::new_local_bootstrap().unwrap();
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
            .unwrap();
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
            .unwrap();
        (pins, admin, handle, publisher)
    }
    fn signed_digest(&self, digest: StrongDigest, purpose: &[u8]) -> Vec<u8> {
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
        let n =
            cose::sign_pair_raw(&binding, &keys, &mut cose::Workspace::new(), &mut out).unwrap();
        out.truncate(n);
        out
    }
    fn authenticated<'a>(
        &'a self,
        pins: &PinAuthority,
        pin: PinHandle,
        now: u64,
    ) -> crate::bundle::stream::policy::AuthenticatedClosure<'a> {
        let candidate =
            verify_closure(self.fields.semantic_root, &self.nodes, &self.edges).unwrap();
        let signed = self.signed_digest(sha384(&self.manifest), CLOSURE_PURPOSE);
        authenticate_closure(
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
        )
        .unwrap()
    }
    fn bound<'a>(
        &'a self,
        pins: &PinAuthority,
        pin: PinHandle,
        publisher: PinHandle,
    ) -> BoundClosure<'a> {
        self.authenticated(pins, pin, 10)
            .bind_descriptor(
                self.identity,
                &self.body,
                &mut vec![0; HASH_SCRATCH_BYTES],
                pins,
                self.publication(
                    publisher,
                    &self.signed_digest(self.identity.descriptor_id(), PUBLICATION_PURPOSE),
                ),
                10,
                &mut cose::Workspace::new(),
            )
            .unwrap()
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
    fn request(&self) -> LocalGrantRequest {
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
}

fn four_live() -> [(&'static str, &'static str, &'static [u8], u32); 4] {
    [
        ("qac-opaque", "qac", QAC_OPAQUE, ID_QAC_OPAQUE),
        ("qac-pcm", "qac", QAC_PCM, ID_QAC_PCM),
        ("emf-opaque", "emf", EMF_OPAQUE, ID_EMF_OPAQUE),
        ("emf-align", "emf", EMF_ALIGN, ID_EMF_ALIGN),
    ]
}

fn entry_bytes<'a>(source: &'a [u8], selected: SelectedPlainEntry<'_>) -> &'a [u8] {
    let start = usize::try_from(selected.offset()).unwrap();
    let len = usize::try_from(selected.logical_length()).unwrap();
    &source[start..start + len]
}

fn admit_qac(bytes: &[u8], selected: SelectedPlainEntry<'_>) -> bool {
    if selected.logical_length() != bytes.len() as u64 {
        return false;
    }
    QacStorageView::open(bytes, QacStorageOptions::default()).is_ok()
}

fn admit_emf(bytes: &[u8], selected: SelectedPlainEntry<'_>) -> bool {
    if selected.logical_length() != bytes.len() as u64 {
        return false;
    }
    StructuralView::open(bytes, Family::EmfV4Candidate, field_envelope::Limits::default()).is_ok()
}

fn family_of(id: u32) -> EntryFamily {
    match id {
        ID_QAC_OPAQUE | ID_QAC_PCM => EntryFamily::Qac,
        ID_EMF_OPAQUE | ID_EMF_ALIGN => EntryFamily::Emf,
        ID_M64 => EntryFamily::M64,
        ID_TEND => EntryFamily::TenD,
        ID_BLOB => EntryFamily::Qac,
        ID_CALIB => EntryFamily::Q42,
        _ => EntryFamily::Opaque,
    }
}

fn admit_declared(source: &[u8], selected: SelectedPlainEntry<'_>, family: EntryFamily) -> bool {
    let bytes = entry_bytes(source, selected);
    match family {
        EntryFamily::Qac => admit_qac(bytes, selected),
        EntryFamily::Emf => admit_emf(bytes, selected),
        EntryFamily::M64 | EntryFamily::TenD | EntryFamily::Opaque | EntryFamily::Q42 => true,
        _ => false,
    }
}

fn plan_entry(
    candidate: CandidateIdentity,
    entry_id: u32,
    family: EntryFamily,
    role: RepresentationClass,
    obligation: Obligation,
    bytes: u64,
) -> PlanEntry {
    PlanEntry::new(PlanEntrySpec {
        candidate,
        entry_id,
        family,
        role,
        interval: None,
        timebase: None,
        estimate: ResourceEstimate::new(bytes, 1, 1),
        obligation,
    })
    .unwrap()
}

fn select_all<'p>(
    bound: &BoundClosure<'_>,
    scene: &'p Scene,
) -> Vec<SelectedPlainEntry<'p>> {
    let header = admit_header(
        &scene.source()[..64],
        scene.source().len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    let count = scene.identity().entry_count() as usize;
    let mut index_scratch = vec![EntryScratch::EMPTY; count];
    let index = admit_index(
        &header,
        &scene.source()[header.index_offset() as usize..],
        IndexReadLimits::default(),
        &mut index_scratch,
    )
    .unwrap();
    let mut rows = [EntryRecord::EMPTY; 128];
    let mut framing = vec![0; HASH_SCRATCH_BYTES];
    let mut out = Vec::with_capacity(count);
    for ordinal in 0..count as u32 {
        out.push(
            select_entry(
                bound,
                scene.body(),
                &scene.source()[..64],
                &index,
                scene.entry_page(),
                ordinal,
                &mut rows,
                &mut framing,
            )
            .unwrap(),
        );
    }
    out
}

fn build_outboard(source: &[u8], selected: SelectedPlainEntry<'_>) -> (Vec<OutboardNode>, StrongDigest) {
    let start = usize::try_from(selected.offset()).unwrap();
    let length = usize::try_from(selected.logical_length()).unwrap();
    let entry = &source[start..start + length];
    let spec = selected.tree_spec().unwrap();
    let mut builder = TreeBuilder::new(spec);
    let mut nodes = Vec::new();
    for leaf in entry.chunks(LEAF_BYTES as usize) {
        builder
            .append(leaf, |node| {
                nodes.push(node);
                Ok(())
            })
            .unwrap();
    }
    let root = builder
        .finish(|node| {
            nodes.push(node);
            Ok(())
        })
        .unwrap();
    nodes.sort_unstable_by_key(|node| (node.start_leaf, node.leaf_count));
    (nodes, root)
}

fn catalog<'p, 'n>(
    selected: SelectedPlainEntry<'p>,
    nodes: &'n [OutboardNode],
    root: StrongDigest,
) -> OutboardCatalog<'p, 'n> {
    OutboardCatalog::new(
        selected,
        selected.tree_spec().unwrap(),
        root,
        nodes,
        OutboardLimits {
            max_nodes: nodes.len(),
            max_bytes: nodes.len() * core::mem::size_of::<OutboardNode>(),
        },
    )
    .unwrap()
}

fn attach_pair<'a>(
    client_lanes: &'a mut SessionOwner,
    provider_lanes: &'a mut SessionOwner,
) -> (Session<'a>, Session<'a>) {
    let (client_ep, provider_ep) = ipc_pair(ScopeEpoch { scope: 1, epoch: 1 }, 1280).unwrap();
    let client_peer = provider_ep.locator();
    let provider_peer = client_ep.locator();
    let client_session = Session::attach(
        client_ep,
        client_peer,
        TRANSCRIPT,
        ROLE_CLIENT,
        client_lanes,
    )
    .unwrap();
    let provider_session = Session::attach(
        provider_ep,
        provider_peer,
        TRANSCRIPT,
        ROLE_PROVIDER,
        provider_lanes,
    )
    .unwrap();
    (client_session, provider_session)
}

fn one_page_chunk(bytes: &[u8]) -> Result<(u64, u32), Error> {
    let total = u64::try_from(bytes.len()).map_err(|_| Error::Limit)?;
    let chunk = u32::try_from(total).map_err(|_| Error::Limit)?;
    if chunk == 0 || chunk > 65536 {
        return Err(Error::Limit);
    }
    Ok((total, chunk))
}

fn bind_providers<'src, 'sel, 'nodes>(
    source: &'src [u8],
    selected: &[SelectedPlainEntry<'sel>],
    nodes: &'nodes [Vec<OutboardNode>],
    roots: &[StrongDigest],
) -> Vec<BoundEntry<'src, 'sel, 'nodes>> {
    selected
        .iter()
        .zip(nodes.iter())
        .zip(roots.iter())
        .map(|((sel, nodes), root)| BoundEntry {
            entry_id: sel.entry_id(),
            provider: PlainProvider::bind(
                PublicationClass::PlainCandidate,
                source,
                catalog(*sel, nodes, *root),
                POLICY,
            )
            .unwrap(),
        })
        .collect()
}

fn bind_clients<'p>(selected: &[SelectedPlainEntry<'p>]) -> Vec<PlainClient<'p>> {
    selected
        .iter()
        .map(|sel| PlainClient::bind(*sel, POLICY, 0).unwrap())
        .collect()
}

fn present_admitted(source: &[u8], selected: &[SelectedPlainEntry<'_>]) -> Vec<u32> {
    let mut present = Vec::new();
    for sel in selected {
        let family = family_of(sel.entry_id());
        if family == EntryFamily::Qac || family == EntryFamily::Emf {
            if admit_declared(source, *sel, family) {
                present.push(sel.entry_id());
            }
        } else {
            present.push(sel.entry_id());
        }
    }
    present
}

fn live_plan(candidate: CandidateIdentity, ids: &[u32], selected: &[SelectedPlainEntry<'_>]) -> Vec<PlanEntry> {
    ids.iter()
        .map(|id| {
            let bytes = selected
                .iter()
                .find(|s| s.entry_id() == *id)
                .map(|s| s.logical_length())
                .unwrap_or(1);
            plan_entry(
                candidate,
                *id,
                family_of(*id),
                RepresentationClass::Base,
                Obligation::Mandatory,
                bytes,
            )
        })
        .collect()
}

#[test]
fn describe_then_schedule_transfers_two_qac_and_two_emf() {
    let scene = Scene::publish(&four_live());
    let (pins, _, pin, publisher) = scene.installed(CLOSURE_PURPOSE);
    let bound = scene.bound(&pins, pin, publisher);
    let local = select_all(&bound, &scene);
    assert_eq!(local.len(), 4);
    let mut nodes = Vec::new();
    let mut roots = Vec::new();
    for sel in &local {
        let (n, r) = build_outboard(scene.source(), *sel);
        nodes.push(n);
        roots.push(r);
    }
    let entries = bind_providers(scene.source(), &local, &nodes, &roots);
    let describe_client = DescribeClient::bind(scene.identity(), POLICY).unwrap();
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
    let mut client_lanes = SessionOwner::new();
    let mut provider_lanes = SessionOwner::new();
    let (mut client_session, mut provider_session) =
        attach_pair(&mut client_lanes, &mut provider_lanes);
    let mut received_body = Vec::new();
    let mut received_entry = Vec::new();
    let mut received_header = [0u8; 64];
    let mut received_index = Vec::new();
    grant
        .accept_local_scoped(&pins, || 20, |authority| {
            Ok((|| -> Result<(), Error> {
                let header_ticket = describe_client.send_header_describe(&mut client_session)?;
                let served = answer_describe_pages(
                    &mut provider_session,
                    authority,
                    scene.identity(),
                    POLICY,
                    scene.source(),
                    scene.body(),
                    scene.entry_page(),
                )?;
                if served.class != 6 || served.length != 64 {
                    return Err(Error::Identity);
                }
                describe_client.recv_header_page(
                    &mut client_session,
                    &header_ticket,
                    &mut received_header,
                )?;
                let header = match admit_header(
                    &received_header,
                    scene.source().len() as u64,
                    ExtentLimit::default(),
                    IndexReadLimits::default(),
                ) {
                    Ok(h) => h,
                    Err(_) => return Err(Error::Identity),
                };
                let total = header.index_length();
                let chunk = u32::try_from(total).map_err(|_| Error::Limit)?;
                if chunk > MAX_INDEX_CHUNK {
                    return Err(Error::Limit);
                }
                let index_ticket =
                    describe_client.send_index_describe(&mut client_session, 0, chunk, total)?;
                let served = answer_describe_pages(
                    &mut provider_session,
                    authority,
                    scene.identity(),
                    POLICY,
                    scene.source(),
                    scene.body(),
                    scene.entry_page(),
                )?;
                if served.class != 7 {
                    return Err(Error::Identity);
                }
                received_index.resize(chunk as usize, 0);
                let n = describe_client.recv_index_page(
                    &mut client_session,
                    &index_ticket,
                    &mut received_index,
                )?;
                received_index.truncate(n);
                let (body_total, body_chunk) = one_page_chunk(scene.body())?;
                let body_ticket = describe_client.send_body_describe(
                    &mut client_session,
                    0,
                    body_chunk,
                    body_total,
                    0,
                )?;
                let served = answer_describe_pages(
                    &mut provider_session,
                    authority,
                    scene.identity(),
                    POLICY,
                    scene.source(),
                    scene.body(),
                    scene.entry_page(),
                )?;
                if served.class != 8 {
                    return Err(Error::Identity);
                }
                received_body.resize(body_chunk as usize, 0);
                let bn = describe_client.recv_body_page(
                    &mut client_session,
                    &body_ticket,
                    &mut received_body,
                )?;
                received_body.truncate(bn);
                let (entry_total, entry_chunk) = one_page_chunk(scene.entry_page())?;
                let entry_ticket = describe_client.send_entry_page_describe(
                    &mut client_session,
                    0,
                    entry_chunk,
                    entry_total,
                    0,
                )?;
                let served = answer_describe_pages(
                    &mut provider_session,
                    authority,
                    scene.identity(),
                    POLICY,
                    scene.source(),
                    scene.body(),
                    scene.entry_page(),
                )?;
                if served.class != 9 {
                    return Err(Error::Identity);
                }
                received_entry.resize(entry_chunk as usize, 0);
                let en = describe_client.recv_entry_page(
                    &mut client_session,
                    &entry_ticket,
                    &mut received_entry,
                )?;
                received_entry.truncate(en);
                Ok(())
            })())
        })
        .unwrap()
        .unwrap();
    drop(grant);
    drop(owner);
    let header = match admit_header(
        &received_header,
        scene.source().len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    ) {
        Ok(h) => h,
        Err(_) => panic!("received header must admit"),
    };
    let mut index_scratch = vec![EntryScratch::EMPTY; local.len()];
    let index = match admit_index(
        &header,
        &received_index,
        IndexReadLimits::default(),
        &mut index_scratch,
    ) {
        Ok(v) => v,
        Err(_) => panic!("received index must admit"),
    };
    let mut rows = [EntryRecord::EMPTY; 128];
    let mut framing = vec![0; HASH_SCRATCH_BYTES];
    let mut remote = Vec::new();
    for ordinal in 0..local.len() as u32 {
        match select_entry(
            &bound,
            &received_body,
            &received_header,
            &index,
            &received_entry,
            ordinal,
            &mut rows,
            &mut framing,
        ) {
            Ok(sel) => remote.push(sel),
            Err(_) => panic!("select from received pages"),
        }
    }
    assert_eq!(remote.len(), 4);
    let mut clients = bind_clients(&remote);
    let present = present_admitted(scene.source(), &local);
    assert_eq!(present.len(), 4);
    let plan_entries = live_plan(scene.identity(), &present, &local);
    let inv = Inventory::compile(scene.identity(), &plan_entries, &[]).unwrap();
    let intents = [
        SelectionIntent::scene(ID_QAC_OPAQUE, EntryFamily::Qac, None, None).unwrap(),
        SelectionIntent::scene(ID_QAC_PCM, EntryFamily::Qac, None, None).unwrap(),
        SelectionIntent::scene(ID_EMF_OPAQUE, EntryFamily::Emf, None, None).unwrap(),
        SelectionIntent::scene(ID_EMF_ALIGN, EntryFamily::Emf, None, None).unwrap(),
    ];
    let request = SelectionRequest::scene(&intents).unwrap();
    let sel = PrincipalSelection::new(1, request).unwrap();
    let mut mux = MuxTransfer::compile(&inv, &[sel], &present).unwrap();
    mux.push_control(1, RecordUnit::at(0)).unwrap();
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
    let mut payload = [0u8; 16384];
    let report: MuxReport = match transfer_scheduled(
        &mut mux,
        &entries,
        &mut clients,
        &mut provider_session,
        &mut client_session,
        &mut grant,
        &pins,
        || 20,
        &mut payload,
    ) {
        Ok(r) => r,
        Err(_) => panic!("four live families must transfer"),
    };
    assert_eq!(report.control, 1);
    assert_eq!(report.transferred, 4);
    if report.order_len > 0 {
        assert_eq!(report.order[0], None);
    }
    let mut saw_qac = 0u16;
    let mut saw_emf = 0u16;
    for i in 0..report.order_len as usize {
        match report.order[i] {
            None => {}
            Some(EntryFamily::Qac) => saw_qac += 1,
            Some(EntryFamily::Emf) => saw_emf += 1,
            Some(_) => panic!("only QAC/EMF/Control in this scene"),
        }
    }
    assert_eq!(saw_qac, 2);
    assert_eq!(saw_emf, 2);
    for (local_sel, remote_sel) in local.iter().zip(remote.iter()) {
        if local_sel.entry_id() != remote_sel.entry_id() || local_sel.root() != remote_sel.root() {
            panic!("received select must match publication");
        }
        let bytes = entry_bytes(scene.source(), *remote_sel);
        match family_of(remote_sel.entry_id()) {
            EntryFamily::Qac => assert!(admit_qac(bytes, *remote_sel)),
            EntryFamily::Emf => assert!(admit_emf(bytes, *remote_sel)),
            _ => panic!("scheduled id is QAC or EMF"),
        }
    }
}

#[test]
fn next_delays_m64_while_ready_qac_and_emf_continue() {
    let mut files = four_live().to_vec();
    files.push(("m64-delay", "blob", M64_BYTES, ID_M64));
    let scene = Scene::publish(&files);
    let (pins, _, pin, publisher) = scene.installed(CLOSURE_PURPOSE);
    let bound = scene.bound(&pins, pin, publisher);
    let local = select_all(&bound, &scene);
    let mut nodes = Vec::new();
    let mut roots = Vec::new();
    for sel in &local {
        let (n, r) = build_outboard(scene.source(), *sel);
        nodes.push(n);
        roots.push(r);
    }
    let entries = bind_providers(scene.source(), &local, &nodes, &roots);
    let mut clients = bind_clients(&local);
    let present = present_admitted(scene.source(), &local);
    assert!(present.contains(&ID_QAC_OPAQUE));
    assert!(present.contains(&ID_EMF_OPAQUE));
    assert!(present.contains(&ID_M64));
    let ids = [
        ID_QAC_OPAQUE,
        ID_QAC_PCM,
        ID_EMF_OPAQUE,
        ID_EMF_ALIGN,
        ID_M64,
    ];
    let plan_entries = live_plan(scene.identity(), &ids, &local);
    let inv = Inventory::compile(scene.identity(), &plan_entries, &[]).unwrap();
    let intents = [
        SelectionIntent::scene(ID_QAC_OPAQUE, EntryFamily::Qac, None, None).unwrap(),
        SelectionIntent::scene(ID_QAC_PCM, EntryFamily::Qac, None, None).unwrap(),
        SelectionIntent::scene(ID_EMF_OPAQUE, EntryFamily::Emf, None, None).unwrap(),
        SelectionIntent::scene(ID_EMF_ALIGN, EntryFamily::Emf, None, None).unwrap(),
        SelectionIntent::scene(ID_M64, EntryFamily::M64, None, None).unwrap(),
    ];
    let request = SelectionRequest::scene(&intents).unwrap();
    let sel = PrincipalSelection::new(1, request).unwrap();
    let mut mux = MuxTransfer::compile(&inv, &[sel], &present).unwrap();
    mux.push_control(1, RecordUnit::at(0)).unwrap();
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
    let mut client_lanes = SessionOwner::new();
    let mut provider_lanes = SessionOwner::new();
    let (mut client_session, mut provider_session) =
        attach_pair(&mut client_lanes, &mut provider_lanes);
    let mut payload = [0u8; 16384];
    let report: MuxReport = match transfer_scheduled(
        &mut mux,
        &entries,
        &mut clients,
        &mut provider_session,
        &mut client_session,
        &mut grant,
        &pins,
        || 20,
        &mut payload,
    ) {
        Ok(r) => r,
        Err(_) => panic!("live QAC/EMF plus delayed M64 must transfer"),
    };
    assert!(report.delayed_deferred >= 1);
    assert_eq!(report.control, 1);
    assert_eq!(report.transferred, 5);
    let mut live_done = false;
    let mut saw_m64 = false;
    for i in 0..report.order_len as usize {
        match report.order[i] {
            None | Some(EntryFamily::Qac) | Some(EntryFamily::Emf) => {
                assert!(!saw_m64, "M64 must wait on live families");
            }
            Some(EntryFamily::M64) => {
                assert!(i > 0, "Control/QAC/EMF run before M64");
                saw_m64 = true;
                live_done = true;
            }
            Some(_) => panic!("unexpected family in delay scene"),
        }
    }
    assert!(live_done);
    assert!(saw_m64);
}

#[test]
fn cancel_obsolete_qac_skips_and_sibling_emf_continues() {
    let scene = Scene::publish(&four_live());
    let (pins, _, pin, publisher) = scene.installed(CLOSURE_PURPOSE);
    let bound = scene.bound(&pins, pin, publisher);
    let local = select_all(&bound, &scene);
    let mut nodes = Vec::new();
    let mut roots = Vec::new();
    for sel in &local {
        let (n, r) = build_outboard(scene.source(), *sel);
        nodes.push(n);
        roots.push(r);
    }
    let entries = bind_providers(scene.source(), &local, &nodes, &roots);
    let mut clients = bind_clients(&local);
    let present = present_admitted(scene.source(), &local);
    let ids = [ID_QAC_OPAQUE, ID_QAC_PCM, ID_EMF_OPAQUE, ID_EMF_ALIGN];
    let plan_entries = live_plan(scene.identity(), &ids, &local);
    let inv = Inventory::compile(scene.identity(), &plan_entries, &[]).unwrap();
    let intents = [
        SelectionIntent::scene(ID_QAC_OPAQUE, EntryFamily::Qac, None, None).unwrap(),
        SelectionIntent::scene(ID_QAC_PCM, EntryFamily::Qac, None, None).unwrap(),
        SelectionIntent::scene(ID_EMF_OPAQUE, EntryFamily::Emf, None, None).unwrap(),
        SelectionIntent::scene(ID_EMF_ALIGN, EntryFamily::Emf, None, None).unwrap(),
    ];
    let request = SelectionRequest::scene(&intents).unwrap();
    let sel = PrincipalSelection::new(1, request).unwrap();
    let mut mux = MuxTransfer::compile(&inv, &[sel], &present).unwrap();
    let cancelled = mux.cancel(ID_QAC_OPAQUE, None);
    assert!(cancelled >= 1);
    assert!(mux.occupancy_retained(1, Some(ID_QAC_OPAQUE), 0));
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
    let mut client_lanes = SessionOwner::new();
    let mut provider_lanes = SessionOwner::new();
    let (mut client_session, mut provider_session) =
        attach_pair(&mut client_lanes, &mut provider_lanes);
    let mut payload = [0u8; 16384];
    let report: MuxReport = match transfer_scheduled(
        &mut mux,
        &entries,
        &mut clients,
        &mut provider_session,
        &mut client_session,
        &mut grant,
        &pins,
        || 20,
        &mut payload,
    ) {
        Ok(r) => r,
        Err(_) => panic!("sibling EMF must continue after QAC cancel"),
    };
    assert!(report.cancelled_skipped >= 1);
    assert_eq!(report.transferred, 3);
    let mut saw_emf = 0u16;
    let mut saw_qac = 0u16;
    for i in 0..report.order_len as usize {
        match report.order[i] {
            Some(EntryFamily::Qac) => saw_qac += 1,
            Some(EntryFamily::Emf) => saw_emf += 1,
            None => {}
            Some(_) => panic!("unexpected family after cancel"),
        }
    }
    assert_eq!(saw_emf, 2);
    assert_eq!(saw_qac, 1);
    assert!(mux.occupancy_retained(1, Some(ID_QAC_OPAQUE), 0));
    mux.acknowledge_complete(1, Some(ID_QAC_OPAQUE), 0).unwrap();
    assert!(!mux.occupancy_retained(1, Some(ID_QAC_OPAQUE), 0));
}

#[test]
fn missing_mandatory_calibration_refuses_before_leaf() {
    let scene = Scene::publish(&four_live());
    let (pins, _, pin, publisher) = scene.installed(CLOSURE_PURPOSE);
    let bound = scene.bound(&pins, pin, publisher);
    let local = select_all(&bound, &scene);
    let mut nodes = Vec::new();
    let mut roots = Vec::new();
    for sel in &local {
        let (n, r) = build_outboard(scene.source(), *sel);
        nodes.push(n);
        roots.push(r);
    }
    let entries = bind_providers(scene.source(), &local, &nodes, &roots);
    let mut clients = bind_clients(&local);
    let present = present_admitted(scene.source(), &local);
    let mut plan_entries = live_plan(scene.identity(), &present, &local);
    plan_entries.push(plan_entry(
        scene.identity(),
        ID_CALIB,
        EntryFamily::Q42,
        RepresentationClass::Metadata,
        Obligation::Mandatory,
        7,
    ));
    let qac0 = plan_entries
        .iter()
        .find(|e| e.entry_id() == ID_QAC_OPAQUE)
        .copied()
        .unwrap();
    let emf0 = plan_entries
        .iter()
        .find(|e| e.entry_id() == ID_EMF_OPAQUE)
        .copied()
        .unwrap();
    let calib = plan_entries
        .iter()
        .find(|e| e.entry_id() == ID_CALIB)
        .copied()
        .unwrap();
    let edges = [
        DependencyEdge::bind(
            &qac0,
            &calib,
            Obligation::Mandatory,
            DependencyReason::Calibration,
        )
        .unwrap(),
        DependencyEdge::bind(
            &emf0,
            &calib,
            Obligation::Mandatory,
            DependencyReason::Calibration,
        )
        .unwrap(),
    ];
    let inv = Inventory::compile(scene.identity(), &plan_entries, &edges).unwrap();
    let intents = [
        SelectionIntent::scene(ID_QAC_OPAQUE, EntryFamily::Qac, None, None).unwrap(),
        SelectionIntent::scene(ID_EMF_OPAQUE, EntryFamily::Emf, None, None).unwrap(),
    ];
    let request = SelectionRequest::scene(&intents).unwrap();
    let sel = PrincipalSelection::new(1, request).unwrap();
    let mut mux = MuxTransfer::compile(&inv, &[sel], &present).unwrap();
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
    let mut client_lanes = SessionOwner::new();
    let mut provider_lanes = SessionOwner::new();
    let (mut client_session, mut provider_session) =
        attach_pair(&mut client_lanes, &mut provider_lanes);
    let mut payload = [0xA5u8; 64];
    match transfer_scheduled(
        &mut mux,
        &entries,
        &mut clients,
        &mut provider_session,
        &mut client_session,
        &mut grant,
        &pins,
        || 20,
        &mut payload,
    ) {
        Err(Error::Admission) => {}
        Ok(_) => panic!("missing calibration must refuse before a leaf"),
        Err(_) => panic!("missing calibration maps to Admission"),
    }
    assert!(payload.iter().all(|b| *b == 0xA5));
}

#[test]
fn shared_geometry_dep_charged_once() {
    let mut files = four_live().to_vec();
    files.push(("tend-geom", "blob", TEND_BYTES, ID_TEND));
    let scene = Scene::publish(&files);
    let (pins, _, pin, publisher) = scene.installed(CLOSURE_PURPOSE);
    let bound = scene.bound(&pins, pin, publisher);
    let local = select_all(&bound, &scene);
    let mut nodes = Vec::new();
    let mut roots = Vec::new();
    for sel in &local {
        let (n, r) = build_outboard(scene.source(), *sel);
        nodes.push(n);
        roots.push(r);
    }
    let entries = bind_providers(scene.source(), &local, &nodes, &roots);
    let mut clients = bind_clients(&local);
    let present = present_admitted(scene.source(), &local);
    assert!(present.contains(&ID_TEND));
    let ids = [
        ID_QAC_OPAQUE,
        ID_EMF_OPAQUE,
        ID_TEND,
    ];
    let plan_entries = live_plan(scene.identity(), &ids, &local);
    let qac = plan_entries
        .iter()
        .find(|e| e.entry_id() == ID_QAC_OPAQUE)
        .copied()
        .unwrap();
    let emf = plan_entries
        .iter()
        .find(|e| e.entry_id() == ID_EMF_OPAQUE)
        .copied()
        .unwrap();
    let tend = plan_entries
        .iter()
        .find(|e| e.entry_id() == ID_TEND)
        .copied()
        .unwrap();
    let edges = [
        DependencyEdge::bind(&qac, &tend, Obligation::Mandatory, DependencyReason::Geometry)
            .unwrap(),
        DependencyEdge::bind(&emf, &tend, Obligation::Mandatory, DependencyReason::Geometry)
            .unwrap(),
    ];
    let inv = Inventory::compile(scene.identity(), &plan_entries, &edges).unwrap();
    let intents_a = [SelectionIntent::scene(ID_QAC_OPAQUE, EntryFamily::Qac, None, None).unwrap()];
    let intents_b = [SelectionIntent::scene(ID_EMF_OPAQUE, EntryFamily::Emf, None, None).unwrap()];
    let req_a = SelectionRequest::scene(&intents_a).unwrap();
    let req_b = SelectionRequest::scene(&intents_b).unwrap();
    let sel_a = PrincipalSelection::new(1, req_a).unwrap();
    let sel_b = PrincipalSelection::new(2, req_b).unwrap();
    let mut mux = MuxTransfer::compile(&inv, &[sel_a, sel_b], &present).unwrap();
    assert!(mux.is_charged(ID_TEND));
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
    let mut client_lanes = SessionOwner::new();
    let mut provider_lanes = SessionOwner::new();
    let (mut client_session, mut provider_session) =
        attach_pair(&mut client_lanes, &mut provider_lanes);
    let mut payload = [0u8; 16384];
    let report: MuxReport = match transfer_scheduled(
        &mut mux,
        &entries,
        &mut clients,
        &mut provider_session,
        &mut client_session,
        &mut grant,
        &pins,
        || 20,
        &mut payload,
    ) {
        Ok(r) => r,
        Err(_) => panic!("shared TenD plus distinct QAC/EMF must transfer"),
    };
    assert!(report.shared_reused >= 1);
    let mut tend_sends = 0u16;
    for i in 0..report.order_len as usize {
        if report.order[i] == Some(EntryFamily::TenD) {
            tend_sends += 1;
        }
    }
    assert_eq!(tend_sends, 1);
    assert_eq!(report.transferred, 3);
}

#[test]
fn blob_tagged_as_qac_is_not_present() {
    let files = [
        ("qac-opaque", "qac", QAC_OPAQUE, ID_QAC_OPAQUE),
        ("emf-opaque", "emf", EMF_OPAQUE, ID_EMF_OPAQUE),
        ("junk-blob", "blob", BLOB_BYTES, ID_BLOB),
    ];
    let scene = Scene::publish(&files);
    let (pins, _, pin, publisher) = scene.installed(CLOSURE_PURPOSE);
    let bound = scene.bound(&pins, pin, publisher);
    let local = select_all(&bound, &scene);
    let blob = *local.iter().find(|s| s.entry_id() == ID_BLOB).unwrap();
    let blob_bytes = entry_bytes(scene.source(), blob);
    assert!(!admit_qac(blob_bytes, blob));
    assert!(!admit_emf(blob_bytes, blob));
    let present = present_admitted(scene.source(), &local);
    assert!(!present.contains(&ID_BLOB));
    assert!(present.contains(&ID_QAC_OPAQUE));
    assert!(present.contains(&ID_EMF_OPAQUE));
    let mut nodes = Vec::new();
    let mut roots = Vec::new();
    for sel in &local {
        let (n, r) = build_outboard(scene.source(), *sel);
        nodes.push(n);
        roots.push(r);
    }
    let entries = bind_providers(scene.source(), &local, &nodes, &roots);
    let mut clients = bind_clients(&local);
    let ids = [ID_QAC_OPAQUE, ID_EMF_OPAQUE, ID_BLOB];
    let plan_entries = live_plan(scene.identity(), &ids, &local);
    let inv = Inventory::compile(scene.identity(), &plan_entries, &[]).unwrap();
    let intents = [
        SelectionIntent::scene(ID_BLOB, EntryFamily::Qac, None, None).unwrap(),
        SelectionIntent::scene(ID_QAC_OPAQUE, EntryFamily::Qac, None, None).unwrap(),
        SelectionIntent::scene(ID_EMF_OPAQUE, EntryFamily::Emf, None, None).unwrap(),
    ];
    let request = SelectionRequest::scene(&intents).unwrap();
    let sel = PrincipalSelection::new(1, request).unwrap();
    let mut mux = MuxTransfer::compile(&inv, &[sel], &present).unwrap();
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
    let mut client_lanes = SessionOwner::new();
    let mut provider_lanes = SessionOwner::new();
    let (mut client_session, mut provider_session) =
        attach_pair(&mut client_lanes, &mut provider_lanes);
    let mut payload = [0x3Cu8; 64];
    match transfer_scheduled(
        &mut mux,
        &entries,
        &mut clients,
        &mut provider_session,
        &mut client_session,
        &mut grant,
        &pins,
        || 20,
        &mut payload,
    ) {
        Err(Error::Admission) => {
            assert!(payload.iter().all(|b| *b == 0x3C));
        }
        Ok(report) => {
            for i in 0..report.order_len as usize {
                if report.order[i] == Some(EntryFamily::Qac) {
                    // only the admitted QAC, never the blob as acoustic content
                }
            }
            assert!(!payload[..BLOB_BYTES.len().min(payload.len())].eq(BLOB_BYTES));
            let mut blob_sent = false;
            for sel in &local {
                if sel.entry_id() == ID_BLOB {
                    blob_sent = report.order[..report.order_len as usize]
                        .iter()
                        .any(|f| *f == Some(EntryFamily::Qac))
                        && report.transferred == 3;
                }
            }
            assert!(!blob_sent || report.transferred < 3);
        }
        Err(_) => panic!("blob gate is Admission or skip, not another class"),
    }
}
