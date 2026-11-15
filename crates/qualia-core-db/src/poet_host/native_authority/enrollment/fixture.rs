//! Isolated software-agent fixtures. Never opens the machine credential store.
use super::{super::*, *};
use qualia_cooperative_core::projects::{Project, ProjectMembership, ProjectRole};
pub(super) const WRAP: [u8; 32] = [67; 32];
pub(super) struct Fixture {
    pub owner: NativeAuthority,
    pub authority: NativeSigner,
    pub steward: NativeSigner,
    pub candidate: NativeSigner,
    pub temp: tempfile::TempDir,
}
pub(super) fn document() -> DocumentRevision {
    DocumentRevision {
        project_id: "fixture-project".into(),
        document_id: "fixture-document".into(),
        source_id: "fixture-source".into(),
        revision: 1,
        source_digest: [8; 32],
        author_subject: None,
    }
}
pub(super) fn subject(name: &str, key: [u8; 32], role: ProjectRole) -> SubjectAdmission {
    let now = now().unwrap();
    SubjectAdmission {
        subject: name.into(),
        kind: PrincipalKind::SoftwareAgent,
        controller_public_key: key,
        membership: ProjectMembership {
            id: format!("membership/{name}"),
            project_id: "fixture-project".into(),
            member_did: format!("urn:fixture:{name}"),
            role,
            agreed_at_unix: 1,
            agreed_at_instant: None,
        },
        identifiers: vec![IdentifierAssociationEvidence {
            value: format!("urn:fixture:{name}"),
            scheme: "urn:fixture".into(),
            issuer: "fixture-issuer".into(),
            project_id: "fixture-project".into(),
            evidence_ref: format!("fixture-association/{name}"),
            valid_from_unix: now - 10,
            valid_until_unix: now + 3600,
        }],
        reviews: Vec::new(),
        admission_ref: format!("fixture-admission/{name}"),
        valid_from_unix: now - 10,
        valid_until_unix: now + 3600,
        revoked: false,
    }
}
pub(super) fn anchor(authority: &NativeSigner) -> TrustedProject {
    TrustedProject {
        project_id: "fixture-project".into(),
        issuer_identifier: "fixture-issuer".into(),
        public_key: authority.public_key(),
    }
}
pub(super) fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let authority =
        NativeSigner::create_sealed(&temp.path().join("authority.sealed"), &WRAP).unwrap();
    let steward = NativeSigner::create_sealed(&temp.path().join("steward.sealed"), &WRAP).unwrap();
    let candidate =
        NativeSigner::create_sealed(&temp.path().join("candidate.sealed"), &WRAP).unwrap();
    let now = now().unwrap();
    let snapshot = AdmissionSnapshot {
        version: 1,
        sequence: 1,
        issuer_identifier: "fixture-issuer".into(),
        project: Project {
            id: "fixture-project".into(),
            name: "Software-agent enrollment fixture".into(),
            description: String::new(),
            licensing_ontologies: Vec::new(),
            created_at_unix: 1,
            created_at_instant: None,
        },
        valid_from_unix: now - 10,
        valid_until_unix: now + 3600,
        revoked: false,
        package_reviews: Vec::new(),
        subjects: vec![subject(
            "steward",
            steward.public_key(),
            ProjectRole::Steward,
        )],
    };
    let journal = temp.path().join("journal");
    std::fs::create_dir(&journal).unwrap();
    let owner = NativeAuthority::open(
        &journal,
        anchor(&authority),
        authority.sign_admission(snapshot).unwrap(),
        document(),
        1024 * 1024,
        256,
    )
    .unwrap();
    Fixture {
        owner,
        authority,
        steward,
        candidate,
        temp,
    }
}
pub(super) fn login(
    owner: &NativeAuthority,
    subject: &str,
    signer: &NativeSigner,
) -> SessionHandle {
    let challenge = owner.begin_session(subject, 300).unwrap();
    let proof = signer.prove_session(&challenge).unwrap();
    owner.complete_session(challenge, &proof).unwrap()
}
pub(super) fn permission(f: &Fixture) -> EnrollmentPermission {
    let session = login(&f.owner, "steward", &f.steward);
    f.owner.enrollment_permission(&session).unwrap()
}
pub(super) fn request(f: &Fixture) -> EnrollmentRequest {
    let admission = f.owner.enrollment_snapshot().unwrap();
    let now = now().unwrap();
    EnrollmentRequest::signed(
        "fixture-project".into(),
        admission.snapshot.sequence,
        admission_digest(&admission.snapshot, 1024 * 1024).unwrap(),
        now - 1,
        now + 300,
        subject(
            "contributor",
            f.candidate.public_key(),
            ProjectRole::Contributor,
        ),
        &f.candidate,
    )
    .unwrap()
}
pub(super) fn approval() -> EnrollmentApproval {
    EnrollmentApproval {
        decision_evidence_ref: "fixture-software-agent-operator-decision".into(),
        human_verification_ref: None,
    }
}
