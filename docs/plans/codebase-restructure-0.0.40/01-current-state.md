# 01 — Current state (factual map)

Surveyed from `origin/0.0.40-dev` @ `1cdaa63219bcf679b827acc95bbc3e27a3351c15`  
(~4.7k `.rs` files under `crates/`; ~45 MB of Rust source).

---

## 1. Workspace members (27)

From root `Cargo.toml` `members = [...]` (resolver `"2"`):

| # | Path | Package name (if different) |
|--:|------|-------------------------------|
| 1 | `crates/qualia-inference-kernel` | |
| 2 | `crates/qualia-core-db` | |
| 3 | `crates/vibe` | |
| 4 | `crates/qualia-cli` | |
| 5 | `crates/qualia-solid-bridge` | |
| 6 | `crates/qualia-client-core` | |
| 7 | `crates/qualia-extensions` | |
| 8 | `crates/webizen-component-harvester` | |
| 9 | `crates/webizen-render` | |
| 10 | `crates/webizen-studio` | |
| 11 | `crates/webizen-runtime` | |
| 12 | `crates/webizen-desktop` | |
| 13 | `crates/webizen-web` | **`qualia-wasm`** (dir ≠ package name) |
| 14 | `crates/qualia-mobile-harness` | |
| 15 | `crates/wellfare-core` | |
| 16 | `crates/qualia-cooperative-core` | |
| 17 | `crates/qualia-semantic-library` | |
| 18 | `crates/webizen-lite-wasm` | |
| 19 | `crates/qualia-q-forge` | |
| 20 | `crates/qualia-vision` | |
| 21 | `crates/qualia-audio` | |
| 22 | `crates/poet-cli` | bin `poet` |
| 23 | `crates/work-graph` | |
| 24 | `crates/vibe-lsp` | |
| 25 | `crates/vibe-wasm` | |
| 26 | `crates/poet` | also bin `poet-ui` |
| 27 | `crates/qualia-peer` | |

Workspace also defines `[profile.wasm-release]` (LTO on, for mobile-harness) and `[profile.web-release]` (LTO off, for Dioxus studio) with comments documenting wasm-opt / rust-lld failure modes. Large `[patch.crates-io]` pins Dioxus to a git rev and vendors `imap-proto`.

Edition skew: most crates `2021`; `wellfare-core`, `webizen-component-harvester`, `qualia-q-forge` use `2024`. Declared crate versions are mostly `0.0.39` while the branch name is `0.0.40-dev`.

---

## 2. Crate purposes (one-line + role)

| Crate | Role | Purpose (from Cargo.toml / lib.rs) |
|-------|------|-------------------------------------|
| `qualia-inference-kernel` | **core lib** | Lean, dep-free inference contracts & correctness oracles |
| `vibe` | **core lib** | VibeScript 0.1 lexer/parser/checker/interpreter (no JIT, no wgpu) |
| `qualia-core-db` | **fat core** | Semantic graph, inference, ontology, spatial, WASM runtime — *also hosts most domain modules today* |
| `qualia-audio` | **domain module** | Native auditory intelligence (AudioView ABI); no path deps |
| `qualia-vision` | **domain module** | Visual intelligence ABI; depends on core-db |
| `qualia-inference` path via gguf inside core-db | **domain** | GGUF/P64 native + WASM decode (not a separate crate) |
| `wellfare-core` | **domain module** | Welfare/fairness records, RDF/SPARQL via oxigraph, companion WIT |
| `qualia-cooperative-core` | **domain module** | Cooperative domain on top of wellfare-core |
| `qualia-solid-bridge` | **module / bridge** | W3C Solid personal pod + consumer agent |
| `qualia-client-core` | **app support lib** | Desktop/studio shared business + chat/API surface |
| `webizen-render` | **module** | wgpu 30 N-D renderer (optional qualia/vibe features) |
| `webizen-runtime` | **module** | Fixed-timestep GPU diffusion / ledger runtime |
| `webizen-studio` | **app (UI lib+wasm)** | Dioxus UI; native lib for desktop + wasm web target |
| `webizen-desktop` | **app** | Tauri 2 shell hosting studio + client-core + daemon bridges |
| `webizen-web` / `qualia-wasm` | **app (wasm)** | Combined qualia engine + web graphics glue (`wasm-full`) |
| `webizen-lite-wasm` | **app (wasm)** | Ontology-site MCP JSON-RPC (`wasm-ontology` only) |
| `qualia-mobile-harness` | **app (wasm)** | WellFair companion / mobile Dioxus harness |
| `qualia-cli` | **app** | Operator CLI (ingest, query, MCP, benches, daemon helpers) |
| `poet` | **app (wasm+rlib)** | Poet HyperCanvas — spatial hypermedia / tool-chest browser |
| `poet-cli` | **app** | VibeScript REPL/format/lint/eval CLI (bin name `poet`) |
| `vibe-lsp` / `vibe-wasm` | **tooling** | Language server + browser bindings for Vibe |
| `qualia-extensions` | **module** | Heavy extensions (QPU/PINN/SNN/fluid) outside zero-alloc core |
| `qualia-q-forge` | **module** | Quantum forge (`no_std` qasm/sim) |
| `qualia-peer` | **app facade** | QDNF peer runtime facade (no libp2p) |
| `qualia-semantic-library` | **tooling** | Offline `.hmc` corpus → library toolchain |
| `webizen-component-harvester` | **tooling** | Harvests Dioxus component stubs |
| `work-graph` | **tooling** | Implementation-graph presence indexer |

---

## 3. Dependency edges (workspace path deps only)

Most depended-on:

| Crate | # dependents | Dependents |
|-------|-------------:|------------|
| `qualia-core-db` | 13 | cli, client-core, extensions, mobile-harness, peer, q-forge, solid-bridge, vision, desktop, lite-wasm, studio, web |
| `vibe` | 8 | poet, poet-cli, client-core, core-db, vibe-lsp, vibe-wasm, desktop, wellfare-core |
| `webizen-render` | 4 | cli, client-core, desktop, studio |
| `wellfare-core` | 4 | client-core, cooperative-core, mobile-harness, desktop |
| `qualia-client-core` | 3 | cli, desktop, studio |

Notable edges:

- `qualia-core-db` → `vibe`, `qualia-inference-kernel`, `qualia-cooperative-core`, `qualia-audio`
- `qualia-client-core` → core-db + vibe + vision + audio + solid-bridge + wellfare + cooperative + webizen-render
- `webizen-desktop` → almost everything above + studio + runtime + semantic-library
- Cycle guard already documented: `wellfare-core` must **not** depend on core-db (cooperative-core → wellfare → core-db would cycle once core-db binds cooperative)

`webizen-render` lists optional path deps on core-db/vibe via features (parser may under-count); treat as soft edge.

---

## 4. Apps vs libs (practical)

**Ship / host binaries & cdylibs**

- `webizen-desktop` (Tauri)
- `webizen-studio` (Dioxus web + native lib)
- `webizen-web` (`qualia-wasm` cdylib)
- `webizen-lite-wasm` (cdylib)
- `qualia-mobile-harness`
- `qualia-cli`
- `poet` / `poet-ui`, `poet-cli` (bin `poet` — **name collision risk** with poet package)
- `vibe` bin, `vibe-lsp`, `work-graph` bin

**Libraries consumed by hosts**

- Everything else; `qualia-core-db` is both library and the bulk of product logic.

---

## 5. Line-count hotspots

### Per-crate (approx. `.rs` lines)

| Lines | Files | Crate |
|------:|------:|-------|
| 723 084 | 2377 | `qualia-core-db` |
| 182 858 | 694 | `poet` |
| 84 958 | 578 | `webizen-studio` |
| 79 297 | 238 | `qualia-client-core` |
| 54 713 | 137 | `vibe` |
| 33 102 | 252 | `qualia-audio` |
| 22 214 | 88 | `webizen-desktop` |
| 18 202 | 49 | `wellfare-core` |
| 17 735 | 56 | `qualia-cli` |
| … | … | remaining crates each &lt; 15k |

### Inside `qualia-core-db/src/` (top dirs)

| Lines | Dir |
|------:|-----|
| ~190k | `specialized_libs/` (geometry alone ~3.2 MB on disk) |
| ~66k | `inference/` |
| ~61k | `net/` |
| ~57k | `poet_host/` |
| ~36k | `solvers/` |
| ~32k | `modalities/` |
| ~30k | `wgsl_forge/` |
| ~29k | `sparql_library/` |
| ~23k | `q42/` |
| ~22k | `gguf_bridge/` |

### Largest individual `.rs` files (bytes)

| ~Bytes | Path |
|------:|------|
| 211k | `crates/poet/src/browser/tool_copy.rs` |
| 209k | `crates/poet/src/browser/econ_chain_actions.rs` |
| 193k | `crates/poet/src/browser/tool_actions.rs` |
| 179k | `crates/poet/src/browser/stats_chain_actions.rs` |
| 175k | `crates/poet/src/browser/registration/mod.rs` |
| 141k | `crates/qualia-core-db/src/poet_host/invoke/ids.rs` |
| 133k | `crates/webizen-studio/src/components/wellfair/library_panel.rs` |
| 126k | `crates/qualia-cli/src/llm_testing.rs` |
| 124k | `crates/webizen-studio/src/studio_canvas.rs` |

`DECOMPOSITION_PLAN.md` already split several 2017-era studio/desktop/client monoliths; many new hotspots grew after that.

---

## 6. Docs layout

| Location | Character |
|----------|-----------|
| Root `README.md`, `AGENTS.md` (~65k), `AI_INSTRUCTIONS.md`, `CLAUDE.md`, `MASTER_INDEX.md` | Agent + human entry; overlapping coordination |
| Root plan leftovers | `DECOMPOSITION_PLAN.md`, `NATIVE_INFERENCE_OPTIMIZATION_PLAN.md`, `native-inference-p64-pipeline-remediation.md`, `qwen2_*` |
| `docs/` | GitHub Pages site + manuals + ADRs + `work-in-progress/` (~138 WIP files) + showcases |
| `docs/manuals/adr/` | Accepted ADRs (human-centric consent, quin ABI, etc.) |
| `docs/plans/` | **Gitignored** private working plans; many durable docs *link into* missing files here |
| `docs/work-in-progress/` | Swarm UAT, Continuity, human-surface audits |
| Per-crate `DIRECTORY_INDEX.md` | Auto/index trees; `MASTER_INDEX.md` aggregates |

`docs/plans/` **did not exist as a tracked tree** before this pack; references to it are widespread (audio architecture, ADR 0011 realisations, executable-operator trackers, etc.).

---

## 7. Feature flags & wasm vs native

`qualia-core-db` features (high level):

- **Default (native-ish):** `profile_target_1024`, `zk-culling`, `gpu-runtime`, `wgsl-forge`, `privacy-he`
- **WASM profiles (exclusive / composed):** `wasm-ontology` (lite, exclusive), `wasm-logic`, `wasm-scientific`, `wasm-llm`, `wasm-playground`, `portal`, `wasm-full`
- **compile_error** if `wasm-ontology` combined with heavier wasm profiles
- Optional: `nvml`, `vision-onnx`, `sanctuary-crypto`, `qdnf`, `libp2p-compat`, …

CI (`.github/workflows/pages.yml`, `release-wasm.yml`, `benchmarks.yml`) builds multiple feature matrices with size budgets (e.g. lite-wasm raw 640 KiB / gzip 200 KiB; portal sanity caps). Root comments document:

- wasm-opt Windows stack overflow → `profile.wasm-release`
- Dioxus fat LTO bitcode failure → `profile.web-release` must keep LTO off

`webizen-studio` and `webizen-lite-wasm` use `cfg(target_arch = "wasm32")` dependency splits extensively.

---

## 8. Historical accretion (root & parallel trees)

| Artefact | Note |
|----------|------|
| Root `*.mjs` (`check-gguf-*.mjs`, `test-model.mjs`, …) | Ad-hoc inference/WASM debug scripts |
| `old_lib.rs`, `stash-wip.patch`, `_cfd_err.txt` | Leftover scratch at repo root |
| `vendor/` (~64M) | `imap-proto`, dxc, directml, vision, cloudflare-worker |
| `geometry-showcase/` | Standalone showcase alongside core-db geometry |
| Ontology folders | `core-ontologies/` (~11M), `ontologies/`, `bundled/ontologies/`, `crates/poet/ontologies` — parallel corpora |
| `bundled/` | qapps, models, trust, grounding |
| `benchmarks/`, `benches/`, `artifacts/`, `reports/` | Comparative engines + results |
| `bootstrap_gateway/`, `agent-tools/`, `skills/`, `tools/`, `scripts/` | Operator / agent support |
| `package.json` + `package-lock.json` | Node side for docs/benchmarks |
| `wrangler.jsonc` | Cloudflare worker config |

---

## 9. Continuity / human constraints already in-tree (preserve)

Do **not** invent new policy in the restructure. Existing anchors:

- README: fiduciary obligation to the **natural person** who owns the hardware.
- `AI_INSTRUCTIONS.md`: local LLM/Webizen agents are **sub-agents of human principals**, not independent peers.
- ADR 0011: human-centric consent, accountability, post-incapacity disposition.
- WIP Continuity gate: **handle revoke ≠ who-erase**; Continuity handle ≠ human.
- Product locks in human-surface WIPs: human-alone paths, held/not-yet vocabulary, Directory humans-first.

Civics framing for this pack = citizen-led public-good / WebCivics technical work — not a government-portal tone.

---

## 10. Prior reorg work already done / in flight

- **0.0.17** `DECOMPOSITION_PLAN.md`: file splits in studio/desktop/client-core — completed for listed targets.
- **`MODULE_REORG_PLAN.md`**: category folders under core-db (`services/`, `query/`, `inference/`, …) with `pub use` re-exports — partially realised (lib.rs still huge ~1900 lines of gated re-exports).
- **Inference kernel extract**: `qualia-inference-kernel` already exists as a thin dep-free crate — pattern to copy.
- **Audio / vision / cooperative / wellfare**: already extracted as sibling crates; core-db still contains large parallel domains (`specialized_libs`, `poet_host`, `net`).
