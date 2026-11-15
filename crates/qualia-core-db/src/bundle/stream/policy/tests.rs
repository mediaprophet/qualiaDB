use super::*;
use crate::bundle::stream::{
    descriptor::*,
    publish::{EntryAssignment, PlainPublisher, PublicationClass, PublisherOptions},
};
use crate::bundle::{BundleReadLimits, BundleWriter};
use crate::crypto::network::{
    authority::*,
    cose,
    digest::sha384,
    types::{KeyEpoch, KeyPurpose},
};
use crate::net::qdnf::{
    authority::AuthorityOwner,
    errors::QdnfError,
    types::{OperationId, StrongDigest},
};
use crate::semantic_instruments::resolve::{bounded_closure::*, DependencyKind};
use ed25519_dalek::SigningKey;
use fips204::{
    ml_dsa_65,
    traits::{KeyGen, SerDes},
};

pub(crate) struct Fixture {
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
impl Fixture {
    pub(crate) fn source(&self) -> &[u8] {
        &self.source
    }
    pub(crate) fn body(&self) -> &[u8] {
        &self.body
    }
    pub(crate) fn entry_page(&self) -> &[u8] {
        &self.entry_page
    }
    pub(crate) fn identity(&self) -> CandidateIdentity {
        self.identity
    }
    pub(crate) fn new() -> Self {
        Self::new_with_payload(b"actual-source-payload")
    }
    pub(crate) fn new_with_payload(payload: &[u8]) -> Self {
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
        writer
            .add_file("payload", "blob", payload.to_vec(), None)
            .unwrap();
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
        let mut context_refs = [PageRef::EMPTY; 32];
        let mut entry_refs = [PageRef::EMPTY; 32];
        let assignments = [EntryAssignment {
            id: 7,
            inner_version: 1,
        }];
        let mut index = [crate::bundle::stream::index::EntryScratch::EMPTY; 1];
        let mut ids = [0u32; 1];
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
    pub(crate) fn installed(
        &self,
        purpose: &[u8],
    ) -> (PinAuthority, LocalBootstrap, PinHandle, PinHandle) {
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
    fn signed(&self, manifest: &[u8], purpose: &[u8]) -> Vec<u8> {
        self.signed_digest(sha384(manifest), purpose)
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
    ) -> Result<AuthenticatedClosure<'a>, PolicyError> {
        let candidate =
            verify_closure(self.fields.semantic_root, &self.nodes, &self.edges).unwrap();
        let signed = self.signed(&self.manifest, CLOSURE_PURPOSE);
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
    }
    pub(crate) fn bound<'a>(
        &'a self,
        pins: &PinAuthority,
        pin: PinHandle,
        publisher: PinHandle,
    ) -> BoundClosure<'a> {
        self.authenticated(pins, pin, 10)
            .unwrap()
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
    pub(crate) fn request(&self) -> LocalGrantRequest {
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

#[test]
fn canonical_manifest_roundtrip_permutation_truncation_and_metadata_budget() {
    let f = Fixture::new();
    let view = decode_manifest(&f.manifest).unwrap();
    assert_eq!(view.fields(), f.fields);
    assert_eq!(view.digest(), sha384(&f.manifest));
    for end in 0..f.manifest.len() {
        assert!(decode_manifest(&f.manifest[..end]).is_err());
    }
    let mut trailing = f.manifest.clone();
    trailing.push(0);
    assert!(decode_manifest(&trailing).is_err());
    let nodes = [f.nodes[2], f.nodes[0], f.nodes[1]];
    let edges = [f.edges[1], f.edges[0]];
    let candidate = verify_closure(f.fields.semantic_root, &nodes, &edges).unwrap();
    let mut out = [0; MAX_MANIFEST_BYTES];
    let n = encode_manifest_into(f.fields, &candidate, &mut out).unwrap();
    assert_eq!(&out[..n], f.manifest);
    let mut fields = f.fields;
    fields.policy = sha384(b"missing");
    assert_eq!(
        encode_manifest_into(fields, &candidate, &mut out),
        Err(PolicyError::Reference)
    );
    let large = vec![7; MAX_CLOSURE_ORIGINAL_BYTES];
    let nodes = [ClosureNode {
        digest: sha384(&large),
        kind: DependencyKind::Instrument,
        original_bytes: &large,
    }];
    let candidate = verify_closure(nodes[0].digest, &nodes, &[]).unwrap();
    fields.semantic_root = nodes[0].digest;
    fields.policy = nodes[0].digest;
    fields.consumer = nodes[0].digest;
    assert_eq!(
        encode_manifest_into(fields, &candidate, &mut out),
        Err(PolicyError::Limit)
    );
}
#[test]
fn real_paired_pin_and_descriptor_binding_then_one_shot_local_owner_use() {
    let f = Fixture::new();
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let bound = f.bound(&pins, pin, publisher);
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, f.request()).unwrap();
    assert_eq!(grant.binding().instrument, sha384(&f.manifest));
    let mut effects = 0;
    grant
        .accept_local(
            &pins,
            || 20,
            |_| {
                effects += 1;
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(
        grant.accept_local(
            &pins,
            || 20,
            |_| {
                effects += 1;
                Ok(())
            }
        ),
        Err(PolicyError::Spent)
    );
    assert_eq!(effects, 1);
}
#[test]
fn wrong_pin_scope_owner_expiry_revocation_and_manifest_mutation_refuse() {
    let f = Fixture::new();
    let (pins, _, pin, _publisher) = f.installed(b"publication");
    assert!(f.authenticated(&pins, pin, 10).is_err());
    let (mut pins, admin, pin, _publisher) = f.installed(CLOSURE_PURPOSE);
    assert!(f.authenticated(&pins, pin, 9).is_err());
    assert!(f.authenticated(&pins, pin, 100).is_err());
    let (other, _, _, _) = f.installed(CLOSURE_PURPOSE);
    assert!(f.authenticated(&other, pin, 10).is_err());
    pins.revoke_pin(&admin, pin).unwrap();
    assert!(f.authenticated(&pins, pin, 10).is_err());
    let mut changed = f.manifest.clone();
    changed[0] ^= 1;
    assert!(decode_manifest(&changed).is_err());
    // Changing a declared artifact kind remains canonical but mismatches originals' model.
    let row = super::types::MAGIC.len() + 48 * 5 + 8 + 4;
    changed = f.manifest.clone();
    changed[row + 48] = if changed[row + 48] == 1 { 2 } else { 1 };
    let candidate = verify_closure(f.fields.semantic_root, &f.nodes, &f.edges).unwrap();
    assert_eq!(
        decode_manifest(&changed).unwrap().matches(&candidate),
        Err(PolicyError::Graph)
    );
}
#[test]
fn cancellation_expiry_and_unwind_never_repeat_local_acceptance() {
    let f = Fixture::new();
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let bound = f.bound(&pins, pin, publisher);
    let mut owner = AuthorityOwner::new();
    let mut effects = 0;
    {
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, f.request()).unwrap();
        grant.cancel().unwrap();
        assert_eq!(
            grant.accept_local(
                &pins,
                || 20,
                |_| {
                    effects += 1;
                    Ok(())
                }
            ),
            Err(PolicyError::Spent)
        );
    }
    {
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, f.request()).unwrap();
        let ticks = core::cell::Cell::new(20);
        assert_eq!(
            grant.accept_local(
                &pins,
                || {
                    let t = ticks.get();
                    ticks.set(90);
                    t
                },
                |_| {
                    effects += 1;
                    Ok(())
                }
            ),
            Err(PolicyError::Authority(QdnfError::Expired))
        );
    }
    {
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, f.request()).unwrap();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            grant.accept_local::<()>(&pins, || 20, |_| panic!("effect failed"))
        }));
        assert!(panic.is_err());
        assert_eq!(
            grant.accept_local(&pins, || 20, |_| Ok(())),
            Err(PolicyError::Spent)
        );
    }
    assert_eq!(effects, 0);
}
#[path = "rejection_tests.rs"]
mod rejection;

#[path = "scoped_tests.rs"]
mod scoped;
