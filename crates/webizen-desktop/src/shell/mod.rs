pub mod action;
pub mod launch;
pub mod menu;
pub mod os_shell;
pub mod shell_html;
pub mod tabs;

pub use action::ShellAction;
pub use launch::{
    apply_shell_launch, apply_shell_launch_at, resolve_shell_mode, schedule_shell_launch,
    use_legacy_shell, ShellMode,
};
pub use os_shell::{
    BROWSER_VOLUME_HTML, DIRECTORY_VOLUME_HTML, INSTRUMENTS_VOLUME_HTML, KEEP_VOLUME_HTML,
    LIBRARY_VOLUME_HTML, MAIL_VOLUME_HTML, OS_SHELL_CSS, OS_SHELL_HTML, SETTINGS_VOLUME_HTML,
    TALK_VOLUME_HTML, WALLET_STAGE_HTML,
};
pub use tabs::{TabId, TabInfo, TabManager};

use tauri::AppHandle;

/// Build the native app menu and schedule default OS-shell navigation (Gate 1).
pub fn build_app_menu(
    app: &AppHandle,
) -> Result<tauri::menu::Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    launch::schedule_shell_launch(app);
    menu::build_app_menu(app)
}

pub use menu::dispatch_shell_action;
