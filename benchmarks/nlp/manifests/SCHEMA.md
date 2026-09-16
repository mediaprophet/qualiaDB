# NLP benchmark manifest schema (NLP-004)

Machine-readable schema: [`manifest.schema.json`](manifest.schema.json).
Worked example (all required fields, `UNEVALUATED`): [`schema.example.json`](schema.example.json).
Seed contract: [`SEED_POLICY.md`](SEED_POLICY.md).

This scaffold records *how* a task will be scored. It does **not** freeze
current tokenizer/normalizer output as a quality baseline. NLP-002 may change
those kernels. Do not treat smoke plumbing as F1 or accuracy.
NLP-005 authored corpora and unavailable-dataset rows: [`docs/benchmark-datasets/nlp/DATASETS.md`](../../../docs/benchmark-datasets/nlp/DATASETS.md).

## Required fields

| Field | Meaning |
|---|---|
| `id` | Stable manifest identity (`nlp-004-…`). |
| `task` | Task name (`tokenize`, `normalize`, `analyze_document`, …). |
| `language` | BCP-47-like tag (`en`). |
| `dataset_id` | Dataset identity (authored id or upstream id). |
| `split` | `smoke` / `train` / `dev` / `test` / `calibration`. |
| `metric_names` | Metrics that *will* be scored when data exists. Names only. |
| `seed` | Frozen `u64` (see seed policy). |
| `seed_policy` | Policy id, currently `frozen-u64-42`. |
| `resource_envelope` | Caps + `network: "forbidden"` for CI. |
| `gate_status` | See enum below. |
| `release_required` | If true, this row blocks a release until it *passes*. |

Plan §6.2 also requires (present here, often null until NLP-005/006):
`dataset_revision`, `split_hash`, `domain`, `profile`, `label_mapping`,
`scorer_revision`, `primary_metric`, `minimum_score`, `achieved_score`, `baseline_score`,
`noninferiority_margin`, `minimum_effect`, `confidence_method`,
`coverage_floor`, `latency_budget`, `memory_budget`.

## `gate_status` enum

| Value | Release gate | When to use |
|---|---|---|
| `UNEVALUATED` | **does not pass** | Required threshold unset, or no official score yet. |
| `unavailable` | **does not pass** | Dataset/model absent; license/not-present reason required. |
| `blocked` | **does not pass** | Blocked on an out-of-band decision (not used in Wave 0 smoke). |
| `fail` | **does not pass** | Scored and below a *frozen* threshold (NLP-006+). |
| `pass` | may pass only with frozen `minimum_score` **and** a measured `achieved_score >= minimum_score` (NLP-006) | Illegal while `minimum_score` or `achieved_score` is null. A handwritten `gate_status: pass` without a measured score is UNEVALUATED. |

## CI vs release

- **Smoke** (`nlp-004-smoke-tokenize-en.json`): tiny authored English
  sentences, `network: forbidden`, no download. CI runs plumbing. Status
  stays `UNEVALUATED`.
- **External stub** (`nlp-004-ud-en-ewt-unavailable.json`): `unavailable`
  with a license/not-present reason. Must not succeed a release gate.

Do not download datasets or models from these manifests.
