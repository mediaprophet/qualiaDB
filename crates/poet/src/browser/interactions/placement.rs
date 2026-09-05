//! Container geometry inventory, arrangement, placement, and status notices.

use super::*;

/// Represents a container's 2D bounding box on the manifold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContainerRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl ContainerRect {
    pub fn overlaps(&self, other: &ContainerRect, margin: f32) -> bool {
        let left_a = self.x - margin;
        let right_a = self.x + self.w + margin;
        let top_a = self.y - margin;
        let bottom_a = self.y + self.h + margin;

        let left_b = other.x;
        let right_b = other.x + other.w;
        let top_b = other.y;
        let bottom_b = other.y + other.h;

        !(right_a <= left_b || left_a >= right_b || bottom_a <= top_b || top_a >= bottom_b)
    }
}

/// Parse all existing container rectangles from the DOM.
pub fn get_existing_container_rects(document: &Document) -> Vec<ContainerRect> {
    let mut rects = Vec::new();
    if let Ok(nodes) = document.query_selector_all(".canvas-container-node") {
        for i in 0..nodes.length() {
            if let Some(node) = nodes.get(i) {
                if let Ok(el) = node.dyn_into::<Element>() {
                    let style = el.get_attribute("style").unwrap_or_default();
                    let (x, y) = parse_position(&style);
                    let (w, h) = parse_size(&style);
                    rects.push(ContainerRect { x, y, w, h });
                }
            }
        }
    }
    rects
}

/// Find an optimal non-overlapping slot on the manifold for a new container.
/// The search is unbounded — new rows keep extending the lens instead of
/// clamping to a fixed 4×6 viewport grid.
pub fn find_smart_placement_slot(document: &Document, new_w: f32, new_h: f32) -> (f32, f32) {
    let existing_rects = get_existing_container_rects(document);

    let margin = 24.0;
    let cols = 4usize;
    let needed = existing_rects.len() + 1;
    let rows = (needed / cols + 2).max(6);
    for row in 0..rows {
        for col in 0..cols {
            let cand_x = 80.0 + (col as f32) * (new_w + 40.0);
            let cand_y = 60.0 + (row as f32) * (new_h + 40.0);
            let cand = ContainerRect {
                x: cand_x,
                y: cand_y,
                w: new_w,
                h: new_h,
            };

            let has_overlap = existing_rects.iter().any(|r| cand.overlaps(&r, margin));
            if !has_overlap {
                return (cand_x, cand_y);
            }
        }
    }

    let max_y = existing_rects
        .iter()
        .map(|r| r.y + r.h)
        .fold(0.0f32, f32::max);
    (80.0, max_y + 40.0)
}

/// Smoothly auto-arrange all containers on the current manifold into an organized non-overlapping grid.
pub fn auto_arrange_manifold(document: &Document) {
    reorganize_manifold_internal(document, false);
    super::super::history::push_current_frame("auto-arrange manifold");
    show_tool_notification(
        document,
        "auto-arrange",
        "Manifold containers auto-arranged \u{2728}",
    );
}

fn reorganize_manifold_internal(document: &Document, reserve_first_slot: bool) {
    let nodes = match document.query_selector_all(".canvas-container-node") {
        Ok(n) => n,
        Err(_) => return,
    };
    let total = nodes.length();
    if total == 0 {
        return;
    }

    let offset = if reserve_first_slot { 1 } else { 0 };
    for i in 0..total {
        if let Some(node) = nodes.get(i) {
            if let Ok(el) = node.dyn_into::<Element>() {
                // Apply smooth CSS transition class
                let _ = el.class_list().add_1("manifold-rearranging");

                let slot_idx = i + offset;
                let col = slot_idx % 3;
                let row = slot_idx / 3;
                let target_x = 80.0 + (col as f32) * 440.0;
                let target_y = 60.0 + (row as f32) * 340.0;

                let style = el.get_attribute("style").unwrap_or_default();
                let new_style = update_position(&style, target_x, target_y);
                let _ = el.set_attribute("style", &new_style);
            }
        }
    }

    super::super::canvas_extent::ensure_manifold_extent(document);

    // Schedule cleanup of transition class after 480ms
    let doc_clone = document.clone();
    let timeout = Closure::wrap(Box::new(move || {
        if let Ok(all) = doc_clone.query_selector_all(".canvas-container-node.manifold-rearranging")
        {
            for j in 0..all.length() {
                if let Some(n) = all.get(j) {
                    if let Ok(el) = n.dyn_into::<Element>() {
                        let _ = el.class_list().remove_1("manifold-rearranging");
                    }
                }
            }
        }
    }) as Box<dyn FnMut()>);
    set_timeout(timeout.as_ref().unchecked_ref(), 480);
    timeout.forget();
}

/// Place a new container on the canvas at an intelligent non-overlapping position.
/// Place a container on the canvas via a menu action (public, for topbar).
pub fn place_container_via_menu(document: &Document, container_type: &str, label: &str) {
    place_container_on_canvas(document, container_type, label);
}

pub(super) fn place_container_on_canvas(document: &Document, container_type: &str, label: &str) {
    use crate::tool_chest::core::registry::SeedContainer;

    let width = 400.0;
    let height = 300.0;
    let (x, y) = find_smart_placement_slot(document, width, height);

    let next_z = HIGHEST_Z.fetch_add(1, Ordering::SeqCst) + 1;
    let container = SeedContainer {
        id: super::super::canvas_state::next_container_id(container_type),
        container_type: container_type.into(),
        title: label.trim_start_matches("+ ").to_string(),
        x,
        y,
        width,
        height,
        z: next_z as f32,
        honesty: "missing".into(),
        ..Default::default()
    };

    if let Some(canvas) = document.get_element_by_id("manifold-canvas") {
        let el = super::super::containers::build_container(document, &container);
        let _ = el.class_list().add_1("newly-placed");

        // Append to the content layer if it exists, otherwise to the canvas
        if let Some(content) = canvas.query_selector(".canvas-content-layer").unwrap() {
            content.append_child(&el).unwrap();
        } else {
            canvas.append_child(&el).unwrap();
        }

        // Deselect all existing containers and select this newly placed one
        if let Ok(all) = document.query_selector_all(".canvas-container-node") {
            for j in 0..all.length() {
                if let Some(n) = all.get(j) {
                    if let Ok(ne) = n.dyn_into::<Element>() {
                        let _ = ne.class_list().remove_1("selected");
                    }
                }
            }
        }
        let _ = el.class_list().add_1("selected");

        // Re-wire interactions for the new container
        wire_container_selection(document);
        wire_container_dragging(document);
        wire_container_resize(document);
        wire_container_deletion(document);
        wire_port_dragging(document);

        super::super::canvas_extent::pan_to_show(document, x, y, width, height);
        super::super::history::push_current_frame("place container");
    }
}

/// Show a transient informational notification.
pub fn show_tool_notification(document: &Document, tool_id: &str, label: &str) {
    show_tool_status(document, label, tool_id, "info");
}

/// Show a transient action status without claiming unavailable work succeeded.
pub fn show_tool_status(document: &Document, title_text: &str, message: &str, status_kind: &str) {
    // Remove any existing notification
    if let Some(existing) = document.query_selector(".tool-notification").unwrap() {
        existing.remove();
    }

    let notif = document.create_element("div").unwrap();
    notif.set_class_name("tool-notification");
    let n_el: HtmlElement = notif.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "position: fixed; bottom: 40px; right: 16px; background: var(--surface-panel-elevated); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         padding: 10px 14px; font-size: 12px; color: var(--text-primary); \
         box-shadow: var(--shadow-lg); z-index: 500; max-width: 320px; \
         display: flex; flex-direction: column; gap: 4px;",
    );

    let title = document.create_element("div").unwrap();
    title
        .set_attribute("style", "font-weight: 600; font-size: 11px;")
        .unwrap();
    let glyph = match status_kind {
        "success" => "\u{2713}",
        "error" => "\u{26A0}",
        "running" => "\u{23F3}",
        "unavailable" => "\u{2298}",
        _ => "\u{1F4A1}",
    };
    title.set_text_content(Some(&format!("{} {}", glyph, title_text)));
    notif.append_child(&title).unwrap();

    let status = document.create_element("div").unwrap();
    status
        .set_attribute("style", "color: var(--text-muted); font-size: 10px;")
        .unwrap();
    status.set_text_content(Some(message));
    notif.append_child(&status).unwrap();

    if let Some(body) = document.body() {
        body.append_child(&notif).unwrap();
    }

    // Auto-remove after 3 seconds via window.setTimeout
    let notif_clone = notif.clone();
    let timeout = Closure::wrap(Box::new(move || {
        notif_clone.remove();
    }) as Box<dyn FnMut()>);
    set_timeout(timeout.as_ref().unchecked_ref(), 3000);
    timeout.forget();
}
