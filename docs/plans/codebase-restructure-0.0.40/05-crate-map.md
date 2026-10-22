# 05 — Crate map (current → proposed)

Inventory of all **27** workspace members on `0.0.40-dev`, plus logical modules still inside `qualia-core-db`.

Legend:

| Tag | Meaning |
|-----|---------|
| **C** | Core library (target) |
| **M** | Domain module |
| **A** | App / host |
| **T** | Tooling |
| **F** | Facade (temporary god-crate during migration) |
| keep | Stay as-is near-term |
| extract | Split out over phases |
| thin | Shrink responsibility |
| rename | Optional late naming fix |

---

## A. Workspace crates

| Current crate | Today role | Target tier | Action | Notes |
|---------------|------------|-------------|--------|-------|
| `qualia-inference-kernel` | contracts | **C** | keep | Pattern exemplar: dep-free |
| `vibe` | language | **C** | keep | Hosts: poet-cli, lsp, wasm, core-db |
| `qualia-core-db` | everything | **F → thin C+M** | thin + extract | Facade; see §B |
| `qualia-audio` | auditory | **M** | keep | Already extracted; avoid re-absorption |
| `qualia-vision` | visual | **M** | keep | Depends on core-db today — ok |
| `wellfare-core` | welfare/fairness | **M** | keep | Must not depend on core-db |
| `qualia-cooperative-core` | cooperative | **M** | keep | On wellfare-core |
| `qualia-solid-bridge` | Solid | **M** | keep | |
| `webizen-render` | renderer | **M** | keep | Shares GPU device with inference |
| `webizen-runtime` | sim runtime | **M** | keep | |
| `qualia-extensions` | heavy extensions | **M** | keep | Outside zero-heap core |
| `qualia-q-forge` | quantum forge | **M** | keep | `no_std` |
| `qualia-client-core` | app support | **M/A-support** | thin (Ph7) | Second fat surface |
| `webizen-studio` | UI | **A** | keep | Clarify vs Poet ownership |
| `webizen-desktop` | Tauri shell | **A** | keep | |
| `webizen-web` (`qualia-wasm`) | full wasm app | **A** | rename? | Dir/package skew |
| `webizen-lite-wasm` | ontology MCP | **A** | keep | Size-budget critical |
| `qualia-mobile-harness` | companion | **A** | keep | |
| `qualia-cli` | CLI | **A** | keep | |
| `poet` | HyperCanvas | **A** | keep + consume extracted poet-host | |
| `poet-cli` | vibe CLI | **A/T** | keep | Bin name `poet` collision |
| `qualia-peer` | peer facade | **A** | keep | Slim QDNF consumer |
| `vibe-lsp` | LSP | **T** | keep | |
| `vibe-wasm` | vibe in browser | **T/A** | keep | |
| `work-graph` | impl graph indexer | **T** | keep | |
| `webizen-component-harvester` | codegen aid | **T** | keep | |
| `qualia-semantic-library` | corpus→hmc | **T** | keep | Offline; not core inference |

---

## B. Logical modules inside `qualia-core-db` (extract candidates)

| Module path today | Approx size | Target crate / home | Phase | Priority |
|-------------------|------------:|---------------------|------:|----------|
| `specialized_libs/computational_geometry/` | largest family | `qualia-geometry` | 3 | P0 pilot |
| `specialized_libs/computer_vision/` | large | merge toward `qualia-vision` or `qualia-cv-kernels` | 3b | P1 |
| `specialized_libs/*` (other sciences) | medium each | `qualia-science-*` or feature modules | later | P2 |
| `inference/` + `gguf_bridge/` + parts of `wgsl_forge/` | very large | `qualia-inference-runtime` | 4 | P0 |
| `poet_host/` | ~57k lines | `qualia-poet-host` or under `poet` | 5 | P0 |
| `net/` + `p2p/` (esp. `qdnf`) | ~61k+ | `qualia-net` / peer stack | 6 | P1 |
| `sparql_library/` + `query/` | large | stay facade-long; optional `qualia-query` | late | P2 |
| `q42/` | ~23k | `qualia-q42` core | late | P1 (ABI) |
| `modalities/` + `governance/` | large | `qualia-logic` / governance module | late | P2 |
| `crypto/` | ~10k | stay or `qualia-crypto` | late | P2 |
| `services/` (daemon, etc.) | ~12k | app-adjacent module | with CLI/desktop | P2 |
| `mcp/` | ~8k | module used by CLI/desktop | mid | P2 |
| `wasm_*` bridges | medium | stay with profiles until cores exist | — | — |
| `medical/`, `domains/` | smaller | domain modules | late | P3 |
| `render/` inside core-db | overlaps webizen-render | consolidate carefully | mid | P2 |

---

## C. Apps — dependency appetite (target)

Desired steady state (simplified):

```
webizen-desktop
  → webizen-studio
  → qualia-client-core (thinned)
  → wellfare-core, cooperative-core
  → webizen-render, webizen-runtime
  → qualia-core-db (facade) / extracted modules
  → vibe

poet
  → vibe
  → qualia-poet-host (future)
  → (optional) query/render modules — not full specialized_libs by default

webizen-lite-wasm
  → qualia-core-db with wasm-ontology only
  → (no poet_host, no specialized_libs, no client-core)

qualia-cli
  → facade + modules needed for operator tasks
```

---

## D. Tooling & repo furniture (not crates)

| Path | Proposal |
|------|----------|
| `scripts/`, `tools/`, `agent-tools/` | Keep; index from manuals |
| Root `*.mjs` GGUF helpers | Move under `scripts/inference/` when touched |
| `vendor/` | Keep patches; document in README “why vendored” |
| `geometry-showcase/` | Either demo app under `apps/` or archive if redundant |
| `core-ontologies` vs `ontologies` vs `bundled/ontologies` | Declare authority in Phase 1 |
| `docs/work-in-progress/` | Age-out policy: graduate to manuals or delete |
| Root agent mega-docs | Eventually slim; point to manuals |

---

## E. Checklist for “where does my new code go?”

1. Language / AST / bytecode for VibeScript → **`vibe`**
2. Inference **policy/oracle** with no DB → **`qualia-inference-kernel`**
3. Inference **runtime / GGUF** → inference module (today core-db; future runtime crate)
4. Graph quin / q42 volume ABI → core (today core-db `q42`/`query`)
5. Geometry / chemistry / … library → `specialized_libs/<family>` or extracted science crate — **not** studio
6. Wellfare record types → **`wellfare-core`**
7. Cooperative workflow → **`qualia-cooperative-core`**
8. Dioxus screens → **`webizen-studio`**
9. Tauri commands / OS → **`webizen-desktop`**
10. HyperCanvas tools / manifolds → **`poet`** (+ future poet-host)
11. Public ontology site MCP → **`webizen-lite-wasm`**
12. One-off operator script → **`scripts/`** or **`qualia-cli`** subcommand — not a new workspace member without Phase 0 agreement

---

## F. Mapping summary counts (target end-state sketch)

| Tier | Approx crate count (steady) |
|------|----------------------------:|
| Core | 4–7 (vibe, inference-kernel, q42/quin, maybe query/storage) |
| Modules | 12–20 (existing M crates + extracts) |
| Apps | 7–9 |
| Tooling | 4–6 |
| Facade | 0–1 (`qualia-core-db` until thin enough to rename or keep as umbrella) |
