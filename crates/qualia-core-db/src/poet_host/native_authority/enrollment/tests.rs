use super::{super::*, *};
#[path = "fixture.rs"]
mod fixture;
use fixture::*;
use qualia_cooperative_core::projects::ProjectRole;

#[test]
fn deterministic_stub_vault_cannot_supply_native_production_signer() {
    let mut vault = crate::identity::key_vault::KeyVault::new();
    assert!(!vault.is_native_secure());
    assert!(NativeSigner::from_vault(&vault, "fixture").is_err());
    vault.lock();
    assert!(NativeSigner::from_vault(&vault, "fixture").is_err());
}

#[test]
fn sealed_native_signer_is_retained_and_fails_on_wrong_wrap_corruption_and_plaintext() {
    let f = fixture();
    let path = f.temp.path().join("candidate.sealed");
    let reopened = NativeSigner::open_sealed(&path, &WRAP).unwrap();
    assert_eq!(reopened.public_key(), f.candidate.public_key());
    assert!(NativeSigner::open_sealed(&path, &[68; 32]).is_err());
    assert!(NativeSigner::create_sealed(&path, &WRAP).is_err());
    let mut bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes.len(), 76);
    bytes[30] ^= 1;
    std::fs::write(&path, bytes).unwrap();
    assert!(NativeSigner::open_sealed(&path, &WRAP).is_err());
    std::fs::write(&path, [3; 32]).unwrap();
    assert!(NativeSigner::open_sealed(&path, &WRAP).is_err());
}
#[test]
fn explicit_steward_approval_signs_admission_and_invalidates_all_old_permissions() {
    let f = fixture();
    let req = request(&f);
    let id = f.owner.submit_enrollment(req).unwrap();
    assert!(f.owner.begin_session("contributor", 300).is_err());
    let permission = permission(&f);
    assert_eq!(
        f.owner.list_enrollments(&permission).unwrap()[0].status,
        EnrollmentStatus::Pending
    );
    let signed = f
        .owner
        .approve_enrollment(&permission, &id, approval(), &f.authority)
        .unwrap();
    assert_eq!(signed.snapshot.sequence, 2);
    signed.verify(&anchor(&f.authority), 1024 * 1024).unwrap();
    assert!(signed.snapshot.subjects[1].reviews.is_empty());
    assert!(f.owner.list_enrollments(&permission).is_err());
    let session = login(&f.owner, "contributor", &f.candidate);
    assert!(f.owner.enrollment_permission(&session).is_err());
}
#[test]
fn forged_wrong_project_stale_and_expired_requests_fail_closed() {
    let f = fixture();
    let mut forged = request(&f);
    forged.candidate.subject.push_str("forged");
    assert!(f.owner.submit_enrollment(forged).is_err());
    for case in 0..3 {
        let mut req = request(&f);
        match case {
            0 => req.project_id = "another-project".into(),
            1 => req.admission_sequence += 1,
            _ => {
                req.submitted_at_unix -= 400;
                req.expires_at_unix -= 400;
            }
        }
        req.signature = f
            .candidate
            .sign(&req.digest().unwrap(), ENROLLMENT_CONTEXT)
            .unwrap()
            .to_vec();
        assert!(f.owner.submit_enrollment(req).is_err());
    }
}
#[test]
fn replay_duplicate_controller_and_duplicate_identifier_are_rejected() {
    let f = fixture();
    let req = request(&f);
    let replay = req.clone();
    f.owner.submit_enrollment(req).unwrap();
    assert!(f.owner.submit_enrollment(replay).is_err());
    assert!(f.owner.submit_enrollment(request(&f)).is_err());
    let mut duplicate = request(&f);
    duplicate
        .candidate
        .identifiers
        .push(duplicate.candidate.identifiers[0].clone());
    duplicate.signature = f
        .candidate
        .sign(&duplicate.digest().unwrap(), ENROLLMENT_CONTEXT)
        .unwrap()
        .to_vec();
    assert!(f.owner.submit_enrollment(duplicate).is_err());
}
#[test]
fn requested_steward_review_or_human_claim_cannot_silently_escalate() {
    let f = fixture();
    let mut req = request(&f);
    req.candidate.membership.role = ProjectRole::Steward;
    req.signature = f
        .candidate
        .sign(&req.digest().unwrap(), ENROLLMENT_CONTEXT)
        .unwrap()
        .to_vec();
    assert!(f.owner.submit_enrollment(req).is_err());
    let mut req = request(&f);
    req.candidate.reviews.push(ReviewAdmission {
        id: "injected".into(),
        document_id: "fixture-document".into(),
        admission_ref: "self-granted".into(),
        valid_from_unix: 1,
        valid_until_unix: now().unwrap() + 300,
        revoked: false,
    });
    req.signature = f
        .candidate
        .sign(&req.digest().unwrap(), ENROLLMENT_CONTEXT)
        .unwrap()
        .to_vec();
    assert!(f.owner.submit_enrollment(req).is_err());
    let mut req = request(&f);
    req.candidate.kind = PrincipalKind::Human;
    req.signature = f
        .candidate
        .sign(&req.digest().unwrap(), ENROLLMENT_CONTEXT)
        .unwrap()
        .to_vec();
    let id = f.owner.submit_enrollment(req).unwrap();
    assert!(f
        .owner
        .approve_enrollment(&permission(&f), &id, approval(), &f.authority)
        .is_err());
    // Deliberately do not fabricate verification evidence or enroll a human.
}
#[test]
fn wrong_authority_foreign_permission_and_stale_request_cannot_be_approved() {
    let f = fixture();
    let other = fixture();
    let id = f.owner.submit_enrollment(request(&f)).unwrap();
    let p = permission(&f);
    assert!(f
        .owner
        .approve_enrollment(&p, &id, approval(), &f.candidate)
        .is_err());
    assert!(f
        .owner
        .approve_enrollment(&permission(&other), &id, approval(), &f.authority)
        .is_err());
    let mut replacement = f.owner.enrollment_snapshot().unwrap().snapshot;
    replacement.sequence += 1;
    f.owner
        .replace_admission(f.authority.sign_admission(replacement).unwrap())
        .unwrap();
    assert!(f
        .owner
        .approve_enrollment(&permission(&f), &id, approval(), &f.authority)
        .is_err());
}
#[test]
fn decline_is_durable_and_cannot_be_replayed_as_approval() {
    let f = fixture();
    let req = request(&f);
    let id = f.owner.submit_enrollment(req.clone()).unwrap();
    let p = permission(&f);
    f.owner
        .decline_enrollment(&p, &id, "fixture-decline")
        .unwrap();
    assert_eq!(
        f.owner.list_enrollments(&p).unwrap()[0].status,
        EnrollmentStatus::Declined
    );
    assert!(f
        .owner
        .approve_enrollment(&p, &id, approval(), &f.authority)
        .is_err());
    assert!(f.owner.submit_enrollment(req).is_err());
}
#[test]
fn review_grant_is_distinct_and_revocation_blocks_fresh_and_existing_sessions() {
    let f = fixture();
    let id = f.owner.submit_enrollment(request(&f)).unwrap();
    f.owner
        .approve_enrollment(&permission(&f), &id, approval(), &f.authority)
        .unwrap();
    let until = f.owner.enrollment_snapshot().unwrap().snapshot.subjects[1].valid_until_unix;
    f.owner
        .grant_enrollment_review(
            &permission(&f),
            "contributor",
            ReviewAdmission {
                id: "fixture-review".into(),
                document_id: "fixture-document".into(),
                admission_ref: "fixture-explicit-review-decision".into(),
                valid_from_unix: now().unwrap(),
                valid_until_unix: until,
                revoked: false,
            },
            &f.authority,
        )
        .unwrap();
    let session = login(&f.owner, "contributor", &f.candidate);
    f.owner
        .revoke_enrollment(
            &permission(&f),
            "contributor",
            "fixture-revocation",
            &f.authority,
        )
        .unwrap();
    assert!(f.owner.begin_session("contributor", 300).is_err());
    assert!(!f.owner.publication_is_current(&session, 1));
    let snapshot = f.owner.enrollment_snapshot().unwrap();
    assert!(
        snapshot.snapshot.subjects[1].revoked && snapshot.snapshot.subjects[1].reviews[0].revoked
    );
}
#[test]
fn new_process_reopens_sealed_key_signed_admission_and_requires_fresh_login() {
    let f = fixture();
    let id = f.owner.submit_enrollment(request(&f)).unwrap();
    f.owner
        .approve_enrollment(&permission(&f), &id, approval(), &f.authority)
        .unwrap();
    let old = login(&f.owner, "contributor", &f.candidate);
    drop(old);
    drop(f.owner);
    // This child uses only fixed public test wrapping material, never OS credentials.
    // libtest names omit the crate prefix and use the declaration module, which
    // can differ from a public API reexport such as `poet_host`.
    let (_, module) = module_path!()
        .split_once("::")
        .expect("crate-qualified module path");
    let child_test = format!("{module}::fresh_process_child");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(child_test)
        .arg("--nocapture")
        .env("QUALIA_ENROLLMENT_SOFTWARE_FIXTURE", f.temp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.lines().any(|line| line.trim() == "running 1 test"),
        "{stdout}"
    );
    assert!(stdout.contains("1 passed; 0 failed;"), "{stdout}");
    assert!(
        stdout.contains("ENROLLMENT_FRESH_PROCESS_VERIFIED"),
        "{stdout}"
    );
}
#[test]
fn fresh_process_child() {
    let Some(root) = std::env::var_os("QUALIA_ENROLLMENT_SOFTWARE_FIXTURE") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let authority = NativeSigner::open_sealed(&root.join("authority.sealed"), &WRAP).unwrap();
    let candidate = NativeSigner::open_sealed(&root.join("candidate.sealed"), &WRAP).unwrap();
    let owner = NativeAuthority::open_retained_current(
        &root.join("journal"),
        anchor(&authority),
        1024 * 1024,
        256,
    )
    .unwrap();
    assert_eq!(owner.enrollment_snapshot().unwrap().snapshot.sequence, 2);
    {
        let state = owner.state.lock().unwrap();
        assert!(
            state.sessions.iter().all(Option::is_none) && state.pending.iter().all(Option::is_none)
        );
    }
    let challenge = owner.begin_session("contributor", 300).unwrap();
    let replay = challenge.clone();
    let proof = candidate.prove_session(&challenge).unwrap();
    owner.complete_session(challenge, &proof).unwrap();
    assert!(owner.complete_session(replay, &proof).is_err());
    println!("ENROLLMENT_FRESH_PROCESS_VERIFIED");
}

#[test]
fn concurrent_contributor_does_not_replace_steward_permission_or_audit_actor() {
    let f = fixture();
    let id = f.owner.submit_enrollment(request(&f)).unwrap();
    f.owner
        .approve_enrollment(&permission(&f), &id, approval(), &f.authority)
        .unwrap();
    let steward = login(&f.owner, "steward", &f.steward);
    let permission = f.owner.enrollment_permission(&steward).unwrap();
    // The later login has no steward role and cannot acquire its permission.
    let contributor = login(&f.owner, "contributor", &f.candidate);
    assert!(f.owner.enrollment_permission(&contributor).is_err());
    assert!(f.owner.list_enrollments(&permission).is_ok());
    let challenge = f.owner.begin_session("contributor", 300).unwrap();
    f.owner
        .revoke_enrollment(&permission, "contributor", "fixture/revoke", &f.authority)
        .unwrap();
    assert!(f.owner.list_enrollments(&permission).is_err());
    assert!(f.owner.workflow_evidence(&contributor).is_err());
    assert!(f
        .owner
        .complete_session(
            challenge.clone(),
            &f.candidate.prove_session(&challenge).unwrap()
        )
        .is_err());
    let fresh_steward = login(&f.owner, "steward", &f.steward);
    let fresh_permission = f.owner.enrollment_permission(&fresh_steward).unwrap();
    let records = f.owner.list_enrollments(&fresh_permission).unwrap();
    assert_eq!(records[0].decided_by_subject.as_deref(), Some("steward"));
    assert_eq!(records[0].status, EnrollmentStatus::Revoked);
}
