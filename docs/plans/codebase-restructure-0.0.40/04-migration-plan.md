# 04 — Migration plan (phased, reversible)

**Principle:** incremental over big-bang. Every phase leaves the tree building. Prefer `pub use` shims. One extract family at a time. Resource-aware: each phase should be doable in a short human+AI sprint.

---

## Phase 0 — Docs / agreement only (THIS PACK)

**Do:**

- Land this directory; open PR to `0.0.40-dev`.
- Agree layering vocabulary (core / module / app) via ADR 0001.
- Explicitly mark root `DECOMPOSITION_PLAN.md` as historical (0.0.17 file-split) in a one-line note in a *future* docs PR if desired — not required to unblock.

**Do not:** move crates, rename packages, change features.

**Exit:** Maintainers say “we will steer new work by 03/05”.

---

## Phase 1 — Navigation & hygiene (still mostly docs)

**Do:**

1. Add short `LAYER.md` stubs (or README sections) on the top 10 crates naming their tier.
2. Inventory broken `docs/plans/…` links cited from ADRs/manuals; for each: publish a summary under `docs/manuals/` **or** retarget the link.
3. Propose (but optionally defer) moving root junk (`old_lib.rs`, `stash-wip.patch`, `_cfd_err.txt`) to `artifacts/archaeology/` or delete if confirmed dead.
4. Document ontology authority: which of `core-ontologies` / `bundled/ontologies` / `ontologies` is runtime vs reference.

**Exit:** New contributor start path in pack README works without tribal knowledge; no silent 404s from accepted ADRs.

**Revert:** delete stubs / restore links.

---

## Phase 2 — Stop the bleeding (process, tiny code)

**Do:**

1. **No new files &gt; ~50 KB** under `poet/src/browser/` without splitting — enforce in review.
2. New specialized science code goes under `specialized_libs/<family>/`, not `src/*.rs`.
3. New Poet invoke IDs: prefer append-only generated modules per family directory over growing `ids.rs` monolith (design only if generation already exists — don’t invent a new codegen stack in this phase).
4. Continue `MODULE_REORG_PLAN.md` category moves **inside** core-db (re-export preserving) — complementary, low risk.

**Exit:** Hotspot file list stops growing; category dirs absorb remaining loose files.

---

## Phase 3 — First real extract: specialized geometry (pilot)

**Why first:** Largest cohesive tree (`specialized_libs/computational_geometry/`), already directory-scoped, fewer WASM lite consumers expected.

**Do:**

1. Create `crates/qualia-geometry` (name bikeshed OK) with code moved from `computational_geometry/`.
2. `qualia-core-db` depends on it; `pub use` old paths.
3. Gate native-heavy bits as today; verify `wasm-ontology` / `portal` builds.
4. Update DIRECTORY_INDEX / one manual pointer.

**Exit:** `cargo check` native + wasm profiles used in CI; lite size budget unchanged (±noise).

**Revert:** vendor path back into core-db; remove crate from workspace.

---

## Phase 4 — Inference runtime module boundary

**Do:**

1. Keep `qualia-inference-kernel` as contracts.
2. Introduce `qualia-inference-runtime` (or feature-modules) for `gguf_bridge` + decode orchestration that must share GPU device with render — **design carefully** with `webizen-render` “one physical device” constraint (`webizen-render/Cargo.toml` comment).
3. Do not break P64 / GGUF paths; run existing inference tests/benches.

**Exit:** Kernel remains dep-free; runtime crate has explicit deps; core-db facade still compiles dependents.

---

## Phase 5 — Poet host out of “core”

**Do:**

1. Move `poet_host/` toward `crates/qualia-poet-host` or fold under `poet` with a shared invoke crate.
2. Desktop/CLI that need invoke depend on the new crate; core-db loses Poet registration gravity.
3. Split or generate chain-action sources into per-family files as part of the move.

**Exit:** `qualia-core-db` no longer owns HyperCanvas tool registration; poet builds green.

---

## Phase 6 — Net / QDNF module

**Do:** Extract `net/qdnf` (+ peer-facing types) behind `qualia-peer` / a `qualia-net` module; keep `libp2p-compat` as explicit opt-in.

**Exit:** Peer facade does not require full default core-db feature soup.

---

## Phase 7 — Client-core thinning

**Do:** After core-db facade shrinks, split `qualia-client-core` along product seams (chat mesh, wellfair host API, identity plane) into modules that studio/desktop select.

**Exit:** Studio wasm builds do not compile unused native mail/github stacks (where cfg allows).

---

## Phase 8 — Naming / package skew (optional late)

- Align `webizen-web` directory with package name (or rename package with a transition).
- Disambiguate `poet` CLI bin vs package.
- Consider whether facade `qualia-core-db` keeps its name once it is thin.

---

## Phase gates (every code phase)

| Gate | Command / check |
|------|-----------------|
| Native facade | `cargo check -p qualia-core-db` |
| Lite wasm | `cargo check -p webizen-lite-wasm --target wasm32-unknown-unknown` (+ size script if releasing) |
| Portal / pages subset | match `.github/workflows/pages.yml` checks you touched |
| Desktop lib | `cargo check -p webizen-desktop` when UI/host edges move |
| Revert plan | Written in the PR body (shim removal steps) |

---

## Explicitly deferred / out of scope

- Rewriting Dioxus version pins / Tauri major upgrades.
- Merging Poet and Studio into one crate in a single PR.
- Relicensing or policy changes.
- Deleting specialized science libraries “to simplify” — extract or feature-gate, don’t cull capability for tidiness.
- Big-bang Cargo workspace split into multiple repos.

---

## Suggested ownership for a small team

| Phase | Owner shape |
|-------|-------------|
| 0–1 | Human principal + docs-capable agent |
| 2 | Any PR author (process) |
| 3 | One extract pilot (geometry) — single lane |
| 4 | Inference-experienced lane only |
| 5–6 | Poet / QDNF lanes respectively |
| 7–8 | After 3–6 stable |
