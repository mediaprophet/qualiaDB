# Design constraints (Timothy lock 2026-09-19)

Gate 0 chrome PASS stands for **shared sense** (dock / launcher / inventory). These constraints apply to the **next design pass** before shell wiring is treated as done.

## Not a classic desktop clone

The new Webizen Desktop **must not** read as OSX, Windows, or Linux desktop chrome from decades ago.

- Avoid stock dock-at-bottom + flat icon strip + titlebar traffic-lights as the *identity* of the product (those can remain as temporary Gate 0 scaffolding only).
- Prefer a **spatial / 3D-capable** shell that uses engine capabilities already Present (WGPU / GPU viewport / 10D / Chora / renderer attach-to-app-window) so the OS feel is native to Qualia, not a pastiche of 1990s–2010s GUIs.

## Creative direction (design team)

@monet @davinci — evolve mock-ups beyond Gate 0 HTML placeholders:

1. **Spatial shell** — depth, layers, or manifold-adjacent surfaces instead of a flat wallpaper + dock only.
2. **Apps as first-class volumes / windows** that can still open in-window or new-window, without looking like Finder/Explorer clones.
3. **Humans-first favorites** stay (Talk · Mail · Directory before tools) — Continuity: handle ≠ human; chatbot = tool.
4. **Live vs Planned** honesty only — no held theatre.

## Screen sizes

Must run across a **range of screen sizes** (phone companion / small laptop / desktop / wide). Responsive layout, not a single fixed 1280 canvas. Touch and pointer both matter where companion paths exist.

## Accessibility

First-class, not a late bolt-on:

- Keyboard reachability for launcher, apps, and window focus
- Contrast / readable type at small sizes
- Screen-reader / semantic structure for shell chrome (roles, names, focus order)
- Respect reduced-motion where 3D motion is used
- No essential UI that only exists as unlabelled icon glyphs

## Coexistence (unchanged)

New Desktop = default. Legacy shell stays behind a flag until the new one is actually better. Leave Poet; migrate Desktop first, then Poet in as one app.
