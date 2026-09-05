# WIP — Ontology design notes (Marvin)

**Status:** work-in-progress · **Not standards** · **Freeze:** `vibe-host-0.1` @ `6dc2b8b8` · **Sync tip:** INDEX tip-lock / `d2b79211`+ · **Standards SoT:** `shacl-first-vs-owl-ok-class-list.md`
**Constraint:** no new coding from this crew while G-COORD advances elsewhere. Design markdown only. WIP lives under `docs/work-in-progress/`.

## Locked modeling cuts (cite in chrome/diagnose)
1. **B-OWL-PERSON** — persons / sacred-human relations → SHACL-first; never under `owl:Thing` as commodity.
2. **B-OWL-NATURAL** — living/natural existence vs mankind-created artifacts.
3. **B-OWL-LIFE-UPLIFT** — micro·meso·macro living scale; life-science OWL convert/uplift (IRI bridge + provenance), not wholesale Thing import.
4. Chrome/diagnose copy: “person / living / country” vs “tool / volume / file” — never call persons/living “things”.

## UAT accept (shapes) — fill beside Vibe’s checklist
- **B volume:** dock states align `q42:state` (closed·open·committed·denied·fault); Volume = OWL-ok artifact.
- **A/B copy:** no “thing” wording for persons/living in UI or `suggested_fix`.
- **Mixed Position:** if UAT shows spatiotemporal badges early, *what* is placed uses living-safe words; coords = technical detail.
- Note any chrome that flattens living content into artifact labels → delta row for Vibe.

## G-COORD — design ahead of bind (no code)
1. Shapes already: CoordinateSystem · Realm · Position (Earth/cosmos/fictional/speculative/POV).
2. Scale attr on living entities (micro·meso·macro) — G-COORD may cite without Thing subclassing.
3. Position-on-cells: language cells can carry Position + optional ViewpointRealm; subject living vs artifact per class list.
4. Realm members: mark living country/ecology vs artifact CRS/layer config explicitly (mixed realms).
5. Bind (when Grok/Neo lands): thinnest `ALL_BOUND` only; shapes stay SHACL-first for living subjects.
6. Keep QDNF / Solid out of coord shape copy.

## Created vs living (shape guidance)
| Kind | Framing | Examples |
|------|---------|----------|
| SHACL-first | living / person / sacred / natural | person, kinship, country, ecology-as-life, uplifted GO/OBO living entities |
| OWL-ok | artifact / machinery | Volume, InvokeId, Container/Manifold/Link *as software*, Layout/Stage/Timeline *as UI*, CRS machinery, datasets, instruments |
| Mixed | split carefully | Position, Realm, Provenance/Claim |

## Publish queue (after Capt. unlocks shape docs)
1. `poet-container-manifold-link-shapes.md` (standards, when settled)
2. Layout·Stage·Timeline ontology (≠ FormationStage; aspects, not twins, not planes)
3. Optional Volume SHACL NodeShape under `shapes/` only if Neo wants
4. Shape test fixtures (TTL/N3) for Volume states + Container↔Volume + aspect 1:1
5. InvokeId annotation pack + Provenance/Claim shapes (wishlist §E)

Until then: keep drafts in **WIP**; promote to `docs/manuals/standards/` only when Capt./Neo call them settled.

## Sleep / continuation
- Re-read freeze, INDEX tip-lock, class list, UAT checklist, this note before writing shapes.
- Missing bind → gated join + Vibe row — no Host invent, no dotted `qualia.*`.
- Prefer extending `shapes/` + `core-ontologies/` over parallel vocabs when coding resumes.

## Out of scope now
Implementation · next inventory toolchain · Solid IdP · Host invent · QDNF bleed into Poet/G-COORD copy · fake durable/preview semantics
