# Poet: Mindware Workbench — UI Mock-up Strategy
> *"Poet: Define your mindware, with VibeScript"*

**Copyright © 2026 Timothy Charles Holborn.** All rights reserved.  
**Principal / inventor:** Timothy Charles Holborn &lt;timothy.holborn@gmail.com&gt;

The intended outcome is an innovative hypermedia authoring environment (**Poet**) where human authors, co-authors, and mindware agents interact seamlessly with structured knowledge, 3D assets, tabular data, and dynamic scripts.

Prototyping a dynamic mock-up first allows us to explore interaction flows, context markup UX, Aura (ontology) inspectors, and real-time Pulse event streams before finalizing backend Rust/WASM bindings.

---

## Core Constraints & Principles

1. **NO Node.js Dependency:** Zero reliance on `npm`, `package.json`, Node toolchains, or heavy bundlers (Webpack, Vite, etc.).
2. **Zero Recompilation UI Loops:** Avoid slow Rust compilation cycles when refining visual layouts, styles, and interactive widgets.
3. **Native Web Standards:** Built using modern browser-native primitives: HTML5, CSS Custom Properties (Variables), Flexbox/Grid, and standard ES Modules (`<script type="module">`).
4. **Web Components for Domain Primitives:** Encapsulate hypermedia elements as standard Custom Elements (e.g., `<q-doc>`, `<q-entity>`, `<q-relation>`, `<q-event>`, `<q-cell>`, `<q-graph-view>`).
5. **Decoupled Backend Connection:** Communicate with the running QualiaDB / Webizen backend (`webizen_server`, `solid_ldp`, or RPC endpoint) via standard browser `fetch()`, `WebSocket`, or Server-Sent Events (SSE).

---

## Architectural Layout

```
┌────────────────────────────────────────────────────────┐
│             Webizen Desktop / Native Browser           │
│                                                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │   Poet Workbench Mockup (Zero Build / ESM)       │  │
│  │   - index.html (Main Workbench Shell)            │  │
│  │   - styles/ (Theme, Grid, Annotations, Cards)    │  │
│  │   - components/ (Hyperdoc, Aura Tray, Pulse Bar) │  │
│  │   - api.js (Fetch / WebSocket to QualiaDB)       │  │
│  └────────────────────────┬─────────────────────────┘  │
└───────────────────────────┼────────────────────────────┘
                            │ Standard HTTP / WebSocket / SSE
┌───────────────────────────▼────────────────────────────┐
│              QualiaDB / Webizen Daemon                 │
│   - `webizen_server` / `solid_ldp` / `daemon_query`    │
│   - Serves local RDF 1.2 / Q42 graphs, and schemas     │
└────────────────────────────────────────────────────────┘
```

---

## Mock-up File Structure

```text
UI_Mockup/
├── approach.md             # This strategy document
├── designnotes.md          # Visual hierarchy & interaction notes
├── index.html              # Main Poet workbench shell
├── styles/
│   ├── theme.css           # Color tokens, typography, dark mode, elevation
│   ├── layout.css          # Split-pane layout, ribbon, sidebars, status bar
│   └── components.css      # Context markup badges, entity cards, popovers
├── components/
│   ├── hyperdoc-editor.js  # Rich text & context markup annotation engine
│   ├── aura-tray.js        # Ontological shape & schema inspector (Aura)
│   ├── pulse-panel.js      # Collaborative stream & telemetry monitor (Pulse)
│   ├── graph-panel.js      # Interactive RDF / knowledge graph preview
│   └── ontology-picker.js  # Personal ontology selector & tagger
├── state.js                # Lightweight vanilla reactive state store
└── api.js                  # Communication layer with QualiaDB backend
```

---

## Serving & Previewing the Mock-up

Since no Node.js is used, the mock-up can be previewed instantly via any lightweight mechanism:
- **Option A (QualiaDB Native):** Serve static assets directly through QualiaDB's built-in `webizen_server`.
- **Option B (Python Standard Library):** `python -m http.server 8080` (requires no packages).
- **Option C (Rust Static Binary):** A minimal local static server (e.g. `miniserve` or `basic-http-server`).
- **Option D (Direct WebView):** Load directly via `file://` inside Webizen Desktop's webview container.

---

## Evolution to Production (Rust / WASM)

1. **Phase 1 (Visual & Interaction Mock-up):** Dial in the UX for context markup, entity tagging, split-pane navigation, and hypermedia asset embedding using HTML/CSS/JS.
2. **Phase 2 (Protocol & API Alignment):** Connect the UI to live QualiaDB endpoints to validate data contracts (RDF 1.2, CBOR-LD, Solid LDP).
3. **Phase 3 (Progressive Rust/WASM Core):** Move intensive data processing, symbolic NLP parsers, and the Poet Engine (VibeScript runtime) into Rust crates compiled via `wasm-bindgen`, keeping the UI layer decoupled and responsive.
4. **Phase 4 (VibeScript-Driven Dynamic UI & Live Reconciler):** The compiled WASM host embeds the VibeScript 0.1 AST/Bytecode interpreter, allowing UI furniture (Aura Tray, collapsible subtrays, diagnostics, metrics, action buttons) to be declared in `.vibe` scripts. Editing or hot-reloading scripts live in the browser `<q-vibe-ui>` reconciles the DOM instantly without recompiling the WASM binary.

