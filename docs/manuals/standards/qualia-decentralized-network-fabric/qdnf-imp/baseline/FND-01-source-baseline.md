# FND-01 source baseline (implementation inventory)

Recorded 2026-09-08. This lists live source at the first native slice. It does not rewrite the design suite.

## Inherited libp2p (transition)

| Path | Role |
|---|---|
| `crates/qualia-core-db/src/p2p/swarm.rs` | mDNS + Kademlia + request-response |
| `crates/qualia-core-db/src/p2p/sync_node.rs` | TCP + Noise + Yamux |
| `crates/qualia-core-db/src/p2p/protocol.rs`, `sync_ops.rs` | CRDT sync codec |
| `crates/qualia-core-db/src/services/daemon.rs` | Swarm listen on `/ip6/::/tcp/4243` when `libp2p-compat` |

Feature: `libp2p-compat` (optional dep `libp2p` 0.56.0). Still in core-db **default** so existing daemons compile.

## Native replacement (this slice)

| Path | Role |
|---|---|
| `crates/qualia-core-db/src/net/qdnf/` | QFrame, IPC bearer, QLink, QRoute SPF, QSR, QSession, QPolicy, harness |
| `crates/qualia-core-db/src/crypto/network/` | SHA-384, HKDF/HMAC-SHA-384, ChaCha20-Poly1305, X25519, Ed25519, ML-DSA-65, ML-KEM-768, `qpr-pq-1` |
| `crates/qualia-core-db/src/net/peer/runtime/` | leases, reservation ledger, events |
| `crates/qualia-core-db/src/net/peer/host/` | `NativePeer`: beacon discover, QSR lookup, QSession streams |
| `crates/qualia-peer/` | application facade; no `libp2p` import |

## Not done

Raw Ethernet (unprivileged), QSync services, Native Independent feature isolation (GPU/libp2p off for `qualia-peer`), application call-site migration.
