# Rust Module Decomposition Register

**Status:** Work in progress  
**Date:** 2026-09-05  
**Baseline:** `0.0.36-dev` at `4eade061`

## Purpose

Track large Rust implementation files that need purpose-specific ownership,
without treating line count alone as proof of bad design. This programme fixes
module structure and links as part of each decomposition. It does not suppress
compiler errors or warnings to make an incomplete move appear successful.

The current issue predates the Grok-bot lane plans. For example,
`crates/poet/src/browser/css.rs` entered the repository with POET integration
on 2026-08-22 and now contains 3,322 lines in one embedded CSS string. Agent or
platform identity is not used to assign fault.

## Baseline inventory

The 2026-09-05 scan found, excluding `tests/` directories, `tests.rs`, benches,
and files named `generated_*.rs`:

- 580 Rust files at or above 500 lines.
- 152 Rust files at or above 1,000 lines.
- 47 POET Rust files at or above 500 lines.
- 14 POET Rust files at or above 1,000 lines.

These counts are triage inputs, not 580 automatic refactors. Large generated
artifacts, test vectors, static registries, and cohesive numerical algorithms
may need a documented exception rather than arbitrary fragmentation.

## Structural rules

1. Do not add new behavior to a production Rust file above 1,000 lines without
   decomposing the touched responsibility in the same programme.
2. Review files from 500 to 999 lines when they own more than one lifecycle or
   responsibility; new files should remain below 500 lines.
3. Keep the existing public API stable through a small `mod.rs` router and
   deliberate `pub use` exports unless an ABI/API change is separately approved.
4. Separate state/types, construction, event wiring, rendering/projection,
   persistence, backend code, and tests when those responsibilities coexist.
5. Treat CSS, shaders, fixtures, and other assets as assets. Do not retain a
   multi-thousand-line resource inside Rust solely to avoid module wiring.
6. Every move updates module declarations, imports, re-exports, tests, and build
   references in the same packet. No `allow` attribute or warning suppression
   is an acceptance substitute.
7. Preserve hot-path allocation, 48-byte `NQuin`, 42 MB Sentinel, public ABI,
   and deterministic-order contracts during core decompositions.
8. Decompose on demand before the next feature touches a module. The product is
   not blocked on unrelated large files elsewhere in the workspace.

## POET priority register

| Priority | Current file or cluster | Lines | Responsibility split | Difficulty/functions | Trigger |
|---|---|---:|---|---|---|
| P0 | `browser/registration.rs` plus 16 `register_*` siblings | 81 plus bounded leaves | Establish one coherent registration library/module tree; preserve all toolbox registrations | `D2` `RUST` `QA` | Current compile blocker |
| P0 | `browser/css.rs` | 3,322 | Extract tokens, shell/chrome, forms, canvas/containers, dialogs, workbenches, sheet, and Health into purpose-specific CSS assets composed by a small Rust loader | `D2` `FE` `UX` `QA` | Before further POET/Health styling |
| P1 | `browser/topbar.rs` | 2,734 | Menu construction, action dispatch, save/import dialogs, control bar, pod trays, filters, and manifold controls | `D3` `FE` `UX` `QA` | Before topbar/chrome changes |
| P1 | `browser/interactions.rs` | 2,148 | Pointer state, wire operations, container commands, canvas movement, docking, placement, and geometry helpers | `D3` `FE` `QA` | Before interaction or canvas changes |
| P1 | `browser/search_workbench/` | 34 plus bounded leaves | Completed `RM-05`: faceted search, query builder, SPARQL editor, saved-query persistence, execution, and placement | `D3` `FE` `RUST` `QA` | Before Tool Chest search semantics work |
| P1 | container cluster: `containers/` plus `container_views/` plus `container_views_ext/` plus `container_inline_views.rs` | `containers/` (`RM-06`), `container_views/` (`RM-10`), `container_views_ext/` (`RM-11`) complete; inline 1,016 | Shell and extra renderers split; inline stays a real `pub fn` file under the 1,200 trigger | `D3` `FE` `UX` `QA` | Before broad container restoration |
| P2 | `docks/` plus `instrument_panel/` plus `workflow_panels/` | Complete (`RM-07`–`RM-09`); all leaves under 500 | Dock, instrument, and workflow presentation are directory modules | `D3` `FE` `UX` `QA` | Before Desktop/chrome reuse |
| P2 | `browser/mod.rs`, `native_daemon.rs` | 1,177 and 1,369 | Browser composition root and daemon transport/lifecycle boundaries | `D4` `RUST` `SPEC` `QA` | Only when their contracts are touched |

## Core and cross-crate boundary

The wider scan includes large files in `qualia-core-db`, `qualia-client-core`,
Vibe, Webizen Studio/Desktop, rendering, and extensions. They must not be bulk
split by a UI packet. Begin with a `D4` boundary audit that classifies each file
as router, generated/static data, cohesive algorithm, multi-lifecycle module,
hot path, or public ABI owner. Prioritize files that an accepted implementation
packet must touch.

The first core candidates include `qualia-core-db/src/poet_host/mod.rs` (2,943),
`poet_host/invoke/ids.rs` (2,774), `sparql_library/sparql_filter.rs` (2,549),
`sparql_library/sparql_executor.rs` (2,451), and `lib.rs` (1,862). Their size
does not authorize changing capability IDs, SPARQL behavior, ABI, or hot-path
allocation while moving code.

## Bounded packets

### `RM-01` - Registration library repair

This is the structural form of `FIX-REG-01`. Align module ownership with the
existing purpose-specific registration files. Do not use path attributes as a
permanent workaround, flatten the files back into one source file, omit a
toolbox, widen capability bindings, or change the frozen host facade.

**Completed 2026-09-05:** Moved the unchanged router to
`browser/registration/mod.rs` and the 16 unchanged toolbox modules beneath it.
Rust's ordinary directory-backed module resolution now matches their logical
ownership. No path attributes, warning suppression, capability changes, or
host-facade changes were introduced.

Verification:

- `cargo test -p poet --test product_integrity`: 9 passed.
- `cargo test -p poet --test surface_inventory`: 1 passed.
- `trunk build`: passed after setting the existing `NO_COLOR` environment value
  from unsupported `1` to `true` for that process.
- Existing warnings: two workspace-profile warnings remain; none were caused or
  suppressed by this move.
- Pre-existing modified `Cargo.lock`: preserved byte-for-byte, SHA-256
  `7BA74F191298375E6BCBF0A8DB15E59797F8676D9FB76FAF61E42D7DF5C55D83`.

### `RM-02` - POET style asset decomposition

Replace the single embedded CSS body with purpose-specific `.css` assets and a
small deterministic composition point. Preserve cascade order and exact runtime
injection behavior. Verify stylesheet assembly, both focused POET suites, web
build, and desktop/mobile browser screenshots before claiming visual parity.

**Completed 2026-09-05:** Replaced the 3,322-line embedded Rust raw string with
a 43-line composition module and 14 purpose-specific CSS assets. The largest
asset is 421 lines. Asset concatenation matches the normalized original CSS
body at SHA-256
`BFF95C324960484900865245E6133D2D702A8AF96D5373220EC238EEC84786AB`.

Verification:

- focused stylesheet composition/order test: 1 passed;
- `cargo test -p poet --test product_integrity`: 9 passed;
- `cargo test -p poet --test surface_inventory`: 1 passed;
- `trunk build`: passed;
- browser UAT: rendered at 1280x720 and 390x844 with 113,194 stylesheet
  characters loaded; mobile body width remained 390 pixels;
- scoped `rustfmt --check crates/poet/src/browser/css.rs`: passed;
- crate-wide format check remains blocked by unrelated pre-existing formatting
  drift and was not applied;
- pre-existing modified `Cargo.lock` remained byte-for-byte unchanged.

### `RM-03` onward - Touch-before-grow decomposition

Select exactly one responsibility cluster needed by the next accepted product
packet. Record the old public surface, target module map, import/re-export
changes, focused tests, broader build, warnings introduced or removed, and any
deferred responsibilities. Core/ABI/hot-path work is `D4`; bounded POET module
moves are normally `D3`.

### `RM-03` - POET topbar decomposition

**Completed 2026-09-05:** Replaced the 2,734-line `browser/topbar.rs` with a
33-line stable router and eight purpose-specific modules:

| Module | Responsibility | Lines |
|---|---|---:|
| `menu.rs` | Menubar and dropdown construction | 447 |
| `actions.rs` | Menu event wiring and action dispatch | 416 |
| `save_dialog.rs` | Save-mode dialog and persistence dispatch | 342 |
| `manifold.rs` | Manifold chrome, title, import/export, and creation | 315 |
| `control_bar.rs` | Canvas control bar and pod buttons | 371 |
| `pods.rs` | Technical sidebar, accessibility notice, and tray shell | 168 |
| `filters.rs` | Strata, epistemic, dimension, and timeline controls | 392 |
| `help_dialogs.rs` | Shortcut, honesty, and about dialogs | 296 |

All former public functions and `MenuItemDef` remain available through
`browser::topbar`; browser-parent links were updated for the additional module
depth. No menu action, label, capability, persistence, or host behavior changed.

Verification:

- `cargo check -p poet`: passed;
- `cargo test -p poet --test product_integrity`: 9 passed;
- `cargo test -p poet --test surface_inventory`: 1 passed;
- `trunk build`: passed;
- scoped rustfmt for all nine topbar files: passed;
- browser UAT at 1280x720 and 390x844: File menu opened with nine items,
  Strata tray populated, mobile body width matched the viewport, and browser
  logs contained no warnings or errors;
- pre-existing modified `Cargo.lock` remained byte-for-byte unchanged.

After `RM-02` and `RM-03`, the implementation-focused scan has 67 files over
1,400 lines and 12 over 2,000, down from 69 and 14 respectively.

### `RM-04` - POET interaction decomposition

The next candidate on the decomposition lane is `browser/interactions.rs`
(2,148 baseline lines). Split pointer state, wire operations, container
commands, canvas movement, docking, placement, and geometry helpers behind the
existing `browser::interactions` API. This is `D3` (`FE`, `QA`) and should be
scheduled before interaction or canvas behavior is extended.

**Completed 2026-09-05:** Replaced the monolith with a 92-line stable router
and seven purpose-specific modules:

| Module | Responsibility | Lines |
|---|---|---:|
| `pointer.rs` | Global pointer lifecycle and active interaction dispatch | 193 |
| `wires.rs` | Semantic wire creation, rendering, and refresh | 336 |
| `container_commands.rs` | Selection, deletion, duplication, zoom, and keyboard commands | 479 |
| `canvas_motion.rs` | Container drag/resize and canvas pan/zoom | 209 |
| `docking.rs` | Tool Chest docking, flyouts, chains, and selector controls | 406 |
| `placement.rs` | Geometry inventory, arrangement, placement, and notices | 261 |
| `geometry.rs` | Style parsing, grid snapping, and focused tests | 223 |

Shared pointer state remains in the router. Every former public function and
`ContainerRect` remains available through `browser::interactions`; browser
links were adjusted for the additional module depth. The only sibling seam
needed was the existing internal `place_container_on_canvas` helper.

Verification:

- `cargo check -p poet`: passed;
- focused interaction geometry tests: 5 passed;
- product integrity: 9 passed;
- surface inventory: 1 passed;
- `trunk build`: passed;
- scoped rustfmt for all eight interaction files: passed;
- browser UAT: container selection, View-menu zoom from `1.0` to `1.1000`,
  Epistemic Tool Chest flyout, right/left docking, and 390x844 mobile layout
  passed; browser logs contained no warnings or errors;
- pre-existing modified `Cargo.lock` remained byte-for-byte unchanged.

The implementation-focused scan now has 66 files over 1,400 lines and 11 over
2,000, down from the 69 and 14 baseline recorded before `RM-02`.

### `RM-05` - POET search workbench decomposition

The next POET candidate is `browser/search_workbench.rs` (2,033 baseline
lines). Split faceted search, query construction, SPARQL editing/execution,
saved-query model/persistence, result placement, and workbench shell/wiring
behind the existing `browser::search_workbench` API. This is `D3` (`FE`, `RUST`,
`QA`) because live/local provenance and daemon behavior must remain honest.

**Completed 2026-09-05:** Replaced the 2,033-line monolith with a 34-line
stable router and eight purpose-specific modules:

| Module | Responsibility | Lines |
|---|---|---:|
| `catalog.rs` | Facet option lists and common SPARQL predicates | 106 |
| `shell.rs` | Overlay build, mode tabs, toggle, shortcut, notices | 330 |
| `faceted.rs` | Facet chips and live-canvas matching | 389 |
| `builder.rs` | Visual triple-pattern builder and SPARQL preview | 378 |
| `sparql.rs` | Manual SPARQL editor and daemon-backed execution | 208 |
| `persist.rs` | Saved-query model, localStorage, and parser tests | 192 |
| `saved.rs` | Saved-query list UI and save/load/delete actions | 313 |
| `placement.rs` | Place a query as a graph container on the canvas | 88 |

Every former public function remains available through
`browser::search_workbench`. Callers already used that path, so no parent
relink was required. SPARQL still executes only against a connected QualiaDB
daemon and does not fabricate an offline result set.

Verification:

- `cargo check -p poet`: passed, no new warnings;
- focused persist/parser tests: 5 passed;
- product integrity: 9 passed;
- surface inventory: 1 passed;
- `trunk build`: passed (`INFO success`); new wasm contains
  `search-workbench`, `sparql-editor`, `qualia-ui:saved-queries`, and the
  daemon-unavailable honesty string;
- scoped rustfmt (`--edition 2021`) for all nine search-workbench files: passed;
- interactive browser click-UAT was not re-run in this session (no click
  driver); a live `trunk` process is serving `127.0.0.1:8080` from the new
  dist;
- pre-existing modified `Cargo.lock` remained byte-for-byte unchanged,
  SHA-256 `7BA74F191298375E6BCBF0A8DB15E59797F8676D9FB76FAF61E42D7DF5C55D83`.

A crates-`src` scan now has 66 files over 1,400 lines and 10 over 2,000.
POET itself has no remaining implementation file over 2,000 lines; the next
POET files over 1,400 are `docks.rs` (1,479), `instrument_panel.rs` (1,474),
and `containers.rs` (1,473).

### `RM-06` - POET container shell decomposition

**Complete** (2026-09-05). `browser/containers.rs` (1,507 lines at split)
is now `browser/containers/` with a stable `build_container` export.

Source-to-destination map:

| Old | New |
|---|---|
| Chrome, ports, resize, restore | `containers/shell.rs` (166) |
| Filter attrs, media surface, type tags | `containers/attrs.rs` (388) |
| Body match dispatch | `containers/body.rs` plus `body_{project,health,studio,ontology,core}.rs` |
| `pub fn build_container` | re-exported from `containers/mod.rs` (`pub use shell::build_container` only) |

Behavior, ABI, and allocation: unchanged. Callers still use
`browser::containers::build_container`. Domain renderers remain in
`container_views.rs`, `container_views_ext.rs`, and
`container_inline_views.rs` (real `pub fn` builders). Those files were not
converted to directory `pub use … build_*_view` routers because that
pattern is what `GENERIC_DELEGATION_CEILING` (112) counts.

Verification:

- `cargo +stable check -p poet --lib`: passed
- `cargo +stable test -p poet --lib containers::`: 4 passed
- product integrity: 10 passed (ceiling held; health calculator route still wired)
- surface inventory: 1 passed
- `RUSTUP_TOOLCHAIN=stable NO_COLOR=true trunk build`: success; wasm contains
  `health_calculators`, `canvas-container-node`, `data-code-habitat`
- scoped rustfmt (`--edition 2021`) on the new container files: passed
- interactive browser click-UAT was not re-run

A crates-`src` scan now has 64 files over 1,400 lines and 10 over 2,000.
POET itself still has no file over 2,000. Remaining POET files over 1,400:
`docks.rs` (1,575), `instrument_panel.rs` (1,475), `workflow_panels.rs` (1,418).

### `RM-07` - POET docks decomposition

**Complete** (2026-09-05). `browser/docks.rs` (1,575 lines at split) is now
`browser/docks/` with the stable public API preserved.

Source-to-destination map:

| Old | New |
|---|---|
| View models, family order, toolbox-view storage | `docks/model.rs` (263) |
| Toolbox/tool glyphs and kind badges | `docks/glyphs.rs` (73) |
| `build_toolchain_widgets` | `docks/widgets.rs` (380) |
| Left Tool Chest dock | `docks/toolbox.rs` (238) |
| Flyout show/hide | `docks/flyout.rs` (219) |
| Collapsible panel chrome | `docks/panel.rs` (112) |
| Right dock | `docks/right.rs` (180) |
| Bottom status bar | `docks/statusbar.rs` (155) |

Behavior, ABI, and allocation: unchanged. Callers still use
`browser::docks::{build_toolbox_dock, show_flyout, hide_flyout,
build_right_dock, build_bottom_statusbar, create_collapsible_dock_panel,
extract_toolbox_views, store_toolbox_views, …}`. No `pub use … build_*_view`
wrappers.

Verification:

- `cargo +stable check -p poet --lib`: passed
- `cargo +stable test -p poet --lib docks::`: 2 passed
- product integrity: 10 passed (ceiling held)
- surface inventory: 1 passed
- `RUSTUP_TOOLCHAIN=stable NO_COLOR=true trunk build`: success; wasm contains
  `toolbox-dock`, `Tool Chest`, `bottom-statusbar`, `right-dock`, `Aura Tray`
- scoped rustfmt (`--edition 2021`) on the new dock files: passed
- interactive browser click-UAT was not re-run

POET files over 1,400 are now `instrument_panel.rs` (1,475) and
`workflow_panels.rs` (1,418).

### `RM-08` - POET instrument panel decomposition

**Complete** (2026-09-05). `browser/instrument_panel.rs` (1,475 lines at
split) is now `browser/instrument_panel/` with the stable public API
preserved.

Source-to-destination map:

| Old | New |
|---|---|
| Ribbon tool descriptor | `instrument_panel/ribbon.rs` (10) |
| Tools keyed by container type | `instrument_panel/catalog.rs` (412) |
| Local vs daemon command helpers | `instrument_panel/commands.rs` (284) |
| Click dispatch | `instrument_panel/dispatch.rs` (266) |
| Show/hide/wire panel chrome | `instrument_panel/panel.rs` (161) |
| Tool-chain activate/deactivate | `instrument_panel/chain.rs` (411) |
| Public re-exports | `instrument_panel/mod.rs` (17) |

Behavior, ABI, and allocation: unchanged. Callers still use
`browser::instrument_panel::{show_for_container, hide, activate_chain,
activate_chain_on_container, deactivate_chain}`. No `pub use … build_*_view`
wrappers.

Verification:

- `cargo +stable check -p poet --lib`: passed
- `cargo +stable test -p poet --lib instrument_panel::`: 6 passed
  (catalog 2, commands 2, chain 2)
- product integrity: 10 passed (ceiling held)
- surface inventory: 1 passed
- `RUSTUP_TOOLCHAIN=stable NO_COLOR=true trunk build`: success; wasm contains
  `contextual-instrument-panel`, `instrument-panel-tool-btn`, `doc:bold`,
  and the daemon-unavailable honesty string
- scoped rustfmt (`--edition 2021`) on the new instrument-panel files: passed
- interactive browser click-UAT was not re-run

The only remaining POET implementation file over 1,400 lines is
`workflow_panels.rs` (1,418). That is `RM-09`. Do not close Review Gate A.
Do not start `AST-*`. `PFT-03` remains owner/captain selection.

### `RM-09` - POET workflow panels decomposition

**Complete** (2026-09-05). `browser/workflow_panels.rs` (1,418 lines at
split) is now `browser/workflow_panels/` with the former public
`build_*_view` names preserved.

Source-to-destination map:

| Old | New |
|---|---|
| Checkpoint tray + localStorage parse | `workflow_panels/checkpoint.rs` (306) |
| Credential inspector | `workflow_panels/credentials.rs` (261) |
| Context markup editor | `workflow_panels/markup.rs` (196) |
| Provenance panel | `workflow_panels/provenance.rs` (178) |
| Publication workflow | `workflow_panels/publication.rs` (223) |
| Constituency manager | `workflow_panels/constituency.rs` (186) |
| Capability / checkpoint / consent widgets | `workflow_panels/widgets.rs` (112) |
| Public glob re-exports | `workflow_panels/mod.rs` (27) |

Behavior, ABI, and allocation: unchanged. Callers can still use
`browser::workflow_panels::build_*_view`. Re-exports are `pub use
checkpoint::*` (and siblings), **not** `pub use … build_*_view`, so
`GENERIC_DELEGATION_CEILING` stays 112.

Honest unused-module note: live container routes already use
`checkpoint_panel`, `publication_panel`, and `governance_workflow`. The
`workflow_panels` builders remain a public API but are not on those
routes; wasm-opt/LTO therefore drops their unique honesty strings. That
was true before the split. This packet does not retarget live routes.

Verification:

- `cargo +stable check -p poet --lib`: passed
- `cargo +stable test -p poet --lib workflow_panels::`: 2 passed
- product integrity: 10 passed (ceiling held)
- surface inventory: 1 passed
- `RUSTUP_TOOLCHAIN=stable NO_COLOR=true trunk build`: success
- scoped rustfmt (`--edition 2021`) on the new workflow-panel files: passed
- interactive browser click-UAT was not re-run

POET no longer has an implementation file over 1,400 lines. Files still
over 1,200: `container_views_ext.rs` (1,387), `native_daemon.rs` (1,369),
`command_palette/commands.rs` (1,231), `container_views.rs` (1,227). Do
not convert the view cluster via `pub use … build_*_view`. Do not close
Review Gate A. Do not start `AST-*`. `PFT-03` remains owner/captain
selection. `native_daemon.rs` is `D4` and stays untouched until its
transport contract is the accepted packet.

### `RM-10` - POET live container views decomposition

**Complete** (2026-09-05). `browser/container_views.rs` (1,227 lines at
split) is now `browser/container_views/` with the live
`build_doc_view` / `build_sheet_view` / `build_graph_view` /
`build_ontology_view` / `build_pulse_view` names preserved.

Source-to-destination map:

| Old | New |
|---|---|
| CML HyperDoc chrome | `container_views/doc.rs` (295) |
| Visual toolbar + gazetteer | `container_views/doc_toolbar.rs` (274) |
| Visual/Markdown/RDF tabs | `container_views/doc_switcher.rs` (70) |
| Spreadsheet + formula engine | `container_views/sheet.rs` (428) |
| SPARQL explorer | `container_views/graph.rs` (96) |
| Ontology stats / COP terms | `container_views/ontology.rs` (62) |
| Pulse ledger | `container_views/pulse.rs` (71) |
| Public glob re-exports | `container_views/mod.rs` (19) |

Behavior, ABI, and allocation: unchanged. Live container routes still
call `browser::container_views::build_doc_view` (and siblings).
Re-exports are `pub use doc::*` (and siblings), **not**
`pub use … build_*_view`, so `GENERIC_DELEGATION_CEILING` stays 112.

Verification:

- `cargo +stable check -p poet --lib`: passed
- `cargo +stable test -p poet --lib container_views::`: 2 passed
- product integrity: 10 passed (ceiling held)
- surface inventory: 1 passed
- `RUSTUP_TOOLCHAIN=stable NO_COLOR=true trunk build`: success; wasm
  contains `doc-view-switcher`, `doc-editor`, `RDF-Star (N-Quins)`,
  `never owl:Thing`, and `Topics must be poet/`
- scoped rustfmt (`--edition 2021`) on the new container-view files: passed
- interactive browser click-UAT was not re-run

Next POET files over 1,200: `container_views_ext.rs` (1,387),
`native_daemon.rs` (1,369), `command_palette/commands.rs` (1,231). Do
not close Review Gate A. Do not start `AST-*`. `PFT-03` remains
owner/captain selection. `native_daemon.rs` is `D4`.

### `RM-11` - POET extra container views decomposition

**Complete** (2026-09-05). `browser/container_views_ext.rs` (1,387 lines
at split) is now `browser/container_views_ext/` with the former public
`build_*_view` names preserved.

Source-to-destination map:

| Old | New |
|---|---|
| Library browser | `container_views_ext/library.rs` (152) |
| Aura + LaTeX | `container_views_ext/canvas_media.rs` (140) |
| Health stub + anatomy | `container_views_ext/health.rs` (155) |
| WebView + WebRTC | `container_views_ext/comm.rs` (198) |
| Finance | `container_views_ext/finance.rs` (111) |
| Vision + listen | `container_views_ext/senses.rs` (138) |
| Triad + portal | `container_views_ext/compute.rs` (194) |
| Slide + 3D + subcanvas | `container_views_ext/spatial.rs` (179) |
| Multimodal chips | `container_views_ext/chips.rs` (160) |
| Public glob re-exports | `container_views_ext/mod.rs` (26) |

Behavior, ABI, and allocation: unchanged. Re-exports are `pub use
library::*` (and siblings), **not** `pub use … build_*_view`, so
`GENERIC_DELEGATION_CEILING` stays 112.

Honest unused-module note: live container routes already use
`specialist_persist` / `local_container_views` for these occupant
types. `container_views_ext` remains a public API with no in-crate
callers; that was true before the split. This packet does not retarget
live routes.

Verification:

- `cargo +stable check -p poet --lib`: passed
- `cargo +stable test -p poet --lib container_views::`: 2 passed
- product integrity: 10 passed (ceiling held)
- surface inventory: 1 passed
- `RUSTUP_TOOLCHAIN=stable NO_COLOR=true trunk build`: success
- scoped rustfmt (`--edition 2021`) on the new ext files: passed
- interactive browser click-UAT was not re-run

POET files still over 1,200: `native_daemon.rs` (1,369) and
`command_palette/commands.rs` (1,231). `container_inline_views.rs` is
1,016 (under the 1,200 trigger). Do not close Review Gate A. Do not
start `AST-*`. `PFT-03` remains owner/captain selection.
`native_daemon.rs` is `D4`.

## Acceptance record

Each completed packet records:

- baseline commit and pre-existing worktree changes;
- source-to-destination symbol/module map;
- public API, ABI, allocation, and behavior impact;
- exact compile, test, build, and browser evidence;
- warnings introduced, removed, and still unrelated;
- resulting file sizes and the next triggered decomposition.
