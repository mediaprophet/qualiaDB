# Assignment CSCP-09 — HTTP Datagram capsule fallback (local)

- Parent: CSCP-09. Outcome: RFC 9297-style capsule encode/decode for CSCP bytes on a caller buffer. Not HTTP/3, not MASQUE Internet, not a full HTTP/2 stack.
- Explicit non-goals: no `h2`/`hyper`/`quinn` dependency; do not dial the network; do not set `masque_bound_udp_internet_executed`; do not edit `mod.rs` or Cargo.toml.
- Allowed writes only:
  - `crates/qualia-core-db/src/net/peer/fabric/capsule.rs`
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/handoffs/CSCP-09.md`
- Integrator will add `pub mod capsule;` after review.

## Semantic requirement

When UDP/HTTP/3 is unavailable, CSCP control bytes travel as HTTP Datagrams using the Capsule Protocol (RFC 9297). Implement the **capsule framing** used by that fallback:

- Variable-length integer (QUIC/HTTP varint) type + length + value.
- DATAGRAM capsule type `0x00` carries an HTTP Datagram payload.
- CSCP messages (magic `CSCP`) are the datagram payload; do not re-encode the CSCP TLV grammar.
- Unknown capsule types with the high bit set fail closed; unknown non-critical types are skipped with a counted skip.
- Max capsule value 1024 bytes (same as CSCP `MAX_BODY` plus header room — cap the **value** at 1024).
- Caller-owned `&mut [u8]` output. No `Vec`/`String`/`Box` on the encode/decode hot path. Stack arrays only.
- Nested recovery is degraded: document in module rustdoc that this is not equivalent to UDP.

Public API (keep these names):

```rust
pub const CAPSULE_DATAGRAM: u64 = 0x00;
pub fn encode_datagram_capsule(payload: &[u8], out: &mut [u8]) -> Result<usize, CapsuleError>;
pub fn decode_datagram_capsule(bytes: &[u8]) -> Result<&[u8], CapsuleError>;
```

Tests (in the same file `#[cfg(test)]`):

1. Round-trip a small CSCP-shaped payload (need not call the CSCP codec).
2. Truncated buffer / truncated input fails closed.
3. Oversize payload (>1024) is Capacity.
4. Unknown critical type rejected; unknown non-critical skipped or rejected with a distinct error — pick one, document it, test it.
5. Do not claim MASQUE Internet.

Do not import wgpu, naga, or identity modules. Isolated cargo:

```
CARGO_TARGET_DIR=/tmp/qdnf-continue-target
cargo test -p qualia-core-db --lib --offline net::peer::fabric::capsule -- --test-threads=1
```

That filter will fail until the integrator adds `mod capsule`. Implement the file so it would compile as `pub mod capsule` under `fabric/`. If you cannot compile because `mod.rs` is forbidden, still write correct Rust and record that in the handoff.

File must stay under 500 lines.
