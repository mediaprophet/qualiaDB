# Assignment CSCP-08 — MASQUE bound-UDP Internet (blocked)

- Parent: **CSCP-08**. Child IDs: none until a human-supplied inbound URL exists.
- Outcome: an Internet MASQUE (RFC 9298 CONNECT-UDP / HTTP/3) bound-UDP path that a second host actually dials. Not a docs-only claim.
- Explicit non-goals: do not invent a public URL; do not tick CSCP-08 / parent CSCP-09 / CSCP-12; do not set Internet honesty flags true without two-host evidence; do not admit quinn/noq/iroh (CSCP-07 deferred); do not treat loopback H2 or local rustls WSS as MASQUE; do not conflate CSCP-12 (datatracker) with this task.
- Agent/owner: grok-bot or Timothy on `C:\Projects\qualia-27062026`, branch `0.0.38`. Cloud agents cannot be the public relay.
- Source: live `0.0.38`. Canonical I-D: `docs/standards/ietf/draft-webcivics-cscp-00.md`. Operator directions: `docs/standards/ietf/CSCP-08-LOCAL-CHORES.md`.

## Gate

Do not write network-facing relay code until Timothy supplies:

1. DNS name or VPS that accepts inbound TCP/443 (WSS/H2) or UDP/443 (MASQUE HTTP/3).
2. Named operator.
3. Inbound path (home NAT port-forward / IPv6 vs existing VPS).
4. Certificate for that name.

If those are missing: ask once, then stop.

## Allowed writes (only after the URL is supplied)

Reuse existing patterns; do not add a second transport stack:

- `crates/qualia-core-db/src/p2p/connectivity/wss_tls.rs`
- `crates/qualia-core-db/src/p2p/connectivity/cscp_wss.rs` (today loopback-only)
- New sibling under `p2p/connectivity/` for an inbound listener **if** needed, under 500 lines
- `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/handoffs/CSCP-08.md` (create on completion attempt)
- Progress-log entry only via integrator / this owner after evidence

Forbidden: `evidence.rs` honesty flags flipped to true without measured Internet evidence; `Cargo.toml` QUIC deps; original 30-package checklists; inventing locators; GitHub Pages / CDN / IPFS / ledger as Gate B.

## Honest labels

| What you might build first | Call it | Not |
|---|---|---|
| Inbound WSS/443 on his name | Internet WSS relay trial | MASQUE, Native Independent, CSCP-08 complete |
| Off-LAN second host dials that WSS URL, CSCP then QSession | candidate for `public_relay_dialed()` | MASQUE |
| HTTP/3 CONNECT-UDP bound UDP through an admitted QUIC engine | CSCP-08 | anything currently in-tree |

`cscp_h2.rs` is `127.0.0.1` Extended CONNECT. Nested recovery is degraded. QUIC ALPN `cscp/1` is reserved in the draft and **not implemented**.

## Tests / evidence

After a real bind:

```text
CARGO_TARGET_DIR=<local>
cargo test -p qualia-core-db --lib --offline p2p::connectivity::cscp_wss p2p::connectivity::cscp_h2 net::peer::fabric -- --test-threads=1
```

Do not match `identity::`. Record the human-supplied URL, SNI, source IPs of the off-LAN peer, and that honesty flags remain false unless step evidence in `CSCP-08-LOCAL-CHORES.md` §4 is met.

Handoff using `qdnf-imp/templates/handoff.md`. Do not tick `workstream.md`.
