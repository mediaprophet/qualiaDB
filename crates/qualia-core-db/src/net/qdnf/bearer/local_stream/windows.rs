//! Parent-owned Windows named-pipe `StreamBytes`.
//!
//! Parent `CreateNamedPipeW` with a restrictive current-user DACL generates
//! `\\.\pipe\qbdl-…`. Child `CreateFileW` opens that exact name. No caller
//! bind path, no TCP/IP/DNS/libp2p. PID/OS credential is not cryptographic
//! peer authority. `CreateNamedPipeW` is linked from `kernel32` because
//! `Win32_System_Pipes` is not in crate features.

#![cfg(windows)]

use core::ffi::c_void;
use core::mem::size_of;
use core::sync::atomic::{AtomicU64, Ordering};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ACCESS_DENIED, ERROR_BROKEN_PIPE, ERROR_FILE_NOT_FOUND,
    ERROR_INSUFFICIENT_BUFFER, ERROR_INVALID_HANDLE, ERROR_NO_DATA, ERROR_PIPE_BUSY,
    ERROR_PIPE_CONNECTED, ERROR_PIPE_LISTENING, ERROR_PIPE_NOT_CONNECTED, GENERIC_READ,
    GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows::Win32::Security::{
    AddAccessAllowedAce, CopySid, GetLengthSid, GetTokenInformation, InitializeAcl,
    InitializeSecurityDescriptor, SetSecurityDescriptorDacl, SetSecurityDescriptorOwner, TokenUser,
    ACCESS_ALLOWED_ACE, ACL, ACL_REVISION, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES,
    SECURITY_DESCRIPTOR, TOKEN_QUERY, TOKEN_USER,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FlushFileBuffers, ReadFile, WriteFile, FILE_ATTRIBUTE_NORMAL,
    FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_NONE,
    OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId, OpenProcessToken,
};

use super::{FramedStream, StreamBytes, FRAME_CAP, WIRE_CAP};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

/// Parent-generated prefix. Child `from_exported` refuses any other path.
pub const PIPE_NAME_PREFIX: &str = r"\\.\pipe\qbdl-";

const PIPE_NAME_MAX: usize = 64;
const SECURITY_DESCRIPTOR_REVISION: u32 = 1;
const PIPE_NOWAIT: u32 = 1;
const PIPE_TYPE_BYTE: u32 = 0;
const PIPE_READMODE_BYTE: u32 = 0;
const PIPE_REJECT_REMOTE_CLIENTS: u32 = 8;
const PIPE_MODE: u32 =
    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS;
const ROLE_PARENT: u8 = 0;
const ROLE_CHILD: u8 = 1;
const TOKEN_INFO_CAP: usize = 256;
const SID_COPY_CAP: usize = 68;
const ACL_BUF_CAP: usize = 256;
const ERROR_INVALID_PARAMETER: u32 = 87;
const ERROR_INVALID_NAME: u32 = 123;
const ERROR_INVALID_ACL: u32 = 1336;
const ERROR_INVALID_SID: u32 = 1337;
const ERROR_INVALID_SECURITY_DESCR: u32 = 1338;
const CREATEFILE_TRIES: u32 = 8;
const WAIT_PIPE_MS: u32 = 50;

static NAME_SEQ: AtomicU64 = AtomicU64::new(1);

#[link(name = "kernel32")]
extern "system" {
    fn CreateNamedPipeW(
        lpname: *const u16,
        dwopenmode: u32,
        dwpipemode: u32,
        nmaxinstances: u32,
        noutbuffersize: u32,
        ninbuffersize: u32,
        ndefaulttimeout: u32,
        lpsecurityattributes: *const SECURITY_ATTRIBUTES,
    ) -> HANDLE;
    fn ConnectNamedPipe(hnamedpipe: HANDLE, lpoverlapped: *mut c_void) -> i32;
    fn GetNamedPipeClientProcessId(pipe: HANDLE, clientprocessid: *mut u32) -> i32;
    fn GetNamedPipeServerProcessId(pipe: HANDLE, serverprocessid: *mut u32) -> i32;
    fn SetNamedPipeHandleState(
        hnamedpipe: HANDLE,
        lpmode: *const u32,
        lpmaxcollectioncount: *const u32,
        lpcollectdatatimeout: *const u32,
    ) -> i32;
    fn WaitNamedPipeW(lpnamedpipename: *const u16, ntimeout: u32) -> i32;
}

/// Parent-generated pipe name. Not a bind-path argument.
#[derive(Clone, Copy)]
pub struct PipeName {
    wide: [u16; PIPE_NAME_MAX],
    units: u8,
}

impl PipeName {
    /// Parse a parent-exported name. Rejects anything not `\\.\pipe\qbdl-`
    /// plus hex/`-`. TCP/DNS/filesystem paths are [`QdnfError::Denied`].
    pub fn from_exported(name: &str) -> Result<Self, QdnfError> {
        if !name.starts_with(PIPE_NAME_PREFIX) {
            return Err(QdnfError::Denied);
        }
        let rest = &name[PIPE_NAME_PREFIX.len()..];
        if rest.is_empty() {
            return Err(QdnfError::Malformed);
        }
        if !rest.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
            return Err(QdnfError::Denied);
        }
        if name.len() >= PIPE_NAME_MAX {
            return Err(QdnfError::Capacity);
        }
        Self::from_ascii(name)
    }

    /// UTF-16 including the trailing NUL (for `CreateFileW`).
    pub fn as_wide(&self) -> &[u16] {
        &self.wide[..=self.units as usize]
    }

    /// UTF-16 units excluding NUL. For later env export (S3.3).
    pub fn as_text(&self) -> Result<String, QdnfError> {
        String::from_utf16(&self.wide[..self.units as usize]).map_err(|_| QdnfError::Malformed)
    }

    fn from_ascii(name: &str) -> Result<Self, QdnfError> {
        if !name.is_ascii() || name.len() >= PIPE_NAME_MAX {
            return Err(QdnfError::Capacity);
        }
        let mut wide = [0u16; PIPE_NAME_MAX];
        for (i, b) in name.bytes().enumerate() {
            wide[i] = b as u16;
        }
        Ok(Self {
            wide,
            units: name.len() as u8,
        })
    }

    fn pcwstr(&self) -> PCWSTR {
        PCWSTR(self.as_wide().as_ptr())
    }
}

/// Listening parent handle. Name is generated here; callers do not supply it.
pub struct NamedPipeListener {
    handle: HANDLE,
    name: PipeName,
}

impl NamedPipeListener {
    /// Parent `CreateNamedPipeW` with current-user DACL only.
    pub fn bind() -> Result<Self, QdnfError> {
        let name = generate_name()?;
        let handle = create_parent_pipe(&name)?;
        Ok(Self { handle, name })
    }

    /// Parent-generated name the child must open exactly.
    #[inline]
    pub fn name(&self) -> &PipeName {
        &self.name
    }

    /// Non-blocking accept. Same HANDLE becomes the parent stream.
    /// [`QdnfError::WouldBlock`] while no client (`ERROR_PIPE_LISTENING`).
    pub fn accept(&mut self) -> Result<NamedPipeStream, QdnfError> {
        if self.handle.is_invalid() {
            return Err(QdnfError::Closed);
        }
        let ok = unsafe { ConnectNamedPipe(self.handle, core::ptr::null_mut()) };
        if ok == 0 {
            let code = unsafe { GetLastError() }.0;
            if code != ERROR_PIPE_CONNECTED.0 {
                if code == ERROR_PIPE_LISTENING.0 || code == ERROR_NO_DATA.0 {
                    return Err(QdnfError::WouldBlock);
                }
                return Err(map_win32(code));
            }
        }
        let handle = self.handle;
        self.handle = INVALID_HANDLE_VALUE;
        NamedPipeStream::from_connected(handle, ROLE_PARENT)
    }
}

impl Drop for NamedPipeListener {
    fn drop(&mut self) {
        close_handle(&mut self.handle);
    }
}

/// Connected duplex byte pipe. `PIPE_NOWAIT`; no overlapped pinning.
pub struct NamedPipeStream {
    handle: HANDLE,
    closed: bool,
    local: ObservedLocator,
    peer: ObservedLocator,
}

impl NamedPipeStream {
    /// Adapter-observed local locator (PID + role). Not from a payload.
    #[inline]
    pub fn local_locator(&self) -> ObservedLocator {
        self.local
    }

    /// Adapter-observed peer locator (PID + role). Not cryptographic.
    #[inline]
    pub fn peer_locator(&self) -> ObservedLocator {
        self.peer
    }

    fn from_connected(handle: HANDLE, role: u8) -> Result<Self, QdnfError> {
        if handle.is_invalid() {
            return Err(QdnfError::PlatformUnsupported);
        }
        let local_pid = unsafe { GetCurrentProcessId() };
        let peer_pid = peer_pid(handle, role);
        Ok(Self {
            handle,
            closed: false,
            local: locator_from_pid(local_pid, role)?,
            peer: locator_from_pid(peer_pid, peer_role(role))?,
        })
    }
}

impl Drop for NamedPipeStream {
    fn drop(&mut self) {
        close_handle(&mut self.handle);
    }
}

impl StreamBytes for NamedPipeStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, QdnfError> {
        if self.closed || self.handle.is_invalid() {
            return Err(QdnfError::Closed);
        }
        if buf.is_empty() {
            return Ok(0);
        }
        let mut written = 0u32;
        match unsafe { WriteFile(self.handle, Some(buf), Some(&mut written), None) } {
            Ok(()) => {
                if written == 0 {
                    Err(QdnfError::WouldBlock)
                } else if written as usize > buf.len() {
                    Err(QdnfError::Range)
                } else {
                    Ok(written as usize)
                }
            }
            Err(e) => Err(map_error(e)),
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, QdnfError> {
        if self.closed || self.handle.is_invalid() {
            return Err(QdnfError::Closed);
        }
        if buf.is_empty() {
            return Ok(0);
        }
        let mut got = 0u32;
        match unsafe { ReadFile(self.handle, Some(buf), Some(&mut got), None) } {
            Ok(()) => {
                if got as usize > buf.len() {
                    Err(QdnfError::Range)
                } else {
                    Ok(got as usize)
                }
            }
            Err(e) => Err(map_error(e)),
        }
    }

    fn shutdown(&mut self) -> Result<(), QdnfError> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        if !self.handle.is_invalid() {
            let _ = unsafe { FlushFileBuffers(self.handle) };
            close_handle(&mut self.handle);
        }
        Ok(())
    }
}

/// Child `CreateFileW` on the parent-generated name only.
pub fn child_connect(name: &PipeName) -> Result<NamedPipeStream, QdnfError> {
    let handle = open_child_pipe(name)?;
    NamedPipeStream::from_connected(handle, ROLE_CHILD)
}

/// In-process pair: parent bind, child `CreateFileW`, parent accept.
pub fn named_pipe_pair() -> Result<(NamedPipeStream, NamedPipeStream), QdnfError> {
    let mut listener = NamedPipeListener::bind()?;
    let child = child_connect(listener.name())?;
    let parent = listener.accept()?;
    Ok((parent, child))
}

/// Wrap a connected handle with [`FramedStream::connect`]. Locators from the adapter.
pub fn framed_connect(
    io: NamedPipeStream,
    scope: ScopeEpoch,
    mtu: u16,
) -> Result<FramedStream<NamedPipeStream>, QdnfError> {
    let local = io.local_locator();
    let peer = io.peer_locator();
    FramedStream::connect(io, local, peer, scope, mtu)
}

/// In-process framed pair. Profile is existing `BearerProfile::LocalIpcV1`.
pub fn framed_named_pipe_pair(
    scope: ScopeEpoch,
    mtu: u16,
) -> Result<(FramedStream<NamedPipeStream>, FramedStream<NamedPipeStream>), QdnfError> {
    let (parent, child) = named_pipe_pair()?;
    Ok((
        framed_connect(parent, scope, mtu)?,
        framed_connect(child, scope, mtu)?,
    ))
}

fn generate_name() -> Result<PipeName, QdnfError> {
    let pid = unsafe { GetCurrentProcessId() };
    let tid = unsafe { GetCurrentThreadId() };
    let seq = NAME_SEQ.fetch_add(1, Ordering::Relaxed);
    let mut text = [0u8; PIPE_NAME_MAX];
    let n = write_generated_name(&mut text, pid, tid, seq)?;
    let s = core::str::from_utf8(&text[..n]).map_err(|_| QdnfError::Malformed)?;
    PipeName::from_ascii(s)
}

fn write_generated_name(out: &mut [u8], pid: u32, tid: u32, seq: u64) -> Result<usize, QdnfError> {
    const PREFIX: &[u8] = PIPE_NAME_PREFIX.as_bytes();
    let need = PREFIX.len() + 8 + 1 + 8 + 1 + 16;
    if out.len() < need {
        return Err(QdnfError::Capacity);
    }
    out[..PREFIX.len()].copy_from_slice(PREFIX);
    let mut i = PREFIX.len();
    write_hex(&mut out[i..], pid as u64, 8);
    i += 8;
    out[i] = b'-';
    i += 1;
    write_hex(&mut out[i..], tid as u64, 8);
    i += 8;
    out[i] = b'-';
    i += 1;
    write_hex(&mut out[i..], seq, 16);
    i += 16;
    Ok(i)
}

fn write_hex(out: &mut [u8], mut v: u64, digits: usize) {
    for i in (0..digits).rev() {
        let nibble = (v & 0xF) as u8;
        out[i] = if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + (nibble - 10)
        };
        v >>= 4;
    }
}

fn create_parent_pipe(name: &PipeName) -> Result<HANDLE, QdnfError> {
    with_restrictive_sa(|sa| {
        let handle = unsafe {
            CreateNamedPipeW(
                name.wide.as_ptr(),
                (PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE).0,
                PIPE_MODE,
                1,
                WIRE_CAP as u32,
                WIRE_CAP as u32,
                0,
                sa,
            )
        };
        if handle.is_invalid() {
            let code = unsafe { GetLastError() }.0;
            Err(map_win32(code))
        } else {
            Ok(handle)
        }
    })
}

fn open_child_pipe(name: &PipeName) -> Result<HANDLE, QdnfError> {
    let mut last = QdnfError::Incomplete;
    for _ in 0..CREATEFILE_TRIES {
        match unsafe {
            CreateFileW(
                name.pcwstr(),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        } {
            Ok(handle) => {
                let mode = PIPE_READMODE_BYTE | PIPE_NOWAIT;
                let ok = unsafe {
                    SetNamedPipeHandleState(handle, &mode, core::ptr::null(), core::ptr::null())
                };
                if ok == 0 {
                    let code = unsafe { GetLastError() }.0;
                    let _ = unsafe { CloseHandle(handle) };
                    return Err(map_win32(code));
                }
                return Ok(handle);
            }
            Err(e) => {
                last = map_error(e);
                if last != QdnfError::WouldBlock && last != QdnfError::Incomplete {
                    return Err(last);
                }
                let _ = unsafe { WaitNamedPipeW(name.wide.as_ptr(), WAIT_PIPE_MS) };
            }
        }
    }
    Err(last)
}

fn with_restrictive_sa<T, F>(f: F) -> Result<T, QdnfError>
where
    F: FnOnce(*const SECURITY_ATTRIBUTES) -> Result<T, QdnfError>,
{
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.map_err(map_error)?;
    let result = restrictive_sa_with_token(token, f);
    let _ = unsafe { CloseHandle(token) };
    result
}

fn restrictive_sa_with_token<T, F>(token: HANDLE, f: F) -> Result<T, QdnfError>
where
    F: FnOnce(*const SECURITY_ATTRIBUTES) -> Result<T, QdnfError>,
{
    let mut needed = 0u32;
    #[repr(align(8))]
    struct AlignedToken([u8; TOKEN_INFO_CAP]);
    let mut token_storage = AlignedToken([0u8; TOKEN_INFO_CAP]);
    let token_buf = &mut token_storage.0;
    match unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            Some(token_buf.as_mut_ptr() as *mut c_void),
            token_buf.len() as u32,
            &mut needed,
        )
    } {
        Ok(()) => {}
        Err(e) => {
            if win32_code(&e) == ERROR_INSUFFICIENT_BUFFER.0 {
                return Err(QdnfError::Capacity);
            }
            return Err(map_error(e));
        }
    }
    let user = unsafe { &*(token_buf.as_ptr() as *const TOKEN_USER) };
    let src_sid = user.User.Sid;
    let sid_len = unsafe { GetLengthSid(src_sid) };
    if sid_len == 0 || sid_len as usize > SID_COPY_CAP {
        return Err(QdnfError::Capacity);
    }
    #[repr(align(4))]
    struct AlignedSid([u8; SID_COPY_CAP]);
    let mut sid_storage = AlignedSid([0u8; SID_COPY_CAP]);
    let sid_copy = &mut sid_storage.0;
    unsafe { CopySid(sid_len, PSID(sid_copy.as_mut_ptr() as *mut c_void), src_sid) }
        .map_err(map_error)?;
    let sid = PSID(sid_copy.as_mut_ptr() as *mut c_void);

    let ace_sid = size_of::<ACCESS_ALLOWED_ACE>() - size_of::<u32>() + sid_len as usize;
    let acl_size = size_of::<ACL>() + ace_sid;
    if acl_size > ACL_BUF_CAP {
        return Err(QdnfError::Capacity);
    }
    #[repr(align(4))]
    struct AlignedAcl([u8; ACL_BUF_CAP]);
    let mut acl_storage = AlignedAcl([0u8; ACL_BUF_CAP]);
    let acl_buf = &mut acl_storage.0;
    unsafe {
        InitializeAcl(
            acl_buf.as_mut_ptr() as *mut ACL,
            acl_size as u32,
            ACL_REVISION,
        )
    }
    .map_err(map_error)?;
    let access = FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0;
    unsafe { AddAccessAllowedAce(acl_buf.as_mut_ptr() as *mut ACL, ACL_REVISION, access, sid) }
        .map_err(map_error)?;

    let mut sd = SECURITY_DESCRIPTOR::default();
    let psd = PSECURITY_DESCRIPTOR(&mut sd as *mut SECURITY_DESCRIPTOR as *mut c_void);
    unsafe { InitializeSecurityDescriptor(psd, SECURITY_DESCRIPTOR_REVISION) }
        .map_err(map_error)?;
    unsafe { SetSecurityDescriptorDacl(psd, true, Some(acl_buf.as_ptr() as *const ACL), false) }
        .map_err(map_error)?;
    unsafe { SetSecurityDescriptorOwner(psd, Some(sid), false) }.map_err(map_error)?;

    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: psd.0,
        bInheritHandle: false.into(),
    };
    f(&sa)
}

fn peer_pid(handle: HANDLE, role: u8) -> u32 {
    let mut pid = 0u32;
    let ok = if role == ROLE_PARENT {
        unsafe { GetNamedPipeClientProcessId(handle, &mut pid) }
    } else {
        unsafe { GetNamedPipeServerProcessId(handle, &mut pid) }
    };
    if ok == 0 || pid == 0 {
        unsafe { GetCurrentProcessId() }
    } else {
        pid
    }
}

fn peer_role(role: u8) -> u8 {
    if role == ROLE_PARENT {
        ROLE_CHILD
    } else {
        ROLE_PARENT
    }
}

fn locator_from_pid(pid: u32, role: u8) -> Result<ObservedLocator, QdnfError> {
    let mut bytes = [0u8; 5];
    bytes[..4].copy_from_slice(&pid.to_le_bytes());
    bytes[4] = role;
    ObservedLocator::from_slice(&bytes)
}

fn close_handle(handle: &mut HANDLE) {
    if !handle.is_invalid() {
        let _ = unsafe { CloseHandle(*handle) };
        *handle = INVALID_HANDLE_VALUE;
    }
}

fn win32_code(err: &windows::core::Error) -> u32 {
    let hr = err.code().0 as u32;
    if hr & 0xFFFF_0000 == 0x8007_0000 {
        hr & 0xFFFF
    } else {
        hr
    }
}

fn map_error(err: windows::core::Error) -> QdnfError {
    map_win32(win32_code(&err))
}

fn map_win32(code: u32) -> QdnfError {
    if code == ERROR_NO_DATA.0 || code == ERROR_PIPE_LISTENING.0 || code == ERROR_PIPE_BUSY.0 {
        QdnfError::WouldBlock
    } else if code == ERROR_BROKEN_PIPE.0
        || code == ERROR_PIPE_NOT_CONNECTED.0
        || code == ERROR_INVALID_HANDLE.0
    {
        QdnfError::Closed
    } else if code == ERROR_ACCESS_DENIED.0 {
        QdnfError::Denied
    } else if code == ERROR_FILE_NOT_FOUND.0 {
        QdnfError::Incomplete
    } else if code == ERROR_INVALID_PARAMETER
        || code == ERROR_INVALID_NAME
        || code == ERROR_INVALID_ACL
        || code == ERROR_INVALID_SID
        || code == ERROR_INVALID_SECURITY_DESCR
    {
        QdnfError::Malformed
    } else {
        QdnfError::PlatformUnsupported
    }
}

const _: () = assert!(FRAME_CAP == 8192);
const _: () = assert!(WIRE_CAP == 8196);

#[cfg(all(test, windows))]
mod tests {
    use super::super::super::contract::Bearer;
    use super::{
        framed_connect, framed_named_pipe_pair, named_pipe_pair, NamedPipeListener, PipeName,
        QdnfError, ScopeEpoch, StreamBytes, PIPE_NAME_PREFIX,
    };
    use crate::net::qdnf::registries::BearerProfile;

    fn scope() -> ScopeEpoch {
        ScopeEpoch { scope: 1, epoch: 1 }
    }

    #[test]
    fn parent_generates_qbdl_pipe_name() {
        let listener = NamedPipeListener::bind().unwrap();
        let text = listener.name().as_text().unwrap();
        assert!(text.starts_with(PIPE_NAME_PREFIX));
        assert!(!text.contains('/'));
        assert_eq!(
            text.matches('\\').count(),
            PIPE_NAME_PREFIX.matches('\\').count()
        );
        let rest = &text[PIPE_NAME_PREFIX.len()..];
        assert!(!rest.contains('\\'));
        assert!(rest.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'));
        let parsed = PipeName::from_exported(&text).unwrap();
        assert_eq!(parsed.as_text().unwrap(), text);
    }

    #[test]
    fn child_rejects_arbitrary_incoming_path() {
        assert_eq!(
            PipeName::from_exported(r"\\.\pipe\not-ours").err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            PipeName::from_exported(r"\\.\pipe\qbdl-../x").err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            PipeName::from_exported(r"C:\Temp\qdnf.sock").err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            PipeName::from_exported(r"\\.\pipe\qbdl-").err(),
            Some(QdnfError::Malformed)
        );
        assert_eq!(PipeName::from_exported("").err(), Some(QdnfError::Denied));
    }

    #[test]
    fn tcp_dns_ip_paths_are_denied() {
        assert_eq!(
            PipeName::from_exported("127.0.0.1:0").err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            PipeName::from_exported("localhost:443").err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            PipeName::from_exported("http://127.0.0.1/").err(),
            Some(QdnfError::Denied)
        );
        assert_eq!(
            PipeName::from_exported("/tmp/qdnf.sock").err(),
            Some(QdnfError::Denied)
        );
    }

    #[test]
    fn in_process_pair_exchanges_length_prefixed_frame() {
        let (mut parent, mut child) = framed_named_pipe_pair(scope(), 1280).unwrap();
        let payload = [0xAAu8, 0xBB, 0xCC, 0xDD];
        let dest = parent.peer();
        assert_eq!(parent.send(&dest, &payload), Ok(payload.len()));
        let mut out = [0u8; 16];
        let (n, meta) = child.recv(&mut out).unwrap();
        assert_eq!(n, payload.len());
        assert_eq!(&out[..n], &payload);
        assert_eq!(meta.observed_source, child.peer());
        assert_eq!(meta.scope, scope());
        assert_eq!(meta.mtu, 1280);

        let back = [0x11u8, 0x22, 0x33];
        let dest = child.peer();
        assert_eq!(child.send(&dest, &back), Ok(back.len()));
        let (n, meta) = parent.recv(&mut out).unwrap();
        assert_eq!(&out[..n], &back);
        assert_eq!(meta.observed_source, parent.peer());
    }

    #[test]
    fn framed_stream_reuses_local_ipc_profile() {
        let (parent, child) = framed_named_pipe_pair(scope(), 1280).unwrap();
        assert_eq!(parent.profile(), BearerProfile::LocalIpcV1);
        assert!(parent.profile().native_independent());
        assert_eq!(parent.capabilities().profile, BearerProfile::LocalIpcV1);
        assert!(parent.capabilities().ordered);
        assert!(!parent.capabilities().group_delivery);
        assert!(!parent.capabilities().may_duplicate);
        assert_eq!(child.profile(), BearerProfile::LocalIpcV1);
        assert_eq!(parent.mtu(), 1280);
    }

    #[test]
    fn observed_locator_from_adapter_not_payload() {
        let (mut parent, mut child) = framed_named_pipe_pair(scope(), 1280).unwrap();
        let parent_loc = parent.locator();
        let parent_peer = parent.peer();
        let child_loc = child.locator();
        assert_eq!(parent_loc.as_slice().len(), 5);
        assert_eq!(parent_loc.as_slice()[4], 0);
        assert_eq!(child_loc.as_slice()[4], 1);
        assert_eq!(parent_peer.as_slice(), child_loc.as_slice());
        assert_ne!(parent_loc.as_slice(), child_loc.as_slice());

        let mut payload = [0u8; 8];
        payload.copy_from_slice(b"not-loc!");
        parent.send(&parent_peer, &payload).unwrap();
        let mut out = [0u8; 16];
        let (n, meta) = child.recv(&mut out).unwrap();
        assert_eq!(&out[..n], &payload);
        assert_eq!(meta.observed_source, child.peer());
        assert_ne!(meta.observed_source.as_slice(), &payload);
        assert_eq!(
            parent.send(&parent_loc, &payload),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn idle_read_maps_error_no_data_to_would_block() {
        let (mut parent, mut child) = named_pipe_pair().unwrap();
        let mut buf = [0u8; 8];
        assert_eq!(parent.read(&mut buf), Err(QdnfError::WouldBlock));
        assert_eq!(child.read(&mut buf), Err(QdnfError::WouldBlock));
        let mut fp = framed_connect(parent, scope(), 1280).unwrap();
        let mut fc = framed_connect(child, scope(), 1280).unwrap();
        assert_eq!(fp.recv(&mut buf), Err(QdnfError::WouldBlock));
        assert_eq!(fc.recv(&mut buf), Err(QdnfError::WouldBlock));
    }

    #[test]
    fn broken_pipe_maps_to_closed() {
        let (mut parent, mut child) = named_pipe_pair().unwrap();
        parent.shutdown().unwrap();
        let mut buf = [0u8; 8];
        let r = child.read(&mut buf);
        assert!(
            r == Err(QdnfError::Closed) || r == Ok(0),
            "broken pipe must be Closed or orderly EOF, got {r:?}"
        );

        let (parent, child) = named_pipe_pair().unwrap();
        let mut fp = framed_connect(parent, scope(), 1280).unwrap();
        let mut fc = framed_connect(child, scope(), 1280).unwrap();
        fp.shutdown().unwrap();
        assert_eq!(fc.recv(&mut buf), Err(QdnfError::Closed));
    }

    #[test]
    fn drain_after_shutdown_is_closed() {
        let (mut parent, mut child) = framed_named_pipe_pair(scope(), 1280).unwrap();
        let payload = [0x11u8, 0x22];
        parent.send(&parent.peer(), &payload).unwrap();
        parent.begin_drain().unwrap();
        assert_eq!(
            parent.send(&parent.peer(), &payload),
            Err(QdnfError::Closed)
        );
        let mut out = [0u8; 8];
        let (n, meta) = child.recv(&mut out).unwrap();
        assert_eq!(&out[..n], &payload);
        assert_eq!(meta.observed_source, child.peer());
        parent.shutdown().unwrap();
        assert_eq!(parent.begin_drain(), Err(QdnfError::Closed));
        assert_eq!(parent.drain_write(), Err(QdnfError::Closed));
        assert_eq!(
            parent.send(&parent.peer(), &payload),
            Err(QdnfError::Closed)
        );
        assert_eq!(parent.recv(&mut out), Err(QdnfError::Closed));
    }
}
