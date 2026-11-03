//! Pure CLI boundary fixtures; these tests do not create or open OS credentials.
use super::{io, profile};
use clap::Parser;
use std::path::Path;

#[test]
fn operator_inputs_are_bounded_and_never_echo_policy_content() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("operator.json");
    std::fs::write(&p, b"1234").unwrap();
    assert_eq!(io::bytes(&p, 4).unwrap(), b"1234");
    std::fs::write(&p, b"12345").unwrap();
    assert!(io::bytes(&p, 4).is_err());
    assert!(io::bytes(dir.path(), 4).is_err());
    const MARKER: &str = "PRIVATE_OPERATOR_INPUT_MUST_NOT_BE_ECHOED";
    for text in [
        format!("{{\"{MARKER}\":1}}"),
        format!("{{\"actors\":[],\"model\":{{\"kind\":\"{MARKER}\"}}}}"),
        format!("{{{MARKER}"),
    ] {
        std::fs::write(&p, text).unwrap();
        let error = io::policy(&p).unwrap_err();
        assert!(!error.contains(MARKER), "raw operator field leaked");
        assert!(error.contains("line") && error.contains("column"));
    }
    std::fs::write(&p, [255, 254, 0]).unwrap();
    assert!(io::policy(&p).is_err());
    std::fs::write(&p, vec![b' '; 128 * 1024 + 1]).unwrap();
    assert!(io::policy(&p).is_err());
    let rules = qualia_core_db::llm_agent::governed::PolicyRules {
        actors: Default::default(),
        model: qualia_core_db::llm_agent::governed::DigestScope::Any,
        request: qualia_core_db::llm_agent::governed::DigestScope::Any,
        graph_scopes: Default::default(),
        program: Default::default(),
        norms: Default::default(),
        facts: Default::default(),
        epistemic: Default::default(),
        trace: Default::default(),
        required_claim: 0,
        temporal_invariant: 0,
        non_derogable: false,
    };
    let bytes = serde_json::to_vec(&rules).unwrap();
    std::fs::write(&p, &bytes).unwrap();
    let parsed = io::policy(&p).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&parsed).unwrap(),
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
    );
    // Parser acceptance is not policy installation; empty actor scope is still
    // refused by the actual policy host and this fixture never opens a vault.
}
#[test]
fn operator_commands_require_explicit_profile_and_action() {
    use crate::cli::{Cli, Commands, InferencePolicyAction};
    assert!(Cli::try_parse_from(["qualia-cli", "inference-policy", "status"]).is_err());
    assert!(
        Cli::try_parse_from(["qualia-cli", "inference-policy", "--profile", "profile"]).is_err()
    );
    let parsed = Cli::try_parse_from([
        "qualia-cli",
        "inference-policy",
        "--profile",
        "profile",
        "status",
    ])
    .unwrap();
    assert!(matches!(
        parsed.command,
        Commands::InferencePolicy {
            action: InferencePolicyAction::Status,
            ..
        }
    ));
    for action in ["init-vault", "installer-key"] {
        assert!(Cli::try_parse_from([
            "qualia-cli",
            "inference-policy",
            "--profile",
            "profile",
            action
        ])
        .is_ok());
    }
    assert!(Cli::try_parse_from([
        "qualia-cli",
        "inference-policy",
        "--profile",
        "profile",
        "install",
        "--rules",
        "input.json",
        "--expires-at",
        "10"
    ])
    .is_err());
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("not-created");
    assert!(profile::canonical(Path::new("relative"), true).is_err());
    assert!(profile::canonical(&missing, false).is_err());
    assert!(!missing.exists());
    let planned = profile::canonical(&missing, true).unwrap();
    assert_eq!(planned.file_name().unwrap(), "not-created");
    assert!(!missing.exists());
    assert_eq!(
        profile::canonical(dir.path(), false).unwrap(),
        dir.path().canonicalize().unwrap()
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}
struct ChildGuard(Option<std::process::Child>);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(c) = self.0.as_mut() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}
#[test]
fn profile_child_marker_is_exact_and_nonrecursive() {
    const MODE: &str = "QUALIA_CLI_POLICY_TEST_CHILD";
    const ROOT: &str = "QUALIA_CLI_POLICY_TEST_ROOT";
    const TEST:&str="handlers::inference_policy::integration_tests::profile_child_marker_is_exact_and_nonrecursive";
    if let Ok(mode) = std::env::var(MODE) {
        let root = std::path::PathBuf::from(std::env::var_os(ROOT).unwrap());
        assert_eq!(
            std::fs::read(root.join(".owned-cli-profile-fixture")).unwrap(),
            b"isolated no OS keys"
        );
        let result = profile::bind_child(&root, false);
        if mode == "valid" {
            assert_eq!(result.unwrap(), root.canonicalize().unwrap());
        } else {
            assert!(result.is_err());
        }
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".owned-cli-profile-fixture"),
        b"isolated no OS keys",
    )
    .unwrap();
    for mode in ["valid", "invalid", "missing", "mismatch"] {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", TEST, "--nocapture"])
            .env(MODE, mode)
            .env(ROOT, dir.path());
        command.env(
            "QUALIA_INFERENCE_POLICY_PROFILE_CHILD",
            if mode == "invalid" { "2" } else { "1" },
        );
        if mode == "missing" {
            command.env_remove("QUALIA_APP_META_DIR");
        } else {
            command.env(
                "QUALIA_APP_META_DIR",
                if mode == "mismatch" {
                    dir.path().parent().unwrap()
                } else {
                    dir.path()
                },
            );
        }
        let child = command
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut guard = ChildGuard(Some(child));
        let start = std::time::Instant::now();
        loop {
            if guard.0.as_mut().unwrap().try_wait().unwrap().is_some() {
                break;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(30),
                "profile binding recursed or timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let output = guard.0.take().unwrap().wait_with_output().unwrap();
        assert!(output.stdout.len() + output.stderr.len() <= 64 * 1024);
        let out = String::from_utf8_lossy(&output.stdout);
        let err = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{out}\n{err}");
        assert!(
            out.contains("running 1 test") && out.contains("1 passed; 0 failed; 0 ignored"),
            "{out}"
        );
    }
}
