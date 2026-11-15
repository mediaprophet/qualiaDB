//! Shared signed software-agent fixture; no human enrollment claim.
use super::*;

pub(super) const AUTHORITY: [u8; 32] = [17; 32];
pub(super) const AUTHOR: [u8; 32] = [23; 32];
pub(super) const REVIEWER: [u8; 32] = [29; 32];
pub(super) fn sign(snapshot: AdmissionSnapshot) -> SignedAdmission {
    let digest = admission_digest(&snapshot, 1024 * 1024).unwrap();
    let mut signature = [0; 64];
    ed25519::sign_with_context(&AUTHORITY, &digest, ADMISSION_CONTEXT, &mut signature).unwrap();
    SignedAdmission {
        snapshot,
        signature: signature.to_vec(),
    }
}
pub(super) fn snapshot() -> AdmissionSnapshot {
    let until = now().unwrap() + 3600;
    let subject = |name: &str, seed: &[u8; 32]| SubjectAdmission {
        subject: name.into(),
        kind: PrincipalKind::SoftwareAgent,
        controller_public_key: ed25519::public_from_seed(seed),
        membership: ProjectMembership {
            id: format!("membership/{name}"),
            project_id: "project".into(),
            member_did: format!("did:test:{name}"),
            role: ProjectRole::Contributor,
            agreed_at_unix: 1,
            agreed_at_instant: None,
        },
        identifiers: vec![
            IdentifierAssociationEvidence {
                value: format!("did:test:{name}"),
                scheme: "did:test".into(),
                issuer: "fixture-authority".into(),
                project_id: "project".into(),
                evidence_ref: format!("association/{name}"),
                valid_from_unix: 1,
                valid_until_unix: until,
            },
            IdentifierAssociationEvidence {
                value: format!("urn:alternate:{name}"),
                scheme: "urn".into(),
                issuer: "fixture-alias-issuer".into(),
                project_id: "project".into(),
                evidence_ref: format!("alias/{name}"),
                valid_from_unix: 1,
                valid_until_unix: until,
            },
        ],
        reviews: vec![ReviewAdmission {
            id: format!("review/{name}"),
            document_id: "document".into(),
            admission_ref: "fixture-review-adoption".into(),
            valid_from_unix: 1,
            valid_until_unix: until,
            revoked: false,
        }],
        admission_ref: format!("adoption/{name}"),
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
            name: "fixture project".into(),
            description: String::new(),
            licensing_ontologies: vec![],
            created_at_unix: 1,
            created_at_instant: None,
        },
        valid_from_unix: 1,
        valid_until_unix: until,
        revoked: false,
        subjects: vec![subject("author", &AUTHOR), subject("reviewer", &REVIEWER)],
        package_reviews: Vec::new(),
    }
}
pub(super) fn anchor() -> TrustedProject {
    TrustedProject {
        project_id: "project".into(),
        issuer_identifier: "fixture-authority".into(),
        public_key: ed25519::public_from_seed(&AUTHORITY),
    }
}
pub(super) fn document() -> DocumentRevision {
    DocumentRevision {
        project_id: "project".into(),
        document_id: "document".into(),
        source_id: "source".into(),
        revision: 1,
        source_digest: [1; 32],
        author_subject: Some("author".into()),
    }
}
pub(super) fn open(path: &Path, snapshot: AdmissionSnapshot) -> NativeAuthority {
    NativeAuthority::open(path, anchor(), sign(snapshot), document(), 1024 * 1024, 256).unwrap()
}
pub(super) fn login(owner: &NativeAuthority, subject: &str, seed: &[u8; 32]) -> SessionHandle {
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
pub(super) fn generation(owner: &NativeAuthority, session: &SessionHandle) -> u64 {
    owner
        .transact(session, |view| DocumentTransaction {
            result: Ok(view.generation),
            successor: None,
            reconciliation_required: false,
        })
        .unwrap()
}
