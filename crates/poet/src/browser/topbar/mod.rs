//! Top bar rendering and interaction modules.
//!
//! Keep each concern in a focused child module and preserve this router as the
//! stable `browser::topbar` API surface.

use crate::tool_chest::core::registry::ManifoldSeed;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event, HtmlElement, HtmlInputElement, MouseEvent};

mod actions;
mod control_bar;
mod filters;
mod help_dialogs;
mod manifold;
mod menu;
mod pods;
mod save_dialog;

pub use actions::wire_menu_dropdowns;
pub use control_bar::{build_canvas_control_bar, wire_pods};
pub use manifold::{rebuild_pager, refresh_construct_chrome, wire_title_rename};
pub use menu::{build_top_menubar, MenuItemDef};
pub use pods::toggle_tech_sidebar;

use filters::{populate_dim_tray, populate_epistemic_tray, populate_strata_tray};
use help_dialogs::{open_about_dialog, open_honesty_dialog, open_shortcuts_dialog};
use manifold::{
    add_new_manifold, append_manifold_option, open_new_manifold_dialog, show_menu_notification,
    trigger_file_download, trigger_file_import_dialog,
};
use pods::{show_a11y_notification, toggle_pod_tray};
use save_dialog::open_save_mode_dialog;
