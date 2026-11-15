use super::*;
use ed25519_dalek::SigningKey;
use etalon_inference_contract::{Decision, Instrumentation, Phase};

fn rules() -> PolicyRules {
    PolicyRules {
        actors: PolicyList::from_vec(vec![ActorPolicyScope {
            subject: 9,
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
        graph_scopes: PolicyList::default(),
        program: PolicyList::from_vec(vec![ConstraintInstruction::MatchObject { value: 7 }])
            .unwrap(),
        norms: PolicyList::default(),
        facts: PolicyList::default(),
        epistemic: PolicyList::default(),
        trace: PolicyList::default(),
        required_claim: 0,
        temporal_invariant: 0,
        non_derogable: true,
    }
}
fn request() -> InferenceAdmission {
    InferenceAdmission {
        request_digest: [1; 32],
        model_digest: [2; 32],
        model_generation: 1,
        device_id: 1,
        device_generation: 1,
        graph_scope: 44,
        first_position: 10,
        instrumentation: Instrumentation::default(),
    }
}
fn proposal(value: u64, position: u64) -> TokenProposal {
    TokenProposal {
        token: value as u32,
        position,
        logit: 0.5,
        observation: crate::NQuin {
            subject: 9,
            predicate: 12,
            object: value,
            context: 44,
            metadata: 0,
            parity: 9 ^ 12 ^ value ^ 44,
        },
    }
}
fn trust(key: &SigningKey) -> TrustedPolicyIdentity {
    TrustedPolicyIdentity::from_host_configuration(
        key.verifying_key().to_bytes(),
        9,
        INFERENCE_POLICY_AUDIENCE,
        1,
    )
    .unwrap()
}

#[test]
fn signature_binds_every_policy_component_and_configured_identity() {
    let key = SigningKey::from_bytes(&[31; 32]);
    let document = PolicyDocument {
        version: POLICY_VERSION,
        audience: INFERENCE_POLICY_AUDIENCE,
        principal: 9,
        controller_generation: 1,
        policy_generation: 1,
        not_before: 10,
        expires_at: 50,
        rules: rules(),
    };
    let policy = SignedPolicy::sign(document, &key).unwrap();
    assert!(policy.verify(&trust(&key), 10).is_ok());
    assert_eq!(policy.verify(&trust(&key), 50), Err(PolicyError::Expired));
    let wrong = SigningKey::from_bytes(&[32; 32]);
    assert_eq!(
        policy.verify(&trust(&wrong), 10),
        Err(PolicyError::InvalidSignature)
    );
    let mut changed = policy.clone();
    changed.document.rules.non_derogable = false;
    assert_eq!(
        changed.verify(&trust(&key), 10),
        Err(PolicyError::InvalidSignature)
    );
    let mut changed = policy;
    changed.document.rules.program = PolicyList::default();
    assert_eq!(
        changed.verify(&trust(&key), 10),
        Err(PolicyError::InvalidSignature)
    );
}

#[test]
fn bounded_wire_rejects_truncation_unknown_and_invalid_registers() {
    assert!(PolicyList::<u8, 2>::from_vec(Vec::with_capacity(3)).is_err());
    assert!(serde_json::from_str::<PolicyList<u8, 2>>("[1,2,3]").is_err());
    assert!(
        serde_json::from_str::<ConstraintInstruction>(r#"{"op":"LoadModel","value":1}"#).is_err()
    );
    assert!(ConstraintInstruction::Bind {
        vector: 0,
        register: 16
    }
    .lower()
    .is_err());
    assert!(ConstraintInstruction::GreaterThan {
        vector: 4,
        value: 1.0
    }
    .lower()
    .is_err());
    assert!(ConstraintInstruction::Confidence { minimum: f32::NAN }
        .lower()
        .is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn installed_vm_decision_and_persisted_revocation_are_real() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[33; 32]);
    let mut host = NativePolicyHost::open(
        &dir.path().join("policy"),
        &dir.path().join("authority"),
        trust(&key),
        POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
    let (owner, handle, scope) = actor(&dir.path().join("actors"));
    assert!(matches!(
        host.admit_at(
            &key,
            request(),
            20,
            POLICY_COLD_BUDGET_BYTES,
            std::sync::Arc::clone(&owner),
            bind(&owner, &handle)
        ),
        Err(PolicyError::MissingPolicy)
    ));
    host.install(
        &key,
        actor_rules(scope),
        10,
        50,
        20,
        0,
        POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
    let mut session = host
        .admit_at(
            &key,
            request(),
            20,
            POLICY_COLD_BUDGET_BYTES,
            std::sync::Arc::clone(&owner),
            bind(&owner, &handle),
        )
        .unwrap();
    let accepted = session.prepare_at(proposal(7, 10), 20).unwrap();
    assert_eq!(accepted.decision(), Decision::Allow);
    session.accept_into_at(&accepted, 20, &mut None).unwrap();
    let denied = session.prepare_at(proposal(8, 11), 21).unwrap();
    assert_eq!(denied.decision(), Decision::Deny);
    assert!(session.accept_into_at(&denied, 21, &mut None).is_err());
    assert_eq!(session.snapshot().phase, Phase::AbortRequired);
    host.revoke(&key, 1, POLICY_COLD_BUDGET_BYTES).unwrap();
    drop(host);
    let mut reloaded = NativePolicyHost::open(
        &dir.path().join("policy"),
        &dir.path().join("authority"),
        trust(&key),
        POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
    assert!(!reloaded.status().installed);
    assert!(matches!(
        reloaded.admit_at(
            &key,
            request(),
            22,
            POLICY_COLD_BUDGET_BYTES,
            std::sync::Arc::clone(&owner),
            bind(&owner, &handle)
        ),
        Err(PolicyError::MissingPolicy)
    ));
    assert_eq!(
        std::fs::read_dir(dir.path().join("policy"))
            .unwrap()
            .count(),
        2
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn explicit_permissive_policy_is_allowed_but_disconnect_cannot_be_undone_for_old_session() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[34; 32]);
    let mut host = NativePolicyHost::open(
        &dir.path().join("policy"),
        &dir.path().join("authority"),
        trust(&key),
        POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
    let (owner, handle, scope) = actor(&dir.path().join("actors"));
    let mut permissive = actor_rules(scope);
    permissive.program = PolicyList::default();
    host.install(&key, permissive, 10, 50, 20, 0, POLICY_COLD_BUDGET_BYTES)
        .unwrap();
    let mut session = host
        .admit_at(
            &key,
            request(),
            20,
            POLICY_COLD_BUDGET_BYTES,
            std::sync::Arc::clone(&owner),
            bind(&owner, &handle),
        )
        .unwrap();
    let ticket = session.prepare_at(proposal(8, 10), 20).unwrap();
    assert_eq!(ticket.decision(), Decision::Allow);
    host.disconnect();
    host.reload(POLICY_COLD_BUDGET_BYTES).unwrap();
    assert!(session.accept_into_at(&ticket, 21, &mut None).is_err());
    let mut new_session = host
        .admit_at(
            &key,
            request(),
            21,
            POLICY_COLD_BUDGET_BYTES,
            std::sync::Arc::clone(&owner),
            bind(&owner, &handle),
        )
        .unwrap();
    assert!(new_session.prepare_at(proposal(8, 10), 21).is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
use crate::poet_host::native_authority::{
    inference::{AuthenticatedInferenceAssociation, IdentifierSelection},
    NativeAuthority, SessionHandle,
};
#[cfg(not(target_arch = "wasm32"))]
fn bind(owner: &NativeAuthority, handle: &SessionHandle) -> AuthenticatedInferenceAssociation {
    owner
        .bind_inference_association(
            handle,
            IdentifierSelection {
                value: "urn:actor:one",
                scheme: "urn",
                issuer: "issuer",
                project_id: "project",
                evidence_ref: "evidence",
            },
        )
        .unwrap()
}
#[cfg(not(target_arch = "wasm32"))]
fn actor_rules(scope: ActorPolicyScope) -> PolicyRules {
    let mut r = rules();
    r.actors = PolicyList::from_vec(vec![scope]).unwrap();
    r
}
#[cfg(not(target_arch = "wasm32"))]
fn actor(
    path: &std::path::Path,
) -> (
    std::sync::Arc<NativeAuthority>,
    SessionHandle,
    ActorPolicyScope,
) {
    use crate::crypto::network::ed25519;
    use crate::poet_host::native_authority::*;
    use qualia_cooperative_core::{authoring::*, projects::*};
    std::fs::create_dir_all(path).unwrap();
    let issuer = [71; 32];
    let controller = [72; 32];
    let snapshot = AdmissionSnapshot {
        version: 1,
        sequence: 1,
        issuer_identifier: "issuer".into(),
        project: Project {
            id: "project".into(),
            name: "test".into(),
            description: String::new(),
            licensing_ontologies: vec![],
            created_at_unix: 1,
            created_at_instant: None,
        },
        valid_from_unix: 1,
        valid_until_unix: 4_000_000_000,
        revoked: false,
        package_reviews: vec![],
        subjects: vec![SubjectAdmission {
            subject: "actor".into(),
            kind: PrincipalKind::Human,
            controller_public_key: ed25519::public_from_seed(&controller),
            membership: ProjectMembership {
                id: "membership".into(),
                project_id: "project".into(),
                member_did: "urn:actor:one".into(),
                role: ProjectRole::Contributor,
                agreed_at_unix: 1,
                agreed_at_instant: None,
            },
            identifiers: vec![IdentifierAssociationEvidence {
                value: "urn:actor:one".into(),
                scheme: "urn".into(),
                issuer: "issuer".into(),
                project_id: "project".into(),
                evidence_ref: "evidence".into(),
                valid_from_unix: 1,
                valid_until_unix: 4_000_000_000,
            }],
            reviews: vec![],
            admission_ref: "admitted".into(),
            valid_from_unix: 1,
            valid_until_unix: 4_000_000_000,
            revoked: false,
        }],
    };
    let digest = admission_digest(&snapshot, 1024 * 1024).unwrap();
    let mut signature = [0; 64];
    ed25519::sign_with_context(&issuer, &digest, ADMISSION_CONTEXT, &mut signature).unwrap();
    let owner = std::sync::Arc::new(
        NativeAuthority::open(
            path,
            TrustedProject {
                project_id: "project".into(),
                issuer_identifier: "issuer".into(),
                public_key: ed25519::public_from_seed(&issuer),
            },
            SignedAdmission {
                snapshot,
                signature: signature.to_vec(),
            },
            DocumentRevision {
                project_id: "project".into(),
                document_id: "document".into(),
                source_id: "source".into(),
                revision: 1,
                source_digest: [1; 32],
                author_subject: Some("actor".into()),
            },
            1024 * 1024,
            256,
        )
        .unwrap(),
    );
    let challenge = owner.begin_session("actor", 300).unwrap();
    ed25519::sign_with_context(
        &controller,
        challenge.message_digest(),
        SESSION_CONTEXT,
        &mut signature,
    )
    .unwrap();
    let handle = owner.complete_session(challenge, &signature).unwrap();
    let scope = ActorPolicyScope::from_evidence(bind(&owner, &handle).evidence());
    (owner, handle, scope)
}
