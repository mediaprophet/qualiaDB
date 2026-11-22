//! Unix-domain stream backend for [`FramedStream`].
//!
//! Parent binds a socket path it generated under a private temp directory.
//! Child connects to that exported path. No incoming bind path, no IP/DNS/TCP
//! fallback. Profile is existing [`BearerProfile::LocalIpcV1`].
//!
//! Adapter-observed locators use OS peer credentials. PID/uid is not
//! cryptographic peer authority. Windows named-pipe evidence is a distinct
//! class; this host's Windows tests do not prove Unix.

#![cfg(unix)]

use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

use super::{FramedStream, StreamBytes};

/// Socket file name under the parent-owned temp directory.
pub const SOCKET_FILE: &str = "qdnf-local.sock";

const LOC_TAG_PEERCRED: u8 = 0x01;

/// Native Unix-domain never falls back to IP, DNS, TCP, or libp2p.
pub const fn silent_ip_fallback() -> bool {
    false
}

/// Windows named-pipe delivery is a distinct evidence class.
pub const fn windows_process_evidence() -> bool {
    false
}

/// Two-OS-process Linux/WSL delivery is not established by unit tests here.
#[cfg_attr(not(test), allow(dead_code))]
pub const fn linux_wsl_two_process_evidence() -> bool {
    false
}

/// This adapter never claims two-host Ethernet.
#[cfg_attr(not(test), allow(dead_code))]
pub const fn physical_two_host_from_this_module() -> bool {
    false
}

/// Existing local-ipc-v1 profile. Not a new registry variant.
#[cfg_attr(not(test), allow(dead_code))]
pub const fn profile() -> BearerProfile {
    BearerProfile::LocalIpcV1
}

/// Non-blocking Unix stream. Implements [`StreamBytes`] for [`FramedStream`].
pub struct UnixStreamBytes {
    stream: Option<UnixStream>,
}

impl UnixStreamBytes {
    /// Child connects to a parent-exported filesystem path. Does not bind.
    pub fn connect(path: &Path) -> Result<Self, QdnfError> {
        reject_non_local_path(path)?;
        let stream = UnixStream::connect(path).map_err(map_io)?;
        from_connected(stream)
    }

    /// Same-process pair. Not two-OS-process evidence and not Windows evidence.
    pub fn pair() -> Result<(Self, Self), QdnfError> {
        let (a, b) = UnixStream::pair().map_err(map_io)?;
        Ok((from_connected(a)?, from_connected(b)?))
    }

    /// Local pid/uid locator. Not cryptographic identity.
    pub fn local_locator(&self) -> ObservedLocator {
        locator_from_ids(local_pid(), local_uid())
    }

    /// Peer credentials from the kernel. Not a payload field.
    pub fn peer_locator(&self) -> Result<ObservedLocator, QdnfError> {
        let stream = self.stream.as_ref().ok_or(QdnfError::Closed)?;
        let (pid, uid) = peer_ids(stream)?;
        Ok(locator_from_ids(pid, uid))
    }

    /// Wrap with adapter-observed locators. Profile stays [`BearerProfile::LocalIpcV1`].
    pub fn into_framed(self, scope: ScopeEpoch, mtu: u16) -> Result<FramedStream<Self>, QdnfError> {
        let local = self.local_locator();
        let peer = self.peer_locator()?;
        self.into_framed_with(local, peer, scope, mtu)
    }

    /// Wrap with caller-supplied adapter locators. Never taken from framed bytes.
    pub fn into_framed_with(
        self,
        local: ObservedLocator,
        peer: ObservedLocator,
        scope: ScopeEpoch,
        mtu: u16,
    ) -> Result<FramedStream<Self>, QdnfError> {
        FramedStream::connect(self, local, peer, scope, mtu)
    }
}

impl StreamBytes for UnixStreamBytes {
    fn write(&mut self, buf: &[u8]) -> Result<usize, QdnfError> {
        let stream = self.stream.as_mut().ok_or(QdnfError::Closed)?;
        if buf.is_empty() {
            return Ok(0);
        }
        match stream.write(buf) {
            Ok(n) => Ok(n),
            Err(e) => Err(map_io(e)),
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, QdnfError> {
        let stream = self.stream.as_mut().ok_or(QdnfError::Closed)?;
        if buf.is_empty() {
            return Ok(0);
        }
        match stream.read(buf) {
            Ok(n) => Ok(n),
            Err(e) => Err(map_io(e)),
        }
    }

    fn shutdown(&mut self) -> Result<(), QdnfError> {
        if let Some(stream) = self.stream.as_mut() {
            match stream.shutdown(std::net::Shutdown::Both) {
                Ok(()) => {}
                Err(e) => {
                    let mapped = map_io(e);
                    if mapped != QdnfError::Closed {
                        return Err(mapped);
                    }
                }
            }
        }
        self.stream = None;
        Ok(())
    }
}

/// Parent-owned Unix listener. Generates a private temp dir and socket path.
pub struct UnixIpcListener {
    _dir: tempfile::TempDir,
    listener: UnixListener,
    path: PathBuf,
}

impl UnixIpcListener {
    /// Create a 0700 temp dir, choose `SOCKET_FILE`, bind. No caller bind path.
    pub fn bind() -> Result<Self, QdnfError> {
        let dir = tempfile::TempDir::new().map_err(|_| QdnfError::PlatformUnsupported)?;
        restrict_mode(dir.path(), 0o700)?;
        let path = dir.path().join(SOCKET_FILE);
        let listener = UnixListener::bind(&path).map_err(map_io)?;
        restrict_mode(&path, 0o600)?;
        listener.set_nonblocking(true).map_err(map_io)?;
        Ok(Self {
            _dir: dir,
            listener,
            path,
        })
    }

    /// Parent-generated path to export to the child. Not an incoming locator.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Accept one connected stream. Incoming `SocketAddr` is not a locator.
    pub fn accept(&mut self) -> Result<UnixStreamBytes, QdnfError> {
        match self.listener.accept() {
            Ok((stream, _addr)) => from_connected(stream),
            Err(e) => Err(map_io(e)),
        }
    }

    /// Accept and wrap with adapter-observed locators.
    pub fn accept_framed(
        &mut self,
        scope: ScopeEpoch,
        mtu: u16,
    ) -> Result<FramedStream<UnixStreamBytes>, QdnfError> {
        self.accept()?.into_framed(scope, mtu)
    }
}

fn from_connected(stream: UnixStream) -> Result<UnixStreamBytes, QdnfError> {
    stream.set_nonblocking(true).map_err(map_io)?;
    Ok(UnixStreamBytes {
        stream: Some(stream),
    })
}

fn reject_non_local_path(path: &Path) -> Result<(), QdnfError> {
    let s = path.to_str().ok_or(QdnfError::Malformed)?;
    if s.is_empty() {
        return Err(QdnfError::Incomplete);
    }
    if s.as_bytes().contains(&0) {
        return Err(QdnfError::Denied);
    }
    if s.contains("://") {
        return Err(QdnfError::Denied);
    }
    if s.contains(':') && !s.contains('/') {
        return Err(QdnfError::Denied);
    }
    if !path.is_absolute() {
        return Err(QdnfError::Denied);
    }
    Ok(())
}

fn restrict_mode(path: &Path, mode: u32) -> Result<(), QdnfError> {
    let mut perms = std::fs::metadata(path).map_err(map_io)?.permissions();
    perms.set_mode(mode);
    std::fs::set_permissions(path, perms).map_err(map_io)
}

fn map_io(err: std::io::Error) -> QdnfError {
    if let Some(raw) = err.raw_os_error() {
        if raw == libc::EAGAIN || raw == libc::EWOULDBLOCK || raw == libc::EINTR {
            return QdnfError::WouldBlock;
        }
        if raw == libc::EPIPE
            || raw == libc::ECONNRESET
            || raw == libc::ENOTCONN
            || raw == libc::ECONNABORTED
            || raw == libc::EBADF
        {
            return QdnfError::Closed;
        }
        if raw == libc::EACCES || raw == libc::EPERM {
            return QdnfError::Denied;
        }
        if raw == libc::ENOENT {
            return QdnfError::Incomplete;
        }
        if raw == libc::ETIMEDOUT {
            return QdnfError::Expired;
        }
        if raw == libc::EEXIST {
            return QdnfError::Conflict;
        }
    }
    match err.kind() {
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted => QdnfError::WouldBlock,
        std::io::ErrorKind::BrokenPipe
        | std::io::ErrorKind::ConnectionReset
        | std::io::ErrorKind::ConnectionAborted
        | std::io::ErrorKind::NotConnected
        | std::io::ErrorKind::UnexpectedEof => QdnfError::Closed,
        std::io::ErrorKind::PermissionDenied => QdnfError::Denied,
        std::io::ErrorKind::NotFound => QdnfError::Incomplete,
        std::io::ErrorKind::TimedOut => QdnfError::Expired,
        std::io::ErrorKind::AlreadyExists => QdnfError::Conflict,
        std::io::ErrorKind::InvalidInput => QdnfError::Malformed,
        _ => QdnfError::PlatformUnsupported,
    }
}

fn locator_from_ids(pid: u32, uid: u32) -> ObservedLocator {
    let mut bytes = [0u8; 32];
    bytes[0] = LOC_TAG_PEERCRED;
    bytes[1..5].copy_from_slice(&pid.to_be_bytes());
    bytes[5..9].copy_from_slice(&uid.to_be_bytes());
    ObservedLocator { bytes, len: 9 }
}

fn local_pid() -> u32 {
    unsafe { libc::getpid() as u32 }
}

fn local_uid() -> u32 {
    unsafe { libc::getuid() as u32 }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn peer_ids(stream: &UnixStream) -> Result<(u32, u32), QdnfError> {
    let mut cred = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = core::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let rc = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut cred as *mut libc::ucred as *mut libc::c_void,
            &mut len,
        )
    };
    if rc < 0 {
        return Err(map_io(std::io::Error::last_os_error()));
    }
    Ok((cred.pid as u32, cred.uid))
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn peer_ids(stream: &UnixStream) -> Result<(u32, u32), QdnfError> {
    let mut pid: libc::pid_t = 0;
    let mut len = core::mem::size_of::<libc::pid_t>() as libc::socklen_t;
    let rc = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_LOCAL,
            libc::LOCAL_PEERPID,
            &mut pid as *mut libc::pid_t as *mut libc::c_void,
            &mut len,
        )
    };
    if rc < 0 {
        return Err(map_io(std::io::Error::last_os_error()));
    }
    Ok((pid as u32, u32::MAX))
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios"
)))]
fn peer_ids(stream: &UnixStream) -> Result<(u32, u32), QdnfError> {
    let _ = stream.as_raw_fd();
    Err(QdnfError::PlatformUnsupported)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::net::qdnf::bearer::contract::Bearer;
    use crate::net::qdnf::registries::BearerProfile;
    use std::time::{Duration, Instant};

    fn loc(b: u8) -> ObservedLocator {
        ObservedLocator::from_slice(&[b]).unwrap()
    }

    fn scope() -> ScopeEpoch {
        ScopeEpoch { scope: 1, epoch: 1 }
    }

    fn accept_ready(listener: &mut UnixIpcListener) -> UnixStreamBytes {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match listener.accept() {
                Ok(s) => return s,
                Err(QdnfError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        panic!("accept deadline");
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => panic!("{e:?}"),
            }
        }
    }

    #[test]
    fn parent_generated_path_child_connect_framed_roundtrip() {
        let mut listener = UnixIpcListener::bind().unwrap();
        let path = listener.path().to_path_buf();
        let child = std::thread::spawn(move || {
            let stream = UnixStreamBytes::connect(&path).unwrap();
            let mut framed = stream
                .into_framed_with(loc(0x02), loc(0x01), scope(), 1280)
                .unwrap();
            assert_eq!(framed.profile(), BearerProfile::LocalIpcV1);
            assert!(framed.profile().native_independent());
            let mut out = [0u8; 32];
            let (n, meta) = loop {
                match framed.recv(&mut out) {
                    Ok(v) => break v,
                    Err(QdnfError::WouldBlock) => std::thread::sleep(Duration::from_millis(1)),
                    Err(e) => panic!("{e:?}"),
                }
            };
            assert_eq!(&out[..n], b"hello-uds");
            assert_eq!(meta.observed_source, loc(0x01));
            framed.send(&loc(0x01), b"ack-uds").unwrap();
            framed.shutdown().unwrap();
        });
        let server = accept_ready(&mut listener);
        let mut framed = server
            .into_framed_with(loc(0x01), loc(0x02), scope(), 1280)
            .unwrap();
        assert_eq!(framed.profile(), BearerProfile::LocalIpcV1);
        framed.send(&loc(0x02), b"hello-uds").unwrap();
        let mut out = [0u8; 32];
        let (n, meta) = loop {
            match framed.recv(&mut out) {
                Ok(v) => break v,
                Err(QdnfError::WouldBlock) => std::thread::sleep(Duration::from_millis(1)),
                Err(e) => panic!("{e:?}"),
            }
        };
        assert_eq!(&out[..n], b"ack-uds");
        assert_eq!(meta.observed_source, loc(0x02));
        framed.shutdown().unwrap();
        child.join().unwrap();
    }

    #[test]
    fn eagain_would_block_then_resume_retains_write_cursor() {
        let (a, mut b) = UnixStreamBytes::pair().unwrap();
        let mut framed = a
            .into_framed_with(loc(0x01), loc(0x02), scope(), 1280)
            .unwrap();
        let mut idle = [0u8; 16];
        assert_eq!(framed.recv(&mut idle), Err(QdnfError::WouldBlock));
        let junk = [0xAAu8; 256];
        let mut spins = 0u32;
        loop {
            spins = spins.saturating_add(1);
            if spins > 1_000_000 {
                panic!("send buffer did not fill");
            }
            match framed.inner_mut().write(&junk) {
                Ok(0) | Err(QdnfError::WouldBlock) => break,
                Ok(_) => {}
                Err(e) => panic!("{e:?}"),
            }
        }
        let payload = [1u8, 2, 3, 4, 5];
        assert_eq!(
            framed.send(&loc(0x02), &payload),
            Err(QdnfError::WouldBlock)
        );
        assert!(framed.write_cursor().pending());
        let sealed_len = framed.write_cursor().sealed_len();
        assert!(sealed_len > 0);
        let mut sink = [0u8; 1024];
        spins = 0;
        loop {
            spins = spins.saturating_add(1);
            if spins > 1_000_000 {
                panic!("drain did not complete");
            }
            match framed.drain_write() {
                Ok(()) => break,
                Err(QdnfError::WouldBlock) => {
                    let _ = b.read(&mut sink);
                }
                Err(e) => panic!("{e:?}"),
            }
        }
        assert!(!framed.write_cursor().pending());
        assert_eq!(sealed_len, 4 + payload.len());
    }

    #[test]
    fn connect_rejects_tcp_url_and_abstract_paths() {
        assert_eq!(
            UnixStreamBytes::connect(Path::new("127.0.0.1:0")).err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            UnixStreamBytes::connect(Path::new("[::1]:80")).err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            UnixStreamBytes::connect(Path::new("tcp://127.0.0.1:1")).err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            UnixStreamBytes::connect(Path::new("http://example")).err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            UnixStreamBytes::connect(Path::new("")).err(),
            Some(QdnfError::Incomplete)
        );
        assert_eq!(
            UnixStreamBytes::connect(Path::new("relative.sock")).err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            UnixStreamBytes::connect(Path::new("\0abstract")).err(),
            Some(QdnfError::Denied)
        );
    }

    #[test]
    fn bind_owns_private_temp_socket_not_incoming_path() {
        let a = UnixIpcListener::bind().unwrap();
        let b = UnixIpcListener::bind().unwrap();
        assert!(a.path().is_absolute());
        assert_eq!(a.path().file_name().unwrap(), SOCKET_FILE);
        assert_ne!(a.path(), b.path());
        let mode = std::fs::metadata(a.path()).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let dir_mode = std::fs::metadata(a.path().parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700);
    }

    #[test]
    fn shutdown_then_io_is_closed() {
        let (mut a, _b) = UnixStreamBytes::pair().unwrap();
        a.shutdown().unwrap();
        a.shutdown().unwrap();
        assert_eq!(a.write(&[1]), Err(QdnfError::Closed));
        assert_eq!(a.read(&mut [0u8; 4]), Err(QdnfError::Closed));
    }

    #[test]
    fn framed_profile_is_local_ipc_v1() {
        let (a, _b) = UnixStreamBytes::pair().unwrap();
        let framed = a
            .into_framed_with(loc(0x01), loc(0x02), scope(), 1280)
            .unwrap();
        assert_eq!(profile(), BearerProfile::LocalIpcV1);
        assert_eq!(framed.profile(), BearerProfile::LocalIpcV1);
        assert_eq!(framed.capabilities().profile, BearerProfile::LocalIpcV1);
        assert!(framed.capabilities().ordered);
        assert!(!framed.capabilities().group_delivery);
        assert!(!framed.capabilities().may_duplicate);
        assert!(framed.profile().native_independent());
    }

    #[test]
    fn no_ip_dns_tcp_fallback() {
        assert!(!silent_ip_fallback());
        assert!(!windows_process_evidence());
        assert!(!linux_wsl_two_process_evidence());
        assert!(!physical_two_host_from_this_module());
        assert_eq!(
            UnixStreamBytes::connect(Path::new("localhost:8080")).err(),
            Some(QdnfError::Denied)
        );
    }

    #[test]
    fn peer_locator_is_adapter_observed() {
        let (a, b) = UnixStreamBytes::pair().unwrap();
        match a.peer_locator() {
            Ok(peer) => {
                assert_eq!(peer.as_slice()[0], LOC_TAG_PEERCRED);
                assert_ne!(peer.as_slice(), b"not-a-locator");
                let local_a = a.local_locator();
                let mut framed = a
                    .into_framed_with(local_a, peer, scope(), 1280)
                    .unwrap();
                framed.send(&peer, b"\xffpayload").unwrap();
                let mut other = b
                    .into_framed_with(loc(0x02), loc(0x01), scope(), 1280)
                    .unwrap();
                let mut out = [0u8; 16];
                let (n, meta) = loop {
                    match other.recv(&mut out) {
                        Ok(v) => break v,
                        Err(QdnfError::WouldBlock) => std::thread::sleep(Duration::from_millis(1)),
                        Err(e) => panic!("{e:?}"),
                    }
                };
                assert_eq!(&out[..n], b"\xffpayload");
                assert_eq!(meta.observed_source, loc(0x01));
                assert_ne!(meta.observed_source.as_slice(), &out[..n]);
            }
            Err(QdnfError::PlatformUnsupported) => {}
            Err(e) => panic!("{e:?}"),
        }
    }
}
