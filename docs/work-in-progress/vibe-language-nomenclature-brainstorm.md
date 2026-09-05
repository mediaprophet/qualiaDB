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

## Team slots (fill / amend)
- **davinci** — studio voice; toolbar sayables; REPL drawer lexicon
- **monet** — fun vs clear; motion beat names ↔ language; reduced-motion copy
- **Marvin** — living/created lexicon; banned “thing” list; concept ids for person/country/ecology
- **Neo** — freeze seams: which tokens are ABI-stable under `vibe-host-0.1` vs free to rename in dialect only *(section above)*

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
