//! Browser interoperability contract. Native boringtun is not a browser API.

/// ICE transport policy as used by WebRTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceTransportPolicy {
    All,
    Relay,
}

/// How a browser peer reaches QSession.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserBearer {
    /// WebRTC DataChannel + ICE + configured TURN.
    WebrtcDataChannel,
    /// WSS where WebRTC cannot work. Outer TLS terminates at the relay.
    WssFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowserProfile {
    pub ice_transport: IceTransportPolicy,
    pub bearer: BrowserBearer,
    pub gateway_holds_qsession_keys: bool,
}

impl BrowserProfile {
    pub const RELAY_ONLY: Self = Self {
        ice_transport: IceTransportPolicy::Relay,
        bearer: BrowserBearer::WebrtcDataChannel,
        gateway_holds_qsession_keys: false,
    };

    /// A TURN service cannot convert DTLS/SCTP into WireGuard.
    pub const fn turn_is_not_wireguard(self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_policy_and_no_gateway_keys() {
        let p = BrowserProfile::RELAY_ONLY;
        assert_eq!(p.ice_transport, IceTransportPolicy::Relay);
        assert!(!p.gateway_holds_qsession_keys);
        assert!(p.turn_is_not_wireguard());
    }
}
