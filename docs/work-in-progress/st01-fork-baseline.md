# ST-01 — Local App Fork & Licence/Asset Baseline: GridCraft
_Date: 2026-10-11 | Branch: `0.0.40.8-dev` | Status: Complete Fork Baseline_

This document delivers the normative attribution, rights, asset inventory, and workspace provenance for the initial application fork (`poet-gridcraft`) in accordance with Playbook §18.4 and Implementation Plan ST-C4.

---

## 1. Upstream Provenance & Identity

- **Upstream Source**: `C:\github\storytold\gridcraft-main`
- **Source Repository**: `storytold/gridcraft` (main branch)
- **Snapshot Commit Head**: `90b3c41eec691797129303603d4073269aebab73`
- **Head Date**: `2026-10-10T19:59:42Z`
- **Commit Count in Snapshot History**: 148 commits (verified via `contributors/contributors.json`)
- **Primary Authors & Contributors**: `echelon` (46 commits, 81,717 additions), `switch87`, `h-a-n-a-s-h-i`, `justinjohn0306`
- **Destination Fork**: `C:\github\poet-gridcraft` (local independent workspace)

---

## 2. Licensing & Rights Baseline

- **Declared License**: Dual-licensed **MIT OR Apache-2.0**
- **License Files**:
  - `LICENSE-MIT` (1,101 bytes) — standard MIT License
  - `LICENSE-APACHE` (10,174 bytes) — Apache License Version 2.0
  - `NOTICE` (742 bytes) — Storytold GridCraft attribution and copyright notices
- **Rights Classification**: Open-source dual-licensed code. No proprietary reference assets or trademark encumbrances present in the core spreadsheet engine.
- **Third-Party Dependencies**: Standard open-source Rust crates (`quick-xml` under MIT, `zip` under MIT, `image` under MIT/Apache-2.0, `egui` 0.36 under MIT/Apache-2.0).

---

## 3. Shipped Assets & Fonts

- **Assets Directory**: `assets/` (bundled icons, theme definition files, fallback spreadsheet templates).
- **Font Dependencies**: Uses standard bundled rasterized fallback fonts via egui / kittest; optional external craft-fonts are not required for core calculation, XLSX import/export, or CLI operation.
- **Integrity**: All bundled assets are verified and attributed under `ATTRIBUTION.md`.

---

## 4. Workspace Architecture

The fork workspace is completely self-contained with no relative path dependencies pointing outside its directory tree:

```toml
[workspace]
resolver = "3"
members = ["crates/*", "apps/*", "xtask"]
default-members = ["crates/*", "apps/gridcraft", "apps/gridcraft-cli", "xtask"]
```

### Member Crates:
1. `crates/core` (`gridcraft-core`): Fundamental types, cell coordinates, formatting, workbook models.
2. `crates/numfmt` (`gridcraft-numfmt`): Number formatting engine (Excel format string parser and renderer).
3. `crates/formula` (`gridcraft-formula`): Formula lexer, AST, parser, and Excel formula dialect evaluator.
4. `crates/functions` (`gridcraft-functions`): Standard library of spreadsheet functions (Math, Statistical, Financial, Text, Date, Logic).
5. `crates/model` (`gridcraft-model`): Multi-sheet document model, row/col metadata, styles, cell storage.
6. `crates/calc` (`gridcraft-calc`): Dependency graph, topological sort, circular reference detection, incremental recalc engine.
7. `crates/xlsx` (`gridcraft-xlsx`): OpenXML XLSX importer and exporter.
8. `crates/chart` (`gridcraft-chart`): Chart definition models and data series mappings.
9. `crates/engine` (`gridcraft-engine`): High-level headless command execution and document state manager.
10. `crates/ui-egui` (`gridcraft-ui-egui`): Desktop/native egui interface and virtualized sheet renderer.
11. `crates/mcp` (`gridcraft-mcp`): Model Context Protocol bridge for AI tool calling.
12. `crates/pdf` (`gridcraft-pdf`): PDF export renderer.
13. `apps/gridcraft`: Main desktop application binary.
14. `apps/gridcraft-cli`: Headless CLI evaluation and XLSX conversion utility.
15. `xtask`: Build automation, test runner, and packaging tasks.

---

## 5. Verification & Acceptance Criteria

- [x] Parent path and destination verified before copy.
- [x] Attributed code, assets, `Cargo.lock`, `NOTICE`, and license files preserved without modifications.
- [x] Upstream snapshot digest and provenance recorded (`90b3c41eec691797129303603d4073269aebab73`).
- [x] Independent workspace resolution confirmed (no parent-relative crate dependencies).
- [x] Ready for `ST-19` app adapter wiring against `qualia-app-contract`.

---

## 6. WordCraft Fork (`poet-wordcraft`) Baseline

- **Upstream Source**: `C:\github\storytold\wordcraft-main`
- **Destination Fork**: `C:\github\poet-wordcraft` (initialized local git workspace)
- **Declared License**: Dual-licensed **MIT OR Apache-2.0** (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` preserved)
- **Workspace Resolution**: Verified via `cargo metadata --manifest-path C:\github\poet-wordcraft\Cargo.toml --no-deps --format-version 1` (exit 0)
- **Crate Graph (16 crates)**: `control-key`, `doc`, `docbin`, `docx`, `engine`, `fonts`, `formats`, `geom`, `layout`, `mcp`, `metafile`, `pdf`, `proof`, `render`, `ui-egui`, `zotero`
- **Portability**: All crate paths reside entirely within `C:\github\poet-wordcraft\crates/` with zero dependency on external monorepo paths.

