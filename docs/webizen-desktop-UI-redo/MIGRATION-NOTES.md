# Migration notes (Gate 0 stub)

## Coexistence

- Ship the **new OS shell as the default**.
- Keep the **existing Desktop** in-tree and **launchable via a flag** until the new one is actually better.
- **Do not delete** the old shell while functionality is still migrating.

## Phase 1 — Desktop shell

- Replace life-domain chrome with an OS shell: wallpaper, menu/status strip, dock/taskbar, window manager.
- Surface existing capabilities as **apps** (Browser, Files/Keep, Talk, Settings, VibeScript, …).
- In-window and new-window launch paths.
- Native renderer attachable to an app window (show the capability; don’t bury it).
- Honest status only: **live** vs **planned** — never held/not-yet as the product surface.
- Wired: default = new OS shell; legacy via `WEBIZEN_LEGACY_SHELL=1` and/or `--legacy-shell` (alias `--legacy-desktop`). Default orbit: Talk · Mail · Directory · Browser · Keep · Library · Instruments · Settings · Admin (Gate 1 spatial). QApps parked. See [`SHELL-LAUNCH.md`](SHELL-LAUNCH.md).

## Phase 2 — Poet migrates in

- Leave Poet alone until the shell is real.
- One Poet product: Desktop launches it; WASM is the host form.
- Inventory marks a single canonical path and a fold/retire note for the duplicate half.

## Out of scope for Gate 0

- Shipping the shell
- Rewriting Poet
- Deleting the legacy shell
- Full inventory (Marvin) / WM architecture (Neo)
