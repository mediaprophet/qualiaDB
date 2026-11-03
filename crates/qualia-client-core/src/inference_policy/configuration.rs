//! Explicit local-host configuration. Actor credentials do not grant installation.
//! The configured vault key authenticates this record; its embedded key is never
//! used as a trust root. Project adoption is independently signed by the operator's
//! explicitly selected TrustedProject. No default project, principal or policy.
use ed25519_dalek::{Signer, SigningKey};
use qualia_cooperative_core::authoring::DocumentRevision;
use qualia_core_db::{
    llm_agent::governed::{TrustedPolicyIdentity, INFERENCE_POLICY_AUDIENCE},
    poet_host::native_authority::TrustedProject,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::Path,
};

pub const CONFIGURATION_BUDGET_BYTES: usize = 16 * 1024 * 1024;
pub(super) const RECORD_LIMIT: usize = 32 * 1024;
pub(super) const ENTRY_LIMIT: usize = 128 * 1024;
pub(super) const ENTRY_COUNT: u64 = 256;
const FILE: &str = "inference-host.json";
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceHostConfiguration {
    pub installer_identifier: String,
    pub installer_public_key: [u8; 32],
    pub project_id: String,
    pub project_issuer: String,
    pub project_public_key: [u8; 32],
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedConfiguration {
    version: u32,
    configuration: InferenceHostConfiguration,
    document: DocumentRevision,
    signature: [[u8; 32]; 2],
}
impl InferenceHostConfiguration {
    pub(super) fn validate(&self, key: &SigningKey) -> Result<(), String> {
        if [
            &self.installer_identifier,
            &self.project_id,
            &self.project_issuer,
        ]
        .iter()
        .any(|s| s.trim().is_empty() || s.len() > 4096)
            || self.installer_public_key != key.verifying_key().to_bytes()
            || self.project_public_key == [0; 32]
        {
            return Err("Invalid explicit inference host trust configuration".into());
        }
        self.trust()?;
        Ok(())
    }
    pub(super) fn anchor(&self) -> TrustedProject {
        TrustedProject {
            project_id: self.project_id.clone(),
            issuer_identifier: self.project_issuer.clone(),
            public_key: self.project_public_key,
        }
    }
    pub(super) fn trust(&self) -> Result<TrustedPolicyIdentity, String> {
        TrustedPolicyIdentity::from_host_configuration(
            self.installer_public_key,
            qualia_core_db::q_hash(&self.installer_identifier),
            INFERENCE_POLICY_AUDIENCE,
            1,
        )
        .map_err(|e| e.to_string())
    }
    pub(super) fn digest(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"qualia:inference:host-configuration:v1\0");
        for s in [
            &self.installer_identifier,
            &self.project_id,
            &self.project_issuer,
        ] {
            h.update((s.len() as u64).to_le_bytes());
            h.update(s.as_bytes());
        }
        h.update(self.installer_public_key);
        h.update(self.project_public_key);
        h.finalize().into()
    }
}
pub(super) fn selection_digest(c: &InferenceHostConfiguration, d: &DocumentRevision) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"qualia:inference:configured-source:v1\0");
    h.update(c.digest());
    for s in [&d.project_id, &d.document_id, &d.source_id] {
        h.update((s.len() as u64).to_le_bytes());
        h.update(s.as_bytes());
    }
    h.update(d.revision.to_le_bytes());
    h.update(d.source_digest);
    match &d.author_subject {
        None => h.update([0]),
        Some(s) => {
            h.update([1]);
            h.update((s.len() as u64).to_le_bytes());
            h.update(s.as_bytes());
        }
    }
    h.finalize().into()
}
fn record(
    c: InferenceHostConfiguration,
    d: DocumentRevision,
    key: &SigningKey,
) -> Result<SignedConfiguration, String> {
    c.validate(key)?;
    if d.project_id != c.project_id
        || d.source_digest == [0; 32]
        || [&d.project_id, &d.document_id, &d.source_id]
            .iter()
            .any(|s| s.is_empty() || s.len() > 4096)
        || d.author_subject
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.len() > 4096)
    {
        return Err(
            "Explicit project/source revision exceeds bounds or differs from project".into(),
        );
    }
    let sig = key.sign(&selection_digest(&c, &d)).to_bytes();
    let mut signature = [[0; 32]; 2];
    signature[0].copy_from_slice(&sig[..32]);
    signature[1].copy_from_slice(&sig[32..]);
    Ok(SignedConfiguration {
        version: 1,
        configuration: c,
        document: d,
        signature,
    })
}
pub(super) fn preflight(
    c: &InferenceHostConfiguration,
    d: &DocumentRevision,
    key: &SigningKey,
) -> Result<(), String> {
    // Validate lengths before cloning caller-owned strings.
    c.validate(key)?;
    if [&d.project_id, &d.document_id, &d.source_id]
        .iter()
        .any(|s| s.len() > 4096)
        || d.author_subject.as_ref().is_some_and(|s| s.len() > 4096)
    {
        return Err("Source revision exceeds bounds".into());
    }
    let value = record(c.clone(), d.clone(), key)?;
    serde_json::to_writer(
        LimitedWriter {
            file: std::io::sink(),
            remaining: RECORD_LIMIT,
        },
        &value,
    )
    .map_err(|e| e.to_string())
}
pub(super) fn budget(bytes: usize) -> Result<(), String> {
    if bytes < CONFIGURATION_BUDGET_BYTES {
        Err("Inference configuration byte budget exhausted".into())
    } else {
        Ok(())
    }
}
pub(super) fn save(
    root: &Path,
    c: InferenceHostConfiguration,
    document: DocumentRevision,
    key: &SigningKey,
) -> Result<(), String> {
    let record = record(c, document, key)?;
    let scratch = tempfile::Builder::new()
        .prefix("inference-config-")
        .tempdir_in(root)
        .map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(scratch.path()).map_err(|e| e.to_string())?;
    let mut writer = LimitedWriter {
        file: file.as_file_mut(),
        remaining: RECORD_LIMIT,
    };
    serde_json::to_writer(&mut writer, &record).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    #[cfg(test)]
    match SAVE_FAULT.with(|f| f.take()) {
        Some(SaveFault::Error) => return Err("injected configuration save failure".into()),
        Some(SaveFault::Panic) => panic!("injected configuration save unwind"),
        None => {}
    }
    file.persist(root.join(FILE)).map_err(|e| e.to_string())?;
    Ok(())
}
pub(super) fn load(
    root: &Path,
    key: &SigningKey,
) -> Result<(InferenceHostConfiguration, DocumentRevision), String> {
    let path = root.join(FILE);
    let meta = std::fs::symlink_metadata(&path)
        .map_err(|_| "Explicit inference host configuration is required")?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > RECORD_LIMIT as u64 {
        return Err("Inference host configuration file exceeds bounds".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err("Inference host configuration reparse file refused".into());
        }
    }
    if !path
        .canonicalize()
        .map_err(|e| e.to_string())?
        .starts_with(root)
    {
        return Err("Inference host configuration escapes metadata directory".into());
    }
    let mut bytes = Vec::with_capacity(RECORD_LIMIT + 1);
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(RECORD_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > RECORD_LIMIT {
        return Err("Inference host configuration grew beyond bounds".into());
    }
    let record: SignedConfiguration = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if record.version != 1 {
        return Err("Unsupported inference host configuration".into());
    }
    record.configuration.validate(key)?;
    let mut signature = [0; 64];
    signature[..32].copy_from_slice(&record.signature[0]);
    signature[32..].copy_from_slice(&record.signature[1]);
    key.verifying_key()
        .verify_strict(
            &selection_digest(&record.configuration, &record.document),
            &ed25519_dalek::Signature::from_bytes(&signature),
        )
        .map_err(|_| "Inference host configuration signature rejected")?;
    preflight(&record.configuration, &record.document, key)?;
    Ok((record.configuration, record.document))
}
struct LimitedWriter<W> {
    file: W,
    remaining: usize,
}
impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        if b.len() > self.remaining {
            return Err(std::io::Error::other("configuration output budget"));
        }
        let n = self.file.write(b)?;
        self.remaining -= n;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(super) enum SaveFault {
    Error,
    Panic,
}
#[cfg(test)]
thread_local! {static SAVE_FAULT:std::cell::Cell<Option<SaveFault>>=const{std::cell::Cell::new(None)};}
#[cfg(test)]
pub(super) fn fail_next_save(fault: SaveFault) {
    SAVE_FAULT.with(|f| f.set(Some(fault)));
}
