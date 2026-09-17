# 0.0.37 Windows / macOS attempts

This Linux x86_64 agent cannot run the GitHub Actions jobs in
`release-desktop.yml` (`windows-latest` NSIS + `macos-14` Metal) or the
MSVC/macOS legs of `release-cli.yml`. Those jobs also need
`TAURI_SIGNING_PRIVATE_KEY`. What follows was run **on this host**.

| Target | What ran | exit | Result |
| --- | --- | ---: | --- |
| `17-cli-windows-gnu` | `cargo check -p qualia-cli --target x86_64-pc-windows-gnu` | 0 | Type-checks |
| `18-cli-windows-gnu-release` | `cargo build --release -p qualia-cli --target x86_64-pc-windows-gnu` | 0 | PE32+ `qualia-cli.exe` (MinGW, unstripped ~130 MiB). Not the MSVC CLI from `windows-latest`. |
| `19-desktop-windows-gnu-check` | `cargo check -p webizen-desktop --target x86_64-pc-windows-gnu` | 0 | Type-checks. **Not** `cargo tauri build --bundles nsis`. |
| `20-cli-macos-aarch64-check` | `cargo check -p qualia-cli --target aarch64-apple-darwin` | 101 | Host `cc` cannot compile `ring` / `aws-lc-sys` for Apple (`--target=arm64-apple-macosx`, no SDK / `CoreServices.h`). |

Not produced here:

- Windows NSIS installer + signed updater artifacts
- macOS Metal `.app` / `.dmg` from `build-desktop-macos-metal`
- `qualia-cli` on `windows-latest` (MSVC) or `macos-14` (Apple Silicon)

Those remain the `release-desktop.yml` / `release-cli.yml` jobs on GitHub
runners after a `v*` tag or `workflow_dispatch`.
