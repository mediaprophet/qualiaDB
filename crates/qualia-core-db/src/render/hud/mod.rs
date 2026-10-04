//! Generic in-viewport controls for Qualia Portal clients.
//!
//! A client supplies a bounded declarative document in canvas coordinates. Qualia owns
//! painting, pointer hit testing, and focus traversal; applications own action semantics.
//! Parsing may allocate, while paint and hit testing reuse the parsed document.

use crate::render::camera::{orbit_view_projection_target, CameraState};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

#[derive(Deserialize)]
struct HudDocument {
    items: Vec<HudItem>,
}

#[derive(Deserialize)]
struct HudItem {
    #[serde(default)]
    id: String,
    kind: String,
    #[serde(default)]
    text: String,
    x: f64,
    y: f64,
    #[serde(default)]
    z: f64,
    w: f64,
    h: f64,
    #[serde(default)]
    value: f64,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    color: String,
}

/// Browser canvas HUD. Place its transparent canvas above the Portal canvas.
/// Coordinates are in canvas pixels, independent of CSS scaling.
#[wasm_bindgen]
pub struct QualiaHud {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    items: Vec<HudItem>,
    focus: Option<usize>,
    camera: CameraState,
}

#[wasm_bindgen]
impl QualiaHud {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<QualiaHud, JsValue> {
        let ctx = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("HUD canvas has no 2D context"))?
            .dyn_into::<CanvasRenderingContext2d>()?;
        Ok(Self {
            canvas,
            ctx,
            items: Vec::new(),
            focus: None,
            camera: CameraState::default(),
        })
    }

    /// Replace the presentation document. Bounds and size are checked before adoption.
    pub fn set_document_json(&mut self, json: &str) -> Result<(), JsValue> {
        if json.len() > 96_000 {
            return Err(JsValue::from_str("HUD document too large"));
        }
        let doc: HudDocument =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        if doc.items.len() > 256 {
            return Err(JsValue::from_str("HUD has too many items"));
        }
        for item in &doc.items {
            if !matches!(
                item.kind.as_str(),
                "panel" | "label" | "button" | "meter" | "marker"
            ) || item.id.len() > 96
                || item.text.len() > 512
                || item.icon.len() > 24
                || (item.kind == "marker" && !valid_color(&item.color))
                || ![item.x, item.y, item.z, item.w, item.h, item.value]
                    .iter()
                    .all(|v| v.is_finite())
                || item.w < 0.0
                || item.h < 0.0
                || item.w > 8192.0
                || item.h > 8192.0
                || item.x < -8192.0
                || item.y < -8192.0
            {
                return Err(JsValue::from_str("invalid HUD item"));
            }
        }
        self.items = doc.items;
        self.focus = None;
        self.paint();
        Ok(())
    }

    /// Use the same orbit camera as Portal to project world markers into HUD pixels.
    pub fn set_camera_target(&mut self, yaw: f32, pitch: f32, zoom: f32, x: f32, y: f32, z: f32) {
        self.camera = CameraState::new(yaw, pitch, zoom)
            .with_target([x, y, z])
            .clamped();
        self.paint();
    }

    /// Repaint the current document. Uses no Rust heap allocations.
    pub fn paint(&self) {
        let ctx = &self.ctx;
        ctx.clear_rect(
            0.0,
            0.0,
            self.canvas.width() as f64,
            self.canvas.height() as f64,
        );
        for (index, item) in self.items.iter().enumerate() {
            match item.kind.as_str() {
                "marker" => {
                    if let Some((x, y)) = self.marker_position(item) {
                        draw_marker(ctx, item, x, y, self.focus == Some(index));
                    }
                }
                "panel" => {
                    ctx.set_fill_style_str("rgba(16,35,44,0.92)");
                    ctx.fill_rect(item.x, item.y, item.w, item.h);
                    ctx.set_stroke_style_str("#d8b477");
                    ctx.stroke_rect(item.x + 0.5, item.y + 0.5, item.w - 1.0, item.h - 1.0);
                }
                "button" => {
                    ctx.set_fill_style_str(if item.disabled {
                        "#40545a"
                    } else if self.focus == Some(index) {
                        "#f7d69d"
                    } else {
                        "#e9ad6a"
                    });
                    ctx.fill_rect(item.x, item.y, item.w, item.h);
                    ctx.set_stroke_style_str("#573e35");
                    ctx.stroke_rect(item.x + 0.5, item.y + 0.5, item.w - 1.0, item.h - 1.0);
                    ctx.set_fill_style_str(if item.disabled { "#a9b5b0" } else { "#263b43" });
                    ctx.set_font("bold 15px Trebuchet MS, sans-serif");
                    ctx.set_text_baseline("middle");
                    let _ = ctx.fill_text_with_max_width(
                        &item.text,
                        item.x + 10.0,
                        item.y + item.h * 0.5,
                        (item.w - 20.0).max(1.0),
                    );
                }
                "label" => {
                    ctx.set_fill_style_str("#fff1d0");
                    ctx.set_font("bold 15px Trebuchet MS, sans-serif");
                    ctx.set_text_baseline("middle");
                    let _ = ctx.fill_text_with_max_width(
                        &item.text,
                        item.x,
                        item.y + item.h * 0.5,
                        item.w.max(1.0),
                    );
                }
                "meter" => {
                    ctx.set_fill_style_str("#46585b");
                    ctx.fill_rect(item.x, item.y, item.w, item.h);
                    ctx.set_fill_style_str("#78c6a3");
                    ctx.fill_rect(item.x, item.y, item.w * item.value.clamp(0.0, 1.0), item.h);
                }
                _ => {}
            }
        }
    }

    /// Return an action ID for a button under the pointer, in canvas pixels.
    pub fn hit_test(&mut self, x: f64, y: f64) -> String {
        for (index, item) in self.items.iter().enumerate().rev() {
            if item.kind == "panel" && inside(item, x, y) {
                return String::new();
            }
            if item.kind == "button" && !item.disabled && inside(item, x, y) {
                self.focus = Some(index);
                self.paint();
                return item.id.clone();
            }
            if item.kind == "marker" && !item.disabled {
                if let Some((mx, my)) = self.marker_position(item) {
                    if (x - mx).powi(2) + (y - my).powi(2) <= 18.0_f64.powi(2) {
                        self.focus = Some(index);
                        self.paint();
                        return item.id.clone();
                    }
                }
            }
        }
        String::new()
    }

    /// Move focus between enabled buttons and return the focused action ID.
    pub fn focus_next(&mut self, reverse: bool) -> String {
        let len = self.items.len();
        if len == 0 {
            return String::new();
        }
        let start = self.focus.unwrap_or(if reverse { 0 } else { len - 1 });
        for step in 1..=len {
            let index = if reverse {
                (start + len - step % len) % len
            } else {
                (start + step) % len
            };
            let item = &self.items[index];
            if item.kind == "button" && !item.disabled {
                self.focus = Some(index);
                self.paint();
                return item.id.clone();
            }
        }
        String::new()
    }

    pub fn focused_action(&self) -> String {
        self.focus
            .and_then(|i| self.items.get(i))
            .filter(|item| item.kind == "button" && !item.disabled)
            .map(|item| item.id.clone())
            .unwrap_or_default()
    }
}

impl QualiaHud {
    fn marker_position(&self, item: &HudItem) -> Option<(f64, f64)> {
        let w = self.canvas.width() as f32;
        let h = self.canvas.height() as f32;
        if w == 0.0 || h == 0.0 {
            return None;
        }
        let m = orbit_view_projection_target(
            self.camera.yaw,
            self.camera.pitch,
            self.camera.zoom,
            self.camera.target,
            w / h,
        );
        let p = [item.x as f32, item.y as f32, item.z as f32, 1.0];
        let clip: [f32; 4] = std::array::from_fn(|r| (0..4).map(|c| m[c][r] * p[c]).sum());
        if clip[3] <= 0.0 {
            return None;
        }
        let nx = clip[0] / clip[3];
        let ny = clip[1] / clip[3];
        if nx.abs() > 1.05 || ny.abs() > 1.05 {
            return None;
        }
        Some((((nx + 1.0) * w * 0.5) as f64, ((1.0 - ny) * h * 0.5) as f64))
    }
}

fn inside(item: &HudItem, x: f64, y: f64) -> bool {
    x >= item.x && y >= item.y && x < item.x + item.w && y < item.y + item.h
}

fn valid_color(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

fn draw_marker(ctx: &CanvasRenderingContext2d, item: &HudItem, x: f64, y: f64, focused: bool) {
    let radius = if focused { 16.0 } else { 13.0 };
    ctx.begin_path();
    let _ = ctx.arc(x, y, radius, 0.0, std::f64::consts::TAU);
    ctx.set_fill_style_str(&item.color);
    ctx.fill();
    ctx.set_line_width(if focused { 3.0 } else { 2.0 });
    ctx.set_stroke_style_str("#fff8e8");
    ctx.stroke();
    ctx.begin_path();
    ctx.move_to(x - 4.0, y + radius - 1.0);
    ctx.line_to(x, y + radius + 7.0);
    ctx.line_to(x + 4.0, y + radius - 1.0);
    ctx.set_fill_style_str(&item.color);
    ctx.fill();
    ctx.set_fill_style_str("#fff8e8");
    ctx.set_font("bold 17px Trebuchet MS, sans-serif");
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    let glyph = match item.icon.as_str() {
        "home" => "⌂",
        "salvage" => "↻",
        "trade" => "◆",
        "industry" => "⚙",
        "civic" => "✦",
        "water" => "●",
        "food" => "✿",
        "route" => "═",
        "energy" => "ϟ",
        _ => "•",
    };
    let _ = ctx.fill_text(glyph, x, y + 0.5);
    ctx.set_text_align("start");
    if focused {
        ctx.set_font("bold 13px Trebuchet MS, sans-serif");
        ctx.set_fill_style_str("#fff8e8");
        let _ = ctx.fill_text(&item.text, x + 22.0, y);
    }
}
