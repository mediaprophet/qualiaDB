//! Cross-process profile creation serialization, before any OS secret operation.
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

pub(super) fn acquire(profile: &Path, create: bool) -> Result<(PathBuf, File), String> {
    if !profile.is_absolute() {
        return Err("native vault profile path must be absolute".into());
    }
    if !profile.exists() && create {
        let parent = profile
            .parent()
            .ok_or("native vault profile parent required")?;
        let parent_meta =
            fs::symlink_metadata(parent).map_err(|e| format!("native profile parent: {e}"))?;
        if !parent_meta.is_dir() || parent_meta.file_attributes() & 0x400 != 0 {
            return Err("native vault profile parent must be a regular directory".into());
        }
        // One final directory only; the native host explicitly owns its parent.
        match fs::create_dir(profile) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.to_string()),
        }
    }
    let metadata =
        fs::symlink_metadata(profile).map_err(|e| format!("native profile directory: {e}"))?;
    if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
        return Err("native profile directory cannot be a link/reparse point".into());
    }
    let canonical = profile.canonicalize().map_err(|e| e.to_string())?;
    let lock_path = canonical.join(".native-vault.lock");
    match fs::symlink_metadata(&lock_path) {
        Ok(meta) if !meta.is_file() || meta.file_attributes() & 0x400 != 0 => {
            return Err("native profile lock is not a regular file".into())
        }
        Ok(_) => (),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(e) => return Err(e.to_string()),
    }
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(3)
        .custom_flags(0x00200000)
        .open(&lock_path)
        .map_err(|e| e.to_string())?;
    let opened = lock.metadata().map_err(|e| e.to_string())?;
    if !opened.is_file() || opened.file_attributes() & 0x400 != 0 {
        return Err("native profile lock changed during open".into());
    }
    lock.try_lock()
        .map_err(|e| format!("native vault profile is busy: {e}"))?;
    if profile.canonicalize().map_err(|e| e.to_string())? != canonical {
        return Err("native profile path changed".into());
    }
    Ok((canonical, lock))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn isolated_profile_lock_serializes_create_and_canonical_aliases_without_os_keys() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path().join("fixture-profile");
        let (canonical, held) = acquire(&profile, true).unwrap();
        assert_eq!(canonical, profile.canonicalize().unwrap());
        assert!(acquire(&profile.join("."), true).is_err());
        drop(held);
        assert!(acquire(&profile, false).is_ok());
        assert!(acquire(Path::new("relative-profile"), true).is_err());
    }
    #[test]
    fn unit_test_secure_open_refuses_os_access_and_preserves_local_fixture() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path().join("fixture-profile");
        fs::create_dir(&profile).unwrap();
        fs::write(
            profile.join("keystore.bin"),
            b"isolated non-secret legacy fixture",
        )
        .unwrap();
        assert!(
            super::super::KeyVault::open_native_secure(profile.to_str().unwrap(), false).is_err()
        );
        assert_eq!(
            fs::read(profile.join("keystore.bin")).unwrap(),
            b"isolated non-secret legacy fixture"
        );
    }
}
