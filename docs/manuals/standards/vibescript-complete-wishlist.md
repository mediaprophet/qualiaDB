# Complete wishlist — VibeScript · Poet · QualiaDB (post `vibe-host-0.1`)

**Compiled by:** Vibe · **Seam push:** Neo · **Freeze:** `vibe-host-0.1` @ `6dc2b8b8` · **Plans tip:** `1add316e` · **Amended tip:** (see commit) — davinci/monet/Marvin extras folded
**Branch:** `0.0.36-dev`
**Rules:** no Host widen · no dotted `qualia.*` · live `ALL_BOUND` / `vibe:InvokeId` only · hot-edit must never force host rebuild · gaps → `vibescript-sprint-deltas.md`

## North star
Vibe is the no-compile JS alternative for QualiaDB / Webizen Desktop / Poet. Humans and agents share one language; Poet is a live studio, not a code shell.

---

## A — Language / DevRel (Vibe)
1. Diagnose-first DevRel: every error has `suggested_fix` + evidential where contradictions apply.
2. Fixture pack: graph · volume · infer · render · diagnose loops under `crates/vibe/fixtures`.
3. Span fidelity to cell/token for monet error glow (cross-frame for Timeline).
4. Human↔agent dialect guide (`using`/`effect fn` ↔ `capability.invoke`).
5. G-COORD dialect: realm-scoped Position (Earth/cosmos/fiction/speculative/POV) — extensible, no DNS/IP claim yet.
6. Preview handle kinds still/clip/scene on live `Render.*` (sibling op only if one handle can’t carry three).
7. Catalog honesty: dual-VC split, QISP surfacing, ledger vs showcase, aspirational→`ALL_BOUND` bridges.
8. Empty typography + custom unicode API: real accept criteria or honest gated B rows.
9. Webizen Desktop language checklist (same four-ops; parity tests) — after Poet.
10. G-SOLID-IDP DevRel (QualiaDB-as-IdP, no external pod) — parked until Capt. unparks.
11. InvokeId annotation pack consumed by diagnose/DevRel (chrome/dialect never drift from `ALL_BOUND`).
12. Position (+ optional ViewpointRealm) on vibe cells/modules — language is spatiotemporal content, not only maps.

## B — Seams / Rust (Neo)
1. Keep thin `poet::vibe_host` + wasm parity sacred; never leak Host/AST.
2. Next toolchains from inventory → `ALL_BOUND` end-to-end (or gated).
3. Thinnest G-COORD bind after Marvin shapes.
4. Custom unicode / typography API if product-critical — else stay B.
5. Webizen Desktop host prep using frozen surface.
6. Solid IdP thin binds (start/callback, WebID, domain) — parked.
7. Overnight: Stage 0 of `impl-plan-neo-G-A-followon.md` first.

## C — Poet chrome / UX (davinci)
1. office:graph toolbar polish (human glyphs, gated honesty).
2. Next inventory toolchain chrome (live bind preferred).
3. Volume dock states from `q42:state` (open/committed/denied/fault).
4. Container · Manifold · Link human chrome (not engineer graph-viz).
5. Map containers: OSM-class geo layers + universe/fantasy realms + temporal scrubber.
6. Inference provenance chrome; Render preview dock (still/clip/scene).
7. Layout·Stage·Timeline twin coverage on every surface; named beats only.
8. Webizen chrome handoff docs after Poet stages land.
9. **Media / post toolchain** — import stills·clips·scenes, scrub renders, cache/status chrome on live `Render.*` / volume (post-prod lane, not a code UI).
10. **Keyboard-first toolchains** — Word/Excel-class focus order + shortcuts so toolbars are usable without hunting glyphs.
11. **Commit history cue** — lightweight open→commit timeline on sanctuary volumes (gated until a live history bind exists; never fake).
12. **Heavy-graph / map performance** — chrome stays responsive when Stage has dense geo or cosmos layers (progressive disclosure, not one mega canvas).

## D — Visual / motion (monet)
1. Motion contract doc: entrance / dwell / exit + gated ≠ broken.
2. office:graph icon + error glow polish.
3. Volume state looks; commit beat only on real success.
4. Inference provenance trails; preview dock handle-kind chrome.
5. Distinct visual language for Container / Manifold / Link + spatiotemporal badges.
6. Map/G-COORD skins (geo vs realm) after shapes+bind.
7. Twin coverage checklist; export motion contract for Webizen later.
8. **Accessible motion** — respect reduced-motion: same named beats, shorter/crossfade variants; never rely on motion alone for state.
9. **Dark/light + contrast tokens** — one visual token sheet so toolchest, volume dock, and map skins stay coherent (human chrome, not theme sprawl).
10. **Touch/dense targets** — toolbar and map scrubber hit areas sized for pen/touch, not only mouse.
11. **Provenance density control** — inference trails collapsible (full trail ↔ summary chip) so Stage stays readable on heavy graphs.

## E — Ontology (Marvin)
1. Publish Container · Manifold · Link shapes (+ spatiotemporal attrs).
2. Volume shape hardening / optional SHACL NodeShape.
3. Layout · Stage · Timeline ontology (≠ legal FormationStage).
4. G-COORD shapes publish when unlocked; ground GeoSPARQL/temporal/`did:q42` locus.
5. Dual-VC + QISP + InvokeId annotations; aspirational bridge with Vibe.
6. G-SOLID-IDP shapes — parked.
7. Prefer extending `shapes/` + `core-ontologies/` over parallel vocabs.
8. **InvokeId annotation pack** — SHACL/OWL annotations linking Container/Manifold/Link/Volume/twin classes to concrete `vibe:InvokeId` strings (generated from `catalog_ttl.rs` where possible).
9. **Provenance as first-class graph** — lightweight Provenance/Claim shapes that Inference trails and dual-VC presentations can both cite (not a second Host).
10. **Spatiotemporal on language cells** — Position (+ optional ViewpointRealm) as properties of vibe cells/modules, not only map containers.
11. **Shape test fixtures** — small TTL/N3 fixtures validating Volume states + Container↔Volume backing + twin 1:1 constraints (overnight agents can run without Host widen).
12. **Persons & sacred/human relations (locked)** — do **not** hang humans, personhood, love, kinship, or related “world of God” concepts under `owl:Thing` / OWL Thing hierarchy (avoids commodity framing). Prefer **SHACL** + agency/values/jural vocab; OWL/`owl:Thing` OK for technical/system artifacts. Stage publish docs must mark SHACL-first vs OWL-ok.
13. **Created vs living/natural (locked, extends #12)** — SHACL/non-Thing for the natural/living world (what exists or grew as life — land, waters, creatures, seasons, country), not only persons/sacred relations. Keep mankind-created artifacts (OWL/`owl:Thing` OK where apt) distinct from living/natural existence (incl. sacred-for-peoples-without-book). Stage docs mark SHACL-first (person/sacred/natural) vs OWL-ok (technical). Nuance at shape · chrome · diagnose · realm — don’t flatten into one commodity taxonomy.

14. **Micro/macro + life-science OWL uplift (locked)** — living/natural spans microscopic→macroscopic (scale first-class, not Thing subclass). Uplift/convert historical life-science OWL (GO/OBO-style): living → SHACL-first; instruments/protocols/datasets → artifact/OWL-ok; IRI bridge + uplift provenance — do not adopt as `owl:Thing` taxonomy. Interop fixtures later without Host invent.

## Priority order (suggested)
1. Docs/hygiene Stage 0 across plans (already started)
2. Diagnose + fixtures + span fidelity (unblocks chrome trust)
3. Next toolchain (inventory) end-to-end
4. Volume polish (chrome ↔ `q42:state`)
5. Container/Manifold/Link publish + chrome
6. G-COORD shapes → bind → dialect → map chrome
7. Inference/Render preview
8. Webizen prep
9. Solid IdP (last)

## Sleep protocol
Overnight agents: re-read freeze `6dc2b8b8`, `impl-plans-INDEX.md`, this wishlist, lane impl plan; start first unchecked stage; push markdown before large code; missing bind → gate + Vibe row.

## Non-goals
Host widen · dotted `qualia.*` · fake durable save/preview · free tweens · DNS/IP replacement claims · Solid before Poet/Webizen · mid-flight API churn
