# CSCP-07 handoff — QUIC ALPN evaluation

## Result

- Package and child IDs completed or still pending:
  - CSCP-07.01 **complete** (decision: **defer**; do not admit quinn; do not admit noq).
  - CSCP-07.02 **complete as deferred**: ALPN `cscp/1` is reserved in `draft-webcivics-cscp-00` §6 and §9 and is **not implemented**. No ALPN glue added.
  - CSCP-07.03 **complete as confirmation**: `noq_transport_admitted()` is still `false`. Function not modified.
  - Parent CSCP-07 is a decision package. Integrator records registry status after review. This worker does not tick `workstream.md` or `task-registry.json`.
- Changed files, public behavior and integration source state:
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/decisions/CSCP-07-quic-alpn.md` (new)
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/handoffs/CSCP-07.md` (this file)
  - No `.rs`, `Cargo.toml`, honesty-flag, or runtime behavior change. Branch `0.0.38` at evaluation HEAD `b96304919f68d408cf067fab814fd4f01fc54255`.
- Accepted predecessor/interface versions:
  - CSCP-05 local TLS WSS + QSession control plane (in-tree; not a QUIC carrier).
  - `draft-webcivics-cscp-00` transport binding: QUIC stream + ALPN `cscp/1` after Qualia peer session.
- Evidence manifest paths:
  - Decision record: `qdnf-imp/cscp-imp/decisions/CSCP-07-quic-alpn.md`
  - Live grep capture: `/opt/cursor/artifacts/cscp-07-quic-alpn-source-evidence.log`
- Checks executed with outcomes; checks not executed and why:
  - Executed: workspace `Cargo.toml` grep for `quinn|noq|iroh` → no matches.
  - Executed: `Cargo.lock` inventory → `quinn` 0.11.11 transitive via `libp2p-quic` 0.13.1 and `reqwest` 0.12.28 / 0.13.4; no `noq` or `iroh` packages.
  - Executed: crate source grep for `quinn::` / `use quinn` / ALPN `cscp/1` → no matches.
  - Executed: quoted `noq_transport_admitted()` from `crates/qualia-core-db/src/net/peer/connectivity/evidence.rs` (returns `false`; test asserts `assert!(!noq_transport_admitted())`).
  - Not executed: `cargo test` of a QUIC path (none exists). No loopback QUIC handshake. No Internet trial. No dependency add.
- Measured resource bounds, enforcement limitations and error behavior:
  - No new QUIC buffers, stream limits, or adapter budgets. Deferral introduces no failure surface beyond the existing local CSCP carriers.
- Review findings, fixes and independent reviewer:
  - Author is swarm-cscp-07. Independent review is CSCP-11 (Wave 1 control plane) and does not admit a QUIC engine. No code fix was required.
- Provisional assumptions removed or still blocking acceptance:
  - Removed: “lockfile quinn equals an in-tree CSCP engine.”
  - Still blocking a future admit: reviewed engine revision, Qualia-owned adapter, measured loopback admission, resource contract, integrator Cargo/flag edits.

Quoted confirmation (CSCP-07.03), unchanged:

```rust
/// noq has not been admitted as the Internet QUIC engine.
pub const fn noq_transport_admitted() -> bool {
    false
}
```

Source: `crates/qualia-core-db/src/net/peer/connectivity/evidence.rs`.

## Integration

- Shared exports/Cargo/profile/registry edits requested from their owner:
  - **None.** Do not add quinn/noq/iroh. Do not set `noq_transport_admitted()` true. Integrator may mark CSCP-07 children complete in the registry after reading the decision record.
- Consumer packages and exact interface migration needed:
  - None. CSCP-08 remains blocked on a public operator URL even after this evaluation.
- Merge/conflict concerns and concurrent changes preserved:
  - Write set is disjoint from CSCP-09 (`capsule.rs`), CSCP-10 (browser decision), and CSCP-11 (Wave 1 review). No `mod.rs` touch.
- Rollback/recovery and durable schema compatibility:
  - Documentation-only. Revert the two markdown files if rejected. No durable schema change.
- Outstanding ownership, temporary artifacts and cleanup:
  - Artifact log under `/opt/cursor/artifacts/` is session evidence, not in-tree source. No TempDir leftovers.
- Next owner, next action and proposed registry transition:
  - Integrator: accept deferral; record CSCP-07.01–07.03 complete-as-deferred; leave `noq_transport_admitted` false.
  - Do not start QUIC implementation from this handoff.
  - Reopen only against the reconsideration triggers in the decision record (reviewed revision + measured loopback + resource/security review).
