//! Stable QDNF outcomes. Hot paths return these Copy codes; they never allocate.

use core::fmt;

/// Typed protocol / runtime outcome. Display is diagnostic only; wire codes are
/// the `as u16` discriminants below.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QdnfError {
    Truncated = 1,
    Malformed = 2,
    Unsupported = 3,
    Capacity = 4,
    WouldBlock = 5,
    Closed = 6,
    Unauthorized = 7,
    Denied = 8,
    Challenge = 9,
    NeedsHuman = 10,
    Incomplete = 11,
    Ambiguous = 12,
    Conflict = 13,
    BudgetExhausted = 14,
    Expired = 15,
    Revoked = 16,
    StaleGeneration = 17,
    Replay = 18,
    Downgrade = 19,
    EntropyFailure = 20,
    CryptoFailure = 21,
    Overlap = 22,
    HopLimit = 23,
    NoRoute = 24,
    PlatformUnsupported = 25,
    Cancelled = 26,
    DoubleRelease = 27,
    ReservationUnchanged = 28,
    UnknownProfile = 29,
    CriticalExtension = 30,
    Range = 31,
}

impl QdnfError {
    /// Bytes consumed from the input when a decode fails. Zero means the caller
    /// must not treat any prefix as a complete frame.
    #[inline]
    pub const fn consumed_on_failure(self) -> usize {
        0
    }

    /// Output buffer is invalid after this failure.
    #[inline]
    pub const fn output_valid(self) -> bool {
        false
    }
}

impl fmt::Display for QdnfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Truncated => "truncated",
            Self::Malformed => "malformed",
            Self::Unsupported => "unsupported",
            Self::Capacity => "capacity",
            Self::WouldBlock => "would-block",
            Self::Closed => "closed",
            Self::Unauthorized => "unauthorized",
            Self::Denied => "denied",
            Self::Challenge => "challenge",
            Self::NeedsHuman => "needs-human",
            Self::Incomplete => "incomplete",
            Self::Ambiguous => "ambiguous",
            Self::Conflict => "conflict",
            Self::BudgetExhausted => "budget-exhausted",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
            Self::StaleGeneration => "stale-generation",
            Self::Replay => "replay",
            Self::Downgrade => "downgrade",
            Self::EntropyFailure => "entropy-failure",
            Self::CryptoFailure => "crypto-failure",
            Self::Overlap => "overlap",
            Self::HopLimit => "hop-limit",
            Self::NoRoute => "no-route",
            Self::PlatformUnsupported => "platform-unsupported",
            Self::Cancelled => "cancelled",
            Self::DoubleRelease => "double-release",
            Self::ReservationUnchanged => "reservation-unchanged",
            Self::UnknownProfile => "unknown-profile",
            Self::CriticalExtension => "critical-extension",
            Self::Range => "range",
        })
    }
}

impl core::error::Error for QdnfError {}
