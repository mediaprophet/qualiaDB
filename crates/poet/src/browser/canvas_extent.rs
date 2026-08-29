//! Manifold canvas world extent — the surface grows in any direction.
//!
//! The viewport is a window. World coordinates are not clamped to that
//! window. Pan is a CSS transform; the grid covers the occupied world
//! plus padding so the lens can extend left/up/right/down.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const WORLD_PAD: f32 = 640.0;
const MIN_SPAN: f32 = 4000.0;
const EDGE_PAN_PX: f32 = 48.0;
const EDGE_PAN_SPEED: f32 = 18.0;

/// Axis-aligned world rectangle used to size the grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldExtent {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl WorldExtent {
    pub fn from_viewport(width: f32, height: f32) -> Self {
        let w = width.max(MIN_SPAN);
        let h = height.max(MIN_SPAN);
        Self {
            min_x: -WORLD_PAD,
            min_y: -WORLD_PAD,
            max_x: w + WORLD_PAD,
            max_y: h + WORLD_PAD,
        }
    }

    pub fn include_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.min_x = self.min_x.min(x - WORLD_PAD);
        self.min_y = self.min_y.min(y - WORLD_PAD);
        self.max_x = self.max_x.max(x + w + WORLD_PAD);
        self.max_y = self.max_y.max(y + h + WORLD_PAD);
    }

    pub fn width(self) -> f32 {
        (self.max_x - self.min_x).max(MIN_SPAN)
    }

    pub fn height(self) -> f32 {
        (self.max_y - self.min_y).max(MIN_SPAN)
    }
}

pub fn pan_of(canvas: &Element) -> (f32, f32) {
    let x = canvas
        .get_attribute("data-pan-x")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let y = canvas
        .get_attribute("data-pan-y")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    (x, y)
}

pub fn zoom_of(canvas: &Element) -> f32 {
    canvas
        .get_attribute("data-zoom")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1.0)
}

pub fn set_view(canvas: &Element, pan_x: f32, pan_y: f32, zoom: f32) {
    let zoom = zoom.clamp(0.15, 4.0);
    let _ = canvas.set_attribute("data-pan-x", &format!("{pan_x:.2}"));
    let _ = canvas.set_attribute("data-pan-y", &format!("{pan_y:.2}"));
    let _ = canvas.set_attribute("data-zoom", &format!("{zoom:.4}"));
    apply_view_transform(canvas, pan_x, pan_y, zoom);
}

pub fn apply_view_transform(canvas: &Element, pan_x: f32, pan_y: f32, zoom: f32) {
    if let Ok(Some(content)) = canvas.query_selector(".canvas-content-layer") {
        if let Ok(el) = content.dyn_into::<HtmlElement>() {
            let _ = el.style().set_property("transform-origin", "0 0");
            let _ = el.style().set_property(
                "transform",
                &format!("translate({pan_x}px, {pan_y}px) scale({zoom})"),
            );
        }
    }
    if let Ok(Some(indicator)) = canvas.query_selector(".canvas-zoom-indicator") {
        indicator.set_text_content(Some(&format!("{:.0}%", zoom * 100.0)));
    }
}

/// Convert a viewport client point into manifold world coordinates.
pub fn client_to_world(canvas: &Element, client_x: f32, client_y: f32) -> (f32, f32) {
    let rect = canvas.get_bounding_client_rect();
    let (pan_x, pan_y) = pan_of(canvas);
    let zoom = zoom_of(canvas).max(0.01);
    let x = (client_x - rect.left() as f32 - pan_x) / zoom;
    let y = (client_y - rect.top() as f32 - pan_y) / zoom;
    (x, y)
}

/// If the pointer is near a viewport edge, pan the world in that direction.
/// Returns the pan delta applied in screen pixels.
pub fn edge_pan_if_needed(canvas: &Element, client_x: f32, client_y: f32) -> (f32, f32) {
    let rect = canvas.get_bounding_client_rect();
    let mut dx = 0.0;
    let mut dy = 0.0;
    if client_x < rect.left() as f32 + EDGE_PAN_PX {
        dx = EDGE_PAN_SPEED;
    } else if client_x > rect.right() as f32 - EDGE_PAN_PX {
        dx = -EDGE_PAN_SPEED;
    }
    if client_y < rect.top() as f32 + EDGE_PAN_PX {
        dy = EDGE_PAN_SPEED;
    } else if client_y > rect.bottom() as f32 - EDGE_PAN_PX {
        dy = -EDGE_PAN_SPEED;
    }
    if dx == 0.0 && dy == 0.0 {
        return (0.0, 0.0);
    }
    let (pan_x, pan_y) = pan_of(canvas);
    let zoom = zoom_of(canvas);
    set_view(canvas, pan_x + dx, pan_y + dy, zoom);
    (dx, dy)
}

/// Grow the grid so every container plus padding is on the surface.
pub fn ensure_manifold_extent(document: &Document) {
    let Some(canvas) = document.get_element_by_id("manifold-canvas") else {
        return;
    };
    let viewport_w = canvas.client_width() as f32;
    let viewport_h = canvas.client_height() as f32;
    let mut extent = WorldExtent::from_viewport(viewport_w, viewport_h);
    if let Ok(nodes) = document.query_selector_all(".canvas-content-layer > .canvas-container-node")
    {
        for i in 0..nodes.length() {
            let Some(node) = nodes.get(i) else { continue };
            let Ok(el) = node.dyn_into::<Element>() else {
                continue;
            };
            let style = el.get_attribute("style").unwrap_or_default();
            let (x, y, w, h) = rect_from_style(&style);
            extent.include_rect(x, y, w, h);
        }
    }
    size_world_surface(&canvas, extent);
}

pub fn pan_to_show(document: &Document, x: f32, y: f32, w: f32, h: f32) {
    let Some(canvas) = document.get_element_by_id("manifold-canvas") else {
        return;
    };
    let zoom = zoom_of(&canvas);
    let view_w = canvas.client_width() as f32;
    let view_h = canvas.client_height() as f32;
    let (pan_x, pan_y) = pan_of(&canvas);
    let left = x * zoom + pan_x;
    let top = y * zoom + pan_y;
    let right = (x + w) * zoom + pan_x;
    let bottom = (y + h) * zoom + pan_y;
    let mut next_x = pan_x;
    let mut next_y = pan_y;
    let margin = 48.0;
    if left < margin {
        next_x += margin - left;
    } else if right > view_w - margin {
        next_x -= right - (view_w - margin);
    }
    if top < margin {
        next_y += margin - top;
    } else if bottom > view_h - margin {
        next_y -= bottom - (view_h - margin);
    }
    if (next_x - pan_x).abs() > 0.5 || (next_y - pan_y).abs() > 0.5 {
        set_view(&canvas, next_x, next_y, zoom);
    }
    ensure_manifold_extent(document);
}

fn size_world_surface(canvas: &Element, extent: WorldExtent) {
    let left = format!("{}px", extent.min_x as i32);
    let top = format!("{}px", extent.min_y as i32);
    let width = format!("{}px", extent.width() as i32);
    let height = format!("{}px", extent.height() as i32);
    let view_box = format!(
        "{} {} {} {}",
        extent.min_x as i32,
        extent.min_y as i32,
        extent.width() as i32,
        extent.height() as i32
    );
    if let Ok(Some(grid)) = canvas.query_selector(".canvas-content-layer .canvas-grid-svg") {
        if let Ok(el) = grid.dyn_into::<HtmlElement>() {
            let _ = el.style().set_property("left", &left);
            let _ = el.style().set_property("top", &top);
            let _ = el.style().set_property("width", &width);
            let _ = el.style().set_property("height", &height);
        }
    }
    if let Ok(Some(overlay)) = canvas.query_selector(".canvas-content-layer .wire-overlay") {
        let _ = overlay.set_attribute("width", &format!("{}", extent.width() as i32));
        let _ = overlay.set_attribute("height", &format!("{}", extent.height() as i32));
        let _ = overlay.set_attribute("viewBox", &view_box);
        if let Ok(el) = overlay.clone().dyn_into::<HtmlElement>() {
            let _ = el.style().set_property("left", &left);
            let _ = el.style().set_property("top", &top);
            let _ = el.style().set_property("width", &width);
            let _ = el.style().set_property("height", &height);
            let _ = el.style().set_property("overflow", "visible");
        }
    }
}

fn rect_from_style(style: &str) -> (f32, f32, f32, f32) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut w = 0.0;
    let mut h = 0.0;
    for part in style.split(';') {
        if let Some((name, value)) = part.split_once(':') {
            let n = name.trim();
            let v = value.trim().trim_end_matches("px").parse().unwrap_or(0.0);
            match n {
                "left" => x = v,
                "top" => y = v,
                "width" => w = v,
                "height" => h = v,
                _ => {}
            }
        }
    }
    (x, y, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extent_grows_in_every_direction() {
        let mut extent = WorldExtent::from_viewport(800.0, 600.0);
        let start_min_x = extent.min_x;
        let start_max_x = extent.max_x;
        extent.include_rect(-400.0, -200.0, 100.0, 80.0);
        extent.include_rect(8000.0, 40.0, 400.0, 300.0);
        assert!(extent.min_x < start_min_x);
        assert!(extent.min_x <= -400.0 - WORLD_PAD);
        assert!(extent.max_x > start_max_x);
        assert!(extent.max_x >= 8400.0 + WORLD_PAD);
        assert!(extent.min_y <= -200.0 - WORLD_PAD);
    }

    #[test]
    fn empty_viewport_still_has_room_to_pan() {
        let extent = WorldExtent::from_viewport(100.0, 80.0);
        assert!(extent.width() >= MIN_SPAN);
        assert!(extent.height() >= MIN_SPAN);
        assert!(extent.min_x < 0.0);
        assert!(extent.min_y < 0.0);
    }
}
