# POET UI — remaining backend gaps

**Date:** 2026-08-28  
**Scope:** standalone `crates/poet` after live-invoke wiring.  
**Rule:** the UI must perform the advertised operation or sit disabled with an exact prerequisite. This file lists work that is **not** fakeable from the browser and is larger than a session-sized host bind.

Implemented in this pass (so it is *not* a gap):

- COP `/records` persist/query/delete for project, dataset, ontology, studio, health, governance, device, social, finance, Aura, WebRTC, vision, listen, triad, portal, webview, settings, and `pulse_event`.
- `Pulse.*` invoke now publishes on the process-wide `pulse_transport` (SSE/WebSocket subscribers) and persists a `pulse_event` COP record when the ledger is configured. Bare channel names are rewritten to `poet/{channel}` so they match the 0.1 allowlist (`poet/`, `pulse/`, `clinic/`).
- Live host buttons merge nearby COP form fields into invoke args (typed numbers/bools, `sex` → `sex_female`, `sys_bp` → `systolic_bp`, modality lowercased).
- Health overview runs `ClinicalRisk.cha2ds2_vasc` and `ClinicalRisk.framingham` from entered vitals.
- Anatomy runs `MedicalImaging.hu_window` on an explicit 2×2 demo slice, not an empty payload.
- Pulse and generic ontology containers no longer show canned event/class trees.

---

## Large / out-of-session backend work

These are real products, not missing click handlers.

| Gap | Why it is large | Honest UI state until it exists |
|-----|-----------------|---------------------------------|
| **Binary PDF / scan ingest** | Health documents take *extracted text*. `nlp.analyze` + gazetteer + `Document.ingest` + classified Semantic Library ingest are live. Decoding PDF objects, embedded images, and OCR of scans is a document-pipeline project (codecs, page raster, OCR model, attestation). | Paste extracted text. Binary PDF decode is unbound. |
| **WebRTC signaling host** | Session records persist (`webrtc_session`, `device`). A live `RTCDataChannel` needs a signaling server, ICE/TURN, DID-authenticated offer/answer, and consent. That is a net crate, not a button. | Pulse.publish_sync works. Peer media does not. |
| **Microphone / camera capture** | Browser `getUserMedia` + host permission session + bounded audio/video buffers crossing the WASM/native ABI. Listen/vision currently run `Audio.oscillator` and `ComputerVision.ahash` on **demo** buffers so the capability itself is proven. | Mic/cam instrument tools stay disabled: “needs an explicit device-permission session.” |
| **ILP / Lightning settlement** | Wallet COP records and `FinancialModeling.gbm_var` are live. Interledger / Lightning rails need a registered payment session, keys, and fail-closed metering. | Amounts persist. Settlement is unbound until a rail session is registered. |
| **Real DICOM / imaging studies** | `MedicalImaging.hu_window` is a bounded HU kernel. Loading a study from disk (DICOMDIR, transfer syntaxes, multi-slice, PHI gates) is a medical-imaging store. | Demo 2×2 slice only; paste a real `pixels` array to window a study. |
| **Computer vision on camera frames** | `ComputerVision.ahash` (and blur/sobel/canny) run on caller-supplied gray bytes. A live camera → 8×8 buffer path is capture + permission, not the kernel. | Vision jobs queue a source URI; detections are not fabricated. |
| **Remote DID signing** | SPARQL/query layer holds no private keys (`SparqlDidHandler::sign_with_did` fails closed). Signing belongs in the identity/key-vault session. | Rights “Sign” evaluates deontic norms. It does not emit an Ed25519/ML-DSA signature. |
| **Multi-window OS display layout** | Display records persist. Actually placing OS windows needs platform windowing APIs (Win32 / AppKit / Wayland) outside the WASM UI. | Layouts save. They do not move other windows. |
| **Live CRDT over a data channel** | Workspace-sync notes persist; `Pulse.publish_sync` emits. The existing CRDT resolver is a graph primitive, not a peer mesh. | Sync records save. They do not merge a remote replica until a channel is paired. |
| **Nym / mixnet Pulse delivery** | Local process transport + COP log are now real. Delivering pulses to another machine through Nym (or any mixnet) is the net/privacy stack. | Local daemon subscribers see the event. Remote peers do not. |
| **Legacy QApp structure** | **Next sprint: refactor into native POET UI; do not build a QApp runtime.** Every former QApp is classified as a construct, manifold, typed container, existing POET surface/capability, Library Software stub, or obsolete duplicate. | Follow section K of the parity tracker and ADR 0012. Construct shelf, nested-manifold navigation, normal capability/COP/Semantic Library paths, and checksummed HCF/HMC packages are the destination. |

---

## Small follow-ups (not blockers for UI function)

1. **Chrome UAT** — the final app and daemon are running and startup/Search were verified. Dismiss Chrome's open extension popup, then complete canvas gestures and specialist-surface checks in the retained POET UAT tab. Not a code gap.
2. ~~**Pulse SSE surface in poet-ui**~~ — completed: the existing shared daemon EventSource fans live `/pulse/events` receipts into mounted Pulse containers and refreshes their COP ledger views.
3. **Sheet P64 / chart** — `Sheet.sum_range` and `Sheet.stats` are live. EnCodec P64 latents and chart glyphs need a typed render contract.
4. **WebView navigation** — COP URI records + `Document.ingest` are live. A sandboxed Chromium/WebView session is the Classic-shell leftover, not POET’s destination path.
5. ~~**HCF/HMC construct export**~~ — completed: construct metadata plus authored lenses export as verified, checksummed CBOR HCF/HMC packages from the Construct Shelf.
6. ~~**COP `poet_subject` rows**~~ — completed: declaration retains local `SubjectSeed` persistence and mirrors the authored subject to the allowlisted COP family whenever the daemon is connected.
7. **Live peer presence mesh** — `Pulse.publish_presence` and the participant roster are real. A remote replica joining the same social manifold still needs a signaling/session channel (listed under large backends).

---

## Capability IDs the UI already calls

`Pulse.publish` / `publish_notification` / `publish_telemetry` / `publish_presence` / `publish_sync` / `publish_agent_message` / `open_channel` · `CapabilityDiscovery.list` · `DeonticLogic.evaluate` · `LegalLogic.compute` · `FinancialModeling.gbm_var` · `SHACL.extensions` · `GraphDatabase.stats` · `GraphAuthoring.process` · `Document.ingest` · `nlp.analyze` · gazetteer · Semantic Library ingest (classified/secret) · `ComputerVision.ahash` · `Audio.oscillator` / `Audio.transport` · `Scene.create` · `Render.gpu_adapter_info` · `ClinicalRisk.cha2ds2_vasc` / `framingham` · `MedicalImaging.hu_window` · `Sheet.sum_range` / `Sheet.stats`.
