//! Linux AF_PACKET raw Ethernet. Privilege-gated; no IP/DNS/libp2p fallback.
//!
//! A successful `open` is `EthernetEvidence::AfPacketAttempt`. It is not
//! two-host physical-link evidence.

use std::mem;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

use super::super::contract::{check_frame_mtu, Bearer, RecvMeta};
use super::frame::{
    decapsulate_ethernet, encapsulate_ethernet, locator_from_mac, mac_from_locator, validate_ifname,
    DEV_ETHERTYPE, IFNAMSIZ,
};
use super::EthernetEvidence;

const WIRE_CAP: usize = 2048;

#[derive(Debug)]
pub struct RawEthernet {
    fd: Option<OwnedFd>,
    ifindex: i32,
    src_mac: [u8; 6],
    scope: ScopeEpoch,
    mtu: u16,
}

impl RawEthernet {
    pub fn open(interface: &[u8]) -> Result<Self, QdnfError> {
        validate_ifname(interface)?;
        let mut cname = [0 as libc::c_char; IFNAMSIZ];
        for (i, &b) in interface.iter().enumerate() {
            cname[i] = b as libc::c_char;
        }

        let proto = htons(DEV_ETHERTYPE) as libc::c_int;
        let sock_type = libc::SOCK_RAW | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC;
        let raw = unsafe { libc::socket(libc::AF_PACKET, sock_type, proto) };
        if raw < 0 {
            return Err(map_errno());
        }
        let owned = unsafe { OwnedFd::from_raw_fd(raw) };

        let ifindex = unsafe { libc::if_nametoindex(cname.as_ptr()) };
        if ifindex == 0 {
            return Err(QdnfError::Range);
        }

        let addr = libc::sockaddr_ll {
            sll_family: libc::AF_PACKET as libc::c_ushort,
            sll_protocol: htons(DEV_ETHERTYPE),
            sll_ifindex: ifindex as libc::c_int,
            sll_hatype: 0,
            sll_pkttype: 0,
            sll_halen: 6,
            sll_addr: [0; 8],
        };
        let rc = unsafe {
            libc::bind(
                owned.as_raw_fd(),
                &addr as *const libc::sockaddr_ll as *const libc::sockaddr,
                mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t,
            )
        };
        if rc < 0 {
            return Err(map_errno());
        }

        let src_mac = read_hwaddr(owned.as_raw_fd(), &cname)?;
        let mtu = read_mtu(owned.as_raw_fd(), &cname);

        Ok(Self {
            fd: Some(owned),
            ifindex: ifindex as i32,
            src_mac,
            scope: ScopeEpoch { scope: 0, epoch: 0 },
            mtu,
        })
    }

    pub fn ethernet_evidence_level(&self) -> EthernetEvidence {
        EthernetEvidence::AfPacketAttempt
    }

    fn fd(&self) -> Result<libc::c_int, QdnfError> {
        self.fd
            .as_ref()
            .map(OwnedFd::as_raw_fd)
            .ok_or(QdnfError::Closed)
    }
}

impl Bearer for RawEthernet {
    fn profile(&self) -> BearerProfile {
        BearerProfile::RawEthernetV1
    }

    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn scope(&self) -> ScopeEpoch {
        self.scope
    }

    fn send(&mut self, dest: &ObservedLocator, frame: &[u8]) -> Result<usize, QdnfError> {
        check_frame_mtu(frame.len(), self.mtu)?;
        let dst_mac = mac_from_locator(dest)?;
        let wire_len = 14usize.checked_add(frame.len()).ok_or(QdnfError::Range)?;
        if wire_len > WIRE_CAP {
            return Err(QdnfError::Capacity);
        }
        let fd = self.fd()?;
        let mut wire = [0u8; WIRE_CAP];
        let n = encapsulate_ethernet(&dst_mac, &self.src_mac, frame, &mut wire)?;
        let mut addr = libc::sockaddr_ll {
            sll_family: libc::AF_PACKET as libc::c_ushort,
            sll_protocol: htons(DEV_ETHERTYPE),
            sll_ifindex: self.ifindex,
            sll_hatype: 0,
            sll_pkttype: 0,
            sll_halen: 6,
            sll_addr: [0; 8],
        };
        addr.sll_addr[..6].copy_from_slice(&dst_mac);
        let sent = unsafe {
            libc::sendto(
                fd,
                wire.as_ptr() as *const libc::c_void,
                n,
                0,
                &addr as *const libc::sockaddr_ll as *const libc::sockaddr,
                mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t,
            )
        };
        if sent < 0 {
            return Err(map_errno());
        }
        Ok(frame.len())
    }

    fn recv(&mut self, out: &mut [u8]) -> Result<(usize, RecvMeta), QdnfError> {
        let fd = self.fd()?;
        let mut wire = [0u8; WIRE_CAP];
        let mut addr: libc::sockaddr_ll = unsafe { mem::zeroed() };
        let mut addrlen = mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t;
        let n = unsafe {
            libc::recvfrom(
                fd,
                wire.as_mut_ptr() as *mut libc::c_void,
                WIRE_CAP,
                0,
                &mut addr as *mut libc::sockaddr_ll as *mut libc::sockaddr,
                &mut addrlen,
            )
        };
        if n < 0 {
            return Err(map_errno());
        }
        if n == 0 {
            return Err(QdnfError::Closed);
        }
        let n = n as usize;
        let (got, _hdr_src, _hdr_dst) = decapsulate_ethernet(&wire[..n], out)?;
        let observed = observed_from_ll(&addr);
        Ok((
            got,
            RecvMeta {
                observed_source: observed,
                scope: self.scope,
                mtu: self.mtu,
            },
        ))
    }

    fn shutdown(&mut self) -> Result<(), QdnfError> {
        if let Some(fd) = self.fd.take() {
            let raw = fd.as_raw_fd();
            unsafe {
                let _ = libc::shutdown(raw, libc::SHUT_RDWR);
            }
            drop(fd);
        }
        Ok(())
    }
}

impl Drop for RawEthernet {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn htons(v: u16) -> u16 {
    v.to_be()
}

fn map_errno() -> QdnfError {
    match std::io::Error::last_os_error().raw_os_error() {
        Some(libc::EPERM) | Some(libc::EACCES) => QdnfError::PlatformUnsupported,
        Some(libc::EAGAIN) => QdnfError::WouldBlock,
        Some(libc::EBADF) | Some(libc::ENOTCONN) | Some(libc::EPIPE) => QdnfError::Closed,
        Some(libc::ENODEV) | Some(libc::ENOENT) | Some(libc::ENXIO) => QdnfError::Range,
        Some(libc::EMSGSIZE) => QdnfError::Capacity,
        _ => QdnfError::PlatformUnsupported,
    }
}

fn observed_from_ll(addr: &libc::sockaddr_ll) -> ObservedLocator {
    let n = core::cmp::min(addr.sll_halen as usize, 6);
    if n == 6 {
        let mut mac = [0u8; 6];
        mac.copy_from_slice(&addr.sll_addr[..6]);
        locator_from_mac(&mac)
    } else if n > 0 {
        ObservedLocator::from_slice(&addr.sll_addr[..n]).unwrap_or(ObservedLocator::EMPTY)
    } else {
        ObservedLocator::EMPTY
    }
}

#[repr(C)]
struct IfReqHw {
    name: [libc::c_char; IFNAMSIZ],
    sa_family: libc::c_ushort,
    sa_data: [u8; 14],
}

#[repr(C)]
struct IfReqMtu {
    name: [libc::c_char; IFNAMSIZ],
    mtu: libc::c_int,
    _pad: [u8; 12],
}

fn read_hwaddr(fd: libc::c_int, cname: &[libc::c_char; IFNAMSIZ]) -> Result<[u8; 6], QdnfError> {
    let mut ifr = IfReqHw {
        name: *cname,
        sa_family: 0,
        sa_data: [0; 14],
    };
    let rc = unsafe { libc::ioctl(fd, libc::SIOCGIFHWADDR, &mut ifr as *mut IfReqHw) };
    if rc < 0 {
        return Err(map_errno());
    }
    let mut mac = [0u8; 6];
    mac.copy_from_slice(&ifr.sa_data[..6]);
    Ok(mac)
}

fn read_mtu(fd: libc::c_int, cname: &[libc::c_char; IFNAMSIZ]) -> u16 {
    let mut ifr = IfReqMtu {
        name: *cname,
        mtu: 0,
        _pad: [0; 12],
    };
    let rc = unsafe { libc::ioctl(fd, libc::SIOCGIFMTU, &mut ifr as *mut IfReqMtu) };
    if rc == 0 && ifr.mtu > 0 && ifr.mtu <= u16::MAX as libc::c_int {
        ifr.mtu as u16
    } else {
        1500
    }
}
