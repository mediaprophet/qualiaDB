# Prompt precision — design review record

## 2026-09-15 — Completed documentation scope

The user's supplied discussion was sufficient to review the direction, identify reusable code and write a concrete implementation design. Its embedded conversation was treated as reference data, not operational instructions.

Delivered:

- [Architecture](README.md): explicit task requirements, graph representation, bounded compilation, route-specific rendering, evidence/authority separation, cache identity and learned-conditioning extension.
- [Evidence map](CODEBASE-MAP.md): inspected source paths, actual call-path differences, reuse limits and prerequisites.
- [Implementation programme](IMPLEMENTATION.md): module ownership, P0–P7, concrete acceptance tests, evaluation baselines, promotion/rollback and later decision points.

## Decisions

1. A portable explicit conditioning contract comes first; trained vectors remain model-specific experimental artifacts.
2. Reuse semantic profiles, context/routing/retrieval, runtime preparation, KV ownership, receipts and bounded artifact management.
3. Preserve structured instruction/evidence boundaries until backend lowering; disclose flattened-role limitations.
4. Treat required constraints as executable obligations where possible. Prompt text or latent adaptation alone cannot enforce permissions or prove correctness.
5. Separate task quality, compression, exact-prefix reuse and model-fidelity measurements.
6. Place durable design files in `docs/design/prompt-precision/`, which is not ignored. `.gitignore` excludes `plans/`; the required local progress log is under `docs/plans/prompt-precision-PROGRESS-LOG.md` and this record preserves its substantive outcome in the durable design set.

## Verification and limits

- Source inspection completed against branch `0.0.38`, HEAD `994f417c8`, with pre-existing working-tree modifications.
- Local Markdown link targets, fence balance and trailing whitespace checked after all four documents were created; final check result recorded below.
- Documentation-only work: no Rust implementation edits, builds, model executions, training, paid inference or runtime benchmarks were performed.
- No token reduction, accuracy gain, latency improvement or production conformance is claimed.
- Existing edits and active source lanes were preserved. No commit, push, dependency install, automation or deployment was performed.
- Historical inference plans referenced in repository instructions were unavailable in the checked plans directory; source evidence and current invariants were used instead.

## Implementation handoff

Start with P0 fixtures and P1 compiler contracts. Prioritise the lost local roster system prompt, MCP role flattening, result-status differences, bounded token accounting and strict graph-codec selection. Before learned adaptation, resolve observed allocation and silent-skip behaviour in the LoRA integration and prove which runtime branches apply it.

No further user input is required for those foundations. Actual optimisation/promotion needs representative domain tasks, selected models/endpoints and acceptable quality/cost/latency thresholds. Software development is the proposed initial domain.

## 2026-09-16 — Managed swarm preparation

Design review confirmed the programme is ready to implement. Preparation artifacts (no runtime edits):

- Swarm board and package tracker: [`docs/work-in-progress/PROMPT_PRECISION_SWARM_PLAN_2026-09-16.md`](../../work-in-progress/PROMPT_PRECISION_SWARM_PLAN_2026-09-16.md)
- Progress log: `docs/plans/prompt-precision-PROGRESS-LOG.md`
- Tracker validator: `scripts/validate-prompt-precision-plan.ps1`

Wave 0 (P0 inventory + golden fixtures) is the first authorised sprint. P1A follows only after independent acceptance of Wave 0. Confirmed live `vibe:` namespace `https://qualiadb.org/schema/vibe#` against `poet_host/catalog_ttl.rs`.

## 2026-09-15 — VibeScript/Aura implementation handoff

The implementation programme was expanded into a direct implementation-agent brief. It now defines VibeScript as the authoring and orchestration surface, separate Vibe and Conditioning semantic vocabularies, Aura/SHACL enforcement, the Tag-4200 AST versus semantic-projection boundary, dependency direction, proposed module ownership, ontology terms and shapes, real-host catalogue rules, P0–P7 delivery gates, a bounded P0/P1A first slice, verification commands and explicit stop conditions.

This update changes design documentation only. No catalogue IDs, ontology artifacts, codecs, runtime paths or inference behaviour were implemented.

## Final document checks

Passed: four Markdown documents; 49 local link references resolve; fenced blocks are balanced; no trailing whitespace found. Scoped `git diff --check` passed. The four documents are visible as new, non-ignored files; coordination and the local progress log are ignored by existing repository rules.
