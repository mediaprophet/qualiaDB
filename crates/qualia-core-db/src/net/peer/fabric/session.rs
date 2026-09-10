//! SessionReady exists only after verified path evidence and a live grant.

use super::evidence::PathEvidence;
use super::intent::PeerId;
use super::kernel::{FabricError, Kernel};

/// Successful application session. Fields are readable; construction is crate-private.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionReady {
    pub peer: PeerId,
    pub path: PathEvidence,
    pub generation: u32,
}

impl SessionReady {
    pub(crate) fn try_new(
        kernel: &Kernel,
        peer: PeerId,
        path: PathEvidence,
    ) -> Result<Self, FabricError> {
        if !kernel.grant_live {
            return Err(FabricError::GrantRevoked);
        }
        if !path.validated() || path.stale_generation(kernel.generation) {
            return Err(FabricError::Illegal);
        }
        if kernel.state != super::kernel::FabricState::SessionLive {
            return Err(FabricError::Illegal);
        }
        Ok(Self {
            peer,
            path,
            generation: kernel.generation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::fabric::carrier::PathClass;
    use crate::net::peer::fabric::evidence::{PathEvidence, TransportWitness};
    use crate::net::peer::fabric::intent::{ConnectionIntent, ProtectionPolicy, Purpose};
    use crate::net::peer::fabric::kernel::{Kernel, KernelEvent};
    use crate::net::peer::runtime::ResourceBudget;

    #[test]
    fn no_public_session_without_kernel_admit() {
        let mut k = Kernel::new();
        k.admit(
            ConnectionIntent::new(
                [9u8; 32],
                Purpose::ordinary(),
                ProtectionPolicy::ORDINARY,
                ResourceBudget {
                    bytes: 64,
                    work: 1,
                    io: 1,
                },
                0,
                1000,
            ),
            0,
        )
        .unwrap();
        let path = PathEvidence::from_witness(TransportWitness::from_local(
            PathClass::Relayed,
            1,
            64,
            5,
            1,
        ));
        assert_eq!(
            SessionReady::try_new(&k, [9u8; 32], path),
            Err(FabricError::Illegal)
        );
        k.step(
            KernelEvent::Descriptor {
                stale: false,
                expired: false,
                direct_locator: false,
            },
            1,
        );
        k.step(KernelEvent::LeaseLive, 1);
        k.step(
            KernelEvent::PathValidated {
                class: PathClass::Relayed,
            },
            1,
        );
        k.step(KernelEvent::SessionAuthenticated, 1);
        assert!(SessionReady::try_new(&k, [9u8; 32], path).is_ok());
    }
}
