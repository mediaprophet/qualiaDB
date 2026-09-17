# Human-surface competitive usability (WIP)

**Status:** work-in-progress · Capt ops · **Not a release gate**  
**Branch:** `0.0.38` · **Tip lineage:** `1a77869` (Catalog findable B1–B3 · B4 Wave-22 held) · apps audit fold prior `1e2605c` · walkthrough `d7f0bdc` · Frame B Desktop `9738911` / chrome `592c99a`  
**Repo:** `mediaprophet/qualiaDB` · **Workspace:** `/workspace/qualiaDB`  
**Date:** 2026-09-12 (AEST) · **Runner:** Capt (competitive skim)  
**Product locks:** human-alone (no agent required) · Ask · Keep · Talk = teachable loops **not** locked top nav · ban **"unavailable"** → use **held / not yet** · Directory humans-first · chatbot is tool · Continuity handle ≠ human · One Poet · two hosts · cherry-pick not parity.

**Ground truth:** [`HUMAN_SURFACE_APPS_AUDIT_WIP.md`](./HUMAN_SURFACE_APPS_AUDIT_WIP.md) · [`HUMAN_SURFACE_DESKTOP_APPS_WALKTHROUGH_d7f0bdc.md`](./HUMAN_SURFACE_DESKTOP_APPS_WALKTHROUGH_d7f0bdc.md) · Desktop menu `crates/webizen-desktop/src/shell/menu.rs` · studio `Route` in `webizen-studio/src/main.rs`.

**Grading:** **PEER-OK** | **BEHIND** | **SHIT** | **MISSING** | **N/A-unique**

---

## Desktop / Poet surface inventory (skim)

| Surface | Where humans find it | Notes |
|---------|----------------------|-------|
| Talk (home) | QApps → Talk `Ctrl+0` · `/` | Relations habitat: Chat · People · Reception · Mail · Projects |
| WellFair | QApps `Ctrl+1` · `/wellfair` | Care shell |
| Chora | QApps `Ctrl+2` · `/chora` | Spatio-temporal commons |
| Web Browser | QApps `Ctrl+3` · `/browser` | Native / in-shell |
| 10D Browser | QApps `Ctrl+4` · `/10d-browser` | Anatomy / vision `.10d` |
| QApp Studio / Manage | QApps submenu | Secondary |
| Settings | Tools → Settings `Ctrl+,` · `/settings` (+ external Settings Portal) | Capt: listed, surface often miss |
| Hypermedia Library | Tools → Hypermedia Library · `/library` | Capt bounce → home |
| Wallet / Identity | Tools → Wallet · `/identity` | Identifiers ≠ who |
| Poet Harness | Tools → Poet · `/poet` · Catalog `/poet/catalog` | One Poet dialect |
| Keep / Sanctuary | `/keep` · `/sanctuary` | Vault / volumes |
| Dual Studio | Media manifold (Poet) | VibeScript + GPU |
| Nexus / Jobs / Anatomy / Health / Tools | routes present | Secondary / craft |
| **Mail** | Talk → Mail tab (+ Domains & Mail pane) | Purpose inboxes · local SMTP · optional IMAP/SMTP — **not** a top-level QApp |
| **Word processor** | — | **No** Docs/Word/Pages peer in menu or routes |
| **Calendar app** | Projects manifold seed `calendar` honesty=`present` | No human Calendar QApp / peer surface |

---

## Competitive matrix

### Word processing — peers: Google Docs · Microsoft Word · Apple Pages

| Webizen surface | Grade | Why |
|-----------------|-------|-----|
| *(none — not Keep, not Library, not Dual Studio canvas)* | **MISSING** | No real Desktop/Poet writer. Keep = volumes/vault; Library = hypermedia shelf; Dual Studio = craft viewport; Research Document/LaTeX/Slides Poet seeds are **held**. Honest gap — do not claim document editing parity. |

- **vs-peer gap**
  - No blank-page compose / share / co-edit loop a person recognizes from Docs/Word/Pages.
  - No export/print/styles affordance as a first-class app.
- **Fix priority:** P3 (product decision first — cherry-pick later, not fake a Word clone)
- **Owner lane:** Capt UAT (scope) · Vibe sayables (if ever named) · davinci chrome only after Timothy locks “writer under Keep?”

---

### Mail — peers: Apple Mail · Gmail · Proton Mail

| Webizen surface | Grade | Why |
|-----------------|-------|-----|
| Talk → Mail / Domains & Mail (`mail.rs` cmds · `domains_pane` · social_hub Mail tab) | **BEHIND** | Real apparatus path exists (purpose inboxes, relationship addresses, local SMTP receiver, optional transport) but usability is setup-heavy and not a daily inbox peer. Buried under Talk tabs; no QApps → Mail. |

- **vs-peer gap**
  - No one-click “check mail / reply / thread” polish vs Apple Mail / Gmail / Proton.
  - First-run is domain → onboard → receiver — power-user, not consumer mail.
  - Search, folders, compose UX lag peer mental models (product is semantic mail, but chrome still feels admin).
- **Fix priority:** P1 (discoverable daily path under Talk; honest held when receiver down)
- **Owner lane:** davinci chrome · Neo daemon-connect (receiver/bind) · Vibe sayables · Capt UAT

---

### Chat / Talk — peers: Messages · Signal · Slack · WhatsApp

| Webizen surface | Grade | Why |
|-----------------|-------|-----|
| Talk / ConnectChat / Relations shell | **BEHIND** | Capt walkthrough Talk **PASS** (mounted, home). Human-alone still weak: `NEEDS MODEL` / instrument none; chatbot must stay **tool** not peer; Mesh/presence often “unavailable” theatre. |

- **vs-peer gap**
  - Peers open and chat in seconds; Talk still asks model/instrument readiness without soft held path.
  - Continuity / handle≠who copy PARTIAL — risk of treating handles like persons.
  - Slack-like channels / Signal E2E mental model not the product goal — but basic “message a person” must not need an agent.
- **Fix priority:** P0 (Talk that talks without agent)
- **Owner lane:** Neo daemon-connect / model path · davinci chrome · Vibe sayables · Capt UAT

---

### Browser — peers: Chrome · Safari · Firefox

| Webizen surface | Grade | Why |
|-----------------|-------|-----|
| Web Browser (`/browser`, Tauri `browser_*`) | **PEER-OK** | Capt Browser **PASS**. Native Webizen Browser + WebView; pages into same session as Memory is a product plus. |
| 10D / Infosphere Browser | **BEHIND** | Works as craft; discoverability under World/Keep still weak vs Chrome omnibox habit. |

- **vs-peer gap**
  - Extension/sync/profile ecosystems of Chrome/Safari/Firefox — out of scope (cherry-pick).
  - Keep browser as one track; avoid second chrome theatre.
- **Fix priority:** P2 (10D discoverability) · P3 (browser polish)
- **Owner lane:** davinci chrome · monet motion (arrive feel)

---

### Contacts / Directory — peers: Apple Contacts · Google Contacts

| Webizen surface | Grade | Why |
|-----------------|-------|-----|
| Directory / personal address book (`personal_directory.rs` · `DirectoryPane` under Relations → People) | **SHIT** | **Live but buried** — no QApp/menu/palette entry. Humans-first Directory lock fails if a person needs an agent to find it. Agreement slots empty-until-P1 is honest; bury is not. |

- **vs-peer gap**
  - Apple/Google Contacts: open app → list people. Webizen: dig Relations → People → “Open personal directory”.
  - No top-level findable address book; categorised AD-like depth without human entrance.
- **Fix priority:** P0
- **Owner lane:** davinci chrome · Capt UAT · Vibe sayables (humans-first labels)

---

### Files / Keep / Library — peers: Finder · Drive · Dropbox · Notes

| Webizen surface | Grade | Why |
|-----------------|-------|-----|
| Keep / volumes (`/keep`, Sanctuary vault) | **BEHIND** | Capt Keep **PASS** (Auto/Checkpoint/Snapshot/Pruned; Native Connected honesty). Still path-field heavy — human-alone reopen without absolute paths is the peer gap vs Finder/Drive. |
| Hypermedia Library (`/library`) | **SHIT** | Menu entry **bounces to home** (Capt). Live `library_*` / wellfair cmds exist; Poet library view placeholder — buried/false-held risk. |
| Notes-like freeform | **MISSING** | No Apple Notes / Keep-notes peer; do not stretch Dual Studio or chat drafts into one. |

- **vs-peer gap**
  - Finder/Drive: browse recent + folders. Keep: agent-known paths / truncated fields (Frame D).
  - Library must open or say **held / not yet** — bounce is worse than held.
- **Fix priority:** P0 Keep browse · P0 Library bounce
- **Owner lane:** davinci chrome · Neo daemon-connect (volume/library binds) · monet motion (Keep entrance) · Capt UAT

---

### Settings — peers: OS Settings · Gmail settings

| Webizen surface | Grade | Why |
|-----------------|-------|-----|
| Tools → Settings / `SettingsShell` + Settings Portal | **SHIT** | Capt: **listed, no visible Settings surface**. Route exists; chrome miss. Dual path (in-shell vs portal) confuses human-primary. |

- **vs-peer gap**
  - OS Settings / Gmail: menu → settings paints. Webizen: Ctrl+, may no-op visually.
  - Preferences for model, mail transport, identity scattered without one paint path.
- **Fix priority:** P0
- **Owner lane:** davinci chrome · Capt UAT (which surface is human-primary)

---

### Calendar — peers: Google Calendar · Outlook

| Webizen surface | Grade | Why |
|-----------------|-------|-----|
| Projects manifold `calendar` seed (honesty=`present`) · no QApp/route | **MISSING** | Seed/container language is not a usable Calendar app. No Desktop menu item, no human schedule loop vs Google Calendar/Outlook. |

- **vs-peer gap**
  - No day/week view, invites, or reminders as first-class human chrome (med reminders exist as tray event — not Calendar).
- **Fix priority:** P3 (honest held until product names calendar under Keep/Work)
- **Owner lane:** Capt UAT · Vibe sayables · davinci only after scope

---

### Creative Dual Studio — peers: VS Code (light) · Figma (light)

| Webizen surface | Grade | Why |
|-----------------|-------|-----|
| Dual Studio (Media manifold · Capt **PASS**) | **PEER-OK** *(light craft)* | Live Dual Studio under Media; VibeScript + GPU viewport. Not nested DCC — correct product cut. Adequate light peer for “make something” vs blank VS Code/Figma onboarding, without claiming full IDE/design-tool parity. |

- **vs-peer gap**
  - VS Code extension marketplace / Figma multiplayer — non-goals.
  - Keep Dual Studio as Media craft — not a nav peer to Ask · Keep · Talk.
- **Fix priority:** P2 (discoverability under Media / Ask craft)
- **Owner lane:** monet motion · davinci chrome · Vibe sayables

---

### Sanctuary / WellFair — care-unique

| Webizen surface | Grade | Why |
|-----------------|-------|-----|
| Sanctuary vault UI | **N/A-unique** | Capt Sanctuary **PASS**. No honest Apple/Google “vault care hub” peer — do not fake parity. Continuity handle≠who still PARTIAL. |
| WellFair care shell | **N/A-unique** | Body/rights/welfare/labour — care-unique. Keep as Keep/Care destination. |

- **vs-peer gap:** N/A (unique value). Gap is **internal honesty** (Health/Anatomy held; Continuity copy) not peer catch-up.
- **Fix priority:** P1 Continuity copy · P2 held Health/Anatomy chrome
- **Owner lane:** Vibe sayables · monet motion · Marvin (living-safe) · Capt UAT

---

### Catalog · Lexicon — unique

| Webizen surface | Grade | Why |
|-----------------|-------|-----|
| Catalog · Lexicon (`/poet/catalog` · Vibe → Script → Catalog) | **N/A-unique** | No Chrome/Docs peer. **B1–B3 PASS** at tip `9738911` / lineage `1a77869` — Catalog **findable**; held/not yet wording + living·artifact·machine chips. **B4 PARTIAL** — open-pack on real fixture still held (Desktop↔`:4242` `lexicon_manifest`); Wave-22 remains held. |

- **vs-peer gap:** N/A. Gap is daemon-connected open-pack + ban unavailable on sibling panes (Graph/Morpha/Mesh still say unavailable on same screen).
- **Fix priority:** P1 B4 daemon connect · P0 unavailable→held sweep (adjacent)
- **Owner lane:** Neo daemon-connect · davinci chrome · Capt UAT · Vibe sayables

---

### Secondary surfaces

| Surface | Peers (loose) | Grade | Why | Priority | Owner |
|---------|---------------|-------|-----|----------|-------|
| **Mesh** | AirDrop / Signal linked devices (loose) | **SHIT** | Tauri mesh cmds live; chrome **Unavailable**; buried in Connect. Ban unavailable. | P0 copy · P2 reveal | davinci · Neo |
| **Pulse / Aura / Job** | Notification Center / cron UI | **SHIT** | Pulse.* live in `ALL_BOUND`; UAT still sees unavailable panels. | P0 copy | davinci |
| **Wallet / Identity** | Apple ID / passkeys (loose) | **BEHIND** | `/identity` mounted; Continuity handle≠who PARTIAL. | P1 | Vibe · davinci |
| **Nexus** | Research notebooks (loose) | **BEHIND** | Route present; secondary knowledge craft. | P2 | davinci |
| **Admin Mission Control** | OS activity monitor | **N/A-unique** / do-not-peer | Prototype hubs — **not** first-session human. | P3 hide | Capt · davinci |
| **Ask (studio bay)** | Spotlight / Copilot search (loose) | **BEHIND** | Frame A PASS; Capability.method still loud in places; Ask must answer without agent. | P0 | davinci · Vibe · Neo |
| **Ontology workbench** | Protégé (loose) | **BEHIND** / buried | Poet present; no Desktop menu — craft under Ask, not peer. | P2 | davinci |
| **Chora** | Maps (loose) | **N/A-unique** / **PEER-OK** arrive | Capt QApps PASS; situate under World. | P2 | monet · davinci |
| **Poet shell** | — | **PEER-OK** arrive | Capt Studio/Poet PASS; sought depth PARTIAL. | P1 cherry-pick | davinci · monet |
| **QApps manager** | App Store (loose) | **PEER-OK** secondary | Working; not Ask·Keep·Talk peer. | P3 | davinci |

---

## Scoreboard (at a glance)

| App / surface | Grade |
|---------------|-------|
| Word processing | **MISSING** |
| Mail (Talk → Mail) | **BEHIND** |
| Talk / Chat | **BEHIND** |
| Browser | **PEER-OK** |
| 10D Browser | **BEHIND** |
| Directory / Contacts | **SHIT** |
| Keep / volumes | **BEHIND** |
| Hypermedia Library | **SHIT** |
| Notes-like | **MISSING** |
| Settings | **SHIT** |
| Calendar | **MISSING** |
| Dual Studio | **PEER-OK** (light) |
| Sanctuary | **N/A-unique** |
| WellFair | **N/A-unique** |
| Catalog · Lexicon | **N/A-unique** (findable; B4 held) |
| Mesh | **SHIT** |
| Pulse / Aura / Job | **SHIT** |
| Wallet / Identity | **BEHIND** |
| Nexus | **BEHIND** |
| Admin Mission Control | **N/A-unique** (keep off first-session) |
| Ask (studio bay) | **BEHIND** |
| Ontology workbench | **BEHIND** (buried) |
| Chora | **N/A-unique** / arrive OK |
| Poet shell | **PEER-OK** arrive |
| QApps manager | **PEER-OK** secondary |

---

## Fix queue — “where shit, fix it”

Ordered for Timothy’s ask. Max **8** P0/P1. Acceptance criteria sized so a cloud agent can implement without inventing Host ids.

### 1. P0 — Settings that paints
**Owner:** davinci chrome · Capt UAT  
**Acceptance:**
- Tools → Settings (`Ctrl+,`) navigates to `/settings` and **visible** `SettingsShell` paints within 1s of Native Connected cold-load.
- If portal is secondary, label it “Settings Portal (advanced)” — one human-primary path.
- Capt re-UAT flips Settings from PARTIAL → PASS (screenshot required).

### 2. P0 — Library opens or honest held
**Owner:** davinci chrome · Neo daemon-connect  
**Acceptance:**
- Tools → Hypermedia Library does **not** bounce to Talk home.
- Either lists live `library_*` / wellfair library content **or** shows **held / not yet** (never blank redirect, never “unavailable”).
- Capt Library score ≠ bounce.

### 3. P0 — Directory findable (humans-first)
**Owner:** davinci chrome · Vibe sayables · Capt UAT  
**Acceptance:**
- Command palette and/or Talk → People primary control opens Directory without agent coaching.
- Empty state: humans-first copy; agreement slots may stay empty-until-P1.
- No new Host family invent; use existing `list_directory` / `search_directory`.

### 4. P0 — Unavailable → held / not yet sweep
**Owner:** davinci chrome · Capt UAT  
**Status:** chrome landed on `cursor/held-not-yet-wait-honest-c283` — Capt re-UAT: [`CAPT_REUAT_HELD_NOT_YET_WAIT_HONEST.md`](./CAPT_REUAT_HELD_NOT_YET_WAIT_HONEST.md)  
**Acceptance:**
- Grep/UI sweep: Mesh, Pulse, Aura, Job, Graph/Morpha sibling panes on Catalog screen — zero user-visible **"unavailable"** / **"Unavailable"** for wait-honest states.
- Replace with **held / not yet** (+ short why). Live binds must not false-held.
- Frame B adjacent debt from `9738911` closed for copy; Wave-22 still gated on B4 separately.

### 5. P0 — Talk that talks (human-alone)
**Owner:** Neo daemon-connect · davinci · Vibe · Capt UAT  
**Acceptance:**
- Cold-load Talk: person sends a message in ConnectChat without agent remote-drive.
- Missing model → **held / not yet** instrument path (never peer-person chatbot; never red broken).
- `NEEDS MODEL` / “Instrument none” voice replaced with teachable held sayable.

### 6. P0 — Keep that reopens without absolute paths
**Owner:** davinci · monet · Neo · Capt UAT  
**Acceptance:**
- Keep hub offers recent / sayable volume browse (picker), not only path text field.
- Celebrate commit **only** on real `GraphDatabase.volume_commit` success.
- Frame D human-alone Keep path improves Capt score toward PASS.

### 7. P1 — Catalog B4 open-pack (Wave-22)
**Owner:** Neo daemon-connect · davinci · Capt UAT  
**Acceptance:**
- Desktop Catalog connected to live `:4242`; valid `en-core.lexicon.json` (or tip fixture) opens pack card — not stuck held when bind succeeds.
- Empty/nonsense paths remain held / not yet.
- Capt B4 → PASS or team re-scopes Wave-22 in writing.

### 8. P1 — Mail daily path under Talk
**Owner:** davinci · Neo · Vibe · Capt UAT  
**Acceptance:**
- Talk → Mail: when receiver running, person can see landed mail / purpose inbox list without leaving to Domains admin soup.
- Receiver down → **held / not yet** with one control to start `mail_receiver_*` (existing cmds).
- No claim of Gmail parity; cherry-pick inbox readability only.

**2026-09-12 implement (davinci chrome):** Talk `RelationsSection::Mail` is a first-class daily destination. `MailInboxPane` lists purpose inboxes + landed mail via `mail_list` / `list_mail_addresses`; receiver-down paints **held / not yet** and one `mail_receiver_start` control. Reply/compose stays on the purpose inbox (no Directory). SocialHub Mail tab, palette `mail`, Keep deep-link, and QApp `mail` use the same pane. Domains pane remains Reception admin, not the daily path. Not a Gmail-parity claim.

---

## Non-goals (this doc)

- No Host invent · no second lexicon · no parity theatre Desktop↔WASM  
- No fake Word/Calendar peers · no Admin as first-session  
- Ask · Keep · Talk remain teachable loops — **not** locked top-nav rename without Timothy/BRICS  

## Related

- [`HUMAN_SURFACE_APPS_AUDIT_WIP.md`](./HUMAN_SURFACE_APPS_AUDIT_WIP.md)  
- [`HUMAN_SURFACE_DESKTOP_APPS_WALKTHROUGH_d7f0bdc.md`](./HUMAN_SURFACE_DESKTOP_APPS_WALKTHROUGH_d7f0bdc.md)  
- [`HUMAN_SURFACE_PARITY_AUDIT_WIP.md`](./HUMAN_SURFACE_PARITY_AUDIT_WIP.md)  
- [`ONE_POET_PRODUCT_CUT_WIP.md`](./ONE_POET_PRODUCT_CUT_WIP.md)  
- [`HUMAN_SURFACE_COLDLOAD_UAT_SCORE_FRAME_B_DESKTOP_9738911.md`](./HUMAN_SURFACE_COLDLOAD_UAT_SCORE_FRAME_B_DESKTOP_9738911.md)  

## One-breath

> Peers win on findability and daily loops. Webizen already has Browser, Dual Studio, Sanctuary/WellFair (unique), and Catalog findable (B1–B3). Shit is chrome: Settings miss, Library bounce, buried Directory, unavailable theatre, Talk/Keep human-alone gaps. Fix those — don’t invent Word.
