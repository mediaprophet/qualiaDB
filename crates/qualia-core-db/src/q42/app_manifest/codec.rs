//! Deterministic binary codec for [`PortableAppManifest`].

use super::error::AppManifestError;
use super::manifest::{
    AppAuthor, AppIdentity, Compatibility, EntryProjection, Integrity, PortableAppManifest,
    ProjectionKind, RequiredAsset, RequiredCapability, StateSchema, UpdateChannel,
    APP_MANIFEST_VERSION, MAX_MANIFEST_BYTES,
};
use super::permissions::{PermissionIntent, PermissionKind, PresentationHint};

/// Wire magic: `Q42APP\0\0`.
pub const APP_MANIFEST_MAGIC: [u8; 8] = *b"Q42APP\0\0";

fn write_u16(buf: &mut Vec<u8>, value: u16) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn write_str(buf: &mut Vec<u8>, value: &str) -> Result<(), AppManifestError> {
    let bytes = value.as_bytes();
    let len = u16::try_from(bytes.len()).map_err(|_| AppManifestError::Oversize)?;
    write_u16(buf, len);
    buf.extend_from_slice(bytes);
    Ok(())
}

fn write_str_list(buf: &mut Vec<u8>, values: &[String]) -> Result<(), AppManifestError> {
    let len = u16::try_from(values.len()).map_err(|_| AppManifestError::Oversize)?;
    write_u16(buf, len);
    for value in values {
        write_str(buf, value)?;
    }
    Ok(())
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, AppManifestError> {
    if *cursor + 2 > bytes.len() {
        return Err(AppManifestError::Truncated);
    }
    let value = u16::from_le_bytes(bytes[*cursor..*cursor + 2].try_into().unwrap());
    *cursor += 2;
    Ok(value)
}

fn read_bytes32(bytes: &[u8], cursor: &mut usize) -> Result<[u8; 32], AppManifestError> {
    if *cursor + 32 > bytes.len() {
        return Err(AppManifestError::Truncated);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes[*cursor..*cursor + 32]);
    *cursor += 32;
    Ok(out)
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

fn read_str_list(bytes: &[u8], cursor: &mut usize) -> Result<Vec<String>, AppManifestError> {
    let count = read_u16(bytes, cursor)? as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(read_str(bytes, cursor)?);
    }
    Ok(out)
}

impl PortableAppManifest {
    /// Deterministic little-endian encode. Fails if validation or size budget fails.
    pub fn encode(&self) -> Result<Vec<u8>, AppManifestError> {
        self.validate()?;
        let mut buf = Vec::with_capacity(512);
        buf.extend_from_slice(&APP_MANIFEST_MAGIC);
        write_u16(&mut buf, APP_MANIFEST_VERSION);
        write_u16(&mut buf, 0); // reserved
        write_str(&mut buf, &self.identity.app_id)?;
        write_str(&mut buf, &self.identity.version)?;
        write_str(&mut buf, &self.identity.author.name)?;
        write_str(&mut buf, &self.identity.author.did)?;

        let entry_len =
            u16::try_from(self.entries.len()).map_err(|_| AppManifestError::Oversize)?;
        write_u16(&mut buf, entry_len);
        for entry in &self.entries {
            buf.push(entry.projection as u8);
            buf.push(0); // pad
            write_u16(&mut buf, 0); // reserved
            write_str(&mut buf, &entry.entry_id)?;
            write_str(&mut buf, &entry.relative_path)?;
        }

        let cap_len = u16::try_from(self.required_capabilities.len())
            .map_err(|_| AppManifestError::Oversize)?;
        write_u16(&mut buf, cap_len);
        for cap in &self.required_capabilities {
            write_str(&mut buf, &cap.id)?;
            write_str(&mut buf, &cap.min_version)?;
        }

        let asset_len =
            u16::try_from(self.required_assets.len()).map_err(|_| AppManifestError::Oversize)?;
        write_u16(&mut buf, asset_len);
        for asset in &self.required_assets {
            write_str(&mut buf, &asset.asset_id)?;
            buf.extend_from_slice(&asset.expected_sha256);
        }

        write_str(&mut buf, &self.state_schema.schema_id)?;
        write_str(&mut buf, &self.state_schema.schema_version)?;

        let perm_len = u16::try_from(self.permission_intents.len())
            .map_err(|_| AppManifestError::Oversize)?;
        write_u16(&mut buf, perm_len);
        for intent in &self.permission_intents {
            buf.push(intent.kind as u8);
            buf.push(u8::from(intent.optional));
            write_u16(&mut buf, 0); // reserved
            write_str(&mut buf, &intent.scope)?;
        }

        let hint_len = u16::try_from(self.presentation_hints.len())
            .map_err(|_| AppManifestError::Oversize)?;
        write_u16(&mut buf, hint_len);
        for hint in &self.presentation_hints {
            write_str(&mut buf, &hint.key)?;
            write_str(&mut buf, &hint.value)?;
        }

        write_str(&mut buf, &self.compatibility.min_engine_version)?;
        write_str(&mut buf, &self.compatibility.max_engine_version)?;
        write_str_list(&mut buf, &self.compatibility.required_features)?;

        buf.extend_from_slice(&self.integrity.package_sha256);

        write_str(&mut buf, &self.update_channel.channel_id)?;
        write_str(&mut buf, &self.update_channel.relative_feed)?;

        if buf.len() > MAX_MANIFEST_BYTES {
            return Err(AppManifestError::Oversize);
        }
        Ok(buf)
    }

    /// Decode and re-validate. Unknown versions and permissions fail closed.
    pub fn decode(bytes: &[u8]) -> Result<Self, AppManifestError> {
        if bytes.len() < 12 {
            return Err(AppManifestError::Truncated);
        }
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(AppManifestError::Oversize);
        }
        if bytes[0..8] != APP_MANIFEST_MAGIC {
            return Err(AppManifestError::InvalidMagic);
        }
        let mut cursor = 8;
        let version = read_u16(bytes, &mut cursor)?;
        if version != APP_MANIFEST_VERSION {
            return Err(AppManifestError::UnsupportedVersion);
        }
        let _reserved = read_u16(bytes, &mut cursor)?;

        let identity = AppIdentity {
            app_id: read_str(bytes, &mut cursor)?,
            version: read_str(bytes, &mut cursor)?,
            author: AppAuthor {
                name: read_str(bytes, &mut cursor)?,
                did: read_str(bytes, &mut cursor)?,
            },
        };

        let entry_count = read_u16(bytes, &mut cursor)? as usize;
        let mut entries = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            if cursor + 4 > bytes.len() {
                return Err(AppManifestError::Truncated);
            }
            let projection = ProjectionKind::from_u8(bytes[cursor])?;
            cursor += 1;
            let _pad = bytes[cursor];
            cursor += 1;
            let _entry_reserved = read_u16(bytes, &mut cursor)?;
            entries.push(EntryProjection {
                projection,
                entry_id: read_str(bytes, &mut cursor)?,
                relative_path: read_str(bytes, &mut cursor)?,
            });
        }

        let cap_count = read_u16(bytes, &mut cursor)? as usize;
        let mut required_capabilities = Vec::with_capacity(cap_count);
        for _ in 0..cap_count {
            required_capabilities.push(RequiredCapability {
                id: read_str(bytes, &mut cursor)?,
                min_version: read_str(bytes, &mut cursor)?,
            });
        }

        let asset_count = read_u16(bytes, &mut cursor)? as usize;
        let mut required_assets = Vec::with_capacity(asset_count);
        for _ in 0..asset_count {
            required_assets.push(RequiredAsset {
                asset_id: read_str(bytes, &mut cursor)?,
                expected_sha256: read_bytes32(bytes, &mut cursor)?,
            });
        }

        let state_schema = StateSchema {
            schema_id: read_str(bytes, &mut cursor)?,
            schema_version: read_str(bytes, &mut cursor)?,
        };

        let perm_count = read_u16(bytes, &mut cursor)? as usize;
        let mut permission_intents = Vec::with_capacity(perm_count);
        for _ in 0..perm_count {
            if cursor + 4 > bytes.len() {
                return Err(AppManifestError::Truncated);
            }
            let kind = PermissionKind::from_u8(bytes[cursor])?;
            let optional = bytes[cursor + 1] != 0;
            cursor += 2;
            let _perm_reserved = read_u16(bytes, &mut cursor)?;
            permission_intents.push(PermissionIntent {
                kind,
                optional,
                scope: read_str(bytes, &mut cursor)?,
            });
        }

        let hint_count = read_u16(bytes, &mut cursor)? as usize;
        let mut presentation_hints = Vec::with_capacity(hint_count);
        for _ in 0..hint_count {
            presentation_hints.push(PresentationHint {
                key: read_str(bytes, &mut cursor)?,
                value: read_str(bytes, &mut cursor)?,
            });
        }

        let compatibility = Compatibility {
            min_engine_version: read_str(bytes, &mut cursor)?,
            max_engine_version: read_str(bytes, &mut cursor)?,
            required_features: read_str_list(bytes, &mut cursor)?,
        };

        let integrity = Integrity {
            package_sha256: read_bytes32(bytes, &mut cursor)?,
        };

        let update_channel = UpdateChannel {
            channel_id: read_str(bytes, &mut cursor)?,
            relative_feed: read_str(bytes, &mut cursor)?,
        };

        if cursor != bytes.len() {
            return Err(AppManifestError::Truncated);
        }

        let manifest = Self {
            identity,
            entries,
            required_capabilities,
            required_assets,
            state_schema,
            permission_intents,
            presentation_hints,
            compatibility,
            integrity,
            update_channel,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    /// Deterministic SHA-256 of the encoded canonical bytes.
    pub fn manifest_digest(&self) -> Result<[u8; 32], AppManifestError> {
        let encoded = self.encode()?;
        Ok(super::manifest::sha256_of(&encoded))
    }
}
