//! ConnectAccept / ConnectReject from kernel + local evidence.

use super::carrier::prohibited;
use super::evidence::PathEvidence;
use super::kernel::{FabricError, FabricState, Kernel};
use super::wire::{decode_accept, encode_accept, encode_reject, RejectReason};

pub fn reject_reason(err: FabricError) -> RejectReason {
    match err {
        FabricError::StaleDescriptor => RejectReason::Stale,
        FabricError::Expired => RejectReason::Expired,
        FabricError::GrantRevoked => RejectReason::Revoked,
        FabricError::Capacity => RejectReason::Capacity,
        _ => RejectReason::Policy,
    }
}

/// Accept only a locally validated, permitted path while the kernel is PathLive or SessionLive.
pub fn encode_outcome(
    kernel: &Kernel,
    evidence: PathEvidence,
    out: &mut [u8],
) -> Result<usize, FabricError> {
    let intent = kernel.intent().ok_or(FabricError::Illegal)?;
    if !kernel.grant_live {
        return encode_reject(RejectReason::Revoked, out).map_err(FabricError::from);
    }
    if prohibited(intent.protection.disclosure, evidence.class) || !evidence.validated()
    {
        return encode_reject(RejectReason::Policy, out).map_err(FabricError::from);
    }
    match kernel.state {
        FabricState::PathLive | FabricState::SessionLive => encode_accept(
            evidence.class,
            kernel.generation,
            out,
        )
        .map_err(FabricError::from),
        _ => encode_reject(RejectReason::Policy, out).map_err(FabricError::from),
    }
}

pub fn decode_outcome_accept(bytes: &[u8]) -> Result<(super::carrier::PathClass, u32), FabricError> {
    decode_accept(bytes).map_err(FabricError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::fabric::carrier::PathClass;
    use crate::net::peer::fabric::evidence::TransportWitness;
    use crate::net::peer::fabric::intent::{ConnectionIntent, ProtectionPolicy, Purpose};
    use crate::net::peer::fabric::kernel::{Kernel, KernelEvent};
    use crate::net::peer::runtime::ResourceBudget;

    #[test]
    fn remote_or_direct_under_relay_only_rejects() {
        let mut k = Kernel::new();
        k.admit(
            ConnectionIntent::new(
                [1u8; 32],
                Purpose::ordinary(),
                ProtectionPolicy::RELAY_ONLY,
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
        let local = PathEvidence::from_witness(TransportWitness::from_local(
            PathClass::Relayed,
            1,
            64,
            5,
            1,
        ));
        let mut buf = [0u8; 64];
        let n = encode_outcome(&k, local, &mut buf).unwrap();
        assert_eq!(decode_outcome_accept(&buf[..n]).unwrap().0, PathClass::Relayed);
        let remote = PathEvidence::remote_assertion(PathClass::Relayed, 1, 1);
        let n = encode_outcome(&k, remote, &mut buf).unwrap();
        assert!(decode_outcome_accept(&buf[..n]).is_err());
        let direct = PathEvidence::from_witness(TransportWitness::from_local(
            PathClass::DirectV6,
            1,
            64,
            1,
            1,
        ));
        let n = encode_outcome(&k, direct, &mut buf).unwrap();
        assert!(decode_outcome_accept(&buf[..n]).is_err());
    }
}
