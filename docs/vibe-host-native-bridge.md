# Vibe-host native bridge

`vibe-host` is the execution/adapter boundary for VibeScript. Poet is a user
interface that can use that host; it is not the host ABI.

You do not need a native installation to explore Vibe or Q42: the
[RDF → Q42 → Vibe browser demo](rdf-q42-vibe-demo.html) and the Vibe showcase
run their standalone WASM surfaces locally in the browser. This bridge exists
only when a program deliberately needs a paired native capability.

The daemon now exposes `GET /vibe/capabilities`. Its versioned response uses
the `qualia-vibe-bridge/1` protocol and makes the execution route explicit.
The endpoint is open only in daemon development mode; production requests need
an `X-Qualia-Token` accepted by the local key vault.

There are three important outcomes:

| Mode | Meaning |
| --- | --- |
| `standalone-wasm` / `exact` | The loaded WASM module performs the operation itself. |
| `standalone-snapshot` / `isolated-snapshot` | The WASM host can evaluate against its own graph snapshot, but the result is not a persistent native graph read or transaction. |
| `native-bridge` | The operation needs a paired local native daemon. If the daemon is absent, it is unavailable rather than replaced with a different calculation. |

The first bridge transport is authenticated loopback HTTP because it is already
implemented by the daemon (`/query`, `/update`, and `/vibe/capabilities`). A
browser client must probe only after a user gesture, use the pairing token, and
handle the request asynchronously. The synchronous Vibe `Host` trait is not an
appropriate place to block on browser IPC; an async suspend/resume execution
surface is required before host invocations themselves are tunnelled.

WASI transports are a separate adapter family. A local WASI runner may use
stdio or a Unix-domain socket, but browser WASM must use the loopback service.
