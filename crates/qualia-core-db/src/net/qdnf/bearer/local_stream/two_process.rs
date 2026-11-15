//! Two OS-process local-stream spawn. Never PhysicalNetwork / two-host.
//!
//! Parent owns the endpoint (`CreateNamedPipeW` / `UnixIpcListener`). Child
//! connects to the exported name only. In-process `ipc_pair` is not a fallback.
//! The E05.2 8-byte ping is not the payload.

use core::sync::atomic::{AtomicBool, Ordering};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::net::qdnf::bearer::contract::Bearer;
use crate::net::qdnf::bearer::raw_ethernet::physical_two_host_qualified;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::harness::qualification::{
    EvidenceClass, Instrument, InstrumentResult, InstrumentVerdict,
};
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

use super::{encode_stream_frame, FramedStream, FRAME_CAP, LEN_SIZE};

pub const CHILD_ENV: &str = "QDNF_TWO_PROCESS_CHILD";
#[cfg(all(windows, target_arch = "x86_64"))]
pub const PIPE_ENV: &str = "QDNF_TWO_PROCESS_PIPE";
#[cfg(unix)]
pub const SOCK_ENV: &str = "QDNF_TWO_PROCESS_SOCK";

const ACCEPT_SECS: u64 = 15;
const IO_SECS: u64 = 5;
const ACCEPT_POLL_MS: u64 = 5;
const IO_POLL_MS: u64 = 1;
const MTU: u16 = 1280;

static SPAWNED: AtomicBool = AtomicBool::new(false);

const _: () = assert!(MTU as usize <= FRAME_CAP);
const _: () = assert!(LEN_SIZE == 4);

pub fn is_child() -> bool {
    match std::env::var(CHILD_ENV) {
        Ok(v) => v == "1",
        Err(_) => false,
    }
}

pub fn two_os_process_executed() -> bool {
    SPAWNED.load(Ordering::SeqCst) || is_child()
}

pub fn physical_two_host_from_this_module() -> bool {
    false
}

pub fn process_stream_evidence(spawned_child: bool) -> InstrumentResult {
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

fn scope() -> ScopeEpoch {
    ScopeEpoch { scope: 1, epoch: 1 }
}

fn test_command() -> Result<Command, QdnfError> {
    let exe = std::env::current_exe().map_err(|_| QdnfError::Incomplete)?;
    let test_name = std::thread::current()
        .name()
        .ok_or(QdnfError::Incomplete)?
        .to_owned();
    let mut cmd = Command::new(exe);
    cmd.args(["--exact", &test_name, "--nocapture", "--test-threads", "1"])
        .env(CHILD_ENV, "1")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    Ok(cmd)
}

fn kill_wait(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn wait_success(child: &mut Child) -> Result<(), QdnfError> {
    let status = child.wait().map_err(|_| QdnfError::Closed)?;
    if status.success() {
        Ok(())
    } else {
        Err(QdnfError::Denied)
    }
}

fn send_deadline<B: Bearer>(
    bearer: &mut B,
    dest: &ObservedLocator,
    frame: &[u8],
) -> Result<(), QdnfError> {
    let deadline = Instant::now() + Duration::from_secs(IO_SECS);
    loop {
        match bearer.send(dest, frame) {
            Ok(_) => return Ok(()),
            Err(QdnfError::WouldBlock) => {
                if Instant::now() >= deadline {
                    return Err(QdnfError::Expired);
                }
                std::thread::sleep(Duration::from_millis(IO_POLL_MS));
            }
            Err(e) => return Err(e),
        }
    }
}

fn recv_deadline<B: Bearer>(
    bearer: &mut B,
    out: &mut [u8],
) -> Result<(usize, crate::net::qdnf::bearer::contract::RecvMeta), QdnfError> {
    let deadline = Instant::now() + Duration::from_secs(IO_SECS);
    loop {
        match bearer.recv(out) {
            Ok(v) => return Ok(v),
            Err(QdnfError::WouldBlock) => {
                if Instant::now() >= deadline {
                    return Err(QdnfError::Expired);
                }
                std::thread::sleep(Duration::from_millis(IO_POLL_MS));
            }
            Err(e) => return Err(e),
        }
    }
}

fn assert_process_not_physical() {
    assert!(!physical_two_host_from_this_module());
    assert!(!physical_two_host_qualified());
    let evidence = process_stream_evidence(true);
    assert_eq!(evidence.class, EvidenceClass::Process);
    assert_ne!(evidence.class, EvidenceClass::PhysicalNetwork);
}

#[cfg(all(windows, target_arch = "x86_64"))]
mod win {
    use super::super::windows::{
        child_connect, framed_connect, NamedPipeListener, NamedPipeStream, PipeName,
        PIPE_NAME_PREFIX,
    };
    use super::*;

    pub fn parent_bind_spawn_accept() -> Result<(FramedStream<NamedPipeStream>, Child), QdnfError> {
        let (io, child) = parent_bind_spawn_raw()?;
        let mut child = child;
        match framed_connect(io, scope(), MTU) {
            Ok(framed) => Ok((framed, child)),
            Err(e) => {
                kill_wait(&mut child);
                Err(e)
            }
        }
    }

    pub fn parent_bind_spawn_raw() -> Result<(NamedPipeStream, Child), QdnfError> {
        let mut listener = NamedPipeListener::bind()?;
        let name = listener.name().as_text()?;
        let mut child = test_command()?
            .env(PIPE_ENV, &name)
            .spawn()
            .map_err(|_| QdnfError::PlatformUnsupported)?;
        SPAWNED.store(true, Ordering::SeqCst);
        match accept_named_pipe(&mut listener, &mut child) {
            Ok(io) => Ok((io, child)),
            Err(e) => Err(e),
        }
    }

    fn accept_named_pipe(
        listener: &mut NamedPipeListener,
        child: &mut Child,
    ) -> Result<NamedPipeStream, QdnfError> {
        let deadline = Instant::now() + Duration::from_secs(ACCEPT_SECS);
        loop {
            match listener.accept() {
                Ok(s) => return Ok(s),
                Err(QdnfError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        kill_wait(child);
                        return Err(QdnfError::Expired);
                    }
                    if let Ok(Some(status)) = child.try_wait() {
                        return Err(if status.success() {
                            QdnfError::Closed
                        } else {
                            QdnfError::Denied
                        });
                    }
                    std::thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                }
                Err(e) => {
                    kill_wait(child);
                    return Err(e);
                }
            }
        }
    }

    pub fn child_connect_framed() -> Result<FramedStream<NamedPipeStream>, QdnfError> {
        framed_connect(child_connect_raw()?, scope(), MTU)
    }

    pub fn child_connect_raw() -> Result<NamedPipeStream, QdnfError> {
        let exported = std::env::var(PIPE_ENV).map_err(|_| QdnfError::Incomplete)?;
        let name = PipeName::from_exported(&exported)?;
        child_connect(&name)
    }

    pub fn refuse_tcp_and_foreign() {
        assert_eq!(
            PipeName::from_exported("127.0.0.1:0").err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            PipeName::from_exported(r"\\.\pipe\not-ours").err(),
            Some(QdnfError::Denied)
        );
        assert!(PIPE_NAME_PREFIX.starts_with(r"\\.\pipe\qbdl-"));
    }
}

#[cfg(unix)]
mod uds {
    use super::super::unix::{UnixIpcListener, UnixStreamBytes};
    use super::*;
    use std::path::Path;

    pub fn parent_bind_spawn_accept() -> Result<(FramedStream<UnixStreamBytes>, Child), QdnfError> {
        let (io, mut child) = parent_bind_spawn_raw()?;
        match io.into_framed(scope(), MTU) {
            Ok(framed) => Ok((framed, child)),
            Err(e) => {
                kill_wait(&mut child);
                Err(e)
            }
        }
    }

    pub fn parent_bind_spawn_raw() -> Result<(UnixStreamBytes, Child), QdnfError> {
        let mut listener = UnixIpcListener::bind()?;
        let mut child = test_command()?
            .env(SOCK_ENV, listener.path())
            .spawn()
            .map_err(|_| QdnfError::PlatformUnsupported)?;
        SPAWNED.store(true, Ordering::SeqCst);
        let deadline = Instant::now() + Duration::from_secs(ACCEPT_SECS);
        loop {
            match listener.accept() {
                Ok(s) => return Ok((s, child)),
                Err(QdnfError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        kill_wait(&mut child);
                        return Err(QdnfError::Expired);
                    }
                    if let Ok(Some(status)) = child.try_wait() {
                        return Err(if status.success() {
                            QdnfError::Closed
                        } else {
                            QdnfError::Denied
                        });
                    }
                    std::thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                }
                Err(e) => {
                    kill_wait(&mut child);
                    return Err(e);
                }
            }
        }
    }

    pub fn child_connect_framed() -> Result<FramedStream<UnixStreamBytes>, QdnfError> {
        child_connect_raw()?.into_framed(scope(), MTU)
    }

    pub fn child_connect_raw() -> Result<UnixStreamBytes, QdnfError> {
        let path = std::env::var(SOCK_ENV).map_err(|_| QdnfError::Incomplete)?;
        let deadline = Instant::now() + Duration::from_secs(IO_SECS);
        loop {
            match UnixStreamBytes::connect(Path::new(&path)) {
                Ok(s) => return Ok(s),
                Err(QdnfError::WouldBlock) | Err(QdnfError::Incomplete) => {
                    if Instant::now() >= deadline {
                        return Err(QdnfError::Expired);
                    }
                    std::thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn refuse_tcp_and_foreign() {
        assert_eq!(
            UnixStreamBytes::connect(Path::new("127.0.0.1:0")).err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            UnixStreamBytes::connect(Path::new(r"\\.\pipe\not-ours")).err(),
            Some(QdnfError::Denied)
        );
    }
}

#[cfg(all(windows, target_arch = "x86_64"))]
#[test]
fn two_os_process_exchanges_length_prefixed_frame() {
    assert_process_not_physical();
    if is_child() {
        let mut framed = win::child_connect_framed().unwrap();
        assert_eq!(framed.profile(), BearerProfile::LocalIpcV1);
        let mut out = [0u8; 32];
        let (n, meta) = recv_deadline(&mut framed, &mut out).unwrap();
        assert_eq!(&out[..n], b"\xA1\xB2\xC3\xD4");
        assert_ne!(out[0], b'{');
        assert_eq!(meta.observed_source, framed.peer());
        let dest = framed.peer();
        send_deadline(&mut framed, &dest, b"\x11\x22").unwrap();
        return;
    }
    let (mut framed, mut child) = win::parent_bind_spawn_accept().unwrap();
    assert_eq!(framed.profile(), BearerProfile::LocalIpcV1);
    let dest = framed.peer();
    if let Err(e) = send_deadline(&mut framed, &dest, b"\xA1\xB2\xC3\xD4") {
        kill_wait(&mut child);
        panic!("{e:?}");
    }
    let mut out = [0u8; 32];
    let (n, _) = match recv_deadline(&mut framed, &mut out) {
        Ok(v) => v,
        Err(e) => {
            kill_wait(&mut child);
            panic!("{e:?}");
        }
    };
    assert_eq!(&out[..n], b"\x11\x22");
    wait_success(&mut child).unwrap();
    assert!(two_os_process_executed());
    assert_process_not_physical();
}

#[cfg(unix)]
#[test]
fn two_os_process_exchanges_length_prefixed_frame() {
    assert_process_not_physical();
    if is_child() {
        let mut framed = uds::child_connect_framed().unwrap();
        assert_eq!(framed.profile(), BearerProfile::LocalIpcV1);
        let mut out = [0u8; 32];
        let (n, meta) = recv_deadline(&mut framed, &mut out).unwrap();
        assert_eq!(&out[..n], b"\xA1\xB2\xC3\xD4");
        assert_ne!(out[0], b'{');
        assert_eq!(meta.observed_source, framed.peer());
        let dest = framed.peer();
        send_deadline(&mut framed, &dest, b"\x11\x22").unwrap();
        return;
    }
    let (mut framed, mut child) = uds::parent_bind_spawn_accept().unwrap();
    assert_eq!(framed.profile(), BearerProfile::LocalIpcV1);
    let dest = framed.peer();
    if let Err(e) = send_deadline(&mut framed, &dest, b"\xA1\xB2\xC3\xD4") {
        kill_wait(&mut child);
        panic!("{e:?}");
    }
    let mut out = [0u8; 32];
    let (n, _) = match recv_deadline(&mut framed, &mut out) {
        Ok(v) => v,
        Err(e) => {
            kill_wait(&mut child);
            panic!("{e:?}");
        }
    };
    assert_eq!(&out[..n], b"\x11\x22");
    wait_success(&mut child).unwrap();
    assert!(two_os_process_executed());
    assert_process_not_physical();
}

#[cfg(all(windows, target_arch = "x86_64"))]
#[test]
fn child_refuses_tcp_and_foreign_pipe_name() {
    win::refuse_tcp_and_foreign();
}

#[cfg(unix)]
#[test]
fn child_refuses_tcp_and_foreign_pipe_name() {
    uds::refuse_tcp_and_foreign();
}

#[test]
fn wire_is_length_prefixed_binary_not_json() {
    let payload = [0xA1u8, 0xB2, 0xC3, 0xD4];
    let mut wire = [0u8; 16];
    let n = encode_stream_frame(&payload, &mut wire).unwrap();
    assert_ne!(wire[0], b'{');
    let declared = u32::from_be_bytes(wire[..LEN_SIZE].try_into().unwrap());
    assert_eq!(declared as usize, payload.len());
    assert_eq!(&wire[LEN_SIZE..n], &payload);
    assert_eq!(n, LEN_SIZE + payload.len());
}
