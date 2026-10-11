# ST-00 (d) — Feature/command coverage inventory: VectorCraft, PhotoCraft, LightCraft, FilmCraft

Subunit (d) of packet ST-00 per `POET_LOWER_COST_AGENT_EXECUTION_PLAYBOOK_2026-09-04.md` §18.3: *"inventory commands, UI-only handlers, gestures, formats, payloads, device hooks and real status providers for each app."*

**Method:** pure source reading. No cargo/git/build commands were run. All command IDs were extracted from actual registration code (`cmd!` macros, `CommandSpec::new(...)`, `specs()` modules, `UI_COMMANDS` arrays), cross-referenced with `ROADMAP.md` and architectural documents.

**Provenance (all four):** source snapshots, **no VCS history** (no `.git`). Embedded revision identifiers extracted from `contributors/contributors.json` (generated stats files). Licence for every row below unless noted: **MIT OR Apache-2.0** (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` at each repository root; brand logos/wordmarks under `docs/brand/` are proprietary trademarks, not open-licensed).

**Coverage-row schema (§18.3 verbatim):** source repo/revision/file/symbol · feature/command/gesture ID · read/mutation/external effect · strict-parameter-schema state · target/selection dependencies · native/WASM support · UI destination · Qualia primitive owner or promotion need · fidelity limitation · licence/provenance · synthetic fixture · required assertion · packet/subunit · evidence/state.

To preserve readability across tables, row-invariant fields are recorded once per app header: `lic/prov` = MIT/Apache-2.0 snapshot provenance; `packet` = ST-00(d); `schema` = `params` documentation string (loose doc-string schema unless noted as strict); `fixture` = synthetic unit/corpus fixture cited per app.

---

## VectorCraft (`C:\github\storytold\vectorcraft-main`)

### Baseline facts

| Field | Value | Evidence |
|---|---|---|
| Workspace package | version `0.8.0`, edition `2024`, `rust-version = "1.95"` (MSRV), `resolver = "3"` | `Cargo.toml:6-11` |
| Licence | `MIT OR Apache-2.0` (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`); ArtCraft brand trademarks in `docs/brand/` excepted | `Cargo.toml:9`, `NOTICE` |
| Declared repo / embedded revision | `github.com/storytold/vectorcraft`; snapshot head `866ea5878a7f6db7dd3d6b1287f9a9bb3a241c33` (branch `main`, head_date `2026-10-10T00:04:42Z`, 906 commits) | `Cargo.toml:11`; `contributors/contributors.json:2-7` |
| Workspace members (27) | `crates/*` (23 crates: `geom, color, doc, pathops, brush, trace, text, effects, three-d, render, svg, pdf, format, tools, engine, ui-egui, mcp, testkit, cad, eps, metafile, plugins, affinity`), `apps/*` (`vectorcraft, vectorcraft-cli, vectorcraft-web`), `xtask` | `Cargo.toml:3-4, 15-38` |
| Binaries | `vectorcraft` (desktop eframe GUI with loopback TCP control channel on port 7979); `vectorcraft-cli` (`info, convert, run, commands, mcp, perf, bench`); `vectorcraft-web` | `apps/vectorcraft/src/main.rs`; `apps/vectorcraft-cli/src/main.rs` |
| WASM mechanism | `apps/vectorcraft-web`: eframe `WebRunner` on wgpu (WebGPU / WebGL2 fallback), `wasm-bindgen`/`web-sys`, Trunk runner (`Trunk.toml`); pure-Rust WASM plugin sandboxing via `wasmi 2.0` | `Cargo.toml:53`, `apps/vectorcraft-web/Cargo.toml` |
| UI toolkit | egui `0.36`, eframe `0.36`, egui_kittest `0.36`; raster `vello_cpu 0.2` (f32_pipeline), kurbo `0.13`, peniko `0.6`, usvg `0.48`, arboard `3.6` | `Cargo.toml:40-60` |
| Control channel | Localhost TCP JSON lines on port 7979 (`crates/ui-egui/src/control.rs:60-350`), methods: `engine.execute`, `ui.screenshot`, `ui.render`, `ui.pointer`, `ui.click`, `ui.key`, etc. | `AGENTS.md:51-53`, `control.rs` |
| MCP | `crates/mcp` stdio JSON-RPC server; bridges headless or running app (`vectorcraft-cli mcp`) | `crates/mcp/src/lib.rs`, `docs/mcp.md` |
| Test commands | `cargo xtask ci` (fmt, clippy -D warnings, tests, cleanroom, assets, layers, wasm); `cargo xtask perf` | `xtask/src/main.rs`, `AGENTS.md:48` |
| Parity & roadmap | 348/375 menu items wired (92.8%), 89/92 tools, 44/50 Illustrator effects, 28/57 Photoshop raster filters | `ROADMAP.md:21-28`, `docs/target-app-parity.md` |

### Constraints recorded (AGENTS.md / CLAUDE.md)

- **Clean-room (absolute):** Never run, automate, inspect, or screenshot Adobe Illustrator; no Adobe binaries/plugins/ICC profiles/procsets/prologs copied; behavioural reference from public specs (ISO 32000-2, PostScript Language Reference) and user-supplied owned test files. `cargo xtask cleanroom` enforces.
- **Assets (absolute):** No Adobe icons, cursors, swatches, or artwork; all assets MIT/Apache, CC0 or OSI in `ASSETS.md`; fonts live in `storytold/craft-fonts` via `CRAFT_FONTS_DIR`, never committed.
- **Never crash:** `#![forbid(unsafe_code)]`; no `unwrap/expect/panic!/unreachable!/todo!/unimplemented!` in shipped code; `vectorcraft_engine::guard` catches panics, rolls back document, and reports `EngineError`.
- **Layering:** L0 (`geom, color, doc, pathops...`) → L5 `engine` → L6 `ui-egui, mcp` enforced by `cargo xtask layers`.

### Command registry — 400+ `CommandSpec` (in `crates/engine/src/cmd/`)

`CommandSpec` struct: `{id, label, menu, shortcut, params, enabled, run, journal}`. Assembled in `crates/engine/src/cmd/mod.rs:191-283` (`command_specs()`).

| Family | Source seam | Representative Registered IDs | Effect | Target/deps | Fidelity / notes |
|---|---|---|---|---|---|
| Edit & Clipboard | `cmd/{edit,clipboard,cut}.rs` | `edit.undo`, `edit.redo`, `edit.cut`, `edit.copy`, `edit.paste`, `edit.pasteInFront`, `edit.pasteInBack`, `edit.pasteInPlace`, `edit.pasteOnAllArtboards`, `edit.clear` | M/X (clipboard arboard) | selection, doc | System clipboard synchronization, paste-in-place retains coordinates |
| Document & File I/O | `cmd/{fileio,newdoc,newart,docsetup,docinfo,recovery}.rs` | `document.new`, `document.open`, `document.save`, `document.saveAs`, `document.saveCopy`, `document.revert`, `document.close`, `document.setup`, `document.colorMode.rgb`, `document.colorMode.cmyk`, `document.info`, `recovery.autosave` | M/X (file I/O) | session, active doc | Support for `.vectorcraft`, legacy `.drawcraft`, `.ai` stream reading |
| Object Create & Shapes | `cmd/{create,draw2,freeform,buildcmds}.rs` | `shape.rectangle`, `shape.roundedRectangle`, `shape.ellipse`, `shape.polygon`, `shape.star`, `shape.flare`, `shape.line`, `shape.arc`, `shape.spiral`, `shape.rectGrid`, `shape.polarGrid`, `shape.point`, `shape.bezier` | M | doc, active artboard | Geometry builders create editable vectors with transform anchors |
| Object Transform & Arrange | `cmd/{object,xform,distortcmds}.rs` | `object.arrange.bringToFront`, `object.arrange.bringForward`, `object.arrange.sendBackward`, `object.arrange.sendToBack`, `object.group`, `object.ungroup`, `object.lock`, `object.unlockAll`, `object.hide`, `object.showAll`, `transform.move`, `transform.rotate`, `transform.scale`, `transform.reflect`, `transform.shear`, `transform.each`, `transform.resetBoundingBox` | M | selection | Bounding-box transformations with preserved transform origins |
| Paths & Path Operations | `cmd/{path,pathops,shaper,expand}.rs` | `path.join`, `path.average`, `path.outlineStroke`, `path.simplify`, `path.smooth`, `path.addAnchorPoints`, `path.removeAnchorPoints`, `path.reverseDirection`, `pathfinder.unite`, `pathfinder.minusFront`, `pathfinder.intersect`, `pathfinder.exclude`, `pathfinder.divide`, `pathfinder.trim`, `pathfinder.merge`, `pathfinder.crop`, `pathfinder.outline`, `pathfinder.minusBack`, `shape.expand`, `shape.expandAppearance` | M | path selection | Kurbo-backed path operations and exact boolean topology |
| Appearance, Paint & Styles | `cmd/{paint,stroke,appearance,style,swatch,gradient,opacitymask}.rs` | `paint.fillColor`, `paint.strokeColor`, `paint.swapFillStroke`, `paint.defaultFillStroke`, `stroke.weight`, `stroke.cap`, `stroke.join`, `stroke.dashPattern`, `stroke.align`, `appearance.newFill`, `appearance.newStroke`, `appearance.clear`, `appearance.duplicate`, `style.apply`, `swatch.new`, `gradient.linear`, `gradient.radial`, `mask.makeOpacityMask`, `mask.releaseOpacityMask` | M | target appearance | Hierarchical appearance stacks (multiple fills/strokes per node) |
| Color Management & Recolor | `cmd/{colorcmds,colormgmt,colortheme,swatchlib}.rs` | `color.recolorArtwork`, `color.adjustColors`, `color.invertColors`, `color.overprintFill`, `color.overprintStroke`, `color.proofColors`, `colormgmt.assignProfile`, `colormgmt.convertProfile` | M | selection, doc | ICC profile conversions via `vectorcraft-color` |
| Type & Typography | `cmd/{typecmd,typemenu,textedit,inline,textstyles,fonts,fontfiles,threads,textwrap}.rs` | `type.newPoint`, `type.newArea`, `type.onPath`, `type.size`, `type.fontFamily`, `type.fontStyle`, `type.kerning`, `type.tracking`, `type.baselineShift`, `type.createOutlines`, `type.threadFrames`, `type.breakThreads`, `type.wrap`, `type.unwrap` | M | text story / selection | OpenType layout and HarfBuzz shaping; text frame threading |
| Layers & Artboards | `cmd/{layer,layerpanel,slices,views}.rs` | `layer.new`, `layer.delete`, `layer.duplicate`, `layer.rename`, `layer.lock`, `layer.visibility`, `layer.move`, `artboard.new`, `artboard.delete`, `artboard.resize`, `artboard.reorder`, `artboard.rearrange`, `slices.make`, `slices.divide` | M | layer hierarchy, doc | Multiple artboards with independent pixel/pt dimensions |
| Effects & Raster Filters | `cmd/{effectcmd,rasterfx,halftone}.rs` | `effect.dropShadow`, `effect.feather`, `effect.innerGlow`, `effect.outerGlow`, `effect.gaussianBlur`, `effect.unsharpMask`, `effect.halftone`, `effect.revolve3d`, `effect.warpArc` | M | target appearance | Parametric live effects evaluated during compositing pipeline |
| Export & Print | `cmd/{webexport,print,printpresets,printtiling,assets,css,package}.rs` | `export.svg`, `export.pdf`, `export.eps`, `export.png`, `export.jpg`, `export.screens`, `export.css`, `print.dialog`, `print.execute`, `package.collect` | M/X (file write, print spool) | doc, export plan | Multi-asset export for web/screens, packaging linked assets |
| Automation & Plugins | `cmd/{plugin,variablecmds,graph}.rs` | `plugin.load`, `plugin.run`, `variables.import`, `variables.bind`, `graph.data`, `graph.type` | M/X | workspace, doc | Wasmi-sandboxed plugins; CSV/XML dataset variable binding |

### UI-only commands (`crates/ui-egui/src/menus.rs`)

Handled in `UI_COMMANDS` table:
`app.language`, `file.open`, `file.save`, `file.place`, `file.openRecent1..10`, `view.goto1..10`, `select.recall1..25`, `type.recentFont1..15`, `file.clearRecent`, `type.findFont`, `ui.missingFontsDialog`, `ui.findFontsInFolder`, `file.recentFiles`, `file.export.svg`, `file.export.png`, `file.exportForScreens`, `file.documentSetup`, `file.newDialog`, `app.home`, `edit.preferences`, `edit.keyboardShortcuts`, `shortcuts.set/list/conflicts/reset/preset/export/import`, `view.outline`, `view.pixelPreview`, `view.trimView`, `view.cornerWidget`, `view.snapToPixel`, `view.textThreads`, `view.gradientAnnotator`, `type.hiddenCharacters`, `type.bold`, `type.italic`.

### Gestures & pointer tools

- **Selection & Direct Selection:** Click, marquee crossing, Alt-drag duplicate, Shift-add selection, corner widget radius drag.
- **Pen & Vector Creation:** Pen click-to-anchor, drag-for-bezier-handles, Alt-break-tangent, close path on start node, Curvature tool auto-smooth.
- **Transform Tools:** Interactive rotation center-point placement, scale dragging with aspect locking (Shift), Shear drag.
- **Live Paint & Shape Builder:** Drag across intersecting path segments to unite or delete (Alt-drag).
- **Interactive Gradients:** On-canvas annotator bar with draggable color stops, angle handle, and midpoint slider.

### Formats, device hooks & status providers

- **Formats:** Read: `.vectorcraft`, `.drawcraft`, `.svg`, `.svgz`, `.pdf` (interactive/place), `.ai` (PDF stream), `.dxf`, `.eps`. Write: `.vectorcraft`, `.svg`, `.svgz`, `.pdf`, `.eps`, `.png`, `.jpg`, `.webp`, `.dxf`.
- **Device hooks:** System clipboard (`arboard`), drawing tablet pen pressure/tilt monitor, local print spooler.
- **Status providers:** Real session bounds, memory metrics, undo/redo history depth, active layer/artboard indices, link integrity check (`link_watch.rs`).

---

## PhotoCraft (`C:\github\storytold\photocraft-main`)

### Baseline facts

| Field | Value | Evidence |
|---|---|---|
| Workspace package | version `0.6.0`, edition `2024`, `rust-version = "1.95"` (MSRV), `resolver = "3"` | `Cargo.toml:6-11` |
| Licence | `MIT OR Apache-2.0` (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`); brand trademarks in `docs/brand/` excepted | `Cargo.toml:9`, `NOTICE` |
| Declared repo / embedded revision | `github.com/storytold/photocraft`; snapshot head `9c427a7a2c792d326939f5b27b1d814a7c6321a7` (branch `main`, head_date `2026-10-10T00:34:31Z`, 1054 commits) | `contributors/contributors.json:2-7` |
| Workspace members (27) | `crates/*` (24 crates: `geom, color, cms, raster, doc, ops, psd, codecs, raw, affinity, compose, gpu, paint, algo, io, text, vector, format, automation, plugins, engine, ui-egui, testkit, tablet`), `apps/*` (`photocraft, photocraft-cli, photocraft-web`), `xtask` | `Cargo.toml:3-4, 14-38` |
| Binaries | `photocraft` (desktop eframe/wgpu app with TCP control channel); `photocraft-cli` (`convert, info, run, batch, commands, mcp`); `photocraft-web` | `apps/photocraft/src/main.rs`; `apps/photocraft-cli/src/main.rs` |
| WASM mechanism | `apps/photocraft-web`: eframe `WebRunner` on wgpu with WebGL2/Glow fallback, Trunk runner (`Trunk.toml`), `wasm-bindgen`/`web-sys` | `Cargo.toml:51-53`, `apps/photocraft-web/Cargo.toml` |
| UI toolkit | egui `0.36`, eframe `0.36`, egui_kittest `0.36`; wgpu, image `0.25`, zune-jpeg `0.5`, zune-core `0.5`, rfd `0.17`, rayon `1` | `Cargo.toml:40-60` |
| Control channel | Localhost TCP JSON-lines socket (`docs/control-protocol.md`, `crates/ui-egui/src/control.rs`) | `AGENTS.md:12`, `control.rs` |
| MCP | `photocraft-automation` stdio MCP JSON-RPC server; bridges headless or running app (`photocraft-cli mcp`) | `crates/automation/src/lib.rs` |
| Test commands | `cargo xtask ci` (fmt, clippy -D warnings, tests, layers, wasm); `cargo xtask corpus --all` (photocraft-corpus test oracles) | `AGENTS.md:22, 42` |
| Parity & roadmap | 628/628 menus implemented (100% presence), 53/68 tools, 30/35 panels, 17 file formats | `ROADMAP.md:20-28`, `docs/target-app-parity.md` |

### Constraints recorded (AGENTS.md / CLAUDE.md)

- **Pure Rust:** No JavaScript, TypeScript, Tauri, Electron, or webview wrappers. Native egui on desktop; Trunk WASM on web.
- **Never crash:** `#![forbid(unsafe_code)]` workspace-wide except isolated `photocraft-tablet/src/macos.rs` for AppKit pen event monitoring (`SAFETY:` documented). Non-test code strictly forbids `unwrap/expect/panic!`.
- **Layering:** L0 (`geom, color, cms, raster, psd, codecs, tablet`) → L1 `doc` → L2 `ops, paint, algo, text, vector` → L3 `compose, gpu, format` → L4 `io, plugins` → L5 `engine` → L6 `ui-egui, automation`.
- **Determinism:** CPU compositor (`compose`) serves as reference numerical oracle against GPU compositor (`gpu`).

### Command registry — 628 `CommandSpec` (in `crates/engine/src/commands.rs`)

`CommandSpec` struct: `{id, label, menu, shortcut, params, enabled, run, journal}`. Built via `commands::build()` aggregating ~40 command modules.

| Family | Source seam | Representative Registered IDs | Effect | Target/deps | Fidelity / notes |
|---|---|---|---|---|---|
| File Operations | `file_cmds.rs`, `exr_cmds.rs`, `document_preset_cmds.rs` | `file.new`, `file.open`, `file.save`, `file.saveAs`, `file.revert`, `file.close`, `file.closeAll`, `file.exportAs`, `file.quickExportPng`, `file.generateAssets`, `file.placeEmbedded`, `file.placeLinked`, `file.package` | M/X (file write) | active doc | Native `.pcraft` format and full PSD/PSB round-trip |
| Edit & History | `edit_cmds.rs`, `edit_menu_cmds.rs`, `history_cmds.rs`, `fill_cmds.rs` | `edit.undo`, `edit.redo`, `edit.cut`, `edit.copy`, `edit.copyMerged`, `edit.paste`, `edit.pasteInto`, `edit.pasteInPlace`, `edit.clear`, `edit.fill`, `edit.contentAwareFill`, `edit.stroke`, `edit.colorSettings`, `edit.purgeAll` | M/X (clipboard) | selection, active layer | COW tile surface snapshots in history stack |
| Transformations | `transform_cmds.rs`, `warp_cmds.rs`, `distort_cmds.rs` | `transform.free`, `transform.scale`, `transform.rotate`, `transform.skew`, `transform.distort`, `transform.perspective`, `transform.warp`, `transform.flipHorizontal`, `transform.flipVertical`, `transform.rotate180`, `transform.rotate90CW`, `transform.rotate90CCW`, `transform.contentAwareScale` | M | selection, layer | Affine & mesh-based resampling algorithms |
| Image Adjustments | `adjust_cmds.rs`, `adjust_params.rs` | `image.adjustments.brightnessContrast`, `image.adjustments.levels`, `image.adjustments.curves`, `image.adjustments.exposure`, `image.adjustments.vibrance`, `image.adjustments.hueSaturation`, `image.adjustments.colorBalance`, `image.adjustments.blackWhite`, `image.adjustments.photoFilter`, `image.adjustments.channelMixer`, `image.adjustments.colorLookup`, `image.adjustments.invert`, `image.adjustments.posterize`, `image.adjustments.threshold`, `image.adjustments.gradientMap`, `image.adjustments.selectiveColor` | M | layer pixels / mask | Destructive pixel adjustment or Smart Filter delegation |
| Canvas & Color Modes | `image_cmds.rs`, `mode_cmds.rs`, `multichannel_cmds.rs` | `image.imageSize`, `image.canvasSize`, `image.crop`, `image.trim`, `image.revealAll`, `image.duplicate`, `image.mode.rgb`, `image.mode.cmyk`, `image.mode.lab`, `image.mode.grayscale`, `image.mode.bitmap`, `image.mode.indexed`, `image.depth.8`, `image.depth.16`, `image.depth.32` | M | doc | Resolution resampling, color space transformations via LittleCMS-equivalent `photocraft-cms` |
| Layer Management | `layer_menu_cmds.rs`, `layer_multi_cmds.rs`, `layer_copy_cmds.rs`, `layer_style.rs` | `layer.new`, `layer.duplicate`, `layer.delete`, `layer.rename`, `layer.group`, `layer.ungroup`, `layer.arrange.bringToFront`, `layer.arrange.sendToBack`, `layer.mergeDown`, `layer.mergeVisible`, `layer.flatten`, `layer.layerMask.revealAll`, `layer.layerMask.hideAll`, `layer.createClippingMask`, `layer.releaseClippingMask`, `layer.layerStyle.options` | M | layer selection | Full blend-mode equations, clipping masks, and layer effects |
| Adjustment & Fill Layers | `gradient_fill_cmds.rs`, `solid_fill_cmds.rs`, `pattern_cmds.rs` | `layer.newAdjustmentLayer.*` (16 kinds), `layer.newFillLayer.solidColor`, `layer.newFillLayer.gradient`, `layer.newFillLayer.pattern` | M | layer tree | Non-destructive adjustment layer evaluation |
| Selection & Masks | `selection_cmds.rs`, `select_extra_cmds.rs`, `smartselect_cmds.rs`, `cutout_cmds.rs` | `select.all`, `select.deselect`, `select.inverse`, `select.allLayers`, `select.colorRange`, `select.subject`, `select.sky`, `select.modify.border`, `select.modify.smooth`, `select.modify.expand`, `select.modify.contract`, `select.modify.feather`, `select.grow`, `select.similar`, `select.transformSelection`, `select.quickMaskMode` | M | doc selection mask | 8-bit grayscale alpha masks with feathering |
| Filters & Smart Filters | `filters.rs`, `filters_ext.rs`, `gallery_cmds.rs` | `filter.filterGallery`, `filter.gaussianBlur`, `filter.motionBlur`, `filter.radialBlur`, `filter.smartBlur`, `filter.surfaceBlur`, `filter.unsharpMask`, `filter.highPass`, `filter.liquify`, `filter.vanishingPoint`, `filter.cameraRawFilter`, `filter.convertSmartFilters` | M | pixel layer / smart object | GPU-accelerated and CPU reference filter kernels |
| Painting & Retouching | `paint_cmds.rs`, `brush_cmds.rs`, `brush_preset_cmds.rs`, `eraser_cmds.rs`, `retouch_cmds.rs`, `stamp_cmds.rs` | `paint.stroke`, `brush.setSize`, `brush.setHardness`, `brush.setOpacity`, `brush.setFlow`, `brush.setSpacing`, `retouch.cloneStamp`, `retouch.healingBrush`, `retouch.spotHealing`, `retouch.patch`, `retouch.dodge`, `retouch.burn`, `retouch.sponge`, `retouch.blur`, `retouch.sharpen`, `retouch.smudge` | M | target layer | High-frequency brush engine with jitter, dynamics, and pressure |
| Type & Vector | `type_cmds.rs`, `type_styles_cmds.rs`, `vector_cmds.rs`, `path_edit_cmds.rs` | `type.newPoint`, `type.newParagraph`, `type.commit`, `type.cancel`, `type.createWorkPath`, `type.convertShape`, `vector.newShape`, `vector.pathOperations` | M | text/vector layers | HarfBuzz font shaping and vector path rasterization |
| Animation & Export | `timeline_cmds.rs`, `video_cmds.rs`, `slice_cmds.rs`, `web_cmds.rs`, `print_cmds.rs` | `timeline.createFrameAnimation`, `timeline.createVideoTimeline`, `slice.divide`, `web.saveForWeb`, `print.execute` | M/X | doc, frame sequence | Frame/video timeline animation and web slice export |

### UI-only commands (`crates/ui-egui/src/`)

Workspace toggles, screen modes (Standard, Full Screen with Menu Bar, Full Screen), ruler/guide display, snap settings, navigator zoom/pan, panel docking, and color picker flyout dialogs.

### Gestures & pointer tools

- **Selection Tools:** Rectangular/Elliptical marquee, Lasso, Polygonal Lasso, Magnetic Lasso (edge-snapping), Object Selection, Magic Wand.
- **Painting & Brushes:** Freehand stroke interpolation with tablet pressure/tilt, dual-brush blending, color sampling on Alt-click.
- **Retouch Gestures:** Clone Stamp source-point sampling (Alt-click), Patch tool area lasso and destination drag.
- **Vector Pen & Path Tools:** Bezier path drawing with tangent handle dragging.
- **Canvas Navigation:** Space-drag for Hand pan, Cmd-Space-drag for scrubby zoom, Rotate View canvas angle gesture.

### Formats, device hooks & status providers

- **Formats:** Read: PSD, PSB, TIFF, PNG, JPEG, GIF, BMP, WebP, EXR, HDR, TGA, RAW (DNG, CR2, NEF, ARW), `.pcraft`. Write: PSD, PSB, TIFF, PNG, JPEG, WebP, EXR, `.pcraft`.
- **Device hooks:** Tablet pressure/tilt (`photocraft-tablet`), wgpu compositor (`photocraft-gpu`), color management engine (`photocraft-cms`).
- **Status providers:** GPU memory pool allocations, tile cache eviction metrics, active layer histograms, document color profile validation.

---

## LightCraft (`C:\github\storytold\lightcraft-main`)

### Baseline facts

| Field | Value | Evidence |
|---|---|---|
| Workspace package | version `0.5.0`, edition `2024`, `rust-version = "1.90"` (MSRV), `resolver = "3"` | `Cargo.toml:6-10` |
| Licence | `MIT OR Apache-2.0` (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`) | `Cargo.toml:8`, `NOTICE` |
| Declared repo / embedded revision | `github.com/storytold/lightcraft`; snapshot head `c545cc5e842a74d65f32187d9fdee86ada3099de` (branch `main`, head_date `2026-10-10T05:40:18Z`, 1033 commits) | `contributors/contributors.json:2-7` |
| Workspace members (27) | `crates/*` (24 crates: `denoise-core, denoise, geom, faces, sysmem, color, raster, tiff, raw, codecs, meta, develop, scenes, pipeline, gpu, catalog, preview, merge, engine, ui-egui, mcp, fetch, heif, segment`), `apps/*` (`lightcraft, lightcraft-cli, lightcraft-web`), `xtask` | `Cargo.toml:3, 14-38` |
| Binaries | `lightcraft` (desktop eframe app); `lightcraft-cli` (`catalog, develop, export, mcp, bench`); `lightcraft-web` | `apps/lightcraft/src/main.rs`; `apps/lightcraft-cli/src/main.rs` |
| WASM mechanism | `apps/lightcraft-web`: eframe `WebRunner` on wgpu/WebGL2, Trunk runner (`Trunk.toml`) | `apps/lightcraft-web/Cargo.toml` |
| UI toolkit | egui `0.36`, eframe `0.36`, egui_kittest `0.36`; wgpu, rayon `1`, quick-xml `0.38`, miniz_oxide `0.8`, image `0.25`, zune-jpeg `0.5` | `Cargo.toml:40-65` |
| Control & MCP | `lightcraft-mcp` stdio JSON-RPC server; engine headless driving via CLI and JSON IPC | `crates/mcp/src/lib.rs` |
| Test commands | `cargo xtask ci` (fmt, clippy -D warnings, tests, layers, wasm); `cargo test -p lightcraft-engine` | `xtask/src/main.rs` |
| Parity & roadmap | Non-destructive raw photography workflow matching Adobe Lightroom Classic (Library, Develop, Export) | `ROADMAP.md:1-40` |

### Constraints recorded (AGENTS.md / CLAUDE.md)

- **Immutable Originals:** Source RAW and asset files are strictly read-only; all edits stored as parametric recipes in database / XMP sidecars.
- **Catalogue Integrity:** ACID transaction semantics on SQLite / OmaStore catalogs; torn-write recovery mechanisms.
- **Never crash:** `#![forbid(unsafe_code)]`; no panics in shipped code. Clean error propagation across raw decoding and export pipelines.

### Command registry — 31 command modules (in `crates/engine/src/cmd/`)

`CommandSpec` struct: `{id, label, menu, shortcut, params, enabled, run, journal, explicit_targets}`. Assembled in `crates/engine/src/cmd/mod.rs:124-161`.

| Family | Source seam | Representative Registered IDs | Effect | Target/deps | Fidelity / notes |
|---|---|---|---|---|---|
| Library & Navigation | `cmd/library.rs`, `cmd/browse.rs`, `cmd/cull.rs` | `library.select`, `library.selectAll`, `library.deselectAll`, `library.nextPhoto`, `library.prevPhoto`, `library.setRating`, `library.setFlag`, `library.setColorLabel`, `library.rotateLeft`, `library.rotateRight`, `library.flipHorizontal`, `library.flipVertical`, `library.createVirtualCopy`, `library.delete` | M | photo selection, catalog | Grid, Loupe, Compare, and Survey view selection state |
| Collections & Albums | `cmd/organize.rs`, `cmd/manage.rs` | `album.create`, `album.delete`, `album.rename`, `album.addPhotos`, `album.removePhotos`, `smartAlbum.create`, `smartAlbum.editCriteria`, `folder.synchronize`, `folder.locateMissing` | M | catalog metadata | Hierarchical albums and rule-based smart albums |
| Develop Tone & Presence | `cmd/develop.rs`, `cmd/curves.rs` | `develop.exposure`, `develop.contrast`, `develop.highlights`, `develop.shadows`, `develop.whites`, `develop.blacks`, `develop.texture`, `develop.clarity`, `develop.dehaze`, `develop.vibrance`, `develop.saturation`, `develop.whiteBalance`, `develop.temperature`, `develop.tint`, `develop.toneCurve`, `develop.toneCurveRgb` | M | active photo develop recipe | 32-bit floating point parametric tone processing |
| Color Mixer & Grading | `cmd/color.rs`, `cmd/lut_profiles.rs` | `develop.hsl.hue`, `develop.hsl.saturation`, `develop.hsl.luminance`, `develop.colorGrading.shadows`, `develop.colorGrading.midtones`, `develop.colorGrading.highlights`, `develop.colorGrading.balance`, `develop.lut.apply` | M | active photo | 8-channel HSL mixer, 3-way color grading wheels, 3D LUT profiles |
| Detail & Denoise | `cmd/denoise.rs` | `develop.sharpening.amount`, `develop.sharpening.radius`, `develop.sharpening.detail`, `develop.sharpening.masking`, `develop.noiseReduction.luminance`, `develop.noiseReduction.color`, `develop.aiDenoise.execute` | M/X (AI computation) | active photo | Pyramidal frequency sharpening and AI tensor denoise |
| Geometry & Crop | `cmd/develop.rs` | `crop.rect`, `crop.angle`, `crop.aspectRatio`, `geometry.upright`, `geometry.distortion`, `geometry.vertical`, `geometry.horizontal`, `geometry.rotate`, `geometry.scale` | M | active photo | Non-destructive crop and perspective correction |
| Parametric Masks | `cmd/masks.rs` | `mask.create.brush`, `mask.create.linearGradient`, `mask.create.radialGradient`, `mask.create.colorRange`, `mask.create.luminanceRange`, `mask.create.subject`, `mask.create.sky`, `mask.create.background`, `mask.invert`, `mask.delete` | M | photo develop recipe | AI subject/sky segmentation and vector/bitmap masks |
| Presets & History | `cmd/preset_files.rs`, `cmd/before.rs` | `preset.apply`, `preset.save`, `preset.delete`, `preset.importXmp`, `develop.copySettings`, `develop.pasteSettings`, `develop.history.restoreState`, `develop.reset` | M | photo develop recipe | Snapshot history and XMP preset interoperability |
| Face Detection & Tagging | `cmd/{face_detect,face_models,face_recognize}.rs` | `faces.detect`, `faces.recognize`, `faces.assignName`, `faces.ignore`, `faces.trainModel`, `faces.cluster` | M/X | catalog photos | Local neural embedding face cluster matching |
| Metadata & Keywords | `cmd/{metadata,keywords,xmp}.rs` | `metadata.setCaption`, `metadata.setTitle`, `metadata.setCopyright`, `keywords.add`, `keywords.remove`, `xmp.readSidecar`, `xmp.writeSidecar` | M/X (sidecar I/O) | photo selection | Bidirectional XMP/IPTC/EXIF synchronization |
| Export & Merge | `cmd/{export,merge}.rs` | `export.execute`, `export.batch`, `merge.hdr`, `merge.panorama`, `merge.hdrPanorama` | M/X (disk write) | photo selection | Multi-threaded RAW export and HDR/panoramic stitching |

### UI-only commands & gestures

- **UI commands:** Module switches (Library, Develop, Map, Print, Web), dual-monitor window split, filmstrip visibility, loupe zoom ratios (Fit, Fill, 1:1, 2:1).
- **Gestures:** Grid thumbnail multi-selection (Shift/Cmd click), Loupe pan-and-zoom dragging, Crop tool handles and horizon line leveler, Masking brush stroke drawing, Tone Curve control point drag, White Balance eyedropper pick.

### Formats, device hooks & status providers

- **Formats:** Read: DNG, CR2, CR3, NEF, ARW, ORF, RAF, RW2, TIFF, JPEG, HEIF/HEIC, Adobe Lightroom `.lrcat` SQLite catalogs. Write: JPEG, TIFF, DNG, PNG, XMP sidecars.
- **Device hooks:** Camera tethering preview (`camera_preview.rs`), GPU develop pipeline (`lightcraft-gpu`), face recognition worker (`lightcraft-faces`).
- **Status providers:** Background export progress, preview generation queue, camera profile discovery, missing files locator (`missing.rs`).

---

## FilmCraft (`C:\github\storytold\filmcraft-main`)

### Baseline facts

| Field | Value | Evidence |
|---|---|---|
| Workspace package | version `0.5.0`, edition `2024`, `rust-version = "1.95"` (MSRV), `resolver = "3"` | `Cargo.toml:6-10` |
| Licence | `MIT OR Apache-2.0` (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`) | `Cargo.toml:9`, `NOTICE` |
| Declared repo / embedded revision | `github.com/storytold/filmcraft`; snapshot head `7b6c134472287db4367fca80bcb498240ba36567` (branch `main`, head_date `2026-10-09T15:02:38Z`, 539 commits) | `contributors/contributors.json:2-7` |
| Workspace members (43) | `crates/*` (40 crates: `time, geom, color, bitstream, isobmff, matroska, mxf, cfb, mpegts, mpeg2v, ac3, ogg, h264, hevc, vp9, prores, dnx, apv, av1, aac, opus, h264enc, audio-dsp, frame, media, codecs, project, edit, interchange, text, captions, speech, tts, tts-text, render, gpu, scopes, export, format, engine, ui-egui, automation, platform, testkit`), `apps/*` (`filmcraft, filmcraft-cli, filmcraft-web`), `xtask` | `Cargo.toml:3-4, 14-58` |
| Binaries | `filmcraft` (desktop eframe app); `filmcraft-cli` (`render, info, export, mcp, perf`); `filmcraft-web` | `apps/filmcraft/src/main.rs`; `apps/filmcraft-cli/src/main.rs` |
| WASM mechanism | `apps/filmcraft-web`: eframe `WebRunner` on wgpu, Trunk runner (`Trunk.toml`), WebCodecs integration | `apps/filmcraft-web/Cargo.toml` |
| UI toolkit | egui `0.36`, eframe `0.36`, egui_kittest `0.36`; wgpu, pure-Rust demuxers and decoders (ISOBMFF, Matroska, MXF, MPEG-TS, ProRes, DNx, H.264, HEVC, AV1, VP9, AAC, Opus, AC-3) | `Cargo.toml:14-58` |
| Control & MCP | `filmcraft-automation` stdio MCP JSON-RPC server; loopback TCP control channel (`crates/ui-egui/src/control.rs`) | `crates/automation/src/lib.rs`, `control.rs` |
| Test commands | `cargo xtask ci` (fmt, clippy -D warnings, tests, layers, wasm); `cargo test -p filmcraft-engine` | `xtask/src/main.rs` |
| Parity & roadmap | Full non-linear video editing (NLE) matching Adobe Premiere Pro parity (Timeline, Program, Source, Bins, Audio, Scopes, Export) | `ROADMAP.md:1-50` |

### Constraints recorded (AGENTS.md / CLAUDE.md)

- **Exact Media Time:** Authoritative rational timebase in `filmcraft-time` (`TICKS_PER_SECOND = 254,016,000,000` ticks/sec); exact drop-frame 29.97 / 23.976 timecode arithmetic without cumulative float drift.
- **Never crash:** `#![forbid(unsafe_code)]`; non-destructive edit decision lists; no audio buffer underruns in playback thread; stream budgeting instead of whole-file caching.
- **Layering & Pure Codecs:** Pure-Rust demuxers and codecs; zero external C FFmpeg bindings.

### Command registry — 350+ `CommandSpec` (in `crates/engine/src/commands.rs`)

`CommandSpec` struct: `{id, label, menu, shortcut, params, enabled, run, journal}`. Built via `commands::build()` in `crates/engine/src/commands.rs:49-3940`.

| Family | Source seam | Representative Registered IDs | Effect | Target/deps | Fidelity / notes |
|---|---|---|---|---|---|
| Project & Sequence | `commands.rs:60-350`, `sequence_tools.rs` | `project.new`, `project.open`, `project.save`, `project.saveAs`, `project.import`, `project.close`, `sequence.new`, `sequence.settings`, `sequence.duplicate`, `sequence.closeGaps`, `sequence.renderSelection`, `sequence.renderEffectsInOut` | M/X (disk I/O) | project, active sequence | Project items, master clips, sequence format presets |
| Timeline & Trimming | `commands.rs:351-950`, `clip_ops.rs`, `trim.rs` | `edit.rippleDelete`, `clip.speedDuration`, `clip.reverse`, `clip.enable`, `clip.link`, `clip.unlink`, `clip.group`, `clip.ungroup`, `clip.nest`, `sequence.addEdit`, `sequence.addEditAllTracks`, `sequence.trimEdit`, `sequence.rippleTrimPrev`, `sequence.rippleTrimNext`, `sequence.razorAtPlayhead`, `sequence.extendSelectedEdit` | M | sequence tracks, clip selection | Ripple, roll, slip, and slide trims; nested sequence composition |
| Markers & In/Out | `commands.rs:951-1250`, `marker_export.rs` | `marker.setIn`, `marker.setOut`, `marker.clearIn`, `marker.clearOut`, `marker.clearInOut`, `marker.addMarker`, `marker.gotoPrevMarker`, `marker.gotoNextMarker`, `marker.clearSelectedMarker`, `marker.clearAllMarkers`, `marker.export` | M | sequence / clip timecode | Sequence markers and clip markers with color labels |
| Multi-Camera & Sync | `commands.rs:1251-1450`, `multicam.rs`, `sync.rs` | `multicam.createMultiCameraSource`, `multicam.switchCamera`, `multicam.enableAngle`, `sync.synchronizeClipsByAudio`, `sync.synchronizeClipsByTimecode` | M | multi-track clips | Audio waveform cross-correlation synchronisation |
| Video Effects & Color | `commands.rs:1451-1950`, `color.rs`, `masks.rs` | `effect.addVideoEffect`, `effect.removeVideoEffect`, `effect.animateParameter`, `color.lumetri.exposure`, `color.lumetri.contrast`, `color.lumetri.whites`, `color.lumetri.blacks`, `color.lumetri.lut`, `color.lumetri.curves`, `color.lumetri.colorWheels`, `masks.createBezierMask` | M | clip effects stack | GPU shader pipelines and Lumetri Color grading |
| Audio Editing & Mixer | `commands.rs:1951-2450`, `mixer.rs`, `essential_sound.rs` | `audio.gain`, `audio.channels`, `audio.normalizeMaxPeak`, `mixer.setVolume`, `mixer.setPan`, `mixer.muteTrack`, `mixer.soloTrack`, `mixer.recordTrack`, `sound.essential.autoDucking`, `sound.essential.clarity` | M | audio tracks / clips | Sample-accurate clip gain and track audio automation |
| Titles & Captions | `commands.rs:2451-2850`, `graphics.rs`, `captions.rs`, `transcript.rs` | `graphics.newTextLayer`, `graphics.newShapeLayer`, `captions.newTrack`, `captions.addCaption`, `captions.autoTranscribe`, `captions.exportSrt` | M | timeline tracks | Motion graphics templates and speech-to-text subtitle tracks |
| Interchange & Media | `commands.rs:2851-3400`, `interchange.rs`, `aaf_omf.rs`, `relink.rs` | `interchange.exportFcpXml`, `interchange.importFcpXml`, `interchange.exportAaf`, `interchange.exportEdl`, `media.relinkClips`, `media.replaceFootage`, `media.makeOffline` | M/X | project media pool | FCP XML, AAF, and EDL round-trip interchange |
| Scopes & Monitoring | `commands.rs:3401-3650`, `scopes.rs`, `source_monitor.rs` | `scopes.waveform`, `scopes.vectorscope`, `scopes.histogram`, `scopes.parade`, `source.openClip`, `source.insert`, `source.overwrite` | R/M | program / source frame | Real-time GPU scope calculations (Rec.709 / Rec.2020) |
| Export & Codecs | `commands.rs:3651-3940`, `export_tools.rs`, `previews.rs` | `export.media`, `export.queue`, `export.preset`, `export.frame`, `previews.generate` | M/X (disk render) | sequence range | Multi-threaded pure-Rust encoding (H.264, ProRes, AV1, AAC) |

### UI-only commands (`crates/ui-egui/src/menus.rs`)

`UI_COMMANDS` table:
`app.language.*` (English, Japanese, Spanish, Portuguese, Ukrainian, Chinese, German, Russian, French), `source.playback.toggle/play/stop`, `playback.toggle` (Space), `playback.forward/stop/reverse` (J/K/L shuttle navigation), `playback.slowForward/slowReverse`, `playback.playAround`, `playback.inToOut`, `playback.loop`, `view.zoomIn/Out/ToSequence`, `view.playbackRes.*` (Full, 1/2, 1/4, 1/8, 1/16), `view.pausedRes.*`, `view.highQualityPlayback`, `view.display.composite/alpha`.

### Gestures & pointer tools

- **Timeline Tools:** Selection (V), Track Select Forward/Backward (A/Shift+A), Ripple Edit (B), Rolling Edit (N), Rate Stretch (R), Razor (C), Slip (Y), Slide (U), Pen (P), Hand (H), Zoom (Z).
- **Trimming Gestures:** Hover edit point, drag for head/tail ripple trim or rolling trim, double-click for Trim Mode editor.
- **Monitor Gestures:** Drag from Source monitor to Program monitor (insert/overwrite overlay), scrub playhead timecode, J-K-L keyboard shuttling.
- **Audio Gestures:** Drag volume rubberband on timeline clip, track fader move during write automation.

### Formats, device hooks & status providers

- **Formats:** Containers: MP4, MOV, MKV, MXF, MPEG-TS. Codecs: ProRes (422, 4444), DNxHD/DNxHR, H.264, HEVC, AV1, VP9, AAC, Opus, AC-3, PCM. Subtitles: SRT, VTT, SCC. Project Interchange: Final Cut Pro XML, AAF, EDL, `.filmcraft` project files.
- **Device hooks:** Multi-threaded video playback engine, audio mixer buffer, GPU shaders (`filmcraft-gpu`), microphone voiceover recording (`voiceover.rs`).
- **Status providers:** Dropped frames indicator, render status bar (red/yellow/green), proxy generation queue, audio peak meters (RMS/peak LUFS).
