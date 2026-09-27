# 03 — Target architecture

Civics framing: citizen-led public-good infrastructure for the **natural person**.  
Preserve Continuity (**handle ≠ who**), fiduciary-to-principal duty, and “agents are sub-agents of humans”. Do not invent new legal/policy text here.

---

## 1. Goals

- Make the codebase **manageable**: a new contributor can name the layer they are in within five minutes.
- Prefer **incremental extracts** over a big-bang monorepo rewrite.
- Keep **WASM size budgets** and native GPU paths working every phase.
- Encode **layering rules** so AI agents stop putting app code in core and core ABI in studio.

---

## 2. Three tiers (+ tooling)

```
┌─────────────────────────────────────────────────────────┐
│  APPS / HOSTS                                           │
│  desktop · studio · cli · poet · lite-wasm · web ·      │
│  mobile-harness · peer facade                           │
├─────────────────────────────────────────────────────────┤
│  DOMAIN MODULES                                         │
│  query · modalities/governance · inference runtime ·    │
│  specialized sciences · net/qdnf · crypto · wellfare ·  │
│  cooperative · vision · audio · solid · render/runtime  │
├─────────────────────────────────────────────────────────┤
│  CORE LIBRARIES                                         │
│  vibe · inference-kernel · q42/graph/storage ABI ·      │
│  (thin) quin / sentinel contracts                       │
└─────────────────────────────────────────────────────────┘
         ▲
         │  TOOLING (may depend downward only)
         │  work-graph · harvester · semantic-library ·
         │  vibe-lsp · vibe-wasm · scripts · benches
```

### Core libraries

**Definition:** Small dependency closure; stable ABI; testable without Tauri/Dioxus/Poet; safe to link into lite WASM.

**Today already close:**

| Crate | Keep as core |
|-------|----------------|
| `vibe` | Language kernel |
| `qualia-inference-kernel` | Inference contracts/oracles |

**Extract-toward-core (from `qualia-core-db`, over phases):**

| Future / logical core | Source today |
|-----------------------|--------------|
| `qualia-quin` / graph record ABI | NQuin / frame layout / q_hash paths |
| `qualia-q42` | `src/q42/`, volume/header readers |
| `qualia-storage` | wal, mmap volume drivers (native-gated) |
| `qualia-query-core` | query engine + sparql_library (feature-split for wasm) |

Until extracted, document these as **logical cores inside core-db** with module boundaries and “no upward deps” discipline.

### Domain modules

**Definition:** Optional product capability; may depend on core; must not depend on apps; ideally behind features or their own crates.

| Module family | Today | Target |
|---------------|-------|--------|
| Wellfare / cooperative | `wellfare-core`, `qualia-cooperative-core` | Keep crates; thin core-db bindings only |
| Audio / vision | `qualia-audio`, `qualia-vision` | Keep; reduce core-db re-embedding |
| Solid | `qualia-solid-bridge` | Keep as bridge module |
| Render / sim runtime | `webizen-render`, `webizen-runtime` | Keep |
| Extensions / QPU | `qualia-extensions`, `qualia-q-forge` | Keep outside zero-heap core |
| Specialized sciences | `core-db/specialized_libs/*` | **Extract per family** (geometry first candidate) |
| Inference runtime | `inference/`, `gguf_bridge/`, `wgsl_forge/` | Module crate(s) depending on inference-kernel |
| Net / QDNF | `net/`, `p2p/`, `qualia-peer` | Module + thin peer app facade |
| Poet host invoke | `poet_host/` | Module consumed by poet/desktop — **not** core ABI |
| Modalities / governance / identity / crypto | respective dirs | Domain modules; wasm-gated |

### Apps / hosts

| App | Responsibility |
|-----|----------------|
| `webizen-desktop` | Native shell, OS integration, updater, tray, daemon spawn |
| `webizen-studio` | Shared Dioxus UI (web + embedded in desktop) |
| `poet` | HyperCanvas / tool-chest living document host |
| `qualia-cli` | Operator / science / bench / MCP CLI |
| `webizen-lite-wasm` | Ontology MCP for public ontology sites |
| `webizen-web` (`qualia-wasm`) | Full browser engine package |
| `qualia-mobile-harness` | Companion / mobile WellFair |
| `qualia-peer` | Peer runtime facade over QDNF |

**Product rule (docs, then code):** **One Poet, two hosts** (desktop + wasm studio paths) — align READMEs so studio/poet ownership is explicit before more UI surface lands.

---

## 3. Layering rules (normative for future PRs)

1. **Apps → modules → core** only. No core → app edges. No module → app edges.
2. **Core** may not depend on wgpu UI shells, Tauri, Dioxus, or Poet browser registration.
3. **WASM lite** (`wasm-ontology`) links **core + minimal query/ontology only** — never specialized_libs, poet_host, or desktop chat stacks.
4. **Feature flags** describe *capability profiles*, not catch-all “full product in one crate forever”. New large domains prefer a crate or a clearly named module with its own Cargo feature.
5. **Re-exports** during migration are OK (`pub use new_crate::…` from core-db) — same pattern as `MODULE_REORG_PLAN.md`.
6. **Naming:** prefer `qualia-*` for engine/modules, `webizen-*` for citizen-facing hosts/UI, bare `vibe`/`poet` for language + canvas. Fix `webizen-web`/`qualia-wasm` skew when touching that crate.
7. **Human constraints** are product invariants, not a layer — every tier must honour Continuity / fiduciary / sub-agent rules where identity or agency appears.

See also [adr/0001-layering-and-crate-roles.md](./adr/0001-layering-and-crate-roles.md).

---

## 4. What stays in `qualia-core-db` near-term

Treat core-db as a **facade workspace crate** during migration:

- Continues to be the single dependency most apps use.
- Internally organises toward extractable modules.
- Shrinks by **moving implementation out**, leaving `pub use` or thin wrappers.

Do **not** rename the package early — dependents cost is high. Rename only after extracts stabilize (optional late phase).

---

## 5. Docs architecture (companion to code layers)

| Published | Private (gitignore) |
|-----------|---------------------|
| `docs/manuals/`, `docs/manuals/adr/`, `docs/design/`, this pack under `docs/plans/codebase-restructure-*` (force-added) | Other `docs/plans/*` progress logs |
| Slim root: README + one agent entry | Move long agent prose toward `docs/manuals/agents/` over time |

**Missing plan targets:** either publish summarised manuals for ADR-cited programmes, or retarget links — do not leave permanent 404s for accepted ADRs.

---

## 6. Success metrics (lightweight)

- Contributor quiz: name the tier of `poet_host`, `vibe`, `webizen-desktop` correctly.
- `webizen-lite-wasm` size budget still green after each extract.
- `cargo check -p qualia-inference-kernel` and `-p vibe` remain dep-light.
- Number of `.rs` files &gt; 100 KB declines over phases (split or generate-into-dirs).
- New domain work lands in a module crate **or** a named core-db subdirectory with a feature — not as new loose root files.
