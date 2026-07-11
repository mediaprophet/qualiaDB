pub mod action;
pub mod menu;
pub mod tabs;
pub mod shell_html;

pub use action::ShellAction;
pub use menu::{build_app_menu, dispatch_shell_action};
pub use tabs::{TabManager, TabId, TabInfo};
