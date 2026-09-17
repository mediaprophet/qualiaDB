# Build disk hygiene (WIP) — one `target/`, no silent 100G

**Status:** work-in-progress · **Branch:** `0.0.38` · **Cite:** Timothy approve 2026-09-11 (Capt box `target/` cleanup) · Desktop A `OS_SAFETY_VIOLATION` &lt;15 GB
**Owner:** Capt (ops) · **Fold:** Neo · **Applies to:** shared box + agent build lanes

## Why

Cargo `target/` trees grew to tens of GB (qualiaDB alone ~59G; parallel/old trees approached ~100G). Webizen Desktop first-run then died on `OS_SAFETY_VIOLATION` (~16G free vs &gt;15G spare). Rebuildable caches are not product state — source and Git stay.

## Rule (normative for bots)

1. **One workspace `target/`** per checkout — default Cargo layout only.
2. **No parallel `*-target` / duplicate build trees by default** (no silent second crates-io cache trees, no “just in case” worktree builds without an explicit ops ask).
3. **Check free disk before big builds** (`df -h` / equivalent). Prefer not to start a heavy `cargo` / trunk / Tauri build when free is already tight.
4. **Prune when free &lt; ~20G:** `cargo clean` in the active workspace (or Capt/Disk Saver approved bulk delete of rebuildable `target/` only). Never delete source, `.git`, or QualiaData / sanctuary volumes as “cleanup.”
5. **Report reclaim** after prune (before/after free GB) so Desktop first-run and Continuity UAT aren’t blocked again.

## Non-goals

No Host invent · no Wave-22 tick from cleanup alone · no lowering the Desktop safety gate as a substitute for headroom.

## Related

- Desktop Frame A HELD: `HUMAN_SURFACE_COLDLOAD_UAT_SCORE_FRAME_A_DESKTOP_c6007e0.md`
- Continuity gate (separate): `CONTINUITY_GATE_HANDLE_REVOKE_WIP.md`
