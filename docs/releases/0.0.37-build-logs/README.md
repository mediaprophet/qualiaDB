# 0.0.37 release-build diagnostic logs

Captured on the `0.0.37` line after the workspace version bump. Each
`*.full.log` is the complete compiler output for one release target.
Each `*.diagnostics.log` lists only `error` / `warning` lines plus a
count summary. `SUMMARY.md` is the exit-code table.

Host: Linux x86_64, rustc 1.98.1. Windows NSIS and macOS Metal installer
jobs from `release-desktop.yml` are not reproduced here.

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
