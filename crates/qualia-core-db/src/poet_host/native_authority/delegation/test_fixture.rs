//! Synthetic signed controller/capacity fixtures; no real-person evidence.
use super::super::super::*;
use qualia_cooperative_core::{
    agency_delegation::{AgencyDelegation, ConsentState},
    authoring::delegation::*,
    authority_type::{ids, AuthorityProfile},
};

use qualia_cooperative_core::projects::{Project, ProjectMembership, ProjectRole};

pub(super) const ROOT_SEED: [u8; 32] = [23; 32];
pub(super) const AGENT_SEED: [u8; 32] = [29; 32];

// Reuse the existing native signed software fixture as source; only the root's
// project-admitted capacity is changed explicitly for this controller diagnostic.
#[path = "../session_test_fixture.rs"]
mod original;

pub(super) fn snapshot() -> AdmissionSnapshot {
    let mut s = original::snapshot();
    s.subjects[0].kind = PrincipalKind::Human;
    let mut leaf = s.subjects[1].clone();
    leaf.subject = "leaf".into();
    leaf.controller_public_key = ed25519::public_from_seed(&[31; 32]);
    leaf.membership.id = "membership/leaf".into();
    leaf.membership.member_did = "did:test:leaf".into();
    leaf.admission_ref = "adoption/leaf".into();
    leaf.identifiers.truncate(1);
    leaf.identifiers[0].value = "did:test:leaf".into();
    s.subjects.push(leaf);
    s
}
pub(super) fn owner(path: &Path, s: AdmissionSnapshot) -> NativeAuthority {
    original::open(path, s)
}
pub(super) fn login_root(owner: &NativeAuthority) -> SessionHandle {
    original::login(owner, "author", &ROOT_SEED)
}
pub(super) fn authority_generation(owner: &NativeAuthority, session: &SessionHandle) -> u64 {
    original::generation(owner, session)
}
pub(super) fn root_digest(s: &AdmissionSnapshot) -> [u8; 32] {
    admission_digest(s, 1024 * 1024).unwrap()
}
pub(super) fn record(s: &AdmissionSnapshot) -> AuthoringDelegation {
    let mut agency = AgencyDelegation::new(
        "did:test:author",
        qualia_cooperative_core::agency_domain::ids::AI_PROXY,
        "urn:test:values-reference",
        1,
    );
    agency.id = "root-grant".into();
    agency.agent_dids = vec!["did:test:reviewer".into()];
    agency.authority = AuthorityProfile::new(
        ids::MOD_AUTOMATED,
        ids::TRIG_PERSISTENT,
        ids::ACCT_VALUES_BOUND,
    );
    agency.valid_to_unix = Some(s.valid_until_unix as u32);
    agency.consent = ConsentState::Granted;
    agency.evidence_ref = Some("urn:test:explicit-signed-controller-consent".into());
    agency.scope = vec![
        ("purpose".into(), "edit-source".into()),
        ("action".into(), "material_edit".into()),
        ("data_class".into(), AUTHORING_DATA_CLASS.into()),
    ];
    AuthoringDelegation {
        version: 1,
        root_subject: "author".into(),
        root_adoption_digest: root_digest(s),
        project_id: "project".into(),
        document_id: "document".into(),
        source_id: "source".into(),
        grantor_subject: "author".into(),
        grantor_controller: s.subjects[0].controller_public_key,
        grantee_subject: "reviewer".into(),
        grantee_controller: s.subjects[1].controller_public_key,
        parent_digest: None,
        remaining_depth: 1,
        actions: vec![AuthoringAction::MaterialEdit],
        agency,
        task: TaskDelegation {
            delegation_id: "root-grant".into(),
            principal_id: "author".into(),
            actor_id: "reviewer".into(),
            app_id: AppId::new("poet").unwrap(),
            goal: "edit-source".into(),
            allowed_effects: vec![CommandEffect::DocumentMutation],
            allowed_command_ids: vec!["poet::edit".into()],
            mode: DelegationMode::BoundedDelegation,
            state: DelegationLifecycle::Approved,
            budgets: ResourceBudgets {
                max_steps: 10,
                max_cpu_time_ms: 100,
                max_memory_bytes: 1024,
                external_cost_minor_units: None,
            },
            expiry_timestamp_ms: DecimalU64::new(s.valid_until_unix * 1000),
        },
    }
}
pub(super) fn signed(d: AuthoringDelegation, seed: &[u8; 32]) -> SignedAuthoringDelegation {
    let digest = super::super::verify::authoring_delegation_digest(&d).unwrap();
    let mut signature = [0; 64];
    ed25519::sign_with_context(
        seed,
        &digest,
        super::super::verify::AUTHORING_DELEGATION_CONTEXT,
        &mut signature,
    )
    .unwrap();
    SignedAuthoringDelegation {
        delegation: d,
        signature: signature.to_vec(),
    }
}
pub(super) fn child(
    s: &AdmissionSnapshot,
    parent: &SignedAuthoringDelegation,
) -> SignedAuthoringDelegation {
    let mut c = parent.delegation.clone();
    c.parent_digest =
        Some(super::super::verify::authoring_delegation_digest(&parent.delegation).unwrap());
    c.grantor_subject = "reviewer".into();
    c.grantor_controller = s.subjects[1].controller_public_key;
    c.grantee_subject = "leaf".into();
    c.grantee_controller = s.subjects[2].controller_public_key;
    c.remaining_depth = 0;
    c.agency.id = "child-grant".into();
    c.agency.agent_dids = vec!["did:test:leaf".into()];
    c.task.delegation_id = "child-grant".into();
    c.task.actor_id = "leaf".into();
    signed(c, &AGENT_SEED)
}
