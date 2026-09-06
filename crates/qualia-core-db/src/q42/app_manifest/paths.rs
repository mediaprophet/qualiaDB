//! Package-relative path safety for portable app manifests.
//!
//! Path fields may only name files inside the managed package tree. Absolute
//! escapes, drive roots, UNC paths, URL schemes, and `..` segments fail closed.

use super::error::AppManifestError;

/// Reject absolute, traversal, and scheme-bearing path strings.
///
/// Empty strings are allowed (optional path slots). Non-empty paths must be
/// package-relative with `/` or `\` separators and no `..` components.
pub fn validate_package_relative_path(path: &str) -> Result<(), AppManifestError> {
    if path.is_empty() {
        return Ok(());
    }
    if path.as_bytes().contains(&0) {
        return Err(AppManifestError::PathTraversal);
    }
    // URL / scheme escapes (file://, https://, …) are not package paths.
    if path.contains("://") {
        return Err(AppManifestError::AbsolutePath);
    }
    let bytes = path.as_bytes();
    // Unix absolute or Windows UNC / rooted.
    if bytes[0] == b'/' || bytes[0] == b'\\' {
        return Err(AppManifestError::AbsolutePath);
    }
    // Windows drive letter (`C:` / `c:`).
    if bytes.len() >= 2 && bytes[1] == b':' {
        return Err(AppManifestError::AbsolutePath);
    }
    for component in path.split(['/', '\\']) {
        if component.is_empty() {
            // Reject `foo//bar` and trailing empties from doubled separators.
            return Err(AppManifestError::PathTraversal);
        }
        if component == ".." || component == "." {
            return Err(AppManifestError::PathTraversal);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_simple_relative() {
        assert!(validate_package_relative_path("entries/home.json").is_ok());
        assert!(validate_package_relative_path("ui\\panel.html").is_ok());
        assert!(validate_package_relative_path("").is_ok());
    }

    #[test]
    fn rejects_traversal_and_absolute() {
        assert_eq!(
            validate_package_relative_path("../etc/passwd"),
            Err(AppManifestError::PathTraversal)
        );
        assert_eq!(
            validate_package_relative_path("foo/../bar"),
            Err(AppManifestError::PathTraversal)
        );
        assert_eq!(
            validate_package_relative_path("/etc/passwd"),
            Err(AppManifestError::AbsolutePath)
        );
        assert_eq!(
            validate_package_relative_path("C:\\Windows\\System32"),
            Err(AppManifestError::AbsolutePath)
        );
        assert_eq!(
            validate_package_relative_path("\\\\server\\share"),
            Err(AppManifestError::AbsolutePath)
        );
        assert_eq!(
            validate_package_relative_path("file:///tmp/x"),
            Err(AppManifestError::AbsolutePath)
        );
    }
}
