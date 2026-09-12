# Assignment CSCP-10 — Browser profile decision

- Parent: CSCP-10. Outcome: a profile note for browser WebTransport/WSS. Not a browser implementation in this pass unless a zero-dependency local note is enough.
- Non-goals: do not add wasm UI; do not dial TURN; do not set `browser_turn_interop_executed`; do not edit `.rs` files.
- Allowed writes only:
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/decisions/CSCP-10-browser-profile.md`
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/handoffs/CSCP-10.md`

Read `cscp_wss.rs`, `draft-webcivics-cscp-00.md` browser/WSS paragraphs, `capability-scoped-connection-fabric.md` Gate E, and `evidence.rs` browser flag.

Record: what the local TLS WSS path already proves; what WebTransport would add; why a public browser trial is blocked here; recommended next implementation file once a human supplies a page origin. Keep honesty: local WSS ≠ browser interop.
