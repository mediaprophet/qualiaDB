# Vision / 10D Browser UI

**Status:** stub (U4-A)  
**Surface:** webizen-studio `TenDBrowser` (`components/ten_d_browser.rs`)  
**Plan:** [`docs/plans/webizen-ui-implementation-subagents-2026.md`](../plans/webizen-ui-implementation-subagents-2026.md) §5 U4-A

## What this pane does

| Action | Host command | Notes |
|--------|--------------|--------|
| List all .10d | `browse_10d_containers` | Categories: Anatomy assets, Vision reconstruction, User library, Other |
| Vision recon only | `browse_vision_10d` | Under `{storage}/vision_geometry/` |
| Inspect | `inspect_10d_container` | Header, sections, CRC, mesh, provenance |
| Open file | `open_10d_file_picker` | Manual path |
| Load vision | `load_vision_10d` | Mesh + node paint package |
| Temporal scrub | `scrub_vision_10d_paint` | `t_slice` / `t_window` |

## Citable mode (fail-closed)

When **Citable (require provenance)** is on:

- Access policy = `Vision10dAccess::CitableRequireProvenance`
- Missing / invalid ProvenanceSidecar → host `Err` (barrier Deny)
- UI shows **visible error text** (banner + inline under Load/Scrub)
- Prior load/scrub success is cleared so the UI never looks “loaded” after Forbid

Browse without citable may allow unattested recon (local debug).

## Honesty

- **Partial** — list/inspect/load/scrub wired; not a full volumetric portal product cut
- **Ready** — citable Forbid fails closed with visible error

## Empty states

- No containers → plain language + next actions (build packs, seal recon, Open file, Refresh)
- Vision filter empty → `vision_geometry/` guidance + clear filter
- No selection → select left list or Open file

## Non-goals (this track)

- No new Tauri commands (U4-A commands lock: NO)
- No second GPU stack, no audio, no MCP tool loop (U3 / U4-B)

---

## Vision workbench (U4-B)

**Status:** product honesty cut  
**Surface:** webizen-studio `VisionWorkbench` (`components/vision_workbench.rs`)  
**Route:** `/vision` (`VisionRoute`)

### Sections

| Section | Honesty | Notes |
|---------|---------|--------|
| (a) Synthetic detect demo | **Partial** | Existing host path (`vision_run_synthetic_demo`, generate, Image→3D, G→S, QVWT). Empty state until run. WASM without desktop host → plain offline error. |
| (b) SR / device policy | **Partial** | Engine classical SR + thermal/VRAM policy Present; Studio is **status text only** (no live SR control). Learned / disk QVWT → **Needs model**. |
| (c) Recon load/scrub | **Partial** | Link to **10D Browser** (U4-A). Do not duplicate load/scrub here. |
| (d) Biosense / self-monitor | **Scaffold** + **Needs consent** | Checkbox required. Button disabled until consent. No host biosense command yet → Scaffold message; **never** claims a successful biometric run. |

### Consent rule

- Label: “I consent to process my biometrics on-device”
- No silent camera / rPPG / HR path from this pane
- Session checkbox only until a consent-gated host command exists

### Related host commands (detect / continuum — already present)

| Action | Host command |
|--------|----------------|
| Synthetic demo | `vision_run_synthetic_demo` |
| Generate | `vision_generate_image` |
| Image→3D | `vision_image_to_3d_demo` |
| G→S continuum | `vision_gs_continuum` |
| Ensure weights | `vision_ensure_weights` |
| Disk QVWT detect | `vision_detect_disk_weights_demo` |

### Non-goals (U4-B)

- No new Tauri commands (commands lock: NO)
- No rewrite of detector pipeline
- No audio
- No changes to `ten_d_browser.rs`
