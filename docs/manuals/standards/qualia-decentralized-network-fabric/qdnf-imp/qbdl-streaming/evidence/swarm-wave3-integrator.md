# Wave 3 integrator notes

Not a child brief. Routing and cargo stay here.

## After Wave 3A (E/Q/M) stop — still no cargo until user EMF/QAC notify

```rust
// crates/qualia-core-db/src/bundle/stream/formats/mod.rs
mod emf_stream;
pub use emf_stream::{EmfProfile, EmfStreamView}; // actual names from E
mod qac_stream;
pub use qac_stream::{QacProfile, QacStreamView}; // actual names from Q
// qac.rs already `pub use qac::{QacAdmitError, QacCompleteView};`
// admit_signal is a method on QacCompleteView — no extra route if Q kept it there.

// crates/qualia-core-db/src/net/peer/replication/qbdl/service/mod.rs
mod mux;
pub use mux::{transfer_scheduled, MuxReport, MuxTransfer};
```

Glue only: `PlanError` mapping, `pub use` names. Do not expand scope. `emf_spectrum.rs` stays `super::super::PortalGpu` unless the EMF owner changed the module layout **and** told us to compile.

## After Wave 3A2 (F/H) stop

```rust
// bundle/stream/mod.rs
pub mod mixed;

// bundle/stream/http/mod.rs
#[cfg(all(test, not(target_arch = "wasm32")))]
mod mixed;
```

## Cargo (user notify required, after 3A+3A2 routed)

```
cargo test -p qualia-core-db --lib --locked --offline -j 1
```

Default `target/`. UTF-8 log `evidence/swarm-wave3-cargo-YYYY-MM-DD.log`. LNK1104: retry once. Do not overlap codec rustc on this target. Do not kill Synth MCP.

Focused filters if needed: `formats::qac`, `formats::emf`, `emf_stream`, `qac_stream`, `bundle::stream::plan::`, `bundle::stream::mixed::`, `bundle::stream::http::`, `qbdl::service::`.

Then 3B reviews (E/Q/M/F/H) + C6 records.

## Out of scope

PhysicalNetwork, browser, ZIP HMC, partial legacy QAC, portal EMF display, commit/push.
