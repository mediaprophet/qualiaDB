//! Generic in-viewport controls for Qualia Portal clients.
//!
//! A client supplies a bounded declarative document in canvas coordinates. Qualia owns
//! painting, pointer hit testing, and focus traversal; applications own action semantics.
//! Parsing may allocate, while paint and hit testing reuse the parsed document.

pub mod cbor_ld_doc;

use crate::render::camera::CameraState;
pub use cbor_ld_doc::{
    decode_hud_document_cbor_ld, encode_hud_document_cbor_ld, HUD_CBOR_LD_TAG_COMPRESSED,
    HUD_CBOR_LD_TAG_UNCOMPRESSED,
};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

#[cfg(feature = "gpu-runtime")]
pub use etalon_render::hud::{
    AlignItems as HudAlignItems, ComputedUiRect as HudComputedRect, Edges as HudEdges,
    FlexDirection as HudFlexDirection, HudVertex, JustifyContent as HudJustifyContent,
    Margin as HudMargin, Padding as HudPadding, SizeRule as HudSizeRule,
    UiLayoutNode as HudLayoutNode, UiLayoutStyle as HudLayoutStyle, UiLayoutTree as HudLayoutTree,
    MAX_UI_NODES as MAX_HUD_LAYOUT_NODES, NO_UI_NODE as NO_HUD_NODE,
};

#[derive(Deserialize)]
struct HudDocument {
    items: Vec<HudItem>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct HudItem {
    #[serde(default)]
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub text: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub z: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default)]
    pub value: f64,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub color: String,
}

/// Solves flex layout for a slice of HUD items using Etalon's 2D vector HUD layout engine.
///
/// Auto-sizes items where `w == 0.0` or `h == 0.0` against the available viewport size.
#[cfg(feature = "gpu-runtime")]
pub fn compute_hud_flex_layout(items: &mut [HudItem], viewport_w: f32, viewport_h: f32) {
    compute_hud_flex_layout_internal(items, viewport_w, viewport_h, false);
}

/// Solves and applies flex layout positions and dimensions for all non-marker HUD items.
#[cfg(feature = "gpu-runtime")]
pub fn compute_hud_flex_layout_all(items: &mut [HudItem], viewport_w: f32, viewport_h: f32) {
    compute_hud_flex_layout_internal(items, viewport_w, viewport_h, true);
}

#[cfg(feature = "gpu-runtime")]
fn compute_hud_flex_layout_internal(
    items: &mut [HudItem],
    viewport_w: f32,
    viewport_h: f32,
    apply_all: bool,
) {
    if items.is_empty() || viewport_w <= 0.0 || viewport_h <= 0.0 {
        return;
    }

    let mut tree = etalon_render::hud::UiLayoutTree::new();
    let root_style = etalon_render::hud::UiLayoutStyle {
        direction: etalon_render::hud::FlexDirection::Column,
        justify_content: etalon_render::hud::JustifyContent::Start,
        align_items: etalon_render::hud::AlignItems::Stretch,
        width: etalon_render::hud::SizeRule::Fixed(viewport_w),
        height: etalon_render::hud::SizeRule::Fixed(viewport_h),
        padding: etalon_render::hud::Padding::default(),
        margin: etalon_render::hud::Margin::default(),
        gap: 4.0,
    };
    let root_id = tree
        .add_node(
            etalon_render::hud::NO_UI_NODE,
            root_style,
            [0.0, 0.0, 0.0, 0.0],
        )
        .unwrap_or(0);

    for (idx, item) in items
        .iter()
        .enumerate()
        .take(etalon_render::hud::MAX_UI_NODES - 1)
    {
        if item.kind == "marker" {
            continue;
        }
        let width_rule = if item.w > 0.0 {
            etalon_render::hud::SizeRule::Fixed(item.w as f32)
        } else {
            etalon_render::hud::SizeRule::Auto
        };
        let height_rule = if item.h > 0.0 {
            etalon_render::hud::SizeRule::Fixed(item.h as f32)
        } else {
            etalon_render::hud::SizeRule::Auto
        };
        let style = etalon_render::hud::UiLayoutStyle {
            direction: etalon_render::hud::FlexDirection::Row,
            justify_content: etalon_render::hud::JustifyContent::Start,
            align_items: etalon_render::hud::AlignItems::Stretch,
            width: width_rule,
            height: height_rule,
            padding: etalon_render::hud::Padding::default(),
            margin: etalon_render::hud::Margin::default(),
            gap: 0.0,
        };
        tree.add_node(root_id, style, [idx as f32, 0.0, 0.0, 1.0]);
    }

    tree.solve(viewport_w, viewport_h);

    let mut vertices = [etalon_render::hud::HudVertex {
        pos: [0.0, 0.0],
        uv: [0.0, 0.0],
        color: [0.0; 4],
    }; etalon_render::hud::MAX_UI_NODES * 6];

    let vert_count = tree.generate_quad_vertices(&mut vertices);
    for chunk in vertices[..vert_count].chunks_exact(6) {
        let idx = chunk[0].color[0].round() as usize;
        if let Some(item) = items.get_mut(idx) {
            let x0 = chunk[0].pos[0] as f64;
            let y0 = chunk[0].pos[1] as f64;
            let x1 = chunk[2].pos[0] as f64;
            let y1 = chunk[2].pos[1] as f64;
            let cw = (x1 - x0).max(0.0);
            let ch = (y1 - y0).max(0.0);

            if apply_all {
                item.x = x0;
                item.y = y0;
                if item.w == 0.0 {
                    item.w = cw;
                }
                if item.h == 0.0 {
                    item.h = ch;
                }
            } else {
                if item.w == 0.0 {
                    item.w = cw;
                    item.x = x0;
                }
                if item.h == 0.0 {
                    item.h = ch;
                    item.y = y0;
                }
            }
        }
    }
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
    /// Creates `QualiaHud` with the given state.
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
                "panel" | "label" | "button" | "meter" | "marker" | "mirror"
            ) || item.id.len() > 96
                || item.text.len() > 512
                || item.icon.len() > 24
                || (item.kind == "marker" && !valid_color(&item.color))
                || (item.kind == "label" && !item.color.is_empty() && !valid_color(&item.color))
                || ![item.x, item.y, item.z, item.w, item.h, item.value]
                    .iter()
                    .all(|v| v.is_finite())
                || item.w < 0.0
                || item.h < 0.0
                || item.w > 8192.0
                || item.h > 8192.0
                || item.x < -8192.0
                || item.y < -8192.0
                || (matches!(item.kind.as_str(), "meter" | "mirror")
                    && !item.color.is_empty()
                    && !valid_color(&item.color))
            {
                return Err(JsValue::from_str("invalid HUD item"));
            }
        }
        self.items = doc.items;
        #[cfg(feature = "gpu-runtime")]
        {
            let cw = self.canvas.width() as f32;
            let ch = self.canvas.height() as f32;
            if cw > 0.0 && ch > 0.0 {
                compute_hud_flex_layout(&mut self.items, cw, ch);
            }
        }
        self.focus = None;
        self.paint();
        Ok(())
    }

    /// Solves flex layout for all HUD items using the Etalon 2D vector HUD layout engine.
    /// Updates position and dimensions for auto-sized items (`w == 0.0` or `h == 0.0`).
    /// Replace the presentation document directly from binary W3C CBOR-LD (`0xD9 0x0501`).
    pub fn set_document_cbor_ld(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        let items = decode_hud_document_cbor_ld(bytes).map_err(|e| JsValue::from_str(&e))?;
        self.items = items;
        #[cfg(feature = "gpu-runtime")]
        {
            let cw = self.canvas.width() as f32;
            let ch = self.canvas.height() as f32;
            if cw > 0.0 && ch > 0.0 {
                compute_hud_flex_layout(&mut self.items, cw, ch);
            }
        }
        self.focus = None;
        self.paint();
        Ok(())
    }

    pub fn layout_items_flex(&mut self, viewport_w: f32, viewport_h: f32) {
        #[cfg(feature = "gpu-runtime")]
        {
            compute_hud_flex_layout_all(&mut self.items, viewport_w, viewport_h);
            self.paint();
        }
    }

    /// Use the same orbit camera as Portal to project world markers into HUD pixels.
    pub fn set_camera_target(&mut self, yaw: f32, pitch: f32, zoom: f32, x: f32, y: f32, z: f32) {
        self.camera = CameraState::new(yaw, pitch, zoom)
            .with_near_plane(self.camera.near_plane)
            .with_target([x, y, z])
            .clamped();
        self.paint();
    }
    /// Sets `camera_near_plane`.
    pub fn set_camera_near_plane(&mut self, near: f32) -> Result<(), JsValue> {
        if !near.is_finite() || !(0.00001..=1.0).contains(&near) {
            return Err(JsValue::from_str("HUD near plane invalid"));
        }
        self.camera.near_plane = near;
        Ok(())
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
                    ctx.set_fill_style_str(if item.color.is_empty() {
                        "#fff1d0"
                    } else {
                        &item.color
                    });
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
                    ctx.set_fill_style_str(if item.color.is_empty() {
                        "#78c6a3"
                    } else {
                        &item.color
                    });
                    ctx.fill_rect(item.x, item.y, item.w * item.value.clamp(0.0, 1.0), item.h);
                }
                "mirror" => draw_rear_mirror(ctx, item),
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

    /// Focused HUD action.
    pub fn focused_action(&self) -> String {
        self.focus
            .and_then(|i| self.items.get(i))
            .filter(|item| item.kind == "button" && !item.disabled)
            .map(|item| item.id.clone())
            .unwrap_or_default()
    }
}

impl QualiaHud {
    /// Reference to the current HUD items.
    pub fn items(&self) -> &[HudItem] {
        &self.items
    }

    /// Mutable reference to the current HUD items.
    pub fn items_mut(&mut self) -> &mut [HudItem] {
        &mut self.items
    }

    fn marker_position(&self, item: &HudItem) -> Option<(f64, f64)> {
        let w = self.canvas.width() as f32;
        let h = self.canvas.height() as f32;
        if w == 0.0 || h == 0.0 {
            return None;
        }
        let m = self.camera.view_projection(w / h);
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

pub(crate) fn validate_hud_items(items: &[HudItem]) -> Result<(), &'static str> {
    if items.len() > 256 {
        return Err("HUD has too many items");
    }
    for item in items {
        if !matches!(
            item.kind.as_str(),
            "panel" | "label" | "button" | "meter" | "marker" | "mirror"
        ) || item.id.len() > 96
            || item.text.len() > 512
            || item.icon.len() > 24
            || (item.kind == "marker" && !valid_color(&item.color))
            || (item.kind == "label" && !item.color.is_empty() && !valid_color(&item.color))
            || ![item.x, item.y, item.z, item.w, item.h, item.value]
                .iter()
                .all(|v| v.is_finite())
            || item.w < 0.0
            || item.h < 0.0
            || item.w > 8192.0
            || item.h > 8192.0
            || item.x < -8192.0
            || item.y < -8192.0
            || (matches!(item.kind.as_str(), "meter" | "mirror")
                && !item.color.is_empty()
                && !valid_color(&item.color))
        {
            return Err("invalid HUD item");
        }
    }
    Ok(())
}

fn valid_color(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

fn draw_rear_mirror(ctx: &CanvasRenderingContext2d, item: &HudItem) {
    let x = item.x;
    let y = item.y;
    let w = item.w;
    let h = item.h;
    ctx.set_fill_style_str("#352923");
    ctx.fill_rect(x, y, w, h);
    ctx.set_stroke_style_str("#d8b477");
    ctx.stroke_rect(x + 0.5, y + 0.5, (w - 1.0).max(0.0), (h - 1.0).max(0.0));

    let pad = 4.0;
    let ix = x + pad;
    let iy = y + pad;
    let iw = (w - pad * 2.0).max(1.0);
    let ih = (h - pad * 2.0).max(1.0);
    ctx.save();
    ctx.begin_path();
    ctx.rect(ix, iy, iw, ih);
    ctx.clip();

    ctx.set_fill_style_str(if item.disabled { "#929f96" } else { "#a6c7c1" });
    ctx.fill_rect(ix, iy, iw, ih * 0.55);
    ctx.set_fill_style_str("#66715f");
    ctx.fill_rect(ix, iy + ih * 0.55, iw, ih * 0.45);
    ctx.set_fill_style_str("#d8c6a4");
    ctx.fill_rect(ix, iy + ih * 0.54, iw, (ih * 0.025).max(1.0));

    // A road narrows toward the horizon; its bend makes the traffic line visible.
    ctx.begin_path();
    if item.disabled {
        ctx.move_to(ix + iw * 0.39, iy + ih * 0.56);
        ctx.line_to(ix + iw * 0.60, iy + ih * 0.56);
        ctx.line_to(ix + iw * 0.82, iy + ih);
        ctx.line_to(ix + iw * 0.16, iy + ih);
    } else {
        ctx.move_to(ix + iw * 0.48, iy + ih * 0.56);
        ctx.line_to(ix + iw * 0.58, iy + ih * 0.56);
        ctx.line_to(ix + iw * 0.94, iy + ih);
        ctx.line_to(ix + iw * 0.07, iy + ih);
    }
    ctx.close_path();
    ctx.set_fill_style_str("#555955");
    ctx.fill();

    let vehicles = item.value.floor().clamp(0.0, 7.0) as usize;
    for index in (0..vehicles).rev() {
        let distance = (index as f64 + 1.0) / (vehicles as f64 + 1.0);
        let scale = 0.20 + distance * 0.58;
        let center = if item.disabled {
            ix + iw * (0.49 + (index % 2) as f64 * 0.08)
        } else {
            ix + iw * (0.52 + distance * 0.18)
        };
        let car_y = iy + ih * (0.58 + distance * 0.28);
        let car_w = iw * 0.12 * scale;
        let car_h = ih * 0.12 * scale;
        ctx.set_fill_style_str("#263b40");
        ctx.fill_rect(center - car_w * 0.5, car_y, car_w, car_h);
        ctx.set_fill_style_str("#f5d98b");
        ctx.fill_rect(
            center - car_w * 0.42,
            car_y + car_h * 0.72,
            car_w * 0.18,
            car_h * 0.16,
        );
        ctx.fill_rect(
            center + car_w * 0.24,
            car_y + car_h * 0.72,
            car_w * 0.18,
            car_h * 0.16,
        );
    }

    if item.disabled {
        // The towed caravan occupies the centre view; only fragments of traffic
        // remain visible at the mirror edges until a bend opens the sightline.
        ctx.set_fill_style_str("#e5d7bd");
        ctx.fill_rect(ix + iw * 0.30, iy + ih * 0.47, iw * 0.43, ih * 0.55);
        ctx.set_fill_style_str("#78949a");
        ctx.fill_rect(ix + iw * 0.35, iy + ih * 0.53, iw * 0.13, ih * 0.19);
        ctx.fill_rect(ix + iw * 0.55, iy + ih * 0.53, iw * 0.13, ih * 0.19);
        ctx.set_fill_style_str("#342f2b");
        ctx.fill_rect(ix + iw * 0.36, iy + ih * 0.91, iw * 0.10, ih * 0.13);
        ctx.fill_rect(ix + iw * 0.62, iy + ih * 0.91, iw * 0.10, ih * 0.13);
    }

    ctx.restore();
    ctx.set_fill_style_str(if item.color.is_empty() {
        "#f0c26f"
    } else {
        &item.color
    });
    ctx.fill_rect(x + 6.0, y + 6.0, 5.0, 5.0);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hud_cbor_ld_encode_decode_round_trip_and_bounds() {
        let items = vec![
            HudItem {
                id: "panel_main".into(),
                kind: "panel".into(),
                text: "".into(),
                x: 12.0,
                y: 24.0,
                z: 0.0,
                w: 320.0,
                h: 180.0,
                value: 0.0,
                disabled: false,
                icon: "".into(),
                color: "".into(),
            },
            HudItem {
                id: "btn_submit".into(),
                kind: "button".into(),
                text: "Confirm".into(),
                x: 20.0,
                y: 40.0,
                z: 0.0,
                w: 120.0,
                h: 36.0,
                value: 1.0,
                disabled: false,
                icon: "civic".into(),
                color: "#78c6a3".into(),
            },
        ];

        let encoded = encode_hud_document_cbor_ld(&items);
        // Must start with W3C CBOR-LD compressed tag 0xD9 0x0501
        assert_eq!(&encoded[0..3], &[0xD9, 0x05, 0x01]);
        let decoded = decode_hud_document_cbor_ld(&encoded).unwrap();
        assert_eq!(decoded, items);

        // Invalid HUD item kind must fail validation on decode
        let bad_items = vec![HudItem {
            id: "bad".into(),
            kind: "script_injection".into(),
            text: "".into(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 100.0,
            h: 40.0,
            value: 0.0,
            disabled: false,
            icon: "".into(),
            color: "".into(),
        }];
        let bad_bytes = encode_hud_document_cbor_ld(&bad_items);
        assert!(decode_hud_document_cbor_ld(&bad_bytes).is_err());
    }

    #[test]
    #[cfg(feature = "gpu-runtime")]
    fn test_compute_hud_flex_layout_auto_size() {
        let mut items = vec![
            HudItem {
                id: "panel_1".into(),
                kind: "panel".into(),
                text: "".into(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 0.0,
                h: 0.0,
                value: 0.0,
                disabled: false,
                icon: "".into(),
                color: "".into(),
            },
            HudItem {
                id: "btn_1".into(),
                kind: "button".into(),
                text: "Click".into(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 120.0,
                h: 40.0,
                value: 0.0,
                disabled: false,
                icon: "".into(),
                color: "".into(),
            },
        ];

        compute_hud_flex_layout(&mut items, 800.0, 600.0);
        assert!(items[0].w > 0.0, "auto width should be computed");
        assert!(items[0].h > 0.0, "auto height should be computed");
        assert_eq!(items[1].w, 120.0, "fixed width should be preserved");
        assert_eq!(items[1].h, 40.0, "fixed height should be preserved");
    }

    #[test]
    #[cfg(feature = "gpu-runtime")]
    fn test_compute_hud_flex_layout_all_positions() {
        let mut items = vec![
            HudItem {
                id: "btn_1".into(),
                kind: "button".into(),
                text: "First".into(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 100.0,
                h: 30.0,
                value: 0.0,
                disabled: false,
                icon: "".into(),
                color: "".into(),
            },
            HudItem {
                id: "btn_2".into(),
                kind: "button".into(),
                text: "Second".into(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 100.0,
                h: 30.0,
                value: 0.0,
                disabled: false,
                icon: "".into(),
                color: "".into(),
            },
        ];

        compute_hud_flex_layout_all(&mut items, 800.0, 600.0);
        assert_eq!(items[0].y, 0.0);
        assert!(items[1].y > 0.0, "second child in column should have y offset");
    }
}
