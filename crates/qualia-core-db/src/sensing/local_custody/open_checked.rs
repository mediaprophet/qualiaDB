//! Final-component checks only; root ancestors require host protection.
use super::CustodyError;
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

#[cfg(unix)]
pub(super) fn open_regular(path: &Path) -> Result<File, CustodyError> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(CustodyError::InvalidEncoding);
    }
    Ok(file)
}

#[cfg(windows)]
pub(super) fn open_regular(path: &Path) -> Result<File, CustodyError> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    // FILE_FLAG_OPEN_REPARSE_POINT and FILE_ATTRIBUTE_REPARSE_POINT from Win32.
    const OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(OPEN_REPARSE_POINT)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(CustodyError::InvalidEncoding);
    }
    Ok(file)
}

#[cfg(not(any(unix, windows)))]
pub(super) fn open_regular(_path: &Path) -> Result<File, CustodyError> {
    Err(CustodyError::InvalidEncoding)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn rejects_final_symlink_and_nonregular_directory() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("regular");
        std::fs::write(&target, b"payload").unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(open_regular(&link).is_err());
        assert!(open_regular(dir.path()).is_err());
        assert!(open_regular(&target).is_ok());
    }
}

/// Existing-file exclusive read profile for bounded native recovery. Root and
/// ancestors still require the host's current protected-namespace lease.
#[cfg(unix)]
pub(crate) fn open_regular_exclusive(path: &Path) -> Result<File, CustodyError> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(CustodyError::InvalidEncoding);
    }
    Ok(file)
}
#[cfg(windows)]
pub(crate) fn open_regular_exclusive(path: &Path) -> Result<File, CustodyError> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    let file = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .custom_flags(0x0020_0000)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & 0x0000_0400 != 0 {
        return Err(CustodyError::InvalidEncoding);
    }
    Ok(file)
}
#[cfg(not(any(unix, windows)))]
pub(crate) fn open_regular_exclusive(_path: &Path) -> Result<File, CustodyError> {
    Err(CustodyError::InvalidEncoding)
}
