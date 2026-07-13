# ADR 0003 — Exactness is an explicit, declared profile, not a hidden server preference

**Status:** Accepted (Editor's Draft 0.1, provisional — not a W3C/OGC standard)
**Date:** 2026-07-13
**Requirements:** QISP-R05, QISP-R10
**Plan sections:** §0 layer 3, §4.4, §6.3, decisions QISP-D05/D06

## Context

The renderer/GPU can answer many spatial questions fast but approximately; the computational-geometry
library answers them exactly but sometimes slowly. If the server silently chose, a caller could not
tell whether a `qispf:occludes` result is a robust mesh ray test or a render-space guess — and a
rights-affecting decision could rest on an approximation without anyone knowing. SPARQL optimizers
may also reorder or repeat expression evaluation, so results must be stable and reproducible.

## Decision

Exactness is an **explicit function argument or query/job profile**, never an invisible server
setting (§4.4). Three initial profiles:

- `qisp:Exact` — robust/exact predicates and constructions where supported; the reference semantics;
- `qisp:DeterministicApproximate` — reproducible approximation with **declared** absolute/relative
  error bounds;
- `qisp:InteractiveApproximate` — renderer/GPU-oriented result, **never** accepted for a
  rights-affecting policy decision without a separate exact verification.

Every function descriptor (§4.2) carries its highest available exactness (`qispf:exactness`) and
whether it accepts a trailing exactness argument (`qispf:takesExactnessArg`). Approximate/GPU results
must declare their profile and error bound (QISP-R10). Expression functions are referentially
transparent within one query snapshot; the memo key includes the CRS/profile and
exactness/backend policy so a reordered or repeated call returns the same term. Numeric measurements
must state a QUDT unit (decision QISP-D06); a bare undocumented float is prohibited.

## Consequences

- **Positive:** callers can *require* `qisp:Exact` for rights-sensitive work; approximate results are
  self-describing (profile + error bound); GPU kernels are admitted only through capability manifests
  with CPU-oracle tests, so exactness claims are checkable, not promises (§6.3). Determinism makes
  results cacheable and optimizer-safe (QISP-R06).
- **Negative / cost:** every immersive function needs at least one CPU exact oracle and, for GPU
  paths, differential tests against it; two-profile operations (e.g. `occludes`) double the test
  surface; callers must choose a profile rather than get a silent default.
- **Follow-on:** whether exactness stays a per-call argument or becomes a query-level profile is
  decision QISP-D05 (per-call for MVP, revisit after interop testing).
