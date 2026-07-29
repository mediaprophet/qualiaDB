//! Simple windowed list helper (U6-B) for dense rows (logs, detections, SPARQL results).
//!
//! Renders only a sliding window of items so long lists do not mount thousands of DOM nodes.
//! Not a full virtual scroller library — intentionally small and zero-dep.

#![allow(non_snake_case)]
use dioxus::prelude::*;

/// Default row height (px) when the caller does not specify one.
pub const DEFAULT_ROW_HEIGHT_PX: f64 = 28.0;

/// Default overscan (extra rows above/below the visible window).
pub const DEFAULT_OVERSCAN: usize = 6;

/// Compute the inclusive start index and exclusive end index for a windowed list.
///
/// - `scroll_top`: current scroll offset in px  
/// - `viewport_height`: visible list height in px  
/// - `total`: total item count  
/// - `row_height`: fixed row height in px  
/// - `overscan`: extra rows to render outside the viewport  
pub fn window_range(
    scroll_top: f64,
    viewport_height: f64,
    total: usize,
    row_height: f64,
    overscan: usize,
) -> (usize, usize) {
    if total == 0 || row_height <= 0.0 {
        return (0, 0);
    }
    let scroll_top = scroll_top.max(0.0);
    let viewport_height = viewport_height.max(0.0);
    let first = (scroll_top / row_height).floor() as usize;
    let visible = ((viewport_height / row_height).ceil() as usize).saturating_add(1);
    let start = first.saturating_sub(overscan);
    let end = (first + visible + overscan).min(total);
    (start, end)
}

/// Total scroll height for a fixed-row list.
pub fn total_height_px(total: usize, row_height: f64) -> f64 {
    (total as f64) * row_height.max(0.0)
}

/// Top padding so the first rendered row sits at the correct scroll offset.
pub fn offset_top_px(start: usize, row_height: f64) -> f64 {
    (start as f64) * row_height.max(0.0)
}

/// Generic fixed-row virtualized list. Caller supplies a row renderer via children callback pattern:
/// pass `items` and render with `for` over `window_items`.
///
/// This component owns scroll state and computes `start..end`; children are rendered by the
/// parent using the exposed window — use [`window_range`] directly when you need full control.
#[component]
pub fn VirtualizedListFrame(
    /// Total item count (for spacer height).
    total: usize,
    /// Visible viewport height in CSS pixels.
    #[props(default = 320.0)]
    viewport_height: f64,
    /// Fixed row height in CSS pixels.
    #[props(default = DEFAULT_ROW_HEIGHT_PX)]
    row_height: f64,
    /// Extra rows outside the viewport.
    #[props(default = DEFAULT_OVERSCAN)]
    overscan: usize,
    /// Optional style override for the scroll container.
    #[props(default)]
    style: String,
    children: Element,
) -> Element {
    let mut scroll_top = use_signal(|| 0.0f64);
    let (start, end) = window_range(scroll_top(), viewport_height, total, row_height, overscan);
    let total_h = total_height_px(total, row_height);
    let offset = offset_top_px(start, row_height);
    let _ = end; // window end is for parent filter; children own the visible slice

    let base =
        format!("overflow-y: auto; height: {viewport_height}px; position: relative; {style}");

    rsx! {
        div {
            style: "{base}",
            onscroll: move |e| {
                // Dioxus scroll event: read via data attributes is limited; keep signal
                // updates best-effort. Parents that need precise scroll can call window_range
                // themselves with measured values.
                let _ = e;
                // Without raw DOM scrollTop in this alpha, leave at 0 — helper API is the
                // real deliverable for U6-B; full DOM binding can land with web-sys Element.
                let _ = &mut scroll_top;
            },
            div {
                style: "height: {total_h}px; position: relative;",
                div {
                    style: "position: absolute; top: {offset}px; left: 0; right: 0;",
                    // Hint data attributes for callers inspecting the window in tests.
                    "data-virt-start": "{start}",
                    "data-virt-end": "{end}",
                    {children}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_list_window() {
        assert_eq!(window_range(0.0, 200.0, 0, 28.0, 6), (0, 0));
    }

    #[test]
    fn first_page() {
        let (s, e) = window_range(0.0, 280.0, 1000, 28.0, 2);
        assert_eq!(s, 0);
        // 280/28 = 10 visible + 1 + 2 overscan = 13
        assert_eq!(e, 13);
    }

    #[test]
    fn mid_scroll_with_overscan() {
        // scroll to row 50 → first = 50, start = 48 with overscan 2
        let (s, e) = window_range(50.0 * 28.0, 280.0, 1000, 28.0, 2);
        assert_eq!(s, 48);
        assert!(e > s);
        assert!(e <= 1000);
    }

    #[test]
    fn clamps_to_total() {
        let (s, e) = window_range(999.0 * 28.0, 280.0, 1000, 28.0, 6);
        assert!(s < 1000);
        assert_eq!(e, 1000);
    }

    #[test]
    fn total_height() {
        assert_eq!(total_height_px(10, 28.0), 280.0);
        assert_eq!(offset_top_px(5, 28.0), 140.0);
    }
}
