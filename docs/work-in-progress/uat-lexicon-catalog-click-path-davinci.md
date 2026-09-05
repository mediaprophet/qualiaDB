# WIP — UAT click-path: lexicon Catalog / held-gate (davinci)

**Status:** work-in-progress · **Tip under review:** `07ea593` · **Bind:** `GraphDatabase.lexicon_manifest`
**Audience:** Capt UAT · monet motion · Neo wire · Vibe deltas
**Neo land:** `code` containers mount `build_ide_view` (Zone D Catalog · Lexicon). Vibe-console peer uses Script | Catalog · Lexicon tabs (same `GraphDatabase.lexicon_manifest`).

## Finding (chrome review, daemon down)

1. **`build_ide_view` Zone D (Vibe REPL · Problems · Catalog) is not mounted in the live manifold shell** — `build_ide_view` has **no call sites** outside `ide.rs`. So the Zone D **Catalog** tab cannot be reached from Everyday / Workshop today.
2. **Live mount:** `build_vibescript_console` (`container_inline_views.rs`) appends `lexicon_bay::build_lexicon_bay` under the editor/output — both paths must keep the **same** bind `GraphDatabase.lexicon_manifest` (Capt item 3).
3. On tip `07ea593`, Script/Code containers open a floating **VIBE** window with Layout · Stage · Timeline chips + Diagnose/Run; the Catalog peer is easy to miss **below the fold** (scroll the container body).

## Click-path for UAT (vibe-console peer — live today)

1. Open Poet → Everyday (or Getting started).
2. Place **Script** / `code:place_vibe` / Code container so a **vibe-console** body mounts.
3. Focus that container; **scroll below** Run/Diagnose + editor + output.
4. Catalog peer should show:
   - daemon-down / missing / E300 → **held / not yet — open lexicon pack** (never “broken”)
   - chips: **living** · **artifact** · **machine** (warm / crisp / muted)
5. When `:4242` is up: success → arrive card with `packSemVer` + framing.

## Click-path for Zone D Catalog (after Neo wires)

**Blocker:** mount `build_ide_view` (or extract Zone D drawer) into a reachable Code IDE habitat — not only the floating vibe-console.

Proposed human path once wired:

1. Tool chest → **CODE IDE & VIBE REPL** → open full IDE habitat (not only a Script cell).
2. Zone D bottom studio-bay → tabs **Vibe REPL** · **Problems** · **Catalog**.
3. Click **Catalog** (`data-bay-tab="catalog"`) → same `build_lexicon_bay` / same `lexicon_manifest` bind.

## Chrome accept (both surfaces)

| Check | Accept |
|-------|--------|
| Same bind | Catalog tab **and** vibe-console peer both call `GraphDatabase.lexicon_manifest` |
| Held-gate | held / not yet + open lexicon pack; no “broken” |
| Chips | living / artifact / machine; mixed splits; no Thing-wash |
| Twins | Layout · Stage · Timeline remain on script chrome |
| Stamp | Prefer `0.0.36-dev` in header (UAT watch: tip showed `0.0.35-dev`) |

## Ask Neo — done

1. Wired: `containers.rs` `"code"` → `build_ide_view` (Zone D reachable).
2. Vibe-console: Script | **Catalog · Lexicon** tab strip (not below-fold only).
3. Both call `lexicon_bay::build_lexicon_bay` → `GraphDatabase.lexicon_manifest`.

## Out of scope

Host widen · WordNet engine · G-COORD · Solid
