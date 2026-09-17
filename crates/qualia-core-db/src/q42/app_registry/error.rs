//! Errors for the installed-app registry (WD-02).

use std::fmt;

use crate::q42::app_manifest::AppManifestError;

/// Registry-level failures that reject registration (no slot written).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppRegistryError {
    /// Fixed slot budget exhausted.
    RegistryFull,
    /// Bytes could not be decoded and no recoverable identity for quarantine.
    RejectedMalformed,
    /// Empty or whitespace-only app id after recovery / registration.
    MissingAppId,
    /// Underlying manifest codec / validation error surfaced when rejecting.
    Manifest(AppManifestError),
}

impl fmt::Display for AppRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistryFull => write!(f, "installed-app registry is full"),
            Self::RejectedMalformed => {
                write!(f, "malformed package rejected (identity not recoverable)")
            }
            Self::MissingAppId => write!(f, "app id is required"),
            Self::Manifest(e) => write!(f, "manifest error: {e}"),
        }
    }
}

impl std::error::Error for AppRegistryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Manifest(e) => Some(e),
            _ => None,
        }
    }
}

impl From<AppManifestError> for AppRegistryError {
    fn from(value: AppManifestError) -> Self {
        Self::Manifest(value)
    }
}
