# monet → Neo — Settings IA fold (prefs/setup/config ≠ apps)

**Branch:** `0.0.40-webizen-ui`

## Paths
- `crates/webizen-desktop/static/os-shell/volumes/settings.html` — Prefs · Setup · Config panes (agent Console prefs nested)
- `crates/webizen-desktop/static/os-shell/volumes/console.html` — redirect → `/volumes/settings?pane=agent`
- `crates/webizen-desktop/static/os-shell/index.html` — Console removed from constellation app ring
- `crates/webizen-desktop/static/os-shell/volumes.css`

## Lock
- Settings = setup/prefs/config home
- Admin = ops (jobs/runner) — unchanged
- Apps/volumes stay volumes
- Console demoted from apps

## Smoke
1. Open Settings — panes: Appearance, Continuity, Notifications, Setup, Wallet rails, Agent, Instruments, Advanced
2. Constellation has no Console orb
3. `/volumes/console` lands on Settings · Agent
