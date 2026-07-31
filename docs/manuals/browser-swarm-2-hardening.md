# Browser Swarm 2 — Hardening (Trust Real / Cookies Clear / Agent Aligned)

**Branch:** `0.0.28` (canonical tree `C:\Projects\qualia-27062026` only)  
**Status:** **Ready to execute** when Timothy says go  
**Decisions:** [`browser-advisor-decisions-addendum-2026-07-17.md`](browser-advisor-decisions-addendum-2026-07-17.md)  
**Prior swarm:** [`browser-engine-trust-cookies-swarm.md`](browser-engine-trust-cookies-swarm.md) (wave1 done; Servo deferred forever for *this* plan)  
**Progress log:** `docs/plans/browser-swarm-2-PROGRESS-LOG.md` (create on boot)  
**Prerequisite (done):** Perception closeout + hypermedia **models/ontologies** catalogue in Library — browser must incorporate that pipeline, not fork it.

---

## 0. Goal

Turn the existing skeleton into something that **changes who controls trust when a page loads**:

| Already have | This swarm makes real |
|--------------|------------------------|
| OS WebView path | — |
| User-controlled trust store + empty catalog | Host-pin + optional chain vs **enabled** PEMs |
| Windows cert-error hook (deny default) | Policy A/B + logged interactive escape hatch |
| Cookie jar → graph transparency (partial) | View + clear site data product surface |
| Crypto surface (identity, vault, PQ, SD) | Bounded X.509 helpers + signed catalog + agent RootCertStore |
| Browser agent (deterministic) | Same trust store as chrome |

**Not goals:** Servo embed, full Mozilla root program, CT/OCSP/MDM, silent MITM, inventing PEMs, new LLM deps.

---

## 1. Immovable rules (every track)

1. Canonical tree only; CLAIM/RELEASE exclusive files.  
2. **No invented roots** — principal (or explicitly authorised process) curates PEMs only.  
3. **Fail closed**; honesty fields on incomplete surfaces.  
4. **Never auto-allow** TLS override.  
5. **Every override logged:** host, reason, timestamp, action → existing rights/liability graph.  
6. **No silent MITM proxy.**  
7. Cookie/browsing data → graph only with sensitivity + **explicit consent**.  
8. Never weaken WebView sandbox for untrusted content.  
9. **No Servo work** in this swarm (two waves).  
10. **WebID-TLS lesson:** no constant cert pop-up loops; pin/policy steady-state is silent (see addendum §10).  
11. Library / perception catalogue is the shelf for models & ontologies; browser attaches, does not duplicate stores.  
12. Parent integrates; tests + `cargo check -p webizen-desktop` green before RELEASE.

---

## 2. Three exclusive tracks only

```text
                    ┌──────────────────────┐
                    │  W0  Boot            │
                    │  CLAIM + progress log │
                    └──────────┬───────────┘
           ┌───────────────────┼───────────────────┐
           ▼                   ▼                   ▼
    ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
    │  TRACK C    │     │  TRACK K    │     │  TRACK A    │
    │  Cert+X.509 │     │  Cookies    │     │  Agent      │
    │  + catalog  │     │  view+clear │     │  trust align│
    └──────┬──────┘     └──────┬──────┘     └──────┬──────┘
           │                   │                   │
           └───────────────────┼───────────────────┘
                               ▼
                    ┌──────────────────────┐
                    │  W-FINAL  Integrate  │
                    │  dogfood notes       │
                    └──────────────────────┘
```

Tracks may run **in parallel** only while exclusive file sets do not collide.  
**Track C owns** `webizen_trust` + X.509 helpers + cert_override policy.  
**Track K owns** cookie UI + clear APIs + cookie_graph clear paths.  
**Track A owns** `browser_agent` + agent TLS client construction only — **reads** trust store API from C; does not rewrite policy.

---

## 3. Track C — Cert policy depth + X.509 + signed catalog

### C0 — Policy freeze in code comments + types

| Deliver | Acceptance |
|---------|------------|
| Document security model A/B/escape in `webizen_trust` + `cert_override` | Status JSON lists modes; default deny |
| Enum/reason codes for decisions | Stable strings for audit + UI |

### C1 — Host-pin (A) complete path

| Deliver | Acceptance |
|---------|------------|
| Pin CRUD already partial — ensure cert-error path uses only pins when B not eligible | Site with pin loads; without pin → deny |
| **Always** from interactive → creates host pin (no re-prompt until removed) | WebID-TLS lesson: steady-state silent |
| Allow once → session-scoped, audited, not permanent pin | Expires on session end or explicit clear |

### C2 — Bounded X.509 path building (B)

| Deliver | Acceptance |
|---------|------------|
| Module (e.g. `webizen_trust/x509_path.rs` or `specialized_libs` thin helper): leaf + intermediates + **enabled** PEM roots → Accept/Reject + reason codes | Unit tests on fixture PEMs **supplied in tests as generated ephemeral self-signed only**, or empty-root deny; **no shipped third-party roots** |
| Wire B only when store has enabled PEM roots | Disabled roots never consulted |
| WebView2 hook: if A fails, try B; never auto-allow | Fail closed |

**Hard caps:** no CT, OCSP, full root program, MDM.

### C3 — SPKI pin

| Deliver | Acceptance |
|---------|------------|
| SPKI fingerprint pin per host (optional high-assurance) | Mismatch → deny + reason `spki_mismatch` |

### C4 — Signed suggested catalog

| Deliver | Acceptance |
|---------|------------|
| Catalog file format + signature verify (principal ML-DSA or Ed25519) | Tamper → reject load |
| Empty or disabled-by-default content | Import UI only; agents never invent PEMs |
| Packaging path for principal-authored bundle | Documented |

### C5 — Audit → rights graph

| Deliver | Acceptance |
|---------|------------|
| `cert_override_audit.jsonl` already exists — ensure fields: host, reason, timestamp, action, policy_mode | Parseable; optional quin export later |
| No silent drops | |

### Exclusive files (Track C)

- `crates/qualia-client-core/src/webizen_trust.rs` (+ split submodules if large)  
- `crates/webizen-desktop/src/browser/cert_override.rs`  
- Trust-related desktop commands / chrome Trust UI (coordinate if shared with studio)  
- `bundled/trust/` **schema only** — never commit invented production PEMs  

### Tests

- Host pin allow/deny  
- Empty roots → B never accepts  
- Signature reject on bad catalog  
- Audit line written on interactive decision  

---

## 4. Track K — Cookie view + clear

### K0 — Honest inventory

| Deliver | Acceptance |
|---------|------------|
| Document what WebView2 jar APIs return vs graph coverage | UI shows “coverage” note |

### K1 — View

| Deliver | Acceptance |
|---------|------------|
| Cookies panel lists jar-refreshed cookies for current URL (existing refresh path deepened) | User sees host/name/path/secure/httpOnly where API allows |
| Third-party / purpose notes where known | No false “complete privacy” claim |

### K2 — Clear site data

| Deliver | Acceptance |
|---------|------------|
| Clear cookies for origin / all (as API allows) | Confirm dialog; audit log entry |
| Clear related cookie_graph rows for that origin | Graph and jar not left contradicting silently |

### K3 — Library export (optional, secondary)

| Deliver | Acceptance |
|---------|------------|
| Only if time: export summary to Library with **sensitivity** + **explicit consent** toggle | Default off; sanctuary rules apply |
| No auto-ingest browsing history | |

### Exclusive files (Track K)

- `crates/qualia-client-core/src/cookie_graph.rs`  
- Desktop cookie commands + chrome Cookies panel / studio browser cookies UI  
- Do **not** edit `webizen_trust` or `cert_override`  

### Tests

- Upsert + clear origin  
- Coverage honesty field present  

---

## 5. Track A — Agent alignment to trust store

### A0 — Single policy core

| Deliver | Acceptance |
|---------|------------|
| Agent HTTPS builds `rustls` `RootCertStore` from **enabled** PEMs when policy says so | Same store as UI |
| If no enabled PEMs → system roots or fail closed **as documented** (pick one, honesty field) | Status shows which |

### A1 — Grounded agent only

| Deliver | Acceptance |
|---------|------------|
| Keep deterministic grounded answers | No new model dependency |
| Agent explanations of “why untrusted” use trust store reason codes | Same vocabulary as cert UI |

### A2 — No parallel agent TLS policy

| Deliver | Acceptance |
|---------|------------|
| Agent cannot override pin/root rules the chrome denies | Integration test or shared function |

### Exclusive files (Track A)

- `crates/qualia-client-core/src/browser_agent.rs`  
- Agent HTTP client construction only  
- May **call** public trust APIs from C; must not reimplement path building  

### Tests

- RootCertStore empty vs one enabled PEM (ephemeral fixture)  
- Grounded path still returns without LLM  

---

## 6. WebID-TLS / RWW UX requirements (cross-cutting)

Applies mainly to Track C UI:

| Requirement | Implementation hint |
|-------------|---------------------|
| No per-navigation cert spam | After **Always** → host pin; subsequent errors silent if pin still matches |
| **Allow once** does not create pin | Session map; cleared on tab close or app restart |
| **Deny** sticky optional | Soft-deny list per host (audited) so user isn’t re-prompted every second |
| Client-certificate WebID (future) | Out of scope for swarm-2 code unless hook already exists; **design note only**: selection must be pin-based, not OS popup storm |

Do **not** reintroduce WebID-TLS by wiring system cert picker without principal-store mediation.

---

## 7. Library / perception pipeline incorporation

| Asset | Browser relationship |
|-------|----------------------|
| `model://webizen/*` Library entries | Agent/tools may *reference* honesty-labelled models; browser does not ship weights |
| `ontology://webizen/*` | Trust/policy vocab and SPARQL-MM style metadata stay on Library Software shelf |
| `{storage}/models/*` seed weights | Not TLS roots; never confuse with PEM catalog |
| Sanctuary / vault | Trust store and cookie exports inherit same unlock / sensitivity |

Track agents: if you need a shelf for “trust package receipt,” use **Library Software or Tools** via existing hypermedia APIs — do not invent `browser_only_trust.json` without a migration path to Library.

---

## 8. W0 boot checklist

1. Append CLAIM to `coordination/NOTICES.md`.  
2. Create `browser-swarm-2-PROGRESS-LOG.md` with base SHA.  
3. Re-read addendum §3 security model.  
4. Assign tracks C / K / A (serial if one agent; parallel if three with exclusive files).  
5. Confirm Servo **not** in CLAIM set.  

---

## 9. W-FINAL acceptance

- [ ] Cert error: deny by default; host-pin allows; enabled-root chain allows only when B eligible  
- [ ] Interactive escape hatch fully logged; Always → pin (no re-prompt loop)  
- [ ] X.509 helpers tested; no third-party root program shipped  
- [ ] Signed catalog verify path works (empty catalog OK)  
- [ ] Cookies: view + clear for origin with honesty coverage note  
- [ ] Agent uses same enabled PEM set (or documented fail-closed/system fallback)  
- [ ] Dogfood notes updated; no Servo claim  
- [ ] `cargo test` for touched modules + `cargo check -p webizen-desktop`  

---

## 10. Explicit non-goals (cut list)

- Servo embed / libservo linking  
- Full browser root program / CT / OCSP / MDM  
- Silent MITM or “corporate SSL inspection” product  
- Invented AU/community PEMs in repo  
- New LLM dependency for browser agent  
- Auto-ingest all browsing into Library  
- Weakening sandbox  

---

## 11. Suggested first code slice (when execute)

**C1 + C2 skeleton:** reason codes + host-pin Always/once/deny UX + unit tests for deny-without-pin; then B path building against **test-only ephemeral** certs.

Parallel: **K2 clear site data** if Track K free.

---

*End of browser-swarm-2-hardening. Three tracks. Principal-controlled trust. No Servo. No invented roots.*
