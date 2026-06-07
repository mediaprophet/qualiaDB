//! Download platform installers and launch them (Tauri updater parity for Flutter desktop).

use std::path::PathBuf;

pub async fn download_and_install_update(download_url: String) -> Result<(), String> {
    if download_url.is_empty() {
        return Err("Empty download URL".into());
    }

    let response = reqwest::get(&download_url)
        .await
        .map_err(|e| format!("Download failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Download HTTP {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read body: {e}"))?;

    let file_name = download_url
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("qualia_update.exe");

    let path: PathBuf = std::env::temp_dir().join(file_name);
    std::fs::write(&path, &bytes).map_err(|e| format!("Write installer: {e}"))?;

    launch_installer(&path)
}

fn launch_installer(path: &PathBuf) -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Launch installer: {e}"))?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Launch installer: {e}"))?;
        Ok(())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).map_err(|e| e.to_string())?;
        std::process::Command::new(path)
            .spawn()
            .map_err(|e| format!("Launch installer: {e}"))?;
        Ok(())
    }

    #[cfg(not(any(windows, target_os = "macos", unix)))]
    {
        let _ = path;
        Err("Auto-install not supported on this platform".into())
    }
}
