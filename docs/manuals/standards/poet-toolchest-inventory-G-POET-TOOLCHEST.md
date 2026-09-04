# Poet Toolchest Inventory — G-POET-TOOLCHEST (2026-09-04)

> Branch: `0.0.36-dev` · Slice: inventory + one live toolchain onto `ALL_BOUND`  
> Owner: Neo · Gaps triage: Vibe · Gate reports: Capt.

## Model (locked)

**toolchest → toolbox → toolchain → tools (buttons)**  
Containers / manifolds / links = content chrome + Marvin shapes.  
Buttons hot-edit to live `Capability.method` **or stay gated**. No Host widen. No invented dotted `qualia.*`.

## Where it lives

| Area | Path |
|------|------|
| Core types | `crates/poet/src/tool_chest/core/{toolbox,tool_chain,tool,registry}.rs` |
| Registration (15 toolboxes) | `crates/poet/src/browser/registration.rs` |
| Dispatch honesty | `crates/poet/src/browser/tool_actions.rs` |
| Button widgets | `crates/poet/src/browser/tool_widgets.rs` |
| Icons (PUA + unicode fallback) | `crates/poet/src/browser/icon_registry.rs` |
| Manifold seeds | `crates/poet/src/tool_chest/manifolds/*.rs` |
| Specs | `crates/poet/tool-chest/*.md` |
| Live invoke helper | `crates/poet/src/browser/live_invoke.rs` (`data-live-capability`) |
| Daemon IPC | `crates/poet/src/browser/native_daemon.rs` (`daemon_invoke`) |

## Toolboxes registered (15)

epistemic · office · image · sheet · spatial · audio · communication · erp · mail · scientific · rights · health · code · ai · sdn

## Live vs stub / gated (dispatch)

### Executable today (local DOM / menu)

| Tool id | Behaviour |
|---------|-----------|
| `epistemic:tag_*` (4) | Annotate selected container (`data-semantic-*`) |
| `image:marker` | Annotate selected container |
| `spatial:pin` | Annotate selected container |
| `mail:composer` | Place container via menu |
| `rights:authors_group` | Place container via menu |
| `ai:extractor` | Bounded local token/sentence/entity extraction |
| `ai:sentinel` | Bounded standalone DOM/surface safety check |

### Daemon-enhanced (optional QualiaDB daemon)

| Tool id | Invoke id |
|---------|-----------|
| `graph:sparql_query` | `GraphDatabase.sparql` |

When a daemon is connected, `ai:extractor` upgrades to the structured
`/gazetteer` transport for `NLP.gazetteer_run`; the capability id is retained
in tool metadata so the catalog and transport remain aligned. When connected,
`ai:sentinel` invokes `Sentinel.inspect` directly through `/invoke`; otherwise
it runs the bounded standalone surface check. `graph:sparql_query` similarly
queries Poet's local semantic container graph offline and upgrades to the
daemon-backed graph when connected.

### Honestly gated (`unavailable_reason`)

`audio:mic_capture`, `audio:neural_latents`, `mail:publisher`, `scientific:thermodynamics`, `sdn:energy_governor`, `image:heatmap`, `sheet:import`, `spatial:track`, `rights:fiduciary_sign`, `rights:did_sign`, `health:pathology`, `code:quin_statement`, `ai:co_author`

### Structural stubs

- **Rich control chains:** `office:typography` and `office:paragraph` now expose
  local DOM formatting actions (bold, italic, code, heading, and alignment).
- **Construct stubs:** `constructs/mod.rs` `stub_constructs()` honesty=`stub`, no manifold seed (Library Software).
- **Manifold honesty:** many seed containers marked `partial` / `missing` / `present` (elevated to `live` only with daemon).
- **capability_scope drift:** most tools use scopes like `graph:read` / `graph:annotate` — **not** `Capability.method` strings. First live bind uses `GraphDatabase.sparql` in `capability_scope`.
- **No knowledge toolbox** despite `knowledge` manifold seed — graph explore is a **container**, not a toolchain button (until this slice).

## Unicode / custom buttons

- `icon_registry.rs`: compile-time PUA `U+E000..U+E1FF` + `unicode_fallback` + 4-tier degradation. Lookups by static id.
- **Gap (B):** no runtime API to register **custom** user glyphs/icons; unknown ids render `?`. Specs mention custom buttons; implementation is static registry only.
- Widget layer (`tool_widgets.rs`) renders `icon` string / toggle `glyph` text — fine for human chrome once ids resolve.

## Maps / spatiotemporal (parked)

- Geospatial: `spatial` toolbox places map/3D containers; map quality gap (open layers vs universe/fantasy) stays UX + B — not this slice.
- Temporal: Layout/Stage/Timeline twins remain Marvin B shapes joined to live binds.

## First live slice (done in companion commit)

Wire **`office:graph` toolchain** → tool **`graph:sparql_query`** to a bounded local semantic graph,
with an optional daemon upgrade to **`GraphDatabase.sparql`**:

1. Register toolchain + tool with `capability_scope: Some("GraphDatabase.sparql")`.
2. `requires_daemon("graph:sparql_query")` is false; standalone Poet remains executable.
3. Dispatch: selected text as SPARQL (else bounded `ASK WHERE { ?s ?p ?o } LIMIT 1`), local query when offline, and `daemon_invoke("GraphDatabase.sparql", …)` when connected.
4. Daemon faults preserve the local operation instead of disabling the control.

## Remaining for Capt / Vibe

| Item | Owner |
|------|-------|
| Custom unicode registration API | Neo (B) |
| Remap remaining `capability_scope` → `Capability.method` | Neo + Vibe |
| Map layer / universe containers + temporal attrs | davinci/monet + Marvin |
| G-A four closes / G-B-001 volume | Neo (separate gates) |

## Change log

- 2026-09-04: Initial inventory + first live SPARQL toolchain bind (Neo).
