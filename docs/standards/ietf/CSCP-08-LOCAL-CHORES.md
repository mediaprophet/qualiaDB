# CSCP-08 — local / grok-bot chores

**Status:** blocked on a human-supplied inbound URL. This file is directions, not a completion claim.

CSCP-08 is **MASQUE bound-UDP on the public Internet**. A cloud agent cannot be that relay. GitHub Pages, CloudFront, IPFS, a chain, and a ledger are not Gate B.

Paste the block in §1 to grok-bot on `C:\Projects\qualia-27062026` (canonical tree). Or run the same steps yourself.

---

## 1. Paste this to grok-bot

```text
Work in C:\Projects\qualia-27062026 only. Branch 0.0.38. Do not create worktrees.

Task: CSCP-08 chores. Read docs/standards/ietf/CSCP-08-LOCAL-CHORES.md and
docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/briefs/CSCP-08.md
before writing code.

STOP until I give you a hostname or URL I already control. Do not invent
wss://…, MASQUE, or /dns4/ locators. Do not tick CSCP-08, parent CSCP-09, or
CSCP-12. Do not set public_relay_dialed() or masque_bound_udp_internet_executed()
true. Those are pub const fn in
crates/qualia-core-db/src/net/peer/connectivity/evidence.rs and they stay false
until a second network (not same-LAN loopback) actually dials a URL I supplied.

CSCP-12 (IETF datatracker) is a different chore. Do not conflate it with CSCP-08.

If I have not given a URL yet: ask me for (1) DNS name or VPS that can take
inbound TCP/443 (or UDP/443 for later MASQUE), (2) who is the named relay
operator, (3) whether home NAT can accept inbound vs use an existing VPS,
(4) certificate (Let’s Encrypt name I already have, or rustls with that name).
Then stop.

If I have given a URL: bind inbound WSS/443 using the existing
p2p/connectivity/wss_tls.rs and cscp_wss.rs patterns. Label it an Internet WSS
relay trial, not MASQUE. MASQUE is RFC 9298 CONNECT-UDP / HTTP/3. In-tree
cscp_h2.rs is 127.0.0.1 only. QUIC ALPN cscp/1 is deferred (CSCP-07); there is
no admitted QUIC engine. Do not fake MASQUE with rustls WSS or loopback H2.

Hand back: the URL I supplied (quote me), bind address, SNI, whether a second
host from off-LAN connected, source IPs, which honesty flag if any is justified
(almost certainly none until two-host Internet evidence), and a progress-log
entry that does not tick CSCP-08.
```

---

## 2. What only Timothy can supply

Ask. Do not invent.

1. A DNS name already controlled, or a VPS, that can accept **inbound** sessions: TCP/443 (WSS or HTTP/2) or later UDP/443 (MASQUE HTTP/3).
2. Named relay **operator** (person or commons), not an anonymous cloud pod.
3. Home NAT: port-forward / IPv6 inbound, or use an existing VPS. A firewalled home PC with no inbound is not Gate B.
4. Certificate identity the client will actually verify (name he already has). Self-signed `localhost` is the local WSS test, not CSCP-08.
5. Whether a second host exists for the trial (phone hotspot, second VPS, friend network). Same-machine `127.0.0.1` is already CSCP-05 / CSCP-09.02.

Datatracker posting is **CSCP-12**, using his IETF login. Author already recorded: Timothy Charles Holborn `<timothy.holborn@gmail.com>`. Individual draft. WG unchosen. Expires 14 March 2027. Canonical text: [`draft-webcivics-cscp-00.md`](./draft-webcivics-cscp-00.md).

---

## 3. Forbidden

- Inventing `wss://…` / MASQUE / `/dns4/` URLs.
- Setting `public_relay_dialed` or `masque_bound_udp_internet_executed` true without a **second network** (not same LAN loopback) dialing the human-supplied URL.
- Calling rustls WSS “MASQUE”. In-tree `cscp_wss` / `wss_tls` is TLS WebSocket. MASQUE = RFC 9298 CONNECT-UDP over HTTP/3.
- Calling loopback `cscp_h2.rs` Internet or MASQUE. That fixture binds `127.0.0.1`.
- Implementing QUIC / ALPN `cscp/1` as a shortcut. CSCP-07 **deferred**; `noq_transport_admitted()` stays false.
- Ticking CSCP-08, parent CSCP-09, or CSCP-12 in `cscp-imp/workstream.md`.
- Using GitHub Pages, CloudFront, IPFS, or a chain as the inbound process.
- Marking original 30 QDNF enhancement-plan checkboxes.

---

## 4. Honest stepping stones (after a URL exists)

| Step | What | Honesty |
|---|---|---|
| 1 | Bind inbound WSS/443 on the supplied name, reuse `wss_tls` / `cscp_wss` | Not MASQUE. Not `public_relay_dialed`. |
| 2 | Second host off-LAN dials that URL; CSCP ConnectRequest/Accept then QSession Active | May justify `public_relay_dialed()` **only** with recorded source IPs, SNI, and the URL he supplied. Still not MASQUE. |
| 3 | Two distinct Internet hosts, not a NAT hairpin to the same LAN | May justify `internet_two_host_handshake_executed()`. Still not MASQUE. |
| 4 | HTTP/3 CONNECT-UDP (RFC 9298) through an admitted QUIC stack | Only then consider `masque_bound_udp_internet_executed()`. Needs CSCP-07 revisit. |

Windows notes for grok-bot / local:

- Canonical tree is `C:\Projects\qualia-27062026`. Commit and push from there.
- Binding TCP 443 usually needs an elevated shell and a firewall allow.
- Isolated cargo: `CARGO_TARGET_DIR` local; `--offline` for `qualia-core-db` lib tests. Do **not** run filters matching `identity::`.
- `qualia-cli` / `qualia-client-core` may need OpenSSL on some hosts; CSCP-08 does not require them.

---

## 5. In-tree facts (do not regress)

- Local WSS + QSession: `crates/qualia-core-db/src/p2p/connectivity/cscp_wss.rs` (loopback).
- Local HTTP/2 Extended CONNECT + capsules: `cscp_h2.rs` / `h2_capsule.rs` (`127.0.0.1`).
- Capsule framing: `net/peer/fabric/capsule.rs` (not HTTP/2, not MASQUE Internet).
- Honesty flags: `net/peer/connectivity/evidence.rs` — `public_relay_dialed`, `internet_two_host_handshake_executed`, `masque_bound_udp_internet_executed`, `noq_transport_admitted`, `quic_iroh_benchmark_executed`, `browser_turn_interop_executed` are `const fn` returning **false**.
- Spec §6: MASQUE bound UDP is the preferred Internet profile when the proxy actually implements bound UDP. Ordinary HTTP/3 is not sufficient.

---

## 6. What to hand back

Quote the URL Timothy supplied. Record bind address, SNI, certificate name, whether inbound from off-LAN succeeded, source IPs, logs. State which honesty flag if any is justified. Cloud agents still must not invent the URL. CSCP-08 stays unticked until step 4 evidence exists.
