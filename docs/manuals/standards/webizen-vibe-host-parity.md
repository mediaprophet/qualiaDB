# Webizen Desktop — Vibe host parity

**Packet:** W12 · **Frozen:** `vibe-host-0.1`  
**Commands:** `crates/webizen-desktop/src/commands/vibe_host.rs`

Webizen Desktop exposes the same four ops as `poet::vibe_host` and `vibe-wasm`.
The script is a string argument; editing Vibe source does not require rebuilding
the desktop binary.

| Op | Command | Notes |
|----|---------|-------|
| versions | `vibe_host_info` | `vibe-0.1` / `vibe-host-0.1` / crate stamp |
| parse | `vibe_parse` | cell if source starts with `=` |
| check | `vibe_check` | parse + type/effect |
| diagnose | `vibe_diagnose` | JSON including `errors[]` |
| invoke | `vibe_capability_invoke` | in-process catalog kernels (`invoke_local`); unknown ids fail closed |

`poet_eval` remains the snapshot/eval harness (graph, pulse, ticks). It is not
a fifth frozen op.

## Tests

`cargo test -p webizen-desktop vibe_host` covers versions, diagnose `errors[]`,
good-cell parse/check, live `Animation.evaluate_preset`, unknown-id fail-closed.
