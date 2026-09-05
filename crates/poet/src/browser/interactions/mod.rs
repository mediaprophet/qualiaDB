//! Canvas interaction modules and stable public API.
//!
//! Shared pointer state remains here; focused behavior lives in child modules.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event, HtmlElement, MouseEvent};

use crate::tool_chest::core::registry::ManifoldSeed;

/// JS `window.setTimeout` binding - avoids needing extra web-sys features.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "setTimeout")]
    pub fn set_timeout(callback: &js_sys::Function, delay: u32) -> i32;
}

const SVG_NS: &str = "http://www.w3.org/2000/svg";
static HIGHEST_Z: AtomicU32 = AtomicU32::new(100);

enum ActivePointerInteraction {
    DraggingContainer {
        container: Element,
        grab_dx: f32,
        grab_dy: f32,
    },
    ResizingContainer {
        container: Element,
        start_mx: f32,
        start_my: f32,
        orig_w: f32,
        orig_h: f32,
        style: String,
        zoom: f32,
    },
    PanningCanvas {
        canvas: Element,
        start_mx: f32,
        start_my: f32,
        start_pan_x: f32,
        start_pan_y: f32,
    },
    DraggingPort {
        source_container: Element,
        drag_svg: Element,
        drag_path: Element,
        start_x: f32,
        start_y: f32,
    },
}

thread_local! {
    static ACTIVE_INTERACTION: RefCell<Option<ActivePointerInteraction>> = RefCell::new(None);
    static GLOBAL_LISTENERS_INITIALIZED: RefCell<bool> = RefCell::new(false);
    static SELECTED_CONTAINER: RefCell<Option<String>> = RefCell::new(None);
    static PENDING_WIRE_SOURCE: RefCell<Option<String>> = RefCell::new(None);
}

mod canvas_motion;
mod container_commands;
mod docking;
mod geometry;
mod placement;
mod pointer;
mod wires;

pub use canvas_motion::{wire_canvas_pan_zoom, wire_container_dragging, wire_container_resize};
pub use container_commands::{
    apply_canvas_zoom, auto_arrange_containers, begin_wire_connection, cancel_wire_connection,
    delete_container_by_id, delete_wire_element, duplicate_container_by_id,
    duplicate_selected_containers, wire_container_deletion, wire_container_duplication,
    wire_container_selection, wire_delete_key,
};
pub use docking::{
    apply_toolbox_position, wire_flyout_tools, wire_selector_buttons, wire_toolbox_dock,
};
pub use geometry::{clamp, snap_clamp_position, snap_point, snap_to_grid};
pub use placement::{
    auto_arrange_manifold, find_smart_placement_slot, get_existing_container_rects,
    place_container_via_menu, show_tool_notification, show_tool_status, ContainerRect,
};
pub use pointer::init_global_pointer_listeners;
pub use wires::{create_wire, render_wires, update_all_wires, wire_port_dragging};

use geometry::{
    current_canvas_zoom, parse_position, parse_size, px, update_position, update_size,
    update_z_index,
};
use placement::place_container_on_canvas;
