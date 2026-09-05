# Vibe DevRel — frozen host contract (`vibe-host-0.1`)

**Language:** `vibe-0.1` · **Host ABI:** `vibe-host-0.1` · **Freeze:** `6dc2b8b8`  
**Four ops only:** parse · check · diagnose · `capability.invoke`  
**Rules:** no Host widen · no dotted `qualia.*` · live `ALL_BOUND` / `vibe:InvokeId` only · a script change must never force a host rebuild.

This is the Stage 1 pack for Poet and agents. It describes what is frozen, not a new API.

---

## 1. Diagnose JSON

`vibe::diagnose(src)` never touches disk and never executes. Cells start with `=`.

### Success

```json
{
  "valid": true,
  "kind": "cell",
  "error_code": null,
  "span": null,
  "message": null,
  "suggested_fix": null,
  "evidential": null,
  "shacl_violations": [],
  "errors": []
}
```

`kind` is `"cell"` or `"module"`.

### Failure

Primary fields come from the first diagnostic. `errors[]` lists every collected diagnostic (same body fields, no nested `valid`/`kind`).

```json
{
  "valid": false,
  "kind": "module",
  "error_code": "E300",
  "span": [12, 40],
  "message": "missing capability(\"HID.poll\") for HID.poll",
  "suggested_fix": "add `using HID;` or requires [ capability(\"HID.poll\") ];",
  "evidential": null,
  "shacl_violations": [],
  "errors": [
    {
      "error_code": "E300",
      "span": [12, 40],
      "message": "missing capability(\"HID.poll\") for HID.poll",
      "suggested_fix": "add `using HID;` or requires [ capability(\"HID.poll\") ];",
      "evidential": null,
      "shacl_violations": []
    }
  ]
}
```

`span` is a UTF-8 **byte** range `[start, end]` (core §9). Monet error glow should map those bytes onto the cell/token, not a line-only highlight.

### Codes

| Code | Meaning |
|------|---------|
| `E001` | Parse / lex |
| `E100` | Type |
| `E200` | Effect (including Pure-cell External) |
| `E300` | Capability missing, unknown, or unbound (default wasm invoke) |
| `E400` | Budget / unbounded loop |
| `E500` | Policy |
| `E600` | Evaluation |
| `E700` | Deontic phase |
| `E701` | Assignment to immutable binding |
| `E702` | Clock unavailable on this host |
| `E800` | Disclosure denied (credentialed refusal, not “not found”) |

`suggested_fix` is a **safe rewrite**. It MUST NOT grant new authority.

`evidential` is `[μ, λ]` only on contradiction/conflict diagnostics (type/effect/deontic). Syntax errors stay `null`.

### Honesty at the invoke seam

| Surface | Bound invoke | Unbound / wasm / deny |
|---------|--------------|------------------------|
| Native Poet host | live `ALL_BOUND` id | fail-closed diagnostic |
| `vibe-wasm` default | — | `E300` `capability.invoke not bound on this host: {id}` |
| Sanctuary volume | `GraphDatabase.volume_open` / `volume_commit` | wasm E300 / denied / fault — never fake a durable `.q42` |

Repair from `suggested_fix`. Do not execute invalid source.

---

## 2. Human dialect vs agent dialect

Both lower to the same catalog. They are not two languages.

### Human (workshop / Poet cells)

```vibe
using GraphDatabase;

effect fn explore(query: string) -> List {
    return GraphDatabase.sparql({ query: query, take: 64 });
}
```

- `using Family;` is a lease for every live `Family.*` id.
- `requires [ capability("graph.read") ];` is the explicit lease form.
- Workshop fixtures must **not** contain `capability.invoke`.

### Agent

```vibe
requires [ capability("GraphDatabase.sparql") ];

effect fn explore(query: string) -> List {
    return capability.invoke("GraphDatabase.sparql", { query: query, take: 64 });
}
```

- First argument is a live `Capability.method` string from `poet_host/invoke/ids.rs`.
- Missing lease ⇒ `E300`. Unknown id ⇒ `E100` fail-closed.
- Do not invent `qualia.graph.query` / `qualia.volume.open`. Remap:

| Do not write | Live bind |
|--------------|-----------|
| `qualia.graph.query` | `GraphDatabase.sparql` |
| `qualia.infer.complete` | `Inference.*` (exact method from `ALL_BOUND`) |
| `qualia.render.preview` | `Render.*` (exact method from `ALL_BOUND`) |
| `qualia.volume.open` | `GraphDatabase.volume_open` / `volume_commit` |

Catalog: `capability.invoke("CapabilityDiscovery.catalog", null)` once a host bind exists; otherwise read `ids.rs`.

---

## 3. Fixture pack

See [`crates/vibe/fixtures/FIXTURE_PACK.md`](../../crates/vibe/fixtures/FIXTURE_PACK.md).

Graph, volume, and diagnose-loop files are hot-editable. `cargo test -p vibe --test sprint_b_fixtures` is the Stage 1 acceptance gate.
