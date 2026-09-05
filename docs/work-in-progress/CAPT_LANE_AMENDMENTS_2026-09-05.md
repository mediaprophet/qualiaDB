# Capt lane amendments — overnight (2026-09-05)

**Lane:** Capt — Technical Ops & Integration / Poet browser UAT  
**Fold into:** `docs/work-in-progress/OVERNIGHT_HANDOVER_2026-09-05.md`  
**Against tip:** `c0aec4ed03f376b3f2037e6509a3003b1c3f69a2` (handover) / UAT tip `64b213848201f8e9f4a6257d0390b55ca13ea2ef`  
**Sole push:** Neo

---

## Scoreboard deltas (Capt evidence)

| ID | Item | Capt state | Evidence |
|----|------|------------|----------|
| B1–B2 | Sanctuary HTTP open→commit | **PASS** | Delete stub → `volume_open` `/workspace/qualia-data/uat-sanctuary.q42` → `created: true`, `quin_count: 1`; separate HTTP `volume_commit` → `written: 1` (no empty-graph fail-closed). Tip `0b30cb15`. |
| N1 | Cold-load Native Connected | **PASS** | Hard-reload on `64b21384`; Connected (:4242) immediately; held ~20s; no badge re-click. Frames: `/workspace/uat-64b21384-native.png`. |
| D2 | Open pack arrive | **OPEN** | Connected OK. Open pack with path still left Catalog · Lexicon + Open pack / `held / not yet`; no packSemVer · framing · gate card. One frame showed path truncated to `/wor` — **retry must paste full fixture path**. No DENIED / NO DAEMON. Frames: `/workspace/uat-64b21384-lexicon-arrive.png` (also older `uat-0b30cb15-lexicon-arrive.png`, `uat-a9e14a84-*`). |
| G0 bind | `GraphDatabase.lexicon_manifest` HTTP | **PASS** (daemon) | Live OK on en-core earlier: packSemVer **0.1.0**, framing **mixed**, gate **open**. So D2 FAIL/OPEN is UI click-path / path / bay chrome, not missing bind. |
| B-ui | Save Checkpoint dialog | **OPEN** | HTTP commit closed; UI dialog flaky — after D2. |

---

## Capt overnight runbook (box)

### Daemon (:4242)

```bash
# Separate target dir so trunk does not lock the daemon build
export CARGO_TARGET_DIR=/workspace/qualia-cli-target
export QUALIA_DATA_DIR=/workspace/qualia-data
# Needs dbus + gnome-keyring in session
dbus-run-session -- bash -lc '
  eval $(gnome-keyring-daemon --start --components=secrets)
  /workspace/qualia-cli-target/debug/qualia-cli daemon --dev --port 4242
'
```

- Health: **`GET http://127.0.0.1:4242/health`** (not `/healthz`).
- Invoke: `POST http://127.0.0.1:4242/invoke`.
- Probe tip `64b21384`: **:4242 only**, soft timeout **12s**, offline auto-retry 3s × max 10.

### Poet UI (:8080)

```bash
cd /workspace/qualiaDB/crates/poet
env -u NO_COLOR trunk serve   # → http://127.0.0.1:8080/  (IPv4 literal)
```

Hard-reload after tip land; cold Connected should stick without badge re-probe on `64b21384+`.

### D2 UAT (Capt owns report)

1. Confirm `● Native: Connected (:4242)` on cold-load.
2. Everyday → Script → Vibe REPL → **Catalog · Lexicon**.
3. Paste **full** path (do not type short; watch field for truncation):

```text
crates/vibe/fixtures/lexicon/en-core.lexicon.json
```

   Absolute also valid on Capt box:

```text
/workspace/qualiaDB/crates/vibe/fixtures/lexicon/en-core.lexicon.json
```

4. **Open pack** → wait ≥10–15s (prior UATs used 5s — may be short).
5. **PASS** iff arrive shows packSemVer **0.1.0** · framing **mixed** · gate open (soft-rise for monet).
6. Report Neo: PASS/FAIL + full tip SHA + frame paths. Tick-offs via Neo only.

### If D2 FAIL with verified full path

- Confirm daemon still Connected + HTTP `lexicon_manifest` still live-OK.
- Capture console `[Webizen Probe]` + network `/invoke` for Open pack.
- Hand Neo: bay click-path / `lexicon_bay` / held-vs-arrive chrome — **do not invent Host methods**.

---

## Ops process locks (keep in handover)

- Gates that block work → report Capt; Capt delegates ungate; owner reports when done.
- UAT finds = open todos, not hard fails (Timothy via monet).
- Living copy: **held / not yet**, never “broken”; no fake celebrate on soft-rise.
- Product order: finish Poet (tools/UI/vibe + UAT) under `vibe-host-0.1` before release cut / Webizen Desktop; Solid IdP parked.
- Capt often lags tip — always cite **full SHA** when landing / reporting.
- Sole Git push: Neo. Capt drafts docs/UAT log edits locally; Neo commits + pushes GH.

---

## Capt checklist (overnight)

- [x] Sanctuary HTTP open→commit PASS (`0b30cb15`)
- [x] Cold-load Connected PASS (`64b21384`)
- [x] Daemon `lexicon_manifest` live-OK on en-core (HTTP)
- [ ] D2 Open pack arrive PASS with **full** en-core path + frame (≥10s wait)
- [ ] Ping Neo PASS/FAIL + tip SHA for scoreboard tick
- [ ] Optional after D2: UI sanctuary Save / Checkpoint
- [ ] Keep `uat-office-graph-volume-vibe-host-0.1.md` in sync via Neo

---

## Key Capt box paths

| What | Path |
|------|------|
| Checkout | `/workspace/qualiaDB` |
| Daemon binary | `/workspace/qualia-cli-target/debug/qualia-cli` |
| Sanctuary UAT volume | `/workspace/qualia-data/uat-sanctuary.q42` |
| UAT screenshots | `/workspace/uat-*.png` |
| Lexicon fixture | `/workspace/qualiaDB/crates/vibe/fixtures/lexicon/en-core.lexicon.json` |

---

## Changelog

| When | Note |
|------|------|
| 2026-09-05 ~23:10 AEST | Capt amendments for Neo fold — D2 still OPEN; path truncation + short wait called out; runbook + HTTP bind PASS. |
