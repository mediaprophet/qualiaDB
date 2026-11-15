# Wave 2.5-P — ProtectedSession two-process

Source landed. **Tests written, not run. Did not cargo. Did not commit.**

Exclusive writes: `crates/qualia-core-db/src/net/peer/replication/qbdl/service/two_process.rs` and this summary. Handshake internals, `session.rs`, `protected.rs`, `describe.rs`, `endpoint/tests.rs`, EMF/QAC, `registries.rs`, and Cargo.toml were not edited.

## What landed

Existing framing `Session` fixture is unchanged: `two_os_process_transfers_useful_leaf`, TCP/foreign-name refuse, `wire_is_length_prefixed_binary_not_json`. `physical_two_host_from_this_module` stays `false`. Evidence class is `Process` (asserted not `PhysicalNetwork`). HTTP/DNS/IP/libp2p/WSS TCP stay unused. `ipc_pair` is not a fallback.

Added a second OS-process fixture over the same named-pipe / UDS `FramedStream` (`BearerProfile::LocalIpcV1`):

1. Parent generates both managed signing pairs, writes a binary pre-shared key package, sets `QDNF_TWO_PROCESS_KEYS`, binds, spawns, accepts.
2. Child = Initiator / `ROLE_CLIENT`. Parent = Responder / `ROLE_PROVIDER`.
3. Each process builds its own `PinAuthority::new_local_bootstrap()` and pins the **peer** publics (`INITIATOR_PURPOSE` / `RESPONDER_PURPOSE`, audience = local principal bytes, same request as `endpoint/tests.rs` `Signer::capability`: policies `ml_private`/`ed_private`, `AuthenticationMethod::HardwareToken`, controller 7).
4. Handshake mirrors `protected_channel/endpoint_fixtures.rs` `with_pair` without Tap: `EndpointAuthority::adopt_local`, `EndpointReservation::admit`, `Initiator`/`Responder::new`, `sign_client`/`sign_server` via `begin_paired_signing`, poll `send_step`/`receive_step` on `WouldBlock` (sleep `ACCEPT_POLL_MS`, drop no `CurrentAuthorityGuard` / ledger lock across sleep; `guarded` already leaves WouldBlock un-aborted).
5. `install` then `EstablishedSession::into_protected_channel` under `CurrentAuthorityGuard::try_lock` of **this** process’s `AuthorityOwner`. Guard is dropped before `ProtectedSession` I/O.
6. `ProtectedSession::attach(..., ROLE_*, &mut SessionOwner)`.
7. Same useful-leaf tail as the Session fixture (Open+Credit+RequestLeaves+LeafData of the 19-byte leaf). Transcript is the handshake value; no `Session`+`TRANSCRIPT` manufacture.

Child without `QDNF_TWO_PROCESS_KEYS` / unreadable path returns `Incomplete` and panics with an explicit “no Session/TRANSCRIPT fallback” message.

## Key-package layout (exact-length binary, not JSON)

Temp file, path only in env (`QDNF_TWO_PROCESS_KEYS`). No key bytes in env.

| Offset / field | Size | Content |
|---|---|---|
| magic | 12 | `b"QBDL-PSK1\0\0\0"` |
| expires_unix | 8 | u64be; must equal `Intent.expires_unix` |
| intent | 520 | `Intent::encode` (`INTENT_BYTES`) |
| 8 blobs | `u32be len \|\| bytes` | initiator ml priv, initiator ed priv, initiator ml pub (1952), initiator ed pub (32), responder same |

Exact sizes required: ML-DSA-65 sk 4032 / pk 1952, Ed25519 sk/pk 32. Trailing bytes, truncated input, or intent/header expiry mismatch → `Malformed`. Wrong magic → `Denied`. Past expiry → `Expired`. Missing file/env → `Incomplete`.

Child `store_key`s initiator secrets under fixed ids `ml_private` / `ed_private` and pins responder publics. Parent keeps responder secrets in memory after write, zeros initiator privates in RAM, pins initiator publics. Package bytes and secret vecs are `Zeroize`d after install. File is deleted on parent drop.

## Remaining vis bumps (not applied)

Handshake steps used here are already `pub`. No parallel protocol. No BearerProfile 8+ or Kind 13+.

Exact bumps that would match the brief’s `CryptographicLibrary::generate_managed_signing_pair` + `get_key` wording. Not applied (exclusive write is `two_process.rs` only):

1. `CryptographicLibrary::key_manager` is `pub(super)`, so `get_key` / `store_key` are not callable from this module. This fixture uses the public `KeyManager` API that `generate_managed_signing_pair` already wraps (`generate_key_pair` + `store_key` + `begin_paired_signing`) and keeps the returned private `Key` values for the package.

   One-line vis on `library.rs`:
   `pub fn get_key(&self, key_id: &str) -> Result<Key, CryptographicError> { self.key_manager.get_key(key_id) }`
   and
   `pub fn store_key(&mut self, key: Key) -> Result<(), CryptographicError> { self.key_manager.store_key(key) }`

   Then parent could call `generate_managed_signing_pair("ml", "ed", High)` and `get_key("ml_private")` / `get_key("ed_private")`.

2. `Intent::decode` does not exist (`encode` is public; fields are public). This fixture reconstructs `Intent` from the 520-byte encode layout. Optional: `pub fn decode(bytes: &[u8]) -> Result<Self, E>` on `intent.rs`.

## Tests written, not run

- `two_os_process_protected_session_transfers_useful_leaf` — `#[cfg(all(windows, target_arch = "x86_64"))]` and `#[cfg(unix)]`, same spawn/accept pattern as the Session fixture. `EvidenceClass::Process`.
- `truncated_key_package_is_malformed` — empty/truncated → `Malformed`; mutated magic → `Denied`.
- `protected_child_without_key_package_is_incomplete` — missing file → `Incomplete`.
- Kept: `two_os_process_transfers_useful_leaf`, `child_refuses_tcp_and_foreign_pipe_name`, `wire_is_length_prefixed_binary_not_json`.
