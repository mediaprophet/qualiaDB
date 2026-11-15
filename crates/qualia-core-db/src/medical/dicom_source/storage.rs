use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

/// Admission limit for one original source object, before reading or publication.
/// This is not an aggregate disk quota or a bound on metadata-parser scratch.
#[derive(Clone, Copy, Debug)]
pub struct DicomSourceBudget {
    pub max_source_bytes: usize,
}
impl Default for DicomSourceBudget {
    fn default() -> Self {
        Self {
            max_source_bytes: 256 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DicomSourceArtifact {
    /// BLAKE3 of every original byte, including unknown metadata and trailing data.
    pub blake3: [u8; 32],
    pub byte_length: u64,
    pub path: PathBuf,
}

#[derive(Debug)]
pub enum DicomSourceError {
    Io(io::Error),
    BudgetExceeded,
    Conflict,
    InvalidPath,
    InvalidAssociation,
    NotPart10,
}
impl From<io::Error> for DicomSourceError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl std::fmt::Display for DicomSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "DICOM source IO: {e}"),
            Self::BudgetExceeded => write!(f, "DICOM source exceeds admission budget"),
            Self::Conflict => write!(f, "DICOM source artifact conflicts with existing bytes"),
            Self::InvalidPath => write!(f, "DICOM source path escapes or redirects storage root"),
            Self::InvalidAssociation => write!(f, "invalid DICOM source association"),
            Self::NotPart10 => write!(f, "DICOM source lacks the Part-10 DICM marker"),
        }
    }
}
impl std::error::Error for DicomSourceError {}

pub(super) fn read_source(
    path: &Path,
    budget: DicomSourceBudget,
) -> Result<Vec<u8>, DicomSourceError> {
    let mut file = File::open(path)?;
    let len =
        usize::try_from(file.metadata()?.len()).map_err(|_| DicomSourceError::BudgetExceeded)?;
    if len > budget.max_source_bytes {
        return Err(DicomSourceError::BudgetExceeded);
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(len)
        .map_err(|_| DicomSourceError::BudgetExceeded)?;
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        let total = bytes
            .len()
            .checked_add(read)
            .ok_or(DicomSourceError::BudgetExceeded)?;
        if total > budget.max_source_bytes {
            return Err(DicomSourceError::BudgetExceeded);
        }
        bytes
            .try_reserve_exact(read)
            .map_err(|_| DicomSourceError::BudgetExceeded)?;
        bytes.extend_from_slice(&chunk[..read]);
    }
    Ok(bytes)
}

/// Build only fixed-name directories below the caller-selected canonical root.
pub(super) fn directory(root: &Path, name: &str) -> Result<PathBuf, DicomSourceError> {
    fs::create_dir_all(root)?;
    let root = root.canonicalize()?;
    let imaging = checked_child(&root, "Imaging")?;
    checked_child(&imaging, name)
}
fn checked_child(parent: &Path, name: &str) -> Result<PathBuf, DicomSourceError> {
    let path = parent.join(name);
    match fs::create_dir(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.into()),
    }
    let meta = fs::symlink_metadata(&path)?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(DicomSourceError::InvalidPath);
    }
    let canonical = path.canonicalize()?;
    if canonical.parent() != Some(parent) {
        return Err(DicomSourceError::InvalidPath);
    }
    Ok(canonical)
}

pub(super) fn object_path(root: &Path, digest: &[u8; 32]) -> Result<PathBuf, DicomSourceError> {
    let name = blake3::Hash::from(*digest).to_hex();
    Ok(directory(root, "dicom_sources")?.join(format!("{name}.dcm")))
}

fn regular_file(path: &Path) -> Result<File, DicomSourceError> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(DicomSourceError::InvalidPath);
    }
    Ok(File::open(path)?)
}

/// Write/sync a same-directory temporary file, then atomically publish without replacement.
/// Existing content must be byte-identical. NamedTempFile cleans failure/unwind scratch.
pub(super) fn publish_exact(path: &Path, bytes: &[u8]) -> Result<(), DicomSourceError> {
    let parent = path.parent().ok_or(DicomSourceError::InvalidPath)?;
    let mut staging = tempfile::NamedTempFile::new_in(parent)?;
    staging.write_all(bytes)?;
    staging.as_file().sync_all()?;
    match staging.persist_noclobber(path) {
        Ok(_) => Ok(()),
        Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => {
            // e retains the temporary file; its Drop removes it after verification.
            let mut file = regular_file(path)?;
            if file.metadata()?.len() != bytes.len() as u64 {
                return Err(DicomSourceError::Conflict);
            }
            let mut chunk = [0u8; 64 * 1024];
            for expected in bytes.chunks(chunk.len()) {
                file.read_exact(&mut chunk[..expected.len()])?;
                if &chunk[..expected.len()] != expected {
                    return Err(DicomSourceError::Conflict);
                }
            }
            if file.read(&mut chunk[..1])? != 0 {
                return Err(DicomSourceError::Conflict);
            }
            Ok(())
        }
        Err(e) => Err(e.error.into()),
    }
}

pub(super) fn preserve(
    root: &Path,
    bytes: &[u8],
    budget: DicomSourceBudget,
) -> Result<DicomSourceArtifact, DicomSourceError> {
    if bytes.len() > budget.max_source_bytes {
        return Err(DicomSourceError::BudgetExceeded);
    }
    let digest = *blake3::hash(bytes).as_bytes();
    let path = object_path(root, &digest)?;
    publish_exact(&path, bytes)?;
    verify(root, digest, bytes.len() as u64, budget)
}

pub(super) fn verify(
    root: &Path,
    digest: [u8; 32],
    length: u64,
    budget: DicomSourceBudget,
) -> Result<DicomSourceArtifact, DicomSourceError> {
    if length > budget.max_source_bytes as u64 {
        return Err(DicomSourceError::BudgetExceeded);
    }
    let path = object_path(root, &digest)?;
    let mut file = regular_file(&path)?;
    if file.metadata()?.len() != length {
        return Err(DicomSourceError::Conflict);
    }
    let mut hasher = blake3::Hasher::new();
    let mut chunk = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let n = file.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        total = total
            .checked_add(n as u64)
            .ok_or(DicomSourceError::BudgetExceeded)?;
        if total > length {
            return Err(DicomSourceError::Conflict);
        }
        hasher.update(&chunk[..n]);
    }
    if total != length || hasher.finalize().as_bytes() != &digest {
        return Err(DicomSourceError::Conflict);
    }
    Ok(DicomSourceArtifact {
        blake3: digest,
        byte_length: length,
        path,
    })
}

pub(super) fn read_association(path: &Path) -> Result<Vec<u8>, DicomSourceError> {
    let file = regular_file(path)?;
    // Fixed-size JSON receipt, bounded independently of the source admission.
    if file.metadata()?.len() > 2048 {
        return Err(DicomSourceError::InvalidAssociation);
    }
    let mut bytes = Vec::new();
    file.take(2049).read_to_end(&mut bytes)?;
    if bytes.len() > 2048 {
        return Err(DicomSourceError::InvalidAssociation);
    }
    Ok(bytes)
}

/// Archive original Part-10 bytes under explicit admission, independent of the
/// current pixel/metadata parser. Only the preamble's DICM marker is recognized;
/// this does not establish dataset validity, SOP coverage, or pixel decoding.
/// Unsupported transfer syntaxes and non-pixel objects remain exact originals.
pub fn archive_part10_source(
    storage_root: &Path,
    source_path: &Path,
    budget: DicomSourceBudget,
) -> Result<DicomSourceArtifact, DicomSourceError> {
    let bytes = read_source(source_path, budget)?;
    if !crate::dicom::is_dicom_part10(&bytes) {
        return Err(DicomSourceError::NotPart10);
    }
    preserve(storage_root, &bytes, budget)
}
