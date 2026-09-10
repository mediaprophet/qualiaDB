# Assignment CSCP-07 — QUIC ALPN evaluation

- Parent package and exact child IDs: CSCP-07.01, CSCP-07.02, CSCP-07.03
- Outcome: a decision record. Explicit non-goals: do not add quinn/noq/iroh dependencies; do not implement QUIC; do not set `noq_transport_admitted()` true; do not edit honesty flags or Cargo.toml.
- Agent/owner: swarm-cscp-07. Integrator owns `mod.rs` / Cargo / flags.
- Source branch/HEAD: `0.0.38` at Wave 1 commit; do not leave the branch.
- Allowed writes only:
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/decisions/CSCP-07-quic-alpn.md`
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/handoffs/CSCP-07.md`
- Forbidden: any `.rs` file, `Cargo.toml`, original 30-package checklists, `workstream.md`, `task-registry.json`, `progress-log.md`.

## Required work

Read:

- `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/quic-native-connectivity-research-2026.md`
- `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/capability-scoped-connection-fabric.md`
- `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/draft-webcivics-cscp-00.md` (ALPN `cscp/1`)
- `crates/qualia-core-db/Cargo.toml` and workspace `Cargo.toml` for quinn/noq/iroh
- `crates/qualia-core-db/src/net/peer/connectivity/evidence.rs`

Write the decision record from the template `qdnf-imp/templates/decision-record.md`. Options must include: (1) admit quinn, (2) evaluate/admit noq, (3) defer. Use live source evidence. If no QUIC crate is present, prefer defer unless a measured local loopback admission is already in-tree (it is not).

CSCP-07.02: if deferred, state that ALPN `cscp/1` is reserved in the draft and **not implemented**. Do not add ALPN glue.

CSCP-07.03: confirm `noq_transport_admitted()` is still `false` in source. Quote the function. Do not change it.

Handoff using `qdnf-imp/templates/handoff.md`.
