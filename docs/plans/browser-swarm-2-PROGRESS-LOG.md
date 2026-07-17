# Browser Swarm-2 Progress Log

**Plan:** `browser-swarm-2-hardening.md`  
**Base SHA:** `988cf960`  
**Branch:** `0.0.25`

---

## 2026-07-17 — W0 Boot

**Status:** done  
**CLAIM:** Browser swarm-2 (C + K + A). No Servo. No invented PEMs.

**Baseline:** trust store + host-pin + cert_override hook exist; PEM-enabled path incorrectly mapped to always-allow without chain verify (must fix); cookie graph view/refresh exist; agent uses system TLS only.

**Next:** Track C fix + X.509 + session escape hatch; K clear; A RootCertStore.

---

## 2026-07-17 — Tracks C + K + A executed

**Status:** done (W-FINAL integrate)

### Track C — Cert policy + X.509 + signed catalog
- **Fixed critical bug:** enabled PEMs no longer map to auto-allow; `CandidateCustomRoots` fails closed until chain verify.
- `cert_override_decision_full` + session allow-once / soft-deny; host-pin (A); chain verified (B).
- Escape hatch command: `browser_cert_escape_hatch` (allow_once | always→pin | deny).
- Audit JSONL: host, action, reason, timestamp.
- `webizen_x509`: PEM parse, SPKI fingerprint, chain verify vs enabled roots, rustls RootCertStore builder.
- Signed suggested catalog: Ed25519 verify envelope (principal key).
- Status JSON documents policy modes + WebID-TLS no-nag lesson.

### Track K — Cookies view + clear
- Graph `clear_origin` / `clear_host` / `clear_all` + audit log.
- Desktop `browser_clear_site_data` (graph + jar delete via Tauri).
- Studio Cookies panel: **Clear site data** button.
- Honesty coverage fields retained.

### Track A — Agent trust align
- `agent_tls_status` + `build_agent_http_client` adds principal PEMs via reqwest.
- Trust intent answers cite deny / host-pin / chain-B / escape hatch + agent TLS mode.
- `browser_agent_tls_status` command.

### Measured
- `webizen_*` tests: **13 passed**
- `cookie_graph` tests: **3 passed**
- `cargo check -p webizen-desktop -p webizen-studio`: **Finished**

### Residual closeout (same day)
- **WebView2 leaf PEM:** `ServerCertificate().ToPemEncoding` + `PemEncodedIssuerCertificateChain` → B chain verify in-handler; SPKI pin path too.
- **Agent custom-only TLS:** `reqwest::ClientBuilder::tls_certs_only(principal PEMs)` — platform roots off when PEMs enabled.
- Servo still deferred (by design).
- No invented PEMs shipped (by design).

### P-FINAL checklist (swarm-2)
- [x] Deny by default; host-pin allows  
- [x] PEM alone does not allow  
- [x] Escape hatch logged  
- [x] X.509 helpers tested (fail-closed paths)  
- [x] Signed catalog verify  
- [x] Cookies view + clear  
- [x] Agent same store / honesty  
- [x] No Servo claim  
