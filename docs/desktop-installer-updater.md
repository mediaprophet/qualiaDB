# Webizen Desktop Installer and Updater

Webizen Desktop uses Tauri v2 bundling and the Tauri updater plugin.

## Release Outputs

The `Desktop Release` workflow publishes:

- Windows NSIS installer: `Webizen_<version>_x64-setup.exe`
- Windows updater signature: `Webizen_<version>_x64-setup.exe.sig`
- macOS Apple Silicon DMG/app bundle artifacts
- macOS updater archive/signature artifacts
- Portable diagnostic zips for Windows and macOS
- Updater feed: `webizen-latest.json`

Installed apps check:

```text
https://github.com/mediaprophet/qualiaDB/releases/latest/download/webizen-latest.json
```

## Required Secret

The workflow requires this GitHub Actions secret:

```text
TAURI_SIGNING_PRIVATE_KEY
```

If the private key was generated with a password, also set:

```text
TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

The private key must match `plugins.updater.pubkey` in
`crates/webizen-desktop/tauri.conf.json`. The workflow fails if the key is
missing or if Tauri reports that the key does not match the configured public
key.

Generate a new keypair with:

```powershell
cd crates/webizen-desktop
cargo tauri signer generate --write-keys webizen-updater.key
```

Store the private key content in `TAURI_SIGNING_PRIVATE_KEY`, then replace the
updater `pubkey` in `tauri.conf.json` with the generated public key. Rotating
the key breaks updates for already-installed clients unless a migration release
signed by the old key is shipped first.

## Local Validation

Build without bundling:

```powershell
cd crates/webizen-desktop
cargo tauri build --ci --no-bundle
```

Build a local Windows installer with a temporary signing key:

```powershell
cd crates/webizen-desktop
cargo tauri signer generate --ci --write-keys $env:TEMP\webizen-test.key --force
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $env:TEMP\webizen-test.key -Raw
cargo tauri build --ci --bundles nsis
```

A temporary key is useful only for artifact generation tests. Real update
verification requires the private key matching the committed updater public key.
