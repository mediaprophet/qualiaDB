//! Vector Holo UI & Desktop Studio Widget System (`ui.rs`)
//!
//! Provides zero-heap declarative vector UI primitives for Webizen Desktop apps
//! (Poet, Chora, Studio, and Game HUD):
//! - Glassmorphic rounded rectangle panels with frosted background styling
//! - Interactive widgets: Buttons, Sliders, Toggle Switches, Dials, and Text Inputs
//! - Floating & Docked Windows with draggable headers and collapse states
//! - 2D Visual Node Graphs with cubic bezier wire interconnects
//! - Zero-heap pointer hit-testing and event bubbling
//!
//! Designed according to `AGENTS.md` §0 (Zero heap in hot paths).

use crate::ui_layout::{ComputedUiRect, ZeroHeapUiLayoutTree, MAX_UI_LAYOUT_NODES};
use crate::ui_msdf_text::{
    shape_text_zero_heap, MsdfTextQuad, ShapedTextRunMetrics,
};
use bytemuck::Zeroable;

/// Maximum interactive widgets in a single local viewport frame (upgraded to 512 for Flexbox/Grid layout).
pub const MAX_UI_WIDGETS: usize = MAX_UI_LAYOUT_NODES;
/// Maximum connection wires in a node graph.
pub const MAX_UI_WIRES: usize = 64;
/// Maximum shaped MSDF text quads stored directly in a `ZeroHeapUiCanvas`.
pub const MAX_UI_TEXT_QUADS: usize = 512;

/// 2D bounding rectangle.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect2D {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect2D {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Tests if a point (px, py) lies within this rectangle.
    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.width && py >= self.y && py <= self.y + self.height
    }
}

/// RGBA normalized color [r, g, b, a].
pub type Color4f = [f32; 4];

/// Standard glassmorphic color presets.
pub const COLOR_GLASS_PANEL: Color4f = [0.08, 0.12, 0.18, 0.82];
pub const COLOR_GLASS_BORDER: Color4f = [0.25, 0.45, 0.70, 0.55];
pub const COLOR_ACCENT_CYAN: Color4f = [0.00, 0.85, 0.95, 1.00];
pub const COLOR_TEXT_PRIMARY: Color4f = [0.95, 0.98, 1.00, 1.00];

/// Interactive widget types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WidgetKind {
    /// Decorative or grouping container panel.
    Panel {
        corner_radius: f32,
        background: Color4f,
        border_color: Color4f,
        border_width: f32,
    },
    /// Clickable push button.
    Button {
        is_hovered: bool,
        is_pressed: bool,
    },
    /// Continuous or stepped value slider.
    Slider {
        value: f32,
        min: f32,
        max: f32,
    },
    /// On/Off toggle switch.
    Toggle {
        checked: bool,
    },
    /// Circular telemetry dial or knob.
    Dial {
        value: f32,
        min_angle_rad: f32,
        max_angle_rad: f32,
    },
    /// Node graph connection pin (input or output).
    GraphPin {
        node_id: u16,
        is_output: bool,
    },
}

/// A single UI widget element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiWidget {
    /// Unique 32-bit identifier for event dispatch.
    pub id: u32,
    /// Bounding box in canvas pixel coordinates.
    pub rect: Rect2D,
    /// Widget variant and specific state.
    pub kind: WidgetKind,
    /// True if widget ignores pointer events.
    pub disabled: bool,
}

/// Cubic bezier connection wire between two node graph pins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeWire {
    pub start_pin: u32,
    pub end_pin: u32,
    pub p0: [f32; 2],
    pub p1: [f32; 2],
    pub p2: [f32; 2],
    pub p3: [f32; 2],
    pub color: Color4f,
}

/// Fixed-capacity declarative UI canvas (upgraded to 512 Flexbox/Grid widgets + MSDF text runs).
#[derive(Debug, Clone, Copy)]
pub struct ZeroHeapUiCanvas {
    pub widget_count: usize,
    pub widgets: [UiWidget; MAX_UI_WIDGETS],
    pub clip_rects: [[f32; 4]; MAX_UI_WIDGETS],
    pub wire_count: usize,
    pub wires: [NodeWire; MAX_UI_WIRES],
    pub text_quad_count: usize,
    pub text_quads: [MsdfTextQuad; MAX_UI_TEXT_QUADS],
    pub active_focus_id: Option<u32>,
}

impl Default for ZeroHeapUiCanvas {
    fn default() -> Self {
        Self {
            widget_count: 0,
            widgets: [UiWidget {
                id: 0,
                rect: Rect2D::new(0.0, 0.0, 0.0, 0.0),
                kind: WidgetKind::Panel {
                    corner_radius: 0.0,
                    background: [0.0; 4],
                    border_color: [0.0; 4],
                    border_width: 0.0,
                },
                disabled: false,
            }; MAX_UI_WIDGETS],
            clip_rects: [[0.0, 0.0, 16384.0, 16384.0]; MAX_UI_WIDGETS],
            wire_count: 0,
            wires: [NodeWire {
                start_pin: 0,
                end_pin: 0,
                p0: [0.0; 2],
                p1: [0.0; 2],
                p2: [0.0; 2],
                p3: [0.0; 2],
                color: [0.0; 4],
            }; MAX_UI_WIRES],
            text_quad_count: 0,
            text_quads: [MsdfTextQuad::zeroed(); MAX_UI_TEXT_QUADS],
            active_focus_id: None,
        }
    }
}

impl ZeroHeapUiCanvas {
    /// Clears the canvas for the next frame.
    pub fn clear(&mut self) {
        self.widget_count = 0;
        self.wire_count = 0;
        self.text_quad_count = 0;
    }

    /// Synchronizes resolved `ComputedUiRect`s from a solved [`ZeroHeapUiLayoutTree`] directly into
    /// existing widgets (matched by `widget_id`) or appends new glassmorphic panels.
    pub fn apply_layout_tree(&mut self, tree: &ZeroHeapUiLayoutTree) {
        for i in 0..tree.count {
            let node = &tree.nodes[i];
            let computed: ComputedUiRect = node.computed;
            let rect = Rect2D::new(computed.x, computed.y, computed.width, computed.height);

            // Update existing widget if already registered with `node.widget_id`
            let mut found = false;
            for w_idx in 0..self.widget_count {
                if self.widgets[w_idx].id == node.widget_id {
                    self.widgets[w_idx].rect = rect;
                    self.clip_rects[w_idx] = computed.clip_rect;
                    found = true;
                    break;
                }
            }

            if !found && self.widget_count < MAX_UI_WIDGETS {
                let slot = self.widget_count;
                self.widgets[slot] = UiWidget {
                    id: node.widget_id,
                    rect,
                    kind: WidgetKind::Panel {
                        corner_radius: 8.0,
                        background: COLOR_GLASS_PANEL,
                        border_color: COLOR_GLASS_BORDER,
                        border_width: 1.0,
                    },
                    disabled: false,
                };
                self.clip_rects[slot] = computed.clip_rect;
                self.widget_count += 1;
            }
        }
    }

    /// Shapes a UTF-8 text run into GPU MSDF glyph quads and appends them to `self.text_quads` in zero heap.
    pub fn add_text_run(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_size_px: f32,
        max_width_px: f32,
        color: Color4f,
    ) -> ShapedTextRunMetrics {
        if self.text_quad_count >= MAX_UI_TEXT_QUADS {
            return ShapedTextRunMetrics::default();
        }
        let rem = &mut self.text_quads[self.text_quad_count..];
        let metrics = shape_text_zero_heap(text, x, y, font_size_px, 1.25, max_width_px, color, rem);
        self.text_quad_count += metrics.quad_count;
        metrics
    }

    /// Adds a panel container widget.
    pub fn add_panel(&mut self, id: u32, rect: Rect2D, corner_radius: f32) -> bool {
        if self.widget_count >= MAX_UI_WIDGETS {
            return false;
        }
        self.widgets[self.widget_count] = UiWidget {
            id,
            rect,
            kind: WidgetKind::Panel {
                corner_radius,
                background: COLOR_GLASS_PANEL,
                border_color: COLOR_GLASS_BORDER,
                border_width: 1.5,
            },
            disabled: false,
        };
        self.widget_count += 1;
        true
    }

    /// Adds a push button widget.
    pub fn add_button(&mut self, id: u32, rect: Rect2D) -> bool {
        if self.widget_count >= MAX_UI_WIDGETS {
            return false;
        }
        self.widgets[self.widget_count] = UiWidget {
            id,
            rect,
            kind: WidgetKind::Button {
                is_hovered: false,
                is_pressed: false,
            },
            disabled: false,
        };
        self.widget_count += 1;
        true
    }

    /// Adds a slider widget.
    pub fn add_slider(&mut self, id: u32, rect: Rect2D, value: f32, min: f32, max: f32) -> bool {
        if self.widget_count >= MAX_UI_WIDGETS {
            return false;
        }
        self.widgets[self.widget_count] = UiWidget {
            id,
            rect,
            kind: WidgetKind::Slider {
                value: value.clamp(min, max),
                min,
                max,
            },
            disabled: false,
        };
        self.widget_count += 1;
        true
    }

    /// Adds a cubic bezier wire between two node graph pins.
    pub fn add_wire(&mut self, start_pin: u32, end_pin: u32, p0: [f32; 2], p3: [f32; 2], color: Color4f) -> bool {
        if self.wire_count >= MAX_UI_WIRES {
            return false;
        }
        // Horizontal tangent offset for smooth S-curve wires
        let dx = (p3[0] - p0[0]).abs() * 0.5;
        let p1 = [p0[0] + dx, p0[1]];
        let p2 = [p3[0] - dx, p3[1]];

        self.wires[self.wire_count] = NodeWire {
            start_pin,
            end_pin,
            p0,
            p1,
            p2,
            p3,
            color,
        };
        self.wire_count += 1;
        true
    }

    /// Evaluates pointer hit test, returning topmost widget ID under (px, py).
    pub fn hit_test(&self, px: f32, py: f32) -> Option<u32> {
        // Iterate in reverse (topmost widgets drawn last)
        for i in (0..self.widget_count).rev() {
            let w = &self.widgets[i];
            if !w.disabled && w.rect.contains_point(px, py) {
                return Some(w.id);
            }
        }
        None
    }

    /// Dispatches pointer move event to update hover states.
    pub fn on_pointer_move(&mut self, px: f32, py: f32) {
        for i in 0..self.widget_count {
            let w = &mut self.widgets[i];
            let inside = w.rect.contains_point(px, py);
            if let WidgetKind::Button { ref mut is_hovered, .. } = w.kind {
                *is_hovered = inside;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_hit_testing() {
        let mut ui = ZeroHeapUiCanvas::default();
        ui.add_panel(1, Rect2D::new(0.0, 0.0, 400.0, 300.0), 12.0);
        ui.add_button(2, Rect2D::new(50.0, 50.0, 100.0, 40.0));

        // Click outside
        assert_eq!(ui.hit_test(500.0, 500.0), None);

        // Click on button (should hit button 2, not panel 1)
        assert_eq!(ui.hit_test(60.0, 60.0), Some(2));

        // Click on panel outside button
        assert_eq!(ui.hit_test(200.0, 200.0), Some(1));
    }

    #[test]
    fn test_node_wire_curve() {
        let mut ui = ZeroHeapUiCanvas::default();
        let added = ui.add_wire(10, 20, [100.0, 100.0], [300.0, 200.0], COLOR_ACCENT_CYAN);
        assert!(added);
        assert_eq!(ui.wire_count, 1);
        let wire = &ui.wires[0];
        assert_eq!(wire.p0, [100.0, 100.0]);
        assert_eq!(wire.p3, [300.0, 200.0]);
        // Tangents p1 and p2 must have positive dx offset
        assert!(wire.p1[0] > wire.p0[0]);
        assert!(wire.p2[0] < wire.p3[0]);
    }

    #[test]
    fn test_layout_tree_and_msdf_text_integration() {
        use crate::ui_layout::{FlexDirection, Padding, SizeRule, UiLayoutStyle, NO_UI_NODE};

        let mut tree = ZeroHeapUiLayoutTree::new();
        let root = tree
            .add_node(
                100,
                NO_UI_NODE,
                UiLayoutStyle {
                    direction: FlexDirection::Row,
                    width: SizeRule::Fixed(500.0),
                    height: SizeRule::Fixed(80.0),
                    padding: Padding::all(10.0),
                    gap: 10.0,
                    clip_children: true,
                    ..Default::default()
                },
                [0.0, 0.0],
            )
            .unwrap();
        let _btn = tree
            .add_node(
                101,
                root,
                UiLayoutStyle {
                    width: SizeRule::FlexGrow(1.0),
                    height: SizeRule::Fixed(40.0),
                    ..Default::default()
                },
                [0.0, 0.0],
            )
            .unwrap();
        tree.solve(500.0, 80.0);

        let mut ui = ZeroHeapUiCanvas::default();
        ui.add_button(101, Rect2D::new(0.0, 0.0, 0.0, 0.0));
        ui.apply_layout_tree(&tree);

        assert_eq!(ui.widget_count, 2);
        // Widget 101 was pre-registered at slot 0 and updated by apply_layout_tree
        assert_eq!(ui.widgets[0].id, 101);
        assert!((ui.widgets[0].rect.width - 480.0).abs() < 1e-4);
        assert_eq!(ui.clip_rects[0], [10.0, 10.0, 490.0, 70.0]);

        let text_metrics = ui.add_text_run(
            "Qualia 10D α=1.0",
            14.0,
            18.0,
            14.0,
            200.0,
            COLOR_TEXT_PRIMARY,
        );
        assert!(text_metrics.quad_count > 5);
        assert_eq!(ui.text_quad_count, text_metrics.quad_count);
    }
}
