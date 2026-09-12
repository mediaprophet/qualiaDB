# CSCP-10 — Browser WebTransport / WSS profile

- Decision ID: CSCP-10
- Date: 2026-09-10
- Status: recorded profile; **not implemented**; **not Gate E qualified**
- Owner: swarm-cscp-10 (this note). Integrator owns `.rs` flags, `mod.rs`, and registry ticks.
- Affected tasks: CSCP-10 (parent); depends on CSCP-05 local TLS WSS. Does not unblock CSCP-08 (MASQUE Internet) or CSCP-12 (datatracker).
- Source baseline: branch `0.0.38`, HEAD `b96304919f68d408cf067fab814fd4f01fc54255`. Read-only inspection of `cscp_wss.rs`, `wss_tls.rs`, `browser.rs`, `evidence.rs`, `carrier.rs`, `draft-webcivics-cscp-00.md`, `capability-scoped-connection-fabric.md` Gate E.

## Semantic requirement

Draft CSCP path class **4 Browser** (`PathClass::BrowserGateway`) and `CarrierKind::Browser` are a **distinct profile**. Architecture Gate E (qualification) requires dependency review, interoperability, fuzz, resource and field tests with reproducible artifacts and independent review. A local rustls WebSocket pair is not that evidence.

This pass records the profile. It does **not** add wasm UI, dial TURN, mint a page origin, or set `browser_turn_interop_executed`.

## What local TLS WSS already proves (CSCP-05)

`p2p/connectivity/cscp_wss.rs` (`#![cfg(not(target_arch = "wasm32"))`]) runs two loopback rustls WSS peers:

1. `mint_ca()` + `loopback_tls_wss(&ca)` — client verifies the server certificate against a **pinned process-local CA**. Wrong CA is rejected (`wss_tls.rs::wrong_ca_is_rejected`). Plain TCP is labelled as not TLS.
2. CSCP `ConnectRequest` / `ConnectAccept` on RFC 6455 binary datagrams (`wss.rs` envelope version 2, **u16** length). Path class on that Accept is `PathClass::Relayed`, under `ProtectionPolicy::RELAY_ONLY`. Public DHT stays off.
3. `WssBearer` then `handshake_over_fragments` then `SessionBinding::from_permit` + `admit_application` → `SessionState::Active`.
4. The CSCP-05 test asserts `!public_relay_dialed()` and `!masque_bound_udp_internet_executed()`.

`local_tls_wss_verified()` is `true` because that pinned-CA loopback path exists. That flag is **not** a browser flag.

What it does **not** prove:

- A Secure Context page origin the browser Same-Origin / CORS rules will accept.
- A certificate in a **browser trust store** (the local CA is never installed in Chromium/Firefox/WebKit).
- W3C WebTransport (`WebTransport` streams/datagrams to a server).
- WebRTC ICE/TURN against a live TURN URI.
- Path class 4 (`BrowserGateway`) on the wire from a real browser.
- Public WSS/443, Internet two-host, or MASQUE.

**Local rustls WSS ≠ browser WebTransport interop.** Treating CSCP-05 as CSCP-10 would be a false qualification.

## What WebTransport would add

From `draft-webcivics-cscp-00.md` §4.5 / §6, `capability-scoped-connection-fabric.md` §4–5, and `quic-native-connectivity-research-2026.md` §7:

| Already proven (native loopback WSS) | WebTransport / browser profile would add |
|---|---|
| rustls + RFC 6455 to `127.0.0.1`, process-local CA | Browser `WebTransport` (HTTP/3) to an **approved gateway**, with TLS-protected WebSocket/HTTPS as a **separately tested** fallback |
| `PathClass::Relayed` (wire 3) | `PathClass::BrowserGateway` (wire 4). Not native UDP NAT probing |
| QSession over `WssBearer` in-process | Same QSession requirement; gateway **must not** hold QSession keys (`BrowserProfile.gateway_holds_qsession_keys` is already `false`) |
| No origin | Origin-scoped Secure Context; certificate SAN matching the **human-supplied** host |
| Native `TcpStream` + rustls | Browser networking APIs; no arbitrary UDP sockets; no MASQUE `CONNECT-UDP` via ordinary `fetch` |

Existing `net/peer/connectivity/browser.rs` is a **policy contract**, not a trial:

- `BrowserBearer::{WebrtcDataChannel, WssFallback}` — **no `WebTransport` variant**.
- `BrowserProfile::RELAY_ONLY` uses WebRTC DataChannel + ICE relay policy.
- `turn_is_not_wireguard()` is a const true: TURN cannot convert DTLS/SCTP into WireGuard.

WebRTC ICE/TURN remains a **different** browser bearer. An ICE/TURN-free native fabric does not remove that obligation if browser-to-browser WebRTC is required. This note does not dial TURN.

Isolated disclosure still forbids all live-network classes, including `BrowserGateway`. Relay-only forbids DirectV6/V4; BrowserGateway does not disclose peer IP to the counterpart, but the **gateway still sees the client source**. Two operators are not anonymity.

## Why a public browser trial is blocked here

1. **No page origin.** Gate E field/interop tests need a human-supplied HTTPS origin (scheme + host [+ port]) the browser will treat as a Secure Context, plus a certificate that origin’s browsers actually trust. This programme must not invent a public URL (same rule as CSCP-08).
2. **This pod is not a public relay.** Loopback rustls cannot be reached by a user’s browser on another machine; SNAT/cloud networking does not make `127.0.0.1` a field endpoint.
3. **Honesty flag stays false.** In `evidence.rs`:

```rust
/// Browser TURN/WebRTC interop against a live TURN URI has not been executed.
pub const fn browser_turn_interop_executed() -> bool {
    false
}
```

A WebTransport origin trial is also unexecuted. Neither CSCP-05 nor this markdown sets that flag true.

4. **Gate E is not satisfied by loopback.** Architecture: local fabric tests cover a kernel/lease slice of Gates A and D on loopback. They **do not** satisfy B, C, or E on the public Internet. Browser interop is Gate E work.
5. **Non-goals of this assignment:** no wasm UI, no TURN dial, no `.rs` edits.

Human input required before any implementation file is written: page origin, operator of the gateway host, browser-trusted certificate (or an explicit lab trust-store procedure), and whether the first trial is WebTransport, WSS fallback, or WebRTC/TURN. Until those exist, CSCP-10 remains a profile note.

## Options

1. **Claim CSCP-05 as browser interop.** Rejected. Local WSS is Relayed class, native rustls, no origin, no browser engine.
2. **Add wasm UI / headless Chromium in this pass.** Rejected. Brief forbids wasm UI; still needs an origin and a trusted cert; would not make `browser_turn_interop_executed` true without a live trial.
3. **Record the profile; defer implementation until a human supplies a page origin.** **Accepted.** Zero-dependency local note is enough for this pass.

## Decision

- CSCP-10 in this pass is a **profile decision**, not a carrier implementation.
- CSCP control on local TLS WSS + QSession remains CSCP-05 evidence only.
- Path class 4 / `CarrierKind::Browser` stay specified, unused by a browser.
- `browser_turn_interop_executed()` remains `false`. Do not tick CSCP-10 as field-complete.
- Rejected assumption: “rustls WSS tests imply WebTransport.”

## Next implementation file (blocked on origin)

Once a human supplies a page origin (and the gateway host/cert that origin will accept), the next **new** implementation file is:

`crates/qualia-core-db/src/p2p/connectivity/cscp_browser.rs`

That file is the CSCP-05 analogue: CSCP ConnectRequest/Accept then QSession on a **browser-origin** WebTransport or WSS session to an approved gateway, recording `PathClass::BrowserGateway` from **local** transport observation of that gateway path — not by relabelling loopback Relayed evidence. Integrator adds `pub mod cscp_browser` in `p2p/connectivity/mod.rs` after review. Do not put CSCP-10 into `cscp_wss.rs` (that file is native loopback Relayed).

`net/peer/connectivity/browser.rs` stays the policy contract: it must gain an explicit `WebTransport` bearer and a caller-supplied origin binding **from the human input**, not a minted URL. Do not set `gateway_holds_qsession_keys`. Do not add wasm UI in that first file; the page at the supplied origin is a separate, later assignment. Do not set `browser_turn_interop_executed` until a real browser engine has completed the trial against that origin (and, if the TURN bearer is in scope, against a live TURN URI).

## Limits

- Memory/work: no new hot path in this pass. A future `cscp_browser.rs` reuses CSCP caller buffers and QSession; browser engine buffers are outside the 42 MiB Sentinel and need an explicit resource contract when admitted.
- Authority: CSCP still requires Qualia peer session. A self-signed transport cert without identity binding is not a CSCP peer (`draft-webcivics-cscp-00.md` §7). Browser trust of TLS ≠ QSession admit.
- Privacy: relay-only / browser-gateway still exposes client source to the gateway. Do not export access-network addresses in candidate TLVs.
- Compatibility: WireGuard/ICE/TURN/WSS remains the native compatibility profile; browser WebTransport/WSS is not a substitute for native UDP NAT probing.
- Consumers: none until `cscp_browser.rs` exists. No Cargo or honesty-flag migration.
- Independent review: CSCP-11 covers Wave 1 (CSCP-01–06), not this unimplemented browser path. Gate E review is still outstanding.

## Verification and reconsideration

- Verification this pass: source read; `browser_turn_interop_executed` is `false`; `cscp_wss` test does not touch that flag. No new cargo run required for a markdown profile.
- Reconsider when: (1) human supplies page origin + trusted cert + gateway operator; (2) `cscp_browser.rs` exists and a real browser trial is recorded; (3) only then may the integrator consider the honesty flag — and only if the executed trial matches what the flag’s comment states (live TURN/WebRTC and/or an updated flag whose comment matches the WebTransport origin trial). Do not stretch the current TURN-worded flag to cover WSS loopback.
