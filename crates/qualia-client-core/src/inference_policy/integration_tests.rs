//! Independent actual client qualification. All singleton state, actual sealed keys,
//! journals and policies live in child processes beneath one RAII-owned directory.
//! This exercises client admission and accepted records, not the G3 neural loop.
use super::*;
use crate::api::inference_policy as api;
use qualia_cooperative_core::{
    authoring::{DocumentRevision, IdentifierAssociationEvidence, PrincipalKind},
    projects::{Project, ProjectMembership, ProjectRole},
};
use qualia_core_db::{
    key_vault::KeyRole,
    llm_agent::governed::*,
    poet_host::native_authority::{enrollment::NativeSigner, AdmissionSnapshot, SubjectAdmission},
};
use std::{path::Path, sync::Arc};
const BUDGET: usize = CONFIGURATION_BUDGET_BYTES;
const CHILD: &str = "QUALIA_SENTINEL_CLIENT_TEST_CHILD";
const TEST: &str = "inference_policy::integration_tests::installed_policy_client_route_requires_explicit_identity_and_vault";
const SUBJECTS: [&str; 3] = ["human", "agent", "organization"];

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
static UNCERTAIN_CHILD: AtomicBool = AtomicBool::new(false);
static REAPED_CHILDREN: AtomicUsize = AtomicUsize::new(0);
type ChildDrain = std::thread::JoinHandle<Result<Vec<u8>, String>>;
struct ChildGuard {
    child: Option<std::process::Child>,
    drains: [Option<ChildDrain>; 2],
}
impl ChildGuard {
    fn reaped(&mut self) {
        self.child.take();
        REAPED_CHILDREN.fetch_add(1, Ordering::SeqCst);
    }
    fn terminate(&mut self) -> Result<(), String> {
        let Some(child) = self.child.as_mut() else {
            return Ok(());
        };
        if matches!(child.try_wait(), Ok(Some(_))) {
            self.reaped();
            return Ok(());
        }
        let kill = child.kill();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.reaped();
                    return Ok(());
                }
                Ok(None) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(10))
                }
                wait => {
                    UNCERTAIN_CHILD.store(true, Ordering::SeqCst);
                    return Err(format!("child termination unconfirmed: kill={kill:?}, wait={wait:?}; retain owned directory"));
                }
            }
        }
    }
    fn collect(&mut self) -> Result<[Vec<u8>; 2], String> {
        let mut result = [Vec::new(), Vec::new()];
        let mut error = None;
        for (index, slot) in self.drains.iter_mut().enumerate() {
            if let Some(thread) = slot.take() {
                match thread.join() {
                    Ok(Ok(bytes)) => result[index] = bytes,
                    Ok(Err(message)) => error = Some(message),
                    Err(_) => error = Some("child output drain panicked".into()),
                }
            }
        }
        error.map_or(Ok(result), Err)
    }
}
impl Drop for ChildGuard {
    fn drop(&mut self) {
        match self.terminate() {
            Ok(()) => {
                let _ = self.collect();
            }
            Err(error) => eprintln!("{error}"),
        }
    }
}
struct OwnedClientDirectory(Option<tempfile::TempDir>);
impl Drop for OwnedClientDirectory {
    fn drop(&mut self) {
        if UNCERTAIN_CHILD.load(Ordering::SeqCst) {
            if let Some(directory) = self.0.take() {
                let path = directory.keep();
                let _ = std::fs::write(
                    path.join("cleanup-required.json"),
                    b"{\"reason\":\"child termination unconfirmed; directory retained\"}",
                );
                eprintln!("retained owned client test directory: {}", path.display());
            }
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ChildSetupFault {
    None,
    AfterSpawnPanic,
    AfterFirstDrainPanic,
    SetupError,
}
fn child_phase_with_fault(root: &Path, phase: &str, fault: ChildSetupFault) -> Result<(), String> {
    use std::io::Read;
    // Own immediately: every subsequent allocation, pipe and thread setup can fail.
    let mut guard = ChildGuard {
        child: Some(
            std::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
                .args(["--exact", TEST, "--nocapture"])
                .env(CHILD, phase)
                .env("QUALIA_APP_META_DIR", root)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| e.to_string())?,
        ),
        drains: [None, None],
    };
    assert!(
        fault != ChildSetupFault::AfterSpawnPanic,
        "injected after-spawn panic"
    );
    let overflow = Arc::new(AtomicBool::new(false));
    let drain = |mut stream: Box<dyn Read + Send>| {
        let overflow = Arc::clone(&overflow);
        std::thread::Builder::new()
            .spawn(move || {
                let mut retained = Vec::new();
                let mut chunk = [0; 4096];
                loop {
                    let n = stream.read(&mut chunk).map_err(|e| e.to_string())?;
                    if n == 0 {
                        break;
                    }
                    let room = (32 * 1024usize).saturating_sub(retained.len());
                    retained.extend_from_slice(&chunk[..n.min(room)]);
                    if n > room {
                        overflow.store(true, Ordering::SeqCst);
                    }
                }
                Ok(retained)
            })
            .map_err(|e| e.to_string())
    };
    let stdout = guard
        .child
        .as_mut()
        .unwrap()
        .stdout
        .take()
        .ok_or("missing stdout")?;
    let stderr = guard
        .child
        .as_mut()
        .unwrap()
        .stderr
        .take()
        .ok_or("missing stderr")?;
    if fault == ChildSetupFault::SetupError {
        return Err("injected thread setup error".into());
    }
    guard.drains[0] = Some(drain(Box::new(stdout))?);
    assert!(
        fault != ChildSetupFault::AfterFirstDrainPanic,
        "injected after-drain panic"
    );
    guard.drains[1] = Some(drain(Box::new(stderr))?);
    let started = std::time::Instant::now();
    let status = loop {
        match guard.child.as_mut().unwrap().try_wait() {
            Ok(Some(status)) => {
                guard.reaped();
                break status;
            }
            Err(error) => return Err(error.to_string()),
            Ok(None) => (),
        }
        if overflow.load(Ordering::SeqCst) {
            return Err("child exceeded 64 KiB output budget".into());
        }
        if started.elapsed().as_secs() >= 120 {
            return Err("client child deadline exceeded".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let [out, err] = guard.collect()?;
    let out = String::from_utf8_lossy(&out);
    let err = String::from_utf8_lossy(&err);
    if overflow.load(Ordering::SeqCst) {
        return Err("child exceeded 64 KiB output budget".into());
    }
    if !status.success() {
        return Err(format!("phase {phase}: {out}\n{err}"));
    }
    if !out.contains("running 1 test") || !out.contains("1 passed; 0 failed; 0 ignored") {
        return Err(format!("child exact test was not executed: {out}"));
    }
    Ok(())
}
fn child_phase(root: &Path, phase: &str) {
    child_phase_with_fault(root, phase, ChildSetupFault::None).unwrap();
}
#[test]
fn installed_policy_client_route_requires_explicit_identity_and_vault() {
    let phase = std::env::var(CHILD).ok();
    if phase.is_none() {
        let mut dir = OwnedClientDirectory(Some(tempfile::tempdir().unwrap()));
        let path = dir.0.as_ref().unwrap().path().to_path_buf();
        std::fs::write(path.join(".isolated-policy-test"), b"owned test directory").unwrap();
        for fault in [
            ChildSetupFault::AfterSpawnPanic,
            ChildSetupFault::AfterFirstDrainPanic,
            ChildSetupFault::SetupError,
        ] {
            let before = REAPED_CHILDREN.load(Ordering::SeqCst);
            let result =
                std::panic::catch_unwind(|| child_phase_with_fault(&path, "setup-probe", fault));
            match fault {
                ChildSetupFault::SetupError => {
                    assert_eq!(result.unwrap().unwrap_err(), "injected thread setup error")
                }
                _ => assert!(result.is_err()),
            }
            assert!(!UNCERTAIN_CHILD.load(Ordering::SeqCst));
            assert_eq!(REAPED_CHILDREN.load(Ordering::SeqCst), before + 1);
            assert!(path.exists());
        }
        child_phase(&path, "fresh");
        child_phase(&path, "restart");
        assert!(!UNCERTAIN_CHILD.load(Ordering::SeqCst));
        dir.0.take().unwrap().close().unwrap();
        assert!(!path.exists());
        return;
    }
    if phase.as_deref() == Some("setup-probe") {
        std::thread::sleep(std::time::Duration::from_secs(5));
        return;
    }
    use qualia_core_db::key_vault::KeyVault;
    let root = crate::state::app_meta_dir();
    assert_eq!(
        std::fs::read(root.join(".isolated-policy-test")).unwrap(),
        b"owned test directory"
    );
    assert!(crate::state::APP_STATE.get().is_none());
    let before = tree_snapshot(&root);
    assert!(load_policy_status().is_err());
    let mut locked = KeyVault::new();
    locked.lock();
    assert!(crate::state::init_inference_host_state(locked).is_err());
    assert!(crate::state::APP_STATE.get().is_none());
    assert_eq!(tree_snapshot(&root), before);
    // Actual no-auto host initializer; no production account/keyring is touched.
    let state = crate::state::init_inference_host_state(KeyVault::new()).unwrap();
    assert_eq!(tree_snapshot(&root), before);
    assert!(!*state.daemon_running.lock().unwrap());
    assert!(state.active_downloads.lock().unwrap().is_empty());
    assert!(crate::state::init_inference_host_state(KeyVault::new()).is_err());
    assert!(Arc::ptr_eq(crate::state::APP_STATE.get().unwrap(), &state));
    assert_eq!(tree_snapshot(&root), before);

    if phase.as_deref() == Some("fresh") {
        fresh(&root, &state);
    } else {
        assert_eq!(phase.as_deref(), Some("restart"));
        restart(&root);
    }
    assert!(!root.join("person_identity.json").exists());
    assert!(!root.join("person_wrap.key").exists());
    assert!(!root.join("master.key").exists());
    assert_artifacts(&root);
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
fn document() -> DocumentRevision {
    DocumentRevision {
        project_id: "project".into(),
        document_id: "document".into(),
        source_id: "source".into(),
        revision: 1,
        source_digest: [1; 32],
        author_subject: None,
    }
}
fn selection(name: &str, alternate: bool) -> SelectedInferenceIdentifier {
    SelectedInferenceIdentifier {
        value: if alternate {
            format!("urn:alternate:{name}")
        } else {
            format!("did:test:{name}")
        },
        scheme: if alternate { "urn" } else { "did:test" }.into(),
        issuer: "project-issuer".into(),
        project_id: "project".into(),
        evidence_ref: format!("association/{name}/{alternate}"),
    }
}
fn snapshot(keys: [[u8; 32]; 3]) -> AdmissionSnapshot {
    let until = now() + 3600;
    let kinds = [
        PrincipalKind::Human,
        PrincipalKind::SoftwareAgent,
        PrincipalKind::Organization,
    ];
    AdmissionSnapshot {
        version: 1,
        sequence: 1,
        issuer_identifier: "project-issuer".into(),
        project: Project {
            id: "project".into(),
            name: "isolated project".into(),
            description: String::new(),
            licensing_ontologies: vec![],
            created_at_unix: 1,
            created_at_instant: None,
        },
        valid_from_unix: 1,
        valid_until_unix: until,
        revoked: false,
        package_reviews: vec![],
        subjects: SUBJECTS
            .iter()
            .enumerate()
            .map(|(i, name)| SubjectAdmission {
                subject: (*name).into(),
                kind: kinds[i],
                controller_public_key: keys[i],
                membership: ProjectMembership {
                    id: format!("membership/{name}"),
                    project_id: "project".into(),
                    member_did: format!("did:test:{name}"),
                    role: ProjectRole::Contributor,
                    agreed_at_unix: 1,
                    agreed_at_instant: None,
                },
                identifiers: [false, true]
                    .into_iter()
                    .map(|alt| {
                        let s = selection(name, alt);
                        IdentifierAssociationEvidence {
                            value: s.value,
                            scheme: s.scheme,
                            issuer: s.issuer,
                            project_id: s.project_id,
                            evidence_ref: s.evidence_ref,
                            valid_from_unix: 1,
                            valid_until_unix: until,
                        }
                    })
                    .collect(),
                reviews: vec![],
                admission_ref: format!("adoption/{name}"),
                valid_from_unix: 1,
                valid_until_unix: until,
                revoked: false,
            })
            .collect(),
    }
}
fn login(name: &str, alt: bool) -> InferenceActorSession {
    api::login_inference_actor(name.into(), selection(name, alt), name.into(), 300).unwrap()
}
fn request() -> InferenceAdmission {
    InferenceAdmission {
        request_digest: [1; 32],
        model_digest: [2; 32],
        model_generation: 1,
        device_id: 1,
        device_generation: 1,
        graph_scope: 29,
        first_position: 4,
        instrumentation: Default::default(),
    }
}
fn proposal(actor: &InferenceActorSession, position: u64) -> TokenProposal {
    let subject = actor.policy_scope().unwrap().subject;
    TokenProposal {
        position,
        token: 13,
        logit: 0.5,
        observation: qualia_core_db::NQuin {
            subject,
            predicate: 23,
            object: 31,
            context: 29,
            metadata: 0,
            parity: subject ^ 23 ^ 31 ^ 29,
        },
    }
}
fn rules(actors: &[InferenceActorSession]) -> PolicyRules {
    PolicyRules {
        actors: PolicyList::from_vec(actors.iter().map(|a| a.policy_scope().unwrap()).collect())
            .unwrap(),
        model: DigestScope::Exact([2; 32]),
        request: DigestScope::Exact([1; 32]),
        graph_scopes: PolicyList::from_vec(vec![29]).unwrap(),
        program: PolicyList::default(),
        norms: PolicyList::default(),
        facts: PolicyList::default(),
        epistemic: PolicyList::default(),
        trace: PolicyList::default(),
        required_claim: 0,
        temporal_invariant: 0,
        non_derogable: false,
    }
}
fn install(actors: &[InferenceActorSession]) {
    let generation = load_policy_status().unwrap().generation;
    api::install_inference_policy(
        serde_json::to_string(&rules(actors)).unwrap(),
        now() + 1800,
        generation,
    )
    .unwrap();
}
fn accept(actor: &InferenceActorSession) {
    let mut s = admit_inference(request(), actor, POLICY_COLD_BUDGET_BYTES).unwrap();
    let p = proposal(actor, 4);
    let d = s.prepare(p).unwrap();
    let mut record = None;
    s.accept_into(&d, &mut record).unwrap();
    let accepted = record.unwrap().proposal;
    assert_eq!(accepted.position, p.position);
    assert_eq!(accepted.token, p.token);
    assert_eq!(accepted.logit.to_bits(), p.logit.to_bits());
    assert_eq!(accepted.observation, p.observation);
    assert!(s.accept_into(&d, &mut None).is_err());
}
fn pending(actor: &InferenceActorSession) -> (GovernedTokenSession, TokenDecision) {
    let mut s = admit_inference(request(), actor, POLICY_COLD_BUDGET_BYTES).unwrap();
    let d = s.prepare(proposal(actor, 4)).unwrap();
    (s, d)
}
fn rejected(mut s: GovernedTokenSession, d: TokenDecision) {
    let mut output = None;
    assert!(s.accept_into(&d, &mut output).is_err());
    assert!(output.is_none());
    assert!(s.is_terminal());
}
fn fresh(root: &Path, state: &Arc<crate::state::AppState>) {
    assert!(api::get_inference_policy_status().is_err());
    assert!(!root.join("inference-host.json").exists());
    assert!(api::login_inference_actor(
        "human".into(),
        selection("human", false),
        "missing".into(),
        300
    )
    .is_err());
    assert!(create_inference_actor_signer("insufficient", BUDGET - 1).is_err());
    let keys = SUBJECTS.map(|name| api::create_inference_actor_key(name.into()).unwrap());
    assert!(api::create_inference_actor_key("human".into()).is_err());
    let issuer =
        NativeSigner::create_sealed(&root.join("project-issuer.sealed"), &[27; 32]).unwrap();
    let snapshot = snapshot(keys);
    std::fs::write(
        root.join("signed-fixture.json"),
        serde_json::to_vec(&snapshot).unwrap(),
    )
    .unwrap();
    let config = InferenceHostConfiguration {
        installer_identifier: "urn:isolated:installer".into(),
        installer_public_key: state
            .key_vault
            .lock()
            .unwrap()
            .derive_key_for_role(KeyRole::ControllerSigning)
            .verifying_key()
            .to_bytes(),
        project_id: "project".into(),
        project_issuer: "project-issuer".into(),
        project_public_key: issuer.public_key(),
    };
    for field in 0..3 {
        let mut wrong = config.clone();
        match field {
            0 => wrong.installer_identifier.clear(),
            1 => wrong.installer_public_key = [9; 32],
            _ => wrong.project_public_key = keys[0],
        }
        assert!(api::configure_inference_host(
            wrong,
            issuer.sign_admission(snapshot.clone()).unwrap(),
            document()
        )
        .is_err());
        assert!(!root.join("inference-host.json").exists());
    }
    api::configure_inference_host(
        config.clone(),
        issuer.sign_admission(snapshot.clone()).unwrap(),
        document(),
    )
    .unwrap();
    assert!(!load_policy_status().unwrap().installed);
    let config_bytes = std::fs::read(root.join("inference-host.json")).unwrap();
    let mut actors = Vec::new();
    for (i, name) in SUBJECTS.iter().enumerate() {
        for alt in [false, true] {
            let a = login(name, alt);
            let scope = a.policy_scope().unwrap();
            assert_eq!(scope.kind, snapshot.subjects[i].kind);
            assert_eq!(scope.subject, qualia_core_db::q_hash(name));
            assert_ne!(
                scope.subject,
                qualia_core_db::q_hash(&config.installer_identifier)
            );
            assert!(admit_inference(request(), &a, POLICY_COLD_BUDGET_BYTES).is_err());
            actors.push(a);
        }
    }
    assert_ne!(
        actors[0].policy_scope().unwrap().selected_identifier_digest,
        actors[1].policy_scope().unwrap().selected_identifier_digest
    );
    assert!(api::login_inference_actor(
        "human".into(),
        selection("human", false),
        "agent".into(),
        300
    )
    .is_err());
    assert!(api::login_inference_actor(
        "human".into(),
        selection("agent", false),
        "human".into(),
        300
    )
    .is_err());
    assert!(api::login_inference_actor(
        "human".into(),
        selection("human", false),
        "missing".into(),
        300
    )
    .is_err());
    // Corrupt actual authenticated ciphertext, then restore exact bytes; login
    // must not recreate, migrate, or overwrite the selected sealed credential.
    let key_name = hex::encode(<sha2::Sha256 as sha2::Digest>::digest(b"human"));
    let key_path = root
        .join("inference-actor-keys")
        .join(format!("{key_name}.sealed"));
    let original_key = std::fs::read(&key_path).unwrap();
    assert_eq!(original_key.len(), 76);
    let mut corrupted = original_key.clone();
    corrupted[20] ^= 1;
    std::fs::write(&key_path, &corrupted).unwrap();
    assert!(api::login_inference_actor(
        "human".into(),
        selection("human", false),
        "human".into(),
        300
    )
    .is_err());
    assert_eq!(std::fs::read(&key_path).unwrap(), corrupted);
    std::fs::write(&key_path, &original_key).unwrap();
    install(&actors[..1]);
    accept(&actors[0]);
    assert!(admit_inference(request(), &actors[1], POLICY_COLD_BUDGET_BYTES).is_err());
    let mut refusing = rules(&actors);
    refusing.program = PolicyList::from_vec(vec![
        ConstraintInstruction::MatchObject { value: 0 },
        ConstraintInstruction::HaltIfFalse,
    ])
    .unwrap();
    let generation = load_policy_status().unwrap().generation;
    api::install_inference_policy(
        serde_json::to_string(&refusing).unwrap(),
        now() + 1800,
        generation,
    )
    .unwrap();
    // The same admitted actor and well-formed proposal reaches the actual VM;
    // its explicit installed rejection produces no accepted record.
    let (denied, decision) = pending(&actors[0]);
    rejected(denied, decision);
    install(&actors);
    for a in &actors {
        accept(a);
    }
    let (mut admitted, decision) = pending(&actors[0]);
    assert!(configure_inference_authority(
        config.clone(),
        issuer.sign_admission(snapshot.clone()).unwrap(),
        document(),
        BUDGET - 1
    )
    .is_err());
    let mut kept = None;
    admitted.accept_into(&decision, &mut kept).unwrap();
    assert!(kept.is_some());
    assert_eq!(
        std::fs::read(root.join("inference-host.json")).unwrap(),
        config_bytes
    );
    let (s, d) = pending(&actors[0]);
    let generation = load_policy_status().unwrap().generation;
    api::revoke_inference_policy(generation).unwrap();
    rejected(s, d);
    assert!(admit_inference(request(), &actors[1], POLICY_COLD_BUDGET_BYTES).is_err());
    install(&actors);
    api::logout_inference_actor(&actors[0]).unwrap();
    assert!(actors[0].policy_scope().is_err());
    accept(&actors[1]);
    let (s, d) = pending(&actors[1]);
    api::disconnect_inference_controller();
    rejected(s, d);
    assert!(actors[1].policy_scope().is_err());
    assert!(api::get_inference_policy_status().unwrap()["installed"]
        .as_bool()
        .unwrap());
    // Old handles intentionally remain alive during recovery: no journal lock collision.
    let a = login("human", true);
    accept(&a);
    let (s, d) = pending(&a);
    api::lock_inference_controller_vault().unwrap();
    rejected(s, d);
    assert!(api::get_inference_policy_status().is_err());
    assert!(api::create_inference_actor_key("locked".into()).is_err());
    *state.key_vault.lock().unwrap() = qualia_core_db::key_vault::KeyVault::new();
    assert!(load_policy_status().unwrap().installed);
    assert!(a.policy_scope().is_err());
    let b = login("human", true);
    accept(&b);
    // Signed bytes are authenticated using the vault key, not their own embedded key.
    let mut changed: serde_json::Value = serde_json::from_slice(&config_bytes).unwrap();
    changed["document"]["source_digest"][0] = serde_json::json!(99);
    for bytes in [
        b"{}".to_vec(),
        vec![b' '; 32 * 1024 + 1],
        serde_json::to_vec(&changed).unwrap(),
    ] {
        let current = login("human", true);
        let (s, d) = pending(&current);
        std::fs::write(root.join("inference-host.json"), &bytes).unwrap();
        assert!(load_policy_status().is_err());
        rejected(s, d);
        assert_eq!(
            std::fs::read(root.join("inference-host.json")).unwrap(),
            bytes
        );
        std::fs::write(root.join("inference-host.json"), &config_bytes).unwrap();
        assert!(load_policy_status().unwrap().installed);
        assert!(current.policy_scope().is_err());
    }
    let c = login("human", true);
    accept(&c);
    let mut wrong = config.clone();
    wrong.project_issuer = "untrusted".into();
    let (s, d) = pending(&c);
    assert!(api::configure_inference_host(
        wrong,
        issuer.sign_admission(snapshot.clone()).unwrap(),
        document()
    )
    .is_err());
    rejected(s, d);
    assert_eq!(
        std::fs::read(root.join("inference-host.json")).unwrap(),
        config_bytes
    );
    let recovered = login("human", true);
    accept(&recovered);
    // No credential or policy is inferred from arbitrary profile identifiers.
    std::fs::write(
        root.join("profile.json"),
        br#"{"public_did":"did:untrusted:alias"}"#,
    )
    .unwrap();
    accept(&recovered);
    // Keep A's handles alive while selecting a second independently signed
    // project, then return to A. Weak registry reuse must not orphan either lock.
    let mut project_b = config.clone();
    project_b.project_id = "project-b".into();
    let mut admission_b = snapshot.clone();
    admission_b.project.id = "project-b".into();
    for subject in &mut admission_b.subjects {
        subject.membership.project_id = "project-b".into();
        for id in &mut subject.identifiers {
            id.project_id = "project-b".into();
        }
    }
    let mut document_b = document();
    document_b.project_id = "project-b".into();
    api::configure_inference_host(
        project_b,
        issuer.sign_admission(admission_b).unwrap(),
        document_b,
    )
    .unwrap();
    let mut selected_b = selection("human", true);
    selected_b.project_id = "project-b".into();
    let other_project_actor =
        api::login_inference_actor("human".into(), selected_b, "human".into(), 300).unwrap();
    assert!(admit_inference(request(), &other_project_actor, POLICY_COLD_BUDGET_BYTES).is_err());
    assert!(recovered.policy_scope().is_err());
    api::configure_inference_host(
        config.clone(),
        issuer.sign_admission(snapshot.clone()).unwrap(),
        document(),
    )
    .unwrap();
    assert!(other_project_actor.policy_scope().is_err());
    let back_to_a = login("human", true);
    accept(&back_to_a);
    // Repeated loss/login crosses both fixed tracking capacities without leaking slots.
    for _ in 0..20 {
        let old = login("agent", false);
        accept(&old);
        api::disconnect_inference_controller();
        assert!(old.policy_scope().is_err());
        assert!(load_policy_status().unwrap().installed);
    }
    // Expiry must free tracker capacity through the same current authority
    // predicate; retaining expired opaque handles must not strand native slots.
    let expired: Vec<_> = (0..16)
        .map(|_| {
            api::login_inference_actor("agent".into(), selection("agent", false), "agent".into(), 1)
                .unwrap()
        })
        .collect();
    let latest_deadline = now() + 1;
    while now() < latest_deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    for old in &expired {
        assert!(old.policy_scope().is_err());
    }
    let after_expiry = login("agent", false);
    accept(&after_expiry);
    // Save failure occurs after durable temporary bytes exist, before promotion.
    let before = std::fs::read(root.join("inference-host.json")).unwrap();
    super::configuration::fail_next_save(super::configuration::SaveFault::Error);
    assert!(api::configure_inference_host(
        config.clone(),
        issuer.sign_admission(snapshot.clone()).unwrap(),
        document()
    )
    .is_err());
    assert_eq!(
        std::fs::read(root.join("inference-host.json")).unwrap(),
        before
    );
    assert_artifacts(root);
    let recovery = login("agent", false);
    accept(&recovery);
    let (s, d) = pending(&recovery);
    super::configuration::fail_next_save(super::configuration::SaveFault::Panic);
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = api::configure_inference_host(
            config,
            issuer.sign_admission(snapshot).unwrap(),
            document(),
        );
    }));
    assert!(unwound.is_err());
    rejected(s, d);
    assert_eq!(
        std::fs::read(root.join("inference-host.json")).unwrap(),
        before
    );
    assert_artifacts(root);
    for a in actors {
        let _ = a.logout();
    }
    for a in [&a, &b, &c, &recovered] {
        let _ = a.logout();
    }
}
fn restart(root: &Path) {
    let config_bytes = std::fs::read(root.join("inference-host.json")).unwrap();
    assert!(api::get_inference_policy_status().unwrap()["installed"]
        .as_bool()
        .unwrap());
    let a = login("human", true);
    accept(&a);
    assert_eq!(
        std::fs::read(root.join("inference-host.json")).unwrap(),
        config_bytes
    );
    let issuer = NativeSigner::open_sealed(&root.join("project-issuer.sealed"), &[27; 32]).unwrap();
    let mut snapshot: AdmissionSnapshot =
        serde_json::from_slice(&std::fs::read(root.join("signed-fixture.json")).unwrap()).unwrap();
    snapshot.sequence += 1;
    snapshot.subjects[0].revoked = true;
    let (s, d) = pending(&a);
    api::replace_inference_actor_admission(issuer.sign_admission(snapshot).unwrap()).unwrap();
    rejected(s, d);
    assert!(a.policy_scope().is_err());
    assert!(api::login_inference_actor(
        "human".into(),
        selection("human", true),
        "human".into(),
        300
    )
    .is_err());
    let agent = login("agent", false);
    assert!(admit_inference(request(), &agent, POLICY_COLD_BUDGET_BYTES).is_err());
    install(std::slice::from_ref(&agent));
    accept(&agent);
    let generation = load_policy_status().unwrap().generation;
    let deadline = now() + 2;
    api::install_inference_policy(
        serde_json::to_string(&rules(std::slice::from_ref(&agent))).unwrap(),
        deadline,
        generation,
    )
    .unwrap();
    let (s, d) = pending(&agent);
    while now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    rejected(s, d);
    assert!(admit_inference(request(), &agent, POLICY_COLD_BUDGET_BYTES).is_err());
    install(std::slice::from_ref(&agent));
    accept(&agent);
    // Exhaustion never wraps into a previously authenticated client generation.
    let (s, d) = pending(&agent);
    super::registry::set_epoch_for_test(u64::MAX - 1);
    api::disconnect_inference_controller();
    rejected(s, d);
    assert!(agent.policy_scope().is_err());
    assert!(api::login_inference_actor(
        "agent".into(),
        selection("agent", false),
        "agent".into(),
        300
    )
    .is_err());
}
fn assert_artifacts(root: &Path) {
    let mut pending = vec![root.to_path_buf()];
    let mut bytes = 0u64;
    while let Some(path) = pending.pop() {
        for e in std::fs::read_dir(path).unwrap() {
            let e = e.unwrap();
            let metadata = e.metadata().unwrap();
            assert!(!e.file_type().unwrap().is_symlink());
            if metadata.is_dir() {
                let name = e.file_name();
                let name = name.to_string_lossy();
                assert!(
                    !name.starts_with("inference-signer-")
                        && !name.starts_with("inference-config-"),
                    "leftover temporary directory {name}"
                );
                pending.push(e.path());
            } else {
                bytes = bytes.checked_add(metadata.len()).unwrap();
            }
        }
    }
    assert!(
        bytes < 1024 * 1024,
        "isolated fixture exceeded 1 MiB retention budget"
    );
}

#[test]
fn signed_configuration_authenticates_every_selected_trust_and_source_field() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let key = ed25519_dalek::SigningKey::from_bytes(&[61; 32]);
    let configuration = InferenceHostConfiguration {
        installer_identifier: "urn:installer".into(),
        installer_public_key: key.verifying_key().to_bytes(),
        project_id: "project".into(),
        project_issuer: "issuer".into(),
        project_public_key: [7; 32],
    };
    super::configuration::save(&root, configuration, document(), &key).unwrap();
    let path = root.join("inference-host.json");
    let original = std::fs::read(&path).unwrap();
    let canonical: serde_json::Value = serde_json::from_slice(&original).unwrap();
    for field in 0..12 {
        let mut changed = canonical.clone();
        match field {
            0 => changed["version"] = serde_json::json!(2),
            1 => changed["configuration"]["installer_identifier"] = serde_json::json!("urn:other"),
            2 => {
                changed["configuration"]["installer_public_key"][0] =
                    serde_json::json!(key.verifying_key().to_bytes()[0] ^ 1)
            }
            3 => changed["configuration"]["project_id"] = serde_json::json!("other"),
            4 => changed["configuration"]["project_issuer"] = serde_json::json!("other"),
            5 => changed["configuration"]["project_public_key"][0] = serde_json::json!(8),
            6 => changed["document"]["project_id"] = serde_json::json!("other"),
            7 => changed["document"]["document_id"] = serde_json::json!("other"),
            8 => changed["document"]["source_id"] = serde_json::json!("other"),
            9 => changed["document"]["revision"] = serde_json::json!(2),
            10 => changed["document"]["source_digest"][0] = serde_json::json!(2),
            _ => changed["document"]["author_subject"] = serde_json::json!("unadmitted"),
        }
        let bytes = serde_json::to_vec(&changed).unwrap();
        assert_ne!(bytes, original);
        std::fs::write(&path, &bytes).unwrap();
        assert!(
            super::configuration::load(&root, &key).is_err(),
            "unsigned field {field}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    std::fs::write(&path, &original).unwrap();
    assert!(super::configuration::load(&root, &key).is_ok());
    let wrong = ed25519_dalek::SigningKey::from_bytes(&[62; 32]);
    assert!(super::configuration::load(&root, &wrong).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_artifacts(&root);
}

fn tree_snapshot(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut output = std::collections::BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    let mut total = 0u64;
    while let Some(path) = pending.pop() {
        for e in std::fs::read_dir(path).unwrap() {
            let e = e.unwrap();
            let kind = e.file_type().unwrap();
            assert!(!kind.is_symlink());
            if kind.is_dir() {
                pending.push(e.path());
                output.insert(
                    e.path().strip_prefix(root).unwrap().to_path_buf(),
                    Vec::new(),
                );
            } else {
                total = total.checked_add(e.metadata().unwrap().len()).unwrap();
                assert!(total <= 1024 * 1024);
                output.insert(
                    e.path().strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(e.path()).unwrap(),
                );
            }
        }
    }
    output
}
