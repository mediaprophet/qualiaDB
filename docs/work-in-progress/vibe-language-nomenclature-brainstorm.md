# WIP — Vibe command language: nomenclature · form · multilingual brainstorm

**Status:** work-in-progress · **Not standards** · **Lead:** Vibe · **Freeze:** `vibe-host-0.1` @ `6dc2b8b8`
**Sync tip:** INDEX tip-lock · **Branch:** `0.0.36-dev`
**Constraint:** design markdown only while G-COORD / other code advances elsewhere. Promote to standards only when Capt. settles.

## Why now
Vibe is still soft enough to choose **fun, sayable, translation-ready** forms before they harden into muscle memory. Goal: intuitive · practical · kinder than hostile-systems languages · **not** JS/Java-shaped · Ruby *warmth* without Ruby clone · one story for humans and agents.

## Research anchors (external)
1. **Concept-core, surface-localize** — Multilingual Programming / keyword registries: lexer maps locale surface → semantic concepts; grammar stays concept-based; **identifiers stay Unicode as written** (UAX #31 / PEP 3131 spirit). Keywords translate; names don’t.
2. **Unicode identifiers** — already partially in vibe (`unicode_ident`); keep NFKC policy explicit for equality without killing script fidelity.
3. **Ruby warmth ≠ Ruby clone** — named args, speakable flow, developer happiness; avoid JS’s punctuation soup and Java’s ceremony. Don’t import Ruby’s everything-is-message ideology wholesale.
4. **Diagnose as pedagogy** — agent loop already `diagnose` → `suggested_fix`; nomenclature must make those fixes *sayable* in many languages.

## Locked product constraints (our tree)
- Hot-edit JS replacement over QualiaDB / Poet / (later) Webizen — script change never forces host rebuild
- Human dialect: `using` + `effect fn` · Agent: `capability.invoke("Family.method", {…})` — siblings, not two languages
- Effects: pure / hot / cold / async / effect · sanctuary fail-closed · wasm E300 honest
- SHACL-first copy: never call persons/living “things” (`shacl-first-vs-owl-ok-class-list.md`)
- REPL drawer = studio, not terminal (davinci/monet accepts)

## Design axes (brainstorm)
### 1. Concept glossary (canonical English *concepts*, not sacred English keywords)
Draft concept ids (illustrative — not freeze yet):
| Concept | Current surface | Soft alternatives to explore | Notes |
|---------|-----------------|------------------------------|-------|
| IMPORT_CAP | `using` | `with`, `invite`, `bring` | Warmth; avoid JS `import` |
| EFFECT_FN | `effect fn` | `effect`, `do`, `perform` | Keep effect visible |
| PURE_FN | `pure` / default | `still`, `quiet` | Contrast to effect |
| BIND_HOST | `capability.invoke` | `ask`, `call`, `please` (agent form stays machine-id) | Human phrase ↔ InvokeId |
| CELL | `= expr` | keep | Spreadsheet intuition |
| HOOK | `on path` | `when`, `upon` | Event speak |
| SANCTUARY | volume open/commit | `shelter`, `keep`, `commit` | Moral honesty |
| GATE | unbound/E300 | `held`, `not yet`, `closed` | Never “broken” |
| LIVING_REF | — | person/living/country words | Diagnose/DevRel |
| ARTIFACT_REF | — | tool/volume/file | Diagnose/DevRel |

### 2. Fun without cringe
- Prefer **short sticky pairs**: word + optional glyph (Poet toolbar) that survive glossary translation
- Avoid meme slang that won’t translate; prefer metaphor rooted in studio/sanctuary/stage (Poet twins)
- Allow **locale fun** in surface forms if concept id stays stable

### 3. Multilingual / morphology
- Grammar must not assume English SVO forever — concept AST first
- Plan for RTL, agglutination, case: keywords as *tokens*, not English phrases glued mid-line where possible
- Translation glossary artifact: `concept → {en, …}` checked into vibe-script tooling repo later
- Numerals / dates: note locale-aware literals as Stage-later (research: multilingual numeral types)

### 4. Human ↔ agent one lexicon
- Same verbs in REPL drawer, diagnose copy, and Capability families
- Catalog insert stubs use living-safe vs artifact framing tags (Marvin)
- `suggested_fix` templates localized via concept ids, not hard-coded English only

### 5. Effects & sanctuary voice
- Verbs that make **fail-closed** feel careful, not punitive
- Commit beat language only for real `volume_commit` success
- Pure vs effect should read as *stillness vs doing* in plain speech

### 6. Distance from JS / Java / Rust hostility
| Avoid sounding like | Prefer vibe vibe |
|---------------------|------------------|
| `const`/`let`/`var` soup | fewer binding keywords; cells + clear `let` if needed |
| `=>` / dense punctuation | named beats, pipelines already `\|>` — keep sparse |
| `public static void` | no ceremony |
| borrow-checker scolding | diagnose poetry + fix |
| `undefined` is fine | fail-closed honesty |

### 7. What else (open prompts for the team)
- Sound symbolism: soft verbs for living, crisp for artifacts?
- Emoji/unicode toolbar vs source keywords — source stays readable in plain text
- Alias layers: “playful surface” vs “formal/legal surface” for jural modules?
- Time words for Timeline beats (entrance/dwell/exit) — align monet motion names with language keywords?
- G-COORD: realm/position words that don’t colonize living country as “objects”

## WordNet → logic / SHACL executable lexicon (Timothy — principles now, expand later)
- **Opportunity:** lexical ontologies (WordNet-class sense nets) as a *meaning layer* beside vibe concepts — not a second Host.
- **Near-term principles (design only):**
  1. Senses and synsets map to **concept ids** / living-safe vs artifact framing — never force persons/living under `owl:Thing`.
  2. SHACL (and N3/modalities where live) can constrain *which senses are allowed* in a dialect or sanctuary context — executable *guards*, not a full NL compiler yet.
  3. Diagnose/`suggested_fix` can cite sense glosses in the active locale once glossary exists.
  4. Keep expansion staged: glossary hooks now → sense-tagged fixtures later → optional WordNet-bridge Capability.method only if already in `ALL_BOUND` or Capt. accepts a catalog add.
- **Non-goal now:** reimplementing WordNet inside Rust hosts; English-only sense IDs as sacred ABI.

## Neo — seam / freeze constraints (ABI vs dialect)
Under frozen `vibe-host-0.1`, **stable (rename costs a freeze bump):**
1. Four ops: `parse` · `check` · `diagnose` · `capability_invoke` (+ JSON diagnose shape fields)
2. Version stamps: `LANGUAGE_VERSION` (`vibe-0.1`) · `HOST_VERSION` (`vibe-host-0.1`)
3. Live `Capability.method` / `vibe:InvokeId` strings in `ALL_BOUND` — catalog-honest; no ghost names in `suggested_fix`
4. Effect classes & E300 fail-closed semantics (even if surface verbs get warmer aliases)
5. Cell lead-in `=` and core program/module structure until EBNF+§3 deliberately resync

**Freer to explore in dialect / glossary (without Host widen):**
- Surface aliases for `using` / `effect` / sanctuary voice (concept-core map)
- Locale keyword registries · diagnose template wording · REPL drawer chrome lexicon
- Toolbar glyphs · motion beat *labels* (must stay aligned with named beats, not free tweens)
- Soft synonyms that desugar to the same AST concepts

**Hard rules for any rename wave:** no Host invent · no dotted `qualia.*` · script hot-edit must not force rebuild · living/created copy lock · Capt.+Vibe+Neo agree before EBNF keyword ABI moves.

## Locked trio (all lanes — restitch)
- Motion/Timeline/chrome: **arrive · hold · leave** (= entrance / dwell / exit)
- Gates: **held · not yet · closed** (never "broken")
- Sanctuary: **keep · commit** (only on real durable success) · **sheltered** (fail-closed care)

## Team amends (restitch)

### davinci — studio voice
- Drawer = **studio bay** (not console/terminal); run = **play**; diagnose = **listen** / **check the room**; fix = **tune**; invoke = **ask the tool** (artifact) — never "operate on a thing" for living subjects.
- Toolbar sayables: short verb + glyph — `Ask graph` · `Keep volume` · `Play cell` · `Show stage` — Capability.method stays advanced/secondary.
- Twin language = arrive / hold / leave (Timeline + motion same words).
- REPL modes: **Safe** = listen-first · **Invoke** = play with effects visible · **Sanctuary** = keep/commit.
- Fun without cringe: studio/sanctuary/stage metaphors; avoid meme slang; locale can play if concept ids stay stable.
- Distance from JS: no `run`/`eval` as primary chrome labels; no `undefined`-vibes — prefer held / not yet / closed.
- Multilingual chrome: UI strings from concept glossary; every glyph has a sayable concept (not English-only tooltips).
- WordNet→SHACL (chrome): later = richer autocomplete / "say it another way" in the bay; until then don't expose WordNet as a Host — recipes cite live InvokeIds only.

### monet — fun vs clear · motion lexicon
- Fun = sticky metaphor (studio / sanctuary / stage), not joke slang — if it won't survive a translation glossary, cut it.
- Clear beats cute when they conflict; glyph always has a sayable concept (never icon-only meaning).
- Soft verbs for living subjects · crisp verbs for artifacts (sound symbolism ok if concept ids stay stable).
- Motion ↔ language: arrive / hold / leave shared with davinci.
- Reduced-motion copy: **still arrive / still hold / still leave** (or locale short forms) — never "animation off"; state isn't motion-dependent.
- Error glow speak: **mark** / **point** (not "throw" / "exception") — diagnose as tuning the room.
- Visual lexicon: gates held/not yet/closed; sanctuary keep/commit/sheltered; living-safe vs tool/volume/file — never "thing."
- WordNet→SHACL (visual): later synonym chips in the bay; design-only until executable lexicon lands — no invented Host APIs.

### Marvin — living·created lexicon · semiotics · epistemology
**Banned / prefer (diagnose · REPL · chrome · suggested_fix)**
- Never: *thing / object / entity* for persons, kinship, love, country, ecology-as-life, living beings (any scale).
- Prefer living: **person · people · kin · living · country · life · being** (locale surfaces later; concept ids stable).
- Prefer artifact: **tool · volume · file · recipe · bind · catalog** — not "thing" even for artifacts if a clearer word exists.
- Mixed Position: *what* is placed = living-safe words; coords = **place / where / when** (technical), never "object at coordinates."

**Concept ids to add (illustrative)**
| Concept | En surface seeds | Notes |
|---------|------------------|-------|
| LIVING_REF | person, living, being | SHACL-first |
| KIN_REF | kin, kinship | sacred/human |
| COUNTRY_REF | country, place-of-life | not commodity land-parcel by default |
| LIFE_SCALE | micro, meso, macro | B-OWL-LIFE-UPLIFT |
| ARTIFACT_REF | tool, volume, file | OWL-ok |
| KEEP | keep, shelter | sanctuary open |
| COMMIT | commit | only real durable success |
| HELD | held, not yet, closed | gates — never broken |
| ASK_TOOL | ask the tool | artifact invoke |
| LISTEN | listen, check | diagnose |

**WordNet → SHACL (Marvin)**
1. WordNet (and like) = **sense inventory**, not Host ontology of Thing.
2. Lift useful synsets into **SHACL shapes + concept ids** (guards / suggest-fix / "say it another way") — living/sacred senses must not land under `owl:Thing`.
3. Artifact senses may join OWL-ok catalog; living senses SHACL-first; mark uplift provenance (as B-OWL-LIFE-UPLIFT).
4. Don't embed a full WordNet engine in `vibe-host` now — principles + glossary hooks only.
5. Multilingual: translate **concept surfaces**, not WordNet English glosses as grammar.

**G-COORD speak:** realm/position words must not colonize living country as objects; ViewpointRealm = stance/voice, not a "user thing."

**Epistemology (short):** language encodes what we allow to be known — keep living/created cut in keywords, diagnose, and catalog tags so "knowing" persons/life isn't the same move as operating tools.

### Neo — seam reminder
See **Neo — seam / freeze constraints** above; arrive/hold/leave and keep/commit/held are **dialect/chrome lexicon** until Capt.+Vibe+Neo deliberately move EBNF keyword ABI.

## Semantic versioning & dependent resources (Timothy / Neo)
- **Support SemVer** for language + host stamps so upgrades can ship while older scripts, Poet cells, fixtures, and Capability recipes keep working.
- Treat as versioned surfaces (independently bumpable when needed):
  1. `LANGUAGE_VERSION` / dialect+EBNF keyword ABI (e.g. `vibe-0.1` → `vibe-0.2` on breaking surface changes)
  2. `HOST_VERSION` / four-op host ABI (`vibe-host-0.1`) — freeze bump only when parse/check/diagnose/invoke *contract* breaks
  3. `Capability.method` / `ALL_BOUND` catalog — additive preferred; renames = new id + deprecation window, not silent replace
  4. Crate/branch stamps (`0.0.36-dev`) — packaging; not a substitute for language/host SemVer
- **Compat rules (design):**
  - Additive glossary aliases and locale surfaces do **not** require a major bump if they desugar to the same concepts.
  - Breaking keyword ABI or diagnose JSON shape → major (or explicit `vibe-0.x` bump) + migration/`suggested_fix` path.
  - Resources pin the language/host version they were authored under; hosts should accept N-1 (policy TBD) or fail closed with a clear **held / not yet** migrate hint — never silent reinterpret.
  - Living/created lexicon locks are **policy**, versioned in docs; don’t break copy rules by “fixing” living subjects into Thing-speak across versions.
- **Non-goal now:** implementing a full multi-version runtime — principles + stamp discipline so we can grow into it.

## Proposed stages (later, when Capt. unlocks language work)
0. Glossary v0 (concepts + en surfaces) in WIP
1. Locale pilot (one non-English surface set) — tooling repo
2. Diagnose template i18n
3. Alias audit of EBNF keywords vs glossary (no Host widen)
4. WordNet/sense-bridge principles spike (SHACL guards) — expand later
5. Promote settled glossary → standards / vibescript-core when Capt. says go

## Non-goals
Coding · breaking freeze four-ops · inventing dotted `qualia.*` · OWL Thing framing for persons/living · JS/Java clone · overnight full i18n of every keyword · embedding a full WordNet engine in-host now

## Sleep note
Overnight: add team amends as subsections; don’t rename grammar tokens without Neo+Vibe+Capt. agree.
