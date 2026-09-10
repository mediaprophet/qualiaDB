# Internet two-host SocialWebNet — design, measured barriers, and the job that unblocks it

**Date:** 2026-09-10  
**Status:** design + in-tree observe tool; **two-host internet handshake not executed**.  
**Path:** labelled WireGuard transition (`BearerProfile::WireGuardTransitionV1`). This is **not** Native Independent and **not** physical Ethernet.

This note exists so a human (or grok-bot on a reachable machine) can clear the few remaining barriers. Once those are cleared, this agent can run the **connect** half and record a real handshake. No programme checkbox is marked complete from this document.

## 1. What “state of the art” means here

Two hosts on the public Internet, each behind some NAT, want an authenticated tunnel, then QDNF QFrames inside it.

| Layer | Choice | Why |
|---|---|---|
| Outer carrier | Userspace WireGuard (boringtun), UDP | Already the SocialWebNet profile; roaming updates the peer endpoint from authenticated packets; no `CAP_NET_ADMIN`, no kernel `wg`. |
| Inner overlay | IPv6/UDP datagrams | boringtun drops non-IPv6 inner packets. Chat = port `6420`. QDNF QFrames = port `6423`. |
| Session (optional) | QSession `handshake_over_fragments` | Already proven on two **loopback** WireGuard meshes. Same bytes can ride the internet overlay once the outer tunnel is up. |
| NAT | RFC 5389 STUN Binding on **one** socket toward **two** servers | Classifies endpoint-independent vs address-dependent mapping (RFC 4787). A STUN address is an observation, not a listen locator. |
| Roles | One **listen** (reachable UDP), one **connect** (outbound) | Symmetric / address-dependent SNAT cannot publish a STUN mapping for a third host. Hole punching will not work from that side. |
| Relay | Not implemented | Needed only if **both** sides are address-dependent and neither has a port-forward or public UDP. That requires a host you control. This Cursor cloud VM cannot be that relay. |

What we will **not** do:

- Pretend WireGuard-over-IP is Native Independent Ethernet.
- Treat Cursor TCP port-forward (`environment.json` port `4242`) as a public WireGuard UDP endpoint.
- Claim a two-host internet test from loopback or STUN alone.
- Implement UDP hole punch as the primary path on an address-dependent NAT (it would fail and look like a product bug).

## 2. Architecture that will work

```text
grok-bot machine (LISTEN)                    Cursor cloud agent (CONNECT)
-------------------------                    ---------------------------
UDP 51820 reachable                          private 172.30.0.2
  · public IP, or                            address-dependent SNAT
  · home/VPS UDP port-forward                outbound UDP allowed
                                             STUN ≠ listen locator
        ◄──── WireGuard handshake (UDP) ────  initiate
        ◄──── inner IPv6 / QDNF port 6423 ──  DiscoveryBeacon
```

Sequence:

1. **Listen** side binds UDP `51820`, runs `mesh-probe listen` with a shared passphrase.
2. Operator pastes **one reachable** `host:port` (the port-forward or public IP — **not** a STUN mapping from an address-dependent NAT).
3. **Connect** side (this cloud agent) runs `mesh-probe connect --peer host:port --qdnf`.
4. Outbound handshake creates a SNAT flow; return packets follow that flow. WireGuard roaming locks the endpoint.
5. Connector sends a QDNF QFrame; listener prints `QDNF overlay`.

That is the internet test this programme can actually pass. Ethernet L2 between these two machines is a different test and remains blocked (see §5).

## 3. What this Cursor cloud VM measured (2026-09-10)

Runner: Linux `x86_64-unknown-linux-gnu`, `CapEff=0`, user `ubuntu`, `eth0` `172.30.0.2/24`, gateway `172.30.0.1`, DNS `10.0.0.2`. Egress policy: **unrestricted**. `ip` is not on PATH. No IPv6 global address (link-local only).

| Probe | Result |
|---|---|
| Outbound UDP | Works (`1.1.1.1:53` DNS answer; STUN Binding success). |
| Outbound TCP 443 | Works (`ifconfig.me` HTTP 200). |
| Same UDP socket, two STUN servers | **Address-dependent mapping.** Google `stun.l.google.com:3478` → `54.235.250.208:7013`. Cloudflare `stun.cloudflare.com:3478` → `54.235.254.240:52034`. Same local port `38073`. |
| New socket per STUN server | A **pool** of public IPs (`34.196.70.24`, `3.234.36.95`, `100.55.198.211`, …). No stable listener identity. |
| HTTPS `ifconfig.me` | Yet another pool address (`100.55.198.211` on a later sample). |
| NAT hairpin to own STUN mapping | Timed out. |
| Bind UDP `51820` locally | Succeeds. That does **not** make it reachable from the Internet. |
| Cursor `ports` | TCP `4242` (graph daemon) forwarded to the **operator desktop**, not a public UDP 51820. |
| Privileges | `CapEff=0`. Userspace WireGuard does not need net-admin. Raw Ethernet / veth / AF_PACKET still cannot open. |

**Conclusion for this VM:** it is **connect-only**. It can reach a grok-bot (or desktop, or VPS) that **listens** on a reachable UDP address. It cannot be the listen half of an internet test. UDP hole punch using a STUN address from this VM will send the peer to the wrong SNAT mapping.

`internet_two_host_handshake_executed()` stays **false** until step 3–5 in §2 actually complete against a remote listener.

## 4. Barriers you can clear (action list)

Give these to grok-bot / the desktop / a VPS. Each item is something this agent cannot mint from inside the pod.

### Barrier A — no reachable UDP listener (the blocker for the internet test)

**Need:** one `host:port` on the public Internet where UDP datagrams to that pair reach a `qualia-cli mesh-probe listen` process.

Ways to provide it (any one is enough):

1. **Grok-bot’s machine has a public IPv4** and UDP `51820` is allowed inbound. Run the listen command below and paste `PUBLIC_IP:51820`.
2. **Home/office NAT:** port-forward UDP `51820` to grok-bot’s LAN address. Paste the **WAN** IP and `51820`, not the LAN IP.
3. **Small VPS** (any cheap UDP-capable host) runs listen. Paste that VPS `ip:51820`.
4. **Cursor desktop on a reachable network** (not this cloud pod) runs listen; grok-bot or the cloud agent connects.

Ways that **do not** work:

- Pasting this cloud agent’s STUN mapped address (`54.235.x.x:port`). Address-dependent SNAT: grok-bot’s packets will not hit that flow.
- Cursor auto-forward of TCP `4242`. Different protocol, different audience (operator localhost).
- “Both sides STUN then punch” while this cloud VM is one side.

### Barrier B — shared passphrase and a live window

Listen and connect derive WireGuard keys from the same passphrase (role `a` = listen, role `b` = connect). Both processes must overlap in time (UDP is not queued at a server).

Pick a passphrase in the next message. Example shape (change it): `qdnf-inet-2026-09-10`.

Listen with a deadline so it does not sit forever:

```text
qualia-cli mesh-probe listen --pass 'THE_PHRASE' --port 51820 --seconds 300
```

### Barrier C — binary on the listen host

The listen host needs this branch’s `qualia-cli` (or at least a build that includes `mesh-probe observe/listen` and QDNF overlay port `6423`).

```text
git fetch origin cursor/qdnf-enhancement-e00-e01-cb60
git checkout cursor/qdnf-enhancement-e00-e01-cb60
cargo build -p qualia-cli --offline   # or with network if the host has no cargo cache
```

If grok-bot cannot build, say so: we can add a tiny standalone listen recipe, but someone still has to run a UDP process on a reachable address.

### Barrier D — tell this agent the locator (one line)

Paste back **exactly**:

```text
LISTEN_ADDR=203.0.113.5:51820
PASS=THE_PHRASE
OBSERVE=<paste mesh-probe observe output from the listen host>
```

Then this cloud agent can run:

```text
qualia-cli mesh-probe connect --pass 'THE_PHRASE' --peer 203.0.113.5:51820 --qdnf --timeout 20
```

and record whether the handshake completed. That is the missing evidence. Nothing else in this document substitutes for it.

### Barrier E — both sides address-dependent, no port-forward (relay)

If grok-bot **also** sits on address-dependent SNAT with no port-forward, neither side can listen. Then we need a **TURN/DERP-style relay** on a host you control (VPS with a public UDP or TCP port). This repo does not ship that relay. Do not ask the Cursor cloud pod to be it; it has the same SNAT problem.

If you stand up a relay, say so and we can bind an explicit “QFrames over relay” adapter. Until then, Barrier A is the cheaper fix.

### Barrier F — physical Ethernet (separate from internet)

Two-host **L2** (AF_PACKET, EtherType, veth, `physical_two_host_qualified`) still needs:

- `CAP_NET_RAW` / `CAP_NET_ADMIN` (this pod: `CapEff=0`)
- `ip` for veth/netns
- two NICs on the same broadcast domain

The internet WireGuard test **does not** clear those flags. Do not conflate them. Clearing Barrier A does not make Ethernet “done.”

## 5. Commands

### On the listen host (grok-bot / desktop / VPS)

```text
qualia-cli mesh-probe observe --port 51820
qualia-cli mesh-probe listen --pass 'THE_PHRASE' --port 51820 --seconds 300
```

`observe` prints mapping class. If it says `address-dependent`, you **must** still provide a real public/port-forwarded `host:port`; do not paste the `srflx_stun_*` lines as `LISTEN_ADDR`.

### On this Cursor cloud agent (after Barrier D)

```text
qualia-cli mesh-probe observe --port 51820
qualia-cli mesh-probe connect --pass 'THE_PHRASE' --peer LISTEN_ADDR --qdnf --timeout 20
```

Expect listen to print `QDNF overlay` with `magic_ok=true`.

### In-process (already green, not internet)

Two loopback WireGuard meshes already exchange QFrames and run `handshake_over_fragments`. That proves the overlay codec. It does not prove Internet SNAT.

## 6. Honesty flags

| Flag | Value until Barrier D succeeds |
|---|---|
| `uses_libp2p()` | false |
| `carrier_is_wireguard()` | true |
| `native_independent()` | false |
| `internet_two_host_handshake_executed()` | **false** |
| `physical_two_host_qualified()` | false |
| `silent_ip_fallback()` | false (this path is labelled IP/WG, not a silent Ethernet fallback) |

E05.5 / OPS-01 checkboxes stay unchecked.

## 7. What was implemented in-tree with this note

- `p2p/stun_observe.rs` — RFC 5389 XOR-MAPPED-ADDRESS parse/encode, two-server mapping class, connect-only role recommendation.
- `qualia-cli mesh-probe observe` — live STUN on a bound UDP socket.
- `mesh-probe listen` prints the same observe report on the tunnel socket.
- `mesh-probe connect --qdnf` sends a QFrame on overlay port `6423`.

These tools do not complete the internet test by themselves. They make Barrier D a one-line paste instead of a research project.

## 8. ⚑ What I need from you

One of:

1. **LISTEN_ADDR + PASS** from grok-bot (or any reachable listen host) while listen is running, or
2. Confirmation that grok-bot cannot listen (then we plan a relay on a VPS you name), or
3. A self-hosted Cursor worker on grok-bot’s machine **if and only if** that machine’s UDP `51820` is reachable; the worker still has to run `mesh-probe listen`.

I do not need Ethernet privileges to run the **internet** test. I do need Barrier A and D.
