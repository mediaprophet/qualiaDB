# WIP — qualia-core-db capability uplift audit + exposure plan

**Status:** work-in-progress · **Not standards** · **Branch:** `0.0.36-dev` · **Reviewed tip:** `5c758e63` · **Freeze:** `vibe-host-0.1` @ `6dc2b8b8`
**Owner:** Neo (seams/audit) · Triage: Vibe · Chrome: davinci/monet · Shapes: Marvin · Ops: Capt.
**Constraint:** design markdown only from this crew while G-COORD coding advances elsewhere — this doc is the uplift programme so capabilities are **exposed**, not reimplemented or left dark.

## Why this exists
`crates/qualia-core-db` already contains a large implemented surface (SPARQL/SHACL/N3, volumes, modalities, inference, render, econ, ML, clinical, HID, cosmic, …). Poet/`ALL_BOUND` only expose a fraction end-to-end. Risk: rebuild what exists, or ship UI that never reaches live `Capability.method` ids.

## Snapshot at tip `5c758e63` (counts)
| Signal | Approx |
|--------|--------|
| `Capability.method` consts in `poet_host/invoke/ids.rs` | **~887** unique |
| Distinct capability **families** | **~99** |
| `CAPABILITY_DESCRIPTORS` families in `lib.rs` | **~28** (discovery metadata lag) |
| `poet_host/invoke/*.rs` modules | **~166** |
| Rich `src/` areas beyond invoke | q42, sparql_library, modalities, inference, render, medical, nlp, tensor, wasm_*, governance, identity, hypermedia, … |

### Largest `ALL_BOUND` families (by method count)
Econ (~106) · Statistics (~97) · MachineLearning (~79) · Research (~73) · Render (~53) · Asset · Cosmic · NumberTheory · Physics · ComputationalGeometry · Scene · HID · Audio · Image · …

### Discovery gap
Descriptors name ~28 families; live catalog has ~99. **Uplift work includes descriptor/catalog honesty**, not only new code.

## Exposure matrix (programme)
For each family: **Implemented in core-db?** · **In `ALL_BOUND`?** · **Invoke handler?** · **Poet toolchain / REPL reachable?** · **Docs/fixtures?** · **Action**

Statuses: `exposed` · `catalog-only` (in ALL_BOUND, thin/no UI) · `dark` (code in crate, missing/weak bind) · `duplicate-risk` (UI inventing parallel path) · `parked`

### Already exposed / recently advanced (do not reopen blindly)
- GraphDatabase.sparql · volume_open/volume_commit · SHACL.validate · N3Logic.evaluate (office:shapes) · Cosmic.* (G-COORD slice) · vibe-host four-ops · Poet IDE Vibe REPL (`ide.rs` eval_repl) · spec-tools / tool-chest fill-outs at tip

### High-value uplift candidates (prioritize exposure over rewrite)
1. **Inference.*** — chrome/provenance already designed; ensure every live method has tool or REPL recipe
2. **Render.*** / Scene / Video / ThreeD / Image / Audio — media toolchain wishlist; bind before new engines
3. **Modalities / Deontic / Epistemic / Paraconsistent / Temporal** — logic showcase exists; Poet epistemic toolchains must call live ids
4. **HID / Animation / Pulse** — LocalHost/catalog kernels; REPL demos already probe Animation.*
5. **ClinicalRisk / medical / Chemistry** — sanctuary fail-closed; SHACL-first for living subjects (B-OWL-*)
6. **Econ / Statistics / ML / Research** — huge catalog; expose via REPL playbooks + gated toolchains, not silent dark methods
7. **Identity / DID / VC / governance** — dual-VC honesty; Solid exit-adapter only when unparked
8. **wasm_llm / tensor / gguf_*** — document as native/wasm capability islands; no Host widen

## Matrix columns (Marvin — add on next refresh)
1. **Framing** — `living-SHACL` · `artifact-OWL` · `mixed` (from `shacl-first-vs-owl-ok-class-list.md`)
2. **Uplift?** — for life-science / clinical / ecology families: `native` · `needs-OWL-uplift` · `parked` (B-OWL-LIFE-UPLIFT)
3. **REPL recipe?** — yes/no (catalog-only families should get playbooks before new toolchains)

## Family framing hints (don’t reopen engines)
- ClinicalRisk / medical / Chemistry (living subjects) → SHACL-first subjects; instruments/datasets OWL-ok
- Cosmic / G-COORD Position-on-cells → mixed (coords artifact; *what* is placed may be living)
- Econ / Research about persons → living-safe copy; ledger artifacts OWL-ok
- Inference / Provenance → structure OWL-ok; claims about persons/living SHACL-first
- Render / Scene / HID / Animation → artifact chrome; never Thing-label living content in recipes
- Identity / VC → dual-VC honesty; persons not under Thing

## REPL ontology locks (agree davinci/monet)
- Catalog insert must tag framing so stubs don’t default to “thing” wording
- Sanctuary recipes cite `q42:Volume` states; diagnose copy follows class list
- Prefer InvokeId annotation pack (wishlist §E) so REPL/catalog never drift from `ALL_BOUND`

## Priority agree
UAT (graph+volume) → deepen REPL/catalog as exposure channel for dark/catalog-only → Capt. pick next toolchain. No Host invent; no reimplement of sparql/shacl/volume.

## Stage plan
### Stage 0 — Inventory freeze (docs)
1. Generate machine list: family → method count → handler module path (from `invoke/` + `ids.rs`).
2. Diff vs Poet `capability_scope` / spec-tools dispatch / REPL recipes.
3. Publish matrix table in this WIP (append); tip-lock SHA each refresh.

### Stage 1 — Dark → catalog-only
For implemented handlers missing `ALL_BOUND`: add ids **only** when handler exists (no invent). Prefer remap over new dotted IRIs.

### Stage 2 — Catalog-only → Poet reachability
Per Capt. pick: either next inventory toolchain **or** REPL playbook pack (see sibling REPL doc). Every visible button = live id or gated.

### Stage 3 — Descriptor / DevRel lockstep
Grow `CAPABILITY_DESCRIPTORS` toward live families; Vibe diagnose/DevRel never documents dark APIs as shipped.

### Stage 4 — Living/created cut
When exposing clinical/life-science/econ-about-persons: follow `shacl-first-vs-owl-ok-class-list.md` + B-OWL-LIFE-UPLIFT (uplift OWL, don’t Thing-swallow).

## Acceptance
- [ ] Matrix covers all ~99 families with status
- [ ] No Poet chrome invents parallel APIs for an existing Capability.method
- [ ] Top 15 families by count have either exposed path or explicit parked reason
- [ ] Overnight agents update matrix on tip move — no silent reimplementation

## Out of scope
Reimplementing SPARQL/SHACL/volume stacks · Host widen · dotted `qualia.*` · Webizen Desktop (later) · Solid IdP until Capt. unparks
