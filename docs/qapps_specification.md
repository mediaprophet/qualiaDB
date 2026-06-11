# QApps: Qualia App Specification
_Version: 1.0.0-draft | Target: Webizen Platform_

A **QApp** (Qualia App) is a stateless, declarative user interface shell that binds to the sovereign `qualia-db` engine. Unlike traditional applications that bundle their own backend logic and capabilities, a QApp is fundamentally just a declarative view layout mapped to semantic schemas.

## 1. What is a QApp?

A QApp is entirely defined by a `yaml-ld-q42` file. This manifest describes a `WebizenWorkspace` (or `MindwareWorkspace`) consisting of one or more `Pages`. Each Page contains a `LayoutStrategy` and an array of `PanePlacements`.

A QApp **does not** contain executable logic for things like video transcoding or machine learning. Instead, it relies on the global `ExtensionBus` for capabilities and the Sentinel VM for enforcing fiduciary boundaries.

## 2. Structure of a QApp Manifest

```yaml
---
@context: https://qualia.io/q42
@type: WebizenWorkspace

theme_tokens:
  primary: "#58a6ff"
  bg: "#0d1117"

pages:
  - name: "Health Dashboard"
    url_path: "/health"
    layout_strategy:
      CssGrid:
        cols: 12
        rows: 8
        gap: 16
    panes:
      - component_id: "sensor-data"
        x: 1
        y: 1
        w: 6
        h: 4
        data_bindings: ["did:q42:patient1#heartrate"]
```

## 3. Data Binding & Semantic Linking

Components instantiated by a QApp specify `data_bindings` which act as RDF-Star / LTL pointers. When the Dioxus UI renders a `<qualia-sensor-data>` widget, the Webizen server streams the relevant `NQuin` records matching that binding over the DID-locked WebSocket (`/mobile/stream` or local RPC).

The data binding string is converted to a `q_hash()` inside the engine. The frontend never manages complex local state—it simply reflects the state of the graph.

## 4. Capabilities via the Shared Pool

If a QApp requires specialized functionality (e.g., generating an image, running a local LLM prompt), the UI component emits a capability intent.

1.  **Intent Generation:** The UI sends an intent containing an action and the relevant data path.
2.  **Gatekeeper Check:** The engine evaluates the target data's `SensitivityLabel` against the requested extension. If a `0x02` (Classified) payload is sent to an unauthorized extension, the Sentinel traps and blocks it.
3.  **Execution:** The `ExtensionBus` executes the task recipe using the shared pool in `~/.qualia/extensions/pool/`.

## 5. Deployment Lifecycle

QApps are packaged and distributed as plain text (`yaml-ld-q42` files) or compressed CBO/CBOR-LD-Star payloads.

To install a QApp:
1.  The user drags and drops the manifest into their Studio canvas, or provisions it via WebTorrent.
2.  The engine's `POST /manifest` route parses the YAML.
3.  The engine compiles the definitions into 48-byte `NQuin` primitives and appends them to `qualia_global.wal`.
4.  The frontend automatically rehydrates the UI from the log using Last-Writer-Wins (LWW) CRDT deduplication based on Lamport clocks.
