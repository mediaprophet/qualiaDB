# Capt re-UAT — held / not yet wait-honest chrome

**Lane:** Capt · Desktop / Poet Catalog screen  
**Branch:** `cursor/held-not-yet-wait-honest-c283` (base `0.0.38`)  
**Sole push:** Neo  
**Do not merge.**

## What landed

Wait-honest chrome no longer paints user-visible **"unavailable" / "Unavailable"**.
Copy is **held / not yet** plus a short why.

| Surface | Cold (daemon down) | Daemon connected, empty |
|---------|--------------------|-------------------------|
| Mesh topbar | held / not yet — no live mesh bind | same (no Mesh.* Host family; no invent) |
| Pulse dock | held / not yet — Pulse waits on the local daemon | **waiting** (Pulse.* is live ALL_BOUND — do not false-held) |
| Aura / SHACL | held / not yet — SHACL waits on the local daemon | **waiting** (SHACL.* is live ALL_BOUND) |
| Job dock | held / not yet — no live job-queue bind | same (no Job.* family) |
| Graph footer | held / not yet — graph waits on the local daemon | **live · N quins** (GraphDatabase.*) |
| Merkle / Gas / Strata | held / not yet + why (not mounted) | still held until those mounts exist |
| Catalog · Lexicon | held / not yet — open lexicon pack | B4 open-pack still Wave-22 (not this PR) |
| Telemetry & DAG pods | held / not yet + why | same until DAG/quads connect |

Locks: human-alone · chatbot is tool · human ≠ law’s person for NaturalAgent · no Host invent · no false-held live ALL_BOUND.

## Re-UAT steps (Catalog screen)

Cold-load Desktop Poet or trunk Poet. Everyday → Script → **Catalog · Lexicon**.

1. **Mesh** (topbar): reads held / not yet + why. No "Mesh unavailable".
2. **Aura / Pulse / Job** (right dock badges + bodies): no "unavailable". Daemon down → held + why. Daemon up → Pulse/Aura say **waiting**, not held.
3. **Graph** (footer): daemon down → held / not yet — graph waits on the local daemon. Daemon up → **live · N quins** (not held).
4. **Merkle** (footer / Telemetry sidebar): held / not yet, not unavailable.
5. Catalog gate still **held / not yet — open lexicon pack** on empty/nonsense path. Valid pack + live `:4242` is B4 / Wave-22 (separate).
6. Grep the painted DOM / visible chrome: zero `unavailable` / `Unavailable` on those wait states.

## Not this PR

- B4 Catalog open-pack against `:4242` (Wave-22).
- Talk / Keep human-alone P0s.
- Presence status `here | away | unavailable` (roster state, not wait chrome).
- Internal `data-honesty` legacy token `unavailable` still folds to painted **held**.

## Report back

PASS/FAIL per row + tip SHA + frames. Neo folds; do not merge.
