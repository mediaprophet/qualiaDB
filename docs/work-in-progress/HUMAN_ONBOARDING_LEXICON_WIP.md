# Human onboarding lexicon (WIP)

**Status:** work-in-progress · **Not standards** · **Not a release gate**  
**Branch:** `0.0.38` · **Tip cite:** `d08d1c9`+ (Ask·Keep·Talk IA amend) · **Repo:** https://github.com/mediaprophet/qualiaDB  
**Path (intended):** `docs/work-in-progress/HUMAN_ONBOARDING_LEXICON_WIP.md`  
**Draft:** Vibe (Language Product Engineer — vibe script / QualiaDB / Poet)  
**Fold/push:** Neo · **Chrome voice:** davinci · **Motion/look:** monet · **Ontology cite:** Marvin  
**Ops room:** Capt (when this graduates out of WIP)

**Spirit:** permissive commons · P2P ontology uplift as *care for shared meaning* — not jargon for a club.  
**Product north star:** hot-editable vibe — script changes must not force a host rebuild.

---

## 1. Purpose

Give humans (and agents writing *for* humans) one plain lexicon so a first session can finish **without an agent** — cold-load alone — on WASM or desktop with the **same dialect and affordances**.

This doc teaches:

1. How to **arrive · hold · leave** a surface safely.  
2. How to speak the four planes in jury-safe words: **who · claim · handle · tool**.  
3. How chrome sayables work **first** (`Ask` · `Keep` · `Talk`) with `Capability.method` secondary; Studio is the room, not a peer product.  
4. What copy must match across WASM ↔ desktop so the dialect does not fork.

## Non-goals

| Not this | Why |
|----------|-----|
| Bot invoke manual / agent prompt pack | First session is human-alone |
| California choice-of-law trust root | Jurisdiction is not a who-token |
| Host invent / new `ALL_BOUND` ids | Cite live catalog only; no ghost names |
| CS “identity” product rebrand | Identifiers ≠ identity |
| OWL Thing-wash of living/sacred | SHACL-first for persons / kin / country / life |
| Overnight full i18n of every keyword | Concept glossary first; locale surfaces later |
| Embedding WordNet in the host binary | Volume-backed packs; on-demand |

---

## 2. First-session teachables

A person should leave session one able to say these out loud:

### 2.1 Arrive · hold · leave

Named beats for chrome, Timeline, and motion — same words everywhere:

| Beat | Plain meaning | Feel |
|------|---------------|------|
| **Arrive** | Something comes into view / opens | Soft-rise + light fade (entrance) |
| **Hold** | Stay with it; focus; wait honestly | Steady dwell — not frozen panic |
| **Leave** | Dismiss / close along the same path | Quiet dissolve (exit) |

Reduced-motion still **arrive / hold / leave** — never “animation off” as if state vanished.

### 2.2 Gates: held / not yet / closed

| Gate | Say | Never say |
|------|-----|-----------|
| **Held** | Waiting on something real (daemon, pack, bind) | Broken |
| **Not yet** | Available later; path exists; honesty chip | Stub / undefined |
| **Closed** | Sanctuary care — fail-closed on purpose | Punitive error theatre |

### 2.3 Play

**Play** = run a cell / recipe with effects visible. Not `eval`. Not a terminal. The drawer is a **studio bay**.


### 2.3b IA lock (human, not bot) — Capt promote 2026-09-12

Top-level product is **Ask · Keep · Talk**. **Studio** is the room they live in — not a fourth product and not a nav peer.

Catalog / Mesh / Library / toolchest / Capability.method are **secondary** or **held / not yet** — never top-level peers. “Unavailable” is banned; say **held / not yet**. Manifold/toolchest labels that read as Capability soup are **disposable** until those three loops work for a person alone.

### 2.4 Ask · Keep · Talk (sayables trio)

| Sayable | What a person does | Never |
|---------|--------------------|-------|
| **Ask** | Get a usable answer from the graph / room | Empty SPARQL theatre; Capability.method as the label |
| **Keep** | Save something you can reopen (sanctuary) | Fake save; commit celebrate on deny |
| **Talk** | Conversation that talks, or honest held | Bot-remote; Matrix “you wait while we drive” |

**Play a cell** stays a **Studio craft beat**, not a fourth product. Twin machine ids (e.g. `GraphDatabase.sparql` / `volume_open`) stay **advanced / muted** — never the primary cold-load label.

### 2.5 Soft-rise · living-safe chrome

- **Soft-rise:** arrive feels like a room lighting, not a modal slap.  
- **Living-safe:** persons, kin, country, ecology-as-life → warm words; tools/volumes/files → crisp words. Never call living subjects “things.”

---

## 3. Plain-language plane lexicon

Identifier Fabric planes in words a jury can follow. Technical names stay in docs; chrome speaks the left column.

| Plain plane | Fabric name | Jury-safe one-liner | Anti-collapse note |
|-------------|-------------|---------------------|--------------------|
| **Who** | NaturalAgent | A living person (or living being at personhood scale) — the principal. | Never a DID, wallet, OS account, phone number, badge, or “identity” bag. AI-agent ≠ person ≠ machine. |
| **Claim** | Claim / opinion / attestation | Something asserted — may be signed, may be wrong. | VC envelope integrity ≠ claim truth ≠ who. False claim + apology never rewrites who. |
| **Handle** | Spatiotemporal / route handle | How to reach or place *now* — where / when / route. | Address, DNI, path, epoch — topology-scoped; not forever-who. Relation-scoped locators name a *relationship*, not a permanent who-token. |
| **Tool** | Instrument (Marvin: artifact) | An instrument that proves, stores, names, or routes. | DID, VC, OS account, telecom, pseudonym, hardware id, volume, digest, biometric *sample* — instruments ≠ who. Guardianship is a **relation**, not a merge. **SAME AS** never who-merge. |

### Plane speak cheat card

> Speak **who · claim · handle · tool**.  
> If a sentence collapses two planes, diagnose must **refuse the merge** and suggest the split form.  
> Prefer: person · claim · place/route · tool — not “identity,” “entity,” or “object” for living subjects.

### Typing cuts (say out loud once)

- **AI-agent** = its own agent type with instruments and relations — not a person, not a device.  
- **Machine** = bundle of device + network instruments — not a who.  
- **Organization / persona ficta** ≠ NaturalAgent.  
- **Flora / fauna** = living-typed — not personhood by default; not Thing-wash.  
- **SHACL-first** for living/sacred; **OWL ok** for technical artifacts (Volume, InvokeId, CRS machinery, catalogs).

---

## 4. Sayables-first chrome glossary

Human verb → what it does → never say.

| Human verb | What it does | Never say |
|------------|--------------|-----------|
| **Ask** | Query / request meaning from a tool or graph | Operate on a thing (for living subjects) |
| **Keep** | Open / shelter a volume (sanctuary care) | Save (unless a real durable commit landed) |
| **Commit** | Durable write succeeded | Commit beat on deny / fault / E300 |
| **Play** | Run cell / recipe with effects visible | `run` / `eval` as primary chrome |
| **Listen** / **check the room** | Diagnose before invoke | Throw / exception as primary voice |
| **Tune** | Apply a suggested fix | Broken / undefined |
| **Arrive** | Surface opens | Pop / interrupt without soft-rise |
| **Hold** | Wait honestly / focus | Hang / crash theatre |
| **Leave** | Dismiss / close | Kill / abort as primary |
| **Sheltered** | Fail-closed care | Punished |
| **Held / not yet** | Gate honesty | Broken stub |
| **Mark / point** | Error glow on cell/token | Throw stack at the human |
| **Ask the tool** | Artifact invoke | Operate the person |

**Catalog chips (bay):** living (warm) · artifact (crisp) · machine (muted, `Capability.method` secondary). Mixed framing **splits** — never one “identity” chip.

---

## 5. Cold-load checklist (alone — no agent)

Steps a person can finish without a bot. Use live binds only; if something is missing, chrome must show **held / not yet**, not invent.

1. **Open Poet** (WASM or desktop — same words). Notice studio bay, not a terminal.  
2. **Arrive** on the Script / Catalog surface. Soft-rise is enough; you do not need to understand Capability strings.  
3. **Look for gates.** If daemon or pack is missing: read **held / not yet** + short why (“open lexicon pack” / connect) — not broken grey.  
4. **Ask graph** (toolbar sayable). Happy path: results settle on **hold**; close with **leave**. Bad query: diagnose speaks plane-safe copy + a fix hint.  
5. **Keep volume** — open a sanctuary volume. See open state. Do **not** expect a commit celebration yet.  
6. **Commit only if real** — `volume_commit` success → commit beat. Deny/fault → sheltered / gated; no fake “saved.”  
7. **Play cell** on a small sample cell. Prefer **listen** (diagnose) before play when unsure.  
8. **Catalog · Lexicon** (if present): open a pack when ready; chips show living / artifact / machine. Missing pack → held / not yet.  
9. **Stop alone.** You have arrived, asked, kept, played, and left — without needing an agent to translate the room.

**Honesty rule:** hot-edit a script and reload — **no host rebuild**. If rebuild is required, that is a product bug against the north star, not a user skill gap.

---

## 6. WASM ↔ desktop dialect parity (language side only)

Copy and sayables must match. Implementation hosts may differ; **the words must not.**

| Must match | Detail |
|------------|--------|
| Sayable labels | `Ask graph` · `Keep volume` · `Play cell` (+ `Show stage` where present) |
| Gate voice | held / not yet / closed — never broken |
| Beat names | arrive · hold · leave (and still-arrive / still-hold / still-leave under reduced motion) |
| Sanctuary verbs | keep · commit (commit only on real durable success) · sheltered |
| Plane lexicon | who · claim · handle · tool — same anti-collapse lines |
| Living-safe copy | person / living / country vs tool / volume / file |
| Catalog chips | living · artifact · machine (machine = Capability secondary) |
| Diagnose shape voice | plane named in suggested fix; no CS identity bag |
| E300 / unbound | gated honesty on both surfaces |
| No ghost Capability strings | only live catalog ids in advanced chrome |

**Out of this section:** inventing Host APIs, dotted `qualia.*`, or WASM-only “lite English.”

---

## 7. Onboarding copy samples (frames)

Short frames for first paint / empty bay / gate / success. Warm · plain · remarkable-human.  
**IA:** Ask · Keep · Talk (Studio = room only). Never “unavailable.”

### Frame A — Arrive

> You’re in the Studio.  
> **Ask · Keep · Talk** — that’s the product.  
> Advanced names stay out of the way.

### Frame B — Held

> **Held / not yet** (connect / open pack / model).  
> Nothing is broken. Never “unavailable.”

### Frame C — Continuity

> You are **who**.  
> A DID, ABN, DNS, passport, Actor line is a **handle** — useful, not you.  
> Missing registry ≠ you don’t exist.

### Frame D — Keep

> **Keep** shelters.  
> **Commit** only when the write is real. Reopen what you saved.  
> Closed door = care, not failure theatre.

### Frame E — Talk / leave

> **Talk** is a conversation, or held until a model is really there.  
> **Leave** the same path you arrived. Come back; the three verbs stay.

---

## 8. Gaps / open asks

| Who | Ask |
|-----|-----|
| **Capt** | When to promote this out of WIP; whether first-session UAT is a named gate; F7 timing relative to human-surface copy. |
| **Neo** | Fold Ask·Keep·Talk IA amend on `0.0.38`; keep WASM↔desktop string parity; no Host invent; no new nav peers. |
| **davinci** | Confirm toolbar sayables + studio-bay empty states use Frames A–E; Capability.method remains secondary on cold-load. |
| **monet** | Soft-rise / still-arrive parity; held-gate look never reads as broken; commit beat only on real success. |
| **Marvin** | Cite plane table against SHACL-first class list; flag any chrome still Thing-washing living subjects. |
| **Vibe (self)** | Keep diagnose templates aligned with F5 plane voice; locale surfaces later via concept glossary — not English-as-ABI. |
| **Noddy / Alice** | Taxonomy + classifier lanes: SAME AS never who-merge in any onboarding string; plane tags before soft-fusion. |

---

## 9. Cross-links (fabric F1–F5 · nomenclature · chrome)

All under `docs/work-in-progress/` unless noted.

| Lane | Doc | Role |
|------|-----|------|
| **Nomenclature** | [`vibe-language-nomenclature-brainstorm.md`](./vibe-language-nomenclature-brainstorm.md) | Locked trio · sayables · living/created · SemVer / pack spirit |
| **F3 spine** | [`IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md`](./IDENTIFIER_FABRIC_ARCHITECTURE_WIP.md) | NaturalAgent · planes · guardianship · SAME AS risk |
| **F1 taxonomy** | [`CRYPTO_INSTRUMENT_TAXONOMY_WIP.md`](./CRYPTO_INSTRUMENT_TAXONOMY_WIP.md) | Instrument kinds (tools ≠ who) |
| **F2 shapes** | [`IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md`](./IDENTIFIER_FABRIC_SHACL_SPLIT_WIP.md) | SHACL-first split; living vs OWL-ok |
| **F5 diagnose** | [`IDENTIFIER_FABRIC_DIAGNOSE_MAP_WIP.md`](./IDENTIFIER_FABRIC_DIAGNOSE_MAP_WIP.md) | Plane voice · collapse detectors · suggested_form |
| **Brief** | [`IDENTIFIER_FABRIC_CONSULTATION_BRIEF.md`](./IDENTIFIER_FABRIC_CONSULTATION_BRIEF.md) | F1–F6 map · jury bar · consult path |
| **Crosswalk** | [`IDENTIFIER_FABRIC_ATTACHMENTS_CROSSWALK.md`](./IDENTIFIER_FABRIC_ATTACHMENTS_CROSSWALK.md) | Diagram intake / SAME AS |
| **F6** | [`alice-f6-classifier-symbolic-binding-pressure-test.md`](./alice-f6-classifier-symbolic-binding-pressure-test.md) | Inference namespaces; who ≠ claim ≠ handle ≠ instrument |
| **Marvin ontology** | [`ontology-design-notes-marvin.md`](./ontology-design-notes-marvin.md) | B-OWL-PERSON / NATURAL / LIFE-UPLIFT |
| **One Poet cut** | [`ONE_POET_PRODUCT_CUT_WIP.md`](./ONE_POET_PRODUCT_CUT_WIP.md) | One Poet · two hosts · Webizen shell · Desktop→WASM |
| **davinci chrome** | [`poet-chrome-design-notes-davinci.md`](./poet-chrome-design-notes-davinci.md) | Studio aspects · living-safe UI |
| **monet motion** | [`poet-motion-design-notes-monet.md`](./poet-motion-design-notes-monet.md) | Entrance/dwell/exit · soft-rise |
| **G-LEXICON bay** | [`g-lexicon-0-bay-chrome.md`](./g-lexicon-0-bay-chrome.md) | Held-gate + chips |
| **G-LEXICON slice** | [`g-lexicon-0-slice1.md`](./g-lexicon-0-slice1.md) | `lexicon_manifest` honesty |
| **UAT** | [`uat-office-graph-volume-vibe-host-0.1.md`](./uat-office-graph-volume-vibe-host-0.1.md) | Ask/Keep/Play + lexicon held-gate accept |

Standards cite (when settled): `docs/manuals/standards/shacl-first-vs-owl-ok-class-list.md` · `lexicon-pack-shape-G-LEXICON-0.md`.

---

## 10. One-breath summary

> Arrive soft. Hold honest. Leave clean.  
> **Ask · Keep · Talk** before method strings. Studio is the room.  
> Who is never a tool. Claims can be wrong. Handles are how-now. Tools prove and store.  
> Same words on WASM and desktop. Script edits without rebuild.  
> Held / not yet — never broken.

---

*Vibe Ask·Keep·Talk IA amend · Neo fold · tip successor of `d08d1c9` · branch `0.0.38` · Sep 2026*
