# yaml-ld-q42 Specification

**Status:** Internal draft  
**Updated:** 2026-06-12  
**Purpose:** Freeze the current meaning of `yaml-ld-q42` across the Webizen
workspace, studio renderer, and compiler bridge before any external
standardization attempt.

## 1. Scope

`yaml-ld-q42` is the declarative manifest format used to describe a Webizen
workspace in a human-authorable form.

In the current repo, the format spans two closely related but not identical
shapes:

- a minimal compiler shape in
  [crates/qualia-core-db/src/yaml_ld_q42.rs](/C:/Projects/qualiaDB/crates/qualia-core-db/src/yaml_ld_q42.rs:39)
- a richer studio/runtime shape in
  [crates/webizen-studio/src/studio_canvas.rs](/C:/Projects/qualiaDB/crates/webizen-studio/src/studio_canvas.rs:16)

This document records both and distinguishes between:

- fields accepted by the current compiler bridge
- fields used by the current studio/runtime workspace model
- fields that are semantically intended but not yet fully compiled into Quins

## 2. Relationship to Other Manifests

`yaml-ld-q42` is not the same thing as `qapp.json`.

- `yaml-ld-q42`
  Declares workspace, pages, panes, placement, theming, and presentation
  semantics.
- `qapp.json`
  Declares package metadata, entrypoints, capability requirements, required
  ontologies/models, daemon integration hints, and host launch metadata.

Current code uses:

- `qapp.json` for installed Qapp launch and loopback hosting
- `yaml-ld-q42` for declarative Webizen workspace/layout definitions

## 3. Media Type and Transport

The current system already uses the content type:

```text
application/yaml-ld-q42
```

This appears in the studio/server manifest path and should be treated as the
canonical internal media type until superseded by a formal registration.

## 4. Conceptual Model

At a high level, a `yaml-ld-q42` document describes:

1. a `WebizenWorkspace`
2. containing one or more `Page` definitions
3. each containing one or more pane placements or modules
4. with optional theming and presentation metadata
5. intended to compile into `NQuin` layout/state records and/or render
   directly in the studio runtime

## 5. Minimal Compiler Shape

The current compiler bridge in `qualia-core-db` deserializes this simplified
shape:

```rust
pub struct WebizenWorkspace {
    pub pages: Vec<Page>,
}

pub struct Page {
    pub url_path: String,
    pub name: String,
    pub panes: Vec<Pane>,
}

pub struct Pane {
    pub component_id: String,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}
```

### 5.1 Compiler Behavior

`compile_yaml_ld_to_quins(...)` currently:

- parses YAML via `serde_yaml`
- emits one page-definition Quin per page
- emits one pane-state Quin per pane
- packs `x`, `y`, `w`, and `h` into the lower 32 bits of `metadata`
- stores the Lamport clock in the upper bits of `metadata`

### 5.2 Important Limitation

The compiler does **not** currently persist the richer studio fields such as:

- `layout_strategy`
- `presentation_mode`
- `coordinate_space`
- `pan_and_zoom`
- `data_bindings`
- `binds_rpc`
- `requires_capability`
- `ui_mode`
- `layer`
- `anchor`
- `min_w_points`
- `min_h_points`
- `supported_presentations`
- theme catalog or scoped theme bindings

Those fields belong to the runtime/studio schema today, but are not yet fully
compiled by the `yaml_ld_q42.rs` bridge.

## 6. Current Studio Runtime Shape

The richer workspace model in `webizen-studio` is:

```rust
pub struct WebizenWorkspace {
    pub pages: Vec<Page>,
    pub theme_tokens: HashMap<String, String>,
    pub themes: Vec<ThemeDefinition>,
    pub environment_theme: ThemeBinding,
    pub app_theme: ThemeBinding,
}

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

pub struct PanePlacement {
    pub component_id: String,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub data_bindings: Vec<String>,
    pub binds_rpc: Option<String>,
    pub requires_capability: Vec<String>,
    pub ui_mode: Option<UiMode>,
    pub layer: LayerBehavior,
    pub anchor: Option<String>,
    pub min_w_points: u16,
    pub min_h_points: u16,
    pub supported_presentations: Vec<PresentationMode>,
    pub theme: ThemeBinding,
}
```

## 7. Enumerated Runtime Values

### 7.1 `LayoutStrategy`

```rust
PointGrid {
    width_points: u16,
    height_points: u16,
    snap_step: u16,
    gutter: u16,
}
CssGrid { cols: u8, rows: u8, gap: u8 }
FlexBox
Masonry
```

### 7.2 `PresentationMode`

```rust
GridBound
NodeRelational
Spatial
```

### 7.3 `CoordinateSpace`

```rust
GlobalCartesian
RelativeAnchored
```

### 7.4 `UiMode`

```rust
NativeDioxus
IFrameSandbox
```

### 7.5 `LayerBehavior`

```rust
Docked
FloatingOverlay
ModalOverlay
FullCanvas
```

## 8. Theme Model

The runtime theme system allows bindings at four scopes:

- environment
- app
- page
- module

The relevant structures are `ThemeDefinition` and `ThemeBinding`, with support
for:

- token dictionaries
- optional stylesheet references
- optional class names
- theme preset references by `theme_id`

For details, see
[crates/webizen-studio/THEMING.md](/C:/Projects/qualiaDB/crates/webizen-studio/THEMING.md:1).

## 9. Example Minimal Document

This example is aligned with the currently compiled subset:

```yaml
---
pages:
  - url_path: "/health"
    name: "Health Dashboard"
    panes:
      - component_id: "sensor-data"
        x: 1
        y: 1
        w: 6
        h: 4
      - component_id: "alert-notification"
        x: 8
        y: 1
        w: 4
        h: 2
```

## 10. Example Rich Runtime Document

This example reflects the fuller studio schema:

```yaml
---
theme_tokens:
  qualia-bg: "#0d1117"
  qualia-accent: "#58a6ff"

environment_theme:
  class_name: "theme-root"

app_theme:
  theme_id: "fiduciary-dark"

pages:
  - url_path: "/workspace"
    name: "Human-Centric Workspace"
    layout_strategy:
      PointGrid:
        width_points: 96
        height_points: 64
        snap_step: 2
        gutter: 2
    presentation_mode: GridBound
    coordinate_space: GlobalCartesian
    pan_and_zoom: true
    panes:
      - component_id: "neuro-symbolic-chat"
        x: 4
        y: 6
        w: 28
        h: 18
        data_bindings:
          - "did:q42:user#chat"
        layer: Docked
        supported_presentations:
          - GridBound
          - NodeRelational
      - component_id: "custom-web-module"
        x: 62
        y: 10
        w: 26
        h: 22
        binds_rpc: "qualia://overlay/chat"
        ui_mode: IFrameSandbox
        layer: FloatingOverlay
        anchor: "neuro-symbolic-chat"
```

## 11. Compilation Semantics

The current bridge uses the following Quin mapping:

- page definition Quin
  - `subject = q_hash(page.url_path)`
  - `predicate = q_hash("q42:SystemPageDef")`
  - `object = q_hash(page.name)`
  - `context = namespace`
- pane state Quin
  - `subject = q_hash(pane.component_id)`
  - `predicate = q_hash("q42:SystemPaneState")`
  - `object = q_hash(page.url_path)`
  - `context = namespace`
  - `metadata[0..31] = packed x/y/w/h`

Bounding box packing is currently:

```text
metadata[24..31] = x
metadata[16..23] = y
metadata[ 8..15] = w
metadata[ 0.. 7] = h
```

Lamport clock data is placed in the upper portion of `metadata`.

## 12. Parser Status

The intended architecture is a streaming, low-allocation lexer:

- `YamlStreamingLexer`
- `YamlToken`

However, the current implementation does **not** yet use the streaming lexer in
production. `next_token()` is a placeholder and the compiler currently falls
back to `serde_yaml::from_slice(...)`.

That means:

- the format is conceptually streaming-friendly
- the current implementation is not yet the final zero-allocation parser
- this document should not claim the lexer is fully realized yet

## 13. Canonical Rules for Now

Until the compiler and runtime converge further, the repo should treat these
rules as canonical:

1. `yaml-ld-q42` is the authoritative declarative workspace/layout format.
2. `qapp.json` remains a separate host/package manifest layer.
3. The minimal compiler subset is normative for persisted layout compilation
   today.
4. The richer studio schema is normative for runtime workspace authoring today.
5. Docs must distinguish clearly between fields that render in the studio and
   fields that are currently persisted into Quins.

## 14. Open Questions

Open questions that should be resolved before external standardization:

1. Should `@context` and `@type` be mandatory or optional in all documents?
2. Should `yaml-ld-q42` be purely YAML surface syntax for a CBOR-LD / Quin
   projection, or remain a first-class authoring format on its own?
3. Which rich runtime fields should become mandatory parts of the compiled Quin
   projection?
4. Should pane identifiers remain `component_id` strings, or become explicit
   DID- or ontology-linked identifiers in the canonical model?
5. Should theme bindings compile to Quins directly, or stay as renderer-local
   metadata?
