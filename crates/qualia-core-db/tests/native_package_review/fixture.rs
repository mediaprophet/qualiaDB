//! All identities here are software fixtures; native credential APIs are unused.
pub use qualia_cooperative_core::{
    authoring::*,
    projects::{Project, ProjectMembership, ProjectRole},
};
pub use qualia_core_db::{
    crypto::network::ed25519,
    poet_host::native_authority::{package_review::*, *},
};
pub use sha2::{Digest, Sha256};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
pub const AUTHORITY: [u8; 32] = [17; 32];
pub const AUTHOR: [u8; 32] = [23; 32];
pub const REVIEWER: [u8; 32] = [29; 32];
pub fn now() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| e.to_string())
}
pub fn document() -> DocumentRevision {
    DocumentRevision {
        project_id: "project".into(),
        document_id: "document".into(),
        source_id: "source".into(),
        revision: 1,
        source_digest: [1; 32],
        author_subject: Some("author".into()),
    }
}
pub fn snapshot() -> AdmissionSnapshot {
    let until = now().unwrap() + 3600;
    let subject = |name: &str, seed: &[u8; 32]| SubjectAdmission {
        subject: name.into(),
        kind: PrincipalKind::SoftwareAgent,
        controller_public_key: ed25519::public_from_seed(seed),
        membership: ProjectMembership {
            id: format!("urn:fixture:membership:{name}"),
            project_id: "project".into(),
            member_did: format!("urn:fixture:actor:{name}"),
            role: ProjectRole::Contributor,
            agreed_at_unix: 1,
            agreed_at_instant: None,
        },
        identifiers: vec![IdentifierAssociationEvidence {
            value: format!("urn:fixture:actor:{name}"),
            scheme: "urn".into(),
            issuer: "fixture-authority".into(),
            project_id: "project".into(),
            evidence_ref: format!("urn:fixture:association:{name}"),
            valid_from_unix: 1,
            valid_until_unix: until,
        }],
        reviews: vec![ReviewAdmission {
            id: format!("urn:fixture:source-review:{name}"),
            document_id: "document".into(),
            admission_ref: "urn:fixture:source-admission".into(),
            valid_from_unix: 1,
            valid_until_unix: until,
            revoked: false,
        }],
        admission_ref: format!("urn:fixture:admission:{name}"),
        valid_from_unix: 1,
        valid_until_unix: until,
        revoked: false,
    };
    AdmissionSnapshot {
        version: 1,
        sequence: 1,
        issuer_identifier: "fixture-authority".into(),
        project: Project {
            id: "project".into(),
            name: "software fixture".into(),
            description: String::new(),
            licensing_ontologies: vec![],
            created_at_unix: 1,
            created_at_instant: None,
        },
        valid_from_unix: 1,
        valid_until_unix: until,
        revoked: false,
        subjects: vec![subject("author", &AUTHOR), subject("reviewer", &REVIEWER)],
        package_reviews: vec![],
    }
}
pub fn anchor() -> TrustedProject {
    TrustedProject {
        project_id: "project".into(),
        issuer_identifier: "fixture-authority".into(),
        public_key: ed25519::public_from_seed(&AUTHORITY),
    }
}
pub fn sign(snapshot: AdmissionSnapshot) -> SignedAdmission {
    let digest = admission_digest(&snapshot, 1024 * 1024).unwrap();
    let mut signature = [0; 64];
    ed25519::sign_with_context(&AUTHORITY, &digest, ADMISSION_CONTEXT, &mut signature).unwrap();
    SignedAdmission {
        snapshot,
        signature: signature.to_vec(),
    }
}
pub fn open(path: &Path, snapshot: AdmissionSnapshot) -> NativeAuthority {
    NativeAuthority::open(path, anchor(), sign(snapshot), document(), 1024 * 1024, 256).unwrap()
}
pub fn login(owner: &NativeAuthority, subject: &str, seed: &[u8; 32]) -> SessionHandle {
    let challenge = owner.begin_session(subject, 300).unwrap();
    let mut signature = [0; 64];
    ed25519::sign_with_context(
        seed,
        challenge.message_digest(),
        SESSION_CONTEXT,
        &mut signature,
    )
    .unwrap();
    owner.complete_session(challenge, &signature).unwrap()
}
pub fn generation(owner: &NativeAuthority, session: &SessionHandle) -> u64 {
    owner
        .transact(session, |view| DocumentTransaction {
            result: Ok(view.generation),
            successor: None,
            reconciliation_required: false,
        })
        .unwrap()
}
