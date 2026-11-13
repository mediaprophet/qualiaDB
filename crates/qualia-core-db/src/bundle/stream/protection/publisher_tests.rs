use super::{geometry::*, ids::*, metadata::*, publisher::*, *};
use crate::bundle::stream::{index::EntryScratch, proof::*};
use crate::bundle::{BundleReadLimits, BundleWriter};
use crate::crypto::network::{
    cose,
    digest::sha384,
    key_provider::ProviderGrant,
    object_stream::*,
    types::{KeyEpoch, KeyPurpose},
};
use crate::net::qdnf::types::StrongDigest as D;
use crate::specialized_libs::cryptographic_library::{
    AccessPolicy, AuthenticationMethod, KeyAlgorithm, KeyManager, KeyOperation, KeyType,
    ManagedOperationControl, PairedSigningRequest, PublicationRequest, PublicationWorkspace,
    SecurityLevel, SigningAccess, TimeRestrictions,
};
use std::cell::Cell;
fn access() -> SigningAccess<'static> {
    SigningAccess {
        authenticated_methods: &[AuthenticationMethod::HardwareToken],
        ip_address: "local",
    }
}
fn policy(id: &str, operations: Vec<KeyOperation>) -> AccessPolicy {
    AccessPolicy {
        policy_id: id.into(),
        key_id: id.into(),
        allowed_operations: operations,
        required_auth: vec![AuthenticationMethod::HardwareToken],
        time_restrictions: TimeRestrictions {
            allowed_hours: vec![],
            allowed_days: vec![],
            start_date: None,
            end_date: None,
        },
        ip_restrictions: vec!["local".into()],
    }
}
fn grant(purpose: KeyPurpose) -> ProviderGrant {
    ProviderGrant {
        epoch: KeyEpoch {
            purpose,
            epoch: 1,
            controller: 7,
        },
        cell_epoch: 1,
        authority_generation: 1,
        deadline_unix: 2_000_000_100,
        revoked: false,
    }
}
fn refs() -> PrivateReferences {
    PrivateReferences {
        closure: D([7; 48]),
        policy: D([8; 48]),
        consumer: D([9; 48]),
    }
}
fn bytes() -> Vec<u8> {
    let mut w = BundleWriter::new();
    for (i, n) in [0, 1, 17000].into_iter().enumerate() {
        w.add_file(
            format!("private-source-entry-name-{i}"),
            "blob",
            vec![13; n],
            None,
        )
        .unwrap();
    }
    w.build().unwrap()
}
struct PublicPair {
    ml: Vec<u8>,
    ed: Vec<u8>,
    ids: cose::KeyIds,
}
fn fixture(
    signing: bool,
    f: impl FnOnce(
        &mut ProtectedPublisher<'_, '_, '_, '_>,
        &ManagedOperationControl,
        &Cell<u64>,
        Option<&PublicPair>,
    ),
) {
    let bytes = bytes();
    let mut idx = [EntryScratch::EMPTY; 3];
    let mut entries = [EntryRange::EMPTY; 3];
    let mut objects = [SourceObject::EMPTY; 7];
    let mut order = [0; 7];
    let source = PreparedSource::prepare(
        &bytes,
        BundleReadLimits::default(),
        &mut idx,
        &mut entries,
        &mut objects,
        &mut order,
    )
    .unwrap();
    let mut mappings = [Mapping::EMPTY; 7];
    let mut gr = [EMPTY_REF; 1];
    let mut geo = [0; 1560];
    let geometry = PreparedGeometry::prepare(source, &mut mappings, &mut gr, &mut geo).unwrap();
    let mut manager = KeyManager::new();
    manager.initialize().unwrap();
    manager.install_managed_signing_policy(policy(
        "publish",
        vec![KeyOperation::Write, KeyOperation::Encrypt],
    ));
    let public = if signing {
        let (ml, mp) = manager
            .generate_key_pair(
                "ml".into(),
                KeyType::Private,
                KeyAlgorithm::MLDSA,
                SecurityLevel::High,
            )
            .unwrap();
        let (ed, ep) = manager
            .generate_key_pair(
                "ed".into(),
                KeyType::Private,
                KeyAlgorithm::EdDSA,
                SecurityLevel::High,
            )
            .unwrap();
        manager.store_key(ml).unwrap();
        manager.store_key(ed).unwrap();
        manager.install_managed_signing_policy(policy("ml_private", vec![KeyOperation::Sign]));
        manager.install_managed_signing_policy(policy("ed_private", vec![KeyOperation::Sign]));
        let ids = cose::PairedPublicKeys {
            ml_dsa_65: mp.key_data.as_slice().try_into().unwrap(),
            ed25519: ep.key_data.as_slice().try_into().unwrap(),
        }
        .key_ids();
        Some(PublicPair {
            ml: mp.key_data,
            ed: ep.key_data,
            ids,
        })
    } else {
        None
    };
    let control = ManagedOperationControl::new(1).unwrap();
    let time = Cell::new(2_000_000_000);
    let clock = || time.get();
    let mut workspace = PublicationWorkspace::new();
    let request = PublicationRequest {
        policy_id: "publish",
        policy_key_id: "publish",
        grant: grant(KeyPurpose::ObjectContent),
        access: access(),
        generation: 0,
        predecessor: D::ZERO,
        objects: geometry.object_count() as u32,
        metadata_length: geometry.metadata_length() as u64,
        workspace_budget: PublicationWorkspace::required_bytes(),
    };
    let publication = manager
        .begin_object_publication(request, &mut workspace, &control, &clock)
        .unwrap();
    let mut context = [0; MAX_CONTEXT];
    let n = geometry
        .encode_context(publication.context_fields(), &mut context)
        .unwrap();
    let context = Context::parse(&context[..n]).unwrap();
    let mut roots = [EMPTY_ROOT; 7];
    let mut rr = [EMPTY_REF; 1];
    let mut publisher =
        ProtectedPublisher::bind(geometry, publication, context, &mut roots, &mut rr).unwrap();
    f(&mut publisher, &control, &time, public.as_ref());
}
#[derive(Default)]
struct Sink {
    artifacts: Vec<(Artifact, Vec<u8>)>,
    leaves: Vec<(ObjectScope, u64, Vec<u8>)>,
    fail_cipher: bool,
    fail_descriptor: bool,
    panic_roots: bool,
}
impl CandidateSink for Sink {
    fn artifact(&mut self, k: Artifact, _: u16, b: &[u8]) -> Result<(), ()> {
        if k == Artifact::Descriptor && self.fail_descriptor {
            return Err(());
        }
        if k == Artifact::Roots && self.panic_roots {
            panic!("root sink panic");
        }
        self.artifacts.push((k, b.to_vec()));
        Ok(())
    }
    fn ciphertext(&mut self, s: ObjectScope, i: u64, b: &[u8]) -> Result<(), ()> {
        if self.fail_cipher {
            return Err(());
        }
        self.leaves.push((s, i, b.to_vec()));
        Ok(())
    }
    fn outboard(&mut self, _: &TreeSpec, _: OutboardNode) -> Result<(), ()> {
        Ok(())
    }
}
fn payloads(p: &mut ProtectedPublisher<'_, '_, '_, '_>, sink: &mut Sink) {
    let mut leaf = [0; 16400];
    p.stage_header(sink).unwrap();
    for _ in 0..p.geometry.object_count() {
        p.seal_next_object(&mut leaf, sink).unwrap();
        assert!(leaf.iter().all(|b| *b == 0));
    }
}
fn complete(p: &mut ProtectedPublisher<'_, '_, '_, '_>, sink: &mut Sink) -> Vec<u8> {
    payloads(p, sink);
    let mut m = vec![99; p.geometry.metadata_length()];
    let mut leaf = [0; 16400];
    p.seal_metadata(refs(), &mut m, &mut leaf, sink).unwrap();
    assert!(m.iter().all(|b| *b == 0));
    assert!(leaf.iter().all(|b| *b == 0));
    let mut d = [0; MAX_DESCRIPTOR];
    let candidate = p.finish(&mut d, sink).unwrap();
    let encoded = sink
        .artifacts
        .iter()
        .find(|(k, _)| *k == Artifact::Descriptor)
        .unwrap()
        .1
        .clone();
    assert_eq!(candidate.descriptor_id(), sha384(&encoded));
    encoded
}
#[test]
fn protected_publication_uses_sorted_private_mapping_real_crypto_and_ciphertext_trees() {
    fixture(false, |p, _, _, _| {
        let mut sink = Sink::default();
        payloads(p, &mut sink);
        let mut m = vec![0; p.geometry.metadata_length()];
        metadata::write(&p.geometry, &p.publication, p.context, refs(), &mut m).unwrap();
        assert_eq!(&m[..12], b"QBDL-PMD-V1\0");
        assert_eq!(&m[16..64], &p.context.id().0);
        assert_eq!(
            u32::from_be_bytes(m[284..288].try_into().unwrap()) as usize,
            p.geometry.object_count()
        );
        for (i, mapping) in p.geometry.mappings.iter().enumerate() {
            let row = &m[288 + i * 108..288 + (i + 1) * 108];
            let original = &p.geometry.source.objects()[mapping.source_slot];
            assert_eq!(u32::from_be_bytes(row[..4].try_into().unwrap()), mapping.id);
            assert_eq!(
                u64::from_be_bytes(row[4..12].try_into().unwrap()),
                original.offset()
            );
            let ordinal = match original.kind() {
                ObjectKind::Entry(n) => n,
                ObjectKind::Gap => u32::MAX,
            };
            assert_eq!(u32::from_be_bytes(row[21..25].try_into().unwrap()), ordinal);
            assert_eq!(&row[28..76], &mapping.digest.0);
            assert_eq!(row[76..].iter().all(|v| *v == 0), original.length() == 0);
            if original.length() == 0 {
                let f = p.context.fields();
                let spec = TreeSpec::new(
                    p.context.id(),
                    mapping.id,
                    f.generation,
                    Representation::Ciphertext,
                    f.key_epoch,
                    0,
                )
                .unwrap();
                assert_eq!(p.roots[i].root, empty_root(&spec).unwrap());
                assert!(!sink.leaves.iter().any(|(s, _, _)| s.id() == mapping.id));
            }
        }
        let mut leaf = [0; 16400];
        p.seal_metadata(refs(), &mut m, &mut leaf, &mut sink)
            .unwrap();
        assert!(m.iter().all(|v| *v == 0));
        let mut out = [0; MAX_DESCRIPTOR];
        p.finish(&mut out, &mut sink).unwrap();
        for (scope, _, _) in &sink.leaves {
            let spec = TreeSpec::new(
                scope.context(),
                scope.id(),
                scope.generation(),
                Representation::Ciphertext,
                scope.epoch(),
                scope.length(),
            )
            .unwrap();
            let mut tree = TreeBuilder::new(spec);
            for (_, _, bytes) in sink.leaves.iter().filter(|(s, _, _)| s.id() == scope.id()) {
                tree.append(bytes, |_| Ok(())).unwrap();
            }
            let root = tree.finish(|_| Ok(())).unwrap();
            assert_eq!(
                root,
                if scope.id() == 0 {
                    p.metadata_root.unwrap()
                } else {
                    p.roots.iter().find(|r| r.id == scope.id()).unwrap().root
                }
            );
        }
        for (_, b) in &sink.artifacts {
            let private_name = b"private-source-entry-name-";
            assert!(!b.windows(private_name.len()).any(|w| w == private_name));
            assert!(!b.windows(48).any(|w| w == refs().closure.0));
        }
    });
}
#[test]
fn protected_sink_error_and_unwind_after_finished_sealers_abort_entire_attempt() {
    for panic in [false, true] {
        fixture(false, |p, _, _, _| {
            let mut sink = Sink::default();
            payloads(p, &mut sink);
            let mut m = vec![0; p.geometry.metadata_length()];
            p.seal_metadata(refs(), &mut m, &mut [0; 16400], &mut sink)
                .unwrap();
            sink.fail_descriptor = !panic;
            sink.panic_roots = panic;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                p.finish(&mut [0; MAX_DESCRIPTOR], &mut sink)
            }));
            if panic {
                assert!(result.is_err());
            } else {
                assert!(result.unwrap().is_err());
            }
            assert!(p.is_poisoned());
            assert!(p.publication.metadata_master_row(0, &mut [0; 32]).is_err());
            assert!(
                p.finish(&mut [0; MAX_DESCRIPTOR], &mut Sink::default())
                    .is_err()
            );
        });
    }
}
#[test]
fn protected_cipher_sink_failure_and_late_cancellation_erase_scratch() {
    fixture(false, |p, control, _, _| {
        let mut sink = Sink::default();
        p.stage_header(&mut sink).unwrap();
        control.cancel().unwrap();
        let mut scratch = [99; 16400];
        assert!(p.seal_next_object(&mut scratch, &mut sink).is_err());
        assert!(p.is_poisoned());
        assert!(sink.leaves.is_empty());
        assert!(scratch.iter().all(|v| *v == 0));
    });
    fixture(false, |p, _, _, _| {
        let mut sink = Sink {
            fail_cipher: true,
            ..Default::default()
        };
        p.stage_header(&mut sink).unwrap();
        let mut scratch = [99; 16400];
        while !p.is_poisoned() {
            let result = p.seal_next_object(&mut scratch, &mut sink);
            if result.is_err() {
                break;
            }
        }
        assert!(p.is_poisoned());
        assert!(scratch.iter().all(|v| *v == 0));
    });
}
#[test]
fn protected_metadata_capacity_failure_after_payloads_is_terminal_and_erases_buffer() {
    fixture(false, |p, _, _, _| {
        let mut sink = Sink::default();
        payloads(p, &mut sink);
        let mut short = vec![99; p.geometry.metadata_length() - 1];
        assert!(
            p.seal_metadata(refs(), &mut short, &mut [0; 16400], &mut sink)
                .is_err()
        );
        assert!(short.iter().all(|v| *v == 0));
        assert!(p.is_poisoned());
    });
}
#[test]
fn protected_candidate_signing_reuses_same_managed_owner_and_exact_binding() {
    fixture(true, |p, _, _, public| {
        let public = public.unwrap();
        let mut sink = Sink::default();
        let bytes = complete(p, &mut sink);
        let descriptor = Descriptor::parse(&bytes).unwrap();
        let digest = descriptor.id();
        let audience = p.context.fields().scope_token;
        let binding = cose::Binding {
            descriptor_digest: &digest.0,
            key_ids: public.ids,
            controller_epoch: 1,
            purpose: PUBLISHER_PURPOSE,
            audience: &audience,
        };
        let request = PairedSigningRequest {
            ml_key_id: "ml_private",
            ed_key_id: "ed_private",
            ml_policy_id: "ml_private",
            ed_policy_id: "ed_private",
            binding,
            grant: grant(KeyPurpose::ControllerSign),
            access: access(),
        };
        let mut workspace = cose::Workspace::new();
        p.sign_candidate(&bytes, request, &mut workspace, &mut sink)
            .unwrap();
        let proof = &sink.artifacts.last().unwrap().1;
        let public = cose::PairedPublicKeys {
            ml_dsa_65: public.ml.as_slice().try_into().unwrap(),
            ed25519: public.ed.as_slice().try_into().unwrap(),
        };
        assert!(cose::verify_pair(proof, &binding, &public, &mut workspace).is_ok());
    });
}
#[test]
fn protected_wrong_publisher_purpose_cannot_stage_signature() {
    fixture(true, |p, _, _, public| {
        let public = public.unwrap();
        let mut sink = Sink::default();
        let bytes = complete(p, &mut sink);
        let digest = sha384(&bytes);
        let audience = p.context.fields().scope_token;
        let request = PairedSigningRequest {
            ml_key_id: "ml_private",
            ed_key_id: "ed_private",
            ml_policy_id: "ml_private",
            ed_policy_id: "ed_private",
            binding: cose::Binding {
                descriptor_digest: &digest.0,
                key_ids: public.ids,
                controller_epoch: 1,
                purpose: b"other",
                audience: &audience,
            },
            grant: grant(KeyPurpose::ControllerSign),
            access: access(),
        };
        assert!(
            p.sign_candidate(&bytes, request, &mut cose::Workspace::new(), &mut sink)
                .is_err()
        );
        assert!(p.is_poisoned());
        assert!(
            !sink
                .artifacts
                .iter()
                .any(|(k, _)| *k == Artifact::PublisherProof)
        );
    });
}

#[path = "publisher_boundary_tests.rs"]
mod boundary;
