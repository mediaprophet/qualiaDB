# monet → Capt / Neo — live /shell chrome (inlined)

**Tip target:** branch `0.0.40-webizen-ui` · path `crates/webizen-desktop/static/os-shell/index.html`

## Why Capt still saw tip ~11677f80
- Desktop serves `/shell` via `include_str!` of this HTML (compile-time).
- Prior elevate tip linked `shell.css` as a separate file — **not served** on `/shell`, so Capt either stayed on old tip or got unstyled chrome.
- Docs mock `spatial/01-spatial-shell.html` ≠ live Desktop until this file matches **and** Capt rebuilds.

## This tip
- Single-file HTML: Continuity ribbon (who·claim·handle·tool), humans-first halo, depth field, soft stage — **CSS inlined** for `include_str`.
- Same orbit routes / `__WEBIZEN_OS_SHELL__` / settings-port origin helper.
- Empty: person waiting for a person · no Classic/Relations copy.

## Capt UAT
1. Pull tip · **rebuild** Desktop (include_str).
2. Cold-load `/shell` shot — expect Continuity ribbon + halo (not left-rail-only classic).
3. Soft-rise still held until Neo strips Studio Classic/Relations from orbit iframes (bare app volumes).
