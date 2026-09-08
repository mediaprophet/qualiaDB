# 0.0.37 GitHub Pages / WASM size gates

Measured with `node docs/tests/wasm-size-check.mjs` (raw bytes + `zlib.gzipSync`).

The previous portal gate was **2.6 MiB raw / 800 KiB gzip**. The full
WASM-safe portal and playground binaries **fail** that gate:

| Artifact | raw | gzip | Old 2.6 MiB / 800 KiB | Pages / release-wasm 16 MiB / 4 MiB |
| --- | ---: | ---: | --- | --- |
| Ontology MCP `webizen_lite_wasm_bg.wasm` | 540,847 (0.52 MiB) | 165,726 (162 KiB) | n/a (separate 640/200 KiB gate) | n/a |
| Portal `qualia_core_db_bg.wasm` (`--features portal`) | 8,075,313 (7.70 MiB) | 2,256,664 (2.15 MiB) | FAIL | OK |
| Playground `qualia_core_db_bg.wasm` (`--features wasm-full`) | 8,630,624 (8.23 MiB) | 2,431,169 (2.32 MiB) | FAIL | OK |

CI gates (keep `pages.yml` and `release-wasm.yml` in step):

| Product | max raw | max gzip |
| --- | ---: | ---: |
| Ontology MCP | 655,360 (640 KiB) | 204,800 (200 KiB) |
| Portal | 16,777,216 (16 MiB) | 4,194,304 (4 MiB) |
| Playground (`wasm-full`) | 16,777,216 (16 MiB) | 4,194,304 (4 MiB) |

Portal / playground numbers are a **sanity cap** for the full WASM-safe
engine, not a slim viewport budget. Ontology MCP stays the tight product.
Do not fold science or LLM into `wasm-ontology` to keep Pages green.

`docs/pkg/qualia/.gitignore` ignores the wasm blob; GitHub Pages CI
rebuilds it with `wasm-pack` and then runs the size check. A stale local
`docs/pkg/qualia/qualia_bg.wasm` (~1.88 MiB) is not what Pages ships.
