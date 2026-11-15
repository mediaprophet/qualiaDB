use std::{
    fs::OpenOptions,
    io::{self, Read},
    path::Path,
};

pub(super) fn validate_path(path: &Path) -> io::Result<()> {
    if path.as_os_str().len() > 4096
        || !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "absolute local path without parent traversal required",
        ));
    }
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        if !matches!(path.components().next(), Some(Component::Prefix(p))
            if matches!(p.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "UNC and device paths are refused",
            ));
        }
        for part in path.components() {
            if let Component::Normal(name) = part {
                let name = name.to_str().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "non-Unicode path refused")
                })?;
                let stem = name
                    .split('.')
                    .next()
                    .unwrap_or("")
                    .trim_end_matches([' ', '.']);
                let serial = ["COM", "LPT"].iter().any(|prefix| {
                    stem.get(..3)
                        .is_some_and(|s| s.eq_ignore_ascii_case(prefix))
                        && matches!(
                            stem.get(3..),
                            Some(
                                "1" | "2"
                                    | "3"
                                    | "4"
                                    | "5"
                                    | "6"
                                    | "7"
                                    | "8"
                                    | "9"
                                    | "¹"
                                    | "²"
                                    | "³"
                            )
                        )
                });
                if name.contains(':')
                    || serial
                    || ["CON", "PRN", "AUX", "NUL", "CLOCK$", "CONIN$", "CONOUT$"]
                        .iter()
                        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "DOS devices and alternate streams are refused",
                    ));
                }
            }
        }
    }
    #[cfg(unix)]
    if ["/dev", "/proc", "/sys"]
        .iter()
        .any(|root| path.starts_with(root))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "device and process pseudo-filesystems are refused",
        ));
    }
    Ok(())
}

// Final-component protection only. The caller must supply a protected local
// root: mapped volumes, ancestor reparse points, mounts and disguised device
// nodes remain host policy. Lexical checks cannot establish physical locality.
pub(super) fn read(path: &Path, budget: usize) -> io::Result<Vec<u8>> {
    validate_path(path)?;
    if budget == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "zero byte budget",
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x0020_0000);
    }
    #[cfg(not(any(unix, windows)))]
    return Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "platform not supported",
    ));
    let mut file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > budget as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "regular file exceeds byte budget",
        ));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "reparse point refused",
            ));
        }
    }
    let length = usize::try_from(metadata.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "file too large"))?;
    let mut bytes = vec![0; length];
    file.read_exact(&mut bytes)?;
    let mut trailing = [0; 1];
    if file.read(&mut trailing)? != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file grew while reading",
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relative_and_parent_paths_rejected_before_io() {
        assert!(validate_path(Path::new("observation.qes")).is_err());
        #[cfg(windows)]
        assert!(validate_path(Path::new(r"C:\data\..\observation.qes")).is_err());
        #[cfg(unix)]
        assert!(validate_path(Path::new("/data/../observation.qes")).is_err());
    }
    #[cfg(windows)]
    #[test]
    fn remote_and_device_prefixes_rejected_before_io() {
        for path in [
            r"\\server\share\data",
            r"\\?\UNC\server\share\data",
            r"\\.\pipe\sensor",
            r"C:\data\NUL",
            r"C:\data\con.txt",
            r"C:\data\LPT1",
            r"C:\data\file:stream",
        ] {
            assert!(validate_path(Path::new(path)).is_err());
        }
    }
    #[cfg(unix)]
    #[test]
    fn obvious_device_paths_are_rejected_before_open() {
        for path in ["/dev/sda", "/dev/zero", "/proc/self/mem", "/sys/devices"] {
            assert!(validate_path(Path::new(path)).is_err());
        }
    }
}
