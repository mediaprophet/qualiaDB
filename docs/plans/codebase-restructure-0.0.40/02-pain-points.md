# 02 — Pain points (manageability)

Each item ties to **real paths** on `0.0.40-dev`. Severity is for a small team / AI-assisted workflow, not for blame.

---

## P1 — `qualia-core-db` is still the product (not a core library)

**Evidence:** ~723k lines / 2377 `.rs` files; 13 workspace dependents; dirs include `specialized_libs/` (~190k), `poet_host/` (~57k), `net/` (~61k), medical, MCP, daemons, WASM bridges, geometry, economics, …

**Why it hurts:** Any change risk radius is workspace-wide. WASM feature matrices must cfg-gate huge unrelated trees. New contributors cannot tell “ABI core” from “optional science library” from “Poet invoke host”.

**Symptom paths:**

- `crates/qualia-core-db/src/lib.rs` (~1900 lines of feature/`target_arch` re-exports)
- `crates/qualia-core-db/src/specialized_libs/computational_geometry/`
- `crates/qualia-core-db/src/poet_host/invoke/`

---

## P2 — Generated / chain action monsters in Poet + invoke id tables

**Evidence:** Multiple 170–210 KB single files under `crates/poet/src/browser/` (`tool_copy.rs`, `econ_chain_actions.rs`, `tool_actions.rs`, …); `poet_host/invoke/ids.rs` ~141 KB.

**Why it hurts:** Reviews are unreadable; merges conflict constantly; AI agents burn context on noise; “edit one tool” becomes “open a novel”.

---

## P3 — Dual (triple) UI / host stacks without a crisp ownership map

**Evidence:**

- `webizen-studio` (Dioxus) + `webizen-desktop` (Tauri hosting studio) + `poet` (separate HyperCanvas wasm/rlib) + leftover portal static under desktop
- Studio still contains hundreds of `*_qapp.rs` academic stubs under `components/`
- Human-surface WIP vows “One Poet · two hosts” but crate layout still looks like three products

**Why it hurts:** Feature land in the wrong host; parity audits proliferate (`docs/work-in-progress/HUMAN_SURFACE_*`); contributors ask “which UI is real?”

---

## P4 — WASM / native feature matrix complexity lives in one crate

**Evidence:** Exclusive `wasm-ontology` vs `portal` / `wasm-full` with `compile_error!` in `lib.rs`; CI builds 5+ feature combinations; two workspace release profiles with incompatible LTO settings; `cfg(target_arch)` dependency splits in studio / lite-wasm / web / render.

**Why it hurts:** “Does this compile for wasm?” is not a local question — it is a combinatorial one. Accidental default-feature pulls break lite size budgets.

---

## P5 — Docs accretion + private `docs/plans/` black hole

**Evidence:**

- Root: `AGENTS.md` (65 KB), `AI_INSTRUCTIONS.md`, `CLAUDE.md`, `MASTER_INDEX.md`, multiple inference plan markdowns, `DECOMPOSITION_PLAN.md` (stale era label)
- `docs/work-in-progress/` ~138 files
- Dozens of links to `docs/plans/*.md` that are **gitignored and often absent** (audio lib.rs, ADR 0011, executable-operator swarm, vibescript plans, …)

**Why it hurts:** Truth is split across published manuals, private plans, and WIP. Agents re-invent programmes that already exist off-git. New humans cannot find a single starting description (the original motivator for this pack).

---

## P6 — Parallel ontology / asset trees

**Evidence:** `core-ontologies/`, `ontologies/`, `bundled/ontologies/`, `crates/poet/ontologies`, plus `bundled/{qapps,models,trust,grounding}`.

**Why it hurts:** Unclear which corpus is authoritative for runtime vs docs vs poet tool-chest; duplicate curation cost.

---

## P7 — Naming collisions and dir/package skew

**Evidence:**

- Directory `webizen-web` packages as `qualia-wasm`
- `poet-cli` binary name `poet` vs `poet` package / `poet-ui` binary
- Product name **Webizen** vs engine **QualiaDB** vs shell **Poet** vs domain **WellFair** — all correct, but crate prefixes (`webizen-*` vs `qualia-*` vs bare `poet`/`vibe`) do not encode layer

**Why it hurts:** `cargo run -p …` and docs search misfire; AI tools invent wrong crate names.

---

## P8 — Root junk and vendored gravity

**Evidence:** `old_lib.rs`, `stash-wip.patch`, `_cfd_err.txt` (442 KB), root `check-gguf-*.mjs`, `vendor/` (~64 MB), `geometry-showcase/` beside in-tree geometry.

**Why it hurts:** Cognitive load at `ls` time; unclear what is load-bearing vs archaeology.

---

## P9 — Client-core as a second “god crate”

**Evidence:** `qualia-client-core` ~79k lines; `lib.rs` exports a very wide module surface (chat_*, wellfair, api/, identity_plane, mail, github, …); depended on by desktop, studio, and CLI.

**Why it hurts:** Extracting core-db domains without a plan for client-core just moves the fat upward into the app-support layer.

---

## P10 — Incomplete prior reorg signalling

**Evidence:** `MODULE_REORG_PLAN.md` still reads like a to-do for loose files; many categories exist, but `lib.rs` remains a mega re-export switchboard. Root `DECOMPOSITION_PLAN.md` looks like “the” plan to newcomers but only covers 0.0.17 file splits.

**Why it hurts:** Agents resume the wrong plan; humans assume restructure is “already done”.

---

## Priority order for action (feeds migration phases)

1. **Agree layering + crate roles** (this pack / ADR) — no code.
2. **Publish or relocate missing `docs/plans` anchors** that ADRs already cite — or retarget links to `docs/manuals` / `docs/design`.
3. **Carve extractable domains out of core-db** starting with already-cohesive trees (`specialized_libs/*`, then `poet_host`, then `net/qdnf`) using the inference-kernel pattern.
4. **Shrink Poet generated surfaces** (split or generate-into-dirs; stop growing single files).
5. **Clarify One Poet / two hosts** ownership in docs + crate READMEs before further UI features.
6. **Hygiene pass** on root junk and ontology authority (move/archive, don’t rewrite behaviour).
