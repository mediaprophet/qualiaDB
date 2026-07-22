# Monolith Decomposition Plan

_Branch: `0.0.17-dev` | Created: 2026-07-22 | Updated: 2026-07-23_

## Completed
- [x] `webizen-studio/src/components/social_hub.rs` (2642 lines → 5 files in `social_hub/`)
- [x] `webizen-desktop/src/commands/mod.rs` (8417 lines → 25 files in `commands/`)
- [x] `qualia-client-core/src/api.rs` (5991 lines → 21 files in `api/`)
- [x] `qualia-client-core/src/wellfair/api.rs` (4589 lines → 16 files in `wellfair/api/`)
- [x] `webizen-studio/src/components/wellfair/host_client.rs` (4102 lines → 19 files in `wellfair/host_client/`)
- [x] `webizen-studio/src/components/qapps.rs` (3924 lines → 4 files in `qapps/`)
- [x] `webizen-studio/src/components/shoelace.rs` (2138 lines → 8 files in `shoelace/`)
- [x] `webizen-studio/src/components/browser_panes.rs` (1491 lines → 4 files in `browser_panes/`)

## Skipped
- `webizen-studio/src/components/domains_pane.rs` (1513 lines) — single component, too invasive to split without behavioral changes

## Summary
All 8 high-priority monolithic files have been decomposed into modular sub-directories.
Total: ~33,000 lines refactored across 100+ new files.
All crates compile successfully (`cargo check` passes for `qualia-client-core` and `webizen-desktop` and `webizen-studio`).
Warnings are limited to unused imports from blanket `use` statements in generated headers.

## Rules
- Preserve all `#[cfg(target_arch = "wasm32")]` gates
- Preserve all `#[allow(...)]` attributes
- No behavioral changes — pure structural refactor
- Verify compilation after each file decomposition
- Keep public API surface identical (re-exports in mod.rs)
