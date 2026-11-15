# Swarm S3.1 in-process QBDL service

In-process provider/client over existing `ipc_pair`. Reuses `qbdl/wire` (QBS1 / `PREFIX_BYTES=208` / `SERVICE_IRI`), `qbdl/carrier` assembly, `qbdl/provider/outboard`, and `LocalWholeSourceGrant`. HTTP/DNS/IP/libp2p unused. Wire Grant is protocol ack only. Mixed/private/protected wraps return `Unsupported`. No JSON, no new magic, no second Merkle.

## Integrator

Add to `crates/qualia-core-db/src/net/peer/replication/qbdl/mod.rs`:

```rust
pub mod service;
```

Do not edit `qbdl/wire`, `qbdl/carrier`, or `qbdl/provider`. Cargo remains integrator-owned. This child did not run `cargo`.

## Test names (`service/tests.rs`, submodule of `service/mod.rs`)

- `useful_leaf_transfers_while_unrelated_payload_stays_unrequested`
- `malformed_prefix_refuses_and_fail_closes`
- `cancel_fail_closes_without_leaf_disclosure`
- `close_fail_closes_session`
- `mixed_private_protected_wraps_are_unsupported`

First profile: Open → Grant (ack) → Credit → RequestLeaves → LeafData for one selected entry and one leaf. Client verifies with existing `verify_leaf`. Bearer is `BearerProfile::LocalIpcV1` only (not OS processes).

## Two-process join still owes

- S3.2 Windows named pipe and Unix-domain `local_stream` (this host’s in-process queue is not OS IPC).
- S3.3 spawn two OS processes, transfer a useful framed QBDL record, HTTP/DNS/IP/libp2p disabled; Windows vs independent Linux/WSL are distinct evidence.
- `policy/grant/current.rs` same-guard bridge so `CurrentAuthorityGuard` can feed `LocalWholeSourceGrant` without a long exclusive owner borrow.
- QBS10 consumed common channel / one `PacketProtection` (in-process frames here are plaintext `ServiceRecordHeader` + chunk).
- Describe/DescriptorPage/Select/Resume/Outcome, remote Share/Sentinel, and native storage job handoff (`write_leaf` still holds StorageStage across OS wait).
