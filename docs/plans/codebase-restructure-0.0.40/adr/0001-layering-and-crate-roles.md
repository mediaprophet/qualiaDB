# ADR 0001 — Layering and crate roles (restructure)

- **Status:** Proposed (docs pack `codebase-restructure-0.0.40`)
- **Date:** 2026-09-27
- **Context tip:** `0.0.40-dev` @ `1cdaa6321`
- **Deciders:** Timothy Holborn (principal); implementers follow once accepted

---

## Context

QualiaDB grew emergently. `qualia-core-db` accumulated graph ABI, inference, specialized sciences, Poet hosting, networking, WASM bridges, and daemons in one Cargo package (~723k lines) with 13 workspace dependents. Parallel UI hosts (studio, desktop, poet) and a gitignored `docs/plans/` tree make “where does this go?” expensive for humans and agents.

A prior 0.0.17 **file-split** plan and an in-crate **MODULE_REORG** plan exist; neither defines **workspace layering**.

## Decision

Adopt three product tiers for all new work and for phased extracts:

1. **Core libraries** — dep-light, ABI-stable, wasm-lite safe (`vibe`, `qualia-inference-kernel`, future quin/q42/storage slices).
2. **Domain modules** — optional capabilities (wellfare, vision, audio, sciences, net, inference runtime, poet-host, …). May depend on core; never on apps.
3. **Apps / hosts** — desktop, studio, cli, poet, wasm products, peer facade. Depend on modules/core only.

**Forbidden edges:** core → module-upward or app; module → app; lite-wasm → specialized_libs / poet_host / client-core.

**Migration style:** facade + `pub use` shims; one family extract per phase; CI native + wasm gates; no big-bang repo split.

**Product invariants unchanged:** natural-person fiduciary framing; Continuity handle ≠ who; agents are sub-agents of human principals; civics (citizen-led) tone — not government-portal.

## Consequences

### Positive

- Shared vocabulary for PRs and agent prompts.
- Clear pilot extract (geometry) without renaming the world.
- Protects `webizen-lite-wasm` size budgets structurally.

### Negative / costs

- Temporary duplicate paths via re-exports.
- More workspace members over time (manage with DIRECTORY_INDEX + this map).
- Requires discipline: rejecting “just put it in core-db” shortcuts.

### Follow-ups

- Accept/reject this ADR in review of the docs PR.
- Execute Phase 0–1 of `04-migration-plan.md` before code extracts.
- Do not treat root `DECOMPOSITION_PLAN.md` as the workspace architecture plan.

## Alternatives considered

| Alternative | Why not (now) |
|-------------|----------------|
| Multi-repo split | High coordination cost; breaks single CI/pages story |
| Keep one crate forever; only folders | Folders help but do not shrink dependency/feature blast radius |
| App-first merge (Poet+Studio one crate) | UI ownership still unsettled; wrong first move |
| Delete specialized_libs to “simplify” | Destroys capability; extract/feature-gate instead |
