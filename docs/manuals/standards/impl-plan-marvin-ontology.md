# Impl plan — Marvin / ontology (shapes · vocab · joins)

**Owner:** Marvin · **Seam commits:** Neo · **Language triage:** Vibe · **Chrome pair:** davinci/monet
**Frozen surface:** `vibe-host-0.1` @ `6dc2b8b8` · **Plans tip:** `7318a049`+ · **Branch:** `0.0.36-dev`
**North star:** Shared vocabularies and class/property/relation models so QualiaDB, Poet, and vibe script read the same world — human chrome machine-readable underneath.
**Rules:** shapes join only live `ALL_BOUND` / `vibe:InvokeId` · no Host invent · no dotted `qualia.*` IRIs · gaps → @Vibe → `vibescript-sprint-deltas.md` · script hot-edit must never force host rebuild

## Modeling rule — persons & sacred/human relations (locked)
- Do **not** model human beings, personhood, love, kinship, or related “world of God” concepts as subclasses of `owl:Thing` (or otherwise under OWL’s Thing hierarchy in a way that commodifies them).
- Prefer **SHACL** shapes (constraints without forcing Thing-taxonomy) + existing `core-ontologies` agency/values/jural vocab.
- OWL/`owl:Thing` remains fine for technical/system artifacts (volumes, invoke ids, containers-as-software, CRS, etc.) where “thing” framing is appropriate.
- Stage 1+ publish docs must state which classes are SHACL-first vs OWL-ok.

## Done (do not reopen)
- Inventory of `qualia-core-db` + SHACL / DID / modalities / volumes / invoke catalog (Capability.method truth)
- G-POET-TOOLCHEST: Container · Manifold · Link join to `office:graph` → `GraphDatabase.sparql`
- G-B-001: `q42:Volume` shape → `docs/manuals/standards/q42-volume-shape-G-B-001.md`
- G-A accept @ `6dc2b8b8` — shapes lock to thin facade only
- G-COORD v0 sketch parked (CoordinateSystem · Realm · Position)
- G-SOLID-IDP shapes parked (SolidIdP · WebID · DomainLink · SolidSession)

## Stage 0 — Hygiene (docs only)
1. Sync this plan + Volume cite; mark Marvin row landed in `impl-plans-INDEX.md`.
2. One-page ontology join contract: every Poet surface cites `vibe:InvokeId` from `ids.rs` / `catalog_ttl.rs` — never aspirational dotted IDs.
3. Index existing packs: `shapes/*.shacl.ttl` · `core-ontologies/` · `ontologies/` · `bundled/ontologies/` — reuse before invent.
**Accept:** contract + index on-branch; no new Host/API.

## Stage 1 — Container · Manifold · Link (publish)
1. Publish `poet-container-manifold-link-shapes.md`: Container (content-shaped; optional Volume/Position; twin refs) · Manifold (nests; optional shared CoordinateSystem) · Link (typed semantic ends).
2. Spatiotemporal attrs first-class on content (incl. language cells).
3. Join notes for davinci/monet chrome.
**Accept:** markdown on GH; chrome can cite classes without inventing binds.

## Stage 2 — Volume shape hardening
1. Keep `q42-volume-shape-G-B-001.md` SoT; SHACL NodeShape only if Neo wants under `shapes/`.
2. States closed·open·committed·denied·fault align sanctuary fail-closed + wasm E300.
3. Manifold backing-store when Container sits on Volume.
**Accept:** chrome states 1:1 with ontology; no fake durable success.

## Stage 3 — Layout · Stage · Timeline (ontology)
1. Twin classes ≠ legal `FormationStage`: Layout (2D) · Stage (depth/z/camera) · Timeline (entrance·dwell·exit only).
2. 1:1 every UI surface; named beats only.
3. Join remaps: sparql · Inference.* · Render.* · volume_commit.
**Accept:** shape doc on-branch.

## Stage 4 — G-COORD (when Capt. unlocks)
1. Publish `g-coord-coordinate-system-shapes.md`: CoordinateSystem · Realm (Earth/Cosmos/Fictional/Speculative/Viewpoint) · Position.
2. Ground GeoSPARQL / temporal / `did:q42` locus (≠ person).
3. Extensible toward DNS/IP later — v0 must not claim network replacement.
4. Neo thinnest bind after; until then shapes-only + gated map chrome.
**Accept:** shapes on GH; bind via Neo; dialect via Vibe.

## Stage 5 — Catalog honesty joins
1. Dual-VC split (VCDM+ML-DSA vs quin+Ed25519).
2. QISP join notes to `sparql_library/immersive/`.
3. Annotate to live InvokeIds (`SHACL.validate`, `N3Logic.evaluate`, …).
4. Bridge aspirational → `ALL_BOUND` only (with Vibe).
**Accept:** deltas closed or dated defer.

## Stage 6 — G-SOLID-IDP (parked)
Activate only after Neo IdP/WebID/domain binds + Capt. unpark. QualiaDB-as-IdP; no external pod.
**Accept:** parked; zero mid-Poet churn.

## Stage 7 — Webizen Desktop ontology prep
Reuse checklist for Container/Manifold/Volume/COORD; docs-only until Webizen gate.
**Accept:** checklist; no Solid here.

## Sleep / continuation protocol
1. Start at first unchecked stage; re-read freeze `6dc2b8b8`, INDEX, Volume shape, inventory, vibe sprint-B.
2. Push markdown to GH before large TTL/SHACL churn — Neo owns remote commits.
3. Missing bind → gated join + Vibe row — never invent Host/dotted IRIs.
4. Prefer extending existing `shapes/` + `core-ontologies/` over parallel vocabs.

## Dependencies
Frozen facade @ `6dc2b8b8` · live GraphDatabase.sparql / volume_* / Inference.* / Render.* / SHACL.* / N3Logic.* · sibling plans (neo/vibe/davinci/monet) · Volume shape doc

## Out of scope
Host widen · dotted `qualia.*` · Solid this sprint · DNS/IP claims · Layout/Stage as Host IDs · mid-flight API churn · fake durable/preview
