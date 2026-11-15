//! Mixed-HMC native mux over loopback `WssBearer`.
//!
//! `HttpMux` is HTTP-only. This module attaches `Session` to `WssBearer<TcpStream>`
//! and drives `MuxTransfer` / `transfer_scheduled`. JSON is not an application
//! object. Kind 1–12 stay frozen. Profile stays `TlsWssTransitionV1`.
//!
//! Integrator adds `mod wss_mixed;` in `http/mod.rs`.

#[cfg(all(test, not(target_arch = "wasm32")))]
mod wss_mixed_tests {
    //! EvidenceClass::Component (`net/qdnf/harness/qualification.rs`).
    //! Never Process. Never PhysicalNetwork.
    use crate::audio::qac::{QacStorageOptions, QacStorageView};
    use crate::bundle::stream::{
        descriptor::{
            CandidateIdentity, ContextParameters, EntryRecord, PageRef, HASH_SCRATCH_BYTES,
            MAX_ENCODED, MAX_METADATA,
        },
        formats::{
            emf_present_id, qac_present_id, AuthenticatedRange, EmfStreamView,
            Q42AuthenticatedRangeSource, QacStreamView,
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
        proof::{
            verify_leaf_span, OutboardNode, SpanLimits, TreeBuilder, VerifiedLeafSpan, LEAF_BYTES,
            MAX_SIBLINGS,
        },
        publish::{EntryAssignment, PlainPublisher, PublicationClass, PublisherOptions},
        selection::{select_entry, SelectedPlainEntry},
    };
    use crate::bundle::{BundleReadLimits, BundleWriter};
    use crate::crypto::network::{
        authority::{LocalBootstrap, PinAuthority, PinConfig, PinHandle},
        cose,
        digest::sha384,
        types::{KeyEpoch, KeyPurpose},
    };
    use crate::mini_parser::hash_token;
    use crate::net::peer::replication::qbdl::provider::{OutboardCatalog, OutboardLimits};
    use crate::net::peer::replication::qbdl::service::{
        transfer_scheduled, BoundEntry, Error, MuxReport, MuxTransfer, PlainClient, PlainProvider,
        ServiceIo, Session, SessionOwner, ROLE_CLIENT, ROLE_PROVIDER,
    };
    use crate::net::peer::replication::qbdl::wire::Header;
    use crate::net::qdnf::authority::AuthorityOwner;
    use crate::net::qdnf::registries::BearerProfile;
    use crate::net::qdnf::types::{ObservedLocator, OperationId, ScopeEpoch, StrongDigest};
    use crate::p2p::connectivity::wss_bearer::WssBearer;
    use crate::q42::field_envelope::{Family, Limits, StructuralView};
    use crate::q42::q42_volume::write_unified_volume;
    use crate::semantic_instruments::resolve::{bounded_closure::*, DependencyKind};
    use crate::NQuin;
    use ed25519_dalek::SigningKey;
    use fips204::{
        ml_dsa_65,
        traits::{KeyGen, SerDes},
    };
    use std::collections::HashMap;
    use std::io::Read;
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;
    use tempfile::NamedTempFile;

    const TRANSCRIPT: StrongDigest = StrongDigest([9; 48]);
    const POLICY: u64 = 1;

    const QAC_OPAQUE: &[u8] =
        include_bytes!("../../../q42/field_envelope/fixtures/candidate/qac_candidate-opaque.qac");
    const QAC_PCM: &[u8] = include_bytes!(
        "../../../q42/field_envelope/fixtures/candidate/qac_candidate-pcm-signed-zero.qac"
    );
    const EMF_OPAQUE: &[u8] =
        include_bytes!("../../../q42/field_envelope/fixtures/candidate/emf_candidate-opaque.emf");
    const EMF_ALIGN: &[u8] = include_bytes!(
        "../../../q42/field_envelope/fixtures/candidate/emf_candidate-aligned4096.emf"
    );
    const M64_BYTES: &[u8] = b"m64-delay-payload-not-a-field";

    const ID_QAC_OPAQUE: u32 = 10;
    const ID_QAC_PCM: u32 = 11;
    const ID_EMF_OPAQUE: u32 = 20;
    const ID_EMF_ALIGN: u32 = 21;
    const ID_M64: u32 = 30;
    const ID_Q42: u32 = 40;
    const ID_CALIB: u32 = 60;

    /// Copies LeafData payloads after Session unframing. Not a second codec.
    struct LeafCapture<S> {
        inner: S,
        bodies: Vec<(u32, Vec<u8>)>,
    }

    impl<S: ServiceIo> ServiceIo for LeafCapture<S> {
        fn send_message(&mut self, header: &Header, body: &[u8]) -> Result<(), Error> {
            self.inner.send_message(header, body)
        }
        fn recv_control(&mut self) -> Result<(Header, [u8; 80], usize), Error> {
            self.inner.recv_control()
        }
        fn recv_describe(&mut self) -> Result<(Header, [u8; 96], usize), Error> {
            self.inner.recv_describe()
        }
        fn recv_descriptor(&mut self, page: &mut [u8]) -> Result<(Header, usize), Error> {
            self.inner.recv_descriptor(page)
        }
        fn recv_leaf(
            &mut self,
            payload: &mut [u8],
            proof: &mut [u8],
        ) -> Result<(Header, usize, usize), Error> {
            let (header, p, q) = self.inner.recv_leaf(payload, proof)?;
            self.bodies.push((header.entry, payload[..p].to_vec()));
            Ok((header, p, q))
        }
        fn fail_close(&mut self, e: Error) -> Error {
            self.inner.fail_close(e)
        }
        fn closed(&self) -> bool {
            self.inner.closed()
        }
        fn next_transfer_id(&mut self) -> Result<[u8; 16], Error> {
            self.inner.next_transfer_id()
        }
        fn next_credit_serial(&mut self) -> Result<u64, Error> {
            self.inner.next_credit_serial()
        }
    }

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
            let n = cose::sign_pair_raw(&binding, &keys, &mut cose::Workspace::new(), &mut out)
                .unwrap();
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

    fn loc(b: u8) -> ObservedLocator {
        ObservedLocator::from_slice(&[b]).unwrap()
    }

    fn q42_volume_bytes() -> Vec<u8> {
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
        let file = NamedTempFile::new().unwrap();
        write_unified_volume(file.path(), &lexicon, &[(object, object)], &[vec![quin]]).unwrap();
        let mut q42 = Vec::new();
        std::fs::File::open(file.path())
            .unwrap()
            .read_to_end(&mut q42)
            .unwrap();
        assert!(!q42.is_empty());
        assert!(q42.len() as u64 <= LEAF_BYTES);
        q42
    }

    fn family_of(id: u32) -> EntryFamily {
        match id {
            ID_QAC_OPAQUE | ID_QAC_PCM => EntryFamily::Qac,
            ID_EMF_OPAQUE | ID_EMF_ALIGN => EntryFamily::Emf,
            ID_M64 => EntryFamily::M64,
            ID_Q42 | ID_CALIB => EntryFamily::Q42,
            _ => EntryFamily::Opaque,
        }
    }

    fn entry_bytes<'a>(source: &'a [u8], selected: SelectedPlainEntry<'_>) -> &'a [u8] {
        let start = usize::try_from(selected.offset()).unwrap();
        let len = usize::try_from(selected.logical_length()).unwrap();
        &source[start..start + len]
    }

    fn whole_span<'a>(
        selected: SelectedPlainEntry<'a>,
        bytes: &'a [u8],
        siblings: &'a mut [StrongDigest],
    ) -> Result<VerifiedLeafSpan<'a>, Error> {
        if selected.logical_length() != bytes.len() as u64 {
            return Err(Error::Identity);
        }
        let spec = selected.tree_spec().map_err(|_| Error::Identity)?;
        verify_leaf_span(
            &spec,
            selected.root(),
            0,
            bytes,
            &[&[0]],
            siblings,
            SpanLimits {
                max_leaves: 1,
                max_encoded_bytes: LEAF_BYTES as usize,
            },
        )
        .map_err(|_| Error::Proof)
    }

    fn entry_admits_family(
        source: &[u8],
        selected: SelectedPlainEntry<'_>,
        family: EntryFamily,
    ) -> bool {
        let bytes = entry_bytes(source, selected);
        if selected.logical_length() != bytes.len() as u64 {
            return false;
        }
        match family {
            EntryFamily::Qac => {
                let mut siblings = [StrongDigest::ZERO; MAX_SIBLINGS];
                if let Ok(span) = whole_span(selected, bytes, &mut siblings) {
                    if let Ok(range) = AuthenticatedRange::new(selected, &span, 0, bytes.len()) {
                        if let Ok(view) = QacStreamView::admit(range, QacStorageOptions::default()) {
                            return qac_present_id(selected.entry_id(), &view).is_some();
                        }
                    }
                }
                QacStorageView::open(bytes, QacStorageOptions::default()).is_ok()
            }
            EntryFamily::Emf => {
                let mut siblings = [StrongDigest::ZERO; MAX_SIBLINGS];
                if let Ok(span) = whole_span(selected, bytes, &mut siblings) {
                    if let Ok(range) = AuthenticatedRange::new(selected, &span, 0, bytes.len()) {
                        if let Ok(view) = EmfStreamView::admit(range, Limits::default()) {
                            return emf_present_id(selected.entry_id(), &view).is_some();
                        }
                    }
                }
                StructuralView::open(bytes, Family::EmfV4Candidate, Limits::default()).is_ok()
            }
            EntryFamily::Q42 => {
                let mut siblings = [StrongDigest::ZERO; MAX_SIBLINGS];
                let Ok(span) = whole_span(selected, bytes, &mut siblings) else {
                    return false;
                };
                let Ok(range) = AuthenticatedRange::new(selected, &span, 0, bytes.len()) else {
                    return false;
                };
                Q42AuthenticatedRangeSource::new(selected, &[range], bytes.len() as u64, 1).is_ok()
            }
            EntryFamily::Opaque => false,
            EntryFamily::TenD | EntryFamily::M64 | EntryFamily::P64 => true,
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

    fn select_all<'p>(bound: &BoundClosure<'_>, scene: &'p Scene) -> Vec<SelectedPlainEntry<'p>> {
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

    fn build_outboard(
        source: &[u8],
        selected: SelectedPlainEntry<'_>,
    ) -> (Vec<OutboardNode>, StrongDigest) {
        let entry = entry_bytes(source, selected);
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
            if family == EntryFamily::Qac
                || family == EntryFamily::Emf
                || family == EntryFamily::Q42
            {
                if entry_admits_family(source, *sel, family) {
                    present.push(sel.entry_id());
                }
            } else if family != EntryFamily::Opaque {
                present.push(sel.entry_id());
            }
        }
        present
    }

    fn live_plan(
        candidate: CandidateIdentity,
        ids: &[u32],
        selected: &[SelectedPlainEntry<'_>],
    ) -> Vec<PlanEntry> {
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

    fn selected_by_id<'p>(
        selected: &[SelectedPlainEntry<'p>],
        id: u32,
    ) -> SelectedPlainEntry<'p> {
        *selected.iter().find(|s| s.entry_id() == id).unwrap()
    }

    fn attach_wss_pair<'a>(
        client_lanes: &'a mut SessionOwner,
        provider_lanes: &'a mut SessionOwner,
    ) -> (
        Session<'a, WssBearer<TcpStream>>,
        Session<'a, WssBearer<TcpStream>>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let client_tcp = TcpStream::connect(addr).unwrap();
        let (server_tcp, _) = listener.accept().unwrap();
        client_tcp.set_nodelay(true).ok();
        server_tcp.set_nodelay(true).ok();
        let timeout = Some(Duration::from_secs(8));
        client_tcp.set_read_timeout(timeout).ok();
        server_tcp.set_read_timeout(timeout).ok();
        client_tcp.set_write_timeout(timeout).ok();
        server_tcp.set_write_timeout(timeout).ok();
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let dest_a = loc(0x01);
        let dest_b = loc(0x02);
        let client_bearer = WssBearer::new(client_tcp, dest_a, dest_b, scope, true);
        let provider_bearer = WssBearer::new(server_tcp, dest_b, dest_a, scope, false);
        let client_session = match Session::attach(
            client_bearer,
            dest_b,
            TRANSCRIPT,
            ROLE_CLIENT,
            client_lanes,
        ) {
            Ok(s) => s,
            Err(_) => panic!("client Session::attach over WssBearer"),
        };
        let provider_session = match Session::attach(
            provider_bearer,
            dest_a,
            TRANSCRIPT,
            ROLE_PROVIDER,
            provider_lanes,
        ) {
            Ok(s) => s,
            Err(_) => panic!("provider Session::attach over WssBearer"),
        };
        if client_session.profile() != BearerProfile::TlsWssTransitionV1
            || provider_session.profile() != BearerProfile::TlsWssTransitionV1
        {
            panic!("WssBearer stays TlsWssTransitionV1");
        }
        (client_session, provider_session)
    }

    #[test]
    fn wss_two_qac_and_two_emf_match_native_bytes() {
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
        let mut clients = bind_clients(&local);
        let present = present_admitted(scene.source(), &local);
        assert_eq!(present.len(), 4);
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
        let mut mux = match MuxTransfer::compile(&inv, &[sel], &present) {
            Ok(m) => m,
            Err(_) => panic!("four live families must compile"),
        };
        mux.push_control(1, RecordUnit::at(0)).unwrap();
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
        let mut client_lanes = SessionOwner::new();
        let mut provider_lanes = SessionOwner::new();
        let (client_session, mut provider_session) =
            attach_wss_pair(&mut client_lanes, &mut provider_lanes);
        let mut client_session = LeafCapture {
            inner: client_session,
            bodies: Vec::new(),
        };
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
            Err(_) => panic!("four live families must transfer over WssBearer"),
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
        assert_eq!(client_session.bodies.len(), 4);
        let mut ids_seen = [0u32; 4];
        let mut roots_seen = [StrongDigest::ZERO; 4];
        for (i, (id, body)) in client_session.bodies.iter().enumerate() {
            ids_seen[i] = *id;
            let sel = selected_by_id(&local, *id);
            roots_seen[i] = sel.root();
            let native = entry_bytes(scene.source(), sel);
            assert_eq!(body.as_slice(), native);
            let family = family_of(*id);
            assert!(entry_admits_family(scene.source(), sel, family));
            match family {
                EntryFamily::Qac => {
                    let mut siblings = [StrongDigest::ZERO; MAX_SIBLINGS];
                    let span = match whole_span(sel, native, &mut siblings) {
                        Ok(s) => s,
                        Err(_) => panic!("qac span after WSS"),
                    };
                    let range = match AuthenticatedRange::new(sel, &span, 0, native.len()) {
                        Ok(r) => r,
                        Err(_) => panic!("qac range after WSS"),
                    };
                    match QacStreamView::admit(range, QacStorageOptions::default()) {
                        Ok(_) => {}
                        Err(_) => panic!("qac must admit after WSS"),
                    }
                }
                EntryFamily::Emf => {
                    let mut siblings = [StrongDigest::ZERO; MAX_SIBLINGS];
                    let span = match whole_span(sel, native, &mut siblings) {
                        Ok(s) => s,
                        Err(_) => panic!("emf span after WSS"),
                    };
                    let range = match AuthenticatedRange::new(sel, &span, 0, native.len()) {
                        Ok(r) => r,
                        Err(_) => panic!("emf range after WSS"),
                    };
                    match EmfStreamView::admit(range, Limits::default()) {
                        Ok(_) => {}
                        Err(_) => panic!("emf must admit after WSS"),
                    }
                }
                _ => panic!("scheduled id is QAC or EMF"),
            }
        }
        for i in 0..4 {
            for j in 0..i {
                assert_ne!(ids_seen[i], ids_seen[j]);
                assert_ne!(roots_seen[i], roots_seen[j]);
            }
        }
    }

    #[test]
    fn wss_delays_m64_while_ready_qac_emf_continue() {
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
        let mut mux = match MuxTransfer::compile(&inv, &[sel], &present) {
            Ok(m) => m,
            Err(_) => panic!("live QAC/EMF plus delayed M64 must compile"),
        };
        mux.push_control(1, RecordUnit::at(0)).unwrap();
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
        let mut client_lanes = SessionOwner::new();
        let mut provider_lanes = SessionOwner::new();
        let (mut client_session, mut provider_session) =
            attach_wss_pair(&mut client_lanes, &mut provider_lanes);
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
            Err(_) => panic!("live QAC/EMF plus delayed M64 must transfer over WssBearer"),
        };
        assert!(report.delayed_deferred >= 1);
        assert_eq!(report.control, 1);
        assert_eq!(report.transferred, 5);
        let mut saw_m64 = false;
        for i in 0..report.order_len as usize {
            match report.order[i] {
                None | Some(EntryFamily::Qac) | Some(EntryFamily::Emf) => {
                    assert!(!saw_m64, "M64 must wait on live families");
                }
                Some(EntryFamily::M64) => {
                    assert!(i > 0, "Control/QAC/EMF run before M64");
                    saw_m64 = true;
                }
                Some(_) => panic!("unexpected family in delay scene"),
            }
        }
        assert!(saw_m64);
    }

    #[test]
    fn wss_q42_entry_bytes_match_native() {
        let q42 = q42_volume_bytes();
        let mut files = four_live().to_vec();
        files.push(("q42-query", "q42", q42.as_slice(), ID_Q42));
        let scene = Scene::publish(&files);
        let (pins, _, pin, publisher) = scene.installed(CLOSURE_PURPOSE);
        let bound = scene.bound(&pins, pin, publisher);
        let local = select_all(&bound, &scene);
        let selected = selected_by_id(&local, ID_Q42);
        let native = entry_bytes(scene.source(), selected);
        assert_eq!(native, q42.as_slice());
        assert!(entry_admits_family(scene.source(), selected, EntryFamily::Q42));
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
        assert!(present.contains(&ID_Q42));
        let ids = [
            ID_QAC_OPAQUE,
            ID_QAC_PCM,
            ID_EMF_OPAQUE,
            ID_EMF_ALIGN,
            ID_Q42,
        ];
        let plan_entries = live_plan(scene.identity(), &ids, &local);
        let inv = Inventory::compile(scene.identity(), &plan_entries, &[]).unwrap();
        let intents = [
            SelectionIntent::scene(ID_QAC_OPAQUE, EntryFamily::Qac, None, None).unwrap(),
            SelectionIntent::scene(ID_QAC_PCM, EntryFamily::Qac, None, None).unwrap(),
            SelectionIntent::scene(ID_EMF_OPAQUE, EntryFamily::Emf, None, None).unwrap(),
            SelectionIntent::scene(ID_EMF_ALIGN, EntryFamily::Emf, None, None).unwrap(),
            SelectionIntent::scene(ID_Q42, EntryFamily::Q42, None, None).unwrap(),
        ];
        let request = SelectionRequest::scene(&intents).unwrap();
        let sel = PrincipalSelection::new(1, request).unwrap();
        let mut mux = match MuxTransfer::compile(&inv, &[sel], &present) {
            Ok(m) => m,
            Err(_) => panic!("q42 mixed scene must compile"),
        };
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
        let mut client_lanes = SessionOwner::new();
        let mut provider_lanes = SessionOwner::new();
        let (client_session, mut provider_session) =
            attach_wss_pair(&mut client_lanes, &mut provider_lanes);
        let mut client_session = LeafCapture {
            inner: client_session,
            bodies: Vec::new(),
        };
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
            Err(_) => panic!("q42 whole-entry must transfer over WssBearer"),
        };
        assert_eq!(report.transferred, 5);
        let mut q42_body = None;
        for (id, body) in &client_session.bodies {
            let sel = selected_by_id(&local, *id);
            let native = entry_bytes(scene.source(), sel);
            assert_eq!(body.as_slice(), native);
            if *id == ID_Q42 {
                q42_body = Some(body.as_slice());
            }
        }
        match q42_body {
            Some(body) => assert_eq!(body, q42.as_slice()),
            None => panic!("q42 leaf must arrive over WSS Session framing"),
        }
    }

    #[test]
    fn wss_incomplete_mandatory_is_not_a_complete_answer() {
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
        let mut mux = match MuxTransfer::compile(&inv, &[sel], &present) {
            Ok(m) => m,
            Err(_) => panic!("incomplete scene still compiles; transfer must refuse"),
        };
        let mut owner = AuthorityOwner::new();
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, scene.request()).unwrap();
        let mut client_lanes = SessionOwner::new();
        let mut provider_lanes = SessionOwner::new();
        let (mut client_session, mut provider_session) =
            attach_wss_pair(&mut client_lanes, &mut provider_lanes);
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
            Ok(_) => panic!("missing mandatory must not disclose a complete WSS answer"),
            Err(_) => panic!("missing mandatory maps to Admission"),
        }
        assert!(payload.iter().all(|b| *b == 0xA5));
    }
}
