# Agent task brief template

Copy into a scoped assignment record when implementation is authorized. The integrator updates
the [register](../task-registry.json); do not edit it concurrently with workers.

## Assignment

- Parent package and exact child IDs:
- Outcome and explicit non-goals:
- Agent/owner, independent reviewer and integration owner:
- Accepted dependency evidence and provisional assumptions:
- Source branch/HEAD, dirty-state fingerprint, instruction versions:
- Start, heartbeat, expiry/review time and next checkpoint:
- Available CPU/memory/storage/test budgets:

## File and interface ownership

- Exact allowed existing files and new directory prefixes:
- Shared files forbidden to this worker (exports, Cargo, registries, profiles):
- Concurrent owners checked and required shared-owner changes:
- Single-purpose file map and expected public API:
- Existing oversized files and approved decomposition record:
- Tier-1, Tier-2, storage, backend and test ownership:
- Interface versions consumed/produced, downstream recipients:

## Required work and acceptance

- Selected checklist items in implementation order:
- Semantic use cases, negative cases and failure behavior:
- Caller storage, work quanta, cancellation and resource limits:
- Persistence, identity, crypto and irreversible-effect boundaries:
- Exact test/measurement commands and expected outcomes:
- Retained evidence location, byte budget and access policy:
- Rollback/recovery method and affected callers:
- Conditions requiring domain review or task rescoping:

## Registry claim proposal

The integrator stores `claim` as an object with `owner`, `source_state`,
`allowed_paths`, `started_at`, `review_at` and `interface_versions`.
Paths are repository-relative exact files or explicit owned directory prefixes, with exclusions
recorded in the brief. `assignee` is the same owner identifier. A claim expiry requires checking
that the previous writer has stopped; time alone does not transfer ownership. Keep assignments
small enough to integrate and review, rather than assigning a whole workstream blindly.
