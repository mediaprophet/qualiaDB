//! Content-addressed HMC packaging and zero-copy resolution for compiled render assets.

use super::compile_10d::CompiledAsset;
use std::collections::BTreeMap;

#[derive(Debug)]
pub enum HmcAssetPackageError {
    Bundle(crate::bundle::BundleError),
    TextureDigestMismatch([u8; 32]),
    TextureResourceMissing([u8; 32]),
    TextureMetadataConflict([u8; 32]),
    NoAssets,
}

impl std::fmt::Display for HmcAssetPackageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bundle(error) => write!(f, "HMC package: {error}"),
            Self::TextureDigestMismatch(digest) => write!(
                f,
                "HMC package: texture payload digest mismatch for {}",
                hex::encode(digest)
            ),
            Self::TextureResourceMissing(digest) => write!(
                f,
                "HMC package: texture resource is missing for {}",
                hex::encode(digest)
            ),
            Self::TextureMetadataConflict(digest) => write!(
                f,
                "HMC package: conflicting metadata for shared texture {}",
                hex::encode(digest)
            ),
            Self::NoAssets => write!(f, "HMC package: no compiled assets were supplied"),
        }
    }
}

impl std::error::Error for HmcAssetPackageError {}

impl From<crate::bundle::BundleError> for HmcAssetPackageError {
    fn from(value: crate::bundle::BundleError) -> Self {
        Self::Bundle(value)
    }
}

impl CompiledAsset {
    /// Consume this asset into an HMC pack containing its `.10d` and each referenced original
    /// texture payload. Texture keys are global SHA-256 paths, so identical images can be shared
    /// across assets by an HMC pack assembler that de-duplicates resources before insertion.
    pub fn build_hmc_bundle(self, asset_entry_key: &str) -> Result<Vec<u8>, HmcAssetPackageError> {
        build_hmc_bundle([(asset_entry_key.to_owned(), self)])
    }
}

/// Package one or more assets in a canonical order and store each equal image digest once.
/// All payloads remain original encoded bytes; this builder does not decode or transcode images.
pub fn build_hmc_bundle<I>(assets: I) -> Result<Vec<u8>, HmcAssetPackageError>
where
    I: IntoIterator<Item = (String, CompiledAsset)>,
{
    use sha2::{Digest, Sha256};

    let mut assets = assets.into_iter().collect::<Vec<_>>();
    if assets.is_empty() {
        return Err(HmcAssetPackageError::NoAssets);
    }
    assets.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    if assets
        .iter()
        .any(|(key, _)| key.is_empty() || key.as_bytes().contains(&0))
    {
        return Err(HmcAssetPackageError::Bundle(
            crate::bundle::BundleError::EmptyKey,
        ));
    }
    if let Some(pair) = assets.windows(2).find(|pair| pair[0].0 == pair[1].0) {
        return Err(HmcAssetPackageError::Bundle(
            crate::bundle::BundleError::DuplicateKey(pair[0].0.clone()),
        ));
    }

    let mut writer = crate::bundle::BundleWriter::new();
    let mut textures = BTreeMap::new();
    for (asset_key, asset) in assets {
        writer.add_file(asset_key, "10d", asset.container_10d, None)?;
        for dependency in asset.texture_dependencies {
            let actual: [u8; 32] = Sha256::digest(&dependency.bytes).into();
            if actual != dependency.digest {
                return Err(HmcAssetPackageError::TextureDigestMismatch(
                    dependency.digest,
                ));
            }
            if let Some(existing) = textures.get(&dependency.digest) {
                let existing: &super::assets::TextureDependency = existing;
                if existing.mime_type != dependency.mime_type || existing.bytes != dependency.bytes
                {
                    return Err(HmcAssetPackageError::TextureMetadataConflict(
                        dependency.digest,
                    ));
                }
            } else {
                textures.insert(dependency.digest, dependency);
            }
        }
    }
    for (_, dependency) in textures {
        writer.add_file(
            texture_hmc_key(&dependency.digest),
            dependency.mime_type,
            dependency.bytes,
            None,
        )?;
    }
    Ok(writer.build()?)
}

/// Stable HMC key for a content-addressed source image.
pub fn texture_hmc_key(digest: &[u8; 32]) -> String {
    format!("textures/sha256/{}", hex::encode(digest))
}

/// Borrowed image payload resolved from an integrity-checked HMC bundle.
pub struct HmcTextureResource<'a> {
    pub digest: [u8; 32],
    pub mime_type: &'a str,
    pub bytes: &'a [u8],
}

/// Resolve a MAT1 image digest from HMC by its stable SHA-256 key and verify both bundle index
/// integrity and equality with the MAT1 resource digest before returning a zero-copy view.
pub fn resolve_hmc_texture_resource<'a>(
    bundle: &'a crate::bundle::BundleReader<'a>,
    digest: &[u8; 32],
) -> Result<HmcTextureResource<'a>, HmcAssetPackageError> {
    let key = texture_hmc_key(digest);
    let bytes = bundle
        .get(&key)
        .ok_or(HmcAssetPackageError::TextureResourceMissing(*digest))?;
    if !bundle.verify_entry(&key) {
        return Err(HmcAssetPackageError::TextureDigestMismatch(*digest));
    }
    let entry = bundle
        .entry(&key)
        .ok_or(HmcAssetPackageError::TextureResourceMissing(*digest))?;
    if entry.sha256.as_slice() != digest.as_slice() {
        return Err(HmcAssetPackageError::TextureDigestMismatch(*digest));
    }
    Ok(HmcTextureResource {
        digest: *digest,
        mime_type: &entry.kind,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::assets::TextureDependency;
    use sha2::{Digest, Sha256};

    fn asset_with_texture(bytes: &[u8], mime_type: &str) -> CompiledAsset {
        let mut asset = crate::render::compile_10d::compile_asset(
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            Some("obj"),
            "urn:test:package-fixture",
            "obj",
        )
        .unwrap();
        let digest: [u8; 32] = Sha256::digest(bytes).into();
        asset.texture_dependencies.push(TextureDependency {
            digest,
            mime_type: mime_type.to_owned(),
            bytes: bytes.to_vec(),
        });
        asset
    }

    #[test]
    fn multi_asset_bundle_sorts_assets_and_deduplicates_shared_images() {
        let shared_payload = [7, 8, 9, 10];
        let asset_a = asset_with_texture(&shared_payload, "image/png");
        let asset_b = asset_a.clone();
        let forward = build_hmc_bundle(vec![
            ("assets/b.10d".to_owned(), asset_b.clone()),
            ("assets/a.10d".to_owned(), asset_a.clone()),
        ])
        .unwrap();
        let reverse = build_hmc_bundle(vec![
            ("assets/a.10d".to_owned(), asset_a),
            ("assets/b.10d".to_owned(), asset_b),
        ])
        .unwrap();
        assert_eq!(forward, reverse);

        let bundle = crate::bundle::BundleReader::parse(&forward).unwrap();
        assert_eq!(bundle.entries().len(), 3);
        assert_eq!(bundle.entries()[0].key, "assets/a.10d");
        assert_eq!(bundle.entries()[1].key, "assets/b.10d");
        let digest: [u8; 32] = Sha256::digest(shared_payload).into();
        let resource = resolve_hmc_texture_resource(&bundle, &digest).unwrap();
        assert_eq!(resource.bytes, shared_payload);
        assert_eq!(resource.mime_type, "image/png");
    }

    #[test]
    fn shared_digest_with_conflicting_mime_is_rejected() {
        let payload = [7, 8, 9, 10];
        let png = asset_with_texture(&payload, "image/png");
        let webp = asset_with_texture(&payload, "image/webp");
        assert!(matches!(
            build_hmc_bundle(vec![
                ("assets/a.10d".to_owned(), png),
                ("assets/b.10d".to_owned(), webp),
            ]),
            Err(HmcAssetPackageError::TextureMetadataConflict(_))
        ));
    }
}
