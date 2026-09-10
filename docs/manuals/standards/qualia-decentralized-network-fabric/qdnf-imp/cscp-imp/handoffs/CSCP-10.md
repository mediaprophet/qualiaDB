# Handoff — CSCP-10 browser profile

## Result

- Package and child IDs: **CSCP-10** profile note **done** for this assignment. Parent CSCP-10 **browser implementation / Gate E interop remains pending**. Do not tick `workstream.md` CSCP-10 as field-complete. Child implementation (`cscp_browser.rs`) is blocked on a human-supplied page origin.
- Changed files (allowed writes only; no `.rs`):
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/decisions/CSCP-10-browser-profile.md`
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/handoffs/CSCP-10.md`
- Public behavior: **unchanged**. No wasm UI, no TURN dial, no honesty-flag edit. `browser_turn_interop_executed()` remains `false` in `crates/qualia-core-db/src/net/peer/connectivity/evidence.rs`.
- Accepted predecessor: CSCP-05 local rustls WSS + QSession in `p2p/connectivity/cscp_wss.rs` (loopback Relayed class). That evidence is **not** reused as browser interop.
- Evidence: the decision record itself. Live source quotes:
  - `cscp_wss.rs`: `cscp_then_qsession_over_local_tls_wss`; test asserts Active + `!public_relay_dialed()` + `!masque_bound_udp_internet_executed()`.
  - `evidence.rs`: `browser_turn_interop_executed() -> bool { false }`.
  - `capability-scoped-connection-fabric.md` §6 Gate E: interop/fuzz/field tests + independent review; local tests do not satisfy B, C, or E on the public Internet.
  - Draft path class 4 Browser; architecture: “Browser WebTransport / WSS — Distinct profile; not native UDP NAT probing.”
- Checks executed: source read on branch `0.0.38` at `b96304919f68d408cf067fab814fd4f01fc54255`. **No cargo test run** — this assignment forbids `.rs` edits and is a zero-dependency note. CSCP-05 WSS tests were not re-executed here; Wave 1 log already recorded `p2p::connectivity::cscp_wss` 1 passed.
- Checks not executed: browser engine trial, WebTransport handshake, live TURN, public WSS/443, wasm UI. Blocked: no page origin, pod is not a public relay, local CA is not in a browser trust store.
- Resource bounds: none added. Future browser engine buffers need an explicit contract; they are not the 42 MiB Sentinel hot path.
- Review: not self-approved as Gate E. Independent reviewer is CSCP-11 for Wave 1 only; browser field tests remain Gate E outstanding.
- Provisional assumptions still blocking acceptance of a **browser trial**: HTTPS page origin, gateway operator/host, browser-trusted certificate, choice of WebTransport vs WSS fallback vs WebRTC/TURN. Removed assumption: that CSCP-05 local WSS could stand in for CSCP-10.

## Integration

- Shared exports / Cargo / honesty flags: **none requested**. Do not set `browser_turn_interop_executed` true. Do not add `pub mod cscp_browser` until that file exists after origin supply.
- Consumer packages: none. `BrowserProfile` in `net/peer/connectivity/browser.rs` is unchanged (still no `WebTransport` bearer, no origin field).
- Merge/conflict: only the two markdown paths above. Concurrent CSCP-07/09/11 workers own disjoint files.
- Rollback: delete the two markdown files. No schema, no durable store, no ABI.
- Outstanding ownership: integrator may record this handoff in `cscp-imp/progress-log.md` / registry evidence; this worker must not edit those files. Temporary artifacts: none.
- Next owner: **human** supplies page origin (and cert/operator). **Next implementation file:** `crates/qualia-core-db/src/p2p/connectivity/cscp_browser.rs` (CSCP ConnectRequest/Accept + QSession on a browser-origin WebTransport/WSS gateway path, `PathClass::BrowserGateway`). Policy updates belong in `browser.rs` from that origin, not a minted URL. Integrator merges `mod.rs` after review.
- Proposed registry transition: CSCP-10 stays **not complete** for implementation. This child is “profile recorded / implementation blocked on origin.” Integrator verifies; do not mark parent complete.

Honesty: **local rustls WSS is not browser WebTransport interop.**
