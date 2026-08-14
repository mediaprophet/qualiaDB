# Mobile WASM LLM LAN lab

The lab serves `docs/` to a phone and records opt-in browser diagnostics as bounded JSONL under
`.qualia/mobile-wasm-lab/<session>/events.jsonl`. The session URL enables telemetry only for that
tab; ordinary demo visits do not send anything.

## 1. Create a certificate the phone trusts

WebGPU requires a secure context. `http://192.168.x.x` can test the UI/logger but cannot run the
model. Using `mkcert`:

```powershell
ipconfig  # Find the active Wi-Fi/Ethernet IPv4 address.
$lanIp = '<LAN-IP>'

New-Item -ItemType Directory -Force .qualia\mobile-wasm-lab\certs | Out-Null
mkcert -install
mkcert -cert-file .qualia\mobile-wasm-lab\certs\lan.pem `
       -key-file .qualia\mobile-wasm-lab\certs\lan-key.pem `
       $lanIp localhost 127.0.0.1
mkcert -CAROOT
```

Copy `rootCA.pem` from the printed CA directory to the phone and trust it:

- Android: install it as a CA certificate in Security settings.
- iOS/iPadOS: install the profile, then enable full trust under
  Settings → General → About → Certificate Trust Settings.

The private `rootCA-key.pem` and `lan-key.pem` must remain on the development machine.

## 2. Run the server

```powershell
py tools\mobile_wasm_lab.py `
  --cert .qualia\mobile-wasm-lab\certs\lan.pem `
  --key .qualia\mobile-wasm-lab\certs\lan-key.pem
```

The terminal prints:

- a localhost landing page containing a QR code;
- the exact HTTPS phone URL;
- the local JSONL telemetry path.

Before displaying the QR page, the server validates the male and female QBDL pack magic, byte
budget, and SHA-256 identity under their canonical `.hmc` URLs. If only the legacy `.qualia`
filenames exist, it stages `.hmc` hardlinks (or bounded copies when hardlinks are unavailable)
without deleting the source bundles. Invalid or missing assets stop startup before a phone can be
sent to a broken Anatomy URL.

Allow the Python process through Windows Firewall for private networks if prompted. Both devices
must be on the same LAN. Keep the phone browser in the foreground during model initialization.

The landing page uses `api.qrserver.com` only to render the QR image; copy the printed URL directly
if that service is unavailable or undesirable.

## 3. Review a run

The log contains environment, adapter limits, WASM boot, model download/init, first-token,
completion, visibility, memory (when exposed), and error events. Generated text preview is capped
at 512 characters; user prompt text is never logged.

For a UI/network-only probe without certificate setup:

```powershell
py tools\mobile_wasm_lab.py --allow-insecure --port 8000
```

That mode deliberately warns that LAN WebGPU is unavailable over HTTP.
