//! Best-effort identity recovery from malformed manifest bytes.
//!
//! Used only so a quarantine slot can be written when decode/validate fails
//! but the leading identity fields are still readable. This path never
//! executes an app and never grants authority.

use crate::q42::app_manifest::{AppManifestError, APP_MANIFEST_MAGIC, APP_MANIFEST_VERSION};

/// Recovered identity fragment for quarantine labelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredIdentity {
    pub app_id: String,
    pub version: String,
}

/// Probe `Q42APP` header + identity strings without full validation.
///
/// Returns [`None`] when magic/version is wrong or identity strings are
/// truncated / empty — caller must then reject rather than quarantine.
pub fn probe_identity(bytes: &[u8]) -> Option<RecoveredIdentity> {
    if bytes.len() < 12 {
        return None;
    }
    if bytes[0..8] != APP_MANIFEST_MAGIC {
        return None;
    }
    let mut cursor = 8;
    let version = read_u16(bytes, &mut cursor).ok()?;
    if version != APP_MANIFEST_VERSION {
        return None;
    }
    let _reserved = read_u16(bytes, &mut cursor).ok()?;
    let app_id = read_str(bytes, &mut cursor).ok()?;
    let version_str = read_str(bytes, &mut cursor).ok()?;
    if app_id.trim().is_empty() {
        return None;
    }
    Some(RecoveredIdentity {
        app_id,
        version: version_str,
    })
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, AppManifestError> {
    if *cursor + 2 > bytes.len() {
        return Err(AppManifestError::Truncated);
    }
    let value = u16::from_le_bytes(bytes[*cursor..*cursor + 2].try_into().unwrap());
    *cursor += 2;
    Ok(value)
}

fn read_str(bytes: &[u8], cursor: &mut usize) -> Result<String, AppManifestError> {
    let len = read_u16(bytes, cursor)? as usize;
    if *cursor + len > bytes.len() {
        return Err(AppManifestError::Truncated);
    }
    let slice = &bytes[*cursor..*cursor + len];
    *cursor += len;
    std::str::from_utf8(slice)
        .map(|s| s.to_string())
        .map_err(|_| AppManifestError::InvalidUtf8)
}
