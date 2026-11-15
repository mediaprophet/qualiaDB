//! Signed high-water anchor in separately configured trusted host storage.
//! It detects rollback of the policy file alone. It is not a TPM monotonic
//! counter, and cannot detect rollback of the entire trusted host's storage.
use super::{
    signature::{signature_parts, verify_signature},
    PolicyError, TrustedPolicyIdentity, MAX_POLICY_JSON_BYTES,
};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RecordIdentity {
    pub serial: u64,
    pub generation: u64,
    pub digest: [u8; 32],
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnchorRecord {
    version: u32,
    store: [u8; 32],
    serial: u64,
    generation: u64,
    digest: [u8; 32],
    signature: [[u8; 32]; 2],
}
pub(super) struct AuthorityAnchor {
    path: PathBuf,
    store: [u8; 32],
}
impl AuthorityAnchor {
    pub fn new(root: &Path, authority_root: &Path) -> Result<Self, PolicyError> {
        if authority_root == root || authority_root.starts_with(root) {
            return Err(PolicyError::WrongAuthority);
        }
        let path_text = root.to_str().ok_or(PolicyError::Encoding)?;
        if path_text.len() > 4096 {
            return Err(PolicyError::Capacity);
        }
        // Windows paths are case-insensitive; canonicalization resolves links
        // before this normalization. OS locking additionally handles aliases.
        #[cfg(target_os = "windows")]
        let path_text = path_text.to_lowercase();
        let mut h = Sha256::new();
        h.update(b"qualia:sentinel:store-path:v1\0");
        h.update(path_text.as_bytes());
        let store: [u8; 32] = h.finalize().into();
        let mut name = String::with_capacity(69);
        use std::fmt::Write as _;
        for b in store {
            write!(&mut name, "{b:02x}").expect("writing String");
        }
        name.push_str(".json");
        Ok(Self {
            path: authority_root.join(name),
            store,
        })
    }
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
    fn digest(&self, value: &AnchorRecord, trust: &TrustedPolicyIdentity) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"qualia:sentinel:authority-anchor:v1\0");
        h.update(value.version.to_le_bytes());
        h.update(value.store);
        h.update(trust.key);
        for n in [
            trust.principal,
            trust.audience,
            trust.controller_generation,
            value.serial,
            value.generation,
        ] {
            h.update(n.to_le_bytes());
        }
        h.update(value.digest);
        h.finalize().into()
    }
    pub fn verify(
        &self,
        identity: RecordIdentity,
        trust: &TrustedPolicyIdentity,
    ) -> Result<(), PolicyError> {
        let bytes = read_bounded(&self.path, 4096)?;
        let anchor: AnchorRecord =
            serde_json::from_slice(&bytes).map_err(|_| PolicyError::Encoding)?;
        if anchor.version != 1
            || anchor.store != self.store
            || anchor.serial != identity.serial
            || anchor.generation != identity.generation
            || anchor.digest != identity.digest
        {
            return Err(PolicyError::StaleGeneration);
        }
        verify_signature(trust.key, self.digest(&anchor, trust), anchor.signature)
    }
    pub fn publish(
        &self,
        identity: RecordIdentity,
        trust: &TrustedPolicyIdentity,
        key: &SigningKey,
    ) -> Result<(), PolicyError> {
        let mut anchor = AnchorRecord {
            version: 1,
            store: self.store,
            serial: identity.serial,
            generation: identity.generation,
            digest: identity.digest,
            signature: [[0; 32]; 2],
        };
        anchor.signature = signature_parts(key.sign(&self.digest(&anchor, trust)).to_bytes());
        atomic_json_write(&self.path, &anchor, 4096)
    }
}
pub(super) fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, PolicyError> {
    let file = fs::File::open(path).map_err(|_| PolicyError::Io)?;
    if file.metadata().map_err(|_| PolicyError::Io)?.len() > limit as u64 {
        return Err(PolicyError::Capacity);
    }
    let mut bytes = Vec::with_capacity(limit + 1);
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PolicyError::Io)?;
    if bytes.len() > limit {
        return Err(PolicyError::Capacity);
    }
    Ok(bytes)
}
pub(super) fn atomic_json_write(
    path: &Path,
    value: &impl Serialize,
    limit: usize,
) -> Result<(), PolicyError> {
    if limit > MAX_POLICY_JSON_BYTES {
        return Err(PolicyError::Budget);
    }
    let parent = path.parent().ok_or(PolicyError::Io)?;
    let scratch = tempfile::Builder::new()
        .prefix("sentinel-policy-")
        .tempdir_in(parent)
        .map_err(|_| PolicyError::Io)?;
    let mut file = tempfile::NamedTempFile::new_in(scratch.path()).map_err(|_| PolicyError::Io)?;
    let mut writer = LimitedWriter {
        inner: file.as_file_mut(),
        remaining: limit,
    };
    serde_json::to_writer(&mut writer, value).map_err(|_| PolicyError::Encoding)?;
    writer.flush().map_err(|_| PolicyError::Io)?;
    file.as_file().sync_all().map_err(|_| PolicyError::Io)?;
    file.persist(path).map_err(|_| PolicyError::Io)?;
    Ok(())
}
pub(super) fn validate_json_size(value: &impl Serialize, limit: usize) -> Result<(), PolicyError> {
    let mut writer = LimitedWriter {
        inner: std::io::sink(),
        remaining: limit,
    };
    serde_json::to_writer(&mut writer, value).map_err(|_| PolicyError::Capacity)
}
struct LimitedWriter<W> {
    inner: W,
    remaining: usize,
}
impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(std::io::Error::other("policy file byte budget exceeded"));
        }
        let n = self.inner.write(bytes)?;
        self.remaining -= n;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
