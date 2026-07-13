# ADR 0004 — Asynchronous work is an explicit QISP job protocol, never disguised SPARQL

**Status:** Accepted (Editor's Draft 0.1, provisional — not a W3C/OGC standard)
**Date:** 2026-07-13
**Requirements:** QISP-R11, QISP-R01, QISP-R12
**Plan sections:** §0 layer 4, §7.1, §7.2, decision QISP-D08; RFC 7240, RFC 9457

## Context

Constructive geometry and large tensor batches can exceed a synchronous request budget. SPARQL
Protocol has no standard job model. It would be wrong to return a "job document" in place of a
SPARQL result on `/sparql` — that silently breaks conformance and confuses generic clients (§14). But
long-running work still needs status, cancellation, provenance, and result links.

## Decision

Expose async work as a **separate, advertised QISP job protocol**, not as covert SPARQL behavior:

- keep `/sparql` fully SPARQL-Protocol conformant; it deterministically **rejects** a request that
  requires async execution rather than fabricating a fallback (QISP-R01, §7.1);
- provide a distinct endpoint (e.g. `/qisp/jobs`) first; only later add an explicit
  `Prefer: respond-async` opt-in on `/sparql` after proxy/cache conformance testing (decision
  QISP-D08). When honored, return `Preference-Applied: respond-async`, `202 Accepted`, and an
  absolute `Location` job URI (RFC 7240);
- the job resource supports GET status/progress, DELETE/cancel (authorized), negotiated result links,
  SSE progress, optional WebSocket subscription, expiry/cleanup, idempotency keys, and a
  digest-bound requester DID/session with audit provenance;
- define the job **state machine** ourselves (RFC 7240 deliberately does not):
  `admitted → queued → running → succeeded|failed|cancelled`, with `expired` only before execution,
  then `expired → purged`. Transitions are monotonic and revisioned;
- protocol/job errors use **RFC 9457 Problem Details** with stable HTTPS type IRIs that do not leak
  internal paths, geometry details, policy rules, or principal identifiers;
- a successful job result is a **leased** resource; durable persistence is a separate governed
  mutation/promotion operation (QISP-R12).

## Consequences

- **Positive:** ordinary SPARQL clients are never surprised (QISP-R01); jobs are authenticated,
  monotonic, expiring, and non-enumerable across agents (QISP-R11 test targets); cancellation reuses
  the geometry cancellation token and daemon SSE patterns rather than a QISP-only scheduler silo
  (§10.2).
- **Negative / cost:** a durable, budgeted job table and state machine must be built (the in-memory
  WebSocket session structures are not a sufficient durable registry); `Prefer`-varying responses
  must follow RFC 7240 cache rules; two result-delivery surfaces (sync + job) must stay consistent.
- **Follow-on:** streaming/continuous queries (§15d) and the WebRTC transport (§15e) reuse this job
  state machine rather than inventing new lifecycles.
