//! Bind explicit profile before the child starts; no late process-env mutation.
use std::{
    path::{Path, PathBuf},
    process::Command,
};
const CHILD: &str = "QUALIA_INFERENCE_POLICY_PROFILE_CHILD";
pub(super) fn canonical(path: &Path, create: bool) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("--profile must be an absolute native profile directory".into());
    }
    if path.exists() {
        return path.canonicalize().map_err(|e| e.to_string());
    }
    if !create {
        return Err(
            "Inference profile does not exist; creation requires explicit init-vault".into(),
        );
    }
    let parent = path
        .parent()
        .ok_or("Profile parent required")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let name = path.file_name().ok_or("Profile directory name required")?;
    Ok(parent.join(name))
}
pub(super) fn bind_child(profile: &Path, create: bool) -> Result<PathBuf, String> {
    let canonical = canonical(profile, create)?;
    match std::env::var(CHILD).ok().as_deref() {
        None => {
            let status = Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
                .args(std::env::args_os().skip(1))
                .env("QUALIA_APP_META_DIR", &canonical)
                .env(CHILD, "1")
                .status()
                .map_err(|e| format!("Cannot launch bound native profile: {e}"))?;
            std::process::exit(status.code().unwrap_or(1));
        }
        Some("1") => {
            let inherited =
                std::env::var_os("QUALIA_APP_META_DIR").ok_or("Bound metadata profile missing")?;
            if self::canonical(Path::new(&inherited), create)? != canonical {
                return Err("Bound metadata profile differs from explicit --profile".into());
            }
        }
        Some(_) => {
            return Err("Invalid profile child marker; no recursive launch permitted".into())
        }
    }
    Ok(canonical)
}
