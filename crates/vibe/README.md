# vibe

**VibeScript (`vibe-0.1`)** — a language for humans and machines (the JS replacement).

**Poet** is the human creative environment and CLI that hosts Vibe. Other hosts (desktop, WASM, agents) can host it too.

Copyright © 2026 Timothy Charles Holborn.

Normative spec: `docs/manuals/standards/vibescript-core.md` (QualiaDB tree).

**Language engine lives here.** LSP, syntax highlighting, playground, VS Code, agent skills, and docs pages live in the sibling repo `C:\Projects\vibe-script` so tools are not piled into QualiaDB.

No JIT. No wgpu. No raw Quin overlay literals. Scripts call capabilities; the host seals Quin parity.

Human dialect (lowers to the same leases):

```vibe
using Animation, HID;

effect fn spin(t: f64) {
    return Animation.orbit_spin(t);
}
```

`capability.invoke("Family.method", { ... })` remains the agent/compiled form.

Compatibility correction, 2026-10-25: pinned `vibe-0.1` statement assignment
supports a mutable whole binding (`name = value`). Member/index assignment is
not implemented and now fails closed with E701 in both checker and interpreter,
before evaluating its right-hand side. Earlier interpreter code evaluated the
right-hand side and silently ignored those unsupported targets. Assignment to
an immutable binding is also rejected before right-hand-side effects. Construct
an updated record/list and assign the whole mutable binding instead. This is an
admission correction, not a released new dialect or nested-lvalue feature.

Agent loop: `vibe::diagnose(src)` → JSON (`error_code`, `span`, `suggested_fix`).
Grammar: `grammar/vibe-0.1.ebnf`, `grammar/vibe-0.1.gbnf`, `grammar/source.schema.json`.
