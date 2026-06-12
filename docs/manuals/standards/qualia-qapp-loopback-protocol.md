# Qualia Qapp Loopback Protocol

**Status:** Internal explainer / code-aligned draft  
**Updated:** 2026-06-12

This document describes the current localhost serving boundary for installed Qualia Apps (Qapps) on desktop hosts.

It focuses on the host-side asset protocol implemented in:

- [crates/qualia-client-core/src/qapps_protocol.rs](/C:/Projects/qualiaDB/crates/qualia-client-core/src/qapps_protocol.rs:1)
- [crates/qualia-client-core/src/api.rs](/C:/Projects/qualiaDB/crates/qualia-client-core/src/api.rs:2805)
- [crates/qualia-client-core/src/qapp_registry.rs](/C:/Projects/qualiaDB/crates/qualia-client-core/src/qapp_registry.rs:5)

## 1. Scope

There are two separate manifest layers in the current system:

- `qapp.json`
  The package / host manifest for an installed Qapp. This is what the desktop host uses for launch metadata, entrypoints, daemon integration hints, capability declarations, ontologies, models, and UI surfaces.
- `yaml-ld-q42`
  The declarative Webizen workspace / page-layout manifest format. This remains first-class for workspace and studio layout definitions, but it is not the file the desktop Qapp launcher currently reads when launching an installed Qapp package.

This document is about the first layer: the loopback protocol used to launch and host installed Qapps described by `qapp.json`.

## 2. Current Host Model

The current desktop host does not dynamically unpack a `.qapp.zip` archive into an in-memory sandbox on each launch.

Instead, it serves files from the installed Qapp directory:

```text
{storage_path}/Qapps/{QappName}/
```

The package manifest is expected at:

```text
{storage_path}/Qapps/{QappName}/qapp.json
```

The loopback asset server is separate from the main Qualia daemon. It is a lightweight localhost HTTP server intended to avoid `file://` CORS issues for embedded WebViews and browser launches.

## 3. Asset Server

### 3.1 Binding

- Host: `127.0.0.1`
- Port: first open port in the range `4567..4600`
- Lifetime: started once per process, then reused

If already started, `start_qualia_protocol()` returns the existing port.

### 3.2 Root

The HTTP root is the installed Qapps directory:

```text
{storage_path}/Qapps/
```

Each app is then served under its directory name:

```text
http://127.0.0.1:{port}/{qapp}/index.html
```

### 3.3 Path Safety

The server strips leading `/`, rejects path traversal, and only permits descendants of the Qapps root. Invalid paths return `403`. Missing files return `404`.

### 3.4 MIME Types

The host currently recognizes common asset types including:

- `.html`
- `.js`
- `.css`
- `.json`
- `.png`
- `.jpg` / `.jpeg`
- `.svg`
- `.wasm`
- `.woff` / `.woff2`

## 4. Package Manifest Role

Installed Qapps are launched from `qapp.json`, not from a `manifest.json` inside a transient archive.

Important fields in `qapp.json` include:

- `name`
- `version`
- `required_shapes`
- `dev_port`
- `x_qualia.entrypoints`
- `x_qualia.requires`
- `x_qualia.required_ontologies`
- `x_qualia.required_models`
- `x_qualia.local_daemon`
- `x_qualia.chat_integration`
- `x_qualia.ui_surfaces`

The host manifest type is defined in [crates/qualia-client-core/src/qapp_registry.rs](/C:/Projects/qualiaDB/crates/qualia-client-core/src/qapp_registry.rs:189).

## 5. Launch Resolution

When the host launches an installed Qapp:

1. It locates `{storage}/Qapps/{QappName}`.
2. It loads `qapp.json`.
3. It resolves the requested entrypoint from `x_qualia.entrypoints`.
4. It splits any `#fragment` from the asset path.
5. It chooses one of these base URLs:

- Dev mode:
  `http://localhost:{dev_port}` or `http://localhost:{dev_port}/{asset}`
- Installed loopback mode:
  `http://127.0.0.1:{qualia_protocol_port}/{qapp}/{asset}`
- Fallback mode:
  `file:///...` if the loopback asset server was not started

6. It appends launch context as query parameters.
7. It restores the original hash fragment if present.

## 6. Launch Context Parameters

The current host appends launch context in the URL query string.

Supported parameters include:

- `qualia_source`
- `qualia_surface`
- `qualia_payload`
- `qualia_token`
- `qualia_daemon_port`
- `qualia_qapp`

### 6.1 Token Handling

When a Qapp name is present, the host attempts to mint a session token using:

`issue_qapp_session_token(qapp_name)`

That token is derived from the installed app identity and its `required_shapes`, then appended as:

```text
qualia_token=...
```

### 6.2 Daemon Hint

If the main Qualia daemon is active, its port is appended as:

```text
qualia_daemon_port=4242
```

This is a launch hint for the app. The loopback asset server itself is not the main daemon.

## 7. Daemon Relationship

The desktop system currently has two related localhost surfaces:

- Main Qualia daemon
  Handles graph query, health, and broader engine APIs such as `/health` and `/query`
- Qapp asset server
  Serves installed Qapp static assets from the Qapps directory

An app may use only static assets, or it may also declare daemon integration in `x_qualia.local_daemon`.

That means:

- asset serving and daemon RPC are related but distinct
- the asset server does not itself enforce the full daemon capability model
- readiness for daemon-aware apps is assessed separately by host-side checks

## 8. Readiness Checks

The desktop host already has a launch-readiness inspection path for installed Qapps.

It evaluates:

- declared capabilities
- whether the local daemon is running
- required ontologies
- required models
- optional remote SPARQL endpoints

The result is returned as a JSON readiness report by:

`inspect_installed_qapp_readiness(qapp_name)`

This makes the loopback launch flow more than simple static file serving; it is tied to host resource awareness.

## 9. Trust Boundary Notes

### 9.1 What the loopback server currently does

- serves only from the installed Qapps directory
- rejects path traversal
- provides localhost-only asset access
- helps avoid `file://` WebView restrictions

### 9.2 What it does not currently do by itself

- verify live daemon capabilities on every asset request
- cryptographically validate each request at the HTTP asset layer
- serve each app from isolated in-memory blobs
- replace the main daemon's governance and fiduciary controls

Those controls live elsewhere in the desktop host and daemon stack.

## 10. URI Handler

There is currently a `qualia:` URI handler registration path on Windows via the registry.

On non-Windows targets, the registration function is presently a no-op.

This means protocol-deep-launch behavior is currently strongest on Windows and softer on other targets unless the host app directly launches the computed loopback URL.

## 11. Current Terminology Correction

Older docs may imply the following flow:

1. unpack `.qapp.zip`
2. verify `signature.sig`
3. serve a generated in-memory WASM payload
4. rely on an embedded `manifest.json`

That is not the current host implementation described by the code above.

Today the effective desktop flow is:

1. install / locate Qapp under `{storage}/Qapps/{Name}`
2. read `qapp.json`
3. resolve entrypoint
4. serve files over localhost loopback if the Qapp asset server is active
5. append launch token and daemon context parameters
6. optionally use the main daemon for graph/query/integration work

## 12. Relationship to `yaml-ld-q42`

`yaml-ld-q42` remains important and should not be collapsed into the package-manifest story.

Use this split:

- `qapp.json`
  Package metadata, launch metadata, capabilities, host integration, readiness inputs
- `yaml-ld-q42`
  Declarative workspace / Webizen layout / page graph definitions

If a future packaging format embeds both, this document should be updated to describe that composition explicitly rather than conflating the two layers.
