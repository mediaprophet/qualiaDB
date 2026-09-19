pub mod action;
pub mod launch;
pub mod menu;
pub mod os_shell;
pub mod shell_html;
pub mod tabs;

pub use action::ShellAction;
pub use launch::{resolve_shell_mode, use_legacy_shell, ShellMode};
pub use menu::{build_app_menu, dispatch_shell_action};
pub use os_shell::OS_SHELL_HTML;
pub use tabs::{TabId, TabInfo, TabManager};
