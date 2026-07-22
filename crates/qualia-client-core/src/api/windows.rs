//! Windows runtime prerequisites

#![allow(non_snake_case)]




pub struct PrerequisiteStatus {
    pub platform_requires_check: bool,
    pub webview2_ready: bool,
    pub webview2_bundled: bool,
    pub webview2_evergreen: bool,
    pub vc_redist_ready: bool,
    pub all_ready: bool,
    pub bundled_webview2_dir: String,
}

pub fn check_prerequisites() -> PrerequisiteStatus {
    let s = crate::prerequisites::check_prerequisites();
    PrerequisiteStatus {
        platform_requires_check: s.platform_requires_check,
        webview2_ready: s.webview2_ready,
        webview2_bundled: s.webview2_bundled,
        webview2_evergreen: s.webview2_evergreen,
        vc_redist_ready: s.vc_redist_ready,
        all_ready: s.all_ready,
        bundled_webview2_dir: s.bundled_webview2_dir,
    }
}

pub fn configure_webview2_runtime() -> bool {
    crate::prerequisites::configure_webview2_runtime()
}

pub async fn install_prerequisite(kind: String) -> Result<(), String> {
    crate::prerequisites::install_prerequisite(kind).await
}

