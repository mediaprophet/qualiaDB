# Solid-OutPost + Qualia Solid Bridge — Hackathon Path

**Date:** 2026-07-10  
**Principal:** Timothy Charles Holborn  
**Proposal:** Solid 2026 Hackathon v1.3.0 (Institutional Verifiable Data Egress / Solid-OutPost)  
**Primer:** Solid-OutPost Habeas Data Egress (companion HTML)  
**Code:** `crates/qualia-solid-bridge`, `qualia-cli solid *`, desktop `sync_to_solid_pod` / `fetch_from_solid_pod`

---

## 1. Roles (what “working” means)

| Role | Who runs it | Qualia surface |
|------|-------------|----------------|
| **Issuer / egress point** | Institution (CSS + middleware in proposal) | Out of band for this slice — we **consume** deposits |
| **Personal pod server** | Individual / Webizen local | `qualia-cli solid serve` (LDP + optional demo OIDC) |
| **Consumer agent** | Webizen-desktop / CLI | `solid fetch` / `solid put` / `fetch_from_solid_pod` / `sync_to_solid_pod` |
| **Vault** | QualiaDB | `export-solid` file export + Turtle→Quin import at firewall |

This matches the proposal’s split: **institutions carry the egress burden**; the person runs a **thin agent** that authenticates, retrieves, and curates into Qualia.

---

## 2. What was broken (before this work)

| Surface | Status before |
|---------|----------------|
| `qualia-solid-bridge` LDP | Simulated fixed Turtle; no real storage |
| Mock OIDC (`oidc_micro_idp`) | Compile-only `demo` feature; not on personal pod path |
| `start_proxy_daemon` | Existed; **not** wired to CLI/desktop |
| `sync_to_solid_pod` (desktop) | **Stub** — returned a success string without HTTP |
| Solid consumer HTTP | Missing |
| webizen-studio LDP browser | UI mock only |

---

## 3. What works now (verified smoke 2026-07-10)

### Personal pod server

```powershell
.\target\release\qualia-cli.exe solid serve --data-root $env:TEMP\my-pod --port 4243 --demo-oidc
```

- Filesystem LDP root: `profile/card`, `public/`, `private/`, `inbox/`
- GET/PUT/POST/DELETE on resources
- Container listings as Turtle (`ldp:BasicContainer`)
- WebID profile with `solid:oidcIssuer`, `pim:storage`, `ldp:inbox`
- **Demo OIDC** (local only): `/.well-known/openid-configuration`, `/authorize` auto-approve, `/token`, `/jwks`, `/register`
- Banner: `/.well-known/qualia-solid-bridge`

**Smoke:** PUT `public/deposit.ttl` → **201**; GET → **200** body; OIDC discovery → **200**; profile card → **200**.

### Consumer agent

```powershell
.\target\release\qualia-cli.exe solid fetch http://127.0.0.1:4243/public/deposit.ttl
.\target\release\qualia-cli.exe solid put  http://127.0.0.1:4243/public/out.ttl .\file.ttl
.\target\release\qualia-cli.exe solid post http://127.0.0.1:4243/public/ .\file.ttl --slug deposit.ttl
```

- Optional `--token` Bearer
- Turtle → NQuin best-effort parse (multi-line + `;` lists) at allocation firewall

### Desktop / client-core

| Command | Behaviour |
|---------|-----------|
| `sync_to_solid_pod(url, body_or_path?, token?)` | Real HTTP PUT (was stub) |
| `fetch_from_solid_pod(url, token?)` | Real HTTP GET + quin_count |
| `put_to_solid_pod(...)` | Real HTTP PUT |
| `export_to_solid` | Unchanged file export of `.q42` → LDP dir |

### Demo OIDC honesty

Documented in `crates/qualia-solid-bridge/NON_GOALS.md`:

- **Not** a production Solid-OIDC issuer
- Tokens are `demo-*` opaque strings — good enough for local SolidOS / CSS client smoke, **not** for provenance or access decisions in production
- Runtime flag: `--demo-oidc` / `QUALIA_SOLID_DEMO_OIDC=1` (default on for `solid serve`; off for library `BridgeConfig::default()` unless env set)

---

## 4. Mapping to Solid-OutPost / proposal flows

| Proposal step | Bridge today | Still open |
|---------------|--------------|------------|
| Institution deposits VC into egress slot | Use external CSS **or** PUT into local pod for demos | Entra→Solid-OIDC middleware (issuer bridge kit) |
| LDN to citizen inbox | `inbox/` container exists | LDN receiver + notification UI |
| Consumer authenticates (Solid-OIDC) | Demo OIDC local; real IdP validation planned | DPoP, real JWT, WebID issuer check |
| Retrieve payload | `solid fetch` / desktop fetch | SAI / app registration |
| Import to QualiaDB | Turtle→Quin + `export-solid` reverse path | Persist fetch into vault graph API |
| Return channel (correction) | POST to container | Staff-review queue (institutional middleware) |

---

## 5. Remaining work (priority for hackathon week)

### Must for demo day

1. **Vault import** — `fetch` → write Quins into active vault / daemon graph (not only print quin_count).
2. **Desktop UI** — Solid LDP browser: live list/fetch from `http://127.0.0.1:4243` + remote CSS URL field.
3. **Seed script** — one-command “institutional deposit” Turtle (education VC sample) + fetch into Qualia.
4. **Document** demo OIDC limits in operator notes (already in NON_GOALS).

### Should for credible interop

5. Real **Solid-OIDC relying party** (validate access tokens from CSS/Inrupt/etc.; no mock mint for production).
6. **WAC** — serve `.acl` and enforce Read/Write for private/.
7. **LDN** inbox polling in agent.
8. Wire **webizen-desktop** “start personal pod” toggle that spawns `solid serve` as child process.

### Later (proposal enterprise kit)

9. Entra ID → WebID mapping middleware (issuer bridge).
10. SQL → JSON-LD VC deposit SDK.
11. Helm / docker-compose CSS profiles (institutional; separate from Qualia binary).

---

## 6. Commands cheat-sheet

```powershell
# Personal pod (Webizen side)
.\target\release\qualia-cli.exe solid serve --data-root C:\Pods\me --demo-oidc

# Consumer retrieve (from local pod or institutional CSS URL)
.\target\release\qualia-cli.exe solid fetch "https://pod.example/public/record.ttl" --out record.ttl

# Sync file up (deposit / backup)
.\target\release\qualia-cli.exe solid put "http://127.0.0.1:4243/public/backup.ttl" .\export\data.ttl

# File-based vault export (offline Solid container)
.\target\release\qualia-cli.exe export-solid --input vault.q42 --output .\solid-out\
```

Env:

| Variable | Meaning |
|----------|---------|
| `QUALIA_SOLID_POD_ROOT` | Default data root |
| `QUALIA_SOLID_PORT` / `HOST` | Listen |
| `QUALIA_SOLID_PUBLIC_BASE` | Advertised base URL |
| `QUALIA_SOLID_DEMO_OIDC` | `1`/`true` enable demo IdP |

---

## 7. Ontology sources (W3C ns archive)

Timothy’s archive: `C:\Projects\ontologies-2023\w3c archives\ns-main\w3c-ns`

| File in archive | Bundled as | Namespace |
|-----------------|------------|-----------|
| `ldp.ttl` | `bundled/ontologies/w3c-archives/ldp.ttl` | `http://www.w3.org/ns/ldp#` |
| `auth/acl.ttl` | `auth-acl.ttl` | `http://www.w3.org/ns/auth/acl#` |
| `auth/cert.rdf` | `auth-cert.rdf` | cert (WebID-TLS) |
| `solid/oidc.ttl` | `solid-oidc.ttl` | `http://www.w3.org/ns/solid/oidc#` |
| `solid/oidc-context.jsonld` | `solid-oidc-context.jsonld` | OIDC JSON-LD context |
| `pim/space.ttl` | `pim-space.ttl` | `http://www.w3.org/ns/pim/space#` |
| *(not in 2023 dump)* | `solid-terms.ttl` from solid/vocab | `http://www.w3.org/ns/solid/terms#` |
| `ttl/xmlns/foaf.ttl` | `foaf.ttl` | `http://xmlns.com/foaf/0.1/` |

**Startup seed** (`DEFAULT_BUNDLED_ONTOLOGIES`): `shacl`, `ldp`, `acl`, `solid-terms`, `solid-oidc`, `pim-space`, `foaf`.

**Personal pod:** on `solid serve`, same files are copied to  
`{data_root}/public/ontologies/` and listed as an LDP container  
(e.g. `http://127.0.0.1:4243/public/ontologies/ldp.ttl`).

Rust constants: `qualia_solid_bridge::vocab`.

## 8. Tests

```powershell
cargo test -p qualia-solid-bridge --lib
cargo test -p qualia-client-core --lib solid_stack
# pod_store, consumer turtle, LDP put/get, demo OIDC, vocab resolve
```

---

## 8. Non-goals (unchanged, reinforced)

- Qualia is **not** claiming to be a production OIDC provider.
- Solid bridge is **interop + human-rights retrieval**, not a replacement for institutional CSS at scale.
- Mock tokens must never be treated as court-grade provenance.

---

*Identity · Dignity · Liberté — the bridge is transport; the vault is agency.*
