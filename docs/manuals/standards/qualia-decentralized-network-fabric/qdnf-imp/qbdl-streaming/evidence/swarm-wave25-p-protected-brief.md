# Wave 2.5-P — ProtectedSession two-process (expert)

File-disjoint. **Do not cargo. Do not commit.** Exclusive write: `crates/qualia-core-db/src/net/peer/replication/qbdl/service/two_process.rs` plus evidence `swarm-wave25-p-summary.md`.

## Gap

`two_process.rs` is a framing `Session` plus caller `TRANSCRIPT` (`EvidenceClass::Process`). Production I/O is `ProtectedSession` over an established `ProtectedChannel` (handshake transcript, AEAD AAD = `ServiceRecordHeader` 88 bytes `QBDL-SCT-V1\0`). In-process useful-leaf already exists in `protected_session_tests.rs`. R01 still wants a **fresh handshake between OS processes**.

Handshake `Fixture` in `endpoint/tests.rs` is **private** and uses `generate_managed_signing_pair` (random keys). Intent embeds **both** initiator and responder key ids. PeerTrust pins require the peer’s public keys **before** `Initiator`/`Responder::new`. Two OS processes cannot share Intent/PeerTrust without a pre-shared key package.

Do **not** edit `endpoint/tests.rs`. Reconstruct the signer/intent/pin flow inside `two_process.rs`.

## Keep

Existing `two_os_process_transfers_useful_leaf` Session fixture and TCP/foreign-name refuse tests. Add a **second** test for ProtectedSession. Do not claim `EvidenceClass::PhysicalNetwork`. `physical_two_host_from_this_module` stays `false`. HTTP/DNS/IP/libp2p stay disabled. In-process `ipc_pair` is not a fallback for the OS-process test.

## Binary key package (not JSON)

Parent generates **both** managed signing pairs with `CryptographicLibrary::generate_managed_signing_pair`, then `get_key` for `{ml,ed}_private` and the returned public keys.

Write an **exact-length binary** file (temp path, env e.g. `QDNF_TWO_PROCESS_KEYS`). Suggested layout:

- magic 12: `b"QBDL-PSK1\0\0\0"`
- `expires_unix` u64be
- `Intent::encode` 520 bytes (`INTENT_BYTES`)
- for each of 8 blobs (initiator ml priv, initiator ed priv, initiator ml pub 1952, initiator ed pub 32, responder same): `u32be len || bytes`

Reconstruct `Key` + `KeyMetadata` in the child with **fixed** ids (`ml_private` / `ed_private`) and `store_key`. Do not serde/JSON the package. Do not put key bytes in env. Zeroize buffers after install. Child uses initiator secrets + responder publics; parent uses responder secrets + initiator publics.

Each process builds its own `PinAuthority::new_local_bootstrap()` and installs the **peer** public keys (`INITIATOR_PURPOSE` / `RESPONDER_PURPOSE`, audience = local principal bytes, same as `endpoint/tests.rs`).

## Handshake then useful leaf

Child = Initiator / `ROLE_CLIENT`. Parent = Responder / `ROLE_PROVIDER`. Bearer is the existing named-pipe / UDS `FramedStream` (`BearerProfile::LocalIpcV1`).

Mirror `protected_channel/endpoint_fixtures.rs` `with_pair` **without** Tap:

1. `EndpointAuthority::adopt_local` with the shared Intent and role
2. `EndpointReservation::admit` on a local `ReservationLedger`
3. `Initiator::new` / `Responder::new` + `sign_client` / `sign_server` via `begin_paired_signing` (copy the `Signer::capability` request from `endpoint/tests.rs`: policies `ml_private`/`ed_private`, `AuthenticationMethod::HardwareToken`, epoch controller 7)
4. Poll `send_step` / `receive_step` on WouldBlock with the existing accept-poll sleep. **Drop** `CurrentAuthorityGuard` / ledger locks before sleep.
5. `install` then `EstablishedSession::into_protected_channel` under `CurrentAuthorityGuard::try_lock` of **this** process’s `AuthorityOwner`. Drop the guard before `ProtectedSession` I/O.
6. `ProtectedSession::attach(..., ROLE_*, &mut SessionOwner)`.
7. Same useful-leaf transfer as the Session fixture (`transfer_one_leaf` / Open+Credit+RequestLeaves+LeafData of the tail 19-byte leaf). Wrong-transcript AAD is already enforced by the channel; do not manufacture `Session` from an arbitrary digest here.

WouldBlock send on the service path must drop authority then retry (ProtectedSession already does this).

## Tests (write, do not run)

1. `two_os_process_protected_session_transfers_useful_leaf` — Windows named pipe and Unix UDS cfg like the existing test. `EvidenceClass::Process`. Assert not PhysicalNetwork. Child without the key file must fail closed (Incomplete/Denied), not fall back to `Session`+`TRANSCRIPT`.
2. Keep `two_os_process_transfers_useful_leaf` (framing Session) and `wire_is_length_prefixed_binary_not_json`.
3. If a truncated/mutated key package is cheap to add, refuse it (`Malformed`/`Denied`). Optional.

If handshake over the pipe cannot be completed honestly without editing handshake internals, **stop and document the exact vis bump** needed in the summary. Do not invent a parallel protocol. Do not mint BearerProfile 8+. Do not mint Kind 13+.

## Stop

Write `docs/.../qbdl-streaming/evidence/swarm-wave25-p-summary.md`: what landed, key-package layout, remaining vis bumps, tests written **not run**. Do not cargo. Do not commit. Do not touch EMF/QAC owner files.
