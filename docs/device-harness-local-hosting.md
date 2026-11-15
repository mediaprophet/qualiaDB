# Run the device harness on your local network

Use a freshly built checkout or an already built Pages artifact containing the
matching harness assets and provenance manifest. Tracked WASM files alone do not
establish that the current source changes have been compiled. From the QualiaDB
checkout, use the dedicated diagnostics build from `.github/workflows/pages.yml`. Capture
source identity before generating assets, then generate the manifest after the
build:

```powershell
node scripts/device-harness-build.mjs --capture-source target/device-harness-source.json
$env:RUSTFLAGS = '-C target-feature=+simd128 --cfg getrandom_backend="wasm_js"'
wasm-pack build crates/qualia-core-db --target web --release --out-dir pkg-device-harness --out-name qualia -- --no-default-features --features device-harness
New-Item -ItemType Directory -Force docs/pkg/device-harness | Out-Null
Copy-Item crates/qualia-core-db/pkg-device-harness/qualia.js docs/pkg/device-harness/qualia.js
Copy-Item crates/qualia-core-db/pkg-device-harness/qualia_bg.wasm docs/pkg/device-harness/qualia_bg.wasm
node scripts/device-harness-build.mjs --source-record target/device-harness-source.json
```

These PowerShell commands require Rust's `wasm32-unknown-unknown` target,
and `wasm-pack`. Restore your previous
`RUSTFLAGS` after the build if you normally use different flags. The Pages
workflow also builds other site assets; a complete site artifact includes those
separate products.

For local development with uncommitted source, add `--allow-dirty` to the capture
command. The resulting manifest explicitly identifies a development checkout;
it is not qualification of a clean commit. The full reviewed Etalon revision in
`tools/accelerator-harness/etalon-revision.txt` must match the dependency used to
build the assets. The generated manifest lives at
`docs/playground/device-harness-build.json`. Start the temporary server:

```powershell
node scripts/serve-device-harness.mjs
```

The server listens on LAN interfaces at port 8080 and prints loopback and LAN
addresses ending in `/playground/etalon-vibe-harness.html`. Open the LAN address on a phone connected
to the same network. Stop the server with Ctrl+C. It does not install a service,
change firewall settings, receive uploads or send telemetry. A network/firewall
that blocks inbound connections must be configured separately by its owner.

Only the harness HTML, its build manifest, its four JavaScript modules and the
two exact diagnostics assets selected by the bounded build manifest are served. Those
assets must resolve within public `docs/pkg` and be named `qualia.js` and
`qualia_bg.wasm`. The root URL opens the harness page. Every other document is
denied, including other files that happen to exist under `docs`. Directory
requests, dot paths, traversal, request bodies and methods other than GET/HEAD
are rejected. A missing or invalid manifest prevents startup. File responses are
streamed with a 128 MiB file limit, 32 active requests, bounded headers/path size
and idle/request timeouts. This is a local development server; it has no access
authentication. Use a trusted LAN, and stop it after the test.

Plain HTTP over a phone's LAN address is not a secure context. Use its CPU checks;
browser WebGPU needs HTTPS trusted by that phone. Desktop localhost has separate
browser secure-context treatment, so its behavior does not prove phone support.
For HTTPS, supply your own local PEM certificate and private key:

```powershell
node scripts/serve-device-harness.mjs --port 8443 --cert C:/certs/device-cert.pem --key C:/certs/device-key.pem
```

The certificate must cover the hostname/IP in the URL and be trusted by each
device. The server does not generate certificates or disable certificate checks.
Keep the private key outside the public harness files and generated asset paths.
A trusted HTTPS origin enables the browser's
secure-context gate; it does not establish GPU/NPU availability or successful
execution. The harness must report those separately.

To restrict access to this computer, pass `--host 127.0.0.1`. To choose another
port, pass `--port PORT`. GitHub Pages can serve the same built `docs` assets over
HTTPS; publishing still requires the repository's Pages workflow and permission.

Run the server's routing/budget/configuration checks with:

```powershell
node --test scripts/serve-device-harness.test.mjs
```
