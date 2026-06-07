# Anatomy

Qualia Anatomy is a local-first anatomy visualization app for QualiaDB. It uses
Human Reference Atlas `.glb` models for the body layer and is intended to become
the representation surface for health graph reasoning, personal biometrics, and
chat-driven explanation.

## What This Directory Now Covers

This directory is now documented for three deployment targets:

1. Online app that can probe for a local Qualia install and adapt if present.
2. Pure browser/WASM app that still works when no native install exists.
3. Installed app inside the Qualia Flutter desktop App Vault.

The current repo-backed capability assessment for those modes lives in
[API_CAPABILITIES_ASSESSMENT.md](C:\Projects\qualiaDB\app-development\Anatomy\API_CAPABILITIES_ASSESSMENT.md).

## Current Status

### Works today

- Static Babylon.js anatomy viewer in [index.html](C:\Projects\qualiaDB\app-development\Anatomy\index.html)
- Local helper scaffold in [qualia.js](C:\Projects\qualiaDB\app-development\Anatomy\qualia.js)
- Knowledge assets in [Knowledge](C:\Projects\qualiaDB\app-development\Anatomy\Knowledge)
- Installable App Vault manifest in [app.json](C:\Projects\qualiaDB\app-development\Anatomy\app.json)

### Partially available

- Browser-side WASM use of Qualia engine exports
- Flutter App Vault install and launch flow
- Flutter local chat with in-process model inference
- Qualia daemon health/query surface

### Not complete yet

- Secure app token handoff from App Vault launch to web app
- Reliable browser-side discovery of the active daemon port
- Stable JS bridge between Flutter chat and the Anatomy web surface
- A high-level representation contract for passing graph summaries into anatomy UI
- A richer manifest schema that the installer/launcher actually enforces

## Deployment Modes

| Mode | Goal | Current reality |
|---|---|---|
| Standalone static app | Open the viewer with no Qualia dependency | Works now |
| Standalone WASM app | Run reasoning/parsing in-browser | Partially works; low-level exports exist, high-level graph session API does not |
| Online app + local probe | Remote-hosted app checks for local Qualia and upgrades if found | Health check is plausible; authenticated graph access is not complete |
| Flutter App Vault app | Install, launch, and embed inside Qualia desktop | Install/launch works; daemon/chat wiring still needs follow-up |
| Chat-driven anatomy component | User chats with local Qualia, then sees anatomy representations | Chat exists; component handoff contract does not yet exist |

## Recommended Product Shape

The strongest path is a layered experience:

1. Baseline mode: static anatomy viewer with local demo knowledge.
2. WASM mode: add parsing, validation, and lightweight local reasoning with browser-safe exports.
3. Desktop mode: when launched from Qualia Flutter, receive user context, chat summaries, and local graph-derived representation payloads.

That lets the app remain useful without a daemon, while still becoming much more
powerful when local Qualia capabilities are present.

## Manifest Direction

The current App Vault only requires:

- `name`
- `version`
- `required_shapes`
- optional `dev_port`

This directory now includes a real [app.json](C:\Projects\qualiaDB\app-development\Anatomy\app.json)
using those fields, plus an `x_qualia` extension block for forward-looking
capabilities. The extra fields are for planning and documentation today; they are
not yet consumed by the current installer/launcher.

## Chat + Representation Direction

The intended interaction model is:

1. User chats with their local Qualia install.
2. Chat/orchestrator produces a structured representation payload.
3. Anatomy launches or focuses as a component.
4. Anatomy renders organs, systems, overlays, and explanatory cards from that payload.

The proposed payload contract and required API work are documented in
[API_CAPABILITIES_ASSESSMENT.md](C:\Projects\qualiaDB\app-development\Anatomy\API_CAPABILITIES_ASSESSMENT.md).

## Files

```text
Anatomy/
|-- API_CAPABILITIES_ASSESSMENT.md
|-- README.md
|-- TODO.md
|-- app.json
|-- index.html
|-- qualia.js
`-- Knowledge/
    |-- conditions.ttl
    `-- shapes.shacl
```

## Immediate Next Work

See [TODO.md](C:\Projects\qualiaDB\app-development\Anatomy\TODO.md) for the task
list that follows from the current API assessment.
