//! E05.2 — two OS-process IPC evidence. Never PhysicalNetwork / two-host.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::harness::qualification::{
    EvidenceClass, Instrument, InstrumentResult, InstrumentVerdict,
};

const CHILD_ENV: &str = "QDNF_TWO_PROCESS_CHILD";
const SOCK_ENV: &str = "QDNF_TWO_PROCESS_SOCK";
const PARENT_MSG: &[u8; 8] = b"QDNF-E05";
const CHILD_ACK: &[u8; 8] = b"ACK-E052";

static SPAWNED: AtomicBool = AtomicBool::new(false);

/// True after this process spawned a child, or this process *is* that child.
pub fn two_os_process_ipc_executed() -> bool {
    SPAWNED.load(Ordering::SeqCst) || is_child()
}

/// This module never claims two-host Ethernet.
pub fn physical_two_host_from_this_module() -> bool {
    false
}

/// Process-class evidence helper. Callers must not store PhysicalNetwork here.
pub fn process_ipc_evidence(spawned_child: bool) -> InstrumentResult {
    InstrumentResult {
        instrument: Instrument::GraphOracle,
        class: EvidenceClass::Process,
        verdict: if spawned_child {
            InstrumentVerdict::Pass
        } else {
            InstrumentVerdict::Fail
        },
        component_tables_pass: false,
    }
}

fn is_child() -> bool {
    match std::env::var(CHILD_ENV) {
        Ok(v) => v == "1",
        Err(_) => false,
    }
}

#[cfg(unix)]
fn io_err(err: std::io::Error) -> QdnfError {
    match err.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => QdnfError::Expired,
        std::io::ErrorKind::NotFound => QdnfError::Incomplete,
        std::io::ErrorKind::PermissionDenied => QdnfError::Denied,
        _ => QdnfError::PlatformUnsupported,
    }
}

/// Parent binds a Unix stream socket, spawns this test binary as a child, and
/// exchanges eight bytes. Evidence class is Process only.
pub fn run_two_os_process_ipc() -> Result<InstrumentResult, QdnfError> {
    if is_child() {
        #[cfg(unix)]
        {
            child_exchange()?;
            SPAWNED.store(true, Ordering::SeqCst);
            return Ok(process_ipc_evidence(false));
        }
        #[cfg(not(unix))]
        {
            return Err(QdnfError::PlatformUnsupported);
        }
    }
    #[cfg(not(unix))]
    {
        Err(QdnfError::PlatformUnsupported)
    }
    #[cfg(unix)]
    {
        parent_spawn_and_exchange()
    }
}

#[cfg(unix)]
fn child_exchange() -> Result<(), QdnfError> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::{Duration, Instant};

    let path = std::env::var(SOCK_ENV).map_err(|_| QdnfError::Incomplete)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut stream = loop {
        match UnixStream::connect(&path) {
            Ok(s) => break s,
            Err(_) => {
                if Instant::now() >= deadline {
                    return Err(QdnfError::Expired);
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(io_err)?;
    stream.write_all(PARENT_MSG).map_err(io_err)?;
    let mut buf = [0u8; 8];
    stream.read_exact(&mut buf).map_err(io_err)?;
    if &buf != CHILD_ACK {
        return Err(QdnfError::Malformed);
    }
    Ok(())
}

#[cfg(unix)]
fn parent_spawn_and_exchange() -> Result<InstrumentResult, QdnfError> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;
    use std::process::Command;
    use std::time::{Duration, Instant};

    let dir = tempfile::TempDir::new().map_err(|_| QdnfError::PlatformUnsupported)?;
    let sock = dir.path().join("qdnf-e05.sock");
    let listener = UnixListener::bind(&sock).map_err(io_err)?;
    listener.set_nonblocking(true).map_err(io_err)?;

    let exe = std::env::current_exe().map_err(|_| QdnfError::Incomplete)?;
    let test_name = std::thread::current()
        .name()
        .ok_or(QdnfError::Incomplete)?
        .to_owned();
    let mut child = Command::new(&exe)
        .args(["--exact", &test_name, "--nocapture", "--test-threads", "1"])
        .env(CHILD_ENV, "1")
        .env(SOCK_ENV, &sock)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|_| QdnfError::PlatformUnsupported)?;

    let deadline = Instant::now() + Duration::from_secs(15);
    let mut stream = loop {
        match listener.accept() {
            Ok((s, _)) => break s,
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::Interrupted =>
            {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(QdnfError::Expired);
                }
                if let Ok(Some(status)) = child.try_wait() {
                    if !status.success() {
                        return Err(QdnfError::Denied);
                    }
                    return Err(QdnfError::Closed);
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(io_err(e));
            }
        }
    };
    stream.set_nonblocking(false).map_err(io_err)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(io_err)?;
    let mut buf = [0u8; 8];
    stream.read_exact(&mut buf).map_err(io_err)?;
    if &buf != PARENT_MSG {
        let _ = child.kill();
        let _ = child.wait();
        return Err(QdnfError::Malformed);
    }
    stream.write_all(CHILD_ACK).map_err(io_err)?;
    let status = child.wait().map_err(|_| QdnfError::Closed)?;
    if !status.success() {
        return Err(QdnfError::Denied);
    }
    SPAWNED.store(true, Ordering::SeqCst);
    let evidence = process_ipc_evidence(true);
    if evidence.class != EvidenceClass::Process {
        return Err(QdnfError::Conflict);
    }
    if evidence.class == EvidenceClass::PhysicalNetwork {
        return Err(QdnfError::Conflict);
    }
    Ok(evidence)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::bearer::raw_ethernet::physical_two_host_qualified;

    #[cfg(unix)]
    #[test]
    fn two_os_process_ipc_exchanges_bytes() {
        assert!(!physical_two_host_from_this_module());
        assert!(!physical_two_host_qualified());
        let evidence = run_two_os_process_ipc().unwrap();
        assert!(two_os_process_ipc_executed());
        assert_eq!(evidence.class, EvidenceClass::Process);
        assert_ne!(evidence.class, EvidenceClass::PhysicalNetwork);
        assert!(!physical_two_host_from_this_module());
        assert!(!physical_two_host_qualified());
        if !is_child() {
            assert_eq!(evidence.verdict, InstrumentVerdict::Pass);
        }
    }

    #[test]
    fn process_evidence_is_never_physical_network() {
        let e = process_ipc_evidence(true);
        assert_eq!(e.class, EvidenceClass::Process);
        assert_ne!(e.class, EvidenceClass::PhysicalNetwork);
        assert!(!physical_two_host_from_this_module());
        assert!(!physical_two_host_qualified());
    }
}
