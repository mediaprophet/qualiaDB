# Capt re-UAT — Settings paint + Library open (Desktop)

**Branch:** `cursor/desktop-settings-library-96b9` · **Base:** `0.0.38`  
**Surface:** Webizen Desktop native (in-shell Studio). Human-alone. No agent required.

## What changed

- Talk is home (`/` / empty hash). Lived Memory / Hypermedia Library is **`/library`** — it no longer recycles Talk home.
- Tools → Settings / `Ctrl+,` / palette **Settings** open the **in-shell** Settings shell (backend + prefs). Help → Open Settings Portal remains the loopback portal (secondary).
- Poet library view is **held / not yet** (no fake counts, no `Library.*` Host invent). Live shelf is the Desktop `/library` surface (`library_*` / `wellfair_*_library`).

## Paths to walk

### Settings (was PARTIAL — listed, no visible surface)

1. Cold-load Desktop. Finish or skip onboarding so the habitat chrome is up.
2. **Tools → Settings…** (`Ctrl+,`).
3. Expect a visible two-pane Settings surface (`data-surface="settings"`): left categories (Setup health, Data & memory, AI instruments, …), right human-readable prefs. Not a blank Talk home. Not the word “unavailable”.
4. Palette (`Ctrl+K`) → type `settings` → Enter. Same in-shell surface.
5. Optional: Help → Open Settings Portal — external loopback is fine; **in-shell is human-primary**.

Held / not yet (honest): public-preview / no desktop host → Setup health says **held / not yet**, not unavailable. Advanced Technical remains gated behind the experience switch.

### Hypermedia Library (was PARTIAL — bounced to home)

1. From Talk home, **Tools → Hypermedia Library**.
2. URL / hash must become **`/library`** (or `/studio/#/library`). Must **not** stay on `/` Talk.
3. Expect a shelf UI (`data-surface="library"`): eyebrow **Hypermedia Library**, title **Lived Memory**, collections + search. Empty shelf is OK; bounce to Talk is not.
4. Palette → `library` / `memory` → same `/library` shelf.
5. Address bar `qualia://library` (classic shell) → `/studio/#/library`.

Poet path: if you open a Poet library container, expect **held / not yet** with why (open Tools → Hypermedia Library). Do not score fake document counts as a shelf.

## Score ask

| Surface | PASS if | FAIL if |
|---------|---------|---------|
| Settings | In-shell prefs UI paints and is operable | Listed only / blank / Talk home / “unavailable” |
| Library | Distinct `/library` shelf **or** honest held / not yet | Bounce to Talk home pretending success |

Do not score Directory, Catalog, Mesh, or IA rename in this circuit.
