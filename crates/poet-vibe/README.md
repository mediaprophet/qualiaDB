# poet-vibe

Poet engine for **VibeScript `vibe-0.1`**.

Copyright © 2026 Timothy Charles Holborn.

Normative spec: `docs/manuals/standards/vibescript-core.md`.

No JIT. No wgpu. No raw Quin overlay literals. Scripts call capabilities; the host seals Quin parity.

Agent loop: `poet_vibe::diagnose(src)` → JSON (`error_code`, `span`, `suggested_fix`).
Grammar: `grammar/vibe-0.1.ebnf`, `grammar/vibe-0.1.gbnf`, `grammar/source.schema.json`.
