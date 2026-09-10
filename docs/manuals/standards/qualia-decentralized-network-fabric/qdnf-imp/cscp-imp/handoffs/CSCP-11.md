# Handoff — CSCP-11 independent review of Wave 1

## Result

- Package and child IDs: **CSCP-11** review child **done** for this assignment. Parent CSCP-11 / workstream box **must not be ticked by this worker**. Integrator records status after reading the review. Wave 2 Internet remains out of scope.
- Changed files (allowed writes only; no `.rs`, no workstream ticks, no honesty flags):
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/reviews/CSCP-11-wave1.md`
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/handoffs/CSCP-11.md`
- Public behavior: **unchanged**. Reviewer did not author Wave 1 and did not patch implementation.
- Accepted predecessor: CSCP-01–06 in-tree on `0.0.38` at `4cb9f9b1aeb8c733785324550126164ac7fececf`. Draft: `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/draft-webcivics-cscp-00.md`.
- Evidence: the review file. Independent test re-run (not the author log):
  - `CARGO_TARGET_DIR=/tmp/qdnf-continue-target cargo test -p qualia-core-db --lib --offline net::peer::fabric -- --test-threads=1` → **34 passed**, 0 failed.
  - `cargo test -p qualia-core-db --lib --offline p2p::connectivity::cscp_wss -- --test-threads=1` → **1 passed**, 0 failed, 0.26s.
  - rustc 1.98.1, Linux x86_64, `--offline`.
- Checks executed: full read of `fabric/` (including `wire/`, `mailbox.rs`, `lease_protocol.rs`, `outcome.rs`, `receipt_store.rs`, `kernel.rs`, `select.rs`) and `p2p/connectivity/cscp_wss.rs` plus `wss_tls.rs` rustls binding and `connectivity/evidence.rs` const flags.
- Checks not executed: Internet two-host, MASQUE, QUIC ALPN, browser origin, datatracker. Out of scope.
- Resource bounds: no new runtime bounds. Review notes `receipt_store.rs` `Vec` as cold persist; encode/decode/kernel/mailbox hot paths have no `Vec`/`String`/`Box`.
- Review findings: **verdict = accept-with-fixes** for Wave 1 local completeness. Must-fix: F1 missing/wrong-length required critical TLVs fail open; F2 `remaining_bytes > max_bytes` accepted as forwarding budget. Should-fix: F3 `SessionReady` skips disclosure; F4 v1 flags/reserved ignored. Residual: kernel does not call `exclude_then_rank`; mailbox re-encodes stored Direct locators; WSS mints Relayed witness without a relay; `Descriptor.stale` is caller-supplied.
- Provisional assumptions: none added. Removed assumption: that author “34+1 passed” alone proved fail-closed decode. Reproduced tests do not cover F1/F2.

## Integration

- Shared exports / Cargo / honesty flags / `workstream.md` / `task-registry.json`: **none requested**. Do not set `independent_protocol_review_executed` from this worker. Integrator may set that const only after deciding the review is accepted and if that is the intended meaning of the flag (it currently stays `false`).
- Consumer packages: none. CSCP-07/09/10/12 do not wait on a clean-accept upgrade; they must not treat Wave 1 as Internet-ready.
- Merge/conflict: only the two markdown paths above. Concurrent untracked `fabric/capsule.rs` (CSCP-09) and CSCP-07/10 docs were **not modified**.
- Rollback: delete the two markdown files. No ABI, no durable schema.
- Outstanding ownership: integrator applies F1–F4 in `wire/` / `session.rs` / `lease_protocol.rs` (implementation is the integrator’s lane). This reviewer must not edit those files.
- Next owner: **integrator**. Next action: apply F1+F2 (required) and F3/F4 (recommended); add empty-body and `remaining > max` tests; re-run the two cargo commands; then optionally request a follow-up review pass to upgrade to **accept**. Proposed registry transition: CSCP-11 evidence = this review; status remains integrator-owned. **Do not mark Wave 1 Internet-complete. Do not tick CSCP-08/12.**

Honesty: **Wave 1 local control plane holds the disclosure and demotion invariants; the TLV decoder is not yet fail-closed on missing required fields.**
