# App Vault — Developer Guide

Apps in the App Vault are self-contained web applications that run in the user's
system browser and talk to the local Qualia daemon for semantic graph data.

---

## 1. Concepts

| Term | Meaning |
|---|---|
| **App Vault** | The in-app manager that installs, lists, and launches local web apps |
| **Qualia daemon** | Local HTTP/WebSocket server (`127.0.0.1:4242` by default) — the graph engine |
| **Semantic App Token** | A signed JWT-like token issued at launch that gates what graph shapes the app can query |
| **`required_shapes`** | SHACL shape namespaces declared in `app.json` — the app's data access scope |

The launch flow is:

```
User clicks Launch
  → Flutter calls launch_installed_app(appName)
  → Rust reads app.json, issues a Semantic App Token scoped to required_shapes
  → Returns a URL: file:///…/index.html?token=<token>  (or http://localhost:{dev_port}?token=…)
  → Flutter opens the URL in the default system browser
  → App reads the token from the URL and passes it in X-Qualia-Token on every daemon request
```

---

## 2. Directory Structure

Place your app directory inside `{storage_path}/Apps/`:

```
Apps/
└── my-app/
    ├── app.json          ← required manifest
    ├── index.html        ← entry point
    ├── app.js            ← your application code
    └── ...               ← any other static assets
```

`storage_path` defaults to `%APPDATA%\qualia` on Windows. The App Vault UI
shows any sub-directory of `Apps/` that contains an `app.json`.

---

## 3. The `app.json` Manifest

```jsonc
{
  "name": "My App",
  "version": "0.1.0",

  // Declare every SHACL shape namespace your app will query.
  // The daemon enforces this at the gatekeeper — queries outside these
  // namespaces are rejected with HTTP 403.
  "required_shapes": [
    "schema:HealthCondition",
    "qualia:BiometricRecord"
  ],

  // Optional. If set, Launch opens http://localhost:{dev_port}?token=…
  // instead of file:///. Use this when your app has a local dev server
  // (e.g. Vite, webpack-dev-server).
  "dev_port": 5173
}
```

### Field reference

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | ✅ | Human-readable app name |
| `version` | string | ✅ | Semver string |
| `required_shapes` | string[] | ✅ | SHACL shape namespaces the app is permitted to query. Use `[]` for read-only apps that query nothing. |
| `dev_port` | number | — | Local dev server port. Overrides `file://` launch. |

---

## 4. The Qualia Daemon API

The daemon runs on `http://127.0.0.1:4242` (port may differ if 4242 was in use —
the Flutter app picks the next available port up to 4300).

### `GET /health`

No auth required. Returns daemon version and status.

```json
{ "status": "ok", "mode": "native", "version": "0.0.6" }
```

### `POST /query`

Queries the semantic graph.

**Request headers:**

```
Content-Type: application/json
X-Qualia-Token: <token from URL param>
Accept: application/ld+json          ← default
        application/n-triples        ← optional
        application/x-qualia-q42     ← raw binary (future)
```

**Request body:**

```json
{
  "query": "SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 10",
  "format": "json-ld"
}
```

The `format` field overrides the `Accept` header. Valid values: `"json-ld"`,
`"n-triples"`, `"q42"`.

**Response (200 — `application/ld+json`):**

```json
{
  "@context": { "@vocab": "https://qualia-db.org/vocab#" },
  "@graph": [ ... ],
  "match_count": 5
}
```

**Response header:**

```
X-Qualia-Compute-Cost: 5+1024
```

Format is `{match_count}+{vm_cycles}` — useful for profiling.

**Error responses:**

| Status | Code | Cause |
|---|---|---|
| 401 | `unauthorized` | Missing or invalid `X-Qualia-Token` |
| 403 | `forbidden` | Query namespace not in `required_shapes` |
| 406 | `not_acceptable` | Unsupported `Accept` / `format` value |

### `POST /cache`

Upload a raw dataset shard (`.q42` binary) to the daemon's local cache.

```
POST /cache?filename=my_shard.q42
Content-Type: application/octet-stream
<binary body>
```

Returns `{ "status": "ok", "saved_to": "…/.qualia/cache/my_shard.q42" }`.

### `WS /qualia-bridge`

WebSocket handshake. On connect, the daemon sends:

```json
{ "type": "HANDSHAKE_SUCCESS", "payload": { "mode": "NATIVE", "version": "0.0.6" } }
```

Message handling is a stub — full bidirectional protocol is under development.

---

## 5. Authentication in Practice

The token is appended to the launch URL as `?token=<value>`. Read it on startup
and store it for the session:

```js
// index.html / app.js
const params = new URLSearchParams(window.location.search);
const token  = params.get('token') ?? '';

async function query(sparql) {
  const res = await fetch('http://127.0.0.1:4242/query', {
    method:  'POST',
    headers: {
      'Content-Type':    'application/json',
      'X-Qualia-Token':  token,
    },
    body: JSON.stringify({ query: sparql }),
  });
  if (!res.ok) throw new Error(`${res.status} ${await res.text()}`);
  return res.json();
}
```

---

## 6. CORS — The Critical Constraint

The daemon's CORS policy **only allows these origins in production**:

```
https://mediaprophet.github.io
```

In **dev mode** (daemon started with `dev: true`) it additionally allows:

```
http://localhost:8788
http://127.0.0.1:8788
http://localhost:5173
http://127.0.0.1:5173
```

**Implications for apps:**

| Launch method | Origin seen by daemon | Works? |
|---|---|---|
| `file:///` (no `dev_port`) | `null` | ❌ Blocked in production mode |
| `dev_port: 5173` | `http://localhost:5173` | ✅ In dev mode |
| `dev_port: 8788` | `http://localhost:8788` | ✅ In dev mode |
| Any other port | `http://localhost:{n}` | ❌ Blocked |

**Current recommended workflow:**

Use `dev_port: 5173` (Vite default) or `dev_port: 8788` and start the daemon
in dev mode. A future release will broaden CORS to allow any `localhost` origin
for installed apps.

---

## 7. Shape Enforcement

The gatekeeper does namespace substring matching. If your `required_shapes`
includes `"schema:HealthCondition"`, any query containing the substring `schema`
passes. Queries whose namespace doesn't appear in any declared shape are rejected
with HTTP 403.

Declare shapes conservatively — only what the app actually needs.

---

## 8. Example: WellFair `app.json`

```json
{
  "name": "WellFair",
  "version": "0.0.3",
  "required_shapes": [
    "schema:HealthCondition",
    "schema:MedicalRecord",
    "qualia:BiometricRecord",
    "qualia:SleepRecord"
  ],
  "dev_port": 5173
}
```

---

## 9. Development Workflow

1. Create `{storage_path}/Apps/my-app/` with `app.json` and `index.html`.
2. Open the App Vault screen in QualiaDB.
3. Click **Install Package** and select your app directory.
4. Click **Launch** — the browser opens with the token in the URL.
5. Use the `query()` helper above to hit the daemon.
6. Iterate on your HTML/JS; refresh the browser to reload.

For a dev server workflow (`dev_port`):

```bash
# Terminal 1 — your app's dev server
cd my-app && npx vite --port 5173

# Terminal 2 — QualiaDB with daemon in dev mode
# (dev mode is set automatically when the desktop app is not in release build)
```

---

## 10. Notes for AI Agents

When helping build a Qualia App Vault application:

- The daemon is **not** a general-purpose database. It's a semantic graph engine.
  Queries are SPARQL-style strings interpreted by the Qualia VM.
- Do **not** suggest Ollama, llama.cpp, or any external LLM server. Inference
  runs in-process via `LocalLlmAgent` — apps access it only through the graph.
- The `required_shapes` list is a security boundary, not just metadata. Omitting
  a shape means queries using that namespace will be rejected at runtime.
- The WebSocket `/qualia-bridge` endpoint handshakes successfully but message
  handling is a stub. Do not build features that depend on WS push in this release.
- `file://` launch is blocked by CORS in the current production daemon. Recommend
  `dev_port: 5173` or `dev_port: 8788` for any app under active development.
- The token expires with the session. Apps should not persist it to localStorage.
