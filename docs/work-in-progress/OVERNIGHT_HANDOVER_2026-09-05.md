# Overnight handover — QualiaDB / Poet (2026-09-05)

**Audience:** Timothy + any agent continuing while the day crew sleeps  
**Branch:** `0.0.36-dev`  
**Tip (push lane):** `64b213848201f8e9f4a6257d0390b55ca13ea2ef`  
**Host freeze:** `vibe-host-0.1` — no Host widen, no SemVer bump until release  
**Sole Git push:** Neo (check → commit → push)

This file is the **current-state scoreboard + remaining plan**. Older registers
(`POET_NEXT_WORK_REGISTER_*`, `POET_UPDATED_IMPLEMENTATION_PLAN_*`) still hold
broader POET/Health/Tool Chest backlog; this doc wins for **tonight’s UAT seam**.

---

## How to continue overnight

1. Pull `0.0.36-dev` tip above (or newer if Neo pushes).
2. Daemon: `qualia-cli daemon --dev --port 4242` — health is **`/health`** (not `/healthz`).
3. Poet: `trunk serve` in `crates/poet` → `http://127.0.0.1:8080` (IPv4 literal).
4. Tick items below; send Neo a PASS/FAIL + tip SHA for any code land; Neo pushes.
5. Prefer incremental land-on-tip under `vibe-host-0.1`.

**Decentralized networking (Codex/Cursor):** own feature branch off tip
(e.g. `feat/decentralized-net`) → single PR into `0.0.36-dev` (Neo reviews/merges).
Do **not** land that stack straight on tip.

---

## Scoreboard (UAT / G-LEXICON-0 seam)

| ID | Item | State | Tip / evidence | Owner lane |
|----|------|-------|----------------|------------|
| G0 | G-LEXICON-0 slice1 `lexicon_manifest` | **PASS** | `720bd5a9` lineage; missing → held/not yet | Neo / Marvin |
| G0f | Vibe diagnose / `lexicon:` fixtures | **PASS** | PR #74 / `07ea593` | Vibe |
| D1 | Catalog · Lexicon held-gate chrome | **PASS** | `e070ffc7`+ | davinci / monet |
| D4–D5 | living/artifact/machine chips + Zone D | **PASS** | `f1d34d03` | davinci / monet |
| B1–B2 | Sanctuary `volume_open` → `volume_commit` (HTTP) | **PASS** | `0b30cb15` sticky + create-on-open | Neo / Capt |
| N1 | Cold-load Native Connected (:4242) | **PASS** | `64b21384` hard-reload, held ~20s | Neo / Capt |
| D2 | **Open pack arrive card** (packSemVer · framing · gate) | **OPEN** | Connected OK; daemon `lexicon_manifest` live-OK (0.1.0·mixed·gate open) — UI/path/bay only. Prior path truncated `/wor`; wait ≥10–15s; soft-rise pending | davinci / monet / Capt |
| B-ui | Sanctuary Save / Checkpoint UI path | **OPEN** | HTTP commit PASS; dialog flaky | Capt / davinci |
| A-wish | office:graph sayables-first labels | **WISHLIST** | not blocking Catalog close | davinci / Vibe |
| G-COORD | GIS bind deepen | **HELD** | yellow voice fixed; deepen after UAT | Neo |

---

## Exact next beats (do these in order)

### 1) Close D2 — Open pack arrive (highest priority)

**Fixture path (full, not truncated):**

```text
crates/vibe/fixtures/lexicon/en-core.lexicon.json
```

**UAT steps**

1. Confirm badge **Native: Connected (:4242)** on cold-load (`64b21384`+).
2. Zone D / vibe-console → **Catalog · Lexicon**.
3. Paste **full** fixture path → **Open pack**.
4. Wait **≥10–15s** (5s was often too short in prior UAT).
5. Expect arrive card: `packSemVer 0.1.0` · framing **mixed** · gate open · living/artifact chips (not red missing).
6. Absolute path also OK on Capt box: `/workspace/qualiaDB/crates/vibe/fixtures/lexicon/en-core.lexicon.json`
7. monet: soft-rise score **only** on real arrive (no fake celebrate).
8. Capt: frame + PASS/FAIL + full tip SHA → Neo updates this scoreboard + UAT checklist.

**If FAIL with full path:** Neo digs `lexicon_bay` / `daemon_invoke(GraphDatabase.lexicon_manifest)` click-path (held vs arrive chrome). Daemon bind already live-OK on en-core earlier.

### 2) Sanctuary UI commit beat (after D2 or parallel)

- HTTP open→commit already PASS on `0b30cb15`.
- Remaining: Save / Checkpoint dialog → `volume_commit` from UI without flaky path entry.
- Volume chip should leave **CLOSED** after successful open when product says so.

### 3) Probe hygiene (done — only reopen on regression)

Landed lineage:

| Tip | Change |
|-----|--------|
| `cc5ecb6` | Connected after `/health`; caps in background; trim 4243/8080 |
| `a9e14a8` | Plain fetch (AbortController regression) |
| `ecb99d6` | hostname loop (superseded — caused ::1 hangs) |
| `55c2c58` | IPv4-only + soft race (2.5s too short) |
| **`64b21384`** | **:4242 only, 12s soft timeout, Offline retries — cold-load PASS** |

### 4) Wishlist / later (not tonight blockers)

- office:graph: human sayables first, `Capability.method` secondary.
- 10d / glb seams: artifact/OWL-ok; living depicted = SHACL-first — ping Neo to extend format, no parallel vocab.
- G-COORD deepen after D2 PASS.
- Broader POET Health / Tool Chest packets: see `POET_NEXT_WORK_REGISTER_2026-09-05.md`.

---

## Lane checklists (fill / tick as you go)

### Capt (ops / UAT)

- [x] Sanctuary HTTP open→commit PASS (`0b30cb15`)
- [x] Cold-load Connected PASS (`64b21384`)
- [x] Daemon `lexicon_manifest` live-OK on en-core (HTTP)
- [ ] D2 Open pack arrive PASS with **full** en-core path + frame (≥10–15s wait)
- [ ] Optional: UI sanctuary Save path once D2 green
- [ ] Keep tip SHA in every report

Capt box runbook + evidence tables: `docs/work-in-progress/CAPT_LANE_AMENDMENTS_2026-09-05.md`

Vibe language/diagnose lane: `docs/work-in-progress/VIBE_LANE_HANDOVER_2026-09-05.md`

### davinci (chrome / click-path)

- [x] Catalog held-gate + chips PASS
- [ ] Arrive chrome after Open pack (loaded-pack framing, not stuck on Open pack button)
- [ ] If click-path broken with full path → note DOM/capability id for Neo

### monet (motion)

- [ ] Soft-rise score on real arrive only
- [ ] Commit motion only on real `volume_commit` success (HTTP already OK)

### Vibe (language / diagnose)

- [x] Diagnose bar accept (held/not yet; packSemVer · framing · gate)
- [x] G-LEXICON-0 / fixtures / held-gate string PASS (see VIBE_LANE_HANDOVER)
- [ ] Confirm arrive copy matches diagnose accept when D2 lands
- [ ] office:graph sayables-first stays DevRel wishlist (B, not overnight)

### Marvin (framing)

- [x] mixed + packSemVer + gate open = LexiconPack framing
- [x] 10d/glb = artifact; living subjects SHACL-first
- [ ] No Thing-wash on arrive card chips

### Neo (seam / push)

- [x] create-on-open + sticky host + probe cold-load
- [ ] On D2 FAIL with full path → fix bay/invoke and push
- [ ] Fold lane handovers into this file; push GH
- [ ] Tick scoreboard rows as Capt/davinci/monet report

---

## Key paths

| What | Path |
|------|------|
| UAT checklist | `docs/work-in-progress/uat-office-graph-volume-vibe-host-0.1.md` |
| Lexicon fixture | `crates/vibe/fixtures/lexicon/en-core.lexicon.json` |
| Volume bind | `crates/qualia-core-db/src/poet_host/invoke/graph/volume.rs` |
| Sticky HTTP host | `crates/qualia-core-db/src/services/poet_api.rs` |
| Daemon probe | `crates/poet/src/browser/native_daemon.rs` |
| Lexicon bay UI | `crates/poet/src/browser/lexicon_bay/` |
| Sanctuary UAT file | `/workspace/qualia-data/uat-sanctuary.q42` |

---

## Constraints (do not break)

- No Host widen / no dotted `qualia.*` / Capability.method + `ALL_BOUND` only.
- Living copy: **held / not yet**, never “broken”.
- Fail-closed sanctuary empty-graph commit stays honest.
- Capt often lags tip — always ping **full SHA** when landing.

---

## Changelog (this handover)

| When | Note |
|------|------|
| 2026-09-05 night | Created for Timothy overnight continuity; tip `64b21384`; D2 Open pack arrive is the open gate. |
| 2026-09-05 ~23:10 AEST | Folded Capt lane amendments: D2 wait ≥10–15s; `/wor` truncation; daemon bind PASS; Capt runbook sidecar. |
| 2026-09-05 ~23:13 AEST | Folded Vibe lane handover sidecar (diagnose PASS locks; D2 language accept rules). |
