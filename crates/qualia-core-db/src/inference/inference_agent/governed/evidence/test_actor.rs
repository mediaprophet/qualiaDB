// Actual signed NativeAuthority fixture retained from the V2 author tests.
use super::super::ActorPolicyScope;
#[cfg(not(target_arch = "wasm32"))]
use crate::poet_host::native_authority::{
    inference::{AuthenticatedInferenceAssociation, IdentifierSelection},
    NativeAuthority, SessionHandle,
};
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn bind(
    owner: &NativeAuthority,
    handle: &SessionHandle,
) -> AuthenticatedInferenceAssociation {
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
pub(super) fn actor(
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
