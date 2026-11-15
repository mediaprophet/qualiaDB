use super::super::{AuthoringAction, DocumentRevision};
use super::*;
use crate::{
    agency_delegation::{AgencyDelegation, ConsentState},
    authority_type::{ids, AuthorityProfile},
};
use qualia_app_contract::{
    AppId, CommandEffect, DecimalU64, DelegationLifecycle, DelegationMode, ResourceBudgets,
    TaskDelegation,
};

fn grant() -> AuthoringDelegation {
    let mut agency = AgencyDelegation::new(
        "did:test:principal",
        crate::agency_domain::ids::AI_PROXY,
        "urn:test:retained-values",
        100,
    );
    agency.id = "delegation-one".into();
    agency.agent_dids = vec!["did:test:agent".into()];
    agency.authority = AuthorityProfile::new(
        ids::MOD_AUTOMATED,
        ids::TRIG_PERSISTENT,
        ids::ACCT_VALUES_BOUND,
    );
    agency.consent = ConsentState::Granted;
    agency.valid_to_unix = Some(500);
    agency.evidence_ref = Some("urn:test:controller-consent-reference".into());
    agency.scope = vec![
        ("purpose".into(), "edit this source".into()),
        ("action".into(), "read_source".into()),
        ("action".into(), "material_edit".into()),
        ("data_class".into(), AUTHORING_DATA_CLASS.into()),
    ];
    AuthoringDelegation {
        version: 1,
        root_subject: "principal".into(),
        root_adoption_digest: [1; 32],
        project_id: "project".into(),
        document_id: "document".into(),
        source_id: "source".into(),
        grantor_subject: "principal".into(),
        grantor_controller: [2; 32],
        grantee_subject: "agent".into(),
        grantee_controller: [3; 32],
        parent_digest: None,
        remaining_depth: 1,
        actions: vec![AuthoringAction::ReadSource, AuthoringAction::MaterialEdit],
        agency,
        task: TaskDelegation {
            delegation_id: "delegation-one".into(),
            principal_id: "principal".into(),
            actor_id: "agent".into(),
            app_id: AppId::new("poet").unwrap(),
            goal: "edit this source".into(),
            allowed_effects: vec![CommandEffect::Read, CommandEffect::DocumentMutation],
            allowed_command_ids: vec!["poet::edit".into()],
            mode: DelegationMode::BoundedDelegation,
            state: DelegationLifecycle::Approved,
            budgets: ResourceBudgets {
                max_steps: 10,
                max_cpu_time_ms: 100,
                max_memory_bytes: 1024,
                external_cost_minor_units: None,
            },
            expiry_timestamp_ms: DecimalU64::new(500_000),
        },
    }
}
fn document() -> DocumentRevision {
    DocumentRevision {
        project_id: "project".into(),
        document_id: "document".into(),
        source_id: "source".into(),
        revision: 1,
        source_digest: [1; 32],
        author_subject: Some("agent".into()),
    }
}
fn operation<'a>(target: &'a DocumentRevision) -> DelegatedOperation<'a> {
    DelegatedOperation {
        actor_subject: "agent",
        actor_controller: &[3; 32],
        purpose: "edit this source",
        command_id: "poet::edit",
        action: AuthoringAction::MaterialEdit,
        effects: &[CommandEffect::DocumentMutation],
        target,
        now_unix: 200,
    }
}

#[test]
fn task_and_agency_scope_intersect_without_a_wildcard_command() {
    let mut d = grant();
    let target = document();
    let mut op = operation(&target);
    assert_eq!(check_delegated_operation(&d, &op), Ok(()));
    op.actor_subject = "other";
    assert_eq!(
        check_delegated_operation(&d, &op),
        Err(DelegationError::Scope)
    );
    op.actor_subject = "agent";
    op.purpose = "other purpose";
    assert_eq!(
        check_delegated_operation(&d, &op),
        Err(DelegationError::Scope)
    );
    op.purpose = "edit this source";
    op.command_id = "poet::publish";
    assert_eq!(
        check_delegated_operation(&d, &op),
        Err(DelegationError::Scope)
    );
    d.task.allowed_command_ids.clear();
    assert_eq!(validate_delegation(&d, 200), Err(DelegationError::Bounds));
}

#[test]
fn explicit_read_and_write_actions_require_the_native_data_and_action_scopes() {
    let mut d = grant();
    let target = document();
    let mut op = operation(&target);
    assert_eq!(check_delegated_operation(&d, &op), Ok(()));
    op.action = AuthoringAction::ReadSource;
    op.effects = &[CommandEffect::Read];
    assert_eq!(check_delegated_operation(&d, &op), Ok(()));
    op.action = AuthoringAction::Undo;
    op.effects = &[CommandEffect::DocumentMutation];
    assert_eq!(
        check_delegated_operation(&d, &op),
        Err(DelegationError::Scope)
    );
    op.action = AuthoringAction::MaterialEdit;
    d.agency.scope.retain(|(key, _)| key != "data_class");
    d.agency
        .scope
        .push(("data_class".into(), "foreign_data".into()));
    assert_eq!(
        check_delegated_operation(&d, &op),
        Err(DelegationError::Scope)
    );
    d.agency.scope.retain(|(key, _)| key != "data_class");
    d.agency
        .scope
        .push(("data_class".into(), AUTHORING_DATA_CLASS.into()));
    d.agency
        .scope
        .retain(|(key, value)| key != "action" || value != "material_edit");
    assert_eq!(
        check_delegated_operation(&d, &op),
        Err(DelegationError::Scope)
    );
}
#[test]
fn exclusive_expiry_withdrawal_and_unsupported_approval_modes_are_denied() {
    let mut d = grant();
    let target = document();
    let mut op = operation(&target);
    op.now_unix = 500;
    assert_eq!(
        check_delegated_operation(&d, &op),
        Err(DelegationError::ExpiredOrRevoked)
    );
    op.now_unix = 200;
    d.agency.consent = ConsentState::Withdrawn;
    assert_eq!(
        check_delegated_operation(&d, &op),
        Err(DelegationError::Consent)
    );
    d.agency.consent = ConsentState::Granted;
    for mode in [
        DelegationMode::Observe,
        DelegationMode::Propose,
        DelegationMode::ApproveEachAction,
    ] {
        d.task.mode = mode;
        assert_eq!(
            check_delegated_operation(&d, &op),
            Err(DelegationError::UnsupportedPolicy)
        );
    }
}
#[test]
fn child_cannot_add_scope_extend_time_increase_budget_or_change_root() {
    let p = grant();
    let mut c = p.clone();
    c.grantor_subject = p.grantee_subject.clone();
    c.grantor_controller = p.grantee_controller;
    c.grantee_subject = "child".into();
    c.grantee_controller = [4; 32];
    c.remaining_depth = 0;
    assert_eq!(check_parent_attenuation(&p, &c), Ok(()));
    c.task.budgets.max_steps += 1;
    assert_eq!(
        check_parent_attenuation(&p, &c),
        Err(DelegationError::Attenuation)
    );
    c.task.budgets.max_steps -= 1;
    c.task.allowed_command_ids.push("poet::publish".into());
    assert_eq!(
        check_parent_attenuation(&p, &c),
        Err(DelegationError::Attenuation)
    );
    c.task.allowed_command_ids.pop();
    c.task.expiry_timestamp_ms = DecimalU64::new(501_000);
    assert_eq!(
        check_parent_attenuation(&p, &c),
        Err(DelegationError::Attenuation)
    );
    c.task.expiry_timestamp_ms = p.task.expiry_timestamp_ms;
    c.root_subject = "other".into();
    assert_eq!(
        check_parent_attenuation(&p, &c),
        Err(DelegationError::Attenuation)
    );
}
#[test]
fn no_financial_or_provider_authority_follows_from_controller_consent() {
    let mut d = grant();
    d.task.budgets.external_cost_minor_units = Some(1);
    assert_eq!(validate_delegation(&d, 200), Err(DelegationError::Budget));
    d.task.budgets.external_cost_minor_units = None;
    d.task
        .allowed_effects
        .push(CommandEffect::NetworkDisclosure);
    assert_eq!(
        validate_delegation(&d, 200),
        Err(DelegationError::UnsupportedPolicy)
    );
}
