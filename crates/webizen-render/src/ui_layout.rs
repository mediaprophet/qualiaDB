//! Zero-Heap Flexbox & CSS-Grid-Lite UI Layout Constraint Solver (`ui_layout.rs`)
//!
//! Re-exports the canonical zero-heap UI layout constraint solver from `etalon_render::ui_canvas`.

pub use etalon_render::ui_canvas::{
    AlignItems, ComputedUiRect, Edges, FlexDirection, HudComputedUiRect, HudUiLayoutTree,
    HudVertex, JustifyContent, Margin, Padding, SizeRule, UiLayoutNode, UiLayoutStyle,
    UiLayoutTree, ZeroHeapUiLayoutTree, MAX_UI_LAYOUT_NODES, MAX_UI_NODES, NO_UI_NODE,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flex_row_grow_and_padding_distributes_exact_widths() {
        let mut tree = ZeroHeapUiLayoutTree::new();
        let root = tree
            .add_node(
                1,
                NO_UI_NODE,
                UiLayoutStyle {
                    direction: FlexDirection::Row,
                    width: SizeRule::Fixed(400.0),
                    height: SizeRule::Fixed(100.0),
                    padding: Padding::all(10.0),
                    gap: 10.0,
                    align_items: AlignItems::Stretch,
                    ..Default::default()
                },
                [0.0, 0.0],
            )
            .unwrap();

        let c0 = tree
            .add_node(
                2,
                root,
                UiLayoutStyle {
                    width: SizeRule::Fixed(80.0),
                    height: SizeRule::Auto,
                    ..Default::default()
                },
                [0.0, 0.0],
            )
            .unwrap();
        let c1 = tree
            .add_node(
                3,
                root,
                UiLayoutStyle {
                    width: SizeRule::FlexGrow(1.0),
                    height: SizeRule::Auto,
                    ..Default::default()
                },
                [0.0, 0.0],
            )
            .unwrap();
        let c2 = tree
            .add_node(
                4,
                root,
                UiLayoutStyle {
                    width: SizeRule::FlexGrow(3.0),
                    height: SizeRule::Auto,
                    ..Default::default()
                },
                [0.0, 0.0],
            )
            .unwrap();

        tree.solve(400.0, 100.0);

        let r0 = tree.nodes[c0 as usize].computed;
        let r1 = tree.nodes[c1 as usize].computed;
        let r2 = tree.nodes[c2 as usize].computed;

        assert!((r0.x - 10.0).abs() < 1e-4);
        assert!((r0.width - 80.0).abs() < 1e-4);
        assert!((r0.height - 80.0).abs() < 1e-4);

        assert!((r1.x - 100.0).abs() < 1e-4);
        assert!((r1.width - 70.0).abs() < 1e-4);

        assert!((r2.x - 180.0).abs() < 1e-4);
        assert!((r2.width - 210.0).abs() < 1e-4);
    }

    #[test]
    fn grid_layout_arranges_2d_columns_and_rows() {
        let mut tree = ZeroHeapUiLayoutTree::new();
        let root = tree
            .add_node(
                10,
                NO_UI_NODE,
                UiLayoutStyle {
                    width: SizeRule::Fixed(310.0),
                    height: SizeRule::Fixed(200.0),
                    grid_columns: 3,
                    column_gap: 5.0,
                    row_gap: 10.0,
                    ..Default::default()
                },
                [0.0, 0.0],
            )
            .unwrap();

        let mut items = [0u16; 6];
        for i in 0..6 {
            items[i] = tree
                .add_node(
                    100 + i as u32,
                    root,
                    UiLayoutStyle {
                        width: SizeRule::FlexGrow(1.0),
                        height: SizeRule::Fixed(40.0),
                        ..Default::default()
                    },
                    [0.0, 0.0],
                )
                .unwrap();
        }

        tree.solve(310.0, 200.0);
        // 3 columns across 310px with 2 * 5px gap -> 100px per cell
        let cell0 = tree.nodes[items[0] as usize].computed;
        let cell1 = tree.nodes[items[1] as usize].computed;
        let cell3 = tree.nodes[items[3] as usize].computed;
        assert!((cell0.width - 100.0).abs() < 1e-4);
        assert!((cell1.x - 105.0).abs() < 1e-4);
        assert!((cell3.x - 0.0).abs() < 1e-4);
        assert!((cell3.y - 50.0).abs() < 1e-4);
    }
}
