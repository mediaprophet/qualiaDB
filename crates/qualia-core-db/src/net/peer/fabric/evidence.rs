//! Path evidence. Successful validation is not a public constructor.

use super::carrier::PathClass;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObserverKind {
    /// Produced by this node's transport integration.
    LocalTransport = 1,
    /// A peer or provider assertion. Not local validation.
    RemoteAssertion = 2,
}

/// Opaque witness minted only by a carrier after a real check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportWitness {
    class: PathClass,
    generation: u32,
    max_payload: u16,
    rtt_ms: u32,
    at_ms: u64,
}

impl TransportWitness {
    /// Builds `TransportWitness` from `local`.
    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub(crate) const fn from_local(
        class: PathClass,
        generation: u32,
        max_payload: u16,
        rtt_ms: u32,
        at_ms: u64,
    ) -> Self {
        Self {
            class,
            generation,
            max_payload,
            rtt_ms,
            at_ms,
        }
    }

    /// `TransportWitness::class`.
    pub const fn class(self) -> PathClass {
        self.class
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathEvidence {
    pub class: PathClass,
    pub generation: u32,
    validated: bool,
    pub observed_at_ms: u64,
    pub max_payload: u16,
    pub rtt_ms: u32,
    pub observer: ObserverKind,
}

impl PathEvidence {
    /// `PathEvidence::remote_assertion`.
    pub const fn remote_assertion(class: PathClass, generation: u32, at_ms: u64) -> Self {
        Self {
            class,
            generation,
            validated: false,
            observed_at_ms: at_ms,
            max_payload: 0,
            rtt_ms: u32::MAX,
            observer: ObserverKind::RemoteAssertion,
        }
    }

    /// Builds `PathEvidence` from `witness`.
    pub const fn from_witness(w: TransportWitness) -> Self {
        Self {
            class: w.class,
            generation: w.generation,
            validated: true,
            observed_at_ms: w.at_ms,
            max_payload: w.max_payload,
            rtt_ms: w.rtt_ms,
            observer: ObserverKind::LocalTransport,
        }
    }

    /// `PathEvidence::validated`.
    pub const fn validated(&self) -> bool {
        self.validated && matches!(self.observer, ObserverKind::LocalTransport)
    }

    /// `PathEvidence::stale_generation`.
    pub const fn stale_generation(&self, live: u32) -> bool {
        self.generation != live
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_assertion_is_not_validated() {
        let e = PathEvidence::remote_assertion(PathClass::DirectV6, 1, 10);
        assert!(!e.validated());
        let w = TransportWitness::from_local(PathClass::Relayed, 1, 1152, 12, 10);
        assert!(PathEvidence::from_witness(w).validated());
    }
}
