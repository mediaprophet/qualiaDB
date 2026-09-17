//! Dock rendering: toolbox sidebar, right dock (aura + pulse), bottom status bar.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

mod flyout;
mod glyphs;
mod model;
mod panel;
mod right;
mod statusbar;
mod toolbox;
mod widgets;

pub use flyout::{hide_flyout, show_flyout};
pub use glyphs::{tool_glyph, toolbox_glyph};
pub use model::{
    extract_toolbox_views, family_order, store_toolbox_views, DockPosition, ToolChainView,
    ToolView, ToolboxFamily, ToolboxView,
};
pub use panel::create_collapsible_dock_panel;
pub use right::build_right_dock;
pub use statusbar::{
    build_bottom_statusbar, refresh_bottom_statusbar_from_daemon,
    refresh_bottom_statusbar_in_document,
};
pub use toolbox::build_toolbox_dock;
pub use widgets::build_toolchain_widgets;
