//! Vector Holo UI & Desktop Studio Widget System (`ui.rs`)
//!
//! Re-exports the declarative vector UI primitives and layout solver from `etalon_render::ui_canvas`.

pub use etalon_render::ui_canvas::widgets::*;

pub mod ui_layout {
    pub use etalon_render::ui_canvas::{
        AlignItems, ComputedUiRect, Edges, FlexDirection, HudComputedUiRect, HudUiLayoutTree,
        HudVertex, JustifyContent, Margin, Padding, SizeRule, UiLayoutNode, UiLayoutStyle,
        UiLayoutTree, ZeroHeapUiLayoutTree, MAX_UI_LAYOUT_NODES, MAX_UI_NODES, NO_UI_NODE,
    };
}

pub use ui_layout::{
    AlignItems, ComputedUiRect, Edges, FlexDirection, JustifyContent, Margin, Padding, SizeRule,
    UiLayoutNode, UiLayoutStyle, UiLayoutTree, ZeroHeapUiLayoutTree, MAX_UI_LAYOUT_NODES,
    NO_UI_NODE,
};

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
