//! Bearer phase machine: init, MTU, drain, shutdown.
//!
//! `Failed` is terminal. A retry after `fail_init` requires a new
//! `BearerLifecycle`. This struct does not own a `LeaseTable`.

use crate::net::qdnf::errors::QdnfError;

use super::contract::Bearer;
use super::raw_ethernet::RawEthernet;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BearerPhase {
    Uninitialized = 0,
    Initializing = 1,
    Ready = 2,
    Draining = 3,
    Closed = 4,
    Failed = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BearerLifecycle {
    phase: BearerPhase,
    mtu: u16,
}

impl BearerLifecycle {
    pub const fn new(mtu: u16) -> Self {
        Self {
            phase: BearerPhase::Uninitialized,
            mtu,
        }
    }

    /// Uninitialized → Initializing. Ready/Draining are Conflict.
    /// Failed is terminal (Conflict). Closed stays Closed.
    pub fn begin_init(&mut self) -> Result<(), QdnfError> {
        match self.phase {
            BearerPhase::Uninitialized => {
                self.phase = BearerPhase::Initializing;
                Ok(())
            }
            BearerPhase::Closed => Err(QdnfError::Closed),
            BearerPhase::Failed
            | BearerPhase::Initializing
            | BearerPhase::Ready
            | BearerPhase::Draining => Err(QdnfError::Conflict),
        }
    }

    /// Initializing → Ready. Failed stays Failed.
    pub fn finish_init(&mut self) -> Result<(), QdnfError> {
        match self.phase {
            BearerPhase::Initializing => {
                self.phase = BearerPhase::Ready;
                Ok(())
            }
            BearerPhase::Failed => Err(QdnfError::Conflict),
            BearerPhase::Closed => Err(QdnfError::Closed),
            BearerPhase::Uninitialized | BearerPhase::Ready | BearerPhase::Draining => {
                Err(QdnfError::Conflict)
            }
        }
    }

    /// Initializing → Failed. Ready must use `shutdown`, not `fail_init`.
    pub fn fail_init(&mut self) -> Result<(), QdnfError> {
        match self.phase {
            BearerPhase::Initializing => {
                self.phase = BearerPhase::Failed;
                Ok(())
            }
            BearerPhase::Closed => Err(QdnfError::Closed),
            BearerPhase::Uninitialized
            | BearerPhase::Ready
            | BearerPhase::Draining
            | BearerPhase::Failed => Err(QdnfError::Conflict),
        }
    }

    /// Ready only. `mtu == 0` is Malformed. Decrease (degradation) is allowed.
    pub fn set_mtu(&mut self, mtu: u16) -> Result<u16, QdnfError> {
        match self.phase {
            BearerPhase::Ready => {
                if mtu == 0 {
                    return Err(QdnfError::Malformed);
                }
                self.mtu = mtu;
                Ok(mtu)
            }
            BearerPhase::Closed => Err(QdnfError::Closed),
            _ => Err(QdnfError::Conflict),
        }
    }

    /// Ready → Draining. Send wrappers must fail Closed after this.
    pub fn begin_drain(&mut self) -> Result<(), QdnfError> {
        match self.phase {
            BearerPhase::Ready => {
                self.phase = BearerPhase::Draining;
                Ok(())
            }
            BearerPhase::Closed => Err(QdnfError::Closed),
            _ => Err(QdnfError::Conflict),
        }
    }

    /// Any non-Closed → Closed. Idempotent from Closed.
    pub fn shutdown(&mut self) -> Result<(), QdnfError> {
        self.phase = BearerPhase::Closed;
        Ok(())
    }

    #[inline]
    pub const fn phase(&self) -> BearerPhase {
        self.phase
    }

    #[inline]
    pub const fn mtu(&self) -> u16 {
        self.mtu
    }

    /// Probe raw Ethernet. `PlatformUnsupported` is an explicit failed init:
    /// `RawEthernet::open` never retains a handle, so nothing leaks.
    pub fn bind_raw_ethernet(&mut self, interface: &[u8]) -> Result<(), QdnfError> {
        self.begin_init()?;
        match RawEthernet::open(interface) {
            Ok(mut adapter) => {
                let _ = adapter.shutdown();
                let _ = self.fail_init();
                Err(QdnfError::Unsupported)
            }
            Err(e) => {
                let _ = self.fail_init();
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready(mtu: u16) -> BearerLifecycle {
        let mut lc = BearerLifecycle::new(mtu);
        lc.begin_init().unwrap();
        lc.finish_init().unwrap();
        lc
    }

    #[test]
    fn init_happy_path_is_ready() {
        let lc = ready(1280);
        assert_eq!(lc.phase(), BearerPhase::Ready);
        assert_eq!(lc.mtu(), 1280);
    }

    #[test]
    fn fail_init_is_terminal() {
        let mut lc = BearerLifecycle::new(1280);
        lc.begin_init().unwrap();
        lc.fail_init().unwrap();
        assert_eq!(lc.phase(), BearerPhase::Failed);
        assert_eq!(lc.begin_init(), Err(QdnfError::Conflict));
        assert_eq!(lc.finish_init(), Err(QdnfError::Conflict));
        assert_eq!(lc.phase(), BearerPhase::Failed);
    }

    #[test]
    fn fail_init_from_ready_is_conflict() {
        let mut lc = ready(1280);
        assert_eq!(lc.fail_init(), Err(QdnfError::Conflict));
        assert_eq!(lc.phase(), BearerPhase::Ready);
    }

    #[test]
    fn set_mtu_rules() {
        let mut uninit = BearerLifecycle::new(1280);
        assert_eq!(uninit.set_mtu(512), Err(QdnfError::Conflict));

        let mut lc = ready(1280);
        assert_eq!(lc.set_mtu(0), Err(QdnfError::Malformed));
        assert_eq!(lc.mtu(), 1280);
        assert_eq!(lc.set_mtu(512).unwrap(), 512);
        assert_eq!(lc.mtu(), 512);
    }

    #[test]
    fn shutdown_is_idempotent() {
        let mut lc = ready(1280);
        lc.shutdown().unwrap();
        assert_eq!(lc.phase(), BearerPhase::Closed);
        lc.shutdown().unwrap();
        assert_eq!(lc.phase(), BearerPhase::Closed);
        assert_eq!(lc.begin_init(), Err(QdnfError::Closed));
    }

    #[test]
    fn drain_from_ready() {
        let mut lc = ready(1280);
        lc.begin_drain().unwrap();
        assert_eq!(lc.phase(), BearerPhase::Draining);
        assert_eq!(lc.begin_drain(), Err(QdnfError::Conflict));
        lc.shutdown().unwrap();
        assert_eq!(lc.phase(), BearerPhase::Closed);
    }

    #[test]
    fn raw_ethernet_open_fails_init_without_resource() {
        let mut lc = BearerLifecycle::new(1500);
        assert_eq!(
            lc.bind_raw_ethernet(b"eth0"),
            Err(QdnfError::PlatformUnsupported)
        );
        assert_eq!(lc.phase(), BearerPhase::Failed);
        assert_eq!(lc.begin_init(), Err(QdnfError::Conflict));
    }
}
