# Methodology — exhaustive crate → Vibe → Poet incorporation

**Status:** Normative for incorporation work (2026-09-06)  
**Constraint:** `docs/work-in-progress/VIBE_HOST_CONSTRAINT_CORRECTION_2026-09-06.md`  
**Tools:**
- `scripts/vibe_incorporation_backlog.py` — **exhaustive** pub-fn backlog (this methodology)
- `scripts/vibe_surface_gap_review.py` — heuristic module triage (supplementary)

## 1. Product pipeline (order of work)

```
Built Rust libraries (crates/*)
        │
        ▼
① VibeScript surface = Host Family.method
   (vibe ALL_INVOKE_IDS + poet_host ALL_BOUND + invoke handler)
        │
        ▼
② Poet Live / Tool Chest cites those ids (capability_scope dual-path)
```

Poet does **not** define capabilities. If a library function is not a Host id, Poet
cannot honestly Live-invoke it.

`vibe-host-0.1` is the **outcome** of finishing this incorporation, not a freeze
that forbids new Host ids.

## 2. Why the earlier search was incomplete

| Old approach (`vibe_surface_gap_review.py`) | Gap |
|---|---|
| Priority globs + keyword → family heuristics | Misses crates/modules outside globs; false “covered” when a keyword hits an unrelated family (`Agency`, `ComputerVision`) |
| Module-level gap scores | Does not list **unbound `pub fn`s** |
| No method-name join to `*.method` | Cannot say “`delegation_permits` has no Host twin” |
| Freeze-era “Desktop-only” bias | Treated unbound as out-of-scope instead of backlog |

## 3. Exhaustive scan definition

### 3.1 Universe

All workspace members under `crates/*/src/**/*.rs`, excluding:

- `target/`, generated paths
- files whose stem is `tests`, `*_tests`, `bench`, `mock`, `stub`, `generated`
- `#[cfg(test)]` modules are still in file text; prefer skipping paths under `/tests/`

### 3.2 Extracted atoms

For each file, extract:

- `pub fn` / `pub async fn` / `pub(crate) fn` (optional flag; default **pub only**)
- `pub struct` / `pub enum` / `pub trait` (context only; backlog prioritizes **functions**)

Record: `crate`, `module_path` (from file path under `src/`), `fn_name`, `rel_path`.

### 3.3 Host join (mechanical)

Let `M` = set of method suffixes from ALL_BOUND (`Family.method` → `method`).

For each `pub fn name`:

| Condition | Classification |
|---|---|
| Some `Family.name` ∈ ALL_BOUND | **Host-bound** (exact method match) |
| Else | **Host-missing** → Queue **Q1** (VibeScript bind candidate) |

Exact match is intentionally strict. Fuzzy / stem matches are reported separately as
**hints**, never as proof of coverage (avoids Agency/ComputerVision false positives).

### 3.4 Poet join

Let `P` = Poet Live / `capability_scope` Host id strings.

| Condition | Classification |
|---|---|
| id ∈ ALL_BOUND ∧ id ∉ P | Queue **Q2** — Host exists, **Poet consume** |
| id ∈ P ∧ id ∉ ALL_BOUND | Defect — stale Poet string |
| id ∈ ALL_BOUND ∧ id ∈ P | Incorporated (Host+Poet) |

### 3.5 Catalog integrity

| Condition | Classification |
|---|---|
| ALL_BOUND ≠ ALL_INVOKE_IDS | Queue **Q0** — fix drift before new binds |

## 4. Output artifacts

Run from repo root:

```bash
python scripts/vibe_incorporation_backlog.py
# defaults:
#   docs/work-in-progress/VIBE_INCORPORATION_BACKLOG.md
#   docs/work-in-progress/VIBE_INCORPORATION_BACKLOG.json
```

Markdown for humans; JSON for swarm packetization (`q1_host_missing`, `q2_poet_consume`).

## 5. How agents use the backlog

1. Fix **Q0** catalog drift if any.
2. Pick **Q1** rows by crate priority (core specialized_libs → modalities →
   cooperative-core → vision extractables → client-core pure helpers).
3. For each bind: paired catalogs + invoke handler + focused tests (**vibescript first**).
4. Immediately or next packet: Poet Live dual-path for new ids (**Q2** grows then shrinks).
5. Re-run the script; counts must move: Host-missing ↓, Poet-consume ↓ for shipped work.
6. Shell/windowing-only APIs may stay Desktop — mark in JSON `defer_reason`, do not
   silently omit from the scan.

## 6. Honesty rules

- Every new Host id needs a real handler over existing code (or a thin Value codec
  over caller-supplied data) — no empty stubs.
- Sensitivity / consent fail-closed for person-touching surfaces.
- Do not overload `Agency.*` with cooperative ABAC; use distinct family names.
- Measurement: report script counts; do not claim “exhaustive product done” until
  Q1/Q2 are intentionally empty or explicitly deferred with reasons.
