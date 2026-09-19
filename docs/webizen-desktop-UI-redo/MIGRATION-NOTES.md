# Migration notes (Gate 0 stub)

## Phase 1 — Desktop shell

- Replace life-domain chrome with an OS shell: wallpaper, menu/status strip, dock/taskbar, window manager.
- Surface existing capabilities as **apps** (Browser, Files/Keep, Talk, Settings, VibeScript, …).
- In-window and new-window launch paths.
- Native renderer attachable to an app window (show the capability; don’t bury it).
- Honest status only: **live** vs **planned** — never held/not-yet as the product surface.

## Phase 2 — Poet migrates in

- Leave Poet alone until the shell is real.
- One Poet product: Desktop launches it; WASM is the host form.
- Inventory marks a single canonical path and a fold/retire note for the duplicate half.

## Out of scope for Gate 0

- Shipping the shell
- Rewriting Poet
- Full inventory (Marvin) / WM architecture (Neo)
