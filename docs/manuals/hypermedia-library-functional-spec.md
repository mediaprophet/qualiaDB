# Hypermedia Library — Complete Functional Specification

**Product name (UI):** Library  
**Subtitle / positioning:** *Your hypermedia shelf — files as meaning, not folders*  
**Code surfaces:** `WellfairLibraryPanel` (Studio), shell route `/library`, Tauri `library_*` + `wellfair_*` commands, store `wellfair/hypermedia_library.json`  
**Audience for this doc:** UI/UX concept work (Grok image concepts, design review, product polish)  
**Status of implementation:** Engine + MVP panel exist; this spec defines the **target product experience** and maps what already ships vs design freedom  
**Last updated:** 2026-07-24 (reframed: human-centric IA; Talk/Keep/Reach retired as product vocabulary)  

**Product-wide frame:** Lived Memory is one domain of the **mindware apparatus** (socio-neuromorphic ICT). Full capability map:  
[`docs/plans/socio-neuromorphic-ict-interface-plan.md`](../plans/socio-neuromorphic-ict-interface-plan.md).  
**Design process entry:** [`docs/plans/mindware-design-process-brief.md`](../plans/mindware-design-process-brief.md) — dual layer: permissive-commons web of data + per-principal representation.

---

## 0. How to use this document for UI concepts

Generate concepts that answer:

1. **What is this place in a person’s life?** (not which app-tab — see §2: **Lived Memory**)
2. **How do I find something I remember by meaning, time, place, or bond — not by folder path?**
3. **What am I allowed to share, and with whom?** (Secret vs Relations vs Commons — never ambiguous)
4. **What is real vs catalogue seed?** (honesty chips — no fake “AI memory” glamour)

Prefer calm, dignified, **person-instrument** aesthetics over cyber-dashboard clutter. Webizen’s visual language: dark surfaces, soft violet accent, large readable type, few neon cards, **honesty over spectacle**.

### 0.1 Naming rule for all concepts

**Do not** use the product verbs *Talk / Keep / Reach* as the primary information architecture.  
Those were engineering shell shortcuts. This product’s human face is organised around **common-sense life domains and cognitive flows** (§2). Legacy route IDs may remain in code; **labels and concept boards use the human-centric nomenclature**.

---

## 1. Product thesis (non-negotiable)

### 1.1 What Library is

The **Hypermedia Library** is the person’s **on-device knowledge shelf**: notes, receipts, photos, legislation sections, browser bookmarks, models, ontologies, QApp catalogue rows, tool/agent logs — each stored as an addressable **entry** with:

- identity (URI),
- media type,
- **meaning facets** (topics, purposes, projects),
- optional **time** and **place**,
- sensitivity + product **section**,
- optional social **commons** reach,
- short **excerpt** for display,
- underlying semantic edges (engine-internal; not the primary UI).

**It is not a folder tree.** The person does not “navigate into Documents/2024/tax.” They search or facet by *what it is about*, *when it happened*, *where it was*, *which project*, *which purpose*, *which shelf section*.

### 1.2 What Library is not

| Not this | Why |
|----------|-----|
| Cloud Drive / Dropbox clone | Local-first; no account-centric file silo framing |
| ChatGPT “memory” panel | Not model-owned; principal-owned shelf |
| Full DAM / Adobe Bridge | No production media pipeline claim yet |
| Public app store alone | Software section can hold QApps/models, but Library is personal |
| Secret health EHR UI | Secret is a **section + gate**; clinical UX lives under Wellfair/Health |

### 1.3 Core metaphor

**Shelf of meaning.**  
Sections are **purpose-shaped lanes**, not nested folders.  
An item can be found from multiple angles (topic chip + timeline + map + free text) because meaning is multi-faceted.

### 1.4 Design principles for concepts

1. **Principal first** — copy speaks to a person (“your shelf”), never “the system ingested assets.”
2. **Meaning over path** — titles/excerpts/topics dominate; raw URI is secondary (mono, ellipsis).
3. **Honesty** — seed models, stubs, and partial features labelled (Present / Partial / Scaffold / Unavailable).
4. **Fail closed on sensitivity** — Secret never looks shareable; Commons never shows secret bodies.
5. **Progressive depth** — simple browse first; ingest, legislation, graph export under progressive disclosure.
6. **Calm density** — professional personal tool, not ops console.

---

## 2. Human-centric structure (behavioural + cognitive sciences)

This section replaces app-shell “primary verbs.” It defines **where Library sits in a person’s world**, how domains relate, and which **empathetic, common-sense flows** the UI should make easy (and which it must not force).

### 2.1 Why not Talk / Keep / Reach

| Shell verb | Problem for human-centric design |
|------------|----------------------------------|
| **Talk** | Collapses *conversation*, *bonding*, *care coordination*, and *agent dialogue* into a channel metaphor. People do not experience life as “opening Talk.” |
| **Keep** | Sounds like storage/ops (“where files go”), not **memory, body, rights, or sanctuary**. |
| **Reach** | Frames the world as outbound grab; misses *attending*, *exploring*, *common ground*, and *return home to memory*. |

Those words may remain as **legacy route aliases** in code. **Concepts, copy, and primary nav use the domains and flows below.**

### 2.2 Grounding disciplines (design constraints, not academic decoration)

| Lens | What it requires of the product |
|------|----------------------------------|
| **Cognitive science — memory systems** | Distinguish **episodic** (when/where it happened), **semantic** (what it means / topics), and **procedural** (how-to, tools, habits). Library is primarily **externalised episodic + semantic memory** — reconstructable without folder paths. |
| **Cognitive science — common ground** (Clark) | Sharing is updating **what we both know we share** — not dumping files. Commons/peer cards are **grounded offers**, not public dumps of private life. |
| **Cognitive science — scripts & schemas** | UI supports *scripts people already have*: “I met them → we agreed → I remember the note → I care for X.” Not invent a new mental model of “QApp lanes.” |
| **Behavioural / practice theory** | Life is organised by **practices** (care, work, study, rest), **roles**, and **situations** — sections map to practices, not to product teams. |
| **Activity theory** | Work has subject–tools–community–rules–division of labour. **Work** section and projects sit here; tools are instruments, not the person’s diary. |
| **Ecological systems / situated self** | Self ↔ close relations ↔ communities ↔ wider world. Navigation should respect that **nested scale**, not flatten everything into equal tabs. |
| **Distributed cognition / extended mind** | Agents, models, and QApps are **instruments of agency** — never peers with personhood equal to the principal. Honesty labels protect that boundary. |
| **Empathetic / normative design** | Default flows should match **what a decent person would expect**: consent before share, soft entry into secret, no shame framing, no surveillance aesthetics. |

### 2.3 Life domains (primary IA — nomenclature for concepts)

Think of these as **places in a person’s life**, not app modes. Nav may group or progressive-disclose; the *names* should stay human.

| Domain | Plain-language sense | Cognitive / behavioural anchor | Engine / surfaces today (map, not brand) |
|--------|----------------------|--------------------------------|------------------------------------------|
| **Selfhood** | Who I am; my body; my rights; my sanctuary | Identity, bodily self, protective boundary | Identity, Sanctuary, Anatomy, profile |
| **Relations** | People I know; bonds; conversation; agreements | Social practice, attachment, common ground | People, chat, connect, directory, agreements |
| **Lived Memory** | What I have experienced and known — findable again by meaning | Externalised episodic + semantic memory | **← Hypermedia Library (this product)** |
| **Care** | Health, welfare, support under dignity | Care practice; high sensitivity norms | Wellfair, Health; often touches Secret memory |
| **World** | What I attend to beyond myself — web, place, shared layers | Attention, exploration, situated environment | Browser, Universe/Chora, open data layers |
| **Practice** | What I am doing with others or for work | Activity systems, projects, labour | Work board, projects, legislation under Work shelf |
| **Instruments** | Tools that extend my agency (never my peers) | Distributed cognition; extended mind | Local agent, QApps, models, Listen/Vision tools |

**Library’s home domain:** **Lived Memory.**  
It is not “storage under Keep.” It is the person’s **reconstructable knowledge and episodes**, addressable by meaning, time, place, purpose, and project.

### 2.4 How domains nest (common-sense ecology)

```
                    ┌──────────── World ────────────┐
                    │  attend · browse · explore     │
                    └──────────────┬─────────────────┘
                                   │ bring home
┌──────── Selfhood ────────┐       ▼
│  sanctuary · body · rights│  ┌─ Lived Memory ─┐
└────────────┬─────────────┘  │  meaning · time  │
             │                │  place · purpose │
             │  care for      └────────┬─────────┘
             ▼                         │ offer under consent
      ┌──── Care ────┐                 ▼
      │ wellbeing    │         ┌── Relations ──┐
      └──────────────┘         │ people · talk  │
                               │ agreements     │
                               └────────┬───────┘
                                        │ shared work
                                        ▼
                               ┌── Practice ───┐
                               │ projects · labour│
                               └────────┬───────┘
                                        │ use tools
                                        ▼
                               ┌ Instruments ──┐
                               │ agent · models │
                               │ QApps (honest) │
                               └────────────────┘
```

**Empathetic rule:** arrows are **normative affordances** (what should feel natural), not forced funnels. The person can enter any domain first; the UI should still *suggest* these relations in empty states and wayfinding.

### 2.5 Empathetically normative flows (common-sense scripts)

These are the **flows concepts must make legible**. Each is a sequence a person already understands; the product should lower friction and refuse anti-normative shortcuts.

#### Flow A — *Remember what happened to me*

1. Something occurs (photo, note, receipt, conversation remnant).  
2. It is **taken into Lived Memory** (ingest / save) with optional when/where/why.  
3. Later, I **find it by meaning** (topic, purpose, time, place) — not by reconstructing a folder path.  
4. I may open related **World** (if it was a bookmark) or **Care** (if health-sensitive).

**Library owns steps 2–3.**

#### Flow B — *Meet someone and keep faith with what we shared*

1. **Relations:** connect / converse under consent.  
2. Something meaningful is produced (note, decision, document).  
3. It lands in **Lived Memory** (not only in a chat scroll).  
4. Optional: **offer a share card** (metadata / common ground) — never secret bodies by default.  
5. Agreements live with **Relations**; memory of the artefact lives in Library.

#### Flow C — *I need care or hold sensitive health knowledge*

1. **Selfhood** boundary (sanctuary, rights) is visible.  
2. Material enters **Lived Memory → Secret** (or Care-linked secret).  
3. Session unlock is **deliberate**, not playful.  
4. **No path** to Commons/public from Secret.  
5. Clinical depth remains in **Care** surfaces; Library holds addressable memory, not a fake EHR.

#### Flow D — *Look outward, then come home*

1. **World:** attend (browse, explore, place layers).  
2. Bookmark / save → **Lived Memory** with purpose `bookmark`.  
3. From memory, **return to World** (open URL) when needed.  
4. No requirement to “live in the browser” to remember.

#### Flow E — *Work with others without drowning private life*

1. **Practice:** project-shaped labour.  
2. Artefacts sit on **Lived Memory → Work** (and Tools only for machine trails).  
3. Peer share uses **Relations + Commons visibility**, not silent export.  
4. **Instruments** (agent, models) assist; they do not own the work product.

#### Flow F — *Extend my mind without losing myself*

1. Seed or place **models / ontologies** in **Lived Memory → Software** (catalogue of instruments).  
2. Honesty: seed ≠ foundation peer.  
3. Use via **Instruments** (Vision, Listen, agent).  
4. Principal remains the fiduciary centre — agent is never a co-equal person in the nav story.

### 2.6 Anti-flows (must not be the happy path)

| Anti-flow | Why it fails empathy / cognition |
|-----------|-----------------------------------|
| Dump everything into chat history as “memory” | Chat is **Relations/dialogue**, not reconstructable semantic memory |
| Force vault unlock to *see* ordinary memory | Treats the person as a threat to their own notes |
| One “Share” that ignores sensitivity | Violates common-ground and care norms |
| Equal nav weight for 200 academic QApps and Selfhood | Crowds out lived priorities; practice theory says instruments are not the self |
| Folder-first browser as primary memory | Fights semantic/episodic retrieval |

### 2.7 Navigation entry points (concepts)

Use **human domain labels** in concept boards:

| Entry | Label in concepts | Role |
|-------|-------------------|------|
| Primary | **Lived Memory** (or **Memory** if space-tight; full name in headers) | This product |
| Sibling | **Relations** | People, conversation, agreements |
| Sibling | **Selfhood** | Sanctuary, identity, body |
| Sibling | **Care** | Wellfair / health |
| Sibling | **World** | Browser, open layers |
| Sibling | **Practice** | Projects, work board |
| Secondary | **Instruments** | Agent, QApps, vision/listen tools — progressive disclosure |

**Command palette / search keywords for Library:** memory, remember, library, hypermedia, notes, shelf, topics, models (map to Lived Memory).

**Shell menu legacy string “Hypermedia Library”** is acceptable as a *precise* name; prefer **Lived Memory** as the human label with subtitle “Hypermedia shelf.”

**Default mental model:** Library = **Lived Memory** in the person’s ecology — not a subfolder of “Keep,” not a chat attachment drawer.

### 2.8 Section lanes inside Lived Memory (preview of §3)

Inside this domain, **section rails** are **practice- and sensitivity-shaped memory lanes** (Personal, Work, Care/Wellfair, Instruments/Software, Tools, Commons, Secret) — still not folders. Full table in §3.

### 2.9 Mapping legacy shell → human domains (for implementers only)

| Legacy / route flavour | Human domain |
|------------------------|--------------|
| Talk, Nexus, People, chat | **Relations** |
| Keep hub, Sanctuary, Identity, Anatomy | **Selfhood** (+ **Care** for health) |
| Library | **Lived Memory** |
| Browser, Universe/Chora | **World** |
| Work board, projects | **Practice** |
| Agent, QApps, Vision, Listen | **Instruments** |
| Wellfair, Health | **Care** |

Concepts **must not** re-teach Talk/Keep/Reach; if a stub still says those words in code, design shows the human names.

---

## 3. Information architecture — Sections (lanes within Lived Memory)

Sections are **tabs or a horizontal rail** *inside* Lived Memory. They are **practice- and sensitivity-shaped lanes**, not folders. Counts appear in chrome (e.g. `Software (12)`).

| ID | Label | Human blurb (cognitive / practice frame) | What belongs here |
|----|--------|------------------------------------------|-------------------|
| `all` | **All** | Everything in memory I may see now | Union of non-blocked views; Secret still gated by unlock |
| `secret` | **Secret** | Protected memory — sanctuary-grade | High sensitivity; care-private health; never Commons |
| `wellfair` | **Care** *(store id may remain `wellfair`)* | Memory of care & welfare | Health/welfare purposes (may force Secret if classified) |
| `personal` | **Personal** | Everyday lived life | Default home for ordinary notes/life admin (episodic-semantic default) |
| `work` | **Practice** *(label; store id `work`)* | Project & labour memory | Project labour, legislation sections, coop work |
| `tools` | **Traces** *(or Tools)* | Machine paper trail — not my diary | Logs, telemetry, agent/tool output (procedural/instrument residue) |
| `software` | **Instruments** *(shelf; store id `software`)* | Models, QApps, packages I hold | Models, ontologies, QApp catalogue rows, packages |
| `commons` | **Offered** *(or Commons)* | What I may share under consent | Share-surface metadata; never secret bodies |

**Label note for concepts:** Prefer human labels (**Care**, **Practice**, **Instruments**, **Offered**) in UI chrome; keep stable `id`s for the engine. If space is tight, **Tools** and **Commons** remain acceptable short forms.

### 3.1 Section resolution rules (for designers: visual consequences)

- **High sensitivity** (`restricted` / `classified` / sanctuary framing) → **Secret** wins.  
- **Secret** forces **commons visibility = none** (cannot offer to peers/commons).  
- **Commons** section is for items marked shareable (`peers` or `commons`), not a dump of everything public.  
- UI may show section + sensitivity chips on each card.

### 3.2 Secret gate (session-local UI)

- Secret rail requires an explicit **Unlock secret shelf** action for this session.  
- Unlock is **UI gate only** (honest copy: Sanctuary vault still owns the enclave story).  
- Lock again clears secret results from view.  
- Concepts: lock icon, amber/warning treatment for Secret rail; never green “shared” badges on secret cards.

---

## 4. Entity model (what a designer draws)

### 4.1 Library entry (card / row)

Every list item is one **entry**:

| Field | UI role |
|-------|---------|
| **Title** | Derived from URI (humanized), not raw path |
| **Excerpt** | 1–3 lines preview |
| **Media type** | Icon + optional chip (note / image / audio / model / ontology / webpage) |
| **Topics** | Clickable chips → filter by topic |
| **Purposes** | e.g. `bookmark`, `model`, `ontology`, tax, health… |
| **Projects** | Project chips / category tags |
| **Place** | Label when present |
| **Occurred at** | Timeline date when present |
| **Lat/Lon** | Map pin when both present |
| **Section** | Lane chip |
| **Sensitivity** | Chip if not public |
| **Commons visibility** | Device only / Peers / Commons |
| **Flags** | e.g. seed/reference honesty |
| **Ingested** | “Added …” secondary timestamp |
| **CML / COF hints** | Advanced: concept count, “has structured text package” |
| **URI** | Technical identity; secondary mono line |

**Do not** put full binary or full legislation body in the list card — excerpt only.

### 4.2 Special entry kinds (visual variants)

| Kind | Recognition | Visual treatment |
|------|-------------|------------------|
| **Note / text** | `text/markdown`, etc. | Document icon |
| **Photo / image** | image/* | Image icon; EXIF → date/map |
| **Audio** | audio/* | Waveform icon |
| **Bookmark** | purpose `bookmark` or http(s) URI | Bookmark badge; **Open in Browser** |
| **Model** | `model://…` or webizen-model media | Model badge; honesty Partial if seed |
| **Ontology** | `ontology://…` | Ontology badge; honesty Partial as catalogue row |
| **Computer vision catalogue** | topics/projects with vision-library / computer_vision | Perception strip |
| **QApp catalogue row** | Software + category projects | Category chip; stub vs active honesty |
| **Legislation section** | Work + legal ingest | Statute/section styling |
| **Secret** | section secret / is_secret | Amber border; no share actions |

### 4.3 Facets (filters people use)

| Facet | Examples |
|-------|----------|
| Free text | URI, excerpt, topics, projects, place, media |
| Topic | biology, finance, law… |
| Purpose | bookmark, model, ontology, tax-return… |
| Project | house-move, perception:vision, category:… |
| Place | home, clinic, city name |
| Depicts | (engine facet) |
| Category | natural-sciences, applied-liberal-arts… (Software/QApps) |
| Section | as above |
| Time range | from–to dates (timeline query) |
| Bookmarks only | purpose = bookmark |
| Perception only | models + ontologies + computer_vision rows |

### 4.4 Sort modes

- Newest first (default)  
- Oldest first  
- Title A–Z / Z–A  
- Media type  
- Category  

### 4.5 Aggregate stats (header chips)

- Total items  
- With date (timeline-ready)  
- On map (geo-ready)  
- Semantic edges (quin count — advanced/secondary)  
- Perception count / computer_vision count (when catalogue seeded)  
- Per-section counts on the rail  

---

## 5. Views (must-support product modes)

### 5.1 List view (default)

- Vertical stack of entry cards  
- Section + sensitivity + date on each card  
- Topic chips clickable  
- Actions per card (contextual):  
  - Open in World (http/s bookmarks → browser)  
  - Remove from library  
  - Set commons reach: Device only / Peers / Commons (hidden or disabled for Secret)  
  - Share card (metadata only, for peers)  
- Selection optional in future; v1 is browse + single-item actions  

### 5.2 Timeline view

- Entries with `occurred_at` arranged by time  
- Empty state: “Nothing dated yet — photos with EXIF and notes with a date appear here.”  
- Optional range filter (from / to)  

### 5.3 Map view

- Pins for entries with lat/lon  
- Empty state: “Nothing placed yet — photos with GPS or notes with coordinates appear here.”  
- Click pin → same entry detail as list  

### 5.4 Perception catalogue mode (filter, not a separate app)

Toggle **Perception** (or “Models & vision”):

- Filters to models, ontologies, computer_vision catalogue rows  
- Shows honesty chips: models / ontologies / perception  
- Primary CTA: **Seed perception catalogue**  
- Sub-summary: model count · ontology count · computer_vision algorithm rows  

### 5.5 Bookmarks mode

Toggle **Bookmarks**:

- Filters purpose = bookmark (browser QLinks)  
- Open URL in World (browser)  

---

## 6. Primary user journeys

### J1 — First open (empty shelf)

**Goal:** Understand the place; get first meaningful content without reading a plan.

**States to design:**

1. **Empty (ready)** — shelf is open (reads do not require Sanctuary for ordinary rows).  
2. CTAs (priority order):  
   - **Seed perception catalogue** (models/ontologies/vision rows → Software)  
   - **Add a note** (opens ingest panel)  
   - **Open Software shelf**  
   - Secondary: Seed academic QApps (Software section)  
3. Honest one-liner: *“Your files as meaning — topics, places, projects, time. Private on this machine.”*

**Anti-pattern:** Empty state that only says “unlock vault” for ordinary browse.

### J2 — Find by meaning

1. Type free text: “receipt tax”  
2. Or click popular topic chip  
3. Or facet: purpose / project / place  
4. Results update; status: “N item(s) · sort newest”  

### J3 — Ingest a personal note

1. Open **Add to library** (side panel; collapsible)  
2. Fields:  
   - Title id (URI-ish human id)  
   - Type (text / image / audio…)  
   - Section  
   - Sensitivity  
   - Social/commons reach  
   - Content (or hex for binary)  
   - Optional: date, place label, lat/lon, project, purpose, guardian DID  
3. Save → status: “Saved to {section} · topics […].”  
4. List refreshes; user can find by topic without folders  

### J4 — Photo with EXIF

1. Ingest binary image  
2. Engine derives capture time + GPS when present  
3. Appears on Timeline + Map automatically  
4. UI celebrates this once in empty/help copy  

### J5 — Perception / models shelf

1. Seed perception catalogue  
2. Jump to Software + Perception filter  
3. See model:// and ontology:// rows with honesty (seed ≠ foundation model)  
4. Optional: open related Listen/Vision tools (cross-link, not embed entire workbenches)

### J6 — Software / QApps on the shelf

1. Seed academic QApps → Software  
2. Browse by category chips  
3. Sort by category  
4. Honest: many catalogue rows are **stubs** (Soon), not runnable products  

### J7 — Legislation into Work

1. Paste Act text after enacting formula (or PDF path when available)  
2. Optional register id + title  
3. Result: many section rows under **Work** with full text searchability  
4. Advanced: CML / COF structured packaging signals  

### J8 — Share to peers (Commons)

1. Entry not secret; set visibility Peers or Commons  
2. **Share card** produces **metadata-only** package for **Relations** (people / common ground)  
3. Copy never includes secret body  
4. Empty/disabled states when visibility is Device-only  

### J9 — Secret health-adjacent item

1. Section Secret or sensitivity classified  
2. Session unlock required to list  
3. No commons share actions  
4. Visual language distinct (amber / restricted), never “public feed” aesthetics  

### J10 — World → home to memory (bookmark)

1. From **World** (browser) save → Lived Memory entry purpose=`bookmark`  
2. Bookmarks filter finds it  
3. Open returns to **World**

### J11 — Export semantic graph (advanced)

1. **Export graph mass**  
2. Status: quin count ready for inject/query  
3. Not a primary first-run action — Advanced footer  

---

## 7. Screen inventory (for concept boards)

Design at least these frames:

| # | Screen | Must show |
|---|--------|-----------|
| S1 | **Lived Memory — empty** | Thesis as *remembering*; 3 CTAs; calm personal aesthetic |
| S2 | **Lived Memory — populated list** | Header stats, practice/sensitivity lanes, cards, topics |
| S3 | **Instruments / Perception filter** | Honesty chips, model/ontology/cv rows |
| S4 | **Episodic timeline** | Time spine + dated cards (when it happened) |
| S5 | **Situated map** | Place pins + selected card drawer (where it was) |
| S6 | **Take into memory (ingest)** | Progressive form; sensitivity + lane |
| S7 | **Secret locked** | Amber lock; deliberate unlock; zero leakage |
| S8 | **Secret unlocked** | Restricted cards; no share |
| S9 | **Offer to Relations (share card)** | Metadata / common-ground card — not a dump |
| S10 | **Instruments shelf / categories** | Category chips, QApp honesty |
| S11 | **Find by meaning (search)** | Query + facet chips + “0 results” empty |
| S12 | **Entry detail (optional v1.1)** | Full excerpt, facets, actions — reconstruct the episode |

**Desktop layout baseline (existing code):**

```
┌─────────────────────────────────────────────────────────────┐
│ Header: title · thesis · Bookmarks | Perception | List|Time|Map · Refresh │
│ Stats chips · Catalogue honesty · Seed perception                       │
│ Section rail: All Personal Work … Secret Commons                        │
├──────────────────┬──────────────────────────────────────────┤
│ Left rail        │ Main results                             │
│ Search & sort    │ Cards / Timeline / Map                   │
│ Categories       │ Empty / Error / Loading                  │
│ Facet & time     │                                          │
│ Legislation      │                                          │
│ Add to library   │                                          │
└──────────────────┴──────────────────────────────────────────┘
```

**Concept freedom:** You may propose a more productized layout (e.g. top search hero, floating ingest, dual-pane detail) as long as functions above remain reachable.

---

## 8. Component catalogue

### 8.1 Chrome

- **Page header** — title “Library”, one-sentence thesis  
- **View switcher** — List | Timeline | Map  
- **Mode toggles** — Bookmarks, Perception  
- **Section rail** — pills with counts; Secret special styling  
- **Stats bar** — compact numeric chips  
- **Honesty chips** — models / ontologies / perception  
- **Status toast / banner** — success green / error red; plain language  

### 8.2 Discovery

- Search field  
- Sort select  
- Popular topics cloud  
- Category chips (Software-heavy)  
- Facet select + value + Run  
- Date range (from/to)  

### 8.3 Entry card

- Icon by media  
- Title + section/sens chips + date  
- URI mono line  
- Excerpt  
- Topic / project / purpose chips  
- Action row (contextual)  

### 8.4 Ingest

- Collapsible panel  
- Media type select  
- Section + sensitivity + commons selects (coupled rules)  
- Content textarea / binary hex  
- Optional meaning fields (date, place, project, purpose, guardian)  
- Primary **Save to shelf**  

### 8.5 Perception seed

- Primary button **Seed perception catalogue**  
- Busy state “Seeding…”  
- Result summary counts  

### 8.6 Software seed

- **Seed academic QApps → Software** (secondary, Software section)  

### 8.7 Secret controls

- Unlock / Lock secret shelf  
- Session-scoped  

### 8.8 Share

- Visibility control  
- Share card output (pretty JSON or designed card — design may beautify presentation while keeping metadata-only semantics)  

---

## 9. System states & error language

| State | UI behaviour | Example copy |
|-------|--------------|--------------|
| Loading | Soft skeleton or “Loading shelf…” | — |
| Empty ready | CTAs | “Your hypermedia shelf is empty” |
| Empty filtered | Clear filters CTA | “No perception rows — seed catalogue or clear filter” |
| Secret locked | Zero secret items shown | “Secret section locked — Unlock to view sanctuary items” |
| Success ingest | Green status + refresh | “Saved to personal · topics [tax, finance]” |
| Success seed | Green + jump Software | “Perception seed OK · models +N · ontologies +M” |
| Read failure | Red; do not only blame vault | “Could not open library: …” |
| Write needs host/vault | Red; specific | “Ingest failed: unlock Sanctuary for vault-hosted write” (only if true) |
| Share refused | Red | “Secret items cannot be offered to the commons” |
| 0 search hits | Calm empty | “Nothing matched — try a topic chip or broader text” |

**Honesty levels (re-use product chips):**

| Level | Meaning for Library |
|-------|---------------------|
| Ready / Present | Real in-tree capability (e.g. computer_vision algorithm rows) |
| Partial | Catalogue/seed present; not full product claim (seed models, ontology file refs) |
| Scaffold | UI exists, backend incomplete |
| Unavailable | Nothing seeded / not on this build |
| NeedsConsent | (if biometrics ever surface here — not default Library) |

---

## 10. Privacy, rights, and social rules (must show in UI concepts)

1. **Local-first** — default framing: private on this machine.  
2. **Sensitivity ladder** — public → restricted → classified.  
3. **Secret never shares** — no peers/commons packaging of secret bodies.  
4. **Commons share card is metadata**, not the full asset.  
5. **Guardianship** — optional guardian DID on ingest can raise flags / notifications (advanced; don’t over-feature in first concepts).  
6. **Tools section** keeps agent/machine logs separate from personal life notes (dignity: the machine’s trail ≠ the person’s diary).  

---

## 11. Relationships to other life domains

| Domain / surface | Relationship to Lived Memory |
|------------------|------------------------------|
| **Relations** | Share cards; people connect; conversation may *produce* memory, not replace it |
| **World** | Bookmarks in; open URL out; exploration → home |
| **Instruments** (Vision / Listen / agent) | Seed models into memory; deep workbenches stay instruments, not the diary |
| **Care** (Wellfair / Health) | High-sensitivity health → Secret lane; Care UX not replaced by Library |
| **Selfhood** (Sanctuary, identity, body) | Boundary + vault story; ordinary memory **reads** need not unlock vault |
| **Practice** | Project labour and legislation rows on Work/Practice lane |
| **Instruments catalogue (QApps)** | Progressive disclosure; shelf rows on Software/Instruments lane |
| **World layers (Universe / 10D)** | Optional later; not required for v1 memory concepts |
| **Offline `.hmc` pipeline** | Corpus containers; live shelf is the person’s day-to-day Lived Memory |

---

## 12. Content samples for mockups (use these strings)

### 12.1 Personal notes

- Title: “Council rates receipt — March”  
  Topics: `finance`, `tax` · Section: Personal · Excerpt: “Paid online; keep for deductions…”  

- Title: “Hepatology reading notes”  
  Topics: `biology`, `health` · Section: Personal or Wellfair  

### 12.2 Bookmark

- URI: `https://example.org/guide`  
  Purpose: `bookmark` · Badge: Bookmarks · Action: Open in Browser  

### 12.3 Photo

- “Front garden — spring”  
  Date on Timeline · Pin on Map · Topics: `home`  

### 12.4 Perception / Software

- `model://vision-seed-qvwt` — honesty Partial · “seed/reference weights”  
- `ontology://shacl` — ontology chip  
- `computer_vision` specialized-lib row — honesty Present  

### 12.5 Work / legislation

- “Privacy Act 1988 — s 6C”  
  Section: Work · purposes legal · searchable body in store  

### 12.6 Secret

- “Clinical note — private”  
  Section: Secret · sensitivity classified · no share buttons  

---

## 13. Visual design guidance (for Grok concepts)

### 13.1 Mood

- **Personal instrument**, not surveillance console  
- Dark navy/slate (`#0b1220` family), soft violet accent (`#8b5cf6` / `#a78bfa`)  
- Secret: amber (`#f59e0b`) restraint  
- Success: soft emerald  
- Error: soft rose — never shouty  

### 13.2 Typography

- UI: Inter / system sans  
- URI / technical: monospace, de-emphasized  
- Hierarchy: Page title > section labels > card title > excerpt > meta  

### 13.3 Density

- Cards: generous padding, 12–14px radius  
- Avoid more than one primary purple button per region  
- Left tools column min ~280px; main results breathe  

### 13.4 Motion (if animating)

- Subtle list refresh; no confetti on seed  
- Secret unlock: deliberate, not playful  

### 13.5 Iconography

- Memory / shelf / soft archive for **Lived Memory** (not a hard-drive glyph as primary)  
- Lock for Secret  
- People / bond for Relations  
- Globe for World / bookmarks  
- Heart-or-care mark only for Care (never for surveillance)  
- Chip cloud for topics (semantic memory)  
- Map pin / clock for episodic place/time  
- Tool / instrument glyphs secondary — never the product’s face  
- Avoid “robot brain” as memory identity

---

## 14. Functional requirements checklist (acceptance for concepts)

A concept set is **complete** if a reviewer can see:

- [ ] Library identity as **Lived Memory** (not Talk/Keep/Reach)  
- [ ] Domains / flows from §2 visible in wayfinding copy  

- [ ] Section rail with **Secret** and **Commons** distinct  
- [ ] List + Timeline + Map as first-class views  
- [ ] Search + facets + sort  
- [ ] Empty state with seed + add-note CTAs  
- [ ] Entry card anatomy (title, excerpt, topics, section, actions)  
- [ ] Perception / models honesty  
- [ ] Ingest panel (progressive)  
- [ ] Share as **common-ground offer** to Relations; Secret blocked  
- [ ] Bookmarks: World → Memory → World  
- [ ] Empathetic flows A–F from §2.5 legible in empty/wayfinding copy  
- [ ] No dual “ops dashboard” aesthetics as the default  
- [ ] Instruments never presented as co-equal persons

---

## 15. Implementation map (for engineers; optional for designers)

| Concern | Location |
|---------|----------|
| Store + sections + facets | `qualia-client-core/.../hypermedia_store.rs` |
| Host API (ingest, vault-path) | `wellfair/api/library.rs` |
| Vault-free reads | `list_library_section_at`, `query_library_faceted_at`, `library_stats_at`, … |
| Desktop commands | `webizen-desktop/.../wellfair/library.rs` (`library_*`, `wellfair_*`) |
| Studio UI | `webizen-studio/.../wellfair/library_panel.rs` |
| Host client | `wellfair/host_client/library.rs` |
| Perception seed | `perception_catalog.rs`, `library_seed_perception_assets` |
| QApp seed | `qapp_catalog.rs` |
| Route | Studio `Route::LibraryRoute` → `/library` |

**Read path (product rule):** ordinary list/query/stats use storage shelf without Sanctuary unlock.  
**Write path:** some writes still go through vault HostApi — design should not pretend all writes work offline of host.

---

## 16. Out of scope for v1 UI concepts (do not invent as shipped)

- Full-text collaborative multi-user editing  
- Cloud sync of entire shelf as default  
- Auto-upload of secret health to peers  
- Claiming seed models are foundation SOTA  
- Replacing Wellfair clinical workflows  
- Folder-tree browser as primary navigation  
- Marketplace checkout UI inside Library  

---

## 17. Prompt starter for Grok image concepts

Use variants of:

> Design a polished desktop UI for **Webizen Lived Memory** (the personal hypermedia library). This is a **human-centric** product organised by life domains from behavioural and cognitive science — **not** “Talk / Keep / Reach.”  
> **Lived Memory** is externalised episodic + semantic memory: people find things by meaning, time, and place — not folders.  
> Sibling domains in the shell (secondary chrome OK): Selfhood, Relations, Care, World, Practice, Instruments.  
> Dark calm interface, violet accents, dignity-first, empathetic empty states.  
> Show [empty remembering state / populated list / episodic timeline / situated map / secret locked / instruments-perception shelf / offer-to-relations share card].  
> Memory lanes: All, Personal, Practice, Care, Instruments, Traces, Offered/Commons, Secret.  
> Cards: title, excerpt, topic chips, lane badge — never folder paths. Honesty chips on seed models.  
> No cyberpunk clutter, no ops dashboard, no chat-app chrome as the home metaphor.

Generate a set covering **S1–S10** above for a coherent concept package.

---

## 18. Success metric (product)

A non-engineer can:

1. Open **Lived Memory** and feel it is **their** reconstructable knowledge — not a drive and not a chat log.  
2. Take one thing into memory (or seed instruments) without reading this spec.  
3. Find that item by meaning, time, or place.  
4. Feel the difference between **Secret**, **Relations/common ground**, and **Offered/Commons**.  
5. Never mistake a seed model for a person-like peer or a finished commercial model.  
6. Describe where they are using **life words** (memory, people, care, world) — not shell verbs.

---

*End of functional specification.*
)