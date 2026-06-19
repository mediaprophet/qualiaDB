# Webizen Studio Page Geometry

Webizen Studio pages now support a geometry-first schema instead of assuming every workspace is a column layout.

## Core Ideas

- `LayoutStrategy::PointGrid` treats the page as a coordinate canvas with `x`, `y`, `w`, and `h` measured in points.
- `PresentationMode` declares the intended renderer mode for a page:
  - `GridBound`
  - `NodeRelational`
  - `Spatial`
- `LayerBehavior` lets a pane choose how it appears inside the page:
  - `Docked`
  - `FloatingOverlay`
  - `ModalOverlay`
  - `FullCanvas`
- `CoordinateSpace` distinguishes between globally placed panes and panes that should later be anchored relative to other nodes or objects.

## Page Shape

```rust
pub struct Page {
    pub url_path: String,
    pub name: String,
    pub layout_strategy: LayoutStrategy,
    pub panes: Vec<PanePlacement>,
    pub presentation_mode: PresentationMode,
    pub coordinate_space: CoordinateSpace,
    pub pan_and_zoom: bool,
    pub theme: ThemeBinding,
}
```

## Pane Shape

```rust
pub struct PanePlacement {
    pub component_id: String,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub layer: LayerBehavior,
    pub anchor: Option<String>,
    pub min_w_points: u16,
    pub min_h_points: u16,
    pub supported_presentations: Vec<PresentationMode>,
    pub theme: ThemeBinding,
}
```

## Example Manifest Fragment

```json
{
  "url_path": "/workspace",
  "name": "Human-Centric Workspace",
  "layout_strategy": {
    "PointGrid": {
      "width_points": 96,
      "height_points": 64,
      "snap_step": 2,
      "gutter": 2
    }
  },
  "presentation_mode": "GridBound",
  "coordinate_space": "GlobalCartesian",
  "pan_and_zoom": true,
  "panes": [
    {
      "component_id": "address-book",
      "x": 4,
      "y": 6,
      "w": 28,
      "h": 18,
      "layer": "Docked",
      "min_w_points": 20,
      "min_h_points": 14,
      "supported_presentations": ["GridBound", "NodeRelational"]
    },
    {
      "component_id": "chat-app",
      "x": 62,
      "y": 10,
      "w": 26,
      "h": 22,
      "layer": "FloatingOverlay",
      "anchor": "address-book",
      "min_w_points": 18,
      "min_h_points": 16,
      "supported_presentations": ["GridBound", "NodeRelational", "Spatial"]
    },
    {
      "component_id": "scene-viewer",
      "x": 0,
      "y": 0,
      "w": 96,
      "h": 64,
      "layer": "FullCanvas",
      "supported_presentations": ["Spatial"]
    }
  ]
}
```

## Current Renderer Behavior

- Point-grid pages render as a bounded coordinate plane.
- Docked panes are placed directly on the base canvas.
- Floating, modal, and full-canvas panes render in a separate interaction layer.
- `NodeRelational` and `Spatial` pages are already part of the schema, but today they still fall back to the point-grid adapter until dedicated renderers land.

## Why This Helps

- Responsive behavior can be based on available point-area rather than only viewport breakpoints.
- Chat, notifications, and similar Qapps can be layered without pretending they are just narrow columns.
- 3D or immersive Qapps can declare `FullCanvas` without forcing every other Qapp into the same presentation model.
- The studio can evolve into a renderer/adapter system instead of binding app logic to one layout convention.
