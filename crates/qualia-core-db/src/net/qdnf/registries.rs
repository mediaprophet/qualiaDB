//! Numeric registries. Unknown values fail closed; no inferred algorithm from length.

use super::errors::QdnfError;

/// QFrame type codes from the wire protocol.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameType {
    DiscoveryBeacon = 1,
    LinkChallenge = 2,
    LinkProof = 3,
    LinkClose = 4,
    LinkStateAdvertisement = 16,
    RealmPathAdvertisement = 17,
    RouteWithdraw = 18,
    ResolveQuery = 32,
    ResolveAnswer = 33,
    SessionChallenge = 48,
    SessionProof = 49,
    CapabilityPresent = 50,
    CapabilityDecision = 51,
    ProtocolNegotiate = 52,
    SessionDatagram = 64,
    SessionStream = 65,
    SessionAck = 66,
    SessionReset = 67,
    SyncOperation = 80,
    ContentBlock = 81,
    Error = 255,
}

impl FrameType {
    pub const fn from_u8(v: u8) -> Result<Self, QdnfError> {
        Ok(match v {
            1 => Self::DiscoveryBeacon,
            2 => Self::LinkChallenge,
            3 => Self::LinkProof,
            4 => Self::LinkClose,
            16 => Self::LinkStateAdvertisement,
            17 => Self::RealmPathAdvertisement,
            18 => Self::RouteWithdraw,
            32 => Self::ResolveQuery,
            33 => Self::ResolveAnswer,
            48 => Self::SessionChallenge,
            49 => Self::SessionProof,
            50 => Self::CapabilityPresent,
            51 => Self::CapabilityDecision,
            52 => Self::ProtocolNegotiate,
            64 => Self::SessionDatagram,
            65 => Self::SessionStream,
            66 => Self::SessionAck,
            67 => Self::SessionReset,
            80 => Self::SyncOperation,
            81 => Self::ContentBlock,
            255 => Self::Error,
            _ => return Err(QdnfError::Unsupported),
        })
    }

    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Link-local frames must not be forwarded.
    #[inline]
    pub const fn forwarded(self) -> bool {
        matches!(
            self,
            Self::LinkStateAdvertisement
                | Self::RealmPathAdvertisement
                | Self::RouteWithdraw
                | Self::ResolveQuery
                | Self::ResolveAnswer
                | Self::SessionChallenge
                | Self::SessionProof
                | Self::CapabilityPresent
                | Self::CapabilityDecision
                | Self::ProtocolNegotiate
                | Self::SessionDatagram
                | Self::SessionStream
                | Self::SessionAck
                | Self::SessionReset
                | Self::SyncOperation
                | Self::ContentBlock
        )
    }
}

/// `next_protocol` byte.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NextProtocol {
    QLink = 1,
    QRoute = 2,
    QResolve = 3,
    QPolicy = 4,
    QSession = 5,
    QSync = 6,
}

impl NextProtocol {
    pub const fn from_u8(v: u8) -> Result<Self, QdnfError> {
        Ok(match v {
            1 => Self::QLink,
            2 => Self::QRoute,
            3 => Self::QResolve,
            4 => Self::QPolicy,
            5 => Self::QSession,
            6 => Self::QSync,
            _ => return Err(QdnfError::Unsupported),
        })
    }
}

/// Algorithm identifiers. Length alone never selects an algorithm.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlgorithmId {
    Sha384 = 1,
    HkdfSha384 = 2,
    HmacSha384 = 3,
    X25519 = 4,
    MlKem768 = 5,
    MlDsa65 = 6,
    Ed25519 = 7,
    ChaCha20Poly1305 = 8,
    SlhDsaSha2_256s = 9,
}

impl AlgorithmId {
    pub const fn from_u16(v: u16) -> Result<Self, QdnfError> {
        Ok(match v {
            1 => Self::Sha384,
            2 => Self::HkdfSha384,
            3 => Self::HmacSha384,
            4 => Self::X25519,
            5 => Self::MlKem768,
            6 => Self::MlDsa65,
            7 => Self::Ed25519,
            8 => Self::ChaCha20Poly1305,
            9 => Self::SlhDsaSha2_256s,
            _ => return Err(QdnfError::UnknownProfile),
        })
    }
}

/// Bearer profile identifiers.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BearerProfile {
    LocalIpcV1 = 1,
    RawEthernetV1 = 2,
    UdpTransitionV1 = 3,
    /// SocialWebNet userspace-WireGuard overlay. Labelled transition, not Native Independent.
    WireGuardTransitionV1 = 4,
}

impl BearerProfile {
    pub const fn from_u16(v: u16) -> Result<Self, QdnfError> {
        Ok(match v {
            1 => Self::LocalIpcV1,
            2 => Self::RawEthernetV1,
            3 => Self::UdpTransitionV1,
            4 => Self::WireGuardTransitionV1,
            _ => return Err(QdnfError::UnknownProfile),
        })
    }

    /// Native Independent conformance excludes transition carriers.
    #[inline]
    pub const fn native_independent(self) -> bool {
        matches!(self, Self::LocalIpcV1 | Self::RawEthernetV1)
    }
}

/// QFrame flags (host-order interpretation of the 16-bit flags field).
pub mod flags {
    pub const CRITICAL: u16 = 1 << 0;
    pub const FRAGMENT: u16 = 1 << 1;
    pub const SESSION: u16 = 1 << 2;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_frame_type_is_unsupported() {
        assert_eq!(FrameType::from_u8(99), Err(QdnfError::Unsupported));
    }

    #[test]
    fn discovery_is_not_forwarded() {
        assert!(!FrameType::DiscoveryBeacon.forwarded());
        assert!(FrameType::SessionStream.forwarded());
    }

    #[test]
    fn udp_transition_is_not_native_independent() {
        assert!(!BearerProfile::UdpTransitionV1.native_independent());
        assert!(!BearerProfile::WireGuardTransitionV1.native_independent());
        assert!(BearerProfile::LocalIpcV1.native_independent());
        assert_eq!(
            BearerProfile::from_u16(4).unwrap(),
            BearerProfile::WireGuardTransitionV1
        );
    }
}
