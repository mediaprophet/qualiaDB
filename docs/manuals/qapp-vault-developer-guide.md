# Qapp Vault Developer Guide

Qapps in the Qapp Vault are local web applications stored under the user's
Qualia data directory. The desktop shell can install them from a local
directory, launch them from a local dev server, serve them through the embedded
Qualia loopback asset server, or fall back to `file:///` when needed.

---

## 1. Concepts

| Term | Meaning |
|---|---|
| **Qapp Vault** | Flutter desktop UI (`QappVaultScreen`) for installing, listing, and launching local qapps |
| **Qualia daemon** | The local graph engine exposed over HTTP/WebSocket |
| **`qapp.json`** | The required manifest file discovered by the installer |
| **`required_shapes`** | Declared semantic domains the qapp expects to work with |
| **`x_qualia`** | Optional host metadata for launch modes, entrypoints, chat handoff, and representation contracts |

Current launch flow:

```text
User clicks Launch
  -> Shell calls launch_installed_qapp(qappName)
  -> Rust reads qapp.json and resolves the entrypoint
  -> If dev_port exists, returns http://localhost:{dev_port}/...
  -> Else, if the embedded Qualia asset server is running, returns http://127.0.0.1:{port}/{qapp}/...
  -> Else, falls back to file:///.../index.html
  -> Flutter opens localhost/127.0.0.1 URLs in QualiaQappWebView (embedded WebView)
```

For host-driven launches such as chat handoff, the shell can call
`launch_installed_qapp_with_context(...)` and append:

- `qualia_source`
- `qualia_surface`
- `qualia_payload`
- `qualia_qapp`

---

## 2. Directory Structure

Place your qapp directory inside `{storage_path}/Qapps/`:

```text
Qapps/
└── my-qapp/
    ├── qapp.json
    ├── index.html
    ├── app.js
    └── ...
```

Any subdirectory containing `qapp.json` is a candidate qapp.

---

## 3. The qapp.json Manifest

Minimum supported manifest:

```json
{
  "name": "My Qapp",
  "version": "0.1.0",
  "required_shapes": [
    "schema:HealthCondition",
    "qualia:BiometricRecord"
  ]
}
```

Common development manifest:

```json
{
  "name": "My Qapp",
  "version": "0.1.0",
  "required_shapes": [
    "schema:HealthCondition",
    "qualia:BiometricRecord"
  ],
  "dev_port": 5173,
  "x_qualia": {
    "required_ontologies": [
      "snomedct-us"
    ],
    "optional_remote_endpoints": [
      "wikidata-main"
    ],
    "required_models": [
      "phi-3-mini-4k-instruct-q4km"
    ],
    "entrypoints": {
      "web": "index.html",
      "representation": "index.html#representation"
    },
    "chat_integration": {
      "supports_launch_from_chat": true
    },
    "ui_surfaces": [
      "full-screen",
      "panel",
      "embedded"
    ]
  }
}
```

### Field reference

| Field | Type | Required | Notes |
|---|---|---|---|
| `name` | string | yes | Human-readable name and directory launch key |
| `version` | string | yes | Qapp version string |
| `required_shapes` | string[] | yes | Declared semantic domains/shapes |
| `dev_port` | number | no | Launch through `http://localhost:{dev_port}` |
| `x_qualia` | object | no | Optional host metadata understood by the current launcher |

### Current `x_qualia` support

The launcher reads named entrypoints from `x_qualia.entrypoints`. This
allows calls such as:

- `launch_installed_qapp("Anatomy")`
- `launch_installed_qapp_with_context("Anatomy", "representation", "panel", payload, source)`

The host also inspects these optional requirement lists when generating a qapp
readiness report:

- `x_qualia.required_ontologies`
- `x_qualia.optional_remote_endpoints`
- `x_qualia.required_models`

Unknown keys in `x_qualia` are ignored by the current host.

---

## 4. Local Daemon API

The daemon runs on localhost and currently exposes at least:

- `GET /health`
- `POST /query`
- `WS /qualia-bridge`

Example query request:

```json
{
  "query": "SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 10",
  "format": "json-ld"
}
```

The current Qapp Vault launch path issues qapp session tokens when the daemon
requires auth. Treat `required_shapes` as important metadata and future policy
input for gatekeeper boundaries.

---

## 5. Launch Context in Practice

Qapps that want to support host-driven UX should read launch context from the URL:

```js
const params = new URLSearchParams(window.location.search);
const source = params.get("qualia_source");
const surface = params.get("qualia_surface");
const payload = params.get("qualia_payload");
const qappName = params.get("qualia_qapp");
const context = payload ? JSON.parse(payload) : null;
```

Typical uses:

- opening a specific representation view from local chat
- restoring which UI surface the host requested
- receiving a compact summary or graph card from another Qualia component

---

## 6. Dev Workflow

1. Create `{storage_path}/Qapps/my-qapp/` with `qapp.json` and your web assets.
2. Install the directory from the Qapp Vault screen.
3. Launch the qapp.
4. If you are developing with a live server, set `dev_port` and run your qapp locally.
5. If you need host-driven routing, define `x_qualia.entrypoints`.

---

## 7. Notes for AI Agents

- Prefer the embedded loopback asset server or a localhost dev server over raw `file:///` when browser behavior matters.
- Treat `required_shapes` as an important contract for future gatekeeper policy.
- Use `x_qualia.entrypoints` for named surfaces such as `representation` or `chat_component`.
- Use `launch_installed_qapp_with_context(...)` when the host needs to pass structured state into the qapp.
- Do not rely on `/qualia-bridge` for rich realtime features yet; the handshake exists, but the protocol is still minimal.
