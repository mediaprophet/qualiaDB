# Semantic instruments — SI-02 UX contract

**Status:** SI-09/10 plan-acceptance evidence landed (scripted walkthroughs + Desktop seams) · chrome + acceptance tests green  
**Date:** 2026-09-16  
**Depends on:** SI-01 types and lifecycle; SI-03 collectables; SI-05 collect vs activate

## 1. Surfaces

| Surface | Purpose | Empty / held copy |
|---|---|---|
| Catalogue / library | Discover Demo, Reference, Operational | held / not yet — open a pack or seed demos |
| Badge / card | Handle only; not proof | artwork is a handle, not a credential |
| Inspector | Full context before fetch/activate | inspect before network; size, licence, honesty, deps visible |
| Collect | Store bytes without activating | collected ≠ active |
| Activate / run | Closed lock + entry point | unresolved / revoked / unlabelled Demo cannot start a new run |
| Receipts | History of employment | old receipts remain after revoke |
| Manufacture (Poet) | Author, validate, sign, publish | invalid pack cannot publish; next action named |

## 2. Lifecycle chips (must not collapse)

authoring · publication · resolution · installation · trust · activation

Demo and Reference always show a **labelled** chip. Operational is never inferred from Demo.

## 3. Honesty

- Incomplete required input → **held**, never a default that looks like an observation.
- Catalogue inclusion ≠ endorsement.
- Signature / badge art ≠ truth.
- WASM: inspect ontology; native Q42 v3 volumes may be held with declared degradation.

## 4. Keyboard / a11y

Every primary action (inspect, collect, activate, run, cancel, view receipt) has a named control and accessible text from `si:accessibleText`. Icon without accessible text is not a conformant handle.

## 5. Host IDs

Dispatch is the instrument **entry point** (`assess`, `recognise`). Do not add `Host.*` methods per pack.
