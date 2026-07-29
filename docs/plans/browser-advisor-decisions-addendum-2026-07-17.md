# Browser Advisor Decisions — Addendum (one page)

**Date:** 2026-07-17  
**Status:** **Accepted** (principal direction + advisor priority order)  
**Supersedes open agenda:** `webizen-browser-advisor-briefing.md` §7 (D1–D6)  
**Next plan:** [`browser-swarm-2-hardening.md`](browser-swarm-2-hardening.md)  
**Pipeline note:** Perception / hypermedia Library catalogue (models, ontologies) is **in** before this swarm advances — browser work must incorporate Library + identity governance, not invent a second trust island.

---

## 1. Differentiator (immovable)

The differentiator is **not** “own browser engine.”  
It is **principal-controlled trust** connected to **identity, Library, and agent** under the **same governance model**.

Every new browser surface must either (a) make the principal’s own trust decisions **real**, or (b) be cut.

---

## 2. Priority order (next 4–8 weeks)

| Order | Decision | Verdict |
|------:|----------|---------|
| 1 | **D2 + D6** | **Do now** — cert-error policy depth + bounded X.509 helpers (highest leverage) |
| 2 | **D1** | **Decide once, freeze** — suggested trust content stance |
| 3 | **D4** | **Do** — cookie view + clear site data (transparency first) |
| 4 | **D5** | **Keep** deterministic grounded agent; no new model dependencies |
| 5 | **D3 Servo** | **Hard defer** — preference UI already honest; no embed work next two waves |

---

## 3. Certificate override security model (one sentence)

**Host-pin only by default (A); allow full chain verification against principal-supplied roots only when those roots are explicitly enabled by the user (B); never auto-allow; interactive “Allow once / Always / Deny” is permitted solely as a temporary, fully-logged escape hatch.**

| Mode | When | Default |
|------|------|---------|
| **A Host-pin** | Host matches principal pin | Always available; fail closed if no pin |
| **B Chain vs enabled PEMs** | User enabled those PEMs in trust store | Only if root set non-empty and enabled |
| **Interactive escape** | Allow once / Always / Deny | Logged; never silent; Always → pin or policy row |

---

## 4. D1 — Suggested trust content (decided)

- **Default catalog remains empty** (or disabled-by-default if principal later supplies a bundle).  
- **No agent- or vendor-minted roots** under the Webizen name.  
- Principal may import PEMs / enable packages they authorise; **signed catalog** (principal’s ML-DSA or Ed25519) when packaging.  
- Freeze discussion: implement loaders + honesty UI; do not re-litigate root *content* without principal files.

---

## 5. D6 — Crypto packages (approved §6.3 only)

| # | Package | Cap |
|---|---------|-----|
| 1 | Bounded X.509 path building (leaf + intermediates + **enabled** PEM roots → accept/reject + reason codes) | No full root program, no CT, no OCSP, no MDM |
| 2 | SPKI pinning | High-assurance hosts only |
| 3 | Catalog signed by **principal’s** ML-DSA or Ed25519 | Packaging cannot silently swap |
| 4 | Agent `rustls` `RootCertStore` from **same** enabled set | One policy core |

**Hard caps:** no silent MITM proxy; agents never invent/ship PEMs; every override logged (host, reason, timestamp, action) for rights/liability graph.

---

## 6. D4 — Cookies

- **Yes:** view + clear site data (transparency).  
- **Secondary:** export to Library graph — only with sensitivity classification + **explicit consent**.  
- Do not overclaim completeness of jar coverage.

---

## 7. D5 — Browser agent

- Keep **deterministic grounded** answers.  
- Optional local model later — **not a blocker**.  
- Agent TLS must use the **same** enabled trust store as cert policy (package 4).

---

## 8. D3 — Servo

**Defer / kill for the next two waves.**  
Revisit only if the WebView path becomes a strategic liability (it is not today).

---

## 9. Legal / rights red lines

- Never weaken WebView sandbox for untrusted content.  
- Overrides and custom roots: **attributable + logged**.  
- No third-party root programs shipped as “Webizen roots.”  
- Cookie/browsing data entering personal graph: sanctuary / duress / purpose-bound rules apply.  
- Fail closed; honesty fields on every incomplete surface.

---

## 10. Historical debt: RWW / WebID-TLS (must fix, not re-break)

**Problem:** Read-Write Web **WebID-TLS** worked cryptographically but **failed as product** because browsers forced **constant certificate confirmation pop-ups**, training users to click through or abandon the model.

**Webizen stance:**

| Anti-pattern (old RWW pain) | Required fix |
|----------------------------|--------------|
| Prompt on every navigation / every TLS event | **No nag loop** — once decision is Always (pin) or Deny, silent |
| Opaque system “remember password” for certs | Decisions live in **principal trust store** + audit log |
| No attribution | Who / when / host / action → liability graph |
| Auto-trust “to reduce friction” | **Never auto-allow** |

Interactive Allow once / Always / Deny is the **escape hatch**, not the steady state. Steady state is **host-pin or enabled-root chain verify** without re-prompting until pin/root/policy changes.

---

## 11. Pipeline incorporation (perception / Library)

Already landed and **must stay on the browser path**:

- Hypermedia Library catalogue for **models** and **ontologies** (`perception_catalog`, Software shelf).  
- Seed weights under `{storage}/models/` with honesty flags.  
- Same storage root / vault / identity stack as trust store.

Browser swarm-2 **must not** invent parallel “browser-only” model or ontology stores. Trust anchors, cookie graph exports, and agent receipts attach to **Library + rights graph** when they leave the chrome.

---

## 12. Immediate next step

Execute [`browser-swarm-2-hardening.md`](browser-swarm-2-hardening.md) — **three exclusive tracks only** (cert/X.509/catalog, cookies view+clear, agent trust alignment). Servo out of scope.

*End of addendum.*
