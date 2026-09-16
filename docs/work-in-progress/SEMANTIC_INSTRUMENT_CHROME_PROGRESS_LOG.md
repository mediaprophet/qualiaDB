# Semantic-instrument chrome — progress log

## 2026-09-16 — SI-09/10 plan acceptance

**Step:** close SI-09/10 against plan acceptance (not only chrome unit bar). Status: **done**.

**What was built**
- Poet: `walkthrough` (author-to-publish + collect→revoke), `manufacture_keys` (keyboard-only Tab/Enter path wired into manufacture panel), `round_trip` (concrete source/visual loss reasons).
- Studio: `instrument_bay/walkthrough` composing inspect→collect→run→permission deny→revoke receipts→suspend.
- Desktop: `si_revoke_demo`, `si_list_receipts`, `si_set_run_permitted`; inspect returns `network_fetch: false`; offline resolution helper; scripted SI-10 walkthrough test.
- Conformance: catalogue author-to-publish, offline-resolved dep + closed run, fixture entry-point parity (never Host.).

**Measured results**
- `cargo test -p poet --lib --offline semantic_instruments::` — **105+ passed** (walkthrough/keys/round_trip included).
- `cargo test -p webizen-studio --offline --bin webizen-studio instrument_bay` — **39 passed**.
- `cargo test -p webizen-desktop --lib --offline semantic_instruments` — **11 passed**.
- `cargo test -p qualia-core-db --test semantic_instruments_conformance --offline` — **18 passed**.
- Not measured: interactive human UAT in a live Poet/Studio window (SI-12 human gate).

**Where I need the human**
- Optional live chrome UAT only. No code blocker for SI-09/10 plan acceptance.

**Next step**
- SI-12 remains the release gate (security/a11y/human UAT matrix). SI-09/10 packets are acceptance-complete at the scripted-test bar.

## 2026-09-16 — SI-09/10 chrome closeout (resume after session break)

**Step:** remaining SI-09/10 chrome swarm + resume after usage-limit failure. Status: **done**.

**What was built**
- Poet: manufacture/receipts/validate/version/deps/fixture/capability/manifest panels registered in `mod.rs` and wired into the bay surface.
- Studio: inspect/run/deps/lifecycle panels + InstrumentFlow Ctrl+Z/Y via `LayoutUndo`; session `data-si-flow-layout` persist (fixed `let mut pos` for Dioxus Signal).
- Desktop: `si_suspend_demo` / `si_remove_demo` / `si_cancel_run`; suspend blocks run; remove drops collect.
- Core: `LocalRegistry::uninstall` drops lookup without erasing receipt history; brace regression in registry tests fixed.

**Measured results**
- `cargo test -p poet --lib --offline semantic_instruments::` — **93 passed**, 0 failed.
- `cargo test -p webizen-studio --offline --bin webizen-studio instrument_bay` — **37 passed**, 0 failed.
- `cargo test -p webizen-desktop --lib --offline semantic_instruments` — **7 passed**, 0 failed.
- `cargo test -p qualia-core-db --lib --offline resolve::registry` — **7 passed**, 0 failed.
- Not measured: live Poet/Studio browser UAT of the full collect→activate→run→suspend walkthrough.

**Where I need the human**
- Chrome UAT only if you want a visual pass on Poet bay + Studio InstrumentBay. No code blocker.

**Next step**
- SI-09/10 chrome walkthrough is closed at the unit-test bar. Not a freeform node-graph IDE; not a clinical kernel; no Host IDs. Further work is SI-12 UAT / human walkthrough if desired.

## 2026-09-15 — Graph Ctrl+Z/Y + Studio flow persist

**Step:** finish the open CLAIM (prior Grok slept). Status: **done**.

**What was built**
- Poet `LayoutUndo` now exposes `current` / `can_undo` / `can_redo`, ignores duplicate tops, and is wired into the instrument node graph.
- Graph `keydown`: Ctrl/Meta+Z undoes, Ctrl/Meta+Y redoes. Handled shortcuts `preventDefault` + `stopPropagation` so the canvas workspace history does not also fire. Empty history leaves the event for the document handler.
- Drag end (`pointerup` / `pointerleave` after a real drag) commits a layout frame and writes `data-si-graph-layout`. Edges follow centres while dragging and on restore.
- `undo_keys` is registered in `semantic_instruments/mod.rs`.
- Studio `InstrumentFlow` persists centres as `"x,y;x,y;..."` on `html[data-si-flow-layout]` and restores on remount when the count matches. Host tokens are refused.

**Measured results**
- `cargo test -p poet --lib --offline semantic_instruments::` — **63 passed**, 0 failed.
- `cargo test -p webizen-studio --offline --bin webizen-studio instrument_bay` — **22 passed**, 0 failed.
- Not measured: browser UAT of Ctrl+Z on a live Poet bay, or remount restore in the Studio webview.

**Where I need the human**
- None this step. Chrome UAT still needs you to launch Poet/Studio if you want a visual check of drag → Ctrl+Z → Ctrl+Y.

**Next step**
- Remaining SI-09/10 chrome is still not a freeform node-graph IDE, not a clinical kernel, and not Host IDs. Manufacture/receipts/select rails were already released by the prior swarm.
