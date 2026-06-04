# ADR 006: Zero-Allocation Solid Bridge Isolation

## Status
Accepted (v0.0.4)

## Context
To ensure Qualia-DB can bootstrap within existing decentralized ecosystems, we required interoperability with W3C Solid apps. Solid applications expect to communicate via HTTP REST APIs and string-heavy linked data formats (JSON-LD, Turtle). 
However, Qualia-DB's core architecture strictly prohibits heap string allocations and non-deterministic event loops to maintain its 512MB RAM mechanical sympathy and continuous logic execution bounds.

## Decision
We created the `qualia-solid-bridge` using `warp` and `tokio`, but bounded it by a strict **Allocation Firewall**.

1. The `tokio` multi-threaded runtime is sandboxed explicitly at the network boundary.
2. The `ldp_translator.rs` module translates incoming HTTP strings into raw 64-bit Quin hashes *before* they cross the FFI boundary into the Webizen VM.
3. The native engine (`qualia-core-db`) remains completely oblivious to HTTP or string allocations.
4. Profiling via `dhat-rs` is mandated to ensure zero bytes of heap bleed across this boundary.

## Consequences
- Enables full W3C Solid backward compatibility (LDP Basic Containers).
- Prevents malicious or oversized HTTP payloads from triggering Out-Of-Memory (OOM) panics in the core database.
- Introduces `tokio` to the workspace, requiring strict architectural vigilance to prevent its usage from spreading into the core `SlgArena` logic.
