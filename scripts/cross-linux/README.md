# Cross compile helper for Linux (from this Windows machine)

The exes here are renamed from the system's LLVM-MinGW clang (which supports many targets) to the names the Rust build system (cc-rs for ring, etc.) expects when cross compiling for `x86_64-unknown-linux-gnu`.

## How to export Linux qualia-cli binary

1. Make sure this dir is in your PATH, or run the .ps1 from here.

2. Run the build-linux.ps1 (from PowerShell):

   .\build-linux.ps1

3. The binary will be produced at:

   target\x86_64-unknown-linux-gnu\release\qualia-cli

4. You can then copy it to releases/ as qualia-cli-x86_64-unknown-linux-gnu or similar.

Note: This uses the clang as gcc, with CFLAGS to target linux-gnu. It may work for the deps in this project.

For aarch64, you would need the corresponding clang for aarch64-linux-gnu, which may be in the llvm-mingw if it has it, or install additional.

For full desktop app for Linux, you need to build on a Linux machine (or let the GitHub CI do it via the release workflow).

For macOS (osx), you need a Mac or the CI (macos runner).

The recommended way to export osx and linux binaries for release is:

git tag v0.0.4

git push origin v0.0.4

Then download the produced artifacts from the GitHub release (the workflow builds on native runners for each platform).

We have updated the project (CI fix for tauri version, docs, index.html, latest.json) to support and document this.