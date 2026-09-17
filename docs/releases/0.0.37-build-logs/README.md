# 0.0.37 release-build diagnostic logs

Captured on the `0.0.37` line after the workspace version bump. Each
`*.full.log` is the complete compiler output for one release target.
Each `*.diagnostics.log` lists only `error` / `warning` lines plus a
count summary. `SUMMARY.md` is the exit-code table.

Host: Linux x86_64, rustc 1.98.1.

Windows / macOS: see [`CROSS-PLATFORM.md`](CROSS-PLATFORM.md). This host
produced a MinGW `qualia-cli.exe` and type-checked `webizen-desktop` for
`x86_64-pc-windows-gnu`. It did **not** produce the signed NSIS / Metal
installers from `release-desktop.yml`. macOS `aarch64-apple-darwin` fails
here (no Apple SDK).

GitHub Pages WASM size gates: see [`WASM-SIZE-GATES.md`](WASM-SIZE-GATES.md).
Portal and playground fail the old 2.6 MiB / 800 KiB cap and pass the
16 MiB / 4 MiB sanity cap used by `pages.yml` and `release-wasm.yml`.

## First-pass outcome

Native Linux CLI, P64 converter, anatomy example, studio lib, poet-cli,
and vibe built. WASM portal / lite / playground and Linux desktop did not.

WASM compile failures cluster in three seams:

1. Poet math invoke hosts gated behind `wasm-scientific` but still
   referenced from the shared dispatcher (`dft_complex`, BDF/Verlet/JVP
   hosts, `poly_coeffs`).
2. Specialized CAS libraries (`polynomial_algebra`, `symbolic_*`,
   `constructibility`, `multivar_calculus`) compiled out on wasm32 even
   when `wasm-scientific` is on.
3. `webizen-lite-wasm` (`wasm-ontology`) compiles `coord_seams` while
   `governance::coordination` is feature-gated off.

Linux desktop failed in `gdk-sys` because GTK 3 development packages
were not installed on this host.

Native `qualia-core-db` release also emits ~38 unused-item warnings
(imports, clinical helpers, crypto test vectors).
