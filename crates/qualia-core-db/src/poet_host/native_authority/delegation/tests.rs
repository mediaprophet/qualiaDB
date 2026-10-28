use super::{super::*, verify::*};
use qualia_cooperative_core::authoring::delegation::*;
#[path = "test_fixture.rs"]
mod fixture;
use fixture::*;

fn checked(
    snapshot: &AdmissionSnapshot,
    chain: &[SignedAuthoringDelegation],
    cancelled: &[[u8; 32]],
) -> Result<CheckedChain, String> {
    verify_chain(
        ControllerRoot {
            subject: "author",
            controller: &snapshot.subjects[0].controller_public_key,
            admission_digest: root_digest(snapshot),
        },
        snapshot,
        chain,
        now().unwrap(),
        cancelled,
    )
}

#[test]
fn authenticated_root_can_inspect_signed_two_edge_controller_consent_without_dispatching() {
    let temp = tempfile::tempdir().unwrap();
    let s = snapshot();
    let owner = owner(temp.path(), s.clone());
    let session = login_root(&owner);
    let generation_before = authority_generation(&owner, &session);
    let parent = signed(record(&s), &ROOT_SEED);
    let leaf = child(&s, &parent);
    let target = owner.workflow_evidence(&session).unwrap().target;
    let op = DelegatedOperation {
        actor_subject: "leaf",
        actor_controller: &s.subjects[2].controller_public_key,
        purpose: "edit-source",
        command_id: "poet::edit",
        action: AuthoringAction::MaterialEdit,
        effects: &[CommandEffect::DocumentMutation],
        target: &target,
        now_unix: 0,
    };
    let result = owner
        .inspect_controller_delegation(&session, &[parent, leaf], &op)
        .unwrap();
    assert_eq!(result.project_admitted_root_subject, "author");
    assert_eq!(result.acting_agent_subject, "leaf");
    assert_eq!(result.delegation_path.len(), 2);
    assert_eq!(result.authority_generation, generation_before);
    assert_eq!(authority_generation(&owner, &session), generation_before);
    // Inspection cannot manufacture adopted authority, mutate source or turn a
    // project capacity tag into verified human ancestry.
    let evidence = owner.workflow_evidence(&session).unwrap();
    assert_eq!(evidence.target, target);
    assert_eq!(evidence.human_authority.human_principal_subject, None);
    assert!(evidence.human_authority.delegation_path.is_empty());

    let mut independent_key = record(&s);
    independent_key.grantee_subject = "leaf".into();
    independent_key.grantee_controller = s.subjects[2].controller_public_key;
    independent_key.agency.agent_dids = vec!["did:test:leaf".into()];
    independent_key.task.actor_id = "leaf".into();
    let same_root = owner
        .inspect_controller_delegation(&session, &[signed(independent_key, &ROOT_SEED)], &op)
        .unwrap();
    assert!(result.has_admitted_control_overlap(&same_root));
    assert_eq!(same_root.authority_generation, generation_before);
    assert_eq!(authority_generation(&owner, &session), generation_before);
}
#[test]
fn project_admission_does_not_replace_controller_consent_signature() {
    let s = snapshot();
    let mut grant = signed(record(&s), &ROOT_SEED);
    assert!(checked(&s, &[grant.clone()], &[]).is_ok());
    grant.signature.fill(0);
    assert!(checked(&s, &[grant], &[]).is_err());
    assert!(checked(&s, &[signed(record(&s), &AGENT_SEED)], &[]).is_err());
    let mut d = record(&s);
    d.project_id = "foreign".into();
    assert!(checked(&s, &[signed(d, &ROOT_SEED)], &[]).is_err());
    let mut d = record(&s);
    d.root_subject = "other".into();
    assert!(checked(&s, &[signed(d, &ROOT_SEED)], &[]).is_err());
}
#[test]
fn ancestor_cancellation_expiry_revocation_and_admission_successor_refuse_the_chain() {
    let s = snapshot();
    let parent = signed(record(&s), &ROOT_SEED);
    let leaf = child(&s, &parent);
    let digest = authoring_delegation_digest(&parent.delegation).unwrap();
    assert!(checked(&s, &[parent.clone(), leaf.clone()], &[digest]).is_err());
    let mut expired = parent.clone();
    expired.delegation.agency.valid_to_unix = Some(now().unwrap() as u32);
    assert!(checked(
        &s,
        &[signed(expired.delegation, &ROOT_SEED), leaf.clone()],
        &[]
    )
    .is_err());
    let mut revoked = s.clone();
    revoked.subjects[1].revoked = true;
    assert!(checked(&revoked, &[parent.clone(), leaf.clone()], &[]).is_err());
    let mut successor = s.clone();
    successor.sequence += 1;
    assert!(checked(&successor, &[parent, leaf], &[]).is_err());
}
#[test]
fn child_cannot_widen_budget_forge_parent_or_cycle_back_to_root() {
    let s = snapshot();
    let p = signed(record(&s), &ROOT_SEED);
    let c = child(&s, &p);
    for case in 0..4 {
        let mut d = c.delegation.clone();
        match case {
            0 => d.task.budgets.max_steps += 1,
            1 => d.parent_digest = Some([8; 32]),
            2 => {
                d.grantee_subject = "author".into();
                d.grantee_controller = s.subjects[0].controller_public_key;
            }
            _ => d.remaining_depth = 1,
        }
        assert!(checked(&s, &[p.clone(), signed(d, &AGENT_SEED)], &[]).is_err());
    }
}
#[test]
fn software_capacity_cannot_be_promoted_to_human_root_by_consent_bytes() {
    let mut s = snapshot();
    s.subjects[0].kind = PrincipalKind::SoftwareAgent;
    assert!(checked(&s, &[signed(record(&s), &ROOT_SEED)], &[]).is_err());
}

#[test]
fn observer_root_cannot_delegate_write_but_can_inspect_explicit_read() {
    let temp = tempfile::tempdir().unwrap();
    let mut s = snapshot();
    s.subjects[0].membership.role = qualia_cooperative_core::projects::ProjectRole::Observer;
    let owner = owner(temp.path(), s.clone());
    let session = login_root(&owner);
    let target = owner.workflow_evidence(&session).unwrap().target;
    let write = signed(record(&s), &ROOT_SEED);
    let mut op = DelegatedOperation {
        actor_subject: "reviewer",
        actor_controller: &s.subjects[1].controller_public_key,
        purpose: "edit-source",
        command_id: "poet::edit",
        action: AuthoringAction::MaterialEdit,
        effects: &[CommandEffect::DocumentMutation],
        target: &target,
        now_unix: 0,
    };
    assert!(owner
        .inspect_controller_delegation(&session, &[write], &op)
        .unwrap_err()
        .contains("RoleDenied"));
    let mut read = record(&s);
    read.actions = vec![AuthoringAction::ReadSource];
    read.agency.scope.retain(|(key, _)| key != "action");
    read.agency
        .scope
        .push(("action".into(), "read_source".into()));
    read.task.allowed_effects = vec![CommandEffect::Read];
    read.task.allowed_command_ids = vec!["poet::read".into()];
    op.action = AuthoringAction::ReadSource;
    op.effects = &[CommandEffect::Read];
    op.command_id = "poet::read";
    let inspected = owner
        .inspect_controller_delegation(&session, &[signed(read, &ROOT_SEED)], &op)
        .unwrap();
    assert_eq!(inspected.acting_agent_subject, "reviewer");
    assert_eq!(owner.workflow_evidence(&session).unwrap().target, target);
}

#[test]
fn inspected_distinct_roots_sharing_an_agent_controller_are_not_independent() {
    let tmp = tempfile::tempdir().unwrap();
    let mut admission = snapshot();
    let second_seed = [37; 32];
    let mut second = admission.subjects[0].clone();
    second.subject = "second-human".into();
    second.controller_public_key = ed25519::public_from_seed(&second_seed);
    second.membership.id = "membership/second-human".into();
    second.membership.member_did = "did:test:second-human".into();
    second.admission_ref = "adoption/second-human".into();
    second.identifiers.truncate(1);
    second.identifiers[0].value = "did:test:second-human".into();
    second.reviews.clear();
    admission.subjects.push(second);
    let owner = owner(tmp.path(), admission.clone());
    let first_session = login_root(&owner);
    let challenge = owner.begin_session("second-human", 300).unwrap();
    let mut signature = [0; 64];
    ed25519::sign_with_context(&second_seed, challenge.message_digest(), SESSION_CONTEXT, &mut signature).unwrap();
    let second_session = owner.complete_session(challenge, &signature).unwrap();
    let target = owner.workflow_evidence(&first_session).unwrap().target;
    let operation = DelegatedOperation {
        actor_subject: "reviewer", actor_controller: &admission.subjects[1].controller_public_key,
        purpose: "edit-source", command_id: "poet::edit", action: AuthoringAction::MaterialEdit,
        effects: &[CommandEffect::DocumentMutation], target: &target, now_unix: 0,
    };
    let first = signed(record(&admission), &ROOT_SEED);
    let mut second_grant = record(&admission);
    second_grant.root_subject = "second-human".into();
    second_grant.grantor_subject = "second-human".into();
    second_grant.grantor_controller = admission.subjects[3].controller_public_key;
    second_grant.agency.principal_did = "did:test:second-human".into();
    second_grant.task.principal_id = "second-human".into();
    second_grant.task.delegation_id = "second-root-grant".into();
    second_grant.agency.id = "second-root-grant".into();
    let first_inspection = owner.inspect_controller_delegation(&first_session, &[first], &operation).unwrap();
    let second_inspection = owner.inspect_controller_delegation(&second_session, &[signed(second_grant, &second_seed)], &operation).unwrap();
    assert_ne!(first_inspection.project_admitted_root_subject, second_inspection.project_admitted_root_subject);
    assert_ne!(first_inspection.delegation_path, second_inspection.delegation_path);
    assert!(first_inspection.has_admitted_control_overlap(&second_inspection));
    assert!(second_inspection.has_admitted_control_overlap(&first_inspection));
}

#[test]
fn live_leaf_session_must_match_verified_candidate_without_granting_execution() {
    let temp = tempfile::tempdir().unwrap();
    let s = snapshot();
    let owner = owner(temp.path(), s.clone());
    let root = login_root(&owner);
    let challenge = owner.begin_session("leaf", 300).unwrap();
    let mut signature = [0; 64];
    ed25519::sign_with_context(&[31; 32], challenge.message_digest(), SESSION_CONTEXT, &mut signature).unwrap();
    let leaf = owner.complete_session(challenge, &signature).unwrap();
    let challenge = owner.begin_session("reviewer", 300).unwrap();
    let mut signature = [0; 64];
    ed25519::sign_with_context(&AGENT_SEED, challenge.message_digest(), SESSION_CONTEXT, &mut signature).unwrap();
    let other_agent = owner.complete_session(challenge, &signature).unwrap();
    let target = owner.workflow_evidence(&root).unwrap().target;
    let chain = [signed(record(&s), &ROOT_SEED)];
    let chain = [chain[0].clone(), child(&s, &chain[0])];
    let operation = DelegatedOperation { actor_subject: "leaf", actor_controller: &s.subjects[2].controller_public_key,
        purpose: "edit-source", command_id: "poet::edit", action: AuthoringAction::MaterialEdit,
        effects: &[CommandEffect::DocumentMutation], target: &target, now_unix: 0 };
    let prior = owner.inspect_controller_delegation(&root, &chain, &operation).unwrap();
    assert!(prior.leaf_authentication_ref.is_none());
    assert!(prior.leaf_authenticated_until_unix.is_none());
    let generation = authority_generation(&owner, &root);
    let inspected = owner.inspect_authenticated_controller_delegation(&root, &leaf, &chain, &operation).unwrap();
    assert_eq!(inspected.acting_agent_subject, "leaf");
    assert!(inspected.leaf_authentication_ref.as_ref().is_some_and(|r| r.starts_with("urn:qualia:native-session-proof:sha256:")));
    assert!(inspected.leaf_authenticated_until_unix.unwrap() > now().unwrap());
    assert_eq!(inspected.authority_generation, generation);
    assert!(owner.inspect_authenticated_controller_delegation(&root, &other_agent, &chain, &operation).is_err());
    assert!(owner.inspect_authenticated_controller_delegation(&root, &root, &chain, &operation).is_err());
    let wrong_key = DelegatedOperation { actor_subject: "leaf", actor_controller: &s.subjects[1].controller_public_key,
        purpose: "edit-source", command_id: "poet::edit", action: AuthoringAction::MaterialEdit,
        effects: &[CommandEffect::DocumentMutation], target: &target, now_unix: 0 };
    assert!(owner.inspect_authenticated_controller_delegation(&root, &leaf, &chain, &wrong_key).is_err());
    let other_temp = tempfile::tempdir().unwrap();
    let foreign_owner = fixture::owner(other_temp.path(), s.clone());
    let foreign_root = login_root(&foreign_owner);
    assert!(owner.inspect_authenticated_controller_delegation(&root, &foreign_root, &chain, &operation).is_err());
    owner.disconnect(&leaf).unwrap();
    assert!(owner.inspect_authenticated_controller_delegation(&root, &leaf, &chain, &operation).is_err());
    let after = owner.workflow_evidence(&root).unwrap();
    assert_eq!(after.target, target);
    assert_eq!(after.authority_generation, generation);
    assert!(after.human_authority.delegation_path.is_empty(), "inspection must not adopt a chain");
}
