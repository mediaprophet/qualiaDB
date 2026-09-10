# NAT traversal for SocialWebNet — expert brief

**Date:** 2026-09-10  
**Audience:** networking expert asked to direct the implementation.  
**Status:** recommendation recorded; in-process outbound relay proven; **no public relay operated yet**; internet two-host handshake **not** executed.  
**Related:** [internet-two-host.md](./internet-two-host.md) (measured SNAT on this Cursor cloud pod).

This is not Native Independent Ethernet. It is the labelled **transition** path (WireGuard / ICE / outbound relay / optional WebRTC). Programme checkboxes stay unchecked.

## 1. The misconception

WireGuard does **not** get through arbitrary SNAT or missing port-forwards.

WireGuard is Noise_IK over **UDP datagrams**. It authenticates peers, encrypts payload, and **roams** (the authenticated source address becomes the new endpoint). Roaming helps when a mapping already exists. It does not create a mapping a third host can hit.

What people remember as “WireGuard just works through NAT” is almost always **Tailscale**: WireGuard **plus** ICE-style candidate checks **plus** **DERP** (both peers make *outbound* HTTPS to a relay; the relay forwards opaque WG packets). WebRTC is the same pattern with different brand names: ICE + STUN + **TURN**.

Raw `wg` / boringtun on a socket has none of that. This repo’s `WgTunnel` is that raw path. It cannot violate RFC 4787.

## 2. NAT physics (measured here)

This Cursor cloud agent, 2026-09-10, same local UDP socket, two STUN servers:

| Destination | XOR-MAPPED-ADDRESS |
|---|---|
| `stun.l.google.com:3478` | `54.235.250.208:7013` |
| `stun.cloudflare.com:3478` | `54.235.254.240:52034` |

That is **address-dependent mapping** (ICE “symmetric NAT”). A STUN-learned `ip:port` is bound to *that STUN server*, not to grok-bot.

Consequences:

- Unsolicited inbound UDP to a STUN mapping **does not arrive**.
- UDP hole punch using that mapping **sends the peer to the wrong SNAT flow**.
- Direct WireGuard works only if the **other** side has a reachable UDP locator **or** this side initiates toward a locator that already exists (outbound flow; return traffic follows SNAT).
- If **both** sides are address-dependent and neither has a port-forward, **no amount of WireGuard configuration creates a path**. A third party both can *dial outbound* is required.

Cursor `environment.json` `ports: [4242]` is TCP forwarded to the operator’s localhost. It is not a public UDP 51820. WebRTC in `webizen-desktop` today uses `RTCConfiguration { ..Default::default() }` — **no ICE servers, no TURN** — so it has the same hole on this NAT class.

This pod: outbound UDP and outbound TCP 443 both work. That is enough to **dial a relay**. It is not enough to **be** a relay.

## 3. Recommended solution (this is the definition)

Three layers, raced, fail closed to the next. All labelled transition. QSession/ML-DSA remains the application authentication; the carrier is not identity.

```text
  [1] Direct WireGuard UDP     if a candidate is reachable (host / srflx / port-forward)
         ↓ miss
  [2] ICE connectivity checks  STUN on the WG socket; pair-check; short timeout
         ↓ miss (address-dependent / both-NAT)
  [3] Outbound packet relay    both peers DIAL OUT (HTTPS/WSS or TCP)
                               relay forwards opaque WireGuard datagrams
                               (DERP / TURN shape; not libp2p circuit-relay)
```

**Do not** make WebRTC the SocialWebNet default. WebRTC is the **browser profile** of layer 2+3 (ICE + DTLS datachannel, TURN as the relay). Native mesh should keep boringtun and a small relay frame, not drag `webrtc-rs` into `qualia-core-db`. Desktop already has a `webrtc` crate for HCAI-ANP; that path still needs TURN servers in `RTCConfiguration`.

**Do not** wrap QFrames as raw inner WG packets (boringtun drops non-IPv6). Inner overlay stays IPv6/UDP; QDNF on port `6423`; chat on `6420`.

**Do not** treat the relay as Native Independent, as a signed RAR, or as a substitute for QSession.

### Why this and not “just WebRTC”

| Option | Gets through this SNAT? | Fits SocialWebNet | Cost / sovereignty |
|---|---|---|---|
| Raw WireGuard UDP | No, unless one side is reachable | Yes (already built) | None |
| STUN + hole punch | No on address-dependent mapping | Small | Public STUN (privacy leak of mapping) |
| ICE + **TURN** (coturn / Twilio / Cloudflare Calls) | Yes | WebRTC-shaped; TURN credentials; DTLS extra | Vendor or you operate coturn |
| **WG packets over outbound HTTPS/WSS relay (DERP)** | Yes (both sides outbound 443) | Keeps boringtun; matches WASM “relay not this path” comment | You (or a commons) operate relays |
| QFrames directly on WSS, skip WG | Yes | Drops SocialWebNet outer crypto on that path; QSession can still bind | Simpler relay, weaker outer |
| libp2p holepunch / circuit-relay | Maybe | Explicitly out of the native replacement | libp2p back in the graph |
| Nym mixnet (`RendezvousHint` kind `nym`) | Possibly | Already a hint kind; high latency | Mixnet availability |
| IPv6 end-to-end | If both have global v6 | This pod has **link-local only** | ISP dependent |

**Pick:** layer 1+2 opportunistic, layer 3 **DERP-shaped outbound relay carrying WireGuard datagrams**, WebRTC as the browser adapter using the **same** relay as TURN or as a WSS fallback. That is the industry solution that matches the SocialWebNet profile (keep WG) and this NAT (must dial out).

## 4. What must exist that we cannot mint inside this pod

These are the expert / operator decisions. Implementation is waiting on them, not on more STUN.

1. **Who operates the public relay?**  
   Options: Qualia-operated VPS; volunteer commons; Cloudflare/Tailscale (lock-in); coturn with long-term credentials.  
   This Cursor cloud VM **cannot** be that host (same SNAT). A second cloud agent cannot be the other Ethernet NIC and cannot be the relay either unless it has a reachable inbound address.

2. **Relay transport:** HTTPS/WSS on 443 (best through captive portals and this pod) vs raw TCP vs UDP TURN. Recommendation: **WSS/443** as the guaranteed path.

3. **Signalling** (how two peers learn each other’s WG public keys and relay IDs before the first packet):  
   existing `qcx1_` connection identifier + `RendezvousHint` kinds (`relay`, `mailbox`, `nym`, `edge`) vs a live HTTPS POST vs human paste vs HCAI-ANP SDP.  
   Need one live channel. Human paste already works; it does not scale.

4. **Double encryption:** WG inside WSS (recommended, keeps `Tunn` unchanged) vs QFrames only on WSS (simpler, drops WG on the relayed path).

5. **WebRTC scope:** browser/HCAI-ANP only (recommended) vs also native mesh. If native WebRTC, `RTCConfiguration` must include TURN URIs; today’s desktop default config does not.

6. **STUN privacy:** using Google/Cloudflare STUN discloses that this host exists to those operators. Acceptable for transition? Or only self-hosted STUN?

7. **Billing / abuse:** a public relay is a bandwidth and amplification surface. Need admission (WG public key, short-lived token, rate limit). Fail closed.

Until (1) has a URL this agent can **dial outbound**, the internet two-host test between Cursor cloud and grok-bot **cannot complete** if grok-bot also lacks a reachable UDP listen. If grok-bot *does* listen on a public/port-forwarded UDP address, layer 1 already works (`connect_probe`) and does not need the relay.

## 5. In-tree proof (done) vs internet proof (not done)

| Evidence | Status |
|---|---|
| Userspace WG handshake, no sockets | `wireguard_userspace` tests |
| WG over real loopback UDP | `WgTunnel` / `SocialWebNet` tests |
| QDNF QFrames inside WG overlay | `social_qdnf` tests |
| STUN mapping class | `stun_observe`; this pod `AddressDependent` |
| **Both peers outbound-only, WG datagrams forwarded by a hub** | `p2p/outbound_relay.rs` (in-process; simulates two SNATs) |
| Public relay / TURN / WSS | **not deployed** |
| Cursor cloud ↔ grok-bot internet handshake | **not executed** |
| Native Independent / physical Ethernet | **false** (different problem) |

`internet_two_host_handshake_executed()` remains false. The in-process relay is not an internet test.

## 6. Expert: please answer these and nothing else

A. Confirm or reject §3 (WG + ICE race + outbound DERP-shaped relay; WebRTC = browser profile).  
B. Name the relay operator and the first URL/port this cloud agent may dial (WSS preferred).  
C. Choose signalling for the first two-host run: paste / `qcx1_` mailbox / HTTPS rendezvous.  
D. Confirm TURN URIs for desktop WebRTC if browsers must join the same mesh.  
E. Explicitly reject libp2p circuit-relay unless you want it back in the default graph.

With A–C, implementation of a real dialer against that URL is straightforward. Without a dialable relay or a reachable grok-bot UDP listen, no software change on this pod can complete the internet test.
