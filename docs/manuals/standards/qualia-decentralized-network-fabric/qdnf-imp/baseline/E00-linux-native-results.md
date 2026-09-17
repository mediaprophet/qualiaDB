# E00 Linux native runner results

Recorded 2026-09-09. Not a security certification. Original 30 packages remain pending.

| Field | Value |
|---|---|
| Runner | `linux-x86_64-gnu` rustc 1.98.1 / cargo 1.98.1 |
| MSVC | unavailable (`link.exe` not present; not claimed) |
| Integration HEAD at start | `5646b818d4e6fa63f0b7c4c6a1a469dca382339a` |
| Review baseline | `17b6b467c8a4548e302f603674dd594a67be7398` |
| Evidence class | Component + in-process IPC (not PhysicalNetwork) |

## Executed

```
cargo test -p qualia-core-db --lib --offline -- \
  net::qdnf::harness::qualification \
  net::qdnf::harness::oracles \
  net::qdnf::harness::intercept \
  net::qdnf::harness::negative \
  net::qdnf::authority \
  net::qdnf::crypto::finished \
  net::qdnf::crypto::schedule \
  net::qdnf::session::handshake \
  net::qdnf::session::packet_protection \
  net::peer::host \
  net::peer::runtime::ledger \
  net::peer::cells::admit \
  net::qdnf::vertical \
  crypto::network::vectors \
  crypto::network::pq_handshake \
  crypto::network::kdf::tests
```

Result: **89 passed**, 0 failed, 7710 filtered out.

```
cargo test -p qualia-peer --lib --offline
```

Result: **3 passed**, 0 failed.

## Qualification gates observed

- Instrument failure of allocator / wire / crypto fails `qualify()` even when component tables pass.
- `open_session` cannot mint Active/Allow.
- Public IPC exchange seals application bytes; wire oracle rejects plaintext copies.
- Finished production HMAC ≠ historical unkeyed SHA-384 vector.

## Not executed

Physical Ethernet, process-crash durability, default-feature daemon migration, enhancement-plan checkbox completion.
