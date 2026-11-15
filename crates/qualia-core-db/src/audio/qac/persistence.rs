//! Native immutable publication of admitted final acoustic bytes.
//!
//! The caller must exclusively control a trusted storage root and its directory
//! access. Redirect checks are not a sandbox against directory-replacement races.
//! Publication is atomic and no-clobber, not an aggregate quota or authorization
//! mechanism. File syncing does not establish directory-fsync crash durability.
use super::{QacStorageError, QacStorageOptions, QacStorageView};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy)]
pub struct QacPublishOptions {
    /// Maximum bytes in this object and its temporary file; not total disk usage.
    pub max_bytes: usize,
    /// Current envelope limits; publication checks structure and every payload digest.
    pub storage: QacStorageOptions,
}
impl Default for QacPublishOptions {
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024,
            storage: QacStorageOptions::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QacPublicationAdmission {
    /// Structural/index and every chunk digest checked; no scientific admission.
    CurrentStructure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QacArtifact {
    /// BLAKE3 of every final byte, including headers, metadata and padding.
    pub blake3: [u8; 32],
    pub byte_length: u64,
    /// Canonical `spectral/audio/{64 lowercase hex digits}.qac` destination.
    pub path: PathBuf,
    pub admission: QacPublicationAdmission,
}

#[derive(Debug)]
pub enum QacPublishError {
    Io(io::Error),
    Budget,
    Admission(QacStorageError),
    Conflict,
    InvalidPath,
}
impl From<io::Error> for QacPublishError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
impl std::fmt::Display for QacPublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "QAC publication IO: {e}"),
            Self::Budget => f.write_str("QAC publication byte budget exceeded"),
            Self::Admission(e) => write!(f, "QAC publication admission: {e:?}"),
            Self::Conflict => f.write_str("QAC destination contains different bytes"),
            Self::InvalidPath => f.write_str("QAC destination redirects or is not a regular file"),
        }
    }
}
impl std::error::Error for QacPublishError {}

fn child(parent: &Path, name: &str) -> Result<PathBuf, QacPublishError> {
    let path = parent.join(name);
    match fs::create_dir(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.into()),
    }
    let meta = fs::symlink_metadata(&path)?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(QacPublishError::InvalidPath);
    }
    let canonical = path.canonicalize()?;
    if canonical.parent() != Some(parent) {
        return Err(QacPublishError::InvalidPath);
    }
    Ok(canonical)
}

fn directory(root: &Path) -> Result<PathBuf, QacPublishError> {
    fs::create_dir_all(root)?;
    let root = root.canonicalize()?;
    child(&child(&root, "spectral")?, "audio")
}

fn verify_exact(path: &Path, bytes: &[u8]) -> Result<(), QacPublishError> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(QacPublishError::InvalidPath);
    }
    let mut file = File::open(path)?;
    if file.metadata()?.len() != bytes.len() as u64 {
        return Err(QacPublishError::Conflict);
    }
    let mut scratch = [0u8; 8192];
    for expected in bytes.chunks(scratch.len()) {
        file.read_exact(&mut scratch[..expected.len()])?;
        if &scratch[..expected.len()] != expected {
            return Err(QacPublishError::Conflict);
        }
    }
    if file.read(&mut scratch[..1])? != 0 {
        return Err(QacPublishError::Conflict);
    }
    Ok(())
}

fn stage_file(path: &Path, bytes: &[u8]) -> Result<tempfile::NamedTempFile, QacPublishError> {
    let parent = path.parent().ok_or(QacPublishError::InvalidPath)?;
    let mut staging = tempfile::NamedTempFile::new_in(parent)?;
    staging.write_all(bytes)?;
    staging.as_file().sync_all()?;
    Ok(staging)
}

fn publish_exact(path: &Path, bytes: &[u8]) -> Result<(), QacPublishError> {
    let staging = stage_file(path, bytes)?;
    match staging.persist_noclobber(path) {
        Ok(_) => Ok(()),
        // The error owns the temporary file; dropping it cleans scratch on all exits.
        Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => verify_exact(path, bytes),
        Err(e) => Err(e.error.into()),
    }
}

/// Admit without transcoding, hash all final bytes, then sync and publish a
/// same-directory temporary file without replacing an existing destination.
/// Identical existing content is verified byte-for-byte and reused. No source
/// is changed, and scratch is removed on errors/unwind. Current payload chunk
/// digests are verified in addition to structural admission; semantics stay opaque.
pub fn publish_qac(
    root: &Path,
    bytes: &[u8],
    options: QacPublishOptions,
) -> Result<QacArtifact, QacPublishError> {
    if bytes.len() > options.max_bytes {
        return Err(QacPublishError::Budget);
    }
    let admitted =
        QacStorageView::open(bytes, options.storage).map_err(QacPublishError::Admission)?;
    for index in 1..=admitted.structure().entry_count() {
        admitted
            .structure()
            .verify_chunk(index)
            .map_err(|e| QacPublishError::Admission(QacStorageError::Envelope(e)))?;
    }
    let admission = QacPublicationAdmission::CurrentStructure;
    let digest = blake3::hash(bytes);
    let path = directory(root)?.join(format!("{}.qac", digest.to_hex()));
    publish_exact(&path, bytes)?;
    Ok(QacArtifact {
        blake3: *digest.as_bytes(),
        byte_length: bytes.len() as u64,
        path,
        admission,
    })
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod tests;
