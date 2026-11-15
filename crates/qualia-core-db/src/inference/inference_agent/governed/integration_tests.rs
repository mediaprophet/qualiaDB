//! Independent admission checks use isolated genuine keys and the actual VM.
use super::*;
use ed25519_dalek::{Signer, SigningKey};

fn isolated_key() -> SigningKey {
    let mut seed = zeroize::Zeroizing::new([0u8; 32]);
    getrandom::fill(&mut *seed).expect("isolated test key entropy");
    SigningKey::from_bytes(&seed)
}

fn trusted(key: &SigningKey) -> TrustedPolicyIdentity {
    TrustedPolicyIdentity::from_host_configuration(
        key.verifying_key().to_bytes(),
        17,
        INFERENCE_POLICY_AUDIENCE,
        3,
    )
    .unwrap()
}

fn quin(subject: u64, predicate: u64, object: u64, context: u64) -> crate::NQuin {
    crate::NQuin {
        subject,
        predicate,
        object,
        context,
        metadata: 0,
        parity: subject ^ predicate ^ object ^ context,
    }
}

fn explicit_permissive_document() -> PolicyDocument {
    PolicyDocument {
        version: POLICY_VERSION,
        audience: INFERENCE_POLICY_AUDIENCE,
        principal: 17,
        controller_generation: 3,
        policy_generation: 1,
        not_before: 10,
        expires_at: 100,
        rules: PolicyRules {
            actors: PolicyList::from_vec(vec![ActorPolicyScope {
                subject: 17,
                kind: qualia_cooperative_core::authoring::PrincipalKind::Human,
                controller: [1; 32],
                project_digest: [2; 32],
                selected_identifier_digest: [3; 32],
                association_digest: [4; 32],
                admission_digest: [5; 32],
                admission_sequence: 1,
                authority_generation: 1,
                valid_until: 4_000_000_000,
            }])
            .unwrap(),
            model: DigestScope::Any,
            request: DigestScope::Any,
            graph_scopes: PolicyList::from_vec(vec![29]).unwrap(),
            program: PolicyList::default(),
            norms: PolicyList::default(),
            facts: PolicyList::default(),
            epistemic: PolicyList::default(),
            trace: PolicyList::default(),
            required_claim: 0,
            temporal_invariant: 0,
            non_derogable: false,
        },
    }
}

// Sign through the actual cryptographic library, independently of the host signer.
fn signed(document: PolicyDocument, key: &SigningKey) -> SignedPolicy {
    let bytes = key.sign(&policy_digest(&document)).to_bytes();
    SignedPolicy {
        document,
        signature: [
            bytes[..32].try_into().unwrap(),
            bytes[32..].try_into().unwrap(),
        ],
    }
}

#[test]
fn genuine_signature_uses_independent_host_trust_and_exact_time_boundaries() {
    let key = isolated_key();
    let other = isolated_key();
    let trust = trusted(&key);
    let policy = signed(explicit_permissive_document(), &key);
    assert_eq!(policy.verify(&trust, 10), Ok(()));
    assert_eq!(policy.verify(&trust, 99), Ok(()));
    assert_eq!(policy.verify(&trust, 9), Err(PolicyError::Expired));
    assert_eq!(policy.verify(&trust, 100), Err(PolicyError::Expired));
    assert_eq!(
        policy.verify(&trusted(&other), 11),
        Err(PolicyError::InvalidSignature)
    );
    let attacker = signed(policy.document.clone(), &other);
    assert_eq!(
        attacker.verify(&trust, 11),
        Err(PolicyError::InvalidSignature)
    );
    let mut corrupted = policy.clone();
    corrupted.signature[1][17] ^= 1;
    assert_eq!(
        corrupted.verify(&trust, 11),
        Err(PolicyError::InvalidSignature)
    );
    for (principal, audience, generation) in [
        (18, INFERENCE_POLICY_AUDIENCE, 3),
        (17, 7, 3),
        (17, INFERENCE_POLICY_AUDIENCE, 4),
    ] {
        let wrong = TrustedPolicyIdentity::from_host_configuration(
            key.verifying_key().to_bytes(),
            principal,
            audience,
            generation,
        )
        .unwrap();
        assert_eq!(policy.verify(&wrong, 11), Err(PolicyError::WrongAuthority));
    }
    let mut json = serde_json::to_value(&policy).unwrap();
    json["controller_key"] = serde_json::json!(other.verifying_key().to_bytes());
    assert!(serde_json::from_value::<SignedPolicy>(json).is_err());
}

#[test]
fn canonical_digest_covers_all_document_rules_and_quin_fields() {
    let key = isolated_key();
    let trust = trusted(&key);
    let mut doc = explicit_permissive_document();
    doc.rules.model = DigestScope::Exact([11; 32]);
    doc.rules.request = DigestScope::Exact([12; 32]);
    doc.rules.program = PolicyList::from_vec(vec![
        ConstraintInstruction::MatchSubject { value: 17 },
        ConstraintInstruction::HaltIfFalse,
    ])
    .unwrap();
    doc.rules.norms = PolicyList::from_vec(vec![quin(1, 2, 3, 4)]).unwrap();
    doc.rules.facts = PolicyList::from_vec(vec![quin(5, 6, 7, 8)]).unwrap();
    doc.rules.epistemic = PolicyList::from_vec(vec![quin(9, 10, 11, 12)]).unwrap();
    doc.rules.trace = PolicyList::from_vec(vec![quin(13, 14, 15, 16)]).unwrap();
    let policy = signed(doc, &key);
    let original = policy_digest(&policy.document);
    let base = serde_json::to_value(&policy.document).unwrap();
    let mut paths = vec![
        "/version".to_string(),
        "/audience".into(),
        "/principal".into(),
        "/controller_generation".into(),
        "/policy_generation".into(),
        "/not_before".into(),
        "/expires_at".into(),
        "/rules/graph_scopes/0".into(),
        "/rules/required_claim".into(),
        "/rules/temporal_invariant".into(),
        "/rules/program/0/value".into(),
        "/rules/model/digest/0".into(),
        "/rules/request/digest/0".into(),
    ];
    for field in [
        "subject",
        "admission_sequence",
        "authority_generation",
        "valid_until",
    ] {
        paths.push(format!("/rules/actors/0/{field}"));
    }
    for field in [
        "controller",
        "project_digest",
        "selected_identifier_digest",
        "association_digest",
        "admission_digest",
    ] {
        paths.push(format!("/rules/actors/0/{field}/0"));
    }
    for list in ["norms", "facts", "epistemic", "trace"] {
        for field in [
            "subject",
            "predicate",
            "object",
            "context",
            "metadata",
            "parity",
        ] {
            paths.push(format!("/rules/{list}/0/{field}"));
        }
    }
    for path in paths {
        let mut value = base.clone();
        let field = value.pointer_mut(&path).unwrap();
        *field = serde_json::json!(field.as_u64().unwrap() + 1);
        let changed: PolicyDocument = serde_json::from_value(value).unwrap();
        assert_ne!(policy_digest(&changed), original, "{path}");
        assert!(
            SignedPolicy {
                document: changed,
                signature: policy.signature
            }
            .verify(&trust, 11)
            .is_err(),
            "{path}"
        );
    }
    let mut flag = policy.document.clone();
    flag.rules.non_derogable = true;
    assert_ne!(policy_digest(&flag), original);
    assert_eq!(
        SignedPolicy {
            document: flag,
            signature: policy.signature
        }
        .verify(&trust, 11),
        Err(PolicyError::InvalidSignature)
    );
    let pretty = serde_json::to_string_pretty(&policy).unwrap();
    let roundtrip: SignedPolicy = serde_json::from_str(&pretty).unwrap();
    assert_eq!(policy_digest(&roundtrip.document), original);
    assert_eq!(roundtrip.verify(&trust, 11), Ok(()));
}

#[test]
fn ordered_length_delimited_collections_and_scope_tags_are_distinct() {
    let mut a = explicit_permissive_document();
    let mut b = a.clone();
    a.rules.facts = PolicyList::from_vec(vec![quin(1, 2, 3, 4)]).unwrap();
    b.rules.trace = PolicyList::from_vec(vec![quin(1, 2, 3, 4)]).unwrap();
    assert_ne!(policy_digest(&a), policy_digest(&b));
    b = a.clone();
    b.rules.model = DigestScope::Exact([0; 32]);
    assert_ne!(policy_digest(&a), policy_digest(&b));
    a.rules.graph_scopes = PolicyList::from_vec(vec![1, 2]).unwrap();
    b = a.clone();
    b.rules.graph_scopes = PolicyList::from_vec(vec![2, 1]).unwrap();
    assert_ne!(policy_digest(&a), policy_digest(&b));
    let too_many = vec![0u64; 33];
    assert!(PolicyList::<u64, 32>::from_vec(too_many.clone()).is_err());
    assert!(serde_json::from_str::<PolicyList<u64, 32>>(
        &serde_json::to_string(&too_many).unwrap()
    )
    .is_err());
    let mut json = serde_json::to_value(explicit_permissive_document()).unwrap();
    json["rules"]["program"] = serde_json::json!([{"op":"LoadModel","value":7}]);
    assert!(serde_json::from_value::<PolicyDocument>(json).is_err());
}

#[test]
fn every_program_opcode_and_operand_has_canonical_identity() {
    use ConstraintInstruction::*;
    let operations = [
        MatchSubject { value: 7 },
        MatchPredicate { value: 7 },
        MatchObject { value: 7 },
        MetadataMask { mask: 7 },
        Bind {
            vector: 1,
            register: 2,
        },
        Bind {
            vector: 2,
            register: 2,
        },
        Bind {
            vector: 1,
            register: 3,
        },
        MatchRegister {
            vector: 1,
            register: 2,
        },
        MatchRegister {
            vector: 2,
            register: 2,
        },
        MatchRegister {
            vector: 1,
            register: 3,
        },
        HaltIfFalse,
        LessThan {
            vector: 1,
            value: 0.5,
        },
        GreaterThan {
            vector: 1,
            value: 0.5,
        },
        LessOrEqual {
            vector: 1,
            value: 0.5,
        },
        GreaterOrEqual {
            vector: 1,
            value: 0.5,
        },
        LessThan {
            vector: 2,
            value: 0.5,
        },
        LessThan {
            vector: 1,
            value: 0.25,
        },
        Confidence { minimum: 0.5 },
        Confidence { minimum: 0.0 },
        Confidence { minimum: -0.0 },
    ];
    let mut seen = Vec::new();
    for operation in operations {
        let mut doc = explicit_permissive_document();
        doc.rules.program = PolicyList::from_vec(vec![operation]).unwrap();
        assert!(doc.validate().is_ok());
        let digest = policy_digest(&doc);
        assert!(!seen.contains(&digest));
        seen.push(digest);
    }
    let mut spare = Vec::<u64>::with_capacity(33);
    spare.push(1);
    assert!(PolicyList::<u64, 32>::from_vec(spare).is_err());
}

#[test]
fn actor_kind_order_and_version_are_signed_and_empty_actor_scope_refuses() {
    let original = explicit_permissive_document();
    let digest = policy_digest(&original);
    let mut alternate = original.clone();
    let mut scope = alternate.rules.actors.as_slice()[0];
    scope.kind = qualia_cooperative_core::authoring::PrincipalKind::Organization;
    alternate.rules.actors = PolicyList::from_vec(vec![scope]).unwrap();
    assert_ne!(policy_digest(&alternate), digest);
    let first = original.rules.actors.as_slice()[0];
    let mut both = original.clone();
    both.rules.actors = PolicyList::from_vec(vec![first, scope]).unwrap();
    let ordered = policy_digest(&both);
    both.rules.actors = PolicyList::from_vec(vec![scope, first]).unwrap();
    assert_ne!(policy_digest(&both), ordered);
    both.rules.actors = PolicyList::default();
    assert_eq!(both.validate(), Err(PolicyError::Scope));
    let mut old = original;
    old.version = 1;
    assert!(old.validate().is_err());
}

#[test]
fn candidate_evidence_binds_position_token_logit_and_all_observation_bits() {
    let p = TokenProposal {
        position: 7,
        token: 13,
        logit: -0.0,
        observation: quin(17, 23, 31, 29),
    };
    let bytes = p.encode().unwrap();
    assert_eq!(bytes.len(), 68);
    let roundtrip = TokenProposal::decode(&bytes).unwrap();
    assert_eq!(roundtrip.logit.to_bits(), 0x80000000);
    assert_eq!(roundtrip.encode().unwrap(), bytes);
    assert!(TokenProposal::decode(&bytes[..67]).is_err());
    let output = p.output().unwrap();
    for mode in 0..9 {
        let mut changed = p;
        match mode {
            0 => changed.position += 1,
            1 => changed.token += 1,
            2 => changed.logit = 0.0,
            3 => changed.observation.subject += 1,
            4 => changed.observation.predicate += 1,
            5 => changed.observation.object += 1,
            6 => changed.observation.context += 1,
            7 => changed.observation.metadata += 1,
            _ => changed.observation.parity ^= 1,
        }
        if mode == 8 {
            assert!(changed.output().is_err());
            continue;
        }
        let q = &mut changed.observation;
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        assert_ne!(
            changed.output().unwrap().observation_digest,
            output.observation_digest
        );
        if mode < 3 {
            assert_ne!(
                changed.output().unwrap().output_digest,
                output.output_digest
            );
        }
    }
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(TokenProposal { logit: value, ..p }.encode().is_err());
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use etalon_inference_contract::{Decision, ExecutionMode, Instrumentation, Phase};
    const BUDGET: usize = POLICY_COLD_BUDGET_BYTES;
    fn admission() -> InferenceAdmission {
        InferenceAdmission {
            request_digest: [1; 32],
            model_digest: [2; 32],
            model_generation: 5,
            device_id: 7,
            device_generation: 11,
            graph_scope: 29,
            first_position: 4,
            instrumentation: Instrumentation::default(),
        }
    }
    fn proposal() -> TokenProposal {
        TokenProposal {
            position: 4,
            token: 13,
            logit: 0.5,
            observation: quin(crate::q_hash("actor"), 23, 31, 29),
        }
    }
    use crate::poet_host::native_authority::{
        self as identity,
        inference::{AuthenticatedInferenceAssociation, IdentifierSelection},
        NativeAuthority,
    };
    use qualia_cooperative_core::{
        authoring::{DocumentRevision, IdentifierAssociationEvidence, PrincipalKind},
        projects::{Project, ProjectMembership, ProjectRole},
    };
    use std::sync::Arc;
    fn actor_snapshot(kind: PrincipalKind) -> identity::AdmissionSnapshot {
        identity::AdmissionSnapshot {
            version: 1,
            sequence: 1,
            issuer_identifier: "independent-project-issuer".into(),
            project: Project {
                id: "project".into(),
                name: "independent".into(),
                description: String::new(),
                licensing_ontologies: vec![],
                created_at_unix: 1,
                created_at_instant: None,
            },
            valid_from_unix: 1,
            valid_until_unix: 4_000_000_000,
            revoked: false,
            package_reviews: vec![],
            subjects: vec![identity::SubjectAdmission {
                subject: "actor".into(),
                kind,
                controller_public_key: crate::crypto::network::ed25519::public_from_seed(&[23; 32]),
                membership: ProjectMembership {
                    id: "membership".into(),
                    project_id: "project".into(),
                    member_did: "did:test:actor".into(),
                    role: ProjectRole::Contributor,
                    agreed_at_unix: 1,
                    agreed_at_instant: None,
                },
                identifiers: ["did:test:actor", "urn:alternate:actor"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, value)| IdentifierAssociationEvidence {
                        value: value.into(),
                        scheme: if i == 0 { "did:test" } else { "urn" }.into(),
                        issuer: "identifier-issuer".into(),
                        project_id: "project".into(),
                        evidence_ref: format!("association/{i}"),
                        valid_from_unix: 1,
                        valid_until_unix: 4_000_000_000,
                    })
                    .collect(),
                reviews: vec![],
                admission_ref: "explicit-project-adoption".into(),
                valid_from_unix: 1,
                valid_until_unix: 4_000_000_000,
                revoked: false,
            }],
        }
    }
    fn sign_actor(snapshot: identity::AdmissionSnapshot) -> identity::SignedAdmission {
        let digest = identity::admission_digest(&snapshot, 1024 * 1024).unwrap();
        let mut signature = [0; 64];
        crate::crypto::network::ed25519::sign_with_context(
            &[19; 32],
            &digest,
            identity::ADMISSION_CONTEXT,
            &mut signature,
        )
        .unwrap();
        identity::SignedAdmission {
            snapshot,
            signature: signature.to_vec(),
        }
    }
    fn actor_owner(dir: &std::path::Path, kind: PrincipalKind) -> Arc<NativeAuthority> {
        owner_from_snapshot(dir, actor_snapshot(kind))
    }
    fn owner_from_snapshot(
        dir: &std::path::Path,
        snapshot: identity::AdmissionSnapshot,
    ) -> Arc<NativeAuthority> {
        std::fs::create_dir_all(dir.join("actor")).unwrap();
        Arc::new(
            NativeAuthority::open(
                &dir.join("actor"),
                identity::TrustedProject {
                    project_id: "project".into(),
                    issuer_identifier: "independent-project-issuer".into(),
                    public_key: crate::crypto::network::ed25519::public_from_seed(&[19; 32]),
                },
                sign_actor(snapshot),
                DocumentRevision {
                    project_id: "project".into(),
                    document_id: "document".into(),
                    source_id: "source".into(),
                    revision: 1,
                    source_digest: [9; 32],
                    author_subject: Some("actor".into()),
                },
                1024 * 1024,
                256,
            )
            .unwrap(),
        )
    }
    fn association(
        owner: &NativeAuthority,
        kind: PrincipalKind,
        index: usize,
    ) -> AuthenticatedInferenceAssociation {
        let challenge = owner.begin_session("actor", 300).unwrap();
        let mut signature = [0; 64];
        crate::crypto::network::ed25519::sign_with_context(
            &[23; 32],
            challenge.message_digest(),
            identity::SESSION_CONTEXT,
            &mut signature,
        )
        .unwrap();
        let session = owner.complete_session(challenge, &signature).unwrap();
        let snapshot = actor_snapshot(kind);
        let id = &snapshot.subjects[0].identifiers[index];
        owner
            .bind_inference_association(
                &session,
                IdentifierSelection {
                    value: &id.value,
                    scheme: &id.scheme,
                    issuer: &id.issuer,
                    project_id: &id.project_id,
                    evidence_ref: &id.evidence_ref,
                },
            )
            .unwrap()
    }
    struct TestHost {
        host: NativePolicyHost,
        owner: Arc<NativeAuthority>,
    }
    impl std::ops::Deref for TestHost {
        type Target = NativePolicyHost;
        fn deref(&self) -> &Self::Target {
            &self.host
        }
    }
    impl std::ops::DerefMut for TestHost {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.host
        }
    }
    impl TestHost {
        fn admit_at(
            &mut self,
            key: &SigningKey,
            request: InferenceAdmission,
            now: u64,
            budget: usize,
        ) -> Result<GovernedTokenSession, PolicyError> {
            let actor = association(&self.owner, PrincipalKind::Human, 0);
            self.host
                .admit_at(key, request, now, budget, Arc::clone(&self.owner), actor)
        }
    }
    trait EnqueueRecord {
        fn enqueue_record(&mut self, ticket: &TokenDecision, now: u64) -> Result<(), PolicyError>;
    }
    impl EnqueueRecord for GovernedTokenSession {
        fn enqueue_record(&mut self, ticket: &TokenDecision, now: u64) -> Result<(), PolicyError> {
            let mut out = None;
            let result = self.accept_into_at(ticket, now, &mut out);
            if result.is_ok() {
                let accepted = out.unwrap();
                assert_eq!(
                    accepted.proposal.encode().unwrap(),
                    ticket.proposal().encode().unwrap()
                );
                assert_eq!(accepted.key, ticket.key());
            } else {
                assert!(out.is_none());
            }
            result
        }
    }
    fn open(dir: &std::path::Path, key: &SigningKey) -> Result<TestHost, PolicyError> {
        let host = NativePolicyHost::open(
            &dir.join("policy"),
            &dir.join("authority"),
            trusted(key),
            BUDGET,
        )?;
        Ok(TestHost {
            host,
            owner: actor_owner(dir, PrincipalKind::Human),
        })
    }
    fn reopen(
        dir: &std::path::Path,
        key: &SigningKey,
        owner: Arc<NativeAuthority>,
    ) -> Result<TestHost, PolicyError> {
        let host = NativePolicyHost::open(
            &dir.join("policy"),
            &dir.join("authority"),
            trusted(key),
            BUDGET,
        )?;
        Ok(TestHost { host, owner })
    }
    fn install(host: &mut TestHost, key: &SigningKey, mut rules: PolicyRules) {
        let actor = association(&host.owner, PrincipalKind::Human, 0);
        rules.actors =
            PolicyList::from_vec(vec![ActorPolicyScope::from_evidence(actor.evidence())]).unwrap();
        let generation = host.status().generation;
        host.install(key, rules, 10, 100, 20, generation, BUDGET)
            .unwrap();
    }
    fn clean_dirs(dir: &std::path::Path) {
        for path in [dir.join("policy"), dir.join("authority")] {
            for entry in std::fs::read_dir(path).unwrap() {
                assert!(entry.unwrap().file_type().unwrap().is_file());
            }
        }
    }

    #[test]
    fn explicitly_installed_policy_allows_and_absence_never_installs() {
        let dir = tempfile::tempdir().unwrap();
        let key = isolated_key();
        let mut host = open(dir.path(), &key).unwrap();
        assert!(matches!(
            host.admit_at(&key, admission(), 20, BUDGET),
            Err(PolicyError::MissingPolicy)
        ));
        assert!(!host.status().installed);
        assert!(!dir
            .path()
            .join("policy/sentinel-inference-policy.json")
            .exists());
        install(&mut host, &key, explicit_permissive_document().rules);
        let mut session = host.admit_at(&key, admission(), 20, BUDGET).unwrap();
        assert_eq!(session.snapshot().mode, ExecutionMode::GovernedApplication);
        let ticket = session.prepare_at(proposal(), 20).unwrap();
        assert_eq!(ticket.decision(), Decision::Allow);
        session.enqueue_record(&ticket, 20).unwrap();
        assert_eq!(session.snapshot().phase, Phase::Ready);
        assert!(session.enqueue_record(&ticket, 20).is_err());
        clean_dirs(dir.path());
    }

    #[test]
    fn actual_vm_and_each_graph_guard_action_control_current_proposal() {
        for mode in 0..8 {
            let dir = tempfile::tempdir().unwrap();
            let key = isolated_key();
            let mut host = open(dir.path(), &key).unwrap();
            let mut rules = explicit_permissive_document().rules;
            match mode {
                0 => {
                    rules.program = PolicyList::from_vec(vec![
                        ConstraintInstruction::MatchObject { value: 32 },
                        ConstraintInstruction::HaltIfFalse,
                        ConstraintInstruction::MatchSubject {
                            value: crate::q_hash("actor"),
                        },
                    ])
                    .unwrap()
                }
                1 => {
                    rules.program = PolicyList::from_vec(vec![
                        ConstraintInstruction::MatchObject { value: 32 },
                        ConstraintInstruction::MatchSubject {
                            value: crate::q_hash("actor"),
                        },
                    ])
                    .unwrap()
                }
                2 | 3 => {
                    rules.norms = PolicyList::from_vec(vec![quin(
                        crate::q_hash("actor"),
                        crate::modalities::logic::deontic::OP_FORBID as u64,
                        31,
                        29,
                    )])
                    .unwrap();
                    rules.non_derogable = mode == 2;
                }
                4 | 5 => {
                    rules.required_claim = 47;
                    if mode == 5 {
                        rules.epistemic = PolicyList::from_vec(vec![quin(
                            crate::q_hash("actor"),
                            crate::modalities::epistemic::OP_KNOWS as u64
                                | (255 << crate::modalities::epistemic::CERTAINTY_BIT_SHIFT),
                            47,
                            29,
                        )])
                        .unwrap();
                    }
                }
                6 => rules.temporal_invariant = 99,
                _ => {
                    rules.facts =
                        PolicyList::from_vec(vec![quin(crate::q_hash("actor"), 23, 32, 29)])
                            .unwrap()
                }
            }
            install(&mut host, &key, rules);
            let mut request = admission();
            request.instrumentation = Instrumentation {
                tracing: true,
                profiling: true,
            };
            let mut session = host.admit_at(&key, request, 20, BUDGET).unwrap();
            let mut candidate = proposal();
            if mode == 2 || mode == 3 {
                candidate.observation.predicate = crate::q_hash("q42:performed");
                let q = &mut candidate.observation;
                q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
            }
            let ticket = session.prepare_at(candidate, 20).unwrap();
            match mode {
                1 | 5 => assert_eq!(ticket.decision(), Decision::Allow),
                0 | 2 | 6 => assert_eq!(ticket.decision(), Decision::Deny, "mode {mode}"),
                _ => assert!(matches!(ticket.decision(), Decision::Steer { .. })),
            }
            assert_eq!(session.snapshot().mode, ExecutionMode::GovernedApplication);
            if mode == 1 || mode == 5 {
                session.enqueue_record(&ticket, 20).unwrap();
            } else {
                assert!(session.enqueue_record(&ticket, 20).is_err());
                assert!(session.prepare_at(proposal(), 21).is_err());
            }
        }
    }

    #[test]
    fn exact_admission_scope_and_cross_session_tickets_are_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let key = isolated_key();
        let mut host = open(dir.path(), &key).unwrap();
        let mut rules = explicit_permissive_document().rules;
        rules.model = DigestScope::Exact([2; 32]);
        rules.request = DigestScope::Exact([1; 32]);
        install(&mut host, &key, rules);
        for mode in 0..3 {
            let mut request = admission();
            match mode {
                0 => request.model_digest[0] ^= 1,
                1 => request.request_digest[0] ^= 1,
                _ => request.graph_scope += 1,
            };
            assert!(host.admit_at(&key, request, 20, BUDGET).is_err());
        }
        let mut a = host.admit_at(&key, admission(), 20, BUDGET).unwrap();
        let mut b = host.admit_at(&key, admission(), 20, BUDGET).unwrap();
        assert_ne!(
            a.snapshot().config.identity.session_id,
            b.snapshot().config.identity.session_id
        );
        let first = a.prepare_at(proposal(), 20).unwrap();
        let second = b.prepare_at(proposal(), 20).unwrap();
        assert!(b.enqueue_record(&first, 20).is_err());
        assert!(b.is_terminal());
        assert!(b.enqueue_record(&second, 20).is_err());
        a.enqueue_record(&first, 20).unwrap();
        for mode in 0..2 {
            let mut s = host.admit_at(&key, admission(), 20, BUDGET).unwrap();
            let mut wrong = proposal();
            if mode == 0 {
                wrong.position += 1;
            } else {
                wrong.observation.context += 1;
                let q = &mut wrong.observation;
                q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
            }
            assert!(s.prepare_at(wrong, 20).is_err());
        }
    }

    #[test]
    fn exact_expiry_revocation_replacement_and_reconnection_invalidate_pending_allow() {
        for mode in 0..5 {
            let dir = tempfile::tempdir().unwrap();
            let key = isolated_key();
            let mut host = open(dir.path(), &key).unwrap();
            install(&mut host, &key, explicit_permissive_document().rules);
            let mut s = host.admit_at(&key, admission(), 20, BUDGET).unwrap();
            let ticket = s.prepare_at(proposal(), 20).unwrap();
            match mode {
                0 => {}
                1 => host.disconnect(),
                2 => {
                    host.disconnect();
                    host.reload(BUDGET).unwrap();
                }
                3 => {
                    host.revoke(&key, 1, BUDGET).unwrap();
                }
                _ => install(&mut host, &key, explicit_permissive_document().rules),
            }
            assert!(s
                .enqueue_record(&ticket, if mode == 0 { 100 } else { 21 })
                .is_err());
            assert!(s
                .prepare_at(proposal(), if mode == 0 { 100 } else { 21 })
                .is_err());
        }
    }

    #[test]
    fn exclusive_store_lock_and_host_drop_cover_surviving_session_lifetime() {
        let dir = tempfile::tempdir().unwrap();
        let key = isolated_key();
        let mut host = open(dir.path(), &key).unwrap();
        install(&mut host, &key, explicit_permissive_document().rules);
        assert!(open(dir.path(), &key).is_err());
        assert!(NativePolicyHost::open(
            &dir.path().join("policy/../policy"),
            &dir.path().join("authority"),
            trusted(&key),
            BUDGET
        )
        .is_err());
        let mut old = host.admit_at(&key, admission(), 20, BUDGET).unwrap();
        let ticket = old.prepare_at(proposal(), 20).unwrap();
        let id = ticket.key().identity.session_id;
        let owner = Arc::clone(&host.owner);
        drop(host);
        assert!(dir
            .path()
            .join("policy/sentinel-inference-policy.lock")
            .is_file());
        let mut reopened = reopen(dir.path(), &key, Arc::clone(&owner)).unwrap();
        assert!(old.enqueue_record(&ticket, 21).is_err());
        let mut fresh = reopened.admit_at(&key, admission(), 21, BUDGET).unwrap();
        assert!(fresh.snapshot().config.identity.session_id > id);
        let ticket = fresh.prepare_at(proposal(), 21).unwrap();
        reopened.revoke(&key, 1, BUDGET).unwrap();
        assert!(fresh.enqueue_record(&ticket, 21).is_err());
        drop(reopened);
        let mut revoked = reopen(dir.path(), &key, owner).unwrap();
        assert!(!revoked.status().installed);
        assert!(matches!(
            revoked.admit_at(&key, admission(), 22, BUDGET),
            Err(PolicyError::MissingPolicy)
        ));
    }

    #[test]
    fn restoring_old_signed_policy_record_cannot_undo_separate_anchor() {
        for revoke in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let key = isolated_key();
            let mut host = open(dir.path(), &key).unwrap();
            install(&mut host, &key, explicit_permissive_document().rules);
            let path = dir.path().join("policy/sentinel-inference-policy.json");
            let old = std::fs::read(&path).unwrap();
            assert!(old.len() <= MAX_POLICY_JSON_BYTES);
            if revoke {
                host.revoke(&key, 1, BUDGET).unwrap();
            } else {
                install(&mut host, &key, explicit_permissive_document().rules);
            }
            drop(host);
            std::fs::write(path, old).unwrap();
            assert!(matches!(
                open(dir.path(), &key),
                Err(PolicyError::StaleGeneration)
            ));
            clean_dirs(dir.path());
        }
    }

    #[test]
    fn publication_phase_failure_and_unwind_never_leave_old_authority_usable() {
        use super::super::host::PublishFault;
        for fault in [
            PublishFault::BeforeAnchor,
            PublishFault::AfterAnchor,
            PublishFault::AfterPolicy,
            PublishFault::PanicAfterAnchor,
        ] {
            let dir = tempfile::tempdir().unwrap();
            let key = isolated_key();
            let mut host = open(dir.path(), &key).unwrap();
            install(&mut host, &key, explicit_permissive_document().rules);
            let mut old = host.admit_at(&key, admission(), 20, BUDGET).unwrap();
            let ticket = old.prepare_at(proposal(), 20).unwrap();
            let path = dir.path().join("policy/sentinel-inference-policy.json");
            let previous = std::fs::read(&path).unwrap();
            host.fail_next_publication(fault);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                host.revoke(&key, 1, BUDGET)
            }));
            if fault == PublishFault::PanicAfterAnchor {
                assert!(result.is_err());
            } else {
                assert!(result.unwrap().is_err());
            }
            if fault == PublishFault::BeforeAnchor {
                assert_eq!(std::fs::read(&path).unwrap(), previous);
                old.enqueue_record(&ticket, 21).unwrap();
            } else {
                assert!(old.enqueue_record(&ticket, 21).is_err());
            }
            clean_dirs(dir.path());
            drop(old);
            drop(host);
            match fault {
                PublishFault::BeforeAnchor => {
                    assert!(open(dir.path(), &key).unwrap().status().installed)
                }
                PublishFault::AfterPolicy => {
                    assert!(!open(dir.path(), &key).unwrap().status().installed)
                }
                _ => assert!(open(dir.path(), &key).is_err()),
            }
        }
    }

    #[test]
    fn actual_vm_guard_and_commit_hot_paths_are_zero_alloc_with_positive_control() {
        use crate::specialized_libs::computational_geometry::allocation_counter::{
            assert_zero_alloc, AllocGuard,
        };
        let control = AllocGuard::begin("governed positive control", true);
        let allocated = vec![7u8; 1024];
        std::hint::black_box(&allocated);
        assert!(control.check().is_err());
        drop(allocated);
        for mode in 0..4 {
            let dir = tempfile::tempdir().unwrap();
            let key = isolated_key();
            let mut host = open(dir.path(), &key).unwrap();
            let mut rules = explicit_permissive_document().rules;
            if mode == 1 {
                rules.program =
                    PolicyList::from_vec(vec![ConstraintInstruction::MatchObject { value: 99 }])
                        .unwrap();
            }
            if mode == 2 {
                rules.required_claim = 99;
            }
            install(&mut host, &key, rules);
            let mut session = host.admit_at(&key, admission(), 20, BUDGET).unwrap();
            assert_zero_alloc("real governed VM/graph decision", || {
                if mode == 3 {
                    host.disconnect();
                    assert!(session.prepare_at(proposal(), 20).is_err());
                    return;
                }
                let ticket = session.prepare_at(proposal(), 20).unwrap();
                if mode == 0 {
                    session.enqueue_record(&ticket, 20).unwrap();
                } else {
                    assert!(session.enqueue_record(&ticket, 20).is_err());
                }
            });
        }
    }
    #[test]
    fn all_actor_kinds_and_identifiers_require_independent_installer_authority() {
        for kind in [
            PrincipalKind::Human,
            PrincipalKind::SoftwareAgent,
            PrincipalKind::Organization,
        ] {
            let dir = tempfile::tempdir().unwrap();
            let installer = isolated_key();
            let owner = actor_owner(dir.path(), kind);
            let first = association(&owner, kind, 0);
            let second = association(&owner, kind, 1);
            let mut rules = explicit_permissive_document().rules;
            rules.actors = PolicyList::from_vec(vec![
                ActorPolicyScope::from_evidence(first.evidence()),
                ActorPolicyScope::from_evidence(second.evidence()),
            ])
            .unwrap();
            let mut host = NativePolicyHost::open(
                &dir.path().join("policy"),
                &dir.path().join("authority"),
                trusted(&installer),
                BUDGET,
            )
            .unwrap();
            let actor_key = SigningKey::from_bytes(&[23; 32]);
            assert_eq!(
                host.install(&actor_key, rules.clone(), 10, 100, 20, 0, BUDGET)
                    .unwrap_err(),
                PolicyError::WrongAuthority
            );
            host.install(&installer, rules, 10, 100, 20, 0, BUDGET)
                .unwrap();
            for actor in [first, second] {
                let mut session = host
                    .admit_at(
                        &installer,
                        admission(),
                        20,
                        BUDGET,
                        Arc::clone(&owner),
                        actor,
                    )
                    .unwrap();
                let decision = session.prepare_at(proposal(), 20).unwrap();
                assert_eq!(decision.decision(), Decision::Allow);
                let mut output = None;
                session.accept_into_at(&decision, 20, &mut output).unwrap();
                assert_eq!(output.unwrap().proposal.token, 13);
                assert!(!session.is_terminal());
            }
        }
    }

    #[test]
    fn every_actor_scope_field_is_required_at_live_admission() {
        for field in 0..10 {
            let dir = tempfile::tempdir().unwrap();
            let key = isolated_key();
            let owner = actor_owner(dir.path(), PrincipalKind::Human);
            let actor = association(&owner, PrincipalKind::Human, 0);
            let mut scoped = ActorPolicyScope::from_evidence(actor.evidence());
            match field {
                0 => scoped.subject ^= 1,
                1 => scoped.kind = PrincipalKind::Organization,
                2 => scoped.controller[0] ^= 1,
                3 => scoped.project_digest[0] ^= 1,
                4 => scoped.selected_identifier_digest[0] ^= 1,
                5 => scoped.association_digest[0] ^= 1,
                6 => scoped.admission_digest[0] ^= 1,
                7 => scoped.admission_sequence += 1,
                8 => scoped.authority_generation += 1,
                _ => scoped.valid_until -= 1,
            }
            let mut rules = explicit_permissive_document().rules;
            rules.actors = PolicyList::from_vec(vec![scoped]).unwrap();
            let mut host = NativePolicyHost::open(
                &dir.path().join("policy"),
                &dir.path().join("authority"),
                trusted(&key),
                BUDGET,
            )
            .unwrap();
            host.install(&key, rules, 10, 100, 20, 0, BUDGET).unwrap();
            assert!(
                host.admit_at(&key, admission(), 20, BUDGET, Arc::clone(&owner), actor)
                    .is_err(),
                "field {field}"
            );
        }
    }

    #[test]
    fn record_capacity_enqueue_failure_and_unwind_are_terminal_without_retry() {
        for mode in 0..4 {
            let dir = tempfile::tempdir().unwrap();
            let key = isolated_key();
            let mut host = open(dir.path(), &key).unwrap();
            install(&mut host, &key, explicit_permissive_document().rules);
            let mut session = host.admit_at(&key, admission(), 20, BUDGET).unwrap();
            let ticket = session.prepare_at(proposal(), 20).unwrap();
            if mode == 0 {
                let mut occupied = Some(AcceptedToken {
                    proposal: proposal(),
                    key: ticket.key(),
                });
                let old = occupied.unwrap().key;
                assert_eq!(
                    session.accept_into_at(&ticket, 20, &mut occupied),
                    Err(PolicyError::Capacity)
                );
                assert_eq!(occupied.unwrap().key, old);
            } else if mode == 1 {
                let called = std::cell::Cell::new(false);
                assert_eq!(
                    session.accept_with_at(&ticket, 20, || {
                        called.set(true);
                        Err(PolicyError::Capacity)
                    }),
                    Err(PolicyError::Capacity)
                );
                assert!(called.get());
            } else if mode == 2 {
                assert!(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| session
                        .accept_with_at(&ticket, 20, || panic!(
                            "injected bounded enqueue unwind"
                        ))))
                    .is_err()
                );
            } else {
                let mut output = None;
                session.accept_into_at(&ticket, 20, &mut output).unwrap();
                assert!(output.take().is_some());
                assert!(session.accept_into_at(&ticket, 20, &mut output).is_err());
                assert!(output.is_none());
            }
            assert!(session.is_terminal());
            let mut retry = None;
            assert!(session.accept_into_at(&ticket, 21, &mut retry).is_err());
            assert!(retry.is_none());
            assert!(session.prepare_at(proposal(), 21).is_err());
        }
    }

    #[test]
    fn disconnect_linearizes_after_inflight_enqueue_and_before_any_later_acceptance() {
        let dir = tempfile::tempdir().unwrap();
        let key = isolated_key();
        let mut host = open(dir.path(), &key).unwrap();
        install(&mut host, &key, explicit_permissive_document().rules);
        let mut session = host.admit_at(&key, admission(), 20, BUDGET).unwrap();
        let ticket = session.prepare_at(proposal(), 20).unwrap();
        let host = Arc::new(host);
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (attempt_tx, attempt_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            let accept = scope.spawn(move || {
                let result = session.accept_with_at(&ticket, 20, || {
                    entered_tx.send(()).unwrap();
                    release_rx
                        .recv_timeout(std::time::Duration::from_secs(3))
                        .unwrap();
                    Ok(())
                });
                (session, result)
            });
            entered_rx
                .recv_timeout(std::time::Duration::from_secs(3))
                .unwrap();
            let revoke_host = Arc::clone(&host);
            let revoke = scope.spawn(move || {
                attempt_tx.send(()).unwrap();
                revoke_host.disconnect();
                done_tx.send(()).unwrap();
            });
            attempt_rx
                .recv_timeout(std::time::Duration::from_secs(3))
                .unwrap();
            let waited = done_rx
                .recv_timeout(std::time::Duration::from_millis(50))
                .is_err();
            release_tx.send(()).unwrap();
            let (mut session, result) = accept.join().unwrap();
            revoke.join().unwrap();
            assert!(
                waited,
                "disconnect returned before accepted enqueue completed"
            );
            assert!(result.is_ok());
            let mut next = proposal();
            next.position += 1;
            assert!(session.prepare_at(next, 21).is_err());
        });
    }
    #[test]
    fn public_token_boundary_uses_host_clock_and_cannot_replay_expired_policy_time() {
        let dir = tempfile::tempdir().unwrap();
        let key = isolated_key();
        let mut host = open(dir.path(), &key).unwrap();
        install(&mut host, &key, explicit_permissive_document().rules);
        let actor = association(&host.owner, PrincipalKind::Human, 0);
        let owner = Arc::clone(&host.owner);
        assert!(host
            .host
            .admit(&key, admission(), BUDGET, owner, actor)
            .is_err());
        let mut first = host.admit_at(&key, admission(), 20, BUDGET).unwrap();
        assert!(first.prepare(proposal()).is_err());
        assert!(first.is_terminal());
        let mut second = host.admit_at(&key, admission(), 20, BUDGET).unwrap();
        let ticket = second.prepare_at(proposal(), 20).unwrap();
        let mut output = None;
        assert!(second.accept_into(&ticket, &mut output).is_err());
        assert!(output.is_none());
        assert!(second.is_terminal());
    }

    #[test]
    fn live_session_and_membership_are_rechecked_after_policy_wait() {
        for expiry_case in 0..3 {
            let membership_expires = expiry_case != 0;
            let dir = tempfile::tempdir().unwrap();
            let other_dir = tempfile::tempdir().unwrap();
            let key = isolated_key();
            let mut snapshot = actor_snapshot(PrincipalKind::Human);
            if membership_expires {
                snapshot.subjects[0].identifiers[0].valid_until_unix = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 3;
            }
            if expiry_case == 2 {
                let mut successor = snapshot.subjects[0].identifiers[0].clone();
                successor.scheme = "next-qualified-association".into();
                successor.issuer = "next-admitted-issuer".into();
                successor.evidence_ref = "membership-continuation".into();
                successor.valid_from_unix = successor.valid_until_unix;
                successor.valid_until_unix = 4_000_000_000;
                snapshot.subjects[0].identifiers.push(successor);
            }
            let owner = owner_from_snapshot(dir.path(), snapshot.clone());
            let other = owner_from_snapshot(other_dir.path(), snapshot.clone());
            let holder = association(&owner, PrincipalKind::Human, 1);
            let challenge = other
                .begin_session("actor", if membership_expires { 300 } else { 3 })
                .unwrap();
            let mut signature = [0; 64];
            crate::crypto::network::ed25519::sign_with_context(
                &[23; 32],
                challenge.message_digest(),
                identity::SESSION_CONTEXT,
                &mut signature,
            )
            .unwrap();
            let short = other.complete_session(challenge, &signature).unwrap();
            let id = &snapshot.subjects[0].identifiers[1];
            let actor = other
                .bind_inference_association(
                    &short,
                    IdentifierSelection {
                        value: &id.value,
                        scheme: &id.scheme,
                        issuer: &id.issuer,
                        project_id: &id.project_id,
                        evidence_ref: &id.evidence_ref,
                    },
                )
                .unwrap();
            let deadline = if membership_expires {
                snapshot.subjects[0].identifiers[0].valid_until_unix
            } else {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 3
            };
            assert!(actor.evidence().valid_until > deadline);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let mut rules = explicit_permissive_document().rules;
            rules.actors =
                PolicyList::from_vec(vec![ActorPolicyScope::from_evidence(holder.evidence())])
                    .unwrap();
            let mut host = NativePolicyHost::open(
                &dir.path().join("policy"),
                &dir.path().join("authority"),
                trusted(&key),
                BUDGET,
            )
            .unwrap();
            host.install(&key, rules, now - 1, now + 100, now, 0, BUDGET)
                .unwrap();
            let mut first = host
                .admit(&key, admission(), BUDGET, Arc::clone(&owner), holder)
                .unwrap();
            let mut second = host
                .admit(&key, admission(), BUDGET, Arc::clone(&other), actor)
                .unwrap();
            let a = first.prepare(proposal()).unwrap();
            let b = second.prepare(proposal()).unwrap();
            let (entered_tx, entered_rx) = std::sync::mpsc::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            std::thread::scope(|scope| {
                let blocker = scope.spawn(move || {
                    first.accept_with_at(
                        &a,
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        || {
                            entered_tx.send(()).unwrap();
                            release_rx
                                .recv_timeout(std::time::Duration::from_secs(5))
                                .unwrap();
                            Ok(())
                        },
                    )
                });
                entered_rx
                    .recv_timeout(std::time::Duration::from_secs(2))
                    .unwrap();
                let waiting = scope.spawn(move || {
                    let mut out = None;
                    let result = second.accept_into(&b, &mut out);
                    (second, result, out)
                });
                let started = std::time::Instant::now();
                while other.publication_is_current(&short, 1)
                    && started.elapsed() < std::time::Duration::from_millis(500)
                {
                    std::thread::yield_now();
                }
                let acquired_owner = !other.publication_is_current(&short, 1);
                while std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    < deadline
                {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                release_tx.send(()).unwrap();
                assert!(blocker.join().unwrap().is_ok());
                let (session, result, out) = waiting.join().unwrap();
                assert!(
                    acquired_owner,
                    "acceptance did not reach owner-held policy wait"
                );
                if expiry_case == 2 {
                    assert!(result.is_ok());
                    assert!(out.is_some());
                    assert!(!session.is_terminal());
                } else {
                    assert!(result.is_err());
                    assert!(out.is_none());
                    assert!(session.is_terminal());
                }
            });
        }
    }
}
