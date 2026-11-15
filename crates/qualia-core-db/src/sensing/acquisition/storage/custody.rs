//! Durable local custody publication and verified readback for approved schemas 6/7.
//!
//! Callers must hold an exclusive lease over a trusted local root. Redirect and
//! reparse checks reject final-component links; ancestors require host custody.
use super::*;
use crate::{
    q42::{field_envelope as env, field_semantics as sem},
    sensing::acquisition as a,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAX_CUSTODY_ENTRIES: usize = 65;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug)]
pub struct PublishOptions {
    pub max_bytes: usize,
    pub envelope: env::Limits,
    pub shared: sem::Limits,
    pub limits: Limits,
}

impl Default for PublishOptions {
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024,
            envelope: env::Limits::default(),
            shared: sem::Limits::default(),
            limits: Limits::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishedArtifact {
    pub blake3: [u8; 32],
    pub byte_length: u64,
    pub family: Family,
    pub path: PathBuf,
}

#[derive(Debug)]
pub enum PublishError {
    Io(io::Error),
    Budget,
    InvalidPath,
    Conflict,
    Admission(Error),
}

impl From<io::Error> for PublishError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<Error> for PublishError {
    fn from(error: Error) -> Self {
        Self::Admission(error)
    }
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "acquisition custody IO: {e}"),
            Self::Budget => f.write_str("acquisition custody byte budget exceeded"),
            Self::InvalidPath => f.write_str("acquisition custody path redirects or is unsafe"),
            Self::Conflict => f.write_str("acquisition custody destination bytes mismatch"),
            Self::Admission(e) => write!(f, "acquisition custody admission failed: {e:?}"),
        }
    }
}

impl std::error::Error for PublishError {}

struct StagingGuard(Option<PathBuf>);
impl StagingGuard {
    fn path(&self) -> Result<&Path, PublishError> {
        self.0.as_deref().ok_or(PublishError::InvalidPath)
    }
}
impl Drop for StagingGuard {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = fs::remove_file(path);
        }
    }
}

fn validate_root(root: &Path) -> Result<(), PublishError> {
    if !root.is_absolute() || root.as_os_str().len() > 4096 {
        return Err(PublishError::InvalidPath);
    }
    if root.components().any(|part| matches!(part, Component::ParentDir)) {
        return Err(PublishError::InvalidPath);
    }
    #[cfg(windows)]
    {
        use std::path::Prefix;
        if !matches!(
            root.components().next(),
            Some(Component::Prefix(prefix))
                if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        ) {
            return Err(PublishError::InvalidPath);
        }
    }
    Ok(())
}

fn check_no_reparse(meta: &fs::Metadata) -> Result<(), PublishError> {
    if meta.file_type().is_symlink() {
        return Err(PublishError::InvalidPath);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        if meta.file_attributes() & ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(PublishError::InvalidPath);
        }
    }
    Ok(())
}

fn child_dir(parent: &Path, name: &str) -> Result<PathBuf, PublishError> {
    let path = parent.join(name);
    match fs::create_dir(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.into()),
    }
    let meta = fs::symlink_metadata(&path)?;
    check_no_reparse(&meta)?;
    if !meta.is_dir() {
        return Err(PublishError::InvalidPath);
    }
    let canonical = path.canonicalize()?;
    if canonical.parent() != Some(parent) {
        return Err(PublishError::InvalidPath);
    }
    Ok(canonical)
}

fn storage_dir(root: &Path, create: bool) -> Result<PathBuf, PublishError> {
    validate_root(root)?;
    if create {
        fs::create_dir_all(root)?;
    }
    let canonical_root = root.canonicalize()?;
    if create {
        return child_dir(&child_dir(&canonical_root, "sensing")?, "acquisition");
    }
    let sensing = canonical_root.join("sensing");
    let sm = fs::symlink_metadata(&sensing)?;
    check_no_reparse(&sm)?;
    if !sm.is_dir() {
        return Err(PublishError::InvalidPath);
    }
    let acq = sensing.canonicalize()?.join("acquisition");
    let am = fs::symlink_metadata(&acq)?;
    check_no_reparse(&am)?;
    if !am.is_dir() {
        return Err(PublishError::InvalidPath);
    }
    let canonical = acq.canonicalize()?;
    if canonical.parent() != sensing.canonicalize().ok().as_deref() {
        return Err(PublishError::InvalidPath);
    }
    Ok(canonical)
}

fn open_regular(path: &Path) -> Result<File, PublishError> {
    let pre = fs::symlink_metadata(path)?;
    check_no_reparse(&pre)?;
    if !pre.is_file() {
        return Err(PublishError::InvalidPath);
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(OPEN_REPARSE_POINT);
    }
    let file = options.open(path)?;
    let meta = file.metadata()?;
    check_no_reparse(&meta)?;
    if !meta.is_file() {
        return Err(PublishError::InvalidPath);
    }
    Ok(file)
}

fn verify_exact(path: &Path, bytes: &[u8]) -> Result<(), PublishError> {
    let mut file = open_regular(path)?;
    if file.metadata()?.len() != bytes.len() as u64 {
        return Err(PublishError::Conflict);
    }
    let mut scratch = [0u8; 8192];
    for expected in bytes.chunks(scratch.len()) {
        file.read_exact(&mut scratch[..expected.len()])?;
        if &scratch[..expected.len()] != expected {
            return Err(PublishError::Conflict);
        }
    }
    if file.read(&mut scratch[..1])? != 0 {
        return Err(PublishError::Conflict);
    }
    Ok(())
}

fn create_staging(dir: &Path, bytes: &[u8]) -> Result<StagingGuard, PublishError> {
    let pid = std::process::id();
    for _ in 0..64 {
        let seq = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let candidate = dir.join(format!(".tmp-acq-{pid}-{seq}.part"));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&candidate) {
            Ok(mut file) => {
                let guard = StagingGuard(Some(candidate));
                file.write_all(bytes)?;
                file.sync_all()?;
                return Ok(guard);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(PublishError::Io(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "exhausted staging filenames",
    )))
}

fn extension(family: Family) -> &'static str {
    match family {
        Family::QacV1Candidate => "qac",
        Family::EmfV4Candidate => "emf",
    }
}

/// Admit `bytes` under `family` (schemas 6/7) and atomically publish without
/// overwriting an existing destination. Existing matching content is verified.
pub fn publish_container(
    root: &Path,
    family: Family,
    selected: u32,
    bytes: &[u8],
    scratch: &mut sem::Scratch<'_>,
    options: PublishOptions,
) -> Result<PublishedArtifact, PublishError> {
    validate_root(root)?;
    if bytes.len() > options.max_bytes {
        return Err(PublishError::Budget);
    }
    let structural = env::StructuralView::open(bytes, family, options.envelope)
        .map_err(Error::from)?;
    if structural.entry_count() as usize > MAX_CUSTODY_ENTRIES {
        return Err(Error::Scratch.into());
    }
    let mut originals = [None; MAX_CUSTODY_ENTRIES];
    let _doc = admit(
        &structural,
        selected,
        &mut originals,
        scratch,
        options.shared,
        options.limits,
    )?;
    let digest = *blake3::hash(bytes).as_bytes();
    let dir = storage_dir(root, true)?;
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    let dest = dir.join(format!("{hex}.{}", extension(family)));
    let guard = create_staging(&dir, bytes)?;
    match fs::hard_link(guard.path()?, &dest) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => verify_exact(&dest, bytes)?,
        Err(e) => return Err(e.into()),
    }
    Ok(PublishedArtifact {
        blake3: digest,
        byte_length: bytes.len() as u64,
        family,
        path: dest,
    })
}

/// Read an acquisition artifact into caller-owned `buffer` and re-admit it.
pub fn read_container<'a, 's>(
    root: &Path,
    family: Family,
    digest: &[u8; 32],
    selected: u32,
    buffer: &'a mut [u8],
    originals: &'s mut [Option<a::Original<'a>>],
    scratch: &mut sem::Scratch<'_>,
    options: PublishOptions,
) -> Result<(usize, PublishedArtifact, Document<'a, 's>), PublishError> {
    let dir = storage_dir(root, false)?;
    let mut hex = String::with_capacity(64);
    for byte in *digest {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    let path = dir.join(format!("{hex}.{}", extension(family)));
    let mut file = open_regular(&path)?;
    let file_len = file.metadata()?.len();
    let len = usize::try_from(file_len).map_err(|_| PublishError::Budget)?;
    if len > options.max_bytes || len > buffer.len() {
        return Err(PublishError::Budget);
    }
    file.read_exact(&mut buffer[..len])?;
    let mut extra = [0u8; 1];
    if file.read(&mut extra)? != 0 {
        return Err(PublishError::Conflict);
    }
    let bytes = &buffer[..len];
    let actual_digest = *blake3::hash(bytes).as_bytes();
    if actual_digest != *digest {
        return Err(PublishError::Conflict);
    }
    let structural = env::StructuralView::open(bytes, family, options.envelope)
        .map_err(Error::from)?;
    let doc = admit(
        &structural,
        selected,
        originals,
        scratch,
        options.shared,
        options.limits,
    )?;
    let artifact = PublishedArtifact {
        blake3: actual_digest,
        byte_length: len as u64,
        family,
        path,
    };
    Ok((len, artifact, doc))
}

#[cfg(test)]
#[path = "custody_tests.rs"]
mod tests;
