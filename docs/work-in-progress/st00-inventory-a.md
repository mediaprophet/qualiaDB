# ST-00 (d) — Per-app feature/command coverage inventory: WordCraft, DeckCraft, GridCraft

Subunit (d) of packet ST-00 (playbook §18.3): *"inventory commands, UI-only handlers, gestures, formats, payloads, device hooks and real status providers for each app."*

**Method:** pure source reading. No cargo/git/build commands were run. All command IDs were extracted from the actual registration code (`CommandSpec::new(...)` / `cmd!(...)` literals), not from README/parity prose. Counts were cross-verified (DeckCraft: 226 grep-verified `cmd!` sites = ROADMAP's stated "226 commands"; GridCraft: 315 `cmd!` sites = 212 same-line + 103 multi-line registrations).

**Provenance (all three):** source snapshots, **no VCS history** (no `.git`). Embedded revision identifiers found in `contributors/contributors.json` (generated stats file; see per-app lines). Licence for every row below unless noted: **MIT OR Apache-2.0** (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` at each root; copyright "ArtCraft Team and the <App> contributors"; `docs/brand/` ArtCraft wordmark files are trademark, not open-licensed — `docs/brand/LICENSE-brand.txt`).

**Coverage-row schema (§18.3 verbatim):** source repo/revision/file/symbol · feature/command/gesture ID · read/mutation/external effect · strict-parameter-schema state · target/selection dependencies · native/WASM support · UI destination · Qualia primitive owner or promotion need · fidelity limitation · licence/provenance · synthetic fixture · required assertion · packet/subunit · evidence/state.

Row-invariant fields are stated once per section and abbreviated in tables: `lic/prov` = MIT/Apache-2.0 + snapshot provenance; `packet` = ST-00(d); `schema` = the `params`/`params doc` field (a human/agent doc string like `"{rows, cols}"`, **not** a machine-enforced JSON Schema — "loose doc-string schema" everywhere unless a row says otherwise); `fixture` defaults to `crates/engine/src/sample.rs` synthetic sample doc + hostile-params fuzz test cited per app.

---

## WordCraft (`C:\github\storytold\wordcraft-main`)

### Baseline facts

| Field | Value | Evidence |
|---|---|---|
| Workspace package | version `0.4.0`, edition `2024`, `rust-version = "1.90"` (MSRV), `resolver = "3"` | `Cargo.toml:6-11` |
| Licence | MIT OR Apache-2.0 | `Cargo.toml:9`, `NOTICE` |
| Declared repo / embedded revision | `github.com/storytold/wordcraft`; snapshot head `d7d9702bda1fe626952024dc37fd2624323dad0f` (branch main, head_date 2026-10-09T13:22:42Z, 45 commits) | `Cargo.toml:11`; `contributors/contributors.json:2-6` |
| Workspace members | `crates/*`, `apps/*`, `xtask`; default-members crates/* + wordcraft + wordcraft-cli + xtask | `Cargo.toml:3-4` |
| Crates (16) | geom, doc, fonts, layout, docx, docbin, formats, render, pdf, proof, engine, mcp, zotero, control-key, ui-egui, metafile | `Cargo.toml:15-30`, `crates/` |
| Binaries | `wordcraft` (desktop eframe GUI, `--control <port>`, `--sample`); `wordcraft-cli` (`convert/info/text/inspect/render/run/commands/parity/mcp/zotero/--version`); `wordcraft-web` | `apps/wordcraft/src/main.rs:3-8`; `apps/wordcraft-cli/src/main.rs:9-33` |
| WASM mechanism | `apps/wordcraft-web`: eframe web runner, **wgpu (WebGPU) with WebGL2 fallback**, `wasm-bindgen`/`js-sys`/`web-sys`, built by **Trunk** (`apps/wordcraft-web/Trunk.toml` → `dist/web`, serve port 8771); no handwritten JS/TS | `apps/wordcraft-web/Cargo.toml:24-31`, `Trunk.toml` |
| UI toolkit | egui/eframe/egui_extras `0.36`, egui_kittest `0.36`; raster `vello_cpu 0.2` (CPU), `harfrust 0.13` shaping, `krilla 0.8` PDF, `rfd 0.17` file dialogs, `arboard 3.6` clipboard | `Cargo.toml:65-70` |
| Control channel | localhost JSON-lines TCP on `--control PORT`; per-instance key file `<settings>/control-key.<instance>` mode 0600 | `apps/wordcraft/src/main.rs:5-8`; `crates/control-key/src/lib.rs`; `crates/ui-egui/src/control.rs:106-235`; `docs/control-protocol.md` |
| MCP | `crates/mcp` stdio server, headless or `--connect` bridge to control channel | `crates/mcp/src/{server,tools,backend}.rs`, `apps/wordcraft-cli/src/main.rs` usage |
| Test commands | `cargo xtask ci` (fmt --check, clippy -D warnings, test, assets, layers, wasm); `xtask assets|layers|wasm|corpus|stats|parity|ico|version` | `xtask/src/main.rs:16-49` |
| Parity catalog | `crates/engine/src/catalog.rs` (`CATALOG` text table `Tab|Group|Feature|command id`); ROADMAP reports 402/434 catalog ids implemented | `catalog.rs:10-`; `ROADMAP.md` |

### Constraints recorded (AGENTS.md / CLAUDE.md — both identical content)

- **Never crash:** no `unwrap/expect/panic!/unreachable!/todo!/unimplemented!` in non-test code; `unsafe_code = "forbid"`; `Session::run` catches escaped panics and restores the document (`crates/engine/src/session.rs:650-739`, `catch_unwind` at `:719`); regression test `crates/engine/src/tests.rs::hostile_params_never_panic` runs every command with junk params.
- **Assets:** nothing from Microsoft/Adobe/Avid/Autodesk; every asset needs an `ATTRIBUTION.md` row (`cargo xtask assets` enforces); fonts never in repo (optional `CRAFT_FONTS_DIR`, `.cargo/config.toml` → `../craft-fonts`).
- **Clean room:** black-box observation only; formats from ECMA-376/RTF spec/ODF 1.3; no GPL-family code.
- **Rust only** (no handwritten JS/TS); never break wasm (`cargo xtask wasm` checks L0–L5 + ui-egui crates, `xtask/src/main.rs:119-149`).
- **Layering** L0 geom → L6 ui-egui enforced by `cargo xtask layers`.

### Command registry — 460 `CommandSpec` (all in `crates/engine/src/cmd/`)

`CommandSpec` struct: `{id, label, location (ribbon "Home › Font"), shortcut, params(doc string), mutates (pure() = read-only), enabled: fn(&Session)->Option<&str>, run}` — `crates/engine/src/lib.rs:49-90`. Registry assembled in `crates/engine/src/cmd/mod.rs:37-66` (`registry()` extends 25 `specs()`). One dispatch path for UI/CLI/MCP/control: `Session::run` (`session.rs:650`) — checks `enabled`, applies Restrict Editing gate, macro recording, undo checkpoint, panic guard.

Row-invariant for this table: repo `storytold/wordcraft@d7d9702` (snapshot); native+WASM unless noted; UI dest = egui ribbon/menu/dialog at `location`; lic/prov = MIT/Apache snapshot; fixture = `sample.rs` templates (blank/sample/letter/resume/report); assertion = `hostile_params_never_panic` + per-family unit tests in same file; packet = ST-00(d); evidence/state = verified registration.

| Family (file:line → symbol `specs()`) | Registered IDs (complete) | Effect | Target/selection deps | Fidelity / anomaly |
|---|---|---|---|---|
| `cmd/text.rs:11` typing/deleting | `text.insert`, `text.newParagraph`, `text.lineBreak`, `text.pageBreak`, `text.columnBreak`, `text.tab`, `text.backTab`, `text.backspace`, `text.delete`, `text.deleteWordBack`, `text.deleteWordForward`, `text.nbsp`, `text.nbHyphen`, `text.optionalHyphen` | mutation (doc); keys Enter/Shift+Enter/Mod+Enter/Tab/Backspace/Delete bound | caret/selection in a story | AutoCorrect/AutoFormat hooks inside `text.insert` (`text.rs:48-79`) |
| `cmd/caret.rs` nav/selection (23) | `caret.left/right/up/down/wordLeft/wordRight/home/end/paraUp/paraDown/docStart/docEnd/pageUp/pageDown/set`, `select.all`, `select.range`, `select.word`, `select.sentence`, `select.paragraph`, `select.line`, `select.text`, `select.collapse` | read/no-mutate (`.pure()`) — move caret/selection | none | `caret.wordLeft/Right` flip under RTL para (caret.rs:18-22) |
| `cmd/column.rs:20` | `select.column` | mutation of selection | caret | — |
| `cmd/edit.rs` clipboard/find (23) | `edit.undo`, `edit.redo`, `edit.cut`, `edit.copy`, `edit.paste`, `edit.pasteText`, `edit.pasteMerge`, `edit.clipboardPane`, `edit.clipboardItems`, `edit.pasteClipboardItem`, `edit.pasteAllClipboard`, `edit.deleteClipboardItem`, `edit.clearClipboard`, `edit.find`, `edit.advancedFind`, `edit.findNext`, `edit.findPrevious`, `edit.replace`, `edit.replaceAll`, `edit.goto`, `edit.formatPainter`, `edit.copyFormat`, `edit.pasteFormat` | mutation; `edit.copy/paste*` external effect = OS clipboard via `clipboard_picture`/arboard service | selection for cut/copy | internal clip history + OS clipboard image hook |
| `cmd/paste.rs:29` | `edit.pasteSpecial` | mutation | selection | — |
| `cmd/format.rs` char fmt (35) | `format.bold/italic/underline/doubleUnderline/wordUnderline/strikethrough/border/doubleStrikethrough/subscript/superscript/allCaps/smallCaps/hidden/outline/shadow/emboss/engrave/font/size/growFont/shrinkFont/growFont1/shrinkFont1/color/highlight/shading/changeCase/clear/spacing/scale/position/charStyle/set/fontDialog/state` | mutation; `format.state`/`format.fontDialog` pure | text selection or object | `format.fontDialog`/`format.dialog`-style commands return `UiRequest`-style dialog signal (see para.dialog) |
| `cmd/para.rs` paragraph/styles (52) | `para.alignLeft/alignCenter/alignRight/justify/distribute/align/rtl/ltr/indent/outdent/hangingIndent/removeHanging/indents/lineSpacing/single/double/oneAndHalf/spacing/addSpaceBefore/removeSpaceAfter/style/normal/heading1/heading2/heading3/bullets/numbering/multilevel/listLevel/restartNumbering/shading/borders/sort/keepNext/keepLines/pageBreakBefore/widowControl/asianTypography/tabs/outlineLevel/set/dialog`, `styles.create/modify/updateToMatch/delete/manage/setVisibility/list/pane/apply/addToGallery` | mutation | paragraphs at selection | `styles.*` IDs registered here, not in cmd/inspector.rs |
| `cmd/lists.rs` (2) | `list.define`, `list.get` | mutation / read | selection | — |
| `cmd/inspector.rs` (3) | `styles.inspector`, `styles.inspect` (pure), `styles.inspectorClear` | mutation/read | selection | — |
| `cmd/view.rs` (24) | `view.marks`, `view.printLayout`, `view.webLayout`, `view.draft`, `view.outline`, `view.readMode`, `view.focus`, `view.ruler`, `view.gridlines`, `table.viewGridlines`, `view.navigationPane`, `view.zoom`, `view.zoom100`, `view.zoomIn`, `view.zoomOut`, `view.onePage`, `view.multiplePages`, `view.pageWidth`, `view.darkMode`, `view.stylesPane`, `view.commentsPane`, `view.newWindow`, `view.split`, `view.state` | mostly `.pure()` view state; `view.state` serializes `s.view` | none | `view.newWindow`/`view.split` set flags consumed by host shell (no real multi-window in engine) |
| `cmd/insert.rs` (25) | `insert.pageBreak`, `insert.blankPage`, `insert.coverPage`, `insert.table`, `insert.picture`, `insert.shape`, `insert.textBox`, `insert.link` (Mod+K), `insert.removeLink`, `insert.bookmark`, `insert.header`, `insert.footer`, `insert.pageNumber`, `insert.editHeader`, `insert.editFooter`, `insert.closeHeader`, `insert.removeHeader`, `insert.removeFooter`, `insert.dateTime`, `insert.symbol`, `insert.field` (Mod+F9), `insert.dropCap`, `insert.horizontalLine`, `insert.wordArt`, `insert.textFromFile` | mutation; `insert.picture`/`insert.textFromFile` read external files (path/base64) | caret; `has_*` gates | picture insert accepts `{path}` or `{data: base64}` |
| `cmd/equation.rs` OMML math (21) | `insert.equation`, `equation.edit`, `equation.exit`, `equation.type`, `equation.insertStructure`, `equation.insertSymbol`, `equation.move`, `equation.backspace`, `equation.delete`, `equation.enter`, `equation.convert`, `equation.inputFormat`, `equation.normalText`, `equation.display`, `equation.justify`, `equation.number`, `equation.set` (`{linear?/latex?}`), `equation.get`, `equation.gallery`, `equation.structureActions`, `equation.structure` | mutation; several `.when(editing)`/`when(near_equation)` | math zone at caret | LaTeX and linear input supported via `equation.set` |
| `cmd/page.rs` page setup (13) | `layout.margins`, `layout.orientation`, `layout.size`, `layout.columns`, `layout.break`, `layout.lineNumbers`, `layout.hyphenation`, `layout.pageSetup`, `layout.verticalAlign`, `layout.differentFirstPage`, `layout.differentOddEven`, `layout.pageNumberFormat`, `layout.section` | mutation (doc settings/sections) | section at caret | — |
| `cmd/table.rs` (32) | `table.insertRowAbove/insertRowBelow/insertColumnLeft/insertColumnRight/deleteRow/deleteColumn/deleteTable/deleteCells/merge/split/splitTable/style/look/shading/borders/cellAlign/textDirection/autofit/distributeColumns/distributeRows/columnWidth/rowHeight/repeatHeader/sort/toText/formula/selectTable/selectRow/selectCell/properties`, `table.fromText`, `table.quick` | mutation; `table.formula` evaluates in-table formulas (`=SUM(ABOVE)`) | table at caret | — |
| `cmd/table_style.rs` (3) | `table.newStyle`, `table.modifyStyle`, `table.deleteStyle` | mutation | — | — |
| `cmd/table_draw.rs` (2) | `table.draw` (`{cells}` grid spec), `table.eraser` | mutation; drives `ui-egui/src/table_pen.rs` pointer tool | pointer-drawn rect | gesture pair; canvas pen owns mouse while active (`canvas.rs:672-675`) |
| `cmd/review.rs` (27) | `review.newComment` (Mod+Alt+M), `review.reply`, `review.editComment`, `review.deleteComment`, `review.resolveComment`, `review.nextComment`, `review.previousComment`, `review.comments`, `review.trackChanges`, `review.acceptAll`, `review.rejectAll`, `review.accept`, `review.reject`, `review.nextChange`, `review.previousChange`, `review.markup`, `review.trackingOptions`, `review.wordCount`, `review.changes`, `review.spelling` (F7), `review.issues`, `review.suggestions`, `review.addToDictionary`, `review.ignoreAll`, `review.applySuggestion`, `review.proofing`, `review.thesaurus` (Shift+F7) | mutation + read; track-changes model in `doc` | selection/comment ids | proofing via `crates/proof` |
| `cmd/file.rs` (19) | `file.new` (Mod+N), `file.open`, `file.save`, `file.saveAs` (F12, `{password?}`), `file.encrypt`, `file.exportPdf`, `file.exportPng`, `file.print`, `file.close`, `file.properties`, `file.info`, `file.options`, `file.setAuthor`, `document.inspect`, `document.text`, `document.paragraph`, `document.selection`, `document.layout`, `document.setText` | external file I/O; `file.print` raises print request (needs host hook); `document.*` = agent-facing pure reads | — | `document.setText` replaces whole body (mutating agent op) |
| `cmd/design.rs` (10) | `design.theme`, `design.themeFonts`, `design.themeColors`, `design.styleSet`, `design.paragraphSpacing`, `design.watermark`, `design.pageColor`, `design.pageBorders`, `design.setDefault`, `design.themes` | mutation | doc-wide | — |
| `cmd/references.rs` (10) | `references.toc`, `references.updateToc`, `references.removeToc`, `references.addText`, `references.footnote`, `references.endnote`, `references.nextFootnote`, `references.notes`, `references.caption`, `references.updateFields` | mutation | caret | field model (TOC etc.) in `doc` |
| `cmd/citations.rs` (14) | `references.sources`, `references.citation`, `references.citationStyle`, `references.bibliography`, `references.tableOfFigures`, `references.updateFigures`, `references.markEntry`, `references.index`, `references.updateIndex`, `references.markCitation`, `references.tableOfAuthorities`, `insert.crossReference`, `references.noteOptions`, `references.researcher` | mutation; `references.sources` works with Zotero-crate source model | caret | CSL fields; Zotero wire protocol support (see device hooks) |
| `cmd/mailings.rs` (17) | `mailings.start`, `mailings.recipients`, `mailings.editRecipients`, `mailings.insertField`, `mailings.addressBlock`, `mailings.greetingLine`, `mailings.rules`, `mailings.matchFields`, `mailings.highlightFields`, `mailings.preview`, `mailings.next`, `mailings.previous`, `mailings.findRecipient`, `mailings.checkErrors`, `mailings.finish`, `mailings.envelopes`, `mailings.labels` | mutation; recipient list reads external csv/json | doc + recipient source | — |
| `cmd/objects.rs` floating objects/pictures (31) | `picture.size/crop/altText/corrections/color/effects/transparency/removeBackground/compress/reset/change/style/border`, `arrange.rotate`, `arrange.rotation`, `arrange.wrap`, `arrange.position`, `arrange.bringForward`, `arrange.sendBackward`, `arrange.align`, `arrange.selectionPane`, `arrange.bounds`, `arrange.nudge`, `arrange.group`, `arrange.ungroup`, `select.addObject`, `select.objects`, `shape.fill`, `shape.outline`, `shape.effects`, `shape.change` | mutation; `.when(has_movable/has_picture/has_group)` | floating object/picture selection | — |
| `cmd/draw.rs` ink (8) | `draw.select`, `draw.eraser`, `draw.pen`, `draw.pencil`, `draw.highlighter`, `draw.stroke` (`{points}`), `draw.erase`, `review.hideInk` | mutation (ink strokes model) | page + point payload | `ui-egui/src/ink.rs` pointer capture |
| `cmd/tools.rs` misc/macros/agents (41) | `edit.repeat`, `tools.recordMacro`, `tools.macros`, `tools.autocorrect`, `review.compare`, `review.combine`, `file.accessibility`, `file.inspect`, `review.restrict`, `file.protect`, `file.versions`, `file.recover`, `file.newFromTemplate`, `file.saveTemplate`, `file.autosave`, `file.compatibility`, `file.share`, `insert.quickParts`, `insert.autoText`, `insert.docProperty`, `insert.signatureLine`, `insert.object`, `insert.spreadsheet`, `layout.linkToPrevious`, `hf.next`, `hf.previous`, `hf.position`, `table.cellMargins`, `table.borderPainter`, `para.setNumberingValue`, `para.defineBullet`, `para.defineNumber`, `review.language`, `review.showMarkup`, `review.editor`, `select.similar`, `select.extend`, `design.effects`, `view.immersive`, `view.vertical`, `view.sideToSide` | mixed; `file.share`/`insert.object`/OLE embedded `insert.spreadsheet` are thin/approximate | varies | `file.share` is a stub-level feature (returns share info, no OS share sheet) |
| `cmd/speech.rs` Read Aloud (9) | `review.readAloud`, `readAloud.playPause`, `readAloud.next`, `readAloud.previous`, `readAloud.stop`, `readAloud.speed`, `readAloud.skipCitations`, `readAloud.status`, `caret.pastCitation` | **external effect**: spawns OS speech program (`say`/`espeak`/`spd-say`/PowerShell) on worker thread — **native-only** (`#[cfg(not(wasm32))]` paths in `engine/src/speech.rs:9-12`) | sentence queue from caret | WASM: speech unsupported (cfg-gated) |

### WordCraft UI-only commands (not in engine registry)

Handled in `WordApp::run` → `ui_commands` match at `crates/ui-egui/src/lib.rs:568-660`:

`ui.tab`, `ui.backstage`, `ui.dialog` (param `name` — dialog catalog: find, goto, replace, insertTable, link, bookmark, field, symbol, watermark, pageSetup, columns, newRecipientList, wordCount, zoom, about, commands, newTableStyle, modifyTableStyle, defineList, font, paragraph… per `ribbon.rs`/`dialogs*.rs`/`keytips.rs:331`), `ui.closeDialog`, `ui.changePicture`, `ui.collapseRibbon`, `ui.dark`, `ui.theme`, `ui.language`, `ui.saveChanges` (`{answer: save|dontSave|cancel}`), `ui.openFileDialog`, `ui.openRecipientList`, `ui.print` (system print via host hook, errors when absent — `lib.rs:654-677`), `ui.discord` (opens community link externally).

Effect: UI state mutation or external (file picker, print, URL). None in the engine registry → **all need promotion/mapping** when a shared command registry wraps these apps (ST-11). `ui.print`/`ui.discord`/`ui.openFileDialog` are the external-effect ones.

### WordCraft gestures / pointer tools (UI-side, not command ids)

| Gesture | Handler site | Effect |
|---|---|---|
| Click/drag text selection, double/triple-click word/para, right-click context menu (with spell synonyms + issues lookup via `review.suggestions`/`review.thesaurus`) | `crates/ui-egui/src/canvas.rs:676-706` (mouse()), layout `hit()` | selection mutation |
| Floating object drag/resize/rotate, image handles | `crates/ui-egui/src/objects.rs` (`obj_drag`, `objects::cursor`) | `arrange.*`/`picture.*` mutations |
| Ink pen strokes (pen/pencil/highlighter/eraser) | `crates/ui-egui/src/ink.rs` → `draw.stroke`/`draw.erase` | mutation |
| Draw Table pen / table eraser | `crates/ui-egui/src/table_pen.rs` → `table.draw`/`table.eraser` | mutation |
| Horizontal/vertical ruler drag (margins/indents/tabs) | `canvas.rs` `rulers()` | `layout.*`/`para.*` mutations |
| Scroll/zoom (scroll.rs), mini toolbar on selection (`mini_toolbar.rs`), keytips/Alt ribbon (`keytips.rs`), comment balloons (`canvas.balloon*`) | — | view/UI |

All pointer paths feed `app.run(id)`/`session.run(id)` — no UI-private mutations of the document (good for Qualia adaptation).

### WordCraft formats, device hooks, surfaces

| Item | Detail | Evidence |
|---|---|---|
| Import | docx/docm/dotx/dotm (incl. agile-encrypted via RustCrypto aes/sha1/sha2/hmac + password param), doc/dot (docbin), txt, md, html, rtf, odt, tex/latex, json/wcraft.json | `engine/src/io.rs:6-25`, `io_ext.rs:6-12`, `Cargo.toml:55-62` |
| Export | docx family, pdf, txt, md, html, rtf, odt, tex, png, json | `io.rs:27`, `io_ext.rs:15-25` |
| Clipboard | arboard + `clipboard_picture` service (paste images/screenshots) | `ui-egui/src/lib.rs:82`, `paste_picture.rs` |
| File pickers | rfd native; web async picker via `open_async`+`inbox`; `download` service for browser saves | `ui-egui/src/lib.rs:56-72` |
| Print | `ui.print` → in-memory PDF → host `print` hook; absent hook = error (never silent) | `lib.rs:663-677` |
| Audio/TTS | OS speech executables (`say`, `espeak`/`spd-say`, PowerShell synth) — native-only | `engine/src/speech.rs:9-12` |
| Zotero | `crates/zotero` — LibreOffice word-processor wire protocol client on `127.0.0.1:23116`; CLI `wordcraft-cli zotero <addEditCitation|addEditBibliography|addNote|refresh|removeCodes|setDocPrefs>`; `ui-egui/src/zotero.rs` | `zotero/src/lib.rs:1-15`, `apps/wordcraft-cli/src/main.rs:25-29` |
| External links | `ui.discord` / Help → discord/website URLs | `chrome.rs:134`, `backstage.rs:96` |
| Control methods | `engine.execute`/`command`, `engine.commands`, `document.inspect`, `ui.inspect`, `ui.parity`, `ui.click`, `ui.clickText`, `ui.move`, `ui.press`, `ui.release`, `ui.drag`, `ui.key`, `ui.text`, `ui.screenshot`, `ui.render`, `ui.resize`, `ui.focus`, `app.quit`; unknown method falls back to command-id dispatch | `control.rs:106-235` |
| MCP tools (16) | `list_commands`, `execute`, `batch`, `new_document`, `open_document`, `save_document`, `type_text`, `select_text`, `get_text`, `inspect_document`, `render_page`, `parity`, `screenshot`, `click`, `key`, `ui_inspect` | `crates/mcp/src/tools.rs:47-146` |
| Status providers (real) | status bar: page N of M, word count (selection-aware), story indicator, language (**static "English (United States)" — anomaly**), Track Changes on/off, transient error/status msg (6 s), zoom % + slider, view-mode icons; `ui.inspect` perf: `frameMs/renderMs/layoutMs` | `ui-egui/src/chrome.rs:196-279`, `control.rs:45-63` |
| Panic guard | `Session::run` → `catch_unwind` → `CmdError::Failed("{id} panicked: …")` + doc restore | `session.rs:719-726` |

### WordCraft anomalies / gaps

- Catalog `format.phonetic`, `format.enclose` (and ~32 total catalog gaps, ROADMAP 402/434) have catalog rows but **no registered command** — catalog entries name command ids that don't exist yet (expected parity-gap design, `catalog.rs:74-75`).
- Status-bar language is hardcoded `"English (United States)"` (`chrome.rs:239`) — not a real provider.
- `file.share`, `insert.object` (OLE), `insert.signatureLine`, `insert.spreadsheet` are shallow implementations vs Word behaviour (see bodies in `cmd/tools.rs:171-244`).
- Read Aloud is native-only; `SystemTime` needs `js_sys` fallback on wasm (`cmd/mod.rs:278-290`).
- `view.newWindow`/`view.split` are flags; the egui host has no real multi-window/split implementation (fidelity gap for host-side mapping).

---

## DeckCraft (`C:\github\storytold\deckcraft-main`)

### Baseline facts

| Field | Value | Evidence |
|---|---|---|
| Workspace package | version `0.4.0`, edition `2024`, `rust-version = "1.90"`, `resolver = "3"` | `Cargo.toml:6-11` |
| Licence | MIT OR Apache-2.0 | `Cargo.toml:9`, `NOTICE` |
| Declared repo / embedded revision | `github.com/storytold/deckcraft`; snapshot head `4b09f35c1d6fa50e2d94243a5d65d98fe49558e0` (main, 2026-10-09T14:48:26Z, 41 commits) | `Cargo.toml:11`; `contributors/contributors.json:2-6` |
| Workspace members | `crates/*`, `apps/*`, `xtask` | `Cargo.toml:3` |
| Crates (23) | geom, color, model, fonts, text, render, pdf, pptx, format, anim, engine, mcp, ui-egui, media + codecs ported from FilmCraft (same owner/licence): bitstream, h264, hevc, vp9, av1, isobmff, matroska, ogg, opus | `Cargo.toml:15-38` |
| Binaries | `deckcraft` (desktop), `deckcraft-cli`, `deckcraft-web` | `apps/` |
| WASM | `apps/deckcraft-web`: Trunk (`Trunk.toml`, port 8768, `dist/web`), eframe wgpu runner (same egui 0.36 stack) | `apps/deckcraft-web/` |
| UI toolkit | egui/eframe `0.36`, vello_cpu render, `i_overlay 9` (merge shapes), krilla `0.8` PDF | `Cargo.toml:54-70` |
| Control channel | JSON-lines localhost, methods in `crates/ui-egui/src/control.rs:1-17` | control.rs |
| MCP | `crates/mcp` (backend/headless/server/tools) | `crates/mcp/src/` |
| Store metadata | `omastore.toml` — Linux store packaging (`ai.storyteller.deckcraft` icon id) | `omastore.toml` |
| Test commands | `cargo xtask ci` (fmt, clippy -D warnings, test, layers, assets, wasm); `xtask assets|layers|wasm|stats|parity|ico|version` | `xtask/src/main.rs:16-46` |

### Constraints (AGENTS.md/CLAUDE.md)

Same craft rules as WordCraft (never-crash, `#![deny(...)]` on production crates, asset policy with explicit vendor-font exclusion incl. Source Sans/Serif/Han + Noto CJK in `crates/fonts/build.rs`, clean-room PowerPoint observation, no Tauri — egui only, everything-is-a-command, `cargo xtask ci`). Panic guard: `Session::guarded` (`crates/engine/src/guard.rs:37-63`) restores doc+selection+revision and writes crash recovery via `recovery.rs`.

Command spec: `{id, label, menu: &[&str], shortcut, params: &'static str doc, enabled: fn→Result<(),String>, run, journal: bool, undoable: bool}`; `cmd!` variants — plain (journal+undoable), `query` (read-only, not journaled), `noundo` (journaled, not undoable). Registry: `cmd/mod.rs:30-145` (`command_specs()` OnceLock over 16 `specs()`). Enabled gates: `always`, `has_doc`, `has_slide`, `has_selection`, `has_two`, `has_text`, `has_text_or_shapes`, `can_undo`, `can_redo` (`cmd/mod.rs:70-107`).

### Command registry — 226 `cmd!` specs (verified count; ROADMAP agrees "the 226 commands")

Row-invariants as above; repo `storytold/deckcraft@4b09f35` snapshot; fixture = `engine/src/sample.rs` + `guard.rs` panic test; assertion = every-command param-fuzz + family tests; packet ST-00(d).

| Family (file → specs) | Registered IDs (complete) | Effect | Deps | Notes |
|---|---|---|---|---|
| `cmd/file.rs` (17) | `file.new`(noundo, Cmd+N), `file.open`, `file.openBytes` `{name,data:base64}`, `file.save`, `file.saveAs`, `file.saveTemplate`, `file.saveBytes`(query→base64), `file.export` (`png|jpeg|pptx|deckcraft|outline|pdf`, pdf layout/notes/handouts/perPage/dpi/slides/includeHidden/textLayer/frame), `file.render`(query→png base64), `file.close`, `file.recovery.save`, `file.recovery.list`, `file.recovery.open`, `file.recovery.discard`, `file.revert`, `window.next`, `file.properties` | external file I/O; autorecover side files | `always`/`has_doc` | native `.deckcraft` zip+JSON (`crates/format`, mimetype `application/vnd.storyteller.deckcraft+zip`, v1; legacy `.slidecraft` read) |
| `cmd/edit.rs` (11) | `edit.undo`, `edit.redo`, `edit.cut`, `edit.copy`(noundo→`{text}`), `edit.paste` `{text?}`, `edit.pasteText`, `edit.duplicate`, `edit.delete`, `edit.selectAll`, `edit.deselect`(Esc), `edit.select` `{ids,add,toggle}` | mutation; clipboard external | has_doc/selection | internal clipboard payload `{text}` |
| `cmd/slide.rs` (21) | `slide.new`, `slide.duplicate`, `slide.delete`, `slide.move`, `slide.go`(noundo), `slide.next/previous/first/last` (PageDown/PageUp/Home/End), `slide.selectSlides`, `slide.hide`, `slide.layout`, `slide.reset`, `slide.notes`, `slide.rename`, `section.add/rename/remove/removeAll/move`, `slide.fromOutline` `{text}` | mutation/navigation | has_slide | outline→slides import (plain text) |
| `cmd/insert.rs` (18) | `shape.presets`(query gallery), `insert.textBox`, `insert.table`, `insert.audio` `{path|data}`, `insert.video`, `insert.wordArt`, `insert.slideNumber`, `insert.dateTime`, `insert.symbol`, `chart.type`, `shape.insert` `{preset,rect?}`, `insert.picture`, `insert.chart`, `insert.hyperlink`, `chart.data`, `chart.options`, `insert.smartArt`, `insert.actionButton` | mutation; audio/video read file/base64 payload | has_slide/has_selection | media payloads go through `crates/media` decode + codecs |
| `cmd/shape.rs` (24) | `shape.sites`(query→connection sites), `shape.move`, `shape.resize`, `shape.setBounds`, `shape.rotate`, `shape.flip`, `shape.adjust`, `shape.change`, `shape.rename`, `shape.altText`, `shape.visible`, `shape.lock`, `shape.setDefault`, `picture.crop`, `picture.reset`, `shape.connect`, `shape.freeform`, `shape.fill`, `shape.line`, `shape.effects`, `shape.quickStyle`, `picture.adjust`, `picture.change`, `media.options` | mutation | has_selection/`has_two` | `shape.connect`/`shape.freeform` start interactive pointer sequences |
| `cmd/merge.rs` (1) | `shape.merge` `{op: union|combine|fragment|intersect|subtract}` | mutation (i_overlay boolean) | has_two | tests verify area math (`merge.rs:203-218`) |
| `cmd/arrange.rs` (15) | `arrange.bringToFront/bringForward/sendBackward/sendToBack` (Cmd+[ ]), `arrange.reorder`, `arrange.group`, `arrange.ungroup`, `arrange.regroup`, `arrange.rotateLeft/rotateRight`, `arrange.flipHorizontal/flipVertical`, `arrange.nudge`, `arrange.align`, `arrange.distribute` | mutation | has_selection/has_two | smart-guide snap is UI-side (`ui.smartGuides`) |
| `cmd/text.rs` (11) | `text.edit`(noundo, Enter), `text.exit`, `text.insert`, `text.delete`(Backspace, `dir:`), `text.move`(noundo), `text.select`, `text.selectAll`, `text.selectWord`, `text.selectParagraph`(all noundo), `text.get`(query), `text.set` | mutation/caret | has_text | in-cell text edit (`cell:[r,c]`) |
| `cmd/format.rs` (39) | `format.bold`(Cmd+B), `format.italic`, `format.superscript`, `format.subscript`, `format.font`, `format.size`, `format.grow`, `format.shrink`, `format.highlight`, `format.case`(Shift+F3), `format.caps`, `format.clear`(Cmd+Space), `format.textShadow`, `format.alignLeft/Center/Right`(Cmd+L/E/R), `format.justify`, `format.indent`(Tab), `format.outdent`(Shift+Tab), `format.anchor`, `format.columns`, `format.autofit`, `format.wrap`, `format.painter`(noundo, Cmd+Shift+C), `format.painterApply`(Cmd+Shift+V), `format.state`(query), `format.underline`, `format.strikethrough`, `format.color`, `format.spacing`, `format.textOutline`, `format.align`, `format.rtl`, `format.bullets`, `format.numbering`, `format.lineSpacing`, `format.paragraph`, `format.direction`, `format.margins` | mutation | has_text_or_shapes | — |
| `cmd/design.rs` (14) | `design.theme`, `design.themes`(query gallery), `design.colors`, `design.fonts`, `design.hideBackgroundGraphics`, `view.slideMaster`(noundo), `view.closeMaster`, `master.insertLayout`, `master.renameLayout`, `master.deleteLayout`, `design.background`, `design.slideSize`, `design.headerFooter`, `master.insertPlaceholder` | mutation/view | has_doc | master/layout editing target via `Target::Master|Layout` |
| `cmd/transition.rs` (5) | `transition.set`, `transition.timing`, `transition.options`, `transition.applyAll`, `transition.list`(query gallery) | mutation | has_slide | — |
| `cmd/animation.rs` (11) | `animation.set`, `animation.add`, `animation.timing`, `animation.options`, `animation.remove`, `animation.move`, `animation.clear`, `animation.painterApply`(Alt+Shift+V), `animation.list`(query), `animation.get`(query), `animation.painter`(noundo) | mutation | has_slide/selection | `crates/anim` model |
| `cmd/table.rs` (16) | `table.insertRowAbove/Below`, `table.insertColumnLeft/Right`, `table.deleteRow/Column`, `table.merge`, `table.split`, `table.distributeRows/Columns`, `table.columnWidth`, `table.rowHeight`, `table.selectCells`(noundo), `table.style`, `table.options`, `table.cellFill` | mutation | has_selection + cell sel | — |
| `cmd/review.rs` (11) | `comment.add`(Cmd+Alt+M), `comment.reply`, `comment.delete`, `comment.resolve`, `comment.list`(query), `review.accessibility`(query), `review.spelling`(query, F7), `show.fromStart`(noundo, Cmd+Shift+Enter), `show.fromCurrent`(noundo, Cmd+Enter), `show.setup`, `show.customShow` | mutation/query/show | has_slide/has_doc | spelling via built-in wordlist |
| `cmd/inspect.rs` (4) | `document.inspect`, `slide.inspect`, `shape.inspect`, `commands.list` (all query) | read | varies | agent surface |
| `cmd/media.rs` (8) | `media.play`(Alt+P), `media.pause`, `media.toggle`, `media.stop`, `media.seek`, `media.gifPlay`, `media.info`(query), `media.posterFrame` | external: audio output + video decode state | has_doc | playback via `crates/media` + `audio_out` service |

### DeckCraft UI-only commands — `UI_COMMANDS` table

`crates/ui-egui/src/lib.rs:131-165` — 31 entries `(id, label, shortcut, params)` dispatched in `run_ui`, exposed to control/MCP via `engine.commands` (`control.rs:58-64`):

`view.normal`, `view.outline`, `view.sorter`, `view.notesPage`, `view.reading`, `view.notes`, `view.ruler`, `view.gridlines` (Shift+F9), `view.guides` (Alt+F9), `view.zoomIn` (Cmd+=), `view.zoomOut` (Cmd+-), `view.zoom`, `view.fit`, `view.pane` `{pane: format|animation|selection|comments|designer|background}`, `view.tab`, `view.collapseRibbon` (Cmd+F1), `view.dark`, `view.grayscale`, `show.start` (F5), `show.presenter`, `show.end`, `app.home`, `app.about`, `app.palette` (Cmd+Shift+P command palette), `app.preferences` (Cmd+,), `app.openDialog`, `app.insertPictureDialog`, `app.insertAudioDialog`, `app.insertVideoDialog`, `app.saveAsDialog`, `app.exportDialog`, `app.dialog` `{id}`, `app.links` (community links, external URLs).

### DeckCraft gestures / pointer tools

`Session::pointer` (`crates/engine/src/tools.rs:262`) — single gesture pipeline shared by UI/control/MCP. `PointerKind`: `down, drag, up, move, doubleClick, tripleClick` + `Mods`. `ToolKind`: `Select`, `Shape{preset}`, `TextBox`, `Ink{mode: pen|highlighter|eraser, color, width}` (tools.rs:45-61). `Hit` model: `Resize{id,handle 0-8}`, `Rotate{id}`, `Adjust{id,index}` (yellow handles), `LineEnd{id,end}` (connector endpoints w/ connection-site snapping via `connect.rs::nearest_site`), `Shape{id,text}` (tools.rs:64-88). Drag state machine: move/resize/lineEnd/rotate/adjust/marquee-select/freeform, alt-drag duplicate (tools.rs:90-120). `drawing_freeform`/`finish_freeform` reachable via `shape.freeform` + Enter/Esc (`keys.rs:32-33`). Slide-show input: `show.start`, `show.key`, `show.end` control methods; thumbnail drag in `thumbs.rs`; slide sorter drag in `sorter.rs`.

### DeckCraft formats, hooks, surfaces

| Item | Detail | Evidence |
|---|---|---|
| Import | `.deckcraft` (native zip+JSON; `.slidecraft` legacy mimetype also read), `.pptx/.potx/.ppsx` (crates/pptx), plain-text outline → slides | `engine/src/cmd/file.rs:59-91`, `format/src/lib.rs:13-18` |
| Export | pptx/potx/ppsx, deckcraft, outline txt, png/jpeg per slide, pdf (slides/notes/handouts, 1-9/page, dpi, text layer) | `file.rs:107-152`, `crates/pdf` |
| Media payloads | audio/video insert+play: codecs av1/vp9/h264/hevc/opus, containers isobmff/matroska/ogg (ported from FilmCraft, same owner+licence) | `Cargo.toml:29-38`, `crates/media` |
| Services (device hooks) | `pick_open`/`pick_save` (rfd), `read`/`write`, `open_async`+`inbox` (web), `download`, `clipboard_image` (PNG from OS clipboard), `audio_out` (`deckcraft_media::AudioOut`, cpal on desktop) | `ui-egui/src/lib.rs:45-61` |
| Control methods | `engine.execute`/`ui.menu.invoke`/`command`, `engine.commands`, `document.inspect`, `ui.inspect`, `ui.pointer`, `ui.key`, `ui.text`, `ui.tool.select`, `ui.click`, `ui.move`, `ui.drag`, `ui.set`, `ui.dialog.close`, `ui.screenshot`, `ui.render`, `ui.resize`, `ui.focus`, `show.start`, `show.key`, `show.end`, `app.open`, `app.save`, `app.export`, `app.quit` | `control.rs:1-17`, `104-` |
| MCP tools (25) | `list_commands`, `run_command`, `batch`, `new_presentation`, `open_presentation`, `save_presentation`, `export`, `inspect_document`, `inspect_slide`, `render_slide`, `add_slide`, `go_to_slide`, `add_shape`, `add_text_box`, `set_text`, `insert_picture`, `insert_table`, `insert_chart`, `select`, `pointer`, `select_tool`, `key`, `type_text`, `screenshot`, `call_app` | `mcp/src/tools.rs:65-279`, dispatch `:300-` |
| Status providers | status bar: "Slide N of M"/"Slide Master", language (**static "English (United States)" — anomaly**), transient msg, fit/zoom % + log slider, view icons, comments/notes toggles; `ui.inspect` exposes `documents[]` w/ dirty+path, `showing` state, `tool`, `perf{frameMs,renderMs,fps}` | `ui-egui/src/status.rs:11-87`, `control.rs:66-82` |
| Recovery | AutoRecover: `recovery.rs`, `file.recovery.*` commands, panic guard writes recovery | `guard.rs:55-59`, `recovery.rs` |

### DeckCraft anomalies

- `table.rs:156` string `"table.merge"` is an error-context arg, not a second registration (verified); merge-shapes op lives at `merge.rs:22`.
- Language label hardcoded in status bar (`status.rs:23`).
- `insert.smartArt`/`insert.actionButton` produce primitive placeholders vs real SmartArt (fidelity gap).
- Codec crates (h264/hevc/av1…) are heavy; wasm-gate status for media decode not verified from source (xtask wasm checks L0–L5 + ui-egui; codec crates are standalone — need `cargo check` in ST-00(a/b) to confirm wasm cleanliness).
- `slide.fromOutline`/outline export are plain-text only (no .odp support — gap vs PowerPoint formats).

---

## GridCraft (`C:\github\storytold\gridcraft-main`)

### Baseline facts

| Field | Value | Evidence |
|---|---|---|
| Workspace package | version `0.5.0`, edition `2024`, `rust-version = "1.90"`, `resolver = "3"` | `Cargo.toml:6-11` |
| Licence | MIT OR Apache-2.0 | `Cargo.toml:9`, `NOTICE` |
| Declared repo / embedded revision | `github.com/storytold/gridcraft`; snapshot head `90b3c41eec691797129303603d4073269aebab73` (main, 2026-10-10T19:59:42Z, 148 commits) | `Cargo.toml:11`; `contributors/contributors.json:2-6` |
| Workspace members | `crates/*`, `apps/*`, `xtask` | `Cargo.toml:3` |
| Crates (12) | core, numfmt, formula, functions, model, calc, xlsx, chart, engine, ui-egui, mcp, pdf | `Cargo.toml:15-26`, `crates/` |
| Binaries | `gridcraft` (desktop), `gridcraft-cli` (`eval`, `run --in/--cmd/--out`, `mcp`), `gridcraft-web` | `apps/`; CLAUDE.md |
| WASM | `apps/gridcraft-web`: Trunk (`Trunk.toml`, port 8771, `dist/web`), eframe wgpu | `apps/gridcraft-web/` |
| UI toolkit | egui/eframe `0.36`, `sys-locale 0.3` (first-run language), rfd, arboard | `Cargo.toml:40-47` |
| Control channel | JSON-lines localhost (default `--control 7979`); `docs/control-protocol.md` | `ui-egui/src/control.rs:1-17` |
| MCP | `crates/mcp` stdio + `--connect` bridge | `crates/mcp/src/` |
| Test commands | `cargo xtask ci` = fmt, clippy -D warnings, test, assets, layers, wasm, **parity --check**; `xtask assets|layers|wasm|parity|version|stats|ico` | `xtask/src/main.rs:19-53` |
| Parity catalog | `crates/engine/src/catalog.rs` `CATALOG: &[(id, tab, group, label)]` | `catalog.rs:6-` |
| Crash fuzz | `tests::every_command_survives_empty_params` | AGENTS.md |
| Panic guard | `Session::execute` → `catch_unwind` restores workbook | `engine/src/lib.rs:269` |

Same craft constraints: never-crash/no-unsafe, asset policy (brand exception `docs/brand/`), clean-room Excel observation, corpus repo for real Excel files (pinned+sha256, never committed), everything-is-a-command, UI thin (`UiRequest::Dialog`/`Message`/`EditCell` drained by host, `ui-egui/src/lib.rs:357-369`), layering `core/numfmt → formula/functions → model/calc/xlsx → chart → engine → ui-egui/mcp → apps`.

### Command registry — 315 `cmd!` specs (grep-verified)

Same `CommandSpec` shape as DeckCraft (`cmd/mod.rs:32-43`, `cmd!` variants: plain / `query` (journal=false, undoable=false) / `noundo`). Registry `command_specs()` at `cmd/mod.rs:104-126`. Gates: `always`, `has_doc`, `can_undo`, `can_redo`, `has_clipboard` (`cmd/mod.rs:72-89`). Multi-doc sessions (`Session::documents()`, `window.activate`).

| Family (file → specs, N=315) | Registered IDs (complete) | Effect | Deps | Notes |
|---|---|---|---|---|
| `cmd/file.rs` (10) | `file.new`(noundo,Cmd+N,`{sample?}`), `file.open`(Cmd+O; path or `{name,base64}`), `file.save`(Cmd+S), `file.saveAs`, `file.saveBytes`(query→base64), `file.close`, `file.exportCsv`, `file.exportHtml`, `file.properties`, `window.activate` | external file I/O | always/has_doc | — |
| `cmd/edit.rs` (35) | `cell.set` `{cell,input,sheet?,array?}`, `selection.set/move/next`(noundo), `edit.selectAll`, `selection.currentRegion`, `selection.row`, `selection.column`, `edit.goTo`(Ctrl+G), `edit.goToSpecial`(kinds list), `edit.undo/redo` `{steps?}`, `edit.copy`, `edit.cut` (`{html?}`→`{range,text,html?}`), `edit.clearClipboard`, `edit.clearAll/clearFormats/clearContents/clearComments/clearHyperlinks`, `edit.fillDown/Right/Up/Left`, `edit.flashFill`(Cmd+E), `edit.find`(Cmd+F), `edit.beginEdit`(F2), `range.setValues`, `range.fill`, `edit.paste`, `edit.pasteSpecial`, `edit.autoFill`, `edit.fillSeries`, `edit.replace`, `edit.formatPainter` | mutation; cut/copy → internal clipboard + host HTML/text | has_doc | primitives `cell.set`/`selection.*`/`range.*` for agents |
| `cmd/format.rs` (48) | `home.bold/italic/doubleUnderline/strikethrough/superscript/subscript/fontName/fontSize/increaseFontSize/decreaseFontSize/fillColor/alignLeft/alignCenter/alignRight/justify/alignTop/alignMiddle/alignBottom/wrapText/shrinkToFit/increaseIndent/decreaseIndent/mergeCenter/mergeAcross/mergeCells/unmergeCells/accounting/percent/comma/increaseDecimal/decreaseDecimal/autofitRowHeight/autofitColumnWidth/defaultWidth/hideRows/hideColumns/unhideRows/lockCell`, `home.underline`, `home.fontColor`, `home.borders`, `home.formatCells`, `home.orientation`, `home.numberFormat`, `home.cellStyle`, `home.rowHeight`, `home.columnWidth`, `home.unhideColumns` | mutation on range/selection | has_doc | Excel ribbon id namespace (`home.*`) |
| `cmd/sheet.rs` (16) | `home.insertRows`, `home.insertColumns`, `home.deleteRows`, `home.deleteColumns`, `home.insertCells`, `home.deleteCells`, `home.insertSheet`(Shift+F11), `home.deleteSheet`, `sheet.rename`, `sheet.move`, `sheet.tabColor`, `sheet.hide`, `sheet.unhide`, `sheet.activate`(noundo), `sheet.next/previous`(Ctrl+PageDn/Up) | mutation | has_doc | — |
| `cmd/insert.rs` (37) | `insert.table`, `home.formatAsTable`, `table.totalRow`, `table.totalFunction`, `table.bandedRows`, `table.bandedColumns`, `table.headerRow`, `table.firstColumn`, `table.lastColumn`, `table.style`, `table.rename`, `table.convertToRange`, `insert.chart`, `chart.set`, `chart.switchRowColumn`, `chart.delete`, `insert.recommendedCharts`, `insert.sparkline`, `insert.picture`, `insert.shape`, `insert.textBox`, `shape.setText`, `insert.icons`, `insert.link`, `insert.symbol`, `object.setAnchorMode`, `object.delete`, `object.move`, `home.conditionalFormat`, `home.manageRules`, `home.clearRules`, `review.newNote`, `review.newComment`, `review.replyComment`, `review.resolveComment`, `review.deleteComment`, `review.showNote` | mutation | has_doc | cell-anchored objects w/ `AnchorMode` (`cmd/mod.rs:150-172`); conditional formatting via `cf.rs` |
| `cmd/data.rs` (18) | `data.sortAscending`, `data.sortDescending`, `data.sort`, `data.filter`, `data.filterBy`, `data.clearFilter`, `data.reapply`, `data.removeDuplicates`, `data.textToColumns`, `data.validation`, `data.validate`(query), `data.circleInvalid`(noundo), `data.group`(Cmd+Shift+K), `data.ungroup`, `data.hideDetail`, `data.showDetail`, `data.subtotal`, `data.flashFill` | mutation/query | has_doc | — |
| `cmd/formulas.rs` (17) | `formulas.autoSum`, `formulas.insertFunction`, `formulas.defineName`, `formulas.createFromSelection`, `formulas.deleteName`, `formulas.nameManager`(query,Cmd+F3), `formulas.calculationOptions`, `formulas.functions`(query: `{category?,search?}→[{name,category,signature,description}]`), `formulas.tracePrecedents/traceDependents`(query), `formulas.removeArrows`(noundo **no-op** — anomaly), `formulas.showFormulas`(Ctrl+`), `formulas.errorChecking`(query), `formulas.evaluateFormula`(query), `formulas.evaluate`(query), `formulas.calculateNow`(F9), `formulas.calculateSheet`(Shift+F9) | mutation/query; recalc engine `crates/calc` | has_doc | function catalog lives in `crates/functions`; `formulas.functions` is the real provider |
| `cmd/review.rs` (5) | `review.protectSheet`, `review.unprotectSheet` `{password?}`, `review.protectWorkbook`, `review.workbookStatistics`(query), `review.checkAccessibility`(query) | mutation/query | has_doc | — |
| `cmd/spelling.rs` (4) | `review.spelling`(query,F7), `review.addToDictionary`, `review.changeSpelling`, `review.thesaurus`(query) | query/mutation | has_doc | dictionary-based |
| `cmd/view.rs` (21) | `view.freezePanes`, `view.freezeTopRow`, `view.freezeFirstColumn`, `view.unfreezePanes`, `view.gridlines`, `view.headings`, `view.showZeros`, `view.rightToLeft`, `view.zoom`, `view.zoomToSelection`, `pageLayout.margins`, `pageLayout.printArea`, `pageLayout.printTitles`, `pageLayout.scaleToFit`, `pageLayout.breaks`, `pageLayout.headerFooter`, `pageLayout.theme`, `pageLayout.orientation`, `pageLayout.size`, `pageLayout.printGridlines`, `pageLayout.printHeadings` | mutation/view | has_doc | — |
| `cmd/inspect.rs` (6) | `document.inspect`, `sheet.read` `{range?,formulas?,formatted?}`, `cell.get`, `selection.stats`, `history.list`, `app.commands` (all query) | read | has_doc | agent read surface |
| `cmd/pivot.rs` (23) | `insert.pivotTable`, `insert.recommendedPivotTables`, `table.summarizePivot`, `pivot.fields`(query), `pivot.fieldList`(query), `pivot.list`(query), `pivot.addField`, `pivot.removeField`, `pivot.moveField`, `pivot.valueSettings`, `pivot.filter`, `pivot.sort`, `pivot.group`, `pivot.collapse`, `pivot.layout`, `pivot.grandTotals`, `pivot.subtotals`, `pivot.reportLayout`, `pivot.style`, `pivot.refresh`, `data.refreshAll`, `pivot.changeSource`, `pivot.delete` | mutation/query | has_doc | real pivot engine `engine/src/pivot.rs` |
| `cmd/whatif.rs` (9) | `data.goalSeek`, `data.scenarioManager`(query), `data.scenarioAdd`, `data.scenarioShow`, `data.scenarioDelete`, `data.scenarioSummary`, `data.dataTable`, `data.advancedFilter`, `data.consolidate` | mutation | has_doc | — |
| `cmd/print.rs` (4) | `file.exportPdf`(noundo), `file.print`(noundo), `file.printPreview`(query), `pageLayout.pageSetup` | external (PDF bytes → host) | has_doc | web path returns base64 → `services.download` (`lib.rs:266-277`) |
| `cmd/draw.rs` (5) | `draw.pen`(noundo), `draw.eraser`(noundo), `draw.lasso`, `draw.stroke` `{points,color,width}`, `draw.inkToShape` | mutation | has_doc | ink→shape conversion exists |
| `cmd/extra.rs` (57) | `chart.addElement`, `chart.quickLayout`, `chart.changeColors`, `chart.changeType`, `chart.selectData`, `chart.move`, `chart.formatSelection`, `chart.shapeFill`, `chart.shapeOutline`, `arrange.bringForward`, `arrange.sendBackward`, `arrange.align`, `arrange.group`, `arrange.rotate`, `arrange.selectionPane`(query), `data.fromTextCsv`, `data.getData`, `data.fromTableRange`, `data.queriesConnections`(query), `automate.newScript`, `automate.recordActions`(noundo), `automate.allScripts`(query), `automate.runScript`, `automate.deleteScript`, `view.macros`(query), `insert.checkbox`, `cell.toggleCheckbox`(Space), `pageLayout.themeColors`, `pageLayout.themeFonts`, `pageLayout.themeEffects`, `pageLayout.background`, `home.drawBorder`, `home.newCellStyle`, `formulas.useInFormula`, `home.addIns`(query, honest empty: "GridCraft extends through MCP and scripts instead of add-ins"), `formulas.recentlyUsed/financial/logical/text/dateTime/lookupReference/mathTrig/moreFunctions`(query → `formulas.functions`), `formulas.watchWindow`(query), `view.normal`, `view.pageLayout`, `view.pageBreakPreview`, `view.customViews`, `view.split`(noundo — approximated by frozen panes, honest note in params), `view.newWindow`(noundo — linked copy), `view.arrangeAll`(**no-op**), `view.hideWindow`, `view.unhideWindow`, `view.switchWindows`(query), `view.ruler`(**no-op**) | mixed | has_doc | `automate.*` = command-record/replay script store (macro substrate) |

### GridCraft UI-only commands (`run_ui_command`, `ui-egui/src/lib.rs:291-354`)

`ui.ribbonTab`, `view.formulaBar`, `view.collapseRibbon`, `view.darkMode`, `view.theme` (`{mode: system|light|dark}`), `view.zoom100` (delegates to engine `view.zoom`), `app.language.set` (`{language: en|es|zh|ja|ko|ru|pt}`), `app.language.english`, `app.language.japanese`, `ui.dialog` `{name}`, `ui.message.dismiss`. Plus `UiRequest::{Dialog,Message,EditCell}` raised by engine commands (programmatic calls never open dialogs).

### GridCraft gestures / pointer tools

- Grid canvas (`ui-egui/src/grid.rs`): cell click/drag range select, **fill handle** drag → `edit.autoFill` and double-click fill-down (`grid.rs:1442`), column/row header drag-resize, header select, text-overflow rendering.
- Sheet tab strip `tabs.rs` (activate/reorder/add via `sheet.*`); chart select (`selected_chart`); text-box editor `text_box.rs`; formula bar `formula_bar.rs` + `formula_locale.rs` (localized formula display); cell pictures `cell_pictures.rs`.
- Ink: `draw.pen/eraser/lasso/stroke` strokes over the sheet.
- Control-level pointer: `ui.cell.click`, `ui.cell.drag` (address→pixel resolution), `ui.click`, `ui.drag`, `ui.move`, `ui.key`, `ui.text`, `ui.ribbon`, `ui.dialog`, `ui.dialog.set`, `ui.dialog.confirm`, `ui.dialog.cancel`, `ui.message.dismiss`, `ui.edit.begin/commit/cancel`, `ui.screenshot`, `ui.resize`, `ui.set`, `app.open`, `app.save`, `app.quit`, `engine.execute`, `engine.commands`, `document.inspect`, `sheet.read`, `cell.get`, `ui.inspect` — `control.rs:1-17`.

### GridCraft formats, hooks, surfaces

| Item | Detail | Evidence |
|---|---|---|
| Import | xlsx/xlsm/xltx/xltm (native), xlsb, ods (sniffed by content), csv/tsv/tab/txt, json/scjson; **encrypted → typed error; legacy .xls → typed error** | `engine/src/io.rs:11-91`, `crates/xlsx` (read/sheet_read/fmla/array_formula/cfb/xlsb/ods/…) |
| Export | xlsx, csv, tsv, json, html; pdf via `crates/pdf` (`file.exportPdf`) | `io.rs`, `cmd/file.rs`, `cmd/print.rs` |
| Clipboard | internal clipboard + host `copy_html` service (styled HTML table when small) + `ctx.copy_text` plain | `ui-egui/src/lib.rs:251-264` |
| Services | `pick_open`, `pick_save`, `pick_picture`, `pick_picture_async`, `open_url`, `copy_html`, `download`, `open_async`+`inbox` | `lib.rs:106-123` |
| MCP tools (20) | `execute_command`, `list_commands`, `inspect_workbook`, `read_range`, `write_range`, `set_cell`, `get_cell`, `evaluate_formula`, `select`, `format_range`, `insert_chart`, `create_table`, `sort_range`, `filter`, `open_workbook`, `save_workbook`, `new_workbook`, `undo`, `screenshot`, `list_functions` | `mcp/src/tools.rs:110-370`, dispatch `:564` |
| Status providers | status bar: mode `Ready`/`Edit` (i18n'd), transient toast/message, selection stats (Sum/Count/Average/Min/Max via `selection.stats` provider), zoom % + slider/dialog, sheet tabs; `ui.inspect` exposes editor state, dialog, scroll, `documents[]` w/ dirty, `perf{frameMs,gridMs,fps}` | `ui-egui/src/tabs.rs:189-` (status_bar), `control.rs:66-82` |
| Localization | real i18n (en/es/zh/ja/ko/ru/pt), formula-locale display | `ui-egui/src/i18n.rs`, `i18n/` |

### GridCraft anomalies

- **Honest-stub commands** (return ok but no real function): `view.arrangeAll`, `view.ruler`, `formulas.removeArrows` (`ok()` bodies, `extra.rs:125,129`, `formulas.rs:52`); `view.split` approximates with frozen panes (self-documented in params `extra.rs:123`); `view.newWindow` is a linked copy not a second window (`extra.rs:124`); `home.addIns` returns empty + hint (`extra.rs:227`).
- No `.xls` legacy-binary or encrypted-workbook support — typed errors, not silent failure (`io.rs:43-52`). Fidelity gap to Excel.
- `Mode::Ready/Edit` gates some UI commands — host must map mode state (`control.rs:78`).

---

## Cross-app summary for ST-02/ST-11 planning

- All three share the same command model: registry of `CommandSpec` (doc-string params, `enabled` predicate returning disabled *reason* strings, `run(Session, Json)`), one `Session::run/execute` entry behind a panic guard, journal/undo flags, and a JSON-lines control channel + MCP facade + CLI. Promotion to a Qualia contract (ST-02) is structural: wrap `params` doc strings into real JSON Schemas; preserve IDs verbatim (`home.*`/`format.*`/`slide.*` families already stable).
- Registered command counts (code-verified): **WordCraft 460, DeckCraft 226, GridCraft 315**. UI-only command sets: WordCraft ~14 `ui.*` verbs, DeckCraft 31 `UI_COMMANDS`, GridCraft ~11 `ui.*`/`app.*`/`view.*` UI verbs.
- External-effect command surfaces needing grant scoping (ST-06/08): file open/save/export, clipboard (incl. image), print hook, browser download, `open_url`/community links, TTS subprocess spawn (WordCraft), media audio output (DeckCraft), Zotero socket `127.0.0.1:23116` (WordCraft), control-channel TCP listener + key-file auth.
- Real status providers exist in all three (`ui.inspect` returns live documents/dirty/perf; status bars) except **hardcoded language strings** in WordCraft/DeckCraft status bars.
- Provenance uniform: source snapshot, no VCS history; embedded heads recorded from `contributors/contributors.json`.
