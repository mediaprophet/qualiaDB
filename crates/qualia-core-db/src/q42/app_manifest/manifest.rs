//! Versioned portable application manifest schema (APP-02).

use super::error::AppManifestError;
use super::paths::validate_package_relative_path;
use super::permissions::{PermissionIntent, PresentationHint};

/// Manifest schema version encoded on the wire.
pub const APP_MANIFEST_VERSION: u16 = 1;
/// Soft ceiling for a single serialized manifest (cold metadata).
pub const MAX_MANIFEST_BYTES: usize = 256 * 1024;
/// Maximum entry projections retained.
pub const MAX_ENTRIES: usize = 32;
/// Maximum required capability rows.
pub const MAX_CAPABILITIES: usize = 64;
/// Maximum required asset rows.
pub const MAX_ASSETS: usize = 64;
/// Maximum permission intents.
pub const MAX_PERMISSIONS: usize = 32;
/// Maximum presentation hints.
pub const MAX_HINTS: usize = 64;
/// Maximum compatibility feature tags.
pub const MAX_FEATURES: usize = 32;

/// Host-agnostic entry projection kind.
///
/// These name *projection surfaces*, not Host/Vibe invoke IDs.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionKind {
    Manifold = 1,
    Container = 2,
    FocusedMiniApp = 3,
    DesktopHost = 4,
}

impl ProjectionKind {
    pub fn from_u8(value: u8) -> Result<Self, AppManifestError> {
        match value {
            1 => Ok(Self::Manifold),
            2 => Ok(Self::Container),
            3 => Ok(Self::FocusedMiniApp),
            4 => Ok(Self::DesktopHost),
            _ => Err(AppManifestError::InvalidProjection),
        }
    }
}

/// Author identity recorded on the manifest.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AppAuthor {
    pub name: String,
    /// Author DID or empty when unsigned draft (still requires non-empty name).
    pub did: String,
}

/// App identity / version / author block.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AppIdentity {
    pub app_id: String,
    pub version: String,
    pub author: AppAuthor,
}

/// One entry into a projection surface.
///
/// `entry_id` is an app-local symbolic name (not a Host/Vibe invoke ID).
/// `relative_path` is a package-relative resource path (validated).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EntryProjection {
    pub projection: ProjectionKind,
    pub entry_id: String,
    pub relative_path: String,
}

/// Capability the host must provide before launch.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RequiredCapability {
    pub id: String,
    pub min_version: String,
}

/// Governed asset the package depends on.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RequiredAsset {
    pub asset_id: String,
    /// Optional expected SHA-256; all-zero means “unspecified”.
    pub expected_sha256: [u8; 32],
}

/// State schema reference (SHACL / semantic shape id + version).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StateSchema {
    pub schema_id: String,
    pub schema_version: String,
}

/// Engine / feature compatibility window.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Compatibility {
    pub min_engine_version: String,
    /// Empty string means no upper bound.
    pub max_engine_version: String,
    pub required_features: Vec<String>,
}

/// Package integrity digests.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Integrity {
    pub package_sha256: [u8; 32],
}

/// Update channel descriptor (no network fetch performed here).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UpdateChannel {
    /// Channel id such as `none`, `stable`, or a DID-shaped channel name.
    pub channel_id: String,
    /// Optional package-relative feed descriptor path (validated when non-empty).
    pub relative_feed: String,
}

/// Versioned portable application manifest v1.
///
/// Field order on this struct is the canonical serialization order.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PortableAppManifest {
    pub identity: AppIdentity,
    pub entries: Vec<EntryProjection>,
    pub required_capabilities: Vec<RequiredCapability>,
    pub required_assets: Vec<RequiredAsset>,
    pub state_schema: StateSchema,
    pub permission_intents: Vec<PermissionIntent>,
    /// Layout / theme suggestions only — never authority (see permissions module).
    pub presentation_hints: Vec<PresentationHint>,
    pub compatibility: Compatibility,
    pub integrity: Integrity,
    pub update_channel: UpdateChannel,
}

impl PortableAppManifest {
    /// Fail closed on missing identity, unknown projection paths, oversize lists,
    /// empty integrity, or unsafe path fields.
    pub fn validate(&self) -> Result<(), AppManifestError> {
        if self.identity.app_id.trim().is_empty() {
            return Err(AppManifestError::MissingAppId);
        }
        if self.identity.version.trim().is_empty() {
            return Err(AppManifestError::MissingVersion);
        }
        if self.identity.author.name.trim().is_empty() {
            return Err(AppManifestError::MissingAuthor);
        }
        if self.state_schema.schema_id.trim().is_empty() {
            return Err(AppManifestError::MissingStateSchema);
        }
        if self.entries.is_empty() {
            return Err(AppManifestError::EmptyEntries);
        }
        if self.integrity.package_sha256 == [0u8; 32] {
            return Err(AppManifestError::MissingIntegrity);
        }
        if self.entries.len() > MAX_ENTRIES
            || self.required_capabilities.len() > MAX_CAPABILITIES
            || self.required_assets.len() > MAX_ASSETS
            || self.permission_intents.len() > MAX_PERMISSIONS
            || self.presentation_hints.len() > MAX_HINTS
            || self.compatibility.required_features.len() > MAX_FEATURES
        {
            return Err(AppManifestError::Oversize);
        }
        for entry in &self.entries {
            if entry.entry_id.trim().is_empty() {
                return Err(AppManifestError::EmptyEntries);
            }
            validate_package_relative_path(&entry.relative_path)?;
            if entry.relative_path.is_empty() {
                return Err(AppManifestError::PathTraversal);
            }
        }
        for asset in &self.required_assets {
            if asset.asset_id.trim().is_empty() {
                return Err(AppManifestError::MissingAppId);
            }
        }
        for cap in &self.required_capabilities {
            if cap.id.trim().is_empty() {
                return Err(AppManifestError::MissingAppId);
            }
        }
        validate_package_relative_path(&self.update_channel.relative_feed)?;
        Ok(())
    }

    /// Verify a package payload digest against the integrity block.
    pub fn verify_package_digest(&self, digest: &[u8; 32]) -> Result<(), AppManifestError> {
        if digest == &self.integrity.package_sha256 {
            Ok(())
        } else {
            Err(AppManifestError::InvalidDigest)
        }
    }
}

/// Compute SHA-256 into a caller-owned buffer.
pub fn sha256_into(bytes: &[u8], out: &mut [u8; 32]) {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    out.copy_from_slice(&digest);
}

/// Convenience SHA-256 for cold tests and tooling.
pub fn sha256_of(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    sha256_into(bytes, &mut out);
    out
}
