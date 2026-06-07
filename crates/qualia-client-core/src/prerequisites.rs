//! Windows desktop runtime prerequisites (WebView2 + VC++ redistributable).
//!
//! WebView2: prefer a **Fixed Version** runtime shipped next to the executable
//! (`WebView2Runtime/` or `WebView2/`). If absent, fall back to the system
//! Evergreen runtime (registry). Sets `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER` when
//! a bundled runtime is found.
//!
//! VC++ 2015–2022 x64: not redistributable inside QualiaDB — user installs
//! Microsoft's installer; we detect via registry and re-check after launch.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrerequisiteStatus {
    /// True on Windows when the prerequisite gate should run.
    pub platform_requires_check: bool,
    pub webview2_ready: bool,
    pub webview2_bundled: bool,
    pub webview2_evergreen: bool,
    pub vc_redist_ready: bool,
    pub all_ready: bool,
    /// Folder containing `msedgewebview2.exe` when bundled; empty if none.
    pub bundled_webview2_dir: String,
}

const VC_REDIST_URL: &str = "https://aka.ms/vs/17/release/vc_redist.x64.exe";
const WEBVIEW2_BOOTSTRAPPER_URL: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";

#[cfg(windows)]
mod win {
    use super::{PrerequisiteStatus, VC_REDIST_URL, WEBVIEW2_BOOTSTRAPPER_URL};
    use std::path::{Path, PathBuf};
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    const WEBVIEW2_CLIENT_GUID: &str = r"{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

    fn exe_dir() -> Option<PathBuf> {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    }

    fn folder_with_msedge(base: &Path) -> Option<PathBuf> {
        let direct = base.join("msedgewebview2.exe");
        if direct.is_file() {
            return Some(base.to_path_buf());
        }
        let nested = base.join("x64").join("msedgewebview2.exe");
        if nested.is_file() {
            return nested.parent().map(|p| p.to_path_buf());
        }
        None
    }

    fn find_bundled_webview2() -> Option<PathBuf> {
        let root = exe_dir()?;
        for name in [
            "WebView2Runtime",
            "WebView2",
            "Microsoft.WebView2.FixedVersionRuntime",
            "webview2",
        ] {
            let candidate = root.join(name);
            if let Some(found) = folder_with_msedge(&candidate) {
                return Some(found);
            }
        }
        None
    }

    fn evergreen_webview2_installed() -> bool {
        let hives = [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER];
        let bases = [
            r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients",
            r"SOFTWARE\Microsoft\EdgeUpdate\Clients",
        ];
        for hive in hives {
            for base in bases {
                let path = format!("{base}\\{WEBVIEW2_CLIENT_GUID}");
                if let Ok(key) = RegKey::predef(hive).open_subkey(path) {
                    if key.get_value::<String, _>("pv").is_ok() {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn vc_redist_x64_installed() -> bool {
        let paths = [
            r"SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64",
            r"SOFTWARE\WOW6432Node\Microsoft\VisualStudio\14.0\VC\Runtimes\x64",
        ];
        for path in paths {
            if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path) {
                if key.get_value::<u32, _>("Installed").ok() == Some(1) {
                    return true;
                }
            }
        }
        false
    }

    pub fn check() -> PrerequisiteStatus {
        let bundled = find_bundled_webview2();
        let webview2_bundled = bundled.is_some();
        let webview2_evergreen = evergreen_webview2_installed();
        let webview2_ready = webview2_bundled || webview2_evergreen;
        let vc_redist_ready = vc_redist_x64_installed();
        PrerequisiteStatus {
            platform_requires_check: true,
            webview2_ready,
            webview2_bundled,
            webview2_evergreen,
            vc_redist_ready,
            all_ready: webview2_ready && vc_redist_ready,
            bundled_webview2_dir: bundled
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    }

    pub fn configure_webview2_runtime() -> bool {
        let Some(dir) = find_bundled_webview2() else {
            return evergreen_webview2_installed();
        };
        std::env::set_var(
            "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER",
            dir.to_string_lossy().as_ref(),
        );
        true
    }

    fn launch_downloaded(path: &Path) -> Result<(), String> {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Launch installer: {e}"))?;
        Ok(())
    }

    async fn download_to_temp(url: &str, file_name: &str) -> Result<PathBuf, String> {
        let response = reqwest::get(url)
            .await
            .map_err(|e| format!("Download failed: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("Download HTTP {}", response.status()));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Read body: {e}"))?;
        let path = std::env::temp_dir().join(file_name);
        std::fs::write(&path, &bytes).map_err(|e| format!("Write installer: {e}"))?;
        Ok(path)
    }

    pub async fn install_prerequisite(kind: &str) -> Result<(), String> {
        let (url, file_name) = match kind {
            "vc_redist" => (VC_REDIST_URL, "vc_redist.x64.exe"),
            "webview2" => (WEBVIEW2_BOOTSTRAPPER_URL, "MicrosoftEdgeWebview2Setup.exe"),
            _ => return Err(format!("Unknown prerequisite kind: {kind}")),
        };
        let path = download_to_temp(url, file_name).await?;
        launch_downloaded(&path)
    }
}

#[cfg(not(windows))]
mod win {
    use super::PrerequisiteStatus;

    pub fn check() -> PrerequisiteStatus {
        PrerequisiteStatus {
            platform_requires_check: false,
            webview2_ready: true,
            webview2_bundled: false,
            webview2_evergreen: false,
            vc_redist_ready: true,
            all_ready: true,
            bundled_webview2_dir: String::new(),
        }
    }

    pub fn configure_webview2_runtime() -> bool {
        true
    }

    pub async fn install_prerequisite(_kind: &str) -> Result<(), String> {
        Ok(())
    }
}

pub fn check_prerequisites() -> PrerequisiteStatus {
    win::check()
}

pub fn configure_webview2_runtime() -> bool {
    win::configure_webview2_runtime()
}

pub async fn install_prerequisite(kind: String) -> Result<(), String> {
    win::install_prerequisite(kind.as_str()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_windows_all_ready() {
        #[cfg(not(windows))]
        {
            let s = check_prerequisites();
            assert!(s.all_ready);
            assert!(!s.platform_requires_check);
        }
    }
}
