//! Native tests need their own process because request epochs, masks, and the
//! resident substrate belong to the process, even when a ring is stack-local.
//! Always close an earlier request first: process-start ACTIVE=true otherwise
//! hides missing request activation and reproduces none of the suite ordering.

pub(super) fn after_closed_request(name: &str, exercise: impl FnOnce()) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        const CHILD: &str = "QUALIA_COMPUTE_LIFECYCLE_TEST";
        if std::env::var(CHILD).as_deref() != Ok(name) {
            let exact = format!("inference::compute_universe::tests::{name}");
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", &exact, "--nocapture"])
                .env(CHILD, name)
                .output()
                .expect("run isolated compute lifecycle test");
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success()
                    && stdout.contains("running 1 test")
                    && stdout.contains(&format!("test {exact} ... ok"))
                    && stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
                "compute lifecycle child did not execute {exact}: {}\n{}\n{}",
                output.status,
                stdout,
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
    }
    let previous = super::begin_request_epoch();
    assert!(previous.activate(|| {}));
    drop(previous);
    assert!(!super::epoch::accepts(super::epoch::current()));
    exercise();
}
