//! E05.2 — honest two-host / veth Ethernet probe.
//!
//! This probe never creates a veth pair, never opens a second host, and never
//! upgrades [`super::physical_two_host_qualified`] (that stays `false`). A
//! successful AF_PACKET `socket` is still not two-host evidence.

use crate::net::qdnf::errors::QdnfError;

#[cfg(target_os = "linux")]
use super::frame::DEV_ETHERTYPE;

/// Why two-host physical Ethernet is not qualified in this process.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwoHostProbeReason {
    /// Effective CAP_NET_ADMIN and/or CAP_NET_RAW is missing (`CapEff`).
    CapabilitiesMissing = 1,
    /// iproute2 `ip` is not on PATH; veth cannot be administered here.
    IpMissing = 2,
    /// Non-Linux, or the capability/AF_PACKET syscall is unavailable.
    PlatformUnsupported = 3,
}

/// Observed two-host gates. `veth_created` is never set by this probe.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TwoHostProbe {
    pub cap_net_admin: bool,
    pub cap_net_raw: bool,
    pub ip_present: bool,
    pub af_packet_attempted: bool,
    pub af_packet_ok: bool,
    pub veth_created: bool,
    pub reason: TwoHostProbeReason,
    pub error: QdnfError,
}

impl TwoHostProbe {
    /// This probe never claims two-process or two-host Ethernet.
    pub const fn claims_two_host(self) -> bool {
        false
    }
}

#[cfg(target_os = "linux")]
const CAP_NET_ADMIN: u32 = 12;
#[cfg(target_os = "linux")]
const CAP_NET_RAW: u32 = 13;
#[cfg(target_os = "linux")]
const LINUX_CAPABILITY_VERSION_3: u32 = 0x2008_0522;

#[cfg(target_os = "linux")]
#[repr(C)]
struct CapUserHeader {
    version: u32,
    pid: i32,
}

#[cfg(target_os = "linux")]
#[repr(C)]
struct CapUserData {
    effective: u32,
    permitted: u32,
    inheritable: u32,
}

/// Probe CapEff, `ip`, and AF_PACKET. Does not create veth or claim success.
pub fn probe_two_host() -> TwoHostProbe {
    #[cfg(not(target_os = "linux"))]
    {
        return TwoHostProbe {
            cap_net_admin: false,
            cap_net_raw: false,
            ip_present: ip_present(),
            af_packet_attempted: false,
            af_packet_ok: false,
            veth_created: false,
            reason: TwoHostProbeReason::PlatformUnsupported,
            error: QdnfError::PlatformUnsupported,
        };
    }
    #[cfg(target_os = "linux")]
    {
        let (cap_net_admin, cap_net_raw, caps_ok) = read_effective_caps();
        let ip = ip_present();
        let (af_attempted, af_ok) = probe_af_packet();
        let reason = if !caps_ok {
            TwoHostProbeReason::PlatformUnsupported
        } else if !cap_net_admin || !cap_net_raw {
            TwoHostProbeReason::CapabilitiesMissing
        } else if !ip {
            TwoHostProbeReason::IpMissing
        } else {
            // Caps and `ip` are present; this probe still does not create veth.
            TwoHostProbeReason::PlatformUnsupported
        };
        let error = match reason {
            TwoHostProbeReason::CapabilitiesMissing | TwoHostProbeReason::IpMissing => {
                QdnfError::Denied
            }
            TwoHostProbeReason::PlatformUnsupported => QdnfError::PlatformUnsupported,
        };
        TwoHostProbe {
            cap_net_admin,
            cap_net_raw,
            ip_present: ip,
            af_packet_attempted: af_attempted,
            af_packet_ok: af_ok,
            veth_created: false,
            reason,
            error,
        }
    }
}

fn ip_present() -> bool {
    const FIXED: [&str; 3] = ["/sbin/ip", "/usr/sbin/ip", "/usr/bin/ip"];
    let mut i = 0usize;
    while i < FIXED.len() {
        if std::path::Path::new(FIXED[i]).is_file() {
            return true;
        }
        i += 1;
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path) {
        if dir.join("ip").is_file() {
            return true;
        }
    }
    false
}

#[cfg(target_os = "linux")]
fn cap_bit(data: &[CapUserData; 2], bit: u32) -> bool {
    let idx = (bit / 32) as usize;
    if idx >= data.len() {
        return false;
    }
    (data[idx].effective & (1u32 << (bit % 32))) != 0
}

#[cfg(target_os = "linux")]
fn read_effective_caps() -> (bool, bool, bool) {
    if let Some(pair) = capget_effective() {
        return (pair.0, pair.1, true);
    }
    if let Some(pair) = parse_proc_capeff() {
        return (pair.0, pair.1, true);
    }
    (false, false, false)
}

#[cfg(target_os = "linux")]
fn capget_effective() -> Option<(bool, bool)> {
    let mut hdr = CapUserHeader {
        version: LINUX_CAPABILITY_VERSION_3,
        pid: 0,
    };
    let mut data = [
        CapUserData {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        },
        CapUserData {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        },
    ];
    let rc = unsafe {
        libc::syscall(
            libc::SYS_capget,
            &mut hdr as *mut CapUserHeader,
            data.as_mut_ptr(),
        )
    };
    if rc != 0 {
        return None;
    }
    Some((cap_bit(&data, CAP_NET_ADMIN), cap_bit(&data, CAP_NET_RAW)))
}

#[cfg(target_os = "linux")]
fn parse_proc_capeff() -> Option<(bool, bool)> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("CapEff:") {
            let hex = rest.trim();
            let bits = u64::from_str_radix(hex, 16).ok()?;
            let admin = (bits & (1u64 << CAP_NET_ADMIN)) != 0;
            let raw = (bits & (1u64 << CAP_NET_RAW)) != 0;
            return Some((admin, raw));
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn probe_af_packet() -> (bool, bool) {
    let proto = DEV_ETHERTYPE.to_be() as libc::c_int;
    let fd = unsafe { libc::socket(libc::AF_PACKET, libc::SOCK_RAW | libc::SOCK_CLOEXEC, proto) };
    if fd < 0 {
        return (true, false);
    }
    unsafe {
        libc::close(fd);
    }
    (true, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::bearer::raw_ethernet::{
        EthernetEvidence, physical_two_host_qualified, silent_ip_fallback,
    };

    #[test]
    fn two_host_probe_does_not_claim_physical_link() {
        let probe = probe_two_host();
        assert!(!physical_two_host_qualified());
        assert!(!probe.claims_two_host());
        assert!(!probe.veth_created);
        assert!(!silent_ip_fallback());
        assert!(!EthernetEvidence::AfPacketAttempt.is_physical_link());
        match probe.reason {
            TwoHostProbeReason::CapabilitiesMissing
            | TwoHostProbeReason::IpMissing
            | TwoHostProbeReason::PlatformUnsupported => {}
        }
        assert!(probe.error == QdnfError::Denied || probe.error == QdnfError::PlatformUnsupported);
    }

    #[test]
    fn this_environment_records_missing_caps_or_ip() {
        let probe = probe_two_host();
        assert!(!physical_two_host_qualified());
        assert!(!probe.veth_created);
        assert!(!probe.claims_two_host());
        #[cfg(target_os = "linux")]
        {
            assert!(probe.af_packet_attempted);
            assert!(!probe.af_packet_ok);
            assert!(!probe.cap_net_admin);
            assert!(!probe.cap_net_raw);
            assert!(!probe.ip_present);
            assert_eq!(probe.reason, TwoHostProbeReason::CapabilitiesMissing);
            assert_eq!(probe.error, QdnfError::Denied);
        }
        #[cfg(not(target_os = "linux"))]
        {
            assert_eq!(probe.reason, TwoHostProbeReason::PlatformUnsupported);
            assert!(!probe.af_packet_attempted);
        }
    }
}
