//! Closed permission-intent set and grant-ceiling checks.
//!
//! Unknown permission wire values fail closed. Presentation hints are never
//! consulted when deciding authority — see [`authority_from_presentation_hints`].

use super::error::AppManifestError;

/// Allowlisted permission kinds a portable app may declare as intents.
///
/// Wire values outside this set decode as [`AppManifestError::UnknownPermission`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionKind {
    ReadLocalState = 1,
    WriteLocalState = 2,
    ReadLocalAsset = 3,
    NetworkEgress = 4,
    CameraCapture = 5,
    MicrophoneCapture = 6,
    IdentityRead = 7,
    ConsentDisclose = 8,
}

impl PermissionKind {
    /// Parse a known wire byte. Unknown values fail closed.
    pub fn from_u8(value: u8) -> Result<Self, AppManifestError> {
        match value {
            1 => Ok(Self::ReadLocalState),
            2 => Ok(Self::WriteLocalState),
            3 => Ok(Self::ReadLocalAsset),
            4 => Ok(Self::NetworkEgress),
            5 => Ok(Self::CameraCapture),
            6 => Ok(Self::MicrophoneCapture),
            7 => Ok(Self::IdentityRead),
            8 => Ok(Self::ConsentDisclose),
            _ => Err(AppManifestError::UnknownPermission),
        }
    }

    /// Stable tag for logs / cold JSON. Unknown tags fail closed to [`None`].
    pub fn parse(tag: &str) -> Option<Self> {
        match tag.trim().to_ascii_lowercase().as_str() {
            "read_local_state" => Some(Self::ReadLocalState),
            "write_local_state" => Some(Self::WriteLocalState),
            "read_local_asset" => Some(Self::ReadLocalAsset),
            "network_egress" => Some(Self::NetworkEgress),
            "camera_capture" => Some(Self::CameraCapture),
            "microphone_capture" => Some(Self::MicrophoneCapture),
            "identity_read" => Some(Self::IdentityRead),
            "consent_disclose" => Some(Self::ConsentDisclose),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadLocalState => "read_local_state",
            Self::WriteLocalState => "write_local_state",
            Self::ReadLocalAsset => "read_local_asset",
            Self::NetworkEgress => "network_egress",
            Self::CameraCapture => "camera_capture",
            Self::MicrophoneCapture => "microphone_capture",
            Self::IdentityRead => "identity_read",
            Self::ConsentDisclose => "consent_disclose",
        }
    }

    /// Privilege rank used for escalation checks (higher = more powerful).
    pub const fn privilege_rank(self) -> u8 {
        match self {
            Self::ReadLocalState | Self::ReadLocalAsset | Self::IdentityRead => 1,
            Self::WriteLocalState | Self::ConsentDisclose => 2,
            Self::CameraCapture | Self::MicrophoneCapture => 3,
            Self::NetworkEgress => 4,
        }
    }
}

/// Declared permission intent inside a portable application manifest.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PermissionIntent {
    pub kind: PermissionKind,
    /// Scope string (asset id, state path, or `*`). Not a Host invoke ID.
    pub scope: String,
    /// When true, host may withhold without failing install validation.
    pub optional: bool,
}

/// Decorative / layout suggestion. **Inert for authorization.**
///
/// Hints must never be read by grant evaluation, capability gating, or
/// lifecycle launch decisions. See [`authority_from_presentation_hints`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PresentationHint {
    pub key: String,
    pub value: String,
}

/// Host-side grant ceiling for an installed app (not stored in the manifest).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PermissionGrant {
    pub allowed: Vec<PermissionKind>,
}

impl PermissionGrant {
    pub fn allows(&self, kind: PermissionKind) -> bool {
        self.allowed.contains(&kind)
    }

    /// Maximum privilege rank among granted kinds (0 if empty).
    pub fn max_rank(&self) -> u8 {
        self.allowed
            .iter()
            .map(|k| k.privilege_rank())
            .max()
            .unwrap_or(0)
    }
}

/// Fail closed when any required intent is absent from the grant.
pub fn check_permission_intents(
    intents: &[PermissionIntent],
    grant: &PermissionGrant,
) -> Result<(), AppManifestError> {
    for intent in intents {
        if intent.optional {
            continue;
        }
        if !grant.allows(intent.kind) {
            return Err(AppManifestError::PermissionEscalation);
        }
    }
    Ok(())
}

/// Presentation hints are decorative only and **never** contribute authority.
///
/// Callers must not treat hint key/value pairs as grants, capabilities, or
/// permission escalations. This function always returns an empty grant.
pub fn authority_from_presentation_hints(_hints: &[PresentationHint]) -> PermissionGrant {
    PermissionGrant {
        allowed: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_permission_byte_fails_closed() {
        assert_eq!(
            PermissionKind::from_u8(0),
            Err(AppManifestError::UnknownPermission)
        );
        assert_eq!(
            PermissionKind::from_u8(255),
            Err(AppManifestError::UnknownPermission)
        );
        assert!(PermissionKind::parse("admin").is_none());
        assert!(PermissionKind::parse("shell_exec").is_none());
    }
}
