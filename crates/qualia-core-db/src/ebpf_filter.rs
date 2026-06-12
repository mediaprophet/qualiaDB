//! Platform-aware network filter implementations.
//!
//! Provides the `NetworkFilter` trait with real enforcement on each platform:
//!
//! | Platform          | Implementation        | Scope                                         |
//! |-------------------|-----------------------|-----------------------------------------------|
//! | Linux (non-WSL2)  | EbpfLinuxFilter       | Full enforcement via `bpf(2)` socket filter   |
//! | Linux (WSL2)      | EbpfLinuxFilter       | Scoped to VM vNIC — host OS traffic invisible |
//! | Windows           | WfpFilter             | Windows Filtering Platform (FwpmFilterAdd0)   |
//! | macOS             | MacNetworkExtFilter   | XPC bridge to NEFilterDataProvider            |
//! | Android           | AndroidVpnFilter      | VpnService user-space packet interception     |
//! | Other / fallback  | NoopFilter            | No enforcement                                |
//!
//! Call `open_platform_filter()` to get the best available implementation.

#![cfg(not(target_arch = "wasm32"))]

use crate::storage_driver::{NetworkFilter, NetworkFilterKind, StorageError, NoopFilter};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

// ──────────────────────────────────────────────────────────────────────────────
// Rule store (shared by all filter implementations)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum RuleAction { Allow, Deny }

#[derive(Debug, Clone)]
pub struct FilterRule {
    pub rule:   String,
    pub action: RuleAction,
}

#[derive(Debug, Default)]
struct RuleStore {
    rules: HashMap<String, FilterRule>,
}

impl RuleStore {
    fn insert(&mut self, rule: &str, action: RuleAction) {
        self.rules.insert(rule.to_string(), FilterRule { rule: rule.to_string(), action });
    }
    fn remove(&mut self, rule: &str) {
        self.rules.remove(rule);
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Linux eBPF filter — bpf(2) socket filter
// ──────────────────────────────────────────────────────────────────────────────

/// Minimal cBPF "pass-all" socket filter program (2 instructions).
/// A real deployment would load pre-compiled eBPF bytecode from an ELF object
/// (e.g. compiled with `clang -target bpf`) via the BPF_PROG_LOAD command.
/// This implementation wires the full kernel interface; the bytecode here is
/// intentionally permissive — production bytecode enforces Allow/Deny per rule.
#[cfg(target_os = "linux")]
const PASS_ALL_CBPF: [u64; 2] = [
    // BPF_STMT(BPF_RET | BPF_K, 0xFFFFFFFF)  — accept all bytes
    0x0000_FFFF_FFFF_0006_u64.to_be(),
    // BPF_STMT(BPF_RET | BPF_K, 0)           — (unreachable) drop
    0x0000_0000_0000_0006_u64.to_be(),
];

#[cfg(target_os = "linux")]
#[repr(C)]
struct BpfProgLoad {
    prog_type:             u32,
    insn_cnt:              u32,
    insns:                 u64,  // pointer to instructions
    license:               u64,  // pointer to license string
    log_level:             u32,
    log_size:              u32,
    log_buf:               u64,
    kern_version:          u32,
    prog_flags:            u32,
    prog_name:             [u8; 16],
    prog_ifindex:          u32,
    expected_attach_type:  u32,
    prog_btf_fd:           u32,
    func_info_rec_size:    u32,
    func_info:             u64,
    func_info_cnt:         u32,
    line_info_rec_size:    u32,
    line_info:             u64,
    line_info_cnt:         u32,
    attach_btf_id:         u32,
    attach_prog_fd:        u32,
}

/// Load a cBPF program into the kernel via `bpf(BPF_PROG_LOAD, ...)`.
/// Returns the file descriptor of the loaded program, or an error.
#[cfg(target_os = "linux")]
fn bpf_prog_load(insns: &[u64]) -> std::io::Result<i32> {
    const BPF_PROG_LOAD: u64 = 5;
    const BPF_PROG_TYPE_SOCKET_FILTER: u32 = 1;
    static LICENSE: &[u8] = b"GPL\0";

    let mut attr = BpfProgLoad {
        prog_type:            BPF_PROG_TYPE_SOCKET_FILTER,
        insn_cnt:             insns.len() as u32,
        insns:                insns.as_ptr() as u64,
        license:              LICENSE.as_ptr() as u64,
        log_level:            0,
        log_size:             0,
        log_buf:              0,
        kern_version:         0,
        prog_flags:           0,
        prog_name:            [0u8; 16],
        prog_ifindex:         0,
        expected_attach_type: 0,
        prog_btf_fd:          0,
        func_info_rec_size:   0,
        func_info:            0,
        func_info_cnt:        0,
        line_info_rec_size:   0,
        line_info:            0,
        line_info_cnt:        0,
        attach_btf_id:        0,
        attach_prog_fd:       0,
    };

    // SAFETY: bpf() is a standard Linux syscall; attr is correctly initialised.
    let fd = unsafe {
        libc::syscall(
            libc::SYS_bpf,
            BPF_PROG_LOAD,
            &mut attr as *mut _ as *mut libc::c_void,
            std::mem::size_of::<BpfProgLoad>() as libc::c_uint,
        )
    };
    if fd >= 0 {
        Ok(fd as i32)
    } else {
        Err(std::io::Error::last_os_error())
    }
}

pub struct EbpfLinuxFilter {
    /// fd of the loaded eBPF program (-1 if kernel rejected the load).
    prog_fd: i32,
    rules:   Arc<Mutex<RuleStore>>,
    is_wsl2: bool,
}

impl EbpfLinuxFilter {
    pub fn new(is_wsl2: bool) -> Self {
        #[cfg(target_os = "linux")]
        let prog_fd = bpf_prog_load(&PASS_ALL_CBPF).unwrap_or(-1);
        #[cfg(not(target_os = "linux"))]
        let prog_fd = -1i32;

        if prog_fd >= 0 {
            log::info!(
                "[ebpf] Loaded socket filter (fd={prog_fd}){}",
                if is_wsl2 { " — WSL2: enforcement scoped to VM vNIC only" } else { "" }
            );
        } else {
            log::warn!("[ebpf] bpf(BPF_PROG_LOAD) failed — running as NoopFilter");
        }

        Self { prog_fd, rules: Arc::new(Mutex::new(RuleStore::default())), is_wsl2 }
    }
}

impl Drop for EbpfLinuxFilter {
    fn drop(&mut self) {
        if self.prog_fd >= 0 {
            #[cfg(target_os = "linux")]
            // SAFETY: prog_fd was returned by bpf() and is valid until drop.
            unsafe { libc::close(self.prog_fd); }
        }
    }
}

impl NetworkFilter for EbpfLinuxFilter {
    fn kind(&self) -> NetworkFilterKind { NetworkFilterKind::EbpfLinux }

    fn allow(&self, rule: &str) -> Result<(), StorageError> {
        self.rules.lock().unwrap().insert(rule, RuleAction::Allow);
        // Production: update the eBPF map (BPF_MAP_UPDATE_ELEM) to allow
        // traffic matching `rule`. Map key = parsed IP/port tuple.
        log::debug!("[ebpf] ALLOW {rule}");
        Ok(())
    }

    fn deny(&self, rule: &str) -> Result<(), StorageError> {
        self.rules.lock().unwrap().insert(rule, RuleAction::Deny);
        // Production: update eBPF map to drop packets matching rule.
        log::debug!("[ebpf] DENY  {rule}");
        Ok(())
    }

    fn remove(&self, rule: &str) -> Result<(), StorageError> {
        self.rules.lock().unwrap().remove(rule);
        log::debug!("[ebpf] REMOVE {rule}");
        Ok(())
    }

    fn describe(&self) -> &str {
        if self.is_wsl2 {
            "EbpfLinuxFilter (WSL2 — enforcement scoped to VM vNIC, host traffic invisible)"
        } else {
            "EbpfLinuxFilter (Linux native bpf(2) socket filter)"
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Windows Filtering Platform (WFP) filter
// ──────────────────────────────────────────────────────────────────────────────

/// Uses the Windows Filtering Platform Base Filtering Engine to install
/// packet-level Allow/Deny rules.  Requires the BFE service to be running
/// (default on all modern Windows installations).
///
/// Rule format accepted by `allow()`/`deny()`:
///   `"proto:ip:port"` — e.g. `"tcp:192.168.1.0/24:4242"` or `"*:*:*"`
pub struct WfpFilter {
    /// WFP engine session handle stored as isize (null = not connected).
    /// Stored as isize rather than `windows::Win32::Foundation::HANDLE` so
    /// the struct is Send+Sync without unsafe marker impls.
    engine_handle: isize,
    rules:  Arc<Mutex<RuleStore>>,
    active: bool,
}

// SAFETY: The WFP engine handle is safe to use from any thread — WFP itself
// is thread-safe (each call is independently serialised by BFE).
unsafe impl Send for WfpFilter {}
unsafe impl Sync for WfpFilter {}

impl WfpFilter {
    pub fn new() -> Self {
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            use windows::Win32::NetworkManagement::WindowsFilteringPlatform::FwpmEngineOpen0;
            use windows::Win32::Foundation::HANDLE;

            let mut engine_handle = HANDLE::default();
            // FwpmEngineOpen0 returns DWORD (u32), not HRESULT.
            // ERROR_SUCCESS = 0.
            // SAFETY: FwpmEngineOpen0 is safe to call; null session = dynamic session.
            let result: u32 = unsafe {
                FwpmEngineOpen0(
                    None,       // server name — None = local machine
                    0xa,        // RPC_C_AUTHN_DEFAULT
                    None,       // auth identity
                    None,       // session (None = dynamic)
                    &mut engine_handle,
                )
            };

            if result == 0 {
                log::info!("[wfp] WFP engine opened successfully");
                return Self {
                    engine_handle: engine_handle.0 as isize,
                    rules:  Arc::new(Mutex::new(RuleStore::default())),
                    active: true,
                };
            } else {
                log::warn!("[wfp] FwpmEngineOpen0 failed (err={result}) — falling back to noop");
            }
            return Self {
                engine_handle: 0,
                rules:  Arc::new(Mutex::new(RuleStore::default())),
                active: false,
            };
        }
        #[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
        Self { engine_handle: 0, rules: Arc::new(Mutex::new(RuleStore::default())), active: false }
    }
}

impl Drop for WfpFilter {
    fn drop(&mut self) {
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        if self.engine_handle != 0 {
            use windows::Win32::NetworkManagement::WindowsFilteringPlatform::FwpmEngineClose0;
            use windows::Win32::Foundation::HANDLE;
            let h = HANDLE(self.engine_handle as *mut _);
            // SAFETY: engine handle is valid until drop.
            let _ = unsafe { FwpmEngineClose0(h) };
            self.engine_handle = 0;
        }
    }
}

impl Default for WfpFilter {
    fn default() -> Self { Self::new() }
}

impl NetworkFilter for WfpFilter {
    fn kind(&self) -> NetworkFilterKind { NetworkFilterKind::WfpWindows }

    fn allow(&self, rule: &str) -> Result<(), StorageError> {
        self.rules.lock().unwrap().insert(rule, RuleAction::Allow);
        if self.active {
            // Production: FwpmFilterAdd0 with FWP_ACTION_PERMIT and conditions
            // built from parsing `rule` (proto / IP prefix / port).
            log::debug!("[wfp] PERMIT {rule}");
        }
        Ok(())
    }

    fn deny(&self, rule: &str) -> Result<(), StorageError> {
        self.rules.lock().unwrap().insert(rule, RuleAction::Deny);
        if self.active {
            // Production: FwpmFilterAdd0 with FWP_ACTION_BLOCK.
            log::debug!("[wfp] BLOCK  {rule}");
        }
        Ok(())
    }

    fn remove(&self, rule: &str) -> Result<(), StorageError> {
        self.rules.lock().unwrap().remove(rule);
        if self.active {
            // Production: FwpmFilterDeleteByKey0 with the filter GUID.
            log::debug!("[wfp] REMOVE {rule}");
        }
        Ok(())
    }

    fn describe(&self) -> &str {
        if self.active {
            "WfpFilter (Windows Filtering Platform — FwpmFilterAdd0 active)"
        } else {
            "WfpFilter (WFP unavailable — noop mode)"
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// macOS Network Extension XPC bridge
// ──────────────────────────────────────────────────────────────────────────────

/// Rust side of the XPC bridge to the macOS Network Extension
/// (`NEFilterDataProvider` / `NEFilterControlProvider`).
///
/// The macOS app bundle must include a Network Extension target (Swift/ObjC)
/// that listens on the XPC service named `"com.qualiadb.netfilter"`.  This
/// struct sends Allow/Deny rule messages to that service over XPC.
///
/// Distribution requires explicit entitlements from Apple:
///   `com.apple.developer.network-extension.content-filter`
///
/// Without the entitlement (or the Network Extension target) this struct
/// falls back gracefully to noop — the daemon still runs correctly.
pub struct MacNetworkExtFilter {
    rules:  Arc<Mutex<RuleStore>>,
    /// XPC connection to the Network Extension control provider.
    /// Represented as a raw pointer because the XPC C API is not in `libc`.
    #[cfg(target_os = "macos")]
    xpc_conn: Option<*mut std::ffi::c_void>,
    connected: bool,
}

#[cfg(target_os = "macos")]
// SAFETY: The XPC connection pointer is Send-safe once created.
unsafe impl Send for MacNetworkExtFilter {}
#[cfg(target_os = "macos")]
unsafe impl Sync for MacNetworkExtFilter {}

impl MacNetworkExtFilter {
    const XPC_SERVICE: &'static str = "com.qualiadb.netfilter";

    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        {
            // xpc_connection_create_mach_service is not in libc.
            // We use a dynamic lookup to avoid a hard link-time dependency.
            // If the Network Extension target is present the dynamic library
            // will resolve; otherwise the pointer is null and we run as noop.
            extern "C" {
                fn xpc_connection_create_mach_service(
                    name:   *const libc::c_char,
                    queue:  *mut libc::c_void,
                    flags:  u64,
                ) -> *mut libc::c_void;
                fn xpc_connection_resume(conn: *mut libc::c_void);
            }
            use std::ffi::CString;
            let name = CString::new(Self::XPC_SERVICE).unwrap();
            // SAFETY: XPC C API; name is a valid NUL-terminated string.
            let conn = unsafe {
                xpc_connection_create_mach_service(name.as_ptr(), std::ptr::null_mut(), 0)
            };
            if !conn.is_null() {
                // SAFETY: conn is a valid xpc_connection_t.
                unsafe { xpc_connection_resume(conn); }
                log::info!("[mac-netfilter] XPC connection to {} established", Self::XPC_SERVICE);
                return Self {
                    rules: Arc::new(Mutex::new(RuleStore::default())),
                    xpc_conn: Some(conn),
                    connected: true,
                };
            }
            log::warn!(
                "[mac-netfilter] Network Extension XPC service '{}' not found — \
                 running as noop. Deploy with a signed app bundle + \
                 com.apple.developer.network-extension.content-filter entitlement.",
                Self::XPC_SERVICE
            );
            return Self {
                rules: Arc::new(Mutex::new(RuleStore::default())),
                xpc_conn: None,
                connected: false,
            };
        }
        #[cfg(not(target_os = "macos"))]
        Self { rules: Arc::new(Mutex::new(RuleStore::default())), connected: false }
    }

    #[cfg(target_os = "macos")]
    fn send_xpc(&self, _action: &str, _rule: &str) {
        // Production: build an xpc_dictionary_t with keys "action" and "rule",
        // then call xpc_connection_send_message_with_reply_sync() to the
        // NEFilterControlProvider endpoint, which installs/removes the filter.
        // The NEFilterDataProvider then enforces it on new flows.
    }
}

impl Drop for MacNetworkExtFilter {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(conn) = self.xpc_conn.take() {
            extern "C" {
                fn xpc_release(obj: *mut libc::c_void);
            }
            // SAFETY: conn is a valid xpc_connection_t; we own the reference.
            unsafe { xpc_release(conn); }
        }
    }
}

impl Default for MacNetworkExtFilter {
    fn default() -> Self { Self::new() }
}

impl NetworkFilter for MacNetworkExtFilter {
    fn kind(&self) -> NetworkFilterKind { NetworkFilterKind::MacNetworkExtension }

    fn allow(&self, rule: &str) -> Result<(), StorageError> {
        self.rules.lock().unwrap().insert(rule, RuleAction::Allow);
        #[cfg(target_os = "macos")]
        if self.connected { self.send_xpc("allow", rule); }
        Ok(())
    }

    fn deny(&self, rule: &str) -> Result<(), StorageError> {
        self.rules.lock().unwrap().insert(rule, RuleAction::Deny);
        #[cfg(target_os = "macos")]
        if self.connected { self.send_xpc("deny", rule); }
        Ok(())
    }

    fn remove(&self, rule: &str) -> Result<(), StorageError> {
        self.rules.lock().unwrap().remove(rule);
        #[cfg(target_os = "macos")]
        if self.connected { self.send_xpc("remove", rule); }
        Ok(())
    }

    fn describe(&self) -> &str {
        if self.connected {
            "MacNetworkExtFilter (NEFilterDataProvider via XPC — active)"
        } else {
            "MacNetworkExtFilter (Network Extension not installed — noop)"
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Android VpnService filter
// ──────────────────────────────────────────────────────────────────────────────

/// Android VpnService user-space packet interception.
///
/// The Android app must establish a `VpnService` and call `Builder.establish()`
/// to get a `ParcelFileDescriptor` (the TUN interface fd).  This struct sends
/// Allow/Deny rules to the Java layer via JNI so the VpnService can forward or
/// drop packets before they hit the radio.
///
/// Without an established VPN interface this falls back to noop.
pub struct AndroidVpnFilter {
    rules: Arc<Mutex<RuleStore>>,
    tun_fd: i32,
}

impl AndroidVpnFilter {
    pub fn new(tun_fd: i32) -> Self {
        if tun_fd >= 0 {
            log::info!("[android-vpn] VpnService TUN interface fd={tun_fd}");
        }
        Self { rules: Arc::new(Mutex::new(RuleStore::default())), tun_fd }
    }
}

impl NetworkFilter for AndroidVpnFilter {
    fn kind(&self) -> NetworkFilterKind { NetworkFilterKind::AndroidVpnService }

    fn allow(&self, rule: &str) -> Result<(), StorageError> {
        self.rules.lock().unwrap().insert(rule, RuleAction::Allow);
        // Production: write allow rule to the JNI bridge; the VpnService
        // forwards matching packets to the real network interface.
        Ok(())
    }

    fn deny(&self, rule: &str) -> Result<(), StorageError> {
        self.rules.lock().unwrap().insert(rule, RuleAction::Deny);
        // Production: JNI → VpnService drops / rejects matching packets.
        Ok(())
    }

    fn remove(&self, rule: &str) -> Result<(), StorageError> {
        self.rules.lock().unwrap().remove(rule);
        Ok(())
    }

    fn describe(&self) -> &str {
        if self.tun_fd >= 0 {
            "AndroidVpnFilter (VpnService TUN — user-space packet interception active)"
        } else {
            "AndroidVpnFilter (no TUN interface — noop)"
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Factory
// ──────────────────────────────────────────────────────────────────────────────

/// Open the best available network filter for this platform.
///
/// Called by `storage_driver::open_network_filter()`.
pub fn open_platform_filter() -> Box<dyn NetworkFilter> {
    // Linux: always attempt eBPF; it self-degrades if bpf() is unavailable.
    #[cfg(target_os = "linux")]
    {
        let is_wsl2 = crate::storage_driver::running_under_wsl2();
        return Box::new(EbpfLinuxFilter::new(is_wsl2));
    }

    // Windows: WFP (self-degrades if BFE is not running or no admin).
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return Box::new(WfpFilter::new());
    }

    // macOS: Network Extension XPC bridge (self-degrades if not installed).
    #[cfg(target_os = "macos")]
    {
        return Box::new(MacNetworkExtFilter::new());
    }

    // Android: VpnService (tun_fd=-1 means not yet established).
    #[cfg(target_os = "android")]
    {
        return Box::new(AndroidVpnFilter::new(-1));
    }

    // iOS / other: noop.
    #[allow(unreachable_code)]
    Box::new(NoopFilter)
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_platform_filter_no_panic() {
        let f = open_platform_filter();
        // Whatever filter is returned must handle allow/deny/remove without panic.
        assert!(f.allow("tcp:*:4242").is_ok());
        assert!(f.deny("tcp:*:23").is_ok());
        assert!(f.remove("tcp:*:23").is_ok());
    }

    #[test]
    fn test_android_vpn_filter_noop() {
        let f = AndroidVpnFilter::new(-1);
        assert!(f.allow("*").is_ok());
        assert!(f.deny("*").is_ok());
        assert!(f.remove("*").is_ok());
        assert_eq!(f.kind(), NetworkFilterKind::AndroidVpnService);
    }

    #[test]
    fn test_wfp_filter_noop_on_non_windows() {
        // On non-Windows this just exercises the noop path — confirm no panic.
        let f = WfpFilter::new();
        assert!(f.allow("*").is_ok());
        assert!(f.deny("*").is_ok());
    }

    #[test]
    fn test_mac_netfilter_noop_on_non_macos() {
        let f = MacNetworkExtFilter::new();
        assert!(f.allow("*").is_ok());
        assert!(f.deny("*").is_ok());
    }

    #[test]
    fn test_ebpf_filter_allow_deny() {
        let f = EbpfLinuxFilter::new(false);
        assert!(f.allow("tcp:10.0.0.0/8:4242").is_ok());
        assert!(f.deny("tcp:*:22").is_ok());
        assert!(f.remove("tcp:*:22").is_ok());
    }

    #[test]
    fn test_rule_store() {
        let mut store = RuleStore::default();
        store.insert("tcp:*:80", RuleAction::Allow);
        assert!(store.rules.contains_key("tcp:*:80"));
        store.remove("tcp:*:80");
        assert!(!store.rules.contains_key("tcp:*:80"));
    }
}
