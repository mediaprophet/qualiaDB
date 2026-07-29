pub mod action;
pub mod menu;
pub mod shell_html;
pub mod tabs;

pub use action::ShellAction;
pub use menu::{build_app_menu, dispatch_shell_action};
pub use tabs::{TabId, TabInfo, TabManager};
