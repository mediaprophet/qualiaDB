# QApps: Qualia App Specification
_Version: 1.0.0-draft | Target: Webizen Platform_

A **QApp** (Qualia App) is a stateless, declarative user interface shell that binds to the sovereign `qualia-db` engine. Unlike traditional applications that bundle their own backend logic and capabilities, a QApp is fundamentally just a declarative view layout mapped to semantic schemas.

## 1. What is a QApp?

A QApp is entirely defined by a `yaml-ld-q42` file. This manifest describes a `WebizenWorkspace` consisting of one or more `Pages`. Each Page contains a `LayoutStrategy` and an array of `PanePlacements`.

A QApp **does not** contain executable logic for things like video transcoding or machine learning. Instead, it relies on the global `ExtensionBus` for capabilities and the Sentinel VM for enforcing fiduciary boundaries.

## 2. Structure of a QApp Manifest

```yaml
---
@context: https://webizen.org/q42
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

1. **Intent Generation:** The UI sends an intent containing an action and the relevant data path.
2. **Gatekeeper Check:** The engine evaluates the target data's `SensitivityLabel` against the requested extension. If a `0x02` (Classified) payload is sent to an unauthorized extension, the Sentinel traps and blocks it.
3. **Execution:** The `ExtensionBus` executes the task recipe using the shared pool in `~/.qualia/extensions/pool/`.

## 5. Deployment Lifecycle

QApps are packaged and distributed as plain text (`yaml-ld-q42` files) or compressed CBO/CBOR-LD-Star payloads.

To install a QApp:
1. The user drags and drops the manifest into their Studio canvas, or provisions it via WebTorrent.
2. The engine's `POST /manifest` route parses the YAML.
3. The engine compiles the definitions into 48-byte `NQuin` primitives and appends them to `qualia_global.wal`.
4. The frontend automatically rehydrates the UI from the log using Last-Writer-Wins (LWW) CRDT deduplication based on Lamport clocks.

## 6. Manifold Worlds (`ns/ui` upgrade — Phase 5)

A QApp is no longer only a grid of 2D panes. A **manifold world** declares one or more **views over a single manifold** — a 3D scene *and* a 2D pane drawn from the *same* source — so the same data is shown two ways (the renderer's `Volume3D` and `Plane2D` projections; see `render::projection`). The runtime that resolves these views is `render::authoring` (`plan_qapp` → a `ViewDisposition` per view). The wire-form (`yaml-ld-q42` → CBOR-LD → NQuin `@context` expansion) is task #8; **ShEx *describes*** the view contract and **SHACL *enforces*** the shape (ADR 0009) — one source.

A view (`QappView`) carries three engine-enforced annotations, resolved **before anything is drawn**:

| Annotation | Vocabulary | Engine behaviour |
|---|---|---|
| **Manifold source** | `manifold` (id) | The same id across views = "one manifold, many views". |
| **Sensitivity** | `Public` \| `RightsBounded` | A `RightsBounded` view is **refused** in a *shared/civic* standpoint without consent (the inherited `logic::deontic` gate; **fails closed**). The owner's *private* view always renders. |
| **Attestation gate** | `requires_attestation` | The view is **withheld** until an attestation `(attester) attests (manifold)` is present — the human ratifies by signing (wisdom-out-of-band). Signature verification stays in the identity/key-vault layer. |
| **Budget** | device tier | On a constrained tier (`OperationalMode::Eco`/`Reserve`) a `Scene3D` view **degrades to 2D** (`Collapsed2D`) rather than failing — affordability by construction. |

```yaml
# a manifold-world page (illustrative; wire-form is task #8)
pages:
  - name: "Patient Manifold"
    views:
      - manifold: "did:q42:patient1#vitals"
        kind: Scene3D
        sensitivity: RightsBounded     # refused in a shared/civic view without consent
      - manifold: "did:q42:patient1#vitals"   # SAME manifold → the 2D shadow
        kind: Pane2D
        sensitivity: RightsBounded
        requires_attestation: true     # withheld until a clinician attests
```

Resolution order is **attestation → rights-bounded → budget**: a governance refusal (withheld / refused) takes precedence over budget degradation. The decision is `ViewDisposition`: `Render(kind)`, `Collapsed2D`, `WithheldUnattested`, or `RefusedRightsBounded`. Governance primitives here are the §8 substrate *surfaced* — the QApp author selects them; it does not author new normative rules.
