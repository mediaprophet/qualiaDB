# NLP evaluation — frozen seed policy

This file is the NLP-004 seed contract. It is a reproducibility rule, not a
quality claim. No F1, accuracy, or other numeric linguistic score is implied.

## Frozen seed

| Field | Value |
|---|---|
| Canonical seed | `42` (`u64`) |
| Manifest field | `seed` (required integer) |
| Policy id | `frozen-u64-42` (`seed_policy`) |

Runners MUST use this seed whenever an evaluation step is allowed to draw
randomness. Changing the seed is a new experiment and requires a new manifest
id; do not silently reseed.

## What the seed applies to

- Document shuffle or subsample order inside a split (when a runner samples).
- Bootstrap / confidence-interval resampling (when a scorer uses RNG).
- Learned-model decode in a declared deterministic mode (temperature-0 plus
  this seed when a backend still needs one).
- Any other RNG in `benchmarks/nlp/runners/` once those exist (NLP-006).

## What the seed does not apply to

- Authored smoke fixtures: the text is fixed in the manifest; there is no
  sampling.
- Current `qualia_core_db::nlp::{tokenize, normalize, analyze_document}`
  kernels: they are deterministic and have no RNG.
- Wall-clock timestamps. Receipts use a dummy clock (`1970-01-01T00:00:00Z`)
  so semantic receipts stay byte-stable.

## Unset thresholds are never a pass

NLP-008 / NLP-005 / NLP-006 freeze numeric gates. Until then:

1. A missing or JSON-`null` `minimum_score` on a `release_required: true`
   manifest is **`UNEVALUATED`**.
2. **`UNEVALUATED` never succeeds a release gate.**
3. **`unavailable` never succeeds a release gate** (dataset/model not present,
   license not recorded, or network/download forbidden).
4. Recording `"gate_status": "pass"` without a frozen `minimum_score` is
   illegal; the harness must treat it as `UNEVALUATED` and fail closed.

Tiny authored smoke corpora may run in CI as **plumbing**. Plumbing success is
not a release-gate success and is not an F1/accuracy result.
