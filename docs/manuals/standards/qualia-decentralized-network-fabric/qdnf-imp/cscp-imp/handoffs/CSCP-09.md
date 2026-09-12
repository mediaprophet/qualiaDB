# CSCP-09 handoff — HTTP Datagram capsule framing (local)

## Result

- Package and child IDs completed or still pending:
  - Child: CSCP-09 local capsule framing — **implemented**, awaiting integrator `pub mod capsule;` and review.
  - Parent CSCP-09 (HTTP/2 capsule fallback as a live carrier) remains **incomplete**: this is RFC 9297-style framing only, not an HTTP/2 stack, not MASQUE, not Internet.
- Changed files, public behavior and integration source state:
  - `crates/qualia-core-db/src/net/peer/fabric/capsule.rs` (346 lines, tests in-file).
  - This handoff. `mod.rs` and `Cargo.toml` **not** edited (forbidden).
  - Public API:
    - `CAPSULE_DATAGRAM = 0x00`
    - `MAX_CAPSULE_VALUE = 1024`
    - `encode_datagram_capsule(payload, &mut [u8]) -> Result<usize, CapsuleError>`
    - `decode_datagram_capsule(bytes) -> Result<&[u8], CapsuleError>`
    - Also: `encode_capsule`, `decode_capsule` → `Result<(u64, &[u8]), CapsuleError>`, `capsule_span` for counted skip.
  - Wire: QUIC/HTTP varint type + length + value. Value is opaque CSCP bytes (not re-TLV’d).
  - Unknown policy (local CSCP profile, documented in rustdoc): `0x01..=0x3f` → `UnknownNonCritical` (`decode_capsule` errors; `decode_datagram_capsule` skips and continues); `>= 0x40` → `UnknownCritical` fail-closed. Stricter than RFC 9297 (which ignores all unknown types); stands in for the unencodable `type >= 1<<62` high-bit rule.
  - Nested recovery labelled degraded / not equivalent to UDP. Honesty flags not touched.
- Accepted predecessor/interface versions:
  - CSCP-01 wire codec (`MAX_BODY = 1024`) as the numeric value cap. No import of `wire` (isolated module).
- Evidence manifest paths:
  - `/opt/cursor/artifacts/cscp-09-rustc-standalone.log` — `rustc --edition 2021 --crate-type lib --test` on the file: **8 passed**.
  - `/opt/cursor/artifacts/cscp-09-cargo-filter.log` — crate filter with no `mod capsule`.
- Checks executed with outcomes; checks not executed and why:
  - Standalone: `rustc --edition 2021 --crate-type lib --test crates/qualia-core-db/src/net/peer/fabric/capsule.rs -o /tmp/cscp-capsule-test && /tmp/cscp-capsule-test --test-threads=1` → 8 passed, 0 failed.
  - `CARGO_TARGET_DIR=/tmp/qdnf-continue-target cargo test -p qualia-core-db --lib --offline net::peer::fabric::capsule -- --test-threads=1` → exit 0, **0 tests ran**, 8469 filtered out. Expected: module is not in `fabric/mod.rs` yet, so the crate never compiles these tests. Cargo does not fail a zero-match filter.
  - Tests covered: CSCP-shaped round-trip; truncated encode/decode fail-closed; payload >1024 and claimed length 1025 → Capacity; 1024-byte value round-trips; type `0x01`/`0x3f` skip-and-continue (counted, including two leading skips); type `0x40` critical reject even when a DATAGRAM follows; overlong varint Malformed; empty value; no MASQUE/HTTP/3 claim.
- Measured resource bounds, enforcement limitations and error behavior:
  - Value cap 1024 enforced on encode and decode (`Capacity`).
  - Caller `&mut [u8]` encode; decode returns `&[u8]` into the input. No `Vec`/`String`/`Box` on encode/decode.
  - Truncated varint/value → `Truncated`. Overlong varint → `Malformed`. Short output buffer → `Capacity`.
  - `decode_datagram_capsule` walks a capsule stream: counted skip of non-critical, first DATAGRAM value returned, leftover after that DATAGRAM is not parsed. Stream of only non-critical → `UnknownNonCritical`. Empty → `Truncated`.
- Review findings, fixes and independent reviewer:
  - Author-only. CSCP-11 does not cover this file. Integrator review required before `mod capsule`.
- Provisional assumptions removed or still blocking acceptance:
  - Integration blocked on `pub mod capsule;` in `net/peer/fabric/mod.rs` (integrator-owned).
  - No HTTP/2 SETTINGS/CONNECT, no `h2`/`hyper`/`quinn`, no dial. Framing is not a carrier.

## Integration

- Shared exports/Cargo/profile/registry edits requested from their owner:
  - Integrator: add `pub mod capsule;` to `crates/qualia-core-db/src/net/peer/fabric/mod.rs`. Optional `pub use capsule::{decode_datagram_capsule, encode_datagram_capsule, CapsuleError, CAPSULE_DATAGRAM};`.
  - No Cargo.toml / dependency changes.
  - Do not set `masque_bound_udp_internet_executed` or any Internet honesty flag.
- Consumer packages and exact interface migration needed:
  - None yet. A later HTTP/2 (or H2-over-TLS) carrier would call `encode_datagram_capsule` / `decode_datagram_capsule` on CSCP TLV bytes from `wire/`.
- Merge/conflict concerns and concurrent changes preserved:
  - New file only. Wave 2 siblings (CSCP-07/10/11) write other paths. `mod.rs` left untouched to avoid collision.
- Rollback/recovery and durable schema compatibility:
  - Delete `capsule.rs` if rejected. No on-disk schema. Wire is local framing, not a published IETF profile.
- Outstanding ownership, temporary artifacts and cleanup:
  - `/tmp/cscp-capsule-test` standalone binary (scratch). `/tmp/qdnf-continue-target` reused per brief.
- Next owner, next action and proposed registry transition:
  - Integrator: add `pub mod capsule;`, rerun `cargo test -p qualia-core-db --lib --offline net::peer::fabric::capsule -- --test-threads=1`, then move CSCP-09 child to `review` (framing) — **not** parent-complete for HTTP/2 fallback.
  - Proposed checked child: CSCP-09 framing, with standalone rustc evidence until the module is wired.
