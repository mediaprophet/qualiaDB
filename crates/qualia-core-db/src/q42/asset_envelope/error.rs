//! Errors for Q42 asset envelope validation and codec.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetEnvelopeError {
    UnknownLicence,
    MissingTermsUrl,
    MissingAssetId,
    MissingUpstreamRelease,
    DigestMismatch,
    InvalidMagic,
    UnsupportedVersion,
    Truncated,
    Oversize,
    InvalidUtf8,
    InvalidSensitivity,
    InvalidRoutingLane,
    ChunkBudgetExceeded,
    CountInconsistency,
    EmptyChunkPlan,
}

impl fmt::Display for AssetEnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownLicence => write!(f, "unknown licence fails closed"),
            Self::MissingTermsUrl => write!(f, "licence terms URL is required"),
            Self::MissingAssetId => write!(f, "asset id is required"),
            Self::MissingUpstreamRelease => write!(f, "upstream release identity is required"),
            Self::DigestMismatch => write!(f, "payload digest does not match envelope"),
            Self::InvalidMagic => write!(f, "invalid Q42 asset envelope magic"),
            Self::UnsupportedVersion => write!(f, "unsupported Q42 asset envelope version"),
            Self::Truncated => write!(f, "truncated Q42 asset envelope"),
            Self::Oversize => write!(f, "Q42 asset envelope exceeds size budget"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8 in Q42 asset envelope"),
            Self::InvalidSensitivity => write!(f, "invalid sensitivity class"),
            Self::InvalidRoutingLane => write!(f, "invalid commons routing lane"),
            Self::ChunkBudgetExceeded => {
                write!(f, "chunk plan exceeds the 42 MiB Sentinel pass budget")
            }
            Self::CountInconsistency => {
                write!(f, "accepted + quarantined counts exceed source count")
            }
            Self::EmptyChunkPlan => write!(f, "chunk plan is required for a release envelope"),
        }
    }
}

impl std::error::Error for AssetEnvelopeError {}
