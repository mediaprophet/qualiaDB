# Wave 3 integrator notes

Not a child brief. Routing and cargo stay here.

## After M/F/H stop (still no cargo until user EMF/QAC notify)

```rust
// crates/qualia-core-db/src/net/peer/replication/qbdl/service/mod.rs
mod mux;
pub use mux::{transfer_scheduled, MuxReport, MuxTransfer};
// actual exports: whatever M named. Do not invent extras.

// crates/qualia-core-db/src/bundle/stream/mod.rs
pub mod mixed;

// crates/qualia-core-db/src/bundle/stream/http/mod.rs
#[cfg(all(test, not(target_arch = "wasm32")))]
mod mixed;
```

Fix compile-only glue (PlanError mapping, `pub use` names, `Box::write` / `map_err` style as in Wave 2.5). Do not expand scope.

`emf_spectrum.rs` stays `super::super::PortalGpu` unless the EMF owner changed the module layout **and** told us to compile. Do not preemptively retarget.

## Cargo (user notify required)

One locked:

```
cargo test -p qualia-core-db --lib --locked --offline -j 1
```

Default `target/`. UTF-8 log `evidence/swarm-wave3-cargo-YYYY-MM-DD.log`. LNK1104: retry once, keep both logs. Do not overlap EMF rustc on this target. Do not kill Synth MCP.

Focused filters if the full lib is too wide after EMF churn:

- `bundle::stream::plan::`
- `bundle::stream::formats::qac`
- `bundle::stream::http::`
- `bundle::stream::mixed::`
- `qbdl::service::`
- `protected_session_tests`

Then 3B reviews + C6 records.

## Out of scope for integrator this wave

PhysicalNetwork, browser, ZIP HMC, `DescriptorPage` buffer lift, commit/push.
