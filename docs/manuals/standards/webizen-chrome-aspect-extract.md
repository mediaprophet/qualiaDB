# Webizen Desktop — Poet chrome / aspect extract

**Packet:** davinci Stage 8 · **Pair:** monet motion contract  
**Frozen surface:** `vibe-host-0.1` @ `6dc2b8b8`  
**Poet coverage:** [`poet-aspect-coverage.md`](poet-aspect-coverage.md)  
**Motion:** [`poet-motion-contract.md`](poet-motion-contract.md)

Reuse Poet’s chrome grammar. Do not invent a second motion system, a second Host, or a Solid IdP shell.

## Aspects (not twins, not planes)

Layout · Stage · Timeline are **aspects** of one studio surface — three readings:

| Aspect | Reading | Beat |
|--------|---------|------|
| Layout | 2D structure | — |
| Stage | depth / z / camera | entrance = soft rise |
| Timeline | named time | dwell · exit |

They are **not identical copies**. “Twin” infers identical, which is almost always misleading — do not use it for this grammar. Do not call them “planes” (acoustic-plane and network control/data plane are other vocabularies). Not a credential digital twin. Not legal `FormationStage`.

Chrome tokens (copy as-is):

- `data-aspect-surface`
- `data-aspect-layout` / `data-aspect-stage` / `data-aspect-timeline`
- `data-beat` = `entrance` | `dwell` | `exit` only
- `.aspect-chip` / `.aspect-chip-row` with `data-aspect="layout|stage|timeline"`

Helper: `crates/poet/src/browser/surface_aspects.rs`. CSS: `crates/poet/src/browser/styles/15-studio-chrome.css`.

## Named beats only

| Beat | Look | When |
|------|------|------|
| entrance | soft rise + light fade | surface / dock / flyout opens |
| dwell | steady focus + quiet breath | work in flight, volume open |
| exit | dissolve along the same z-path | close, dismiss, successful commit |

No free tweens. Reduced-motion: same named beats, shorter or crossfade. State must never depend on motion alone. Unbound chrome looks **gated**, never stub-broken.

## Bind rules (unchanged)

- Four ops only: parse / check / diagnose / `capability.invoke`
- Live ids from `ALL_BOUND` / `vibe:InvokeId` only
- No Host widen · no dotted `qualia.*`
- Diagnose glow uses UTF-8 byte spans
- Volume `q42:state`: closed · open · committed · denied · fault; commit beat only on real `GraphDatabase.volume_commit`

## What Webizen reuses vs what it does not

| Reuse | Do not |
|-------|--------|
| Aspect attributes + chips | A second Layout/Stage/Timeline vocabulary |
| Named beats + reduced-motion variants | Free tween / CSS animation sprawl |
| Gated ≠ broken honesty | Grey mystery disable |
| Human chrome names (Container, Manifold, Link) | Engineer graph-viz as the default |
| SHACL-first labels for persons / living | `owl:Thing` copy for people |

## Out of this extract

- **Solid IdP** — parked (`G-SOLID-IDP`). Solid is a Qualia **exit** adapter (LDP/WebID projector), not identity/storage/network.
- **QDNF** — naming/routing without DNS/IP. Not G-COORD. Not this chrome.
- **G-COORD** — spatial/realm Position on map containers; consume existing shapes, do not invent a network address.

## Accept

Webizen Desktop chrome that ships a studio surface carries all three aspects + a named beat, using the tokens above. Missing Stage or Timeline is a regression. Calling those readings “twins” or “planes” is a copy regression.
