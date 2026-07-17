# Native Vision Capability Excellence — Progress Log

**Plan:** `native-vision-capability-excellence-2026.md`  
**Branch:** `0.0.25`

---

## 2026-07-17 — VX0 + VX1 + VXB core + VXP + VX3D ship

**Status:** done (agent-completable core); COMPLETE-WITH-GATE / Missing rows remain honest in registry

### Built

| Area | Layout |
|------|--------|
| **Registry** | `capability/{status,entry,registry}.rs` — D1–D9 rows |
| **cv/** | buffer, color, filter×3, morph×2, edges×2, hist×2, contours, transform×2, features×3, flow, photo, draw — **one function per file** |
| **biosense/** | consent, quality×3, rppg×4, magnification×2, face ROI, affect proposal, template hash, policy/CCTV, respiration |
| **spatial** | `export_stl.rs`, `print_readiness.rs` |
| **recipes/** | `self_monitor_pulse.rs` |

### Tests

`cargo test -p qualia-vision --lib` → **71 passed**, 0 failed

### Monolith check

New algorithm files are single-function modules under subdirs; `mod.rs` files are wiring only.

### Honesty / gates still open

- Licensed face mesh / production embeddings / clinical rPPG corpus  
- Full SPARQL-FED multi-camera  
- Photogrammetry multi-view  
- Video file I/O, stitch  
- WASM edge profile declaration  
- Studio full workbench wiring for every op  

### Next

Wire Studio/desktop commands; deepen face mesh weights; FED policy; multi-view recon — or principal dogfood this core.
