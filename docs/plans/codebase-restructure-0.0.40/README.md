# Codebase restructure recommendations — QualiaDB 0.0.40

**Status:** Docs only — no application/runtime code moved yet.  
**Survey tip:** `origin/0.0.40-dev` @ `1cdaa63219bcf679b827acc95bbc3e27a3351c15`  
**Branch that carries this pack:** `docs/codebase-restructure-0.0.40`  
**Audience:** Timothy + small-team / AI-assisted contributors under resource constraints.  
**Product framing:** **civics** (citizenship / citizen-led public-good). Not a government portal.

---

## How to read this pack

| Order | File | Purpose |
|------:|------|---------|
| 1 | [01-current-state.md](./01-current-state.md) | Factual map of the tree on `0.0.40-dev` |
| 2 | [02-pain-points.md](./02-pain-points.md) | Manageability issues tied to real paths |
| 3 | [03-target-architecture.md](./03-target-architecture.md) | Core libraries vs modules vs apps; layering rules |
| 4 | [04-migration-plan.md](./04-migration-plan.md) | Phased, reversible moves; Phase 0 = agreement only |
| 5 | [05-crate-map.md](./05-crate-map.md) | Proposed inventory mapped from today's 27 workspace crates |
| — | [adr/0001-layering-and-crate-roles.md](./adr/0001-layering-and-crate-roles.md) | Load-bearing decision: layer names and forbidden edges |

Read **01 → 02 → 03** before debating moves. Use **04** only after agreement. Use **05** as the working checklist.

---

## Non-goals (this pack)

- No crate moves, renames, or Cargo.toml edits in this PR.
- No behavioural changes to inference, WASM profiles, Poet, or desktop.
- No rewrite of Continuity / human-vs-handle / fiduciary constraints (preserve; do not invent policy).
- Not a replacement for root `DECOMPOSITION_PLAN.md` (that is the **0.0.17 file-split** ledger — different concern).
- Not a replacement for `crates/qualia-core-db/MODULE_REORG_PLAN.md` (in-crate category moves with re-exports — complementary, not superseded).

---

## Docs-only / gitignore note

Repo `.gitignore` treats `docs/plans/` as private working space. This subdirectory is **intentionally published** via `git add -f` so humans can review the restructure on GitHub. Other files under `docs/plans/` remain local/private unless similarly force-added by explicit decision.

---

## Where a new contributor starts

1. **Product why:** root [`README.md`](../../../README.md) — natural-person / fiduciary framing; watch *The Untransferable Code*.
2. **Agent hard rules:** [`AGENTS.md`](../../../AGENTS.md) §0 (zero-heap tiers, 48-byte NQuin, 42MB Sentinel) + [`CLAUDE.md`](../../../CLAUDE.md) §15 fiduciary duty.
3. **Human surface honesty:** Continuity **handle ≠ who** — see `docs/work-in-progress/CONTINUITY_GATE_HANDLE_REVOKE_WIP.md` and ADR `docs/manuals/adr/0011-human-centric-consent-accountability-and-disposition.md`. Agents are sub-agents of human principals (`AI_INSTRUCTIONS.md`).
4. **Build surfaces (pick one):**
   - Native daemon / CLI → `crates/qualia-cli`, `crates/qualia-core-db`
   - Desktop shell → `crates/webizen-desktop` + `crates/webizen-studio`
   - Browser lite ontology MCP → `crates/webizen-lite-wasm`
   - VibeScript language → `crates/vibe` (+ `poet-cli` / `vibe-lsp`)
5. **Do not** start by editing the largest files under `crates/poet/src/browser/` or `crates/qualia-core-db/src/specialized_libs/` without a phase ticket from [04-migration-plan.md](./04-migration-plan.md).

---

## Executive sketch (detail in 03 / 05)

```
CORE LIBS (stable ABI / few deps)
  vibe · qualia-inference-kernel · (future) q42/graph/storage slices of core-db

DOMAIN MODULES (optional features or eventually crates)
  query/SPARQL · modalities/governance · inference/gguf · specialized_libs.* ·
  net/qdnf · crypto · wellfare/cooperative · vision · audio

APP / HOST LAYER
  webizen-desktop · webizen-studio · qualia-cli · poet (+ poet-cli) ·
  webizen-lite-wasm · webizen-web · qualia-mobile-harness · qualia-peer

SUPPORT / TOOLING
  work-graph · webizen-component-harvester · qualia-semantic-library ·
  vibe-lsp · vibe-wasm · scripts/ · docs/
```

Layering rule in one line: **apps depend on modules; modules depend on core; core never depends on apps.**

---

## Related existing artefacts (do not confuse)

| Artefact | What it is |
|----------|------------|
| `DECOMPOSITION_PLAN.md` (root) | 0.0.17 monolith **file splits** (studio/desktop/client-core) — done |
| `crates/qualia-core-db/MODULE_REORG_PLAN.md` | Move loose `src/*.rs` into category dirs with re-exports |
| `docs/work-in-progress/CRATE_SURFACE_INCORPORATION_*.md` | Vibe/Poet bind inventory — not crate layout |
| Root `NATIVE_INFERENCE_OPTIMIZATION_PLAN.md` | Inference performance programme |

