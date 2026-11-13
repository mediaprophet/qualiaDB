//! Real process termination at actual write/sync barriers, not power-loss proof.
use super::super::write::WriteStep;
use super::fixtures::fixture_at;
use super::*;
use crate::storage_driver::ranges::recovery::LookupLimits;
use std::{
    io::{Read, Write},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};
const CHILD_TEST: &str =
    "bundle::stream::storage::native::tests::process_crash::crash_writer_child";
const ROOT_ENV: &str = "QUALIA_QBS_CRASH_TEST_ROOT";
const POINT_ENV: &str = "QUALIA_QBS_CRASH_TEST_POINT";
const READY: &[u8] = b"QBS_CRASH_READY\n";
const PAYLOAD: &[u8] = b"actual-source-payload";
const MARKER: &[u8] = b"qualia-qbs-process-crash-fixture-v1";
fn point(name: &str) -> WriteStep {
    match name {
        "payload-written" => WriteStep::PayloadWritten,
        "payload-synced" => WriteStep::PayloadSynced,
        "proof-synced" => WriteStep::ProofSynced,
        "header-written" => WriteStep::HeaderWritten,
        "checkpoint-synced" => WriteStep::CheckpointSynced,
        _ => panic!("unknown crash barrier"),
    }
}
#[test]
fn crash_writer_child() {
    let Some(root) = std::env::var_os(ROOT_ENV) else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    assert_eq!(
        std::fs::read(root.join(".process-test-owner")).unwrap(),
        MARKER
    );
    let target = point(&std::env::var(POINT_ENV).unwrap());
    fixture_at(
        PAYLOAD,
        &root,
        [2; 32],
        0,
        |mut store, workspace, rt, pins, bound, f, payload, _, _, _| {
            let mut op = ticket(rt, payload.len(), 1);
            let mut scratch = vec![0; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut owner = AuthorityOwner::new();
            let mut grant = adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
            let mut stage = rt
                .storage_stage(&op, payload, &[0], &mut scratch, 20, Generation(1))
                .unwrap();
            store
                .write_leaf_with_hook(
                    &mut stage,
                    &mut grant,
                    pins,
                    workspace,
                    0,
                    &mut nodes,
                    &|| 20,
                    &|| Generation(1),
                    &mut |step| {
                        if step == target {
                            let mut stdout = std::io::stdout().lock();
                            stdout.write_all(READY).unwrap();
                            stdout.flush().unwrap();
                            drop(stdout);
                            loop {
                                thread::park();
                            }
                        }
                        Ok(())
                    },
                )
                .unwrap();
            panic!("writer returned without reaching requested barrier");
        },
    );
}
struct ProcessOwner {
    child: Child,
    reader: Option<thread::JoinHandle<()>>,
}
impl Drop for ProcessOwner {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
fn terminate_at(root: &std::path::Path, phase: &str) {
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD_TEST, "--nocapture", "--test-threads=1"])
        .env(ROOT_ENV, root)
        .env(POINT_ENV, phase)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut owned = ProcessOwner {
        child,
        reader: None,
    };
    let stdout = owned.child.stdout.take().unwrap();
    let (tx, rx) = mpsc::sync_channel(1);
    owned.reader = Some(thread::spawn(move || {
        let mut suffix = [0; READY.len()];
        let mut count = 0usize;
        for byte in stdout.bytes().take(8192) {
            let Ok(byte) = byte else { break };
            suffix.rotate_left(1);
            suffix[READY.len() - 1] = byte;
            count += 1;
            if count >= READY.len() && suffix.as_slice() == READY {
                let _ = tx.send(true);
                return;
            }
        }
        let _ = tx.send(false);
    }));
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(30)),
        Ok(true),
        "child barrier {phase}"
    );
    assert!(
        owned.child.try_wait().unwrap().is_none(),
        "child must still own live files"
    );
    owned.child.kill().unwrap();
    assert!(!owned.child.wait().unwrap().success());
    owned.reader.take().unwrap().join().unwrap();
}
#[test]
fn killed_writer_reopens_under_new_admission_and_reverifies_every_leaf() {
    for phase in [
        "payload-written",
        "payload-synced",
        "proof-synced",
        "header-written",
        "checkpoint-synced",
    ] {
        let root = tempfile::Builder::new()
            .prefix("qualia-qbs-crash-")
            .tempdir()
            .unwrap();
        std::fs::write(root.path().join(".process-test-owner"), MARKER).unwrap();
        terminate_at(root.path(), phase);
        // A fresh host-admitted snapshot includes the old uncertain charge.
        // Discovery does not refund it or convert checkpoint hints to marks.
        fixture_at(
            PAYLOAD,
            root.path(),
            [3; 32],
            1,
            |mut store, workspace, rt, pins, bound, f, payload, driver, quota, binding| {
                let layout = StoreLayout::for_binding(&binding).unwrap();
                let charge = quota.charged_usage();
                assert_eq!(charge, (layout.aggregate_allowance().unwrap() * 2, 2));
                let scope = quota.recovery_root_scope().unwrap();
                let mut found = [None; 1];
                let report = driver
                    .lookup_range_sources(
                        &scope,
                        [2; 32],
                        LookupLimits {
                            entries: 8,
                            io_steps: 64,
                        },
                        &mut found,
                    )
                    .unwrap();
                assert_eq!(report.matches, 1);
                let mut source = rt
                    .acquire_recovery_source(
                        driver,
                        &scope,
                        found[0].take().unwrap(),
                        layout.range_limits(),
                    )
                    .unwrap();
                assert!(!workspace.has_integrity_mark(0).unwrap());
                let mut op = ticket(rt, 0, 1);
                let mut scratch = vec![0; READ_SCRATCH_BYTES];
                let mut nodes = [StrongDigest::ZERO; 32];
                let mut owner = AuthorityOwner::new();
                let result = {
                    let mut grant =
                        adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                    let mut stage = rt
                        .storage_stage(&op, &[], &[], &mut scratch, 20, Generation(1))
                        .unwrap();
                    store.recover_leaf_from(
                        &mut stage,
                        &mut grant,
                        pins,
                        workspace,
                        &mut source,
                        0,
                        &mut nodes,
                        &|| 20,
                        &|| Generation(1),
                    )
                };
                finish(rt, &mut op);
                if phase == "payload-written" {
                    assert!(result.is_err());
                    assert!(!workspace.has_integrity_mark(0).unwrap());
                    assert_eq!(
                        store
                            .session
                            .observed_file_length(RangeFile::Payload)
                            .unwrap(),
                        0
                    );
                } else {
                    assert_eq!(result.unwrap().sequence, 1);
                    assert!(workspace.has_integrity_mark(0).unwrap());
                    let mut actual = vec![0; payload.len()];
                    store
                        .session
                        .read_exact_at(RangeFile::Payload, 0, &mut actual)
                        .unwrap();
                    assert_eq!(actual, payload);
                }
                rt.release_recovery_source(source).unwrap();
                assert_eq!(quota.charged_usage(), charge);
                store
            },
        );
    }
}
