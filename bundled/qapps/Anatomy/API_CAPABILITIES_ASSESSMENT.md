# API Capabilities Assessment - Qualia Anatomy

## Purpose

This note assesses what the repo currently supports for turning Anatomy into:

1. an online app that checks whether local Qualia is available,
2. a browser/WASM app that can run without native Qualia,
3. an installed app inside the Qualia Flutter desktop app,
4. and eventually a chat-driven representation component for local Qualia.

This assessment is based on the engine, daemon, App Vault, Flutter FRB, and chat
surfaces currently present in the repo. No engine changes were made here; this is
an anatomy-side planning update.

## Executive Summary

### Best near-term path

Build Anatomy as a layered app:

1. Static viewer first.
2. Add optional WASM enhancement.
3. Add App Vault installation and launch.
4. Add structured chat -> anatomy handoff after the desktop APIs are extended.

### Why

- The static viewer already works.
- Browser/WASM exports exist, but they are low-level and not a full app session API.
- Flutter can already install and launch local web apps.
- Flutter chat already performs local in-process inference.
- The missing piece is not rendering; it is the handoff contract and secure local app access model.

## Capability Matrix

| Capability | Online app | WASM app | Flutter App Vault app | Status |
|---|---|---|---|---|
| Render anatomy models | Yes | Yes | Yes | Available |
| Run with no local install | Yes | Yes | No | Available |
| Detect local Qualia daemon | Partial | Partial | Yes | Partial |
| Query local daemon securely | Partial | Partial | Partial | Blocked by auth/token flow |
| Local graph parsing/inspection | Limited | Limited | Limited | Partial |
| Local chat with model inference | No direct path | No direct path | Yes through Flutter chat | Available in Flutter only |
| Launch Anatomy from chat | No | No | Not yet wired | Missing |
| Pass structured representation payload into Anatomy | No | No | Not yet wired | Missing |
| Manifest-driven mode negotiation | Planned | Planned | Planned | Current manifest too small |

## What Exists Today

## 1. Browser/WASM surface

The engine already exposes browser-callable functions for:

- engine version lookup
- query compilation to JSON
- N-Triples query execution over provided `db_bytes`
- Turtle/N3/CBOR-LD/JSON parsing helpers
- several biomedical and chemistry utility functions
- SHACL validation helpers

What this means for Anatomy:

- Anatomy can become more than a static viewer in browser-only mode.
- Anatomy can use WASM for local parsing, validation, and some computation.
- Anatomy does not yet have a high-level browser API for:
  - opening a local user graph,
  - mutating a persistent graph session,
  - discovering a desktop daemon in a real way,
  - or streaming graph-backed anatomy data into the view.

### Assessment

WASM mode is viable for:

- local knowledge loading,
- lightweight reasoning and validation,
- browser-safe demo and offline experiences.

WASM mode is not yet a full replacement for desktop-local graph integration.

## 2. Native daemon surface

The native daemon already exposes:

- `GET /health`
- `POST /query`
- `POST /cache`
- `WS /qualia-bridge`
- CORS handling
- output format negotiation

Important caveats:

- authenticated query flow is still a problem for installed apps
- raw `q42` streaming is not implemented yet
- WebSocket handshake exists, but the interactive protocol is still effectively a stub for app purposes
- current query execution path is scaffolded rather than a complete live app data pipeline

### Online-app implications

An online-hosted Anatomy app could attempt:

- `GET /health` against `127.0.0.1`
- a capability banner like "local Qualia detected"

But robust upgrade into full local data mode is not complete because:

- auth token handoff to third-party/installed app pages is not complete,
- CORS is selective,
- and the active daemon port is not guaranteed from the browser side.

## 3. Flutter App Vault surface

Flutter already supports:

- listing installed apps
- installing apps from directories containing `app.json`
- launching installed apps
- running an internal loopback app server for local app assets
- opening localhost/loopback apps inside a WebView
- checking prerequisites
- starting the daemon

This is the strongest host environment for Anatomy.

### What it enables now

- Anatomy can be packaged as an installed app with `app.json`.
- Flutter can launch it in a WebView.
- Flutter can maintain the local daemon lifecycle.
- Flutter can host a chat experience separately.

### What it does not yet enable

- opening Anatomy as a parameterized chat component
- passing a representation payload into the web app at launch
- an explicit JS bridge for chat -> anatomy -> chat roundtrips
- a first-class capability negotiation handshake between host and app

## 4. Flutter chat surface

Flutter chat already supports:

- model-backed local inference
- full Webizen-gated orchestration
- local file ingestion
- streamed response display

This is an important distinction:

- local Qualia chat exists,
- but Anatomy is not yet part of that chat flow.

### Immediate architectural opportunity

The shortest route to "chat with local Qualia, then show Anatomy" is not to force
the web app to own chat. It is to let Flutter chat remain the chat host and launch
Anatomy with a structured payload when a visual representation is needed.

## Mode-by-Mode Assessment

## A. Online app that checks for local Qualia

### Viability

Partial.

### What can work

- show the static viewer immediately
- attempt a localhost health probe
- show "local Qualia detected" when successful
- offer a "continue with local context" action

### What blocks full success

- no stable browser-visible daemon port contract
- no reliable app/session token issuance flow to the remote page
- CORS/origin constraints for non-approved origins
- no stable browser-side API for "launch installed Anatomy in desktop mode"

### Recommendation

Treat online mode as:

- discovery,
- preview,
- and fallback mode,

not the primary path for deep personal/local graph integration.

## B. WASM-only app

### Viability

Good for fallback and demo. Partial for advanced graph features.

### Good fit for

- model viewing
- local demo knowledge
- parsing anatomy knowledge files
- basic validation
- selected local computations

### Not yet good fit for

- deep local graph querying against the user's installed Qualia data
- secure host/app coordination
- chat-driven orchestration

### Recommendation

This should be the mandatory fallback mode.

## C. Installed via Qualia Flutter desktop app

### Viability

Best strategic path, but still incomplete.

### Strong points

- install and launch are already conceptually supported
- WebView hosting already exists
- prerequisites and daemon lifecycle are desktop-controlled
- chat is already local and in-process

### Missing host features

- launch with structured props
- token/session propagation for graph queries
- explicit capability handshake
- component/deep-link routing from chat to app
- app -> host callback channel

### Recommendation

Use App Vault mode as the primary integrated target once the host-side gaps are addressed.

## Manifest Assessment

## Current manifest support

The current launcher only requires:

```json
{
  "name": "Anatomy",
  "version": "0.0.3-dev",
  "required_shapes": [],
  "dev_port": 5173
}
```

That is enough to install and launch an app, but not enough to describe:

- which execution modes the app supports
- whether the app has a WASM fallback
- whether a daemon is optional or required
- how chat integration works
- what representation payloads the app accepts
- whether the app wants to be launched as a full app, panel, or component

## Proposed direction

The new [app.json](C:\Projects\qualiaDB\app-development\Anatomy\app.json) keeps the
current required fields and adds an `x_qualia` extension block.

That extension block should evolve into a real manifest schema later.

### Fields the platform will likely need

- `launch_modes`
- `preferred_launch_mode`
- `supports_offline`
- `requires_daemon`
- `requires_wasm`
- `entrypoints`
- `capabilities`
- `representation_contract`
- `chat_integration`
- `host_bridges`
- `capability_probe`
- `auth`
- `ui_surfaces`

### Important note

The current installer/launcher will ignore these extension fields. They are
useful now for design and forward-compatibility, but they are not enforced yet.

## Proposed Chat -> Anatomy Contract

The cleanest contract is a structured payload, not raw prompt text.

Suggested shape:

```json
{
  "type": "anatomy.representation.request",
  "version": "1.0.0",
  "source": "qualia.flutter.chat",
  "subject_did": "did:qualia:user:...",
  "session_id": "chat-session-123",
  "title": "Cardiometabolic stress overview",
  "summary": "The current graph suggests elevated cardiometabolic burden.",
  "organs": [
    {
      "id": "heart",
      "label": "Heart",
      "state": "highlighted",
      "intensity": 0.82,
      "reasons": ["hypertension", "sleep deficit", "weight trend"]
    }
  ],
  "systems": [
    {
      "id": "cardiovascular",
      "label": "Cardiovascular system",
      "state": "active"
    }
  ],
  "explanations": [
    {
      "kind": "graph-summary",
      "label": "Why this is shown",
      "text": "Three recent measurements and two longitudinal factors contributed."
    }
  ],
  "actions": [
    {
      "id": "open-source-evidence",
      "label": "Show evidence"
    }
  ]
}
```

Why this is better:

- chat remains responsible for inference and grounding
- Anatomy remains responsible for spatial explanation and UI
- the contract can be reused by online, WASM, and desktop modes

## UX Direction

## 1. In chat

Add actions such as:

- `Show in Anatomy`
- `Open body view`
- `Explain this spatially`

## 2. In Anatomy

Add UI states for:

- `Demo mode`
- `WASM local mode`
- `Desktop local mode`
- `Chat-linked mode`

Add panels for:

- anatomical focus
- graph summary
- evidence/provenance
- selected conditions and modifiers
- system-wide burden overview

## 3. As a component

Anatomy should eventually support at least three surfaces:

- full-screen app
- side panel
- embedded card/component view

That implies host support for `surface=full|panel|embedded` or similar launch metadata.

## What Must Be Added Later

## Host/platform work

- real launch-time app/session token propagation
- exposed active daemon port for installed apps
- chat -> app launch API with structured arguments
- app -> host callback API
- first-class manifest schema and validation
- better localhost origin policy for installed apps

## Anatomy-side work

- representation payload parser
- state manager for mode detection
- fallback UX when daemon/auth are unavailable
- WASM integration against real exported functions
- anatomy overlay mapping from graph concepts to organs/systems

## Engine/API work

- higher-level browser-safe graph session API for WASM
- better installed-app auth model than today's partial flow
- stable daemon discovery contract
- richer query endpoints for app-level use

## Recommended Sequence Once Engine Work Is Clear

1. Keep Anatomy installable now with the current `app.json`.
2. Implement explicit mode detection in the app:
   - static
   - wasm
   - flutter-hosted
3. Add the representation payload parser and demo payloads first.
4. Extend Flutter chat to launch Anatomy with a payload.
5. Add secure daemon/app session handoff.
6. Only then wire live local graph queries into the web app.

## Bottom Line

Anatomy can already be positioned as a serious Qualia app, but today it is best
treated as:

- installable,
- render-capable,
- and future-ready,

rather than fully integrated.

The main blockers are not 3D or UI. They are:

- manifest expressiveness,
- host/app session handoff,
- daemon auth/discovery,
- and chat/component orchestration.
