# WIP — Browser UAT: office:graph + sanctuary volume + lexicon held-gate

**Status:** work-in-progress (not standards) · **Freeze:** `vibe-host-0.1` · **Sync tip:** `64b21384` · **Branch:** `0.0.36-dev`
**Owner:** Vibe (diagnose/DevRel accept) · Chrome: davinci/monet · Seam: Neo · Shapes: Marvin
**Scope:** prove live binds after **G-LEXICON-0** first slice accept. G-COORD held until this UAT passes.

## Rules under test
- Thin four-ops only (parse/check/diagnose/invoke)
- Live ids: `GraphDatabase.sparql` · `volume_open` · `volume_commit` · `lexicon_manifest`
- Unbound/daemon/wasm/missing-pack E300 = **held / not yet** (gated), never stub-broken or fake-durable
- Diagnose copy: never call persons/living “things” (SHACL-first vs OWL-ok list)
- Shape: `docs/manuals/standards/lexicon-pack-shape-G-LEXICON-0.md`
- Bay chrome: `docs/work-in-progress/g-lexicon-0-bay-chrome.md`

## A — office:graph / sparql
1. Toolbar launches query without reading Capability strings
2. Happy path: results settle; dwell/exit beats feel human
3. Bad query: diagnose JSON has code · span · suggested_fix; monet glow hits cell/token
4. Daemon-gated / unbound: chrome explains, looks gated not broken
5. Hot-edit script reload needs no host rebuild

## B — sanctuary volume
1. `volume_open` (path/handle) → open state visible
2. `volume_commit` success → commit beat only then; state committed
3. Fail-closed deny → denied/fault; no “saved” celebration
4. Wasm/E300 path gated honest
5. Volume dock states match `q42:state` (closed·open·committed·denied·fault)

## C — Language / diagnose accept
1. `diagnose` before invoke catches effect/type issues on sample cells
2. suggested_fix actionable for agent loop
3. Spans stable enough for error glow (note if coarse → B row)
4. No Host/AST bleed into Poet chrome during UAT
5. `lexicon:` pin fixtures (Vibe) — missing pack → held / not yet + open-pack suggested_fix; OK pin recorded; alias row round-trip; living framing never rewritten as artifact

## D — Lexicon held-gate / bay (G-LEXICON-0)

**Click-path (tip ≥ this hygiene):**
1. Place a **code** container (Code IDE & Vibe REPL / Script) → full IDE habitat → Zone D → **Catalog · Lexicon**.
2. Lightweight vibe-console surfaces: top tabs **Script** | **Catalog · Lexicon** (same bay / same bind).
3. Held copy must read **held / not yet** + why “open lexicon pack”. A red honesty **missing** chip elsewhere is *not* the lexicon gate.
4. Header stamp must show **`0.0.36-dev`** (`crate::CRATE_STAMP`).

1. Call `GraphDatabase.lexicon_manifest` with missing path → **held / not yet** + “open lexicon pack” (never broken)
2. Valid `*.lexicon.json` (example under `docs/manuals/standards/lexicon-pack-manifest-example.json`) → arrive card shows `packSemVer` + framing chip
3. `.q42` without sidecar / bad volume → held / not yet
4. Framing chips: living warm · artifact crisp · machine muted; **mixed** splits; no Thing-wash
5. Catalog peer / Zone D Catalog tab surfaces gate + chips (listen-only this slice)
6. Upgrade recipe beats: arrive on open · hold on breaking-id list · leave on dismiss · **commit** only on real pack write (N/A this slice if listen-only)

## E — Pass / fail log (fill in session)
| ID | Step | Pass? | Tip SHA | Notes / delta row |
|----|------|-------|---------|-------------------|
| A1–A5 | PARTIAL | e070ffc7+ | office:graph chrome works; Capability.method strings still leak as primary labels (wishlist) |
| B1 | PASS (HTTP) | 0b30cb15 | create-on-open `created:true` quin_count:1 on `/workspace/qualia-data/uat-sanctuary.q42` |
| B2 | PASS (HTTP) | 0b30cb15 | separate `/invoke` volume_commit `written:1` — sticky PoetSnapshot; UI Save Checkpoint still open (dialog flaky) |
| B3–B5 | open | — | UI dock states / deny / wasm not re-scored this pass |
| C1–C4 | PASS | 07ea593+ | G-LEXICON-0 diagnose fixtures accept |
| C5 | PASS | 07ea593 | lexicon: pin fixtures green |
| D1 | PASS | e070ffc7 | held / not yet + open lexicon pack chrome |
| D2 | OPEN | 64b21384 | Native Connected cold-load PASS; Open pack arrive still incomplete — use full `crates/vibe/fixtures/lexicon/en-core.lexicon.json` (avoid truncated `/wor`) |
| D3 | open | — | |
| D4 | PASS | f1d34d03 | living/artifact/machine chips |
| D5 | PASS | f1d34d03 | Zone D Catalog · Lexicon |
| D6 | OPEN | — | arrive UI beat pending |

### Capt session 2026-09-05 (Sydney)
- Daemon: `:4242` `/health` (not `/healthz`); Poet trunk `:8080`; tip `0b30cb15` sticky + `f615f16` create-on-open ancestor.
- HTTP sanctuary open→commit PASS; lexicon_manifest bind PASS; UI arrive + Checkpoint Mode path entry still open todos (not fail).

## Out of scope this UAT
G-COORD bind · full WordNet engine · locale packs beyond en · next toolchain · Solid IdP · QDNF · new Host methods

## Sanctuary volume UAT (create-on-open)

`GraphDatabase.volume_open` with a missing/truncated path **creates** a seeded sanctuary `.q42` when `create` is true (default). Then:

1. `volume_open` path `/workspace/qualia-data/uat-sanctuary.q42` → opens (may `created: true`)
2. Resident graph has seed quin → `volume_commit` same path succeeds (sanctuary fail-closed no longer empty)
3. Lexicon arrive: Catalog · Lexicon Open pack → `crates/vibe/fixtures/lexicon/en-core.lexicon.json`

Set `create: false` to keep fail-closed on missing files.

## Sticky Poet host (HTTP /invoke)

Daemon `POST /invoke` keeps a **process-sticky** `PoetSnapshot` (same idea as desktop `Mutex<PoetSnapshot>`).
`volume_open` load + `volume_commit` must share that host — recreating `from_daemon()` per request caused empty-graph commit fail-closed across HTTP.

## Probe Connected (tip cc5ecb6d)

Poet : Connected immediately after engine-bearing `/health`; `/vibe/capabilities` refreshes in background. Health ports: 4242, 8000, 3030 (not 4243/8080). Per-port 2.5s timeout.

## Overnight continuity

See `docs/work-in-progress/OVERNIGHT_HANDOVER_2026-09-05.md` for scoreboard + remaining plan (tip `64b21384`).


**D2 PASS (2026-09-05):** Catalog Open pack arrive en-core@0.1.0 · mixed · tip `a06179c9` (davinci). Soft-rise → monet.

**D2 soft-rise PASS (2026-09-05):** monet scored arrive on tip `a06179c9`.

**Marvin framing PASS (2026-09-05):** mixed living+artifact · no Thing-wash · tip `a06179c9`/`9a1438d`.

**B-ui PASS (2026-09-05):** Capt tip `f45212c7353a10c7ed8522f33cf220423891b841`
- Native Connected (:4242)
- File → Save Checkpoint (Checkpoint mode; Pruned disabled)
- Durable path: `/workspace/qualia-data/uat-sanctuary.q42`
- Toast Checkpoint saved + `GraphDatabase.volume_commit` → footer **Volume: COMMITTED**
- HTTP corroboration: `volume_commit` → `written: 1` (revision 15)
- Note: dialog may show stale localStorage path (`uat-e070ffc7-…`); accept is COMMITTED + written:1
