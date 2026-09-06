//! Errors for portable application manifest validation and codec.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppManifestError {
    MissingAppId,
    MissingVersion,
    MissingAuthor,
    MissingStateSchema,
    EmptyEntries,
    InvalidMagic,
    UnsupportedVersion,
    Truncated,
    Oversize,
    InvalidUtf8,
    UnknownPermission,
    PermissionEscalation,
    PathTraversal,
    AbsolutePath,
    InvalidProjection,
    InvalidDigest,
    MissingIntegrity,
}

impl fmt::Display for AppManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAppId => write!(f, "app id is required"),
            Self::MissingVersion => write!(f, "app version is required"),
            Self::MissingAuthor => write!(f, "app author is required"),
            Self::MissingStateSchema => write!(f, "state schema id is required"),
            Self::EmptyEntries => write!(f, "at least one entry projection is required"),
            Self::InvalidMagic => write!(f, "invalid portable app manifest magic"),
            Self::UnsupportedVersion => write!(f, "unsupported portable app manifest version"),
            Self::Truncated => write!(f, "truncated portable app manifest"),
            Self::Oversize => write!(f, "portable app manifest exceeds size budget"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8 in portable app manifest"),
            Self::UnknownPermission => write!(f, "unknown permission intent fails closed"),
            Self::PermissionEscalation => {
                write!(f, "permission intent exceeds grant ceiling")
            }
            Self::PathTraversal => {
                write!(f, "package-relative path contains traversal or escape")
            }
            Self::AbsolutePath => write!(f, "absolute or drive-rooted paths are forbidden"),
            Self::InvalidProjection => write!(f, "unknown entry projection kind"),
            Self::InvalidDigest => write!(f, "integrity digest is all-zero or malformed"),
            Self::MissingIntegrity => write!(f, "package integrity digest is required"),
        }
    }
}

impl std::error::Error for AppManifestError {}
